use alloc::vec::Vec;

use crate::{
    firm_packets::*,
    utils::{crc16_ccitt, str_to_bytes},
};
use crate::constants::command_constants::*;

/// Represents a command that can be sent to the FIRM hardware.
pub enum FIRMCommand {
    GetDeviceInfo,
    GetDeviceConfig,
    SetDeviceConfig(DeviceConfig),
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
    /// - `&self` The command to be serialized.
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
            }
            FIRMCommand::GetDeviceConfig => {
                command_bytes.push(DEVICE_CONFIG_MARKER);
            }
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
            }
            FIRMCommand::Cancel => {
                command_bytes.push(CANCEL_MARKER);
            }
            FIRMCommand::Reboot => {
                command_bytes.push(REBOOT_MARKER);
            }
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
