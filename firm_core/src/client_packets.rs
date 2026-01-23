use heapless::Vec;

use crate::constants::command::{DEVICE_NAME_LENGTH, FIRMCommand, FREQUENCY_LENGTH};
use crate::constants::log_parsing::FIRMLogPacketType;
use crate::constants::packet::PacketHeader;
use crate::{
    firm_packets::*,
    framed_packet::{Framed, FramedPacket, FrameError},
    utils::str_to_bytes,
};

pub struct FIRMCommandPacket {
    command_type: FIRMCommand,
    frame: FramedPacket,
}

impl FIRMCommandPacket {
    pub fn new(command_type: FIRMCommand, payload: &[u8]) -> Result<Self, FrameError> {
        let header = PacketHeader::Command;
        let identifier = command_type as u16;
        Ok(Self {
            command_type,
            frame: FramedPacket::new(header, identifier, payload)?,
        })
    }

    pub fn command_type(&self) -> FIRMCommand {
        self.command_type
    }

    pub fn build_get_device_info_command() -> Result<Self, FrameError> {
        Self::new(FIRMCommand::GetDeviceInfo, &[])
    }

    pub fn build_get_device_config_command() -> Result<Self, FrameError> {
        Self::new(FIRMCommand::GetDeviceConfig, &[])
    }

    pub fn build_cancel_command() -> Result<Self, FrameError> {
        Self::new(FIRMCommand::Cancel, &[])
    }

    pub fn build_reboot_command() -> Result<Self, FrameError> {
        Self::new(FIRMCommand::Reboot, &[])
    }

    pub fn build_mock_command() -> Result<Self, FrameError> {
        Self::new(FIRMCommand::Mock, &[])
    }

    pub fn build_set_device_config_command(config: DeviceConfig) -> Result<Self, FrameError> {
        let mut payload: Vec<u8, { DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1 }> = Vec::new();
        let name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(&config.name);
        payload.extend_from_slice(&name_bytes).ok();
        payload.extend_from_slice(&config.frequency.to_le_bytes()).ok();
        payload.push(config.protocol as u8).ok();

        Self::new(FIRMCommand::SetDeviceConfig, &payload)
    }
}

impl Framed for FIRMCommandPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    /// Parses a framed command packet from raw bytes. This method is just for testing.
    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let identifier = frame.identifier();
        let command_type = FIRMCommand::from_u16(identifier)?;
        Ok(Self {
            command_type,
            frame,
        })
    }
}

pub struct FIRMLogPacket {
    packet_type: FIRMLogPacketType,
    frame: FramedPacket,
}

impl FIRMLogPacket {
    pub fn new(packet_type: FIRMLogPacketType, payload: &[u8]) -> Result<Self, FrameError> {
        let header = PacketHeader::LogSensor;
        let identifier = packet_type as u16;
        Ok(Self {
            packet_type,
            frame: FramedPacket::new(header, identifier, payload)?,
        })
    }

    pub fn packet_type(&self) -> FIRMLogPacketType {
        self.packet_type
    }
}

impl Framed for FIRMLogPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    /// Parses a framed mock sensor packet from raw bytes. This method is just for testing.
    fn from_bytes(bytes: &[u8]) -> Result<Self, crate::framed_packet::FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let packet_type = FIRMLogPacketType::from_u16(frame.identifier())
            .unwrap_or(FIRMLogPacketType::HeaderPacket);
        Ok(Self { packet_type, frame })
    }
}

#[cfg(test)]
mod tests {
    use super::{FIRMCommandPacket, FIRMLogPacket};
    use crate::constants::command::{
        CRC_LENGTH, DEVICE_NAME_LENGTH, FIRMCommand, FREQUENCY_LENGTH,
    };
    use crate::constants::log_parsing::FIRMLogPacketType;
    use crate::constants::packet::PacketHeader;
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

    fn assert_zero_payload_command(make: fn() -> Result<FIRMCommandPacket, crate::framed_packet::FrameError>, expected_identifier: u16) {
        let command_packet = make().unwrap().to_bytes();
        assert_common_packet_invariants(&command_packet);
        assert_eq!(
            header_from_bytes(&command_packet),
            PacketHeader::Command as u16
        );
        assert_eq!(identifier_from_bytes(&command_packet), expected_identifier);
        assert_eq!(
            u32::from_le_bytes(command_packet[4..8].try_into().unwrap()),
            0
        );
        assert_eq!(command_packet.len(), 4 + 4 + 0 + CRC_LENGTH);
    }

    #[test]
    fn test_firm_command_packet_to_bytes_zero_payload_commands() {
        let cases: &[(u16, fn() -> Result<FIRMCommandPacket, crate::framed_packet::FrameError>)] = &[
            (
                FIRMCommand::GetDeviceInfo as u16,
                FIRMCommandPacket::build_get_device_info_command,
            ),
            (
                FIRMCommand::GetDeviceConfig as u16,
                FIRMCommandPacket::build_get_device_config_command,
            ),
            (
                FIRMCommand::Cancel as u16,
                FIRMCommandPacket::build_cancel_command,
            ),
            (
                FIRMCommand::Reboot as u16,
                FIRMCommandPacket::build_reboot_command,
            ),
            (
                FIRMCommand::Mock as u16,
                FIRMCommandPacket::build_mock_command,
            ),
        ];

        for (identifier, make) in cases {
            assert_zero_payload_command(*make, *identifier);
        }
    }

    #[test]
    fn test_firm_command_packet_to_bytes_set_device_config() {
        let mut config_name = heapless::String::new();
        let _ = config_name.push_str("FIRM");
        
        let config = DeviceConfig {
            name: config_name,
            frequency: 50,
            protocol: DeviceProtocol::UART,
        };

        let command_packet =
            FIRMCommandPacket::build_set_device_config_command(config.clone()).unwrap().to_bytes();
        assert_common_packet_invariants(&command_packet);

        assert_eq!(
            header_from_bytes(&command_packet),
            PacketHeader::Command as u16
        );

        assert_eq!(
            identifier_from_bytes(&command_packet),
            FIRMCommand::SetDeviceConfig as u16
        );

        let payload_len = u32::from_le_bytes(command_packet[4..8].try_into().unwrap()) as usize;
        assert_eq!(payload_len, DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1);
        assert_eq!(command_packet.len(), 4 + 4 + payload_len + CRC_LENGTH);

        let payload = &command_packet[8..8 + payload_len];
        let (got_name_bytes, rest) = payload.split_at(DEVICE_NAME_LENGTH);
        let (got_freq_bytes, got_protocol_bytes) = rest.split_at(FREQUENCY_LENGTH);

        let expected_name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>(config.name.as_str());
        assert_eq!(got_name_bytes, &expected_name_bytes);

        let freq = u16::from_le_bytes(got_freq_bytes.try_into().unwrap());
        assert_eq!(freq, config.frequency);
        assert_eq!(got_protocol_bytes, &[0x02]);
    }

    #[test]
    fn test_firm_mock_packet_new() {
        let payload = [1u8, 2, 3];
        let packet = FIRMLogPacket::new(FIRMLogPacketType::BarometerPacket, &payload).unwrap();
        assert_eq!(packet.header(), PacketHeader::LogSensor);
        assert_eq!(packet.packet_type(), FIRMLogPacketType::BarometerPacket);
        assert_eq!(packet.len(), payload.len() as u32);
        assert_eq!(packet.payload(), payload.as_slice());
    }

    #[test]
    fn test_firm_mock_packet_to_bytes() {
        let payload = [0x10u8, 0x20, 0x30, 0x40, 0x50];
        let packet = FIRMLogPacket::new(FIRMLogPacketType::IMUPacket, &payload).unwrap();
        let bytes = packet.to_bytes();
        assert_eq!(header_from_bytes(&bytes), PacketHeader::LogSensor.as_u16());
        assert_eq!(identifier_from_bytes(&bytes), b'I' as u16);
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            packet.len()
        );
        assert_eq!(
            u16::from_le_bytes(bytes[bytes.len() - 2..].try_into().unwrap()),
            packet.crc()
        );
        assert_eq!(&bytes[8..bytes.len() - 2], packet.payload());
        assert_eq!(crc_from_bytes(&bytes), calculate_crc(&bytes));
    }

    #[test]
    fn test_firm_mock_packet_roundtrip_from_bytes() {
        let payload = [9u8, 8, 7];
        let packet = FIRMLogPacket::new(FIRMLogPacketType::HeaderPacket, &payload).unwrap();
        let bytes = packet.to_bytes();
        let parsed = FIRMLogPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header(), PacketHeader::LogSensor);
        assert_eq!(parsed.packet_type(), FIRMLogPacketType::HeaderPacket);
        assert_eq!(parsed.len() as usize, parsed.payload().len());
        assert_eq!(parsed.payload(), packet.payload());
        assert_eq!(parsed.crc(), packet.crc());
    }
}
