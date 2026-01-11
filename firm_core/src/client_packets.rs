use alloc::vec::Vec;

use crate::{
    firm_packets::*,
    utils::{crc16_ccitt, str_to_bytes},
};
use crate::constants::command_constants::*;
use crate::constants::data_parser_constants::{
    DATA_PACKET_START_BYTES, FIRST_PADDING_SIZE, SECOND_PADDING_SIZE,
};

/// Represents a command that can be sent to the FIRM hardware.
pub enum FIRMCommandPacket {
    GetDeviceInfo,
    GetDeviceConfig,
    SetDeviceConfig(DeviceConfig),
    Mock,
    Cancel,
    Reboot,
}

// TODO: rewrite this like the mock packets to be a struct and stuff
impl FIRMCommandPacket {
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
            FIRMCommandPacket::GetDeviceInfo => {
                command_bytes.push(DEVICE_INFO_MARKER);
            }
            FIRMCommandPacket::GetDeviceConfig => {
                command_bytes.push(DEVICE_CONFIG_MARKER);
            }
            FIRMCommandPacket::SetDeviceConfig(config) => {
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
            FIRMCommandPacket::Cancel => {
                command_bytes.push(CANCEL_MARKER);
            }
            FIRMCommandPacket::Reboot => {
                command_bytes.push(REBOOT_MARKER);
            }
            FIRMCommandPacket::Mock => {
                command_bytes.push(MOCK_MARKER);
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

pub struct FIRMMockPacket {
    /// Start marker bytes (typically `DATA_PACKET_START_BYTES`).
    pub header: [u8; 2],
    /// Payload length in bytes.
    pub len: u16,
    /// Payload bytes (telemetry payload).
    pub payload: Vec<u8>,
    /// CRC computed over `[header][len][first padding][payload]`.
    pub crc: u16,
}

impl FIRMMockPacket {
    /// Creates a new mock packet from a raw payload.
    ///
    /// Assumes the payload is already correct for the FIRM telemetry format.
    pub fn new(payload: Vec<u8>) -> Self {
        let len = payload.len() as u16;

        // Compute CRC over: header + length + first padding + payload
        let mut crc_input = Vec::with_capacity(2 + 2 + FIRST_PADDING_SIZE + payload.len());
        crc_input.extend_from_slice(&DATA_PACKET_START_BYTES);
        crc_input.extend_from_slice(&len.to_le_bytes());
        crc_input.extend_from_slice(&[0u8; FIRST_PADDING_SIZE]);
        crc_input.extend_from_slice(&payload);
        let crc = crc16_ccitt(&crc_input);

        Self {
            header: DATA_PACKET_START_BYTES,
            len,
            payload,
            crc,
        }
    }

    /// Serializes the mock packet into bytes ready to be written to the serial stream.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + 2 + FIRST_PADDING_SIZE + self.payload.len() + 2);
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&self.len.to_le_bytes());
        out.extend_from_slice(&[0u8; FIRST_PADDING_SIZE]);
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&self.crc.to_le_bytes());
        out.extend_from_slice(&[0u8; SECOND_PADDING_SIZE]);
        out
    }
}
