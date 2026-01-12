pub mod data_parser_constants {
    /// Start byte sequence for packet identification. This is in little-endian format.
    pub const DATA_PACKET_START_BYTES: [u8; 2] = [0x5A, 0xA5];
    /// Start byte sequence for response identification. This is in little-endian format.
    pub const RESPONSE_PACKET_START_BYTES: [u8; 2] = [0xA5, 0x5A];
    /// Start byte sequence for mock sensor packets (B/I/M records) sent to the device in mock mode.
    ///
    /// This is distinct from normal data/response packets.
    pub const MOCK_SENSOR_PACKET_START_BYTES: [u8; 2] = [0x4D, 0x4B];
    /// Size of the packet header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of_val(&DATA_PACKET_START_BYTES);
    /// Size of the length field in bytes.
    pub const LENGTH_FIELD_SIZE: usize = 2;
    /// Size of the padding before the payload in bytes.
    pub const FIRST_PADDING_SIZE: usize = 4;
    /// Length of the payload in bytes.
    pub const PAYLOAD_LENGTH: usize = 120;
    /// Size of the CRC field in bytes.
    pub const CRC_SIZE: usize = 2;
    /// Size of the padding after the CRC in bytes.
    pub const SECOND_PADDING_SIZE: usize = 6;
    /// Total size of a full data packet in bytes.
    pub const FULL_PACKET_SIZE: usize =
        HEADER_SIZE + LENGTH_FIELD_SIZE + FIRST_PADDING_SIZE + PAYLOAD_LENGTH + CRC_SIZE + SECOND_PADDING_SIZE;
}

pub mod command_constants {
    pub const DEVICE_INFO_MARKER: u8 = 0x01;
    pub const DEVICE_CONFIG_MARKER: u8 = 0x02;
    pub const SET_DEVICE_CONFIG_MARKER: u8 = 0x03;
    pub const REBOOT_MARKER: u8 = 0x04;
    pub const MOCK_MARKER: u8 = 0x05;
    pub const CANCEL_MARKER: u8 = 0xFF;
    pub const COMMAND_LENGTH: usize = 64;
    pub const CRC_LENGTH: usize = 2;
    pub const DEVICE_NAME_LENGTH: usize = 32;
    pub const DEVICE_ID_LENGTH: usize = 8;
    pub const FIRMWARE_VERSION_LENGTH: usize = 8;
    pub const FREQUENCY_LENGTH: usize = 2;
    pub const COMMAND_START_BYTES: [u8; 2] = [0x55, 0xAA];
    pub const PADDING_BYTE: u8 = 0x00;
}