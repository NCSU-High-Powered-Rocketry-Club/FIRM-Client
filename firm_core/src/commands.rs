use alloc::vec::Vec;

pub enum DeviceProtocol {
    USB,
    UART,
    I2C,
    SPI,
}

pub struct DeviceConfig {
    pub name: String,
    pub frequency: u16,
    pub protocol: DeviceProtocol,
}

/// Represents a command that can be sent to the FIRM hardware.
pub enum FIRMCommand {
    /// Gets info about the device including name, ID, firmware version, and port.
    GetDeviceInfo,
    GetDeviceConfig,
    SetDeviceConfig(DeviceConfig),
    RunIMUCalibration,
    RunMagnetometerCalibration,
    DownloadLogFile(u32),
    Reboot,
}

impl FIRMCommand {
    /// Serializes the command into a byte vector ready to be sent over serial.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        match self {
            FIRMCommand::GetDeviceInfo => {
                bytes.push(0x01);
            },
            FIRMCommand::GetDeviceConfig => {
                bytes.push(0x02);
            },
            FIRMCommand::SetDeviceConfig(config) => {
                bytes.push(0x03);
                // Serialize DeviceConfig fields
                bytes.extend_from_slice(&config.frequency.to_le_bytes());
                match config.protocol {
                    DeviceProtocol::USB => bytes.push(0x01),
                    DeviceProtocol::UART => bytes.push(0x02),
                    DeviceProtocol::I2C => bytes.push(0x03),
                    DeviceProtocol::SPI => bytes.push(0x04),
                }
            },
            FIRMCommand::RunIMUCalibration => {
                bytes.push(0x04);
            },
            FIRMCommand::RunMagnetometerCalibration => {
                bytes.push(0x05);
            },
            FIRMCommand::DownloadLogFile(file_id) => {
                bytes.push(0x06);
                bytes.extend_from_slice(&file_id.to_le_bytes());
            },
            FIRMCommand::Reboot => {
                bytes.push(0x07);
            },
        }
        
        bytes
    }
}

pub enum FIRMResponse {
    DeviceInfo {
        name: String,
        id: u32,
        firmware_version: String,
        port: String,
    },
    DeviceConfig(DeviceConfig),
    Acknowledgement,
    Error(String),
}

/// Parses incoming bytes from FIRM into command responses. Basically how
/// commands work is you send a command to FIRM, then it sends back a response
/// which you parse using this parser. This response can contain data
/// requested by the command.
impl FIRMResponse {
    pub fn from_bytes(data: &[u8]) -> Self {
        // TODO: implement this
        FIRMResponse::Acknowledgement
    }
}