use alloc::vec::Vec;

use crate::{
    firm_packets::*,
    framed_packet::{FramedPacket, Framed},
    utils::str_to_bytes,
};
use crate::constants::command_constants::{FIRMCommand, DEVICE_NAME_LENGTH, FREQUENCY_LENGTH};
use crate::constants::mock_constants::{FIRMMockPacketType};
use crate::constants::packet_constants::PacketHeader;

pub struct FIRMCommandPacket {
    command_type: FIRMCommand,
    frame: FramedPacket,
}

impl FIRMCommandPacket {
    pub fn new(command_type: FIRMCommand, payload: Vec<u8>) -> Self {
        let header = PacketHeader::Command;
        let identifier = command_type as u16;
        Self {
            command_type,
            frame: FramedPacket::new(header, identifier, payload),
        }
    }

    pub fn command_type(&self) -> FIRMCommand {
        self.command_type
    }

    pub fn build_get_device_info_command() -> Self {
        Self::new(FIRMCommand::GetDeviceInfo, Vec::new())
    }

    pub fn build_get_device_config_command() -> Self {
        Self::new(FIRMCommand::GetDeviceConfig, Vec::new())
    }

    pub fn build_cancel_command() -> Self {
        Self::new(FIRMCommand::Cancel, Vec::new())
    }

    pub fn build_reboot_command() -> Self {
        Self::new(FIRMCommand::Reboot, Vec::new())
    }

    pub fn build_mock_command() -> Self {
        Self::new(FIRMCommand::Mock, Vec::new())
    }

    pub fn build_set_device_config_command(config: DeviceConfig) -> Self {
        let mut payload = Vec::with_capacity(DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1);
        let name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
        payload.extend_from_slice(&name_bytes);
        payload.extend_from_slice(&config.frequency.to_le_bytes());

        payload.push(config.protocol as u8);

        Self::new(FIRMCommand::SetDeviceConfig, payload)
    }
}

impl Framed for FIRMCommandPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    /// Parses a framed command packet from raw bytes. This method is just for testing.
    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let marker = frame.identifier();
        let command_type = FIRMCommand::from_marker(marker)?;
        Ok(Self { command_type, frame })
    }
}

pub struct FIRMMockPacket {
    packet_type: FIRMMockPacketType,
    frame: FramedPacket,
}

impl FIRMMockPacket {
    pub fn new(packet_type: FIRMMockPacketType, payload: Vec<u8>) -> Self {
        let header = PacketHeader::MockSensor;
        let identifier = packet_type as u16;
        Self {
            packet_type,
            frame: FramedPacket::new(header, identifier, payload),
        }
    }

    pub fn packet_type(&self) -> FIRMMockPacketType {
        self.packet_type
    }
}

impl Framed for FIRMMockPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    /// Parses a framed mock sensor packet from raw bytes. This method is just for testing.
    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let packet_type = FIRMMockPacketType::from_u16(frame.identifier())
            .unwrap_or(FIRMMockPacketType::HeaderPacket);
        Ok(Self { packet_type, frame })
    }
}

#[cfg(test)]
mod tests {
    use super::{FIRMCommandPacket, FIRMMockPacket};
    use crate::constants::command_constants::{CRC_LENGTH, DEVICE_NAME_LENGTH, FREQUENCY_LENGTH, FIRMCommand};
    use crate::constants::mock_constants::{FIRMMockPacketType};
    use crate::constants::packet_constants::PacketHeader;
    use crate::firm_packets::{DeviceConfig, DeviceProtocol};
    use crate::framed_packet::Framed;
    use crate::utils::{crc16_ccitt, str_to_bytes};

    fn crc_from_bytes(bytes: &[u8]) -> u16 {
        u16::from_le_bytes(bytes[bytes.len() - CRC_LENGTH..].try_into().unwrap())
    }

    fn calculate_crc(bytes: &[u8]) -> u16 {
        crc16_ccitt(&bytes[..bytes.len() - CRC_LENGTH])
    }

    fn header_from_bytes(bytes: &[u8]) -> u16 {
        u16::from_le_bytes(bytes[0..2].try_into().unwrap())
    }

    fn identifier_from_bytes(bytes: &[u8]) -> u16 {
        u16::from_le_bytes(bytes[2..4].try_into().unwrap())
    }

    fn assert_common_packet_invariants(bytes: &[u8]) {
        assert_eq!(crc_from_bytes(bytes), calculate_crc(bytes));
    }

    fn assert_zero_payload_command(make: fn() -> FIRMCommandPacket, expected_marker: u16) {
        let command_packet = make().to_bytes();
        assert_common_packet_invariants(&command_packet);
        assert_eq!(header_from_bytes(&command_packet), PacketHeader::Command as u16);
        assert_eq!(identifier_from_bytes(&command_packet), expected_marker);
        assert_eq!(u32::from_le_bytes(command_packet[4..8].try_into().unwrap()), 0);
        assert_eq!(command_packet.len(), 4 + 4 + 0 + CRC_LENGTH);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_zero_payload_commands() {
        let cases: &[(u16, fn() -> FIRMCommandPacket)] = &[
            (FIRMCommand::GetDeviceInfo as u16, FIRMCommandPacket::build_get_device_info_command),
            (FIRMCommand::GetDeviceConfig as u16, FIRMCommandPacket::build_get_device_config_command),
            (FIRMCommand::Cancel as u16, FIRMCommandPacket::build_cancel_command),
            (FIRMCommand::Reboot as u16, FIRMCommandPacket::build_reboot_command),
            (FIRMCommand::Mock as u16, FIRMCommandPacket::build_mock_command),
        ];

        for (marker, make) in cases {
            assert_zero_payload_command(*make, *marker);
        }
    }

    #[test]
    fn test_firm_command_packet_to_bytes_set_device_config() {
        let config = DeviceConfig {
            name: "FIRM".to_string(),
            frequency: 50,
            protocol: DeviceProtocol::UART,
        };

        let command_packet = FIRMCommandPacket::build_set_device_config_command(config.clone()).to_bytes();
        assert_common_packet_invariants(&command_packet);

        assert_eq!(header_from_bytes(&command_packet), PacketHeader::Command as u16);

        assert_eq!(identifier_from_bytes(&command_packet), FIRMCommand::SetDeviceConfig as u16);

        let payload_len = u32::from_le_bytes(command_packet[4..8].try_into().unwrap()) as usize;
        assert_eq!(payload_len, DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1);
        assert_eq!(command_packet.len(), 4 + 4 + payload_len + CRC_LENGTH);

        let payload = &command_packet[8..8 + payload_len];
        let (got_name_bytes, rest) = payload.split_at(DEVICE_NAME_LENGTH);
        let (got_freq_bytes, got_protocol_bytes) = rest.split_at(FREQUENCY_LENGTH);

        let expected_name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
        assert_eq!(got_name_bytes, &expected_name_bytes);

        let freq = u16::from_le_bytes(got_freq_bytes.try_into().unwrap());
        assert_eq!(freq, config.frequency);
        assert_eq!(got_protocol_bytes, &[0x02]);
    }

    #[test]
    fn test_firm_mock_packet_new() {
        let payload = vec![1u8, 2, 3];
        let packet = FIRMMockPacket::new(FIRMMockPacketType::BarometerPacket, payload.clone());
        assert_eq!(packet.header(), PacketHeader::MockSensor);
        assert_eq!(packet.packet_type(), FIRMMockPacketType::BarometerPacket);
        assert_eq!(packet.len(), payload.len() as u32);
        assert_eq!(packet.payload(), payload.as_slice());
    }

    #[test]
    fn test_firm_mock_packet_to_bytes() {
        let payload: Vec<u8> = vec![0x10, 0x20, 0x30, 0x40, 0x50];
        let packet = FIRMMockPacket::new(FIRMMockPacketType::IMUPacket, payload);
        let bytes = packet.to_bytes();
        assert_eq!(header_from_bytes(&bytes), PacketHeader::MockSensor.as_u16());
        assert_eq!(identifier_from_bytes(&bytes), b'I' as u16);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), packet.len());
        assert_eq!(u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()), packet.crc());
        assert_eq!(&bytes[8..bytes.len() - 2], packet.payload());
        assert_eq!(crc_from_bytes(&bytes), calculate_crc(&bytes));
    }

    #[test]
    fn test_firm_mock_packet_roundtrip_from_bytes() {
        let payload = vec![9u8, 8, 7];
        let packet = FIRMMockPacket::new(FIRMMockPacketType::HeaderPacket, payload);
        let bytes = packet.to_bytes();
        let parsed = FIRMMockPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header(), PacketHeader::MockSensor);
        assert_eq!(parsed.packet_type(), FIRMMockPacketType::HeaderPacket);
        assert_eq!(parsed.len() as usize, parsed.payload().len());
        assert_eq!(parsed.payload(), packet.payload());
        assert_eq!(parsed.crc(), packet.crc());
    }
}
