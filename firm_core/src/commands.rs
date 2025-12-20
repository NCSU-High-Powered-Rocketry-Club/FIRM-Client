use alloc::vec::Vec;

use crate::utils::crc16_ccitt;

const COMMAND_LENGTH: u8 = 64;

const COMMAND_START_BYTES: [u8; 2] = [0x55, 0xAA];

const DEVICE_INFO_MARKER: u8 = 0x01;
const DEVICE_CONFIG_MARKER: u8 = 0x02;
const SET_DEVICE_CONFIG_MARKER: u8 = 0x03;
const RUN_IMU_CALIBRATION_MARKER: u8 = 0x04;
const RUN_MAGNETOMETER_CALIBRATION_MARKER: u8 = 0x05;
const REBOOT_MARKER: u8 = 0x06;

const PADDING_BYTE: u8 = 0x00;

/// Size of the CRC field in bytes.
const CRC_SIZE: usize = 2;

pub enum DeviceProtocol {
    USB,
    UART,
    I2C,
    SPI,
}

pub struct DeviceConfig {
    pub frequency: u16,
    pub protocol: DeviceProtocol,
    pub name: [u8; 32],  // Fixed-size array for device name
}

/// Represents a command that can be sent to the FIRM hardware.
pub enum FIRMCommand {
    /// Gets info about the device including name, ID, firmware version, and port.
    GetDeviceInfo,
    GetDeviceConfig,
    SetDeviceConfig(DeviceConfig),
    RunIMUCalibration,
    RunMagnetometerCalibration,
    Reboot,
    // TODO: figure out how to implement log file downloads DownloadLogFile(u32),
}

impl FIRMCommand {
    /// Serializes the command into a byte vector ready to be sent over serial. This
    /// makes the command in the following format:
    /// [START_MARKER][COMMAND_PAYLOAD][PADDING][CRC]
    /// 
    /// # Arguments
    /// 
    /// - `&self` (`undefined`) - The command to be serialized.
    /// 
    /// # Returns
    /// 
    /// - `Vec<u8>` - The command serialized into bytes ready to be sent over serial.
    /// 
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut command_bytes = Vec::new();

        // Adds the start marker for the command
        command_bytes.extend_from_slice(&COMMAND_START_BYTES);

        // This match adds the payload for the command
        match self {
            FIRMCommand::GetDeviceInfo => {
                command_bytes.push(DEVICE_INFO_MARKER);
            },
            FIRMCommand::GetDeviceConfig => {
                command_bytes.push(DEVICE_CONFIG_MARKER);   
            },
            FIRMCommand::SetDeviceConfig(config) => {
                // The device config command payload is in the following format:
                // [SET_DEVICE_CONFIG_MARKER][FREQUENCY (2 bytes)][PROTOCOL (1 byte)][NAME (32 bytes)][PADDING]
                command_bytes.push(SET_DEVICE_CONFIG_MARKER);
                command_bytes.extend_from_slice(&config.frequency.to_le_bytes());
                match config.protocol {
                    DeviceProtocol::USB => command_bytes.push(0x01),
                    DeviceProtocol::UART => command_bytes.push(0x02),
                    DeviceProtocol::I2C => command_bytes.push(0x03),
                    DeviceProtocol::SPI => command_bytes.push(0x04),
                }
            },
            FIRMCommand::RunIMUCalibration => {
                command_bytes.push(RUN_IMU_CALIBRATION_MARKER);
            },
            FIRMCommand::RunMagnetometerCalibration => {
                command_bytes.push(RUN_MAGNETOMETER_CALIBRATION_MARKER);
            },
            FIRMCommand::Reboot => {
                command_bytes.push(REBOOT_MARKER);
            },
        }

        // Now add padding bytes to reach COMMAND_LENGTH - CRC size
        while command_bytes.len() < (COMMAND_LENGTH as usize - CRC_SIZE) {
            command_bytes.push(PADDING_BYTE);
        }

        // Finally, compute and append CRC
        let data_crc = crc16_ccitt(&command_bytes[COMMAND_START_BYTES.len()..]);
        command_bytes.extend_from_slice(&data_crc.to_le_bytes());
        
        command_bytes
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