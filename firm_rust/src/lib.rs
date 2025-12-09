use firm_core::parser::{FIRMPacket, SerialParser};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use anyhow::Result;

pub struct FirmClient {
    packet_receiver: Receiver<FIRMPacket>,
    error_receiver: Receiver<String>,
    running: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
    port_name: String,
    baud_rate: u32,
}

impl FirmClient {
    pub fn new(port_name: &str, baud_rate: u32) -> Self {
        let (_, rx) = mpsc::channel();
        let (_, err_rx) = mpsc::channel();
        
        Self {
            packet_receiver: rx,
            error_receiver: err_rx,
            running: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            port_name: port_name.to_string(),
            baud_rate,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        let port_name = self.port_name.clone();
        let baud_rate = self.baud_rate;
        let running = self.running.clone();
        let (tx, rx) = mpsc::channel();
        let (err_tx, err_rx) = mpsc::channel();
        
        // Re-create channels for new thread
        self.packet_receiver = rx;
        self.error_receiver = err_rx;

        running.store(true, Ordering::Relaxed);

        let handle = thread::spawn(move || {
            let port_result = serialport::new(&port_name, baud_rate)
                .timeout(Duration::from_millis(10))
                .open();

            let mut port = match port_result {
                Ok(p) => p,
                Err(e) => {
                    let _ = err_tx.send(e.to_string());
                    running.store(false, Ordering::Relaxed);
                    return;
                }
            };

            let mut parser = SerialParser::new();
            let mut buffer: [u8; 1024] = [0; 1024];

            while running.load(Ordering::Relaxed) {
                match port.read(&mut buffer) {
                    Ok(bytes_read) if bytes_read > 0 => {
                        parser.parse_bytes(&buffer[..bytes_read]);
                        while let Some(packet) = parser.get_packet() {
                            if tx.send(packet).is_err() {
                                return; // Receiver dropped
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => {
                        let _ = err_tx.send(e.to_string());
                        running.store(false, Ordering::Relaxed);
                        break;
                    }
                }
            }
        });

        self.join_handle = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn get_packets(&self) -> Vec<FIRMPacket> {
        let mut packets = Vec::new();
        while let Ok(packet) = self.packet_receiver.try_recv() {
            packets.push(packet);
        }
        packets
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
    fn test_new_client() {
        let client = FirmClient::new("/dev/ttyUSB0", 115200);
        assert!(!client.is_running());
        assert_eq!(client.port_name, "/dev/ttyUSB0");
        assert_eq!(client.baud_rate, 115200);
    }

    #[test]
    fn test_start_failure() {
        // Test that starting with an invalid port eventually reports an error and stops
        let mut client = FirmClient::new("invalid_port_name", 115200);
        
        // start() should succeed in spawning the thread
        assert!(client.start().is_ok());
        
        // Initially it might be marked as running
        assert!(client.is_running());
        
        // Wait for the thread to attempt connection and fail
        std::thread::sleep(Duration::from_millis(200));
        
        // Should have stopped running due to error
        assert!(!client.is_running());
        
        // Should have an error message
        assert!(client.check_error().is_some());
    }

    #[test]
    fn test_stop_idempotent() {
        let mut client = FirmClient::new("test", 115200);
        client.stop(); // Should not panic
        client.stop(); // Should still not panic
    }
}
