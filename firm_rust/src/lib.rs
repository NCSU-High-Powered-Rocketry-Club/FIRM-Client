use firm_core::data_parser::{SerialParser};
use firm_core::firm_packets::FIRMDataPacket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Read};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use anyhow::Result;

/// Interface to the FIRM Client device.
/// 
/// # Example:
/// 
/// 
/// use firm_rust::FIRMClient;
/// use std::{thread, time::Duration};
/// 
/// fn main() {
///    let mut client = FIRMClient::new("/dev/ttyUSB0", 2_000_000, 0.1);
///    client.start();
///
///    loop {
///         while let Ok(packet) = client.get_packets(Some(Duration::from_millis(100))) {
///             println!("{:#?}", packet);
///         }
///     }
/// }
pub struct FIRMClient {
    packet_receiver: Receiver<FIRMDataPacket>,
    error_receiver: Receiver<String>,
    running: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Box<dyn Read + Send>>>,
    sender: Sender<FIRMDataPacket>,
    error_sender: Sender<String>,
    port: Option<Box<dyn Read + Send>>,
    // Offset for zeroing pressure altitude readings.
    pressure_altitude_offset_meters: f32,
    current_altitude_meters: f32,
}

impl FIRMClient {
    /// Creates a new FIRMClient instance connected to the specified serial port.
    /// 
    /// # Arguments
    /// 
    /// - `port_name` (`&str`) - The name of the serial port to connect to (e.g., "/dev/ttyUSB0").
    /// - `baud_rate` (`u32`) - The baud rate for the serial connection. Commonly 2,000,000 for FIRM devices.
    /// - `timeout` (`f64`) - Read timeout in seconds for the serial port.
    pub fn new(port_name: &str, baud_rate: u32, timeout: f64) -> Result<Self> {
        let (sender, receiver) = channel();
        let (error_sender, error_receiver) = channel();
        
        let port = serialport::new(port_name, baud_rate)
            .data_bits(serialport::DataBits::Eight)
            .flow_control(serialport::FlowControl::None)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .timeout(Duration::from_millis((timeout * 1000.0) as u64))
            .open_native()
            .map_err(io::Error::other)?;
        
        let port: Box<dyn Read + Send> = Box::new(port);

        Ok(Self {
            packet_receiver: receiver,
            error_receiver: error_receiver,
            running: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            sender,
            error_sender,
            port: Some(port),
            pressure_altitude_offset_meters: 0.0,
            current_altitude_meters: 0.0,
        })
    }

    /// Starts the background thread to read from the serial port and parse packets.
    pub fn start(&mut self) {
        if self.join_handle.is_some() {
            return;
        }

        // Get the port: either the one from new(), or open a new one (restart)
        let mut port = match self.port.take() {
            Some(s) => s,
            None => return,
        };

        self.running.store(true, Ordering::Relaxed);
        // Clone variables for the thread. This way we can move them in, and the original ones
        // are still owned by self.
        let running_clone = self.running.clone();
        let sender = self.sender.clone();
        let error_sender = self.error_sender.clone();

        let handle: JoinHandle<Box<dyn Read + Send>> = thread::spawn(move || {
            let mut parser = SerialParser::new();
            // Buffer for reading from serial port. 1024 bytes should be sufficient.
            let mut buffer: [u8; 1024] = [0; 1024];

            while running_clone.load(Ordering::Relaxed) {
                // Read bytes from the serial port
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        // Feed the read bytes into the parser
                        parser.parse_bytes(&buffer[..bytes_read]);
                        while let Some(packet) = parser.get_packet() {
                            if sender.send(packet).is_err() {
                                return port; // Receiver dropped
                            }
                        }
                    }
                    Ok(_) => {}
                    // Timeouts might happen; just continue reading
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    // Other errors should be reported and stop the thread:
                    Err(e) => {
                        let _ = error_sender.send(e.to_string());
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
            port
        });

        self.join_handle = Some(handle);
    }

    /// Stops the background thread and closes the serial port.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // todo: explain this properly when I understand it better (it's mostly for restarting)
        if let Some(handle) = self.join_handle.take() {
            if let Ok(port) = handle.join() {
                self.port = Some(port);
            }
        }
    }

    /// Retrieves all available data packets, optionally blocking until at least one is available.
    /// 
    /// # Arguments
    /// 
    /// - `timeout` (`Option<Duration>`) - If `Some(duration)`, the method will block for up to `duration` waiting for a packet.
    pub fn get_data_packets(&mut self, timeout: Option<Duration>) -> Result<Vec<FIRMDataPacket>, RecvTimeoutError> {
        let mut packets = Vec::new();

        // If blocking, wait for at most one packet. The next loop will drain any others.
        if let Some(duration) = timeout {
            let mut pkt = self.packet_receiver.recv_timeout(duration)?;
            pkt.pressure_altitude_meters = self.assign_pressure_altitude_meters(pkt.pressure_pascals);
            self.current_altitude_meters = pkt.pressure_altitude_meters;
            packets.push(pkt);
        }

        while let Ok(mut pkt) = self.packet_receiver.try_recv() {
            pkt.pressure_altitude_meters = self.assign_pressure_altitude_meters(pkt.pressure_pascals);
            self.current_altitude_meters = pkt.pressure_altitude_meters;
            packets.push(pkt);
        }
        Ok(packets)
    }

    /// Zeros the pressure altitude based on the current pressure reading.
    /// Subsequent calls to `get_pressure_altitude_meters` will return altitude relative to
    /// this offset.
    /// 
    /// # Arguments
    /// 
    /// - `packet` (`&FIRMPacket`) - The packet from which to read the current pressure altitude. 
    ///    The pressure altitude from this packet will be used to set the offset.
    pub fn zero_out_pressure_altitude(&mut self) {
        self.pressure_altitude_offset_meters = self.current_altitude_meters;
    }

    /// Retrieves all available data packets without blocking.
    pub fn get_all_packets(&mut self) -> Result<Vec<FIRMDataPacket>, RecvTimeoutError> {
        self.get_data_packets(None)
    }

    /// Checks for any errors that have occurred in the background thread.
    /// 
    /// # Returns
    /// 
    /// - `Option<String>` - `Some(error_message)` if an error has occurred, otherwise `None`.
    pub fn check_error(&self) -> Option<String> {
        self.error_receiver.try_recv().ok()
    }

    /// Returns true if the client is currently running and reading data.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Computes the pressure altitude in meters using the international standard atmosphere model.
    /// The altitude is zeroed based on the initial pressure reading.
    /// 
    /// # Returns
    /// 
    /// - `f32` - Calculated pressure altitude in meters.
    fn assign_pressure_altitude_meters(&self, pressure_pascals: f32) -> f32 {
        (44330.0 * (1.0 - (pressure_pascals / 101325.0).powf(1.0 / 5.255))) - self.pressure_altitude_offset_meters
    }
}

/// Ensures that the client is properly stopped when dropped, i.e. .stop() is called.
impl Drop for FIRMClient {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_failure() {
        // Test that creating a client with an invalid port fails immediately
        let result = FIRMClient::new("invalid_port_name", 115200, 0.1);
        assert!(result.is_err());
    }
}
