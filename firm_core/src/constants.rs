pub mod packet_constants {
    /// Header is stored as two little-endian u16s on the wire.
    pub const HEADER_SIZE: usize = 2;
    pub const IDENTIFIER_SIZE: usize = 2;
    pub const LENGTH_SIZE: usize = 4;
    pub const CRC_SIZE: usize = 2;

    pub const MIN_PACKET_SIZE: usize = HEADER_SIZE + IDENTIFIER_SIZE + LENGTH_SIZE + CRC_SIZE;

    /// First u16 in the framed header.
    #[repr(u16)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PacketHeader {
        Data = 0xA55A,
        Response = 0x5AA5,
        MockSensor = 0x6BB6,
        Command = 0xB66B,
    }

    impl PacketHeader {
        pub const fn as_u16(self) -> u16 {
            self as u16
        }

        pub const fn from_u16(v: u16) -> Option<Self> {
            match v {
                x if x == PacketHeader::Data as u16 => Some(PacketHeader::Data),
                x if x == PacketHeader::Response as u16 => Some(PacketHeader::Response),
                x if x == PacketHeader::MockSensor as u16 => Some(PacketHeader::MockSensor),
                x if x == PacketHeader::Command as u16 => Some(PacketHeader::Command),
                _ => None,
            }
        }
    }
}

pub mod command_constants {
    use crate::framed_packet::FrameError;

    #[repr(u16)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FIRMCommand {
        GetDeviceInfo = 0x0001,
        GetDeviceConfig = 0x0002,
        SetDeviceConfig = 0x0003,
        Reboot = 0x0004,
        Mock = 0x0005,
        Cancel = 0x00FF,
    }

    impl FIRMCommand {
        pub const fn marker(self) -> u16 {
            self as u16
        }

        pub const fn from_marker(marker: u16) -> Result<Self, FrameError> {
            if marker == FIRMCommand::GetDeviceInfo as u16 {
                return Ok(FIRMCommand::GetDeviceInfo);
            }
            if marker == FIRMCommand::GetDeviceConfig as u16 {
                return Ok(FIRMCommand::GetDeviceConfig);
            }
            if marker == FIRMCommand::SetDeviceConfig as u16 {
                return Ok(FIRMCommand::SetDeviceConfig);
            }
            if marker == FIRMCommand::Reboot as u16 {
                return Ok(FIRMCommand::Reboot);
            }
            if marker == FIRMCommand::Mock as u16 {
                return Ok(FIRMCommand::Mock);
            }
            if marker == FIRMCommand::Cancel as u16 {
                return Ok(FIRMCommand::Cancel);
            }

            Err(FrameError::UnknownMarker(marker))
        }
    }

    pub const COMMAND_LENGTH: usize = 64;
    pub const CRC_LENGTH: usize = 2;
    pub const DEVICE_NAME_LENGTH: usize = 32;
    pub const DEVICE_ID_LENGTH: usize = 8;
    pub const FIRMWARE_VERSION_LENGTH: usize = 8;
    pub const FREQUENCY_LENGTH: usize = 2;
}

pub mod mock_constants {
    use std::time::Duration;
    /// Mock sensor packet type identifier stored in the second u16 header field.
    #[repr(u16)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FIRMMockPacketType {
        HeaderPacket = b'H' as u16,
        BarometerPacket = b'B' as u16,
        IMUPacket = b'I' as u16,
        MagnetometerPacket = b'M' as u16,
    }

    impl FIRMMockPacketType {
        pub const fn as_u16(self) -> u16 {
            self as u16
        }

        // TODO: make Result
        pub const fn from_u16(v: u16) -> Option<Self> {
            match v {
                x if x == b'H' as u16 => Some(FIRMMockPacketType::HeaderPacket),
                x if x == b'B' as u16 => Some(FIRMMockPacketType::BarometerPacket),
                x if x == b'I' as u16 => Some(FIRMMockPacketType::IMUPacket),
                x if x == b'M' as u16 => Some(FIRMMockPacketType::MagnetometerPacket),
                _ => None,
            }
        }
    }

    pub const BMP581_ID: u8 = b'B';
    pub const ICM45686_ID: u8 = b'I';
    pub const MMC5983MA_ID: u8 = b'M';

    // The length of the payloads not including the 3 byte timestamp
    pub const BMP581_SIZE: usize = 6;
    pub const ICM45686_SIZE: usize = 15;
    pub const MMC5983MA_SIZE: usize = 7;

    pub const LOG_FILE_EOF_PADDING_LENGTH: usize = 20;
    pub const MOCK_PACKET_TIMESTAMP_SIZE: usize = 3;

    pub const HEADER_SIZE_TEXT: usize = 14; // "FIRM LOG vx.x"
    pub const HEADER_UID_SIZE: usize = 8;
    pub const HEADER_DEVICE_NAME_LEN: usize = 32;
    pub const HEADER_COMM_SIZE: usize = 4; // 1 byte usb, 1 byte uart, 1 byte spi, 1 byte i2c
    pub const HEADER_FIRMWARE_VERSION_SIZE: usize = 8; // "vX.X.X.X"
    pub const HEADER_FREQUENCY_SIZE: usize = 2;
    pub const HEADER_PADDING_SIZE: usize = 2;
    pub const HEADER_CAL_SIZE: usize = (3 + 9) * 3 * 4; // (offsets + 3x3 matrix) * 3 sensors * 4 bytes
    pub const HEADER_NUM_SCALE_FACTOR_SIZE: usize = 5 * 4; // 5 floats

    pub const HEADER_TOTAL_SIZE: usize = HEADER_SIZE_TEXT
        + HEADER_UID_SIZE
        + HEADER_DEVICE_NAME_LEN
        + HEADER_COMM_SIZE
        + HEADER_FIRMWARE_VERSION_SIZE
        + HEADER_FREQUENCY_SIZE
        + HEADER_PADDING_SIZE
        + HEADER_CAL_SIZE
        + HEADER_NUM_SCALE_FACTOR_SIZE;

    pub const HEADER_PARSE_DELAY: Duration = Duration::from_millis(100);
}
