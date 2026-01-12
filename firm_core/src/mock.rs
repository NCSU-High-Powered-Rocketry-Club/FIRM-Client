use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::client_packets::FIRMMockPacket;

const BMP581_ID: u8 = b'B';
const ICM45686_ID: u8 = b'I';
const MMC5983MA_ID: u8 = b'M';

const BMP581_SIZE: usize = 6;
const ICM45686_SIZE: usize = 15;
const MMC5983MA_SIZE: usize = 7;

const HEADER_SIZE_TEXT: usize = 14; // "FIRM LOG vx.x"
const HEADER_UID_SIZE: usize = 8;
const HEADER_DEVICE_NAME_LEN: usize = 33;
const HEADER_COMM_SIZE: usize = 2; // 1 byte usb, 1 byte uart
const HEADER_CAL_SIZE: usize = (3 + 9) * 3 * 4; // (offsets + 3x3 matrix) * 3 sensors * 4 bytes
const HEADER_NUM_SCALE_FACTORS: usize = 5; // 5 floats

const HEADER_PADDING_SIZE: usize = 8 - ((HEADER_UID_SIZE + HEADER_DEVICE_NAME_LEN + HEADER_COMM_SIZE) % 8);
const HEADER_TOTAL_SIZE: usize = HEADER_SIZE_TEXT
    + HEADER_UID_SIZE
    + HEADER_DEVICE_NAME_LEN
    + HEADER_COMM_SIZE
    + HEADER_PADDING_SIZE
    + HEADER_CAL_SIZE
    + (HEADER_NUM_SCALE_FACTORS * 4);

pub const LOG_HEADER_SIZE: usize = HEADER_TOTAL_SIZE;

pub struct MockParser {
    /// Rolling buffer of unprocessed bytes.
    bytes: Vec<u8>,
    /// Queue of parsed mock packets and their inter-packet delay.
    parsed_packets: VecDeque<(FIRMMockPacket, f64)>,

    // Log header state.
    header_parsed: bool,

    // Timestamp state (clock-count based).
    last_clock_count: Option<u32>,

    // Whitespace repeat counter (used by the Python decoder to detect end-of-data).
    num_repeat_whitespace: usize,
}

impl MockParser {
    /// Creates a new empty `MockParser`.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            parsed_packets: VecDeque::new(),
            header_parsed: false,

            last_clock_count: None,
            num_repeat_whitespace: 0,
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

        // Reset streaming state for a fresh playback run.
        self.bytes.clear();
        self.parsed_packets.clear();
        self.last_clock_count = None;
        self.num_repeat_whitespace = 0;

        self.header_parsed = true;
    }

    /// Feeds a new chunk of bytes into the parser.
    ///
    /// Parses as many log records as possible and enqueues framed mock sensor packets.
    pub fn parse_bytes(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);

        // Parse records
        let mut pos = 0usize;
        while pos < self.bytes.len() {
            let record_start = pos;

            let id = self.bytes[pos];
            if id == 0 {
                // whitespace padding between records
                self.num_repeat_whitespace += 1;
                // End-of-data if whitespace repeats enough times, matching the Python decoder.
                if self.num_repeat_whitespace
                    > core::cmp::max(BMP581_SIZE, core::cmp::max(ICM45686_SIZE, MMC5983MA_SIZE))
                        + 4
                {
                    // Treat as EOF padding; drop buffered bytes.
                    self.bytes.clear();
                    break;
                }
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

            // Compute the delay from the previous record timestamp.
            // The log clock is 24-bit and ticks at 168 MHz.
            let delay_seconds = match self.last_clock_count {
                None => 0.0,
                Some(prev) => {
                    let delta = if clock_count < prev {
                        (clock_count + (1 << 24)) - prev
                    } else {
                        clock_count - prev
                    };
                    (delta as f64) / 168e6
                }
            };
            self.last_clock_count = Some(clock_count);

            match id {
                BMP581_ID => {
                    if pos + BMP581_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + BMP581_SIZE];
                    pos += BMP581_SIZE;

                    let mut payload = Vec::with_capacity(1 + 3 + BMP581_SIZE);
                    payload.push(id);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                ICM45686_ID => {
                    if pos + ICM45686_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + ICM45686_SIZE];
                    pos += ICM45686_SIZE;

                    let mut payload = Vec::with_capacity(1 + 3 + ICM45686_SIZE);
                    payload.push(id);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                MMC5983MA_ID => {
                    if pos + MMC5983MA_SIZE > self.bytes.len() {
                        pos = record_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + MMC5983MA_SIZE];
                    pos += MMC5983MA_SIZE;

                    let mut payload = Vec::with_capacity(1 + 3 + MMC5983MA_SIZE);
                    payload.push(id);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                _ => {
                    // Unknown/garbage byte. Don't give up immediately: advance by one byte and
                    // keep scanning so we can re-sync if we're offset or the file has junk.
                    pos = record_start + 1;
                    continue;
                }
            }
        }

        self.bytes = self.bytes[pos..].to_vec();
    }

    /// Pops the next parsed mock packet and returns it with its delay since the last one.
    pub fn get_packet_with_delay(&mut self) -> Option<(FIRMMockPacket, f64)> {
        self.parsed_packets.pop_front()
    }

    /// Pops the next parsed mock packet (no delay info).
    pub fn get_packet(&mut self) -> Option<FIRMMockPacket> {
        self.parsed_packets.pop_front().map(|(pkt, _)| pkt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_emits_packet_on_icm_record() {
        // Build a minimal valid header. Header contents are ignored by MockParser.
        let mut header = Vec::new();
        header.extend_from_slice(&[0u8; HEADER_SIZE_TEXT]);
        header.extend_from_slice(&[0u8; HEADER_UID_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_DEVICE_NAME_LEN]);
        header.extend_from_slice(&[0u8; HEADER_COMM_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_PADDING_SIZE]);
        header.extend_from_slice(&[0u8; HEADER_CAL_SIZE]);

        // scale factors bytes (ignored)
        header.extend_from_slice(&[0u8; HEADER_NUM_SCALE_FACTORS * 4]);
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
        assert_eq!(pkt.payload.len(), 1 + 3 + ICM45686_SIZE);
        assert_eq!(pkt.len as usize, pkt.payload.len());
        assert_eq!(pkt.payload[0], ICM45686_ID);
    }
}
