use crate::crc::crc16_ccitt;
use serde::Serialize;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Start byte sequence for packet identification. This is in little-endian format.
const START_BYTES: [u8; 2] = [0x5a, 0xa5];

/// Size of the packet header in bytes.
const HEADER_SIZE: usize = core::mem::size_of_val(&START_BYTES);

/// Size of the length field in bytes.
const LENGTH_FIELD_SIZE: usize = 2;

/// Size of the padding buffer in bytes.
const PADDING_SIZE: usize = 4;

/// Length of the payload in bytes.
const PAYLOAD_LENGTH: usize = 56;

/// Size of the CRC field in bytes.
const CRC_SIZE: usize = 2;

/// Total size of a full data packet in bytes.
const FULL_PACKET_SIZE: usize =
    HEADER_SIZE + LENGTH_FIELD_SIZE + PADDING_SIZE + PAYLOAD_LENGTH + CRC_SIZE;

/// Standard gravity in m/s².
const GRAVITY_METERS_PER_SECONDS_SQUARED: f32 = 9.80665;

/// Represents a decoded FIRM telemetry packet with converted physical units.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(get_all, freelist = 20, frozen))]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct FIRMPacket {
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub timestamp_seconds: f64,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_x_meters_per_s2: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_y_meters_per_s2: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_z_meters_per_s2: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_x_radians_per_s: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_y_radians_per_s: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_z_radians_per_s: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub pressure_pascals: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub temperature_celsius: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_x_microteslas: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_y_microteslas: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_z_microteslas: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub pressure_altitude_meters: f32,
}

impl FIRMPacket {
    /// Constructs a `FIRMPacket` from a raw payload byte slice.
    /// 
    /// # Arguments
    /// 
    /// - `bytes` (`&[u8]`) - Raw payload bytes in the FIRM on-wire format.
    /// 
    /// # Returns
    /// 
    /// - `Self` - Parsed packet with converted sensor and timestamp values.
    fn from_bytes(bytes: &[u8]) -> Self {
        /// Reads 4 bytes from `bytes` at `idx` and advances the index.
        /// 
        /// # Arguments
        /// 
        /// - `bytes` (`&[u8]`) - Source byte slice to read from.
        /// - `idx` (`&mut usize`) - Current read offset, updated in place.
        /// 
        /// # Returns
        /// 
        /// - `[u8; 4]` - Four-byte chunk starting at the current index.
        fn four_bytes(bytes: &[u8], idx: &mut usize) -> [u8; 4] {
            let res = [
                bytes[*idx],
                bytes[*idx + 1],
                bytes[*idx + 2],
                bytes[*idx + 3],
            ];
            *idx += 4;
            res
        }

        let mut idx = 0;

        // Scalars.
        let temperature_celsius: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let pressure_pascals: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Accelerometer values originally in g, converted to m/s².
        let accel_x_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;
        let accel_y_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;
        let accel_z_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;

        // Gyroscope values in rad/s.
        let gyro_x_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let gyro_y_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let gyro_z_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Magnetometer values in µT.
        let mag_x_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let mag_y_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let mag_z_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Skip padding before timestamp.
        idx += 4;
        let timestamp_seconds: f64 = f64::from_le_bytes([
            bytes[idx],
            bytes[idx + 1],
            bytes[idx + 2],
            bytes[idx + 3],
            bytes[idx + 4],
            bytes[idx + 5],
            bytes[idx + 6],
            bytes[idx + 7],
        ]);

        Self {
            timestamp_seconds,
            accel_x_meters_per_s2,
            accel_y_meters_per_s2,
            accel_z_meters_per_s2,
            gyro_x_radians_per_s,
            gyro_y_radians_per_s,
            gyro_z_radians_per_s,
            pressure_pascals,
            temperature_celsius,
            mag_x_microteslas,
            mag_y_microteslas,
            mag_z_microteslas,
            pressure_altitude_meters: 0.0,
        }
    }
}

/// Streaming parser that accumulates serial bytes and produces `FIRMPacket` values.
pub struct SerialParser {
    /// Rolling buffer of unprocessed serial bytes.
    serial_bytes: Vec<u8>,
    /// Queue of fully decoded packets ready to be consumed.
    serial_packets: VecDeque<FIRMPacket>,
}

impl SerialParser {
    /// Creates a new empty `SerialParser`.
    /// 
    /// # Arguments
    /// 
    /// - *None* - The parser starts with no buffered bytes or queued packets.
    /// 
    /// # Returns
    /// 
    /// - `Self` - A new parser instance with empty internal state.
    pub fn new() -> Self {
        SerialParser {
            serial_bytes: Vec::new(),
            serial_packets: VecDeque::new(),
        }
    }

    /// Feeds new bytes into the parser and queues any fully decoded packets.
    /// 
    /// # Arguments
    /// 
    /// - `bytes` (`&[u8]`) - Incoming raw bytes read from the FIRM serial stream.
    /// 
    /// # Returns
    /// 
    /// - `()` - No direct return; parsed packets are stored internally for `get_packet`.
    pub fn parse_bytes(&mut self, bytes: &[u8]) {
        // Append new bytes onto the rolling buffer.
        self.serial_bytes.extend(bytes);

        let mut pos = 0usize;
        // Scan through the buffer looking for start bytes and valid packets.
        while pos < self.serial_bytes.len().saturating_sub(1) {
            if self.serial_bytes[pos] != START_BYTES[0]
                || self.serial_bytes[pos + 1] != START_BYTES[1]
            {
                pos += 1;
                continue;
            }

            let header_start = pos;

            // Ensure we have enough bytes buffered to contain a full packet.
            if header_start + FULL_PACKET_SIZE > self.serial_bytes.len() {
                break;
            }

            let length_start = header_start + HEADER_SIZE;

            let length_bytes = &self.serial_bytes[length_start..length_start + LENGTH_FIELD_SIZE];
            let length = u16::from_le_bytes([length_bytes[0], length_bytes[1]]);

            // Reject packets with an unexpected payload length.
            if length as usize != PAYLOAD_LENGTH {
                pos = length_start;
                continue;
            }

            let payload_start = length_start + LENGTH_FIELD_SIZE + PADDING_SIZE;
            let crc_start = payload_start + length as usize;

            // Compute CRC over header + length + padding + payload.
            let data_to_crc = &self.serial_bytes[header_start..crc_start];
            let data_crc = crc16_ccitt(data_to_crc);
            let crc_value = u16::from_le_bytes([
                self.serial_bytes[crc_start],
                self.serial_bytes[crc_start + 1],
            ]);

            // Verify CRC before trusting the payload.
            if data_crc != crc_value {
                pos = length_start;
                continue;
            }

            let payload_slice = &self.serial_bytes[payload_start..payload_start + length as usize];

            let packet = FIRMPacket::from_bytes(payload_slice);

            // Queue the parsed packet for downstream consumers.
            self.serial_packets.push_back(packet);

            // Advance past this full packet and continue scanning.
            pos = crc_start + CRC_SIZE;
        }

        // Drop all bytes that were processed; keep only the tail for next call.
        self.serial_bytes = self.serial_bytes[pos..].to_vec();
    }

    /// Pops the next parsed packet from the internal queue, if available.
    /// 
    /// # Arguments
    /// 
    /// - *None* - Operates on the parser's existing queued packets.
    /// 
    /// # Returns
    /// 
    /// - `Option<FIRMPacket>` - `Some(packet)` if a packet is available, otherwise `None`.
    pub fn get_packet(&mut self) -> Option<FIRMPacket> {
        self.serial_packets.pop_front()
    }
}
