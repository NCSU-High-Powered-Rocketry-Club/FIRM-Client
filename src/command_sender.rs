
/// Represents a command that can be sent to the FIRM hardware.
pub enum FirmCommand {
    /// Pings the device to check connectivity.
    Ping,
    /// Resets the device.
    Reset,
    /// Sets the data reporting rate in Hz.
    /// 
    /// # Arguments
    /// * `rate_hz` - The desired rate in Hertz (e.g., 10, 100).
    SetRate(u32),
}

impl FirmCommand {
    /// Serializes the command into a byte vector ready to be sent over serial.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // TODO: Implement actual protocol serialization
        // Example: [START_BYTE, CMD_ID, PAYLOAD..., CRC]
        match self {
            FirmCommand::Ping => {
                bytes.push(0x01); // Example ID
            },
            FirmCommand::Reset => {
                bytes.push(0x02);
            },
            FirmCommand::SetRate(rate) => {
                bytes.push(0x03);
                bytes.extend_from_slice(&rate.to_le_bytes());
            },
        }
        
        bytes
    }
}
