use anyhow::Result;
use firm_core::client_packets::{FIRMCommandPacket, FIRMMockPacket};
use firm_core::constants::mock_constants::{HEADER_PARSE_DELAY, HEADER_TOTAL_SIZE};
use firm_core::constants::mock_constants::FIRMMockPacketType;
use firm_core::data_parser::SerialParser;
use firm_core::framed_packet::Framed;
use firm_core::firm_packets::{
    DeviceConfig, DeviceInfo, DeviceProtocol, FIRMData, FIRMResponse,
};
use firm_core::mock::LogParser;
use serialport::SerialPort;
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub mod mock_serial;

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
    packet_receiver: Receiver<FIRMData>,
    response_receiver: Receiver<FIRMResponse>,
    error_receiver: Receiver<String>,
    running: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Box<dyn SerialPort>>>,
    sender: Sender<FIRMData>,
    response_sender: Sender<FIRMResponse>,
    error_sender: Sender<String>,
    command_sender: Sender<FIRMCommandPacket>,
    command_receiver: Option<Receiver<FIRMCommandPacket>>,
    mock_sender: Sender<FIRMMockPacket>,
    mock_receiver: Option<Receiver<FIRMMockPacket>>,
    port: Option<Box<dyn SerialPort>>,

    response_buffer: VecDeque<FIRMResponse>,
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
        let (mock_sender, mock_receiver) = channel();

        // Sets up the serial port
        let port: Box<dyn SerialPort> = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis((timeout * 1000.0) as u64))
            .open()
            .map_err(io::Error::other)?;

        Ok(Self::new_from_port(
            port,
            sender,
            receiver,
            response_sender,
            response_receiver,
            error_sender,
            error_receiver,
            command_sender,
            command_receiver,
            mock_sender,
            mock_receiver,
        ))
    }

    /// Creates a new client backed by an in-memory mock serial port.
    pub fn new_mock(timeout: f64) -> (Self, mock_serial::MockDeviceHandle) {
        let (sender, receiver) = channel();
        let (response_sender, response_receiver) = channel();
        let (error_sender, error_receiver) = channel();
        let (command_sender, command_receiver) = channel();
        let (mock_sender, mock_receiver) = channel();

        let (port, device) =
            mock_serial::MockSerialPort::pair(Duration::from_secs_f64(timeout));

        let client = Self::new_from_port(
            port,
            sender,
            receiver,
            response_sender,
            response_receiver,
            error_sender,
            error_receiver,
            command_sender,
            command_receiver,
            mock_sender,
            mock_receiver,
        );

        (client, device)
    }

    fn new_from_port(
        port: Box<dyn SerialPort>,
        sender: Sender<FIRMData>,
        receiver: Receiver<FIRMData>,
        response_sender: Sender<FIRMResponse>,
        response_receiver: Receiver<FIRMResponse>,
        error_sender: Sender<String>,
        error_receiver: Receiver<String>,
        command_sender: Sender<FIRMCommandPacket>,
        command_receiver: Receiver<FIRMCommandPacket>,
        mock_sender: Sender<FIRMMockPacket>,
        mock_receiver: Receiver<FIRMMockPacket>,
    ) -> Self {
        Self {
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
            mock_sender,
            mock_receiver: Some(mock_receiver),
            port: Some(port),
            response_buffer: VecDeque::new(),
        }
    }

    /// Starts the background thread to read from the serial port and parse packets.
    pub fn start(&mut self) {
        // Return early if already running
        if self.join_handle.is_some() {
            return;
        }

        // Gets the port or return if not available
        let mut port = match self.port.take() {
            Some(s) => s,
            None => return,
        };

        let command_receiver = match self.command_receiver.take() {
            Some(r) => r,
            None => return,
        };

        let mock_receiver = match self.mock_receiver.take() {
            Some(r) => r,
            None => return,
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
            // Buffer for reading from serial port
            let mut buffer: [u8; 1024] = [0; 1024];

            while running_clone.load(Ordering::Relaxed) {
                // Drain pending command packets first and write them to the port.
                while let Ok(cmd) = command_receiver.try_recv() {
                    let cmd_bytes = cmd.to_bytes();
                    if let Err(e) = port.write_all(&cmd_bytes) {
                        let _ = error_sender.send(e.to_string());
                        running_clone.store(false, Ordering::Relaxed);
                        return port;
                    }
                }
                let _ = port.flush();

                // Then drain pending mock packets and write them to the port.
                while let Ok(pkt) = mock_receiver.try_recv() {
                    let pkt_bytes = pkt.to_bytes();
                    // let hex = pkt_bytes
                    //     .iter()
                    //     .map(|b| format!("{:02X}", b))
                    //     .collect::<Vec<_>>()
                    //     .join(" ");
                    // println!("Mock pkt bytes: {hex}");
                    
                    if let Err(e) = port.write_all(&pkt_bytes) {
                        let _ = error_sender.send(e.to_string());
                        running_clone.store(false, Ordering::Relaxed);
                        return port;
                    }
                }
                let _ = port.flush();

                // Read bytes from the serial port
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        // Feed the read bytes into the parser
                        parser.parse_bytes(&buffer[..bytes_read]);

                        // Reads all available data packets and send them to the main thread
                        while let Some(firm_data_packet) = parser.get_data_packet() {
                            let packet = firm_data_packet.data().clone();
                            if sender.send(packet).is_err() {
                                return port; // Receiver dropped
                            }
                        }

                        // Reads all available response packets and send them to the main thread
                        while let Some(firm_response_packet) = parser.get_response_packet() {
                            let response = firm_response_packet.response().clone();
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

        // The receivers are moved into the background thread on start()
        // This remakes them so the client can be restarted.
        if self.command_receiver.is_none() {
            let (new_sender, new_receiver) = channel();
            self.command_sender = new_sender;
            self.command_receiver = Some(new_receiver);
        }

        if self.mock_receiver.is_none() {
            let (new_sender, new_receiver) = channel();
            self.mock_sender = new_sender;
            self.mock_receiver = Some(new_receiver);
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
    ) -> Result<Vec<FIRMData>, RecvTimeoutError> {
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
    ) -> Result<Vec<FIRMResponse>, RecvTimeoutError> {
        let mut responses: Vec<FIRMResponse> = self.response_buffer.drain(..).collect();

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

    /// Requests device info and waits for the response.
    pub fn get_device_info(&mut self, timeout: Duration) -> Result<Option<DeviceInfo>> {
        self.send_command(FIRMCommandPacket::build_get_device_info_command())?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponse::GetDeviceInfo(info) => Some(info.clone()),
            _ => None,
        })
    }

    /// Requests device configuration and waits for the response.
    pub fn get_device_config(&mut self, timeout: Duration) -> Result<Option<DeviceConfig>> {
        self.send_command(FIRMCommandPacket::build_get_device_config_command())?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponse::GetDeviceConfig(cfg) => Some(cfg.clone()),
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
        self.send_command(FIRMCommandPacket::build_set_device_config_command(config))?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponse::SetDeviceConfig(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Streams framed mock sensor packets from a `.TXT` log file.
    ///
    /// This will:
    /// 1) Send the mock command and wait for ack
    /// 2) Read the log header (`firm_core::mock::LOG_HEADER_SIZE` bytes)
    /// 3) Parse the remaining file bytes as log records
    /// 4) Send framed mock sensor packets (`FIRMMockPacket::to_bytes()`) to the device
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
        let mut header = vec![0u8; HEADER_TOTAL_SIZE];
        file.read_exact(&mut header)?;

        // Send the log header to the device, framed as a mock packet
        let header_packet = FIRMMockPacket::new(FIRMMockPacketType::HeaderPacket, header.clone());
        self.send_mock_packet(header_packet)?;

        // After we send the header we pause for a short time to let the device process it
        thread::sleep(HEADER_PARSE_DELAY);

        let mut parser = LogParser::new();
        parser.read_header(&header);

        let mut buf = vec![0u8; chunk_size];
        let mut packets_sent = 0usize;
        let stream_start = Instant::now();
        let mut total_delay_seconds = 0.0f64;

        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }

            parser.parse_bytes(&buf[..n]);
            let mock_start = Instant::now();

            while let Some((pkt, delay_seconds)) = parser.get_packet_and_time_delay() {
                total_delay_seconds += delay_seconds;

                // Mock packets are raw framed data packets; send them as raw bytes.
                self.send_mock_packet(pkt)?;
                packets_sent += 1;

                if realtime && delay_seconds > 0.0 {
                    let stream_elapsed = stream_start.elapsed().as_secs_f64();
                    // Only sleep if we're not already behind
                    if stream_elapsed <= total_delay_seconds / speed {
                        thread::sleep(Duration::from_secs_f64(delay_seconds / speed));
                    }
                }
            }

            if parser.eof_reached() {
                println!(
                    "Finished streaming mock log file in {:.2?}, sent {} packets",
                    mock_start.elapsed(),
                    packets_sent
                );

                break;
            }
        }

        // Drain any remaining packets buffered by the parser.
        while let Some((pkt, delay_seconds)) = parser.get_packet_and_time_delay() {
            total_delay_seconds += delay_seconds;

            self.send_mock_packet(pkt)?;
            packets_sent += 1;

            if realtime && delay_seconds > 0.0 {
                let stream_elapsed = stream_start.elapsed().as_secs_f64();
                // Only sleep if we're not already behind
                if stream_elapsed <= total_delay_seconds / speed {
                    thread::sleep(Duration::from_secs_f64(delay_seconds / speed));
                }
            }
        }

        // TODO: check this works
        // Send a cancel command when it finishes
        self.cancel(Duration::from_secs(5))?;

        Ok(packets_sent)
    }

    /// Sends a cancel command and waits for acknowledgement.
    pub fn cancel(&mut self, timeout: Duration) -> Result<Option<bool>> {
        self.send_command(FIRMCommandPacket::build_cancel_command())?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponse::Cancel(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Sends a reboot command.
    pub fn reboot(&self) -> Result<()> {
        self.send_command(FIRMCommandPacket::build_reboot_command())
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

    /// Sends a mock command, waits for acknowledgement, then starts sending mock data packets.
    fn mock(&mut self, timeout: Duration) -> Result<Option<bool>> {
        self.send_command(FIRMCommandPacket::build_mock_command())?;
        self.wait_for_matching_response(timeout, |res| match res {
            FIRMResponse::Mock(ok) => Some(*ok),
            _ => None,
        })
    }

    /// Starts mock mode and requires an acknowledgement.
    ///
    /// Returns `Ok(())` only if the device explicitly acknowledges mock mode.
    fn start_mock_mode(&mut self, timeout: Duration) -> Result<()> {
        match self.mock(timeout)? {
            Some(true) => Ok(()),
            Some(false) => Err(anyhow::anyhow!("Device rejected mock mode")),
            None => Err(anyhow::anyhow!("Timed out waiting for mock acknowledgement")),
        }
    }
    
    /// Sends a high-level command to the device.
    fn send_command(&self, command: FIRMCommandPacket) -> Result<()> {
        self.command_sender
            .send(command)
            .map_err(|_| io::Error::other("Command channel closed"))?;
        Ok(())
    }

    /// Sends a mock packet to the device.
    fn send_mock_packet(&self, packet: FIRMMockPacket) -> Result<()> {
        self.mock_sender
            .send(packet)
            .map_err(|_| io::Error::other("Mock channel closed"))?;
        Ok(())
    }

    fn wait_for_response(&mut self, timeout: Duration) -> Result<Option<FIRMResponse>> {
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
        mut matcher: impl FnMut(&FIRMResponse) -> Option<T>,
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

}

/// Ensures that the client is properly stopped when dropped, i.e. .stop() is called.
impl Drop for FIRMClient {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use firm_core::{constants::packet_constants::PacketHeader, framed_packet::FramedPacket};

    use super::*;

    #[test]
    fn test_new_failure() {
        // Test that creating a client with an invalid port fails immediately
        let result = FIRMClient::new("invalid_port_name", 2_000_000, 0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_data_packet_over_mock_serial() {
        let (mut client, device) = FIRMClient::new_mock(0.01);
        client.start();

        let mut payload = vec![0u8; 120];
        payload[0..8].copy_from_slice(&1.5f64.to_le_bytes());
        payload[8..12].copy_from_slice(&25.0f32.to_le_bytes());

        let mocked_packet = FramedPacket::new(PacketHeader::Data, 0, payload);
        device.inject_framed_packet(mocked_packet);

        // Need to give some time for the background thread to read the data
        let packets = client.get_data_packets(Some(Duration::from_millis(100))).unwrap();
        assert!(!packets.is_empty());
        assert!((packets[0].timestamp_seconds - 1.5).abs() < 1e-9);
    }
}
