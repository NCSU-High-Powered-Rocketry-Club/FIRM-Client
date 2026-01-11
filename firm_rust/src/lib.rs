use anyhow::Result;
use firm_core::client_packets::FIRMCommandPacket;
use firm_core::data_parser::SerialParser;
use firm_core::firm_packets::{
    DeviceConfig, DeviceInfo, DeviceProtocol, FIRMDataPacket, FIRMResponsePacket,
};
use firm_core::mock::{LOG_HEADER_SIZE, MockParser};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    response_receiver: Receiver<FIRMResponsePacket>,
    error_receiver: Receiver<String>,
    running: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Box<dyn SerialPort>>>,
    sender: Sender<FIRMDataPacket>,
    response_sender: Sender<FIRMResponsePacket>,
    error_sender: Sender<String>,
    command_sender: Sender<Vec<u8>>,
    command_receiver: Option<Receiver<Vec<u8>>>,
    port: Option<Box<dyn SerialPort>>,

    response_buffer: VecDeque<FIRMResponsePacket>,
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
        let (response_sender, response_receiver) = channel();
        let (error_sender, error_receiver) = channel();
        let (command_sender, command_receiver) = channel();

        let port: Box<dyn SerialPort> = serialport::new(port_name, baud_rate)
            .data_bits(serialport::DataBits::Eight)
            .flow_control(serialport::FlowControl::None)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .timeout(Duration::from_millis((timeout * 1000.0) as u64))
            .open()
            .map_err(io::Error::other)?;

        Ok(Self {
            packet_receiver: receiver,
            response_receiver,
            error_receiver: error_receiver,
            running: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            sender,
            response_sender,
            error_sender,
            command_sender,
            command_receiver: Some(command_receiver),
            port: Some(port),
            response_buffer: VecDeque::new(),
        })
    }

    /// Starts the background thread to read from the serial port and parse packets.
    pub fn start(&mut self) {
        if self.join_handle.is_some() {
            return;
        }

        // Get the port and command receiver: either the ones from new(), or none (restart)
        let mut port = match self.port.take() {
            Some(s) => s,
            None => return,
        };

        let command_receiver = match self.command_receiver.take() {
            Some(r) => r,
            None => {
                let (new_sender, new_receiver) = channel();
                self.command_sender = new_sender;
                new_receiver
            }
        };

        self.running.store(true, Ordering::Relaxed);
        // Clone variables for the thread. This way we can move them in, and the original ones
        // are still owned by self.
        let running_clone = self.running.clone();
        let sender = self.sender.clone();
        let response_sender = self.response_sender.clone();
        let error_sender = self.error_sender.clone();

        let handle: JoinHandle<Box<dyn SerialPort>> = thread::spawn(move || {
            let mut parser = SerialParser::new();
            // Buffer for reading from serial port. 1024 bytes should be sufficient.
            let mut buffer: [u8; 1024] = [0; 1024];

            while running_clone.load(Ordering::Relaxed) {
                // Drain pending commands and write them to the port.
                while let Ok(cmd_bytes) = command_receiver.try_recv() {
                    if let Err(e) = port.write_all(&cmd_bytes) {
                        let _ = error_sender.send(e.to_string());
                        running_clone.store(false, Ordering::Relaxed);
                        return port;
                    }
                    let _ = port.flush();
                }

                // Read bytes from the serial port
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        // Feed the read bytes into the parser
                        parser.parse_bytes(&buffer[..bytes_read]);
                        while let Some(packet) = parser.get_data_packet() {
                            if sender.send(packet).is_err() {
                                return port; // Receiver dropped
                            }
                        }

                        while let Some(response) = parser.get_response_packet() {
                            if response_sender.send(response).is_err() {
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

        // The command receiver is moved into the background thread on start(); recreate the
        // channel after stopping so the client can be restarted.
        if self.command_receiver.is_none() {
            let (new_sender, new_receiver) = channel();
            self.command_sender = new_sender;
            self.command_receiver = Some(new_receiver);
        }
    }

    /// Retrieves all available data packets, optionally blocking until at least one is available.
    ///
    /// # Arguments
    ///
    /// - `timeout` (`Option<Duration>`) - If `Some(duration)`, the method will block for up to `duration` waiting for a packet.
    pub fn get_data_packets(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<Vec<FIRMDataPacket>, RecvTimeoutError> {
        let mut packets = Vec::new();

        // If blocking, wait for at most one packet. The next loop will drain any others.
        if let Some(duration) = timeout {
            packets.push(self.packet_receiver.recv_timeout(duration)?);
        }

        // Drains the rest of the available packets without blocking
        while let Ok(packet) = self.packet_receiver.try_recv() {
            packets.push(packet);
        }
        Ok(packets)
    }

    /// Retrieves all available response packets, optionally blocking until at least one is available.
    ///
    /// # Arguments
    ///
    /// - `timeout` (`Option<Duration>`) - If `Some(duration)`, the method will block for up to `duration` waiting for a response.
    pub fn get_response_packets(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<Vec<FIRMResponsePacket>, RecvTimeoutError> {
        let mut responses: Vec<FIRMResponsePacket> = self.response_buffer.drain(..).collect();

        // If blocking and we have nothing buffered, wait for one response.
        if responses.is_empty() {
            if let Some(duration) = timeout {
                responses.push(self.response_receiver.recv_timeout(duration)?);
            }
        }

        while let Ok(res) = self.response_receiver.try_recv() {
            responses.push(res);
        }

        Ok(responses)
    }

    /// Retrieves all available data packets without blocking.
    pub fn get_all_packets(&mut self) -> Result<Vec<FIRMDataPacket>, RecvTimeoutError> {
        self.get_data_packets(None)
    }

    /// Retrieves all available response packets without blocking.
    pub fn get_all_responses(&mut self) -> Result<Vec<FIRMResponsePacket>, RecvTimeoutError> {
        self.get_response_packets(None)
    }

    /// Sends a high-level command to the device.
    pub fn send_command(&self, command: FIRMCommandPacket) -> Result<()> {
        self.send_command_bytes(command.to_bytes())
    }

    /// Sends raw command bytes to the device.
    pub fn send_command_bytes(&self, bytes: Vec<u8>) -> Result<()> {
        self.command_sender
            .send(bytes)
            .map_err(|_| io::Error::other("Command channel closed"))?;
        Ok(())
    }

    fn wait_for_response(&mut self, timeout: Duration) -> Result<Option<FIRMResponsePacket>> {
        // Prefer already-buffered responses.
        if let Some(res) = self.response_buffer.pop_front() {
            return Ok(Some(res));
        }

        match self.response_receiver.recv_timeout(timeout) {
            Ok(res) => Ok(Some(res)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(io::Error::other("Response channel closed").into())
            }
        }
    }

    /// Wait for a response matching `matcher` up to `timeout`.
    ///
    /// Looks through buffered responses first and keeps non-matching responses. It makes sure
    /// that the total time spent waiting is less than `timeout`.
    fn wait_for_matching_response<T>(
        &mut self,
        timeout: Duration,
        mut matcher: impl FnMut(&FIRMResponsePacket) -> Option<T>,
    ) -> Result<Option<T>> {
        // Pull any immediately-available responses into our buffer so we can search them first.
        while let Ok(res) = self.response_receiver.try_recv() {
            self.response_buffer.push_back(res);
        }

        // First, search the buffer for a match without blocking.
        if let Some((idx, value)) = self
            .response_buffer
            .iter()
            .enumerate()
            .find_map(|(i, res)| matcher(res).map(|v| (i, v)))
        {
            // Remove the matched response from the buffer and return it.
            self.response_buffer.remove(idx);
            return Ok(Some(value));
        }

        // Makes a deadline to enforce the overall timeout
        let deadline = std::time::Instant::now() + timeout;

        loop {
            // If we've already passed the deadline, give up and return None.
            let now = std::time::Instant::now();
            if now >= deadline {
                return Ok(None);
            }

            let remaining = deadline - now;

            // If the receiver was disconnected, propagate the error.
            let Some(next) = self.wait_for_response(remaining)? else {
                // No response arrived before the remaining time elapsed.
                return Ok(None);
            };

            // Keep the response in the buffer so non-matching responses are kept for other calls
            self.response_buffer.push_back(next);

            // Re-scan the buffer for a match now that we have new data.
            if let Some((idx, value)) = self
                .response_buffer
                .iter()
                .enumerate()
                .find_map(|(i, res)| matcher(res).map(|v| (i, v)))
            {
                self.response_buffer.remove(idx);
                return Ok(Some(value));
            }
        }
    }

    /// Requests device info and waits for the response.
    pub fn get_device_info(&mut self, timeout: Duration) -> Result<Option<DeviceInfo>> {
        self.send_command(FIRMCommandPacket::GetDeviceInfo)?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponsePacket::GetDeviceInfo(info) => Some(info.clone()),
            _ => None,
        })
    }

    /// Requests device configuration and waits for the response.
    pub fn get_device_config(&mut self, timeout: Duration) -> Result<Option<DeviceConfig>> {
        self.send_command(FIRMCommandPacket::GetDeviceConfig)?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponsePacket::GetDeviceConfig(cfg) => Some(cfg.clone()),
            _ => None,
        })
    }

    /// Sets device configuration and waits for acknowledgement.
    pub fn set_device_config(
        &mut self,
        name: String,
        frequency: u16,
        protocol: DeviceProtocol,
        timeout: Duration,
    ) -> Result<Option<bool>> {
        let config = DeviceConfig {
            name,
            frequency,
            protocol,
        };
        self.send_command(FIRMCommandPacket::SetDeviceConfig(config))?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponsePacket::SetDeviceConfig(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Sends a mock command, waits for acknowledgement, then starts sending mock data packets.
    pub fn mock(&mut self, timeout: Duration) -> Result<Option<bool>> {
        self.send_command(FIRMCommandPacket::Mock)?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponsePacket::Mock(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Starts mock mode and requires an acknowledgement.
    ///
    /// Returns `Ok(())` only if the device explicitly acknowledges mock mode.
    pub fn start_mock_mode(&mut self, timeout: Duration) -> Result<()> {
        match self.mock(timeout)? {
            Some(true) => Ok(()),
            Some(false) => Err(anyhow::anyhow!("Device rejected mock mode")),
            None => Err(anyhow::anyhow!("Timed out waiting for mock acknowledgement")),
        }
    }

    /// Streams mock telemetry packets synthesized from a `.bin` log file.
    ///
    /// This will:
    /// 1) Send the mock command and wait for ack
    /// 2) Read the log header (`firm_core::mock::LOG_HEADER_SIZE` bytes)
    /// 3) Parse the remaining file bytes as log records
    /// 4) Send mock telemetry frames (`FIRMMockPacket::to_bytes()`) to the device
    ///
    /// If `realtime` is true, the stream is paced based on the log timestamps. `speed` is a
    /// multiplier (1.0 = real-time, 2.0 = 2x faster, 0.5 = half-speed).
    pub fn stream_mock_log_file(
        &mut self,
        log_path: &str,
        start_timeout: Duration,
        realtime: bool,
        speed: f64,
        chunk_size: usize,
    ) -> Result<usize> {
        if speed <= 0.0 {
            return Err(anyhow::anyhow!("speed must be > 0"));
        }

        self.start_mock_mode(start_timeout)?;

        let mut file = File::open(log_path)?;
        let mut header = vec![0u8; LOG_HEADER_SIZE];
        file.read_exact(&mut header)?;

        let mut parser = MockParser::new();
        parser.read_header(&header);

        let mut buf = vec![0u8; chunk_size.max(1)];
        let mut packets_sent = 0usize;

        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }

            parser.parse_bytes(&buf[..n]);

            while let Some((pkt, delay_seconds)) = parser.get_packet_with_delay() {
                if realtime && delay_seconds > 0.0 {
                    thread::sleep(Duration::from_secs_f64(delay_seconds / speed));
                }

                // Mock packets are raw framed data packets; send them as raw bytes.
                self.send_command_bytes(pkt.to_bytes())?;
                packets_sent += 1;
            }
        }

        // Drain any remaining packets buffered by the parser.
        while let Some((pkt, delay_seconds)) = parser.get_packet_with_delay() {
            if realtime && delay_seconds > 0.0 {
                thread::sleep(Duration::from_secs_f64(delay_seconds / speed));
            }
            self.send_command_bytes(pkt.to_bytes())?;
            packets_sent += 1;
        }

        Ok(packets_sent)
    }

    /// Sends a cancel command and waits for acknowledgement.
    pub fn cancel(&mut self, timeout: Duration) -> Result<Option<bool>> {
        self.send_command(FIRMCommandPacket::Cancel)?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponsePacket::Cancel(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Sends a reboot command.
    pub fn reboot(&self) -> Result<()> {
        self.send_command(FIRMCommandPacket::Reboot)
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
