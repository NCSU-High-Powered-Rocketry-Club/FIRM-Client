pub mod packet_constants {
    /// Maximum allowed payload size in bytes.
    pub const HEADER_SIZE: usize = 4;
    pub const LENGTH_SIZE: usize = 4;
    pub const CRC_SIZE: usize = 2;
}

pub mod data_parser_constants {
    /// Start byte sequence for packet identification. This is in little-endian format.
    pub const DATA_PACKET_START_BYTES: [u8; 2] = [0x5A, 0xA5];
    /// Start byte sequence for response identification. This is in little-endian format.
    pub const RESPONSE_PACKET_START_BYTES: [u8; 2] = [0xA5, 0x5A];

    pub const MOCK_SENSOR_PACKET_START_BYTES: [u8; 2] = [0xB6, 0x6B];

    /// Message IDs (u16) in little-endian byte order.
    pub const MSGID_DATA_PACKET: u16 = 0xA55A;
    pub const MSGID_RESPONSE_PACKET: u16 = 0x5AA5;

    /// Padding bytes used in the 4-byte header.
    pub const PADDING_BYTE: u8 = 0x00;

    /// Packet format: [header(4)][len(u32 le)(4)][payload(len)][crc(u16 le)(2)]
    pub const HEADER_SIZE: usize = 4;
    pub const LENGTH_FIELD_SIZE: usize = 4;
    pub const CRC_SIZE: usize = 2;
    pub const MIN_PACKET_SIZE: usize = HEADER_SIZE + LENGTH_FIELD_SIZE + CRC_SIZE;
}

pub mod command_constants {
    pub const DEVICE_INFO_MARKER: u8 = 0x01;
    pub const DEVICE_CONFIG_MARKER: u8 = 0x02;
    pub const SET_DEVICE_CONFIG_MARKER: u8 = 0x03;
    pub const REBOOT_MARKER: u8 = 0x04;
    pub const MOCK_MARKER: u8 = 0x05;
    pub const CANCEL_MARKER: u8 = 0xFF;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FIRMCommand {
        GetDeviceInfo,
        GetDeviceConfig,
        SetDeviceConfig,
        Reboot,
        Mock,
        Cancel,
    }

    impl FIRMCommand {
        pub const fn marker(self) -> u8 {
            match self {
                FIRMCommand::GetDeviceInfo => DEVICE_INFO_MARKER,
                FIRMCommand::GetDeviceConfig => DEVICE_CONFIG_MARKER,
                FIRMCommand::SetDeviceConfig => SET_DEVICE_CONFIG_MARKER,
                FIRMCommand::Reboot => REBOOT_MARKER,
                FIRMCommand::Mock => MOCK_MARKER,
                FIRMCommand::Cancel => CANCEL_MARKER,
            }
        }

        pub const fn from_marker(marker: u8) -> Option<Self> {
            match marker {
                DEVICE_INFO_MARKER => Some(FIRMCommand::GetDeviceInfo),
                DEVICE_CONFIG_MARKER => Some(FIRMCommand::GetDeviceConfig),
                SET_DEVICE_CONFIG_MARKER => Some(FIRMCommand::SetDeviceConfig),
                REBOOT_MARKER => Some(FIRMCommand::Reboot),
                MOCK_MARKER => Some(FIRMCommand::Mock),
                CANCEL_MARKER => Some(FIRMCommand::Cancel),
                _ => None,
            }
        }
    }

    pub const COMMAND_LENGTH: usize = 64;
    pub const CRC_LENGTH: usize = 2;
    pub const DEVICE_NAME_LENGTH: usize = 32;
    pub const DEVICE_ID_LENGTH: usize = 8;
    pub const FIRMWARE_VERSION_LENGTH: usize = 8;
    pub const FREQUENCY_LENGTH: usize = 2;
    /// Protocol value: COMMAND_PACKET = 0x6BB6.
    pub const COMMAND_START_BYTES: [u8; 2] = [0x6B, 0xB6];
    pub const PADDING_BYTE: u8 = 0x00;
}

pub mod mock_constants {
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
