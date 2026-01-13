use alloc::vec::Vec;

use crate::{
    firm_packets::*,
    framed_packet::{FramedPacket, Framed},
    utils::str_to_bytes,
};
use crate::constants::command_constants::{self, FIRMCommand, DEVICE_NAME_LENGTH, FREQUENCY_LENGTH, PADDING_BYTE};
use crate::constants::data_parser_constants::{
    MOCK_SENSOR_PACKET_START_BYTES,
};

/// Header format: `[COMMAND_START_BYTES(2)][padding(1)][command_marker(1)]`.
pub struct FIRMCommandPacket {
    command_type: FIRMCommand,
    frame: FramedPacket,
}

impl FIRMCommandPacket {
    pub fn new(command_type: FIRMCommand, payload: Vec<u8>) -> Self {
        let header = [
            command_constants::COMMAND_START_BYTES[0],
            command_constants::COMMAND_START_BYTES[1],
            command_constants::PADDING_BYTE,
            command_type.marker(),
        ];
        Self {
            command_type,
            frame: FramedPacket::new(header, payload),
        }
    }

    pub fn command_type(&self) -> FIRMCommand {
        self.command_type
    }

    pub fn header(&self) -> &[u8; 4] {
        self.frame.header()
    }

    pub fn payload(&self) -> &[u8] {
        self.frame.payload()
    }

    pub fn len(&self) -> u32 {
        self.frame.len()
    }

    pub fn crc(&self) -> u16 {
        self.frame.crc()
    }

    pub fn get_device_info() -> Self {
        Self::new(FIRMCommand::GetDeviceInfo, Vec::new())
    }

    pub fn get_device_config() -> Self {
        Self::new(FIRMCommand::GetDeviceConfig, Vec::new())
    }

    pub fn cancel() -> Self {
        Self::new(FIRMCommand::Cancel, Vec::new())
    }

    pub fn reboot() -> Self {
        Self::new(FIRMCommand::Reboot, Vec::new())
    }

    pub fn mock() -> Self {
        Self::new(FIRMCommand::Mock, Vec::new())
    }

    pub fn set_device_config(config: DeviceConfig) -> Self {
        let mut payload = Vec::with_capacity(DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1);
        let name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
        payload.extend_from_slice(&name_bytes);
        payload.extend_from_slice(&config.frequency.to_le_bytes());

        let protocol_byte = match config.protocol {
            DeviceProtocol::USB => 0x01,
            DeviceProtocol::UART => 0x02,
            DeviceProtocol::I2C => 0x03,
            DeviceProtocol::SPI => 0x04,
        };
        payload.push(protocol_byte);

        Self::new(FIRMCommand::SetDeviceConfig, payload)
    }
}

impl Framed for FIRMCommandPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let marker = frame.header()[3];
        let command_type = FIRMCommand::from_marker(marker)
            .ok_or(crate::framed_packet::FrameError::UnknownMarker(marker))?;
        Ok(Self { command_type, frame })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FIRMMockPacketType {
    Header,
    B,
    I,
    M,
}

impl FIRMMockPacketType {
    fn as_byte(self) -> u8 {
        match self {
            FIRMMockPacketType::Header => b'H',
            FIRMMockPacketType::B => b'B',
            FIRMMockPacketType::I => b'I',
            FIRMMockPacketType::M => b'M',
        }
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'H' => Some(FIRMMockPacketType::Header),
            b'B' => Some(FIRMMockPacketType::B),
            b'I' => Some(FIRMMockPacketType::I),
            b'M' => Some(FIRMMockPacketType::M),
            _ => None,
        }
    }
}

pub struct FIRMMockPacket {
    packet_type: FIRMMockPacketType,
    frame: FramedPacket,
}

impl FIRMMockPacket {
    /// Creates a new mock packet.
    ///
    /// Packet format:
    /// `[header(2)][pad(1)][type(1)][len(u32 LE)][payload(len)][crc(u16 LE)]`.
    pub fn new(packet_type: FIRMMockPacketType, payload: Vec<u8>) -> Self {
        let header4 = [
            MOCK_SENSOR_PACKET_START_BYTES[0],
            MOCK_SENSOR_PACKET_START_BYTES[1],
            PADDING_BYTE,
            packet_type.as_byte(),
        ];
        Self {
            packet_type,
            frame: FramedPacket::new(header4, payload),
        }
    }

    pub fn packet_type(&self) -> FIRMMockPacketType {
        self.packet_type
    }

    pub fn header(&self) -> &[u8; 4] {
        self.frame.header()
    }

    pub fn payload(&self) -> &[u8] {
        self.frame.payload()
    }

    pub fn len(&self) -> u32 {
        self.frame.len()
    }

    pub fn crc(&self) -> u16 {
        self.frame.crc()
    }

    /// Serializes the mock packet into bytes ready to be written to the serial stream.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.frame.to_bytes()
    }

    /// Parses a framed mock sensor packet from raw bytes. This is just used for testing.
    ///
    /// Expected wire format: `[header(2)][pad(1)][type(1)][len(u32 LE)][payload(len)][crc(u16 LE)]`.
    /// Returns `None` if the header doesn't match, the length is inconsistent, or CRC fails.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let frame = FramedPacket::from_bytes(bytes).ok()?;
        let header = frame.header();
        if header[0..2] != MOCK_SENSOR_PACKET_START_BYTES {
            return None;
        }
        let packet_type = FIRMMockPacketType::from_byte(header[3])?;
        Some(Self { packet_type, frame })
    }
}

impl Framed for FIRMMockPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let packet_type = FIRMMockPacketType::from_byte(frame.header()[3]).unwrap_or(FIRMMockPacketType::Header);
        Ok(Self { packet_type, frame })
    }
}

#[cfg(test)]
mod tests {
    use super::{FIRMCommandPacket, FIRMMockPacket, FIRMMockPacketType};
    use crate::constants::command_constants::*;
    use crate::constants::data_parser_constants::MOCK_SENSOR_PACKET_START_BYTES;
    use crate::firm_packets::{DeviceConfig, DeviceProtocol};
    use crate::framed_packet::Framed;
    use crate::utils::{crc16_ccitt, str_to_bytes};

    fn crc_from_bytes(bytes: &[u8]) -> u16 {
        u16::from_le_bytes(bytes[bytes.len() - CRC_LENGTH..].try_into().unwrap())
    }

    fn calculate_crc(bytes: &[u8]) -> u16 {
        crc16_ccitt(&bytes[..bytes.len() - CRC_LENGTH])
    }

    fn assert_common_packet_invariants(bytes: &[u8], expected_start: &[u8; 2]) {
        assert_eq!(&bytes[0..2], expected_start);
        assert_eq!(bytes[2], PADDING_BYTE);
        assert_eq!(crc_from_bytes(bytes), calculate_crc(bytes));
    }

    #[test]
    fn test_firm_command_packet_to_bytes_get_device_info() {
        let command_packet = FIRMCommandPacket::get_device_info().to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);
        assert_eq!(command_packet[3], DEVICE_INFO_MARKER);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + 0 + 2);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_get_device_config() {
        let command_packet = FIRMCommandPacket::get_device_config().to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);
        assert_eq!(command_packet[3], DEVICE_CONFIG_MARKER);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + 0 + 2);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_cancel() {
        let command_packet = FIRMCommandPacket::cancel().to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);
        assert_eq!(command_packet[3], CANCEL_MARKER);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + 0 + 2);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_reboot() {
        let command_packet = FIRMCommandPacket::reboot().to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);
        assert_eq!(command_packet[3], REBOOT_MARKER);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + 0 + 2);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_mock() {
        let command_packet = FIRMCommandPacket::mock().to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);
        assert_eq!(command_packet[3], MOCK_MARKER);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + 0 + 2);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_set_device_config() {
        let config = DeviceConfig {
            name: "FIRM".to_string(),
            frequency: 50,
            protocol: DeviceProtocol::UART,
        };

        let command_packet = FIRMCommandPacket::set_device_config(config.clone()).to_bytes();
        assert_common_packet_invariants(&command_packet, &COMMAND_START_BYTES);

        assert_eq!(command_packet[3], SET_DEVICE_CONFIG_MARKER);

        let payload_len = u32::from_le_bytes(command_packet[4..8].try_into().unwrap()) as usize;
        assert_eq!(payload_len, DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1);
        assert_eq!(command_packet.len(), 2 + 1 + 1 + 4 + payload_len + 2);

        let payload_start = 8;
        let name_start = payload_start;
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
        let packet = FIRMMockPacket::new(FIRMMockPacketType::B, payload.clone());
        assert_eq!(&packet.header()[0..2], &MOCK_SENSOR_PACKET_START_BYTES);
        assert_eq!(packet.packet_type(), FIRMMockPacketType::B);
        assert_eq!(packet.len(), payload.len() as u32);
        assert_eq!(packet.payload(), payload.as_slice());
    }

    #[test]
    fn test_firm_mock_packet_to_bytes() {
        let payload: Vec<u8> = vec![0x10, 0x20, 0x30, 0x40, 0x50];
        let packet = FIRMMockPacket::new(FIRMMockPacketType::I, payload);
        let bytes = packet.to_bytes();
        assert_eq!(&bytes[0..2], &MOCK_SENSOR_PACKET_START_BYTES);
        assert_eq!(bytes[2], PADDING_BYTE);
        assert_eq!(bytes[3], b'I');
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), packet.len());
        assert_eq!(u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()), packet.crc());
        assert_eq!(&bytes[8..bytes.len() - 2], packet.payload());
        assert_eq!(crc_from_bytes(&bytes), calculate_crc(&bytes));
    }

    #[test]
    fn test_firm_mock_packet_roundtrip_from_bytes() {
        let payload = vec![9u8, 8, 7];
        let packet = FIRMMockPacket::new(FIRMMockPacketType::Header, payload);
        let bytes = packet.to_bytes();
        let parsed = FIRMMockPacket::from_bytes(&bytes).unwrap();
        assert_eq!(&parsed.header()[0..2], &MOCK_SENSOR_PACKET_START_BYTES);
        assert_eq!(parsed.packet_type(), FIRMMockPacketType::Header);
        assert_eq!(parsed.len() as usize, parsed.payload().len());
        assert_eq!(parsed.payload(), packet.payload());
        assert_eq!(parsed.crc(), packet.crc());
    }
}
