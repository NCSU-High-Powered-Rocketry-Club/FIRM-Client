use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::client_packets::FIRMMockPacket;
use crate::constants::data_parser_constants::PAYLOAD_LENGTH;

const BMP581_ID: u8 = b'B';
const ICM45686_ID: u8 = b'I';
const MMC5983MA_ID: u8 = b'M';

// Log packet sizes (not counting ID + timestamp).
const BMP581_SIZE: usize = 6;
const ICM45686_SIZE: usize = 15;
const MMC5983MA_SIZE: usize = 7;

// Header layout (matches the Python decoder you provided).
const HEADER_SIZE_TEXT: usize = 14; // "FIRM LOG vx.x"
const HEADER_UID_SIZE: usize = 8;
const HEADER_DEVICE_NAME_LEN: usize = 33;
const HEADER_COMM_SIZE: usize = 2; // 1 byte usb, 1 byte uart
const HEADER_CAL_SIZE: usize = (3 + 9) * 3 * 4; // (offsets + 3x3 matrix) * 3 sensors * 4 bytes
const HEADER_NUM_SCALE_FACTORS: usize = 5; // 5 floats

// This is constant for the current header sizes (43 bytes -> needs 5 bytes to reach 48).
const HEADER_PADDING_SIZE: usize = 5;
const HEADER_TOTAL_SIZE: usize = HEADER_SIZE_TEXT
    + HEADER_UID_SIZE
    + HEADER_DEVICE_NAME_LEN
    + HEADER_COMM_SIZE
    + HEADER_PADDING_SIZE
    + HEADER_CAL_SIZE
    + (HEADER_NUM_SCALE_FACTORS * 4);

/// Total size (in bytes) of the `.bin` log header expected by `MockParser::read_header()`.
pub const LOG_HEADER_SIZE: usize = HEADER_TOTAL_SIZE;

/// Streaming parser for FIRM log-file bytes that produces `FIRMMockPacket` values.
///
/// You feed it arbitrary-sized chunks from a `.bin` log file; it parses as many sensor records as
/// it can and enqueues `FIRMMockPacket`s. Each emitted mock packet contains a 120-byte telemetry
/// payload that mirrors the layout expected by `FIRMDataPacket::from_bytes`.
pub struct MockParser {
    /// Rolling buffer of unprocessed bytes.
    bytes: Vec<u8>,
    /// Queue of parsed mock packets.
    parsed_packets: VecDeque<FIRMMockPacket>,

    // Log header state.
    header_parsed: bool,
    // Scale factors from header.
    bmp_temp_sf: f32,
    bmp_pressure_sf: f32,
    icm_accel_sf: f32,
    icm_gyro_sf: f32,
    mmc_mag_sf: f32,

    // Timestamp state (clock-count based).
    timestamp_seconds: f64,
    last_clock_count: u32,

    // Whitespace repeat counter (used by the Python decoder to detect end-of-data).
    num_repeat_whitespace: usize,

    // Latest decoded sensor values (used to synthesize a full telemetry payload).
    temperature_celsius: f32,
    pressure_pascals: f32,
    accel_gs: [f32; 3],
    gyro_deg_s: [f32; 3],
    mag_ut: [f32; 3],

    /// Placeholder for future timestamp-based delay calculation.
    last_emitted_timestamp_seconds: Option<f64>,
}

impl MockParser {
    /// Creates a new empty `MockParser`.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            parsed_packets: VecDeque::new(),
            header_parsed: false,

            // Default scale factors (will be overwritten by header).
            bmp_temp_sf: 1.0,
            bmp_pressure_sf: 1.0,
            icm_accel_sf: 1.0,
            icm_gyro_sf: 1.0,
            mmc_mag_sf: 1.0,

            timestamp_seconds: 0.0,
            last_clock_count: 0,
            num_repeat_whitespace: 0,

            temperature_celsius: 0.0,
            pressure_pascals: 0.0,
            accel_gs: [0.0; 3],
            gyro_deg_s: [0.0; 3],
            mag_ut: [0.0; 3],

            last_emitted_timestamp_seconds: None,
        }
    }

    /// Reads the log header and initializes scale factors.
    ///
    /// The caller must pass a byte buffer that contains exactly the header bytes
    /// (as written by the device) and then call `parse_bytes()` with only the
    /// subsequent log record bytes.
    pub fn read_header(&mut self, header_bytes: &[u8]) {
        // No error handling per request: assume correct header length.
        debug_assert_eq!(header_bytes.len(), HEADER_TOTAL_SIZE);

        let scale_factors_start = HEADER_TOTAL_SIZE - (HEADER_NUM_SCALE_FACTORS * 4);
        let sf_bytes = &header_bytes[scale_factors_start..HEADER_TOTAL_SIZE];

        // Order follows your Python decoder:
        // bmp: [temp, pressure], icm: [accel, gyro], mmc: [mag]
        self.bmp_temp_sf = f32::from_le_bytes(sf_bytes[0..4].try_into().unwrap());
        self.bmp_pressure_sf = f32::from_le_bytes(sf_bytes[4..8].try_into().unwrap());
        self.icm_accel_sf = f32::from_le_bytes(sf_bytes[8..12].try_into().unwrap());
        self.icm_gyro_sf = f32::from_le_bytes(sf_bytes[12..16].try_into().unwrap());
        self.mmc_mag_sf = f32::from_le_bytes(sf_bytes[16..20].try_into().unwrap());

        // Reset streaming state for a fresh playback run.
        self.bytes.clear();
        self.parsed_packets.clear();
        self.timestamp_seconds = 0.0;
        self.last_clock_count = 0;
        self.num_repeat_whitespace = 0;
        self.last_emitted_timestamp_seconds = None;

        self.header_parsed = true;
    }

    /// Feeds a new chunk of bytes into the parser.
    ///
    /// Parses as many log records as possible and enqueues synthesized mock telemetry packets.
    pub fn parse_bytes(&mut self, chunk: &[u8]) {
        // Caller must load the header via `read_header()` first.
        if !self.header_parsed {
            return;
        }

        self.bytes.extend_from_slice(chunk);

        // Parse records.
        let mut pos = 0usize;
        while pos < self.bytes.len() {
            let record_start = pos;

            let id = self.bytes[pos];
            if id == 0 {
                // whitespace padding between records
                self.num_repeat_whitespace += 1;
                pos += 1;
                continue;
            }
            self.num_repeat_whitespace = 0;

            // Need timestamp.
            if pos + 1 + 3 > self.bytes.len() {
                pos = record_start;
                break;
            }

            pos += 1;
            let t = &self.bytes[pos..pos + 3];
            pos += 3;
            let clock_count = u32::from_be_bytes([0, t[0], t[1], t[2]]);

            // 24-bit overflow handling.
            let delta = if clock_count < self.last_clock_count {
                (clock_count + (1 << 24)) - self.last_clock_count
            } else {
                clock_count - self.last_clock_count
            };
            self.timestamp_seconds += (delta as f64) / 168e6;
            self.last_clock_count = clock_count;

            match id {
                BMP581_ID => {
                    if pos + BMP581_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let packet = &self.bytes[pos..pos + BMP581_SIZE];
                    pos += BMP581_SIZE;

                    let temp_raw = u32::from_le_bytes([packet[0], packet[1], packet[2], 0]);
                    let pressure_raw = u32::from_le_bytes([packet[3], packet[4], packet[5], 0]);

                    self.temperature_celsius = (temp_raw as f32) / self.bmp_temp_sf;
                    self.pressure_pascals = (pressure_raw as f32) / self.bmp_pressure_sf;
                }
                ICM45686_ID => {
                    if pos + ICM45686_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let packet = &self.bytes[pos..pos + ICM45686_SIZE];
                    pos += ICM45686_SIZE;

                    let accel_x_bin = ((packet[0] as u32) << 12)
                        | ((packet[1] as u32) << 4)
                        | ((packet[12] as u32) >> 4);
                    let accel_y_bin = ((packet[2] as u32) << 12)
                        | ((packet[3] as u32) << 4)
                        | ((packet[13] as u32) >> 4);
                    let accel_z_bin = ((packet[4] as u32) << 12)
                        | ((packet[5] as u32) << 4)
                        | ((packet[14] as u32) >> 4);

                    let gyro_x_bin = ((packet[6] as u32) << 12)
                        | ((packet[7] as u32) << 4)
                        | ((packet[12] as u32) & 0x0F);
                    let gyro_y_bin = ((packet[8] as u32) << 12)
                        | ((packet[9] as u32) << 4)
                        | ((packet[13] as u32) & 0x0F);
                    let gyro_z_bin = ((packet[10] as u32) << 12)
                        | ((packet[11] as u32) << 4)
                        | ((packet[14] as u32) & 0x0F);

                    fn twos_complement_20(v: u32) -> i32 {
                        let sign_bit = 1u32 << 19;
                        if (v & sign_bit) != 0 {
                            (v as i32) - (1i32 << 20)
                        } else {
                            v as i32
                        }
                    }

                    let ax = twos_complement_20(accel_x_bin) as f32 / self.icm_accel_sf;
                    let ay = twos_complement_20(accel_y_bin) as f32 / self.icm_accel_sf;
                    let az = twos_complement_20(accel_z_bin) as f32 / self.icm_accel_sf;

                    let gx = twos_complement_20(gyro_x_bin) as f32 / self.icm_gyro_sf;
                    let gy = twos_complement_20(gyro_y_bin) as f32 / self.icm_gyro_sf;
                    let gz = twos_complement_20(gyro_z_bin) as f32 / self.icm_gyro_sf;

                    self.accel_gs = [ax, ay, az];
                    self.gyro_deg_s = [gx, gy, gz];

                    // Emit a mock telemetry packet on each IMU sample.
                    let payload = self.build_telemetry_payload();
                    self.parsed_packets.push_back(FIRMMockPacket::new(payload));
                }
                MMC5983MA_ID => {
                    if pos + MMC5983MA_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let packet = &self.bytes[pos..pos + MMC5983MA_SIZE];
                    pos += MMC5983MA_SIZE;

                    let mag_x_bin = ((packet[0] as u32) << 10)
                        | ((packet[1] as u32) << 2)
                        | ((packet[6] as u32) >> 6);
                    let mag_y_bin = ((packet[2] as u32) << 10)
                        | ((packet[3] as u32) << 2)
                        | (((packet[6] as u32) & 0x30) >> 4);
                    let mag_z_bin = ((packet[4] as u32) << 10)
                        | ((packet[5] as u32) << 2)
                        | ((packet[6] as u32) & 0x0C);

                    self.mag_ut = [
                        (mag_x_bin as f32 - 131072.0) / self.mmc_mag_sf,
                        (mag_y_bin as f32 - 131072.0) / self.mmc_mag_sf,
                        (mag_z_bin as f32 - 131072.0) / self.mmc_mag_sf,
                    ];
                }
                _ => {
                    // Unknown/garbage byte; treat it like end-of-data.
                    break;
                }
            }
        }

        self.bytes = self.bytes[pos..].to_vec();
    }

    fn build_telemetry_payload(&self) -> Vec<u8> {
        // `FIRMDataPacket::from_bytes` reads 8 + 27*4 = 116 bytes; the protocol payload is 120.
        let mut out = Vec::with_capacity(PAYLOAD_LENGTH);

        out.extend_from_slice(&self.timestamp_seconds.to_le_bytes());

        // Core sensor fields.
        out.extend_from_slice(&self.temperature_celsius.to_le_bytes());
        out.extend_from_slice(&self.pressure_pascals.to_le_bytes());

        out.extend_from_slice(&self.accel_gs[0].to_le_bytes());
        out.extend_from_slice(&self.accel_gs[1].to_le_bytes());
        out.extend_from_slice(&self.accel_gs[2].to_le_bytes());

        out.extend_from_slice(&self.gyro_deg_s[0].to_le_bytes());
        out.extend_from_slice(&self.gyro_deg_s[1].to_le_bytes());
        out.extend_from_slice(&self.gyro_deg_s[2].to_le_bytes());

        out.extend_from_slice(&self.mag_ut[0].to_le_bytes());
        out.extend_from_slice(&self.mag_ut[1].to_le_bytes());
        out.extend_from_slice(&self.mag_ut[2].to_le_bytes());

        // Estimated fields are not present in the log; fill with zeros for now.
        for _ in 0..(3 + 3 + 3 + 3 + 4) {
            out.extend_from_slice(&0.0f32.to_le_bytes());
        }

        // Pad to 120 bytes.
        while out.len() < PAYLOAD_LENGTH {
            out.push(0);
        }

        out
    }

    /// Pops the next parsed mock packet and returns it with its delay since the last one.
    pub fn get_packet_with_delay(&mut self) -> Option<(FIRMMockPacket, f64)> {
        let pkt = self.parsed_packets.pop_front()?;

        // Telemetry payload starts with a little-endian f64 timestamp.
        let ts = f64::from_le_bytes(pkt.payload[0..8].try_into().unwrap());
        let delay_seconds = match self.last_emitted_timestamp_seconds {
            Some(prev) => (ts - prev).max(0.0),
            None => 0.0,
        };

        self.last_emitted_timestamp_seconds = Some(ts);
        Some((pkt, delay_seconds))
    }

    /// Pops the next parsed mock packet (no delay info).
    pub fn get_packet(&mut self) -> Option<FIRMMockPacket> {
        self.parsed_packets.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le_f32_bytes(v: f32) -> [u8; 4] {
        v.to_le_bytes()
    }

    #[test]
    fn parses_header_and_emits_packet_on_icm_record() {
        // Build a minimal valid header. Most fields are ignored by MockParser; scale factors are used.
        let mut header = Vec::new();
        header.extend_from_slice(&[0u8; HEADER_SIZE_TEXT]);
        header.extend_from_slice(&[0u8; HEADER_UID_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_DEVICE_NAME_LEN]);
        header.extend_from_slice(&[0u8; HEADER_COMM_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_PADDING_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_CAL_SIZE]);

        // temp_sf, pressure_sf, accel_sf, gyro_sf, mag_sf
        for _ in 0..HEADER_NUM_SCALE_FACTORS {
            header.extend_from_slice(&le_f32_bytes(1.0));
        }
        assert_eq!(header.len(), HEADER_TOTAL_SIZE);

        // One ICM record: [id][timestamp(3)][15 bytes]
        let mut record = Vec::new();
        record.push(ICM45686_ID);
        record.extend_from_slice(&[0x00, 0x00, 0x01]); // clock count = 1
        record.extend_from_slice(&[0u8; ICM45686_SIZE]); // all zeros -> all values 0

        let mut parser = MockParser::new();
        parser.read_header(&header);
        parser.parse_bytes(&record);

        let (pkt, delay) = parser.get_packet_with_delay().unwrap();
        assert_eq!(delay, 0.0);
        assert_eq!(pkt.len as usize, PAYLOAD_LENGTH);
        assert_eq!(pkt.payload.len(), PAYLOAD_LENGTH);
    }
}
