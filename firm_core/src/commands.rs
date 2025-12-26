use alloc::vec::Vec;

use crate::firm_packets::{DeviceConfig, DeviceProtocol};
use crate::utils::{crc16_ccitt, str_to_bytes};

const COMMAND_START_BYTES: [u8; 2] = [0x55, 0xAA];
const PADDING_BYTE: u8 = 0x00;

const DEVICE_INFO_MARKER: u8 = 0x01;
const DEVICE_CONFIG_MARKER: u8 = 0x02;
const SET_DEVICE_CONFIG_MARKER: u8 = 0x03;
const RUN_IMU_CALIBRATION_MARKER: u8 = 0x04;
const RUN_MAGNETOMETER_CALIBRATION_MARKER: u8 = 0x05;
const REBOOT_MARKER: u8 = 0x06;
const CANCEL_MARKER: u8 = 0x07;

const COMMAND_LENGTH: u8 = 64;
const CRC_LENGTH: usize = 2;
const DEVICE_NAME_LENGTH: usize = 32;

/// Represents a command that can be sent to the FIRM hardware.
pub enum FIRMCommand {
    GetDeviceInfo,
    GetDeviceConfig,
    SetDeviceConfig(DeviceConfig),
    RunIMUCalibration,
    RunMagnetometerCalibration,
    Cancel,
    Reboot,
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
            FIRMCommand::Cancel => {
                command_bytes.push(CANCEL_MARKER);
            },
            FIRMCommand::Reboot => {
                command_bytes.push(REBOOT_MARKER);
            },
        }

        // Now add padding bytes to reach COMMAND_LENGTH - CRC size
        while command_bytes.len() < (COMMAND_LENGTH as usize - CRC_LENGTH) {
            command_bytes.push(PADDING_BYTE);
        }

        // Finally, compute and append CRC
        let data_crc = crc16_ccitt(&command_bytes);
        command_bytes.extend_from_slice(&data_crc.to_le_bytes());
        
        command_bytes
    }
}


