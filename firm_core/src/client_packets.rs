use alloc::vec::Vec;

use crate::{
    firm_packets::*,
    utils::{crc16_ccitt, str_to_bytes},
};
use crate::constants::command_constants::*;
use crate::constants::data_parser_constants::{
    MOCK_SENSOR_PACKET_START_BYTES,
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
    /// Start marker bytes for mock sensor packets.
    pub header: [u8; 2],
    /// Payload length in bytes.
    pub len: u16,
    /// Payload bytes (telemetry payload).
    pub payload: Vec<u8>,
    /// CRC computed over `[header][len][payload]`.
    pub crc: u16,
}

impl FIRMMockPacket {
    /// Creates a new mock packet from a raw payload.
    ///
    /// Assumes the payload is already correct for the mock sensor packet format.
    pub fn new(payload: Vec<u8>) -> Self {
        let len = payload.len() as u16;

        // Compute CRC over: header + length + payload
        let mut crc_input = Vec::with_capacity(2 + 2 + payload.len());
        crc_input.extend_from_slice(&MOCK_SENSOR_PACKET_START_BYTES);
        crc_input.extend_from_slice(&len.to_le_bytes());
        crc_input.extend_from_slice(&payload);
        let crc = crc16_ccitt(&crc_input);

        Self {
            header: MOCK_SENSOR_PACKET_START_BYTES,
            len,
            payload,
            crc,
        }
    }

    /// Serializes the mock packet into bytes ready to be written to the serial stream.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + 2 + self.payload.len() + 2);
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&self.len.to_le_bytes());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&self.crc.to_le_bytes());
        out
    }

    /// Parses a framed mock sensor packet from raw bytes. This is just used for testing.
    ///
    /// Expected wire format: `[header(2)][len(u16 LE)][payload(len)][crc(u16 LE)]`.
    /// Returns `None` if the header doesn't match, the length is inconsistent, or CRC fails.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 2 + 2 + 2 {
            return None;
        }

        let header: [u8; 2] = bytes[0..2].try_into().ok()?;
        if header != MOCK_SENSOR_PACKET_START_BYTES {
            return None;
        }

        let len = u16::from_le_bytes(bytes[2..4].try_into().ok()?);
        let expected_total = 2usize + 2 + (len as usize) + 2;
        if bytes.len() != expected_total {
            return None;
        }

        let payload_start = 4;
        let payload_end = payload_start + (len as usize);
        let payload = bytes[payload_start..payload_end].to_vec();
        let crc = u16::from_le_bytes(bytes[payload_end..payload_end + 2].try_into().ok()?);

        let computed = Self::compute_crc(&header, len, &payload);
        if computed != crc {
            return None;
        }

        Some(Self {
            header,
            len,
            payload,
            crc,
        })
    }

    fn compute_crc(header: &[u8; 2], len: u16, payload: &[u8]) -> u16 {
        let mut crc_input = Vec::with_capacity(2 + 2 + payload.len());
        crc_input.extend_from_slice(header);
        crc_input.extend_from_slice(&len.to_le_bytes());
        crc_input.extend_from_slice(payload);
        crc16_ccitt(&crc_input)
    }
}

#[cfg(test)]
mod tests {
    use super::{FIRMCommandPacket, FIRMMockPacket};
    use crate::constants::command_constants::*;
    use crate::constants::data_parser_constants::MOCK_SENSOR_PACKET_START_BYTES;
    use crate::firm_packets::{DeviceConfig, DeviceProtocol};
    use crate::utils::{crc16_ccitt, str_to_bytes};

    fn crc_from_command_bytes(bytes: &[u8]) -> u16 {
        u16::from_le_bytes(bytes[COMMAND_LENGTH - CRC_LENGTH..COMMAND_LENGTH].try_into().unwrap())
    }

    fn calculate_command_crc(bytes: &[u8]) -> u16 {
        crc16_ccitt(&bytes[..COMMAND_LENGTH - CRC_LENGTH])
    }

    fn assert_command_built_correct(command_packet: &[u8]) {
        assert_eq!(command_packet.len(), COMMAND_LENGTH);
        assert_eq!(&command_packet[0..2], &COMMAND_START_BYTES);
        assert_eq!(
            crc_from_command_bytes(command_packet),
            calculate_command_crc(command_packet)
        );
    }

    #[test]
    fn test_firm_command_packet_to_bytes_get_device_info() {
        let command_packet = FIRMCommandPacket::GetDeviceInfo.to_bytes();
        assert_command_built_correct(&command_packet);
        assert_eq!(command_packet[2], DEVICE_INFO_MARKER);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_get_device_config() {
        let command_packet = FIRMCommandPacket::GetDeviceConfig.to_bytes();
        assert_command_built_correct(&command_packet);
        assert_eq!(command_packet[2], DEVICE_CONFIG_MARKER);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_cancel() {
        let command_packet = FIRMCommandPacket::Cancel.to_bytes();
        assert_command_built_correct(&command_packet);
        assert_eq!(command_packet[2], CANCEL_MARKER);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_reboot() {
        let command_packet = FIRMCommandPacket::Reboot.to_bytes();
        assert_command_built_correct(&command_packet);
        assert_eq!(command_packet[2], REBOOT_MARKER);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_mock() {
        let command_packet = FIRMCommandPacket::Mock.to_bytes();
        assert_command_built_correct(&command_packet);
        assert_eq!(command_packet[2], MOCK_MARKER);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_set_device_config() {
        let config = DeviceConfig {
            name: "FIRM".to_string(),
            frequency: 50,
            protocol: DeviceProtocol::UART,
        };

        let command_packet = FIRMCommandPacket::SetDeviceConfig(config.clone()).to_bytes();
        assert_command_built_correct(&command_packet);

        assert_eq!(command_packet[2], SET_DEVICE_CONFIG_MARKER);

        let name_start = 3;
        let name_end = name_start + DEVICE_NAME_LENGTH;
        let freq_start = name_end;
        let freq_end = freq_start + FREQUENCY_LENGTH;
        let protocol_idx = freq_end;

        let expected_name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
        assert_eq!(&command_packet[name_start..name_end], &expected_name_bytes);

        let freq = u16::from_le_bytes(command_packet[freq_start..freq_end].try_into().unwrap());
        assert_eq!(freq, config.frequency);

        assert_eq!(command_packet[protocol_idx], 0x02);
    }

    #[test]
    fn test_firm_mock_packet_new() {
        let payload = vec![1u8, 2, 3];
        let packet = FIRMMockPacket::new(payload.clone());
        assert_eq!(packet.header, MOCK_SENSOR_PACKET_START_BYTES);
        assert_eq!(packet.len, payload.len() as u16);
        assert_eq!(packet.payload, payload);
    }

    #[test]
    fn test_firm_mock_packet_to_bytes() {
        let payload: Vec<u8> = vec![0x10, 0x20, 0x30, 0x40, 0x50];
        let packet = FIRMMockPacket::new(payload);
        let bytes = packet.to_bytes();
        assert_eq!(&bytes[0..2], &MOCK_SENSOR_PACKET_START_BYTES);
        assert_eq!(u16::from_le_bytes(bytes[2..4].try_into().unwrap()), packet.len);
        assert_eq!(u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()), packet.crc);
        assert_eq!(&bytes[4..bytes.len() - 2], &packet.payload);
    }
}
