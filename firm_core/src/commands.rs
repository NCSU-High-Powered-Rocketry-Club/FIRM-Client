use std::mem;

use alloc::vec::Vec;
use serde::{Serialize, Deserialize};

use crate::utils::{bytes_to_str, crc16_ccitt, str_to_bytes};

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

const DEVICE_NAME_LENGTH: usize = 32;
const DEVICE_ID_LENGTH: usize = mem::size_of::<u64>();
const FIRMWARE_VERSION_LENGTH: usize = 8;
const PORT_LENGTH: usize = 16;
const FREQUENCY_LENGTH: usize = mem::size_of::<u16>();

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeviceProtocol {
    USB,
    UART,
    I2C,
    SPI,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub firmware_version: String, // Max 8 characters
    pub port: String, // Max 16 characters
    pub id: u64,
}

/// Represents the configuration settings of the FIRM device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub name: String, // Max 32 characters
    pub frequency: u16,
    pub protocol: DeviceProtocol,
}

/// Represents the status of a calibration process.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibrationStatus {
    /// True if the calibration is complete, false otherwise.
    pub calibration_complete: bool,
    /// Progress percentage of the calibration process (0-100).
    pub progress_percentage: u8,
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
        let mut command_bytes = Vec::with_capacity(COMMAND_LENGTH as usize);

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
                // [SET_DEVICE_CONFIG_MARKER][NAME (32 bytes)][FREQUENCY (2 bytes)][PROTOCOL (1 byte)]]
                command_bytes.push(SET_DEVICE_CONFIG_MARKER);
                // Add the name
                let name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
                command_bytes.extend_from_slice(&name_bytes);
                // Add the frequency
                command_bytes.extend_from_slice(&config.frequency.to_le_bytes());
                // Add the protocol
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
        let data_crc = crc16_ccitt(&command_bytes);
        command_bytes.extend_from_slice(&data_crc.to_le_bytes());
        
        command_bytes
    }
}

/// Represents a response received from the FIRM hardware after sending a command.
/// It can contain anything from a simple status to actual data requested by the command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FIRMResponse {
    GetDeviceInfo(DeviceInfo),
    GetDeviceConfig(DeviceConfig),
    SetDeviceConfig(bool),
    RunIMUCalibration(CalibrationStatus),
    RunMagnetometerCalibration(CalibrationStatus),
    Error(String),
}


/// Parses incoming bytes from FIRM into command responses. Basically how
/// commands work is you send a command to FIRM, then it sends back a response
/// which you parse using this parser. This response can contain data
/// requested by the command. To see the format of each response, look at the
/// match statement below.
impl FIRMResponse {
    /// Constructs a `FIRMResponse` from a raw payload byte slice. The format of this
    /// payload byte slice is as follows: [COMMAND MARKER][DATA...]
    /// 
    /// # Arguments
    /// 
    /// - `data` (`&[u8]`) - The raw payload byte slice to parse into a `FIRMResponse`.
    /// 
    /// # Returns
    /// 
    /// - `Self` - The constructed `FIRMResponse` from the given byte slice.
    pub fn from_bytes(data: &[u8]) -> Self {
        match data[0] {
            DEVICE_INFO_MARKER => {
                // Parse device info response, which is in the following format:
                // [DEVICE_INFO_MARKER][ID (8 bytes)][FIRMWARE_VERSION (8 bytes)][PORT (16 bytes)]
                let id_bytes = &data[1..1 + DEVICE_ID_LENGTH];
                let firmware_version_bytes = &data[1 + DEVICE_ID_LENGTH..1 + DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH];
                let port_bytes = &data[1 + DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH..1 + DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH + PORT_LENGTH];
                let id = u64::from_le_bytes(id_bytes.try_into().unwrap());
                let firmware_version = bytes_to_str(firmware_version_bytes);
                let port = bytes_to_str(port_bytes);

                let info = DeviceInfo {
                    id,
                    firmware_version,
                    port,
                };

                FIRMResponse::GetDeviceInfo(info)
            },
            DEVICE_CONFIG_MARKER => {
                // Parse GetDeviceConfig response, which is in the following format:
                // [DEVICE_CONFIG_MARKER][NAME (32 bytes)][FREQUENCY (2 bytes)][PROTOCOL (1 byte)]
                let name_bytes: [u8; DEVICE_NAME_LENGTH] = data[1..DEVICE_NAME_LENGTH + 1].try_into().unwrap();
                let name = bytes_to_str(&name_bytes);
                let frequency = u16::from_le_bytes(data[DEVICE_NAME_LENGTH + 1..DEVICE_NAME_LENGTH + 1 + FREQUENCY_LENGTH].try_into().unwrap());
                let protocol = match data[DEVICE_NAME_LENGTH + 1 + FREQUENCY_LENGTH] {
                    0x01 => DeviceProtocol::USB,
                    0x02 => DeviceProtocol::UART,
                    0x03 => DeviceProtocol::I2C,
                    0x04 => DeviceProtocol::SPI,
                    _ => DeviceProtocol::USB, // Default
                };

                let config = DeviceConfig {
                    frequency,
                    protocol,
                    name,
                };

                FIRMResponse::GetDeviceConfig(config)
            },
            SET_DEVICE_CONFIG_MARKER => {
                // Parsing the SetDeviceConfig acknowledgement response is just checking if the
                // first byte after the marker is 1 (success) or 0 (failure).
                let success = data[1] == 1;
                FIRMResponse::SetDeviceConfig(success)
            },
            RUN_IMU_CALIBRATION_MARKER => {
                // Parse the IMU calibration status response, which is in the following format:
                // [RUN_IMU_CALIBRATION_MARKER][CALIBRATION_COMPLETE (1 byte)][PROGRESS_PERCENTAGE (1 byte)]
                let calibration_complete = data[1] == 1;
                let progress_percentage = data[2];
                FIRMResponse::RunIMUCalibration(CalibrationStatus {
                    calibration_complete,
                    progress_percentage,
                })
            },
            RUN_MAGNETOMETER_CALIBRATION_MARKER => {
                // Parse the Magnetometer calibration status response, which is in the following format:
                // [RUN_MAGNETOMETER_CALIBRATION_MARKER][CALIBRATION_COMPLETE (1 byte)][PROGRESS_PERCENTAGE (1 byte)]
                let calibration_complete = data[1] == 1;
                let progress_percentage = data[2];
                FIRMResponse::RunMagnetometerCalibration(CalibrationStatus {
                    calibration_complete,
                    progress_percentage,
                })
            },
            _ => FIRMResponse::Error("Unknown response marker".to_string()),
        }
    }
}
