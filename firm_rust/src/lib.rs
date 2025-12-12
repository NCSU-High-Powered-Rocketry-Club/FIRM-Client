use firm_core::parser::{FIRMPacket, SerialParser};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, Read};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use anyhow::Result;

pub struct FirmClient {
    packet_receiver: Receiver<FIRMPacket>,
    error_receiver: Receiver<String>,
    running: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Box<dyn Read + Send>>>,
    sender: Sender<FIRMPacket>,
    error_sender: Sender<String>,
    port: Option<Box<dyn Read + Send>>,
}

impl FirmClient {
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
        })
    }

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
        let running_clone = self.running.clone();
        let sender = self.sender.clone();
        let error_sender = self.error_sender.clone();

        let handle: JoinHandle<Box<dyn Read + Send>> = thread::spawn(move || {
            let mut parser = SerialParser::new();
            let mut buffer: [u8; 1024] = [0; 1024];

            while running_clone.load(Ordering::Relaxed) {
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        parser.parse_bytes(&buffer[..bytes_read]);
                        while let Some(packet) = parser.get_packet() {
                            if sender.send(packet).is_err() {
                                return port; // Receiver dropped
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
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

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            if let Ok(port) = handle.join() {
                self.port = Some(port);
            }
        }
    }

    pub fn get_packets(&self, timeout: Option<Duration>) -> Result<Vec<FIRMPacket>, RecvTimeoutError> {
        let mut packets = Vec::new();

        // If blocking, wait for at most one packet. The next loop will drain any others.
        if let Some(duration) = timeout {
            let pkt = self.packet_receiver.recv_timeout(duration)?;
            packets.push(pkt);
        }

        while let Ok(pkt) = self.packet_receiver.try_recv() {
            packets.push(pkt);
        }
        Ok(packets)
    }

    pub fn get_all_packets(&self) -> Result<Vec<FIRMPacket>, RecvTimeoutError> {
        self.get_packets(None)
    }

    pub fn check_error(&self) -> Option<String> {
        self.error_receiver.try_recv().ok()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl Drop for FirmClient {
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
        let result = FirmClient::new("invalid_port_name", 115200, 0.1);
        assert!(result.is_err());
    }
}
