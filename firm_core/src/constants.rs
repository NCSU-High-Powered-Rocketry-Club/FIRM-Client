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
        /// Equivalent wire bytes: [0x5A, 0xA5]
        Data = 0xA55A,
        /// Equivalent wire bytes: [0xA5, 0x5A]
        Response = 0x5AA5,
        /// Equivalent wire bytes: [0xB6, 0x6B]
        MockSensor = 0x6BB6,
        /// Equivalent wire bytes: [0x6B, 0xB6]
        Command = 0xB66B,
    }

    impl PacketHeader {
        pub const fn as_u16(self) -> u16 {
            self as u16
        }
    }
}

pub mod command_constants {
    use crate::framed_packet::FrameError;

    pub const DEVICE_INFO_MARKER: u16 = 0x0001;
    pub const DEVICE_CONFIG_MARKER: u16 = 0x0002;
    pub const SET_DEVICE_CONFIG_MARKER: u16 = 0x0003;
    pub const REBOOT_MARKER: u16 = 0x0004;
    pub const MOCK_MARKER: u16 = 0x0005;
    pub const CANCEL_MARKER: u16 = 0x00FF;

    #[repr(u16)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FIRMCommand {
        GetDeviceInfo = DEVICE_INFO_MARKER,
        GetDeviceConfig = DEVICE_CONFIG_MARKER,
        SetDeviceConfig = SET_DEVICE_CONFIG_MARKER,
        Reboot = REBOOT_MARKER,
        Mock = MOCK_MARKER,
        Cancel = CANCEL_MARKER,
    }

    impl FIRMCommand {
        pub const fn from_marker(marker: u16) -> Result<Self, FrameError> {
            match marker {
                DEVICE_INFO_MARKER => Ok(FIRMCommand::GetDeviceInfo),
                DEVICE_CONFIG_MARKER => Ok(FIRMCommand::GetDeviceConfig),
                SET_DEVICE_CONFIG_MARKER => Ok(FIRMCommand::SetDeviceConfig),
                REBOOT_MARKER => Ok(FIRMCommand::Reboot),
                MOCK_MARKER => Ok(FIRMCommand::Mock),
                CANCEL_MARKER => Ok(FIRMCommand::Cancel),
                _ => Err(FrameError::UnknownMarker(marker)),
            }
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
    use crate::constants::packet_constants::PacketHeader;

    pub const MOCK_SENSOR_PACKET_HEADER: u16 = PacketHeader::MockSensor as u16;

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

    pub const BMP581_SIZE: usize = 6;
    pub const ICM45686_SIZE: usize = 15;
    pub const MMC5983MA_SIZE: usize = 7;

    pub const HEADER_SIZE_TEXT: usize = 14; // "FIRM LOG vx.x"
    pub const HEADER_UID_SIZE: usize = 8;
    pub const HEADER_DEVICE_NAME_LEN: usize = 32;
    pub const HEADER_COMM_SIZE: usize = 2; // 1 byte usb, 1 byte uart
    pub const HEADER_CAL_SIZE: usize = (3 + 9) * 3 * 4; // (offsets + 3x3 matrix) * 3 sensors * 4 bytes
    pub const HEADER_NUM_SCALE_FACTORS: usize = 5; // 5 floats

    pub const HEADER_PADDING_SIZE: usize = (8
        - ((HEADER_UID_SIZE + HEADER_DEVICE_NAME_LEN + HEADER_COMM_SIZE) % 8))
        % 8;
    pub const HEADER_TOTAL_SIZE: usize = HEADER_SIZE_TEXT
        + HEADER_UID_SIZE
        + HEADER_DEVICE_NAME_LEN
        + HEADER_COMM_SIZE
        + HEADER_PADDING_SIZE
        + HEADER_CAL_SIZE
        + (HEADER_NUM_SCALE_FACTORS * 4);
}
