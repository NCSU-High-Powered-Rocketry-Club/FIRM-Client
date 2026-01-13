use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::client_packets::{FIRMMockPacket, FIRMMockPacketType};
use crate::constants::mock_constants::*;

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
    pub fn read_header(&mut self, header_bytes: &[u8]) {
        assert_eq!(header_bytes.len(), HEADER_TOTAL_SIZE);

        // Reset streaming state for a fresh playback run.
        self.bytes.clear();
        self.parsed_packets.clear();
        self.last_clock_count = None;
        self.num_repeat_whitespace = 0;

        self.header_parsed = true;
    }

    /// Feeds a new chunk of bytes into the parser.
    ///
    /// Parses as many log packets as possible and enqueues framed mock packets.
    pub fn parse_bytes(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);

        // Parse log packets
        let mut pos = 0usize;
        while pos < self.bytes.len() {
            let log_packet_start = pos;

            let id = self.bytes[pos];
            if id == 0 {
                // whitespace padding between log packets
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
                pos = log_packet_start;
                break;
            }

            pos += 1;
            let t = &self.bytes[pos..pos + 3];
            pos += 3;
            let clock_count = u32::from_be_bytes([0, t[0], t[1], t[2]]);

            // Compute the delay from the previous log packet timestamp.
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
                        pos = log_packet_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + BMP581_SIZE];
                    pos += BMP581_SIZE;

                    // Payload excludes the type byte.
                    let mut payload = Vec::with_capacity(3 + BMP581_SIZE);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(FIRMMockPacketType::B, payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                ICM45686_ID => {
                    if pos + ICM45686_SIZE > self.bytes.len() {
                        pos = log_packet_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + ICM45686_SIZE];
                    pos += ICM45686_SIZE;

                    // Payload excludes the type byte.
                    let mut payload = Vec::with_capacity(3 + ICM45686_SIZE);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(FIRMMockPacketType::I, payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                MMC5983MA_ID => {
                    if pos + MMC5983MA_SIZE > self.bytes.len() {
                        pos = log_packet_start;
                        break;
                    }
                    let raw = &self.bytes[pos..pos + MMC5983MA_SIZE];
                    pos += MMC5983MA_SIZE;

                    // Payload excludes the type byte.
                    let mut payload = Vec::with_capacity(3 + MMC5983MA_SIZE);
                    payload.extend_from_slice(t);
                    payload.extend_from_slice(raw);
                    let pkt = FIRMMockPacket::new(FIRMMockPacketType::M, payload);
                    self.parsed_packets.push_back((pkt, delay_seconds));
                }
                _ => {
                    // Unknown/garbage byte. Don't give up immediately: advance by one byte and
                    // keep scanning so we can re-sync if we're offset or the file has junk.
                    pos = log_packet_start + 1;
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

    fn make_header() -> Vec<u8> {
        let mut header = Vec::new();
        header.resize(HEADER_TOTAL_SIZE, 0u8);
        header
    }

    fn make_log_packet_bytes(id: u8, clock_count_24bit: u32, raw_len: usize) -> Vec<u8> {
        // Timestamp is stored as a 24-bit big-endian counter (3 bytes).
        let clock = clock_count_24bit & 0x00FF_FFFF;
        let be = clock.to_be_bytes();
        let t = [be[1], be[2], be[3]];

        let mut out = vec![0u8; 1 + 3 + raw_len];
        out[0] = id;
        out[1..4].copy_from_slice(&t);
        out
    }

    #[test]
    fn test_reads_header_and_packet() {
        let header = make_header();
        let log_packet_bytes = make_log_packet_bytes(ICM45686_ID, 1, ICM45686_SIZE);

        let mut parser = MockParser::new();
        parser.read_header(&header);
        parser.parse_bytes(&log_packet_bytes);

        let (mock_packet, delay) = parser.get_packet_with_delay().unwrap();
        assert_eq!(delay, 0.0);
        assert_eq!(mock_packet.packet_type(), FIRMMockPacketType::I);
        assert_eq!(mock_packet.payload().len(), 3 + ICM45686_SIZE);
        assert_eq!(mock_packet.len() as usize, mock_packet.payload().len());
        assert_eq!(&mock_packet.payload()[0..3], &[0x00, 0x00, 0x01]);
        assert!(parser.get_packet_with_delay().is_none());
    }

    #[test]
    fn test_delay_works() {
        let header = make_header();

        // delta = 168 ticks => delay = 1e-6 seconds
        let log_packet_bytes1 = make_log_packet_bytes(BMP581_ID, 1000, BMP581_SIZE);
        let log_packet_bytes2 = make_log_packet_bytes(BMP581_ID, 1168, BMP581_SIZE);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&log_packet_bytes1);
        bytes.extend_from_slice(&log_packet_bytes2);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&log_packet_bytes1);
        bytes.extend_from_slice(&log_packet_bytes2);

        let mut parser = MockParser::new();
        parser.read_header(&header);
        parser.parse_bytes(&bytes);

        let (mock_packet1, d1) = parser.get_packet_with_delay().unwrap();
        let (mock_packet2, d2) = parser.get_packet_with_delay().unwrap();
        assert_eq!(d1, 0.0);

        assert_eq!(mock_packet1.packet_type(), FIRMMockPacketType::B);
        assert_eq!(mock_packet2.packet_type(), FIRMMockPacketType::B);
        assert_eq!(mock_packet1.payload(), log_packet_bytes1[1..].as_ref());
        assert_eq!(mock_packet2.payload(), log_packet_bytes2[1..].as_ref());

        let expected = 168.0f64 / 168e6;
        assert!((d2 - expected).abs() < 1e-12);
        assert!(parser.get_packet_with_delay().is_none());
    }

    #[test]
    fn split_bytes_and_garbage_resyncs() {
        let header = make_header();
        let log_packet_bytes = make_log_packet_bytes(MMC5983MA_ID, 0x123456, MMC5983MA_SIZE);

        let mut chunk1 = Vec::new();
        chunk1.push(0x99); // garbage byte
        chunk1.extend_from_slice(&log_packet_bytes[..5]);
        let chunk2 = &log_packet_bytes[5..];

        let mut parser = MockParser::new();
        parser.read_header(&header);

        parser.parse_bytes(&chunk1);
        assert!(parser.get_packet().is_none());

        parser.parse_bytes(chunk2);
        let mock_packet = parser.get_packet().unwrap();
        assert_eq!(mock_packet.packet_type(), FIRMMockPacketType::M);
        assert_eq!(mock_packet.payload().len(), 3 + MMC5983MA_SIZE);
        assert!(parser.get_packet().is_none());
    }
}
