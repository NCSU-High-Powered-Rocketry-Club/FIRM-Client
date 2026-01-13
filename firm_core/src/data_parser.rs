use crate::firm_packets::{FIRMDataPacket, FIRMResponsePacket};
use crate::framed_packet::Framed;
use crate::utils::crc16_ccitt;
use crate::constants::data_parser_constants::*;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Streaming parser that accumulates serial bytes and queues wire-level frames.
pub struct SerialParser {
    /// Rolling buffer of unprocessed serial bytes.
    serial_bytes: Vec<u8>,
    /// Queue of framed data packets ready to be consumed.
    parsed_data_frames: VecDeque<FIRMDataPacket>,
    /// Queue of framed responses ready to be consumed.
    parsed_response_frames: VecDeque<FIRMResponsePacket>,
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
            parsed_data_frames: VecDeque::new(),
            parsed_response_frames: VecDeque::new(),
        }
    }

    /// Feeds new bytes into the parser and queues any fully decoded data packets or command
    /// responses. How this function works is that it appends incoming bytes to an internal
    /// buffer, then scans through that buffer looking for data packets or responses. When
    /// it finds one, it extracts and decodes it and then queues it for later retrieval.
    ///
    /// Additionally, command responses have the same amount of bytes as data packets, so
    /// they follow the same length and CRC rules. However, they have different start bytes.
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
        while pos + 1 < self.serial_bytes.len() {
            // Need at least the 2-byte message id to consider a start.
            let start = &self.serial_bytes[pos..pos + 2];
            let is_data = start == &DATA_PACKET_START_BYTES;
            let is_response = start == &RESPONSE_PACKET_START_BYTES;
            if !is_data && !is_response {
                pos += 1;
                continue;
            }

            let header_start = pos;

            // Need at least header+len+crc.
            if header_start + MIN_PACKET_SIZE > self.serial_bytes.len() {
                break;
            }

            let length_start = header_start + HEADER_SIZE;
            let length_bytes: [u8; 4] = self.serial_bytes[length_start..length_start + LENGTH_FIELD_SIZE]
                .try_into()
                .unwrap();
            let length = u32::from_le_bytes(length_bytes) as usize;

            let payload_start = length_start + LENGTH_FIELD_SIZE;
            let crc_start = payload_start + length;
            let packet_end = crc_start + CRC_SIZE;

            if packet_end > self.serial_bytes.len() {
                break;
            }

            // Compute CRC over [header][len][payload].
            let data_to_crc = &self.serial_bytes[header_start..crc_start];
            let data_crc = crc16_ccitt(data_to_crc);
            let crc_value = u16::from_le_bytes([
                self.serial_bytes[crc_start],
                self.serial_bytes[crc_start + 1],
            ]);

            // If CRC doesn't match, skip this start byte and keep looking
            if data_crc != crc_value {
                pos += 1;
                continue;
            }

            let packet_bytes = &self.serial_bytes[header_start..packet_end];

            if is_data {
                if let Ok(frame) = FIRMDataPacket::from_bytes(packet_bytes) {
                    self.parsed_data_frames.push_back(frame);
                } else {
                    pos += 1;
                    continue;
                }
            } else {
                if let Ok(frame) = FIRMResponsePacket::from_bytes(packet_bytes) {
                    self.parsed_response_frames.push_back(frame);
                } else {
                    pos += 1;
                    continue;
                }
            }

            pos = packet_end;
        }

        // Drop all bytes that were processed, we keep only the tail for next call.
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
    /// - `Option<FIRMDataPacket>` - `Some(frame)` if a frame is available, otherwise `None`.
    pub fn get_data_frame(&mut self) -> Option<FIRMDataPacket> {
        self.parsed_data_frames.pop_front()
    }

    /// Pops the next parsed command response from the internal queue, if available.
    ///
    /// # Arguments
    ///
    /// - *None* - Operates on the parser's existing queued responses.
    ///
    /// # Returns
    ///
    /// - `Option<FIRMResponsePacket>` - `Some(frame)` if a frame is available, otherwise `None`.
    pub fn get_response_frame(&mut self) -> Option<FIRMResponsePacket> {
        self.parsed_response_frames.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::SerialParser;
    use crate::constants::command_constants::SET_DEVICE_CONFIG_MARKER;
    use crate::constants::data_parser_constants::*;
    use crate::framed_packet::FramedPacket;
    use crate::firm_packets::{FIRMData, FIRMResponse};

    fn build_framed_packet(header: [u8; 4], payload: &[u8]) -> Vec<u8> {
        FramedPacket::new(header, payload.to_vec()).to_bytes()
    }

    fn data_header() -> [u8; 4] {
        [
            DATA_PACKET_START_BYTES[0],
            DATA_PACKET_START_BYTES[1],
            PADDING_BYTE,
            PADDING_BYTE,
        ]
    }

    fn response_header(marker: u8) -> [u8; 4] {
        [
            RESPONSE_PACKET_START_BYTES[0],
            RESPONSE_PACKET_START_BYTES[1],
            PADDING_BYTE,
            marker,
        ]
    }

    #[test]
    fn test_serial_parser_parses_data_packet() {
        let mut payload = vec![0u8; 120];
        payload[0..8].copy_from_slice(&42.0f64.to_le_bytes());
        payload[8..12].copy_from_slice(&25.0f32.to_le_bytes());

        let bytes = build_framed_packet(data_header(), &payload);
        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);

        let frame = parser.get_data_frame().expect("expected one data frame");
        let pkt = FIRMData::from_bytes(frame.payload());
        assert_eq!(pkt.timestamp_seconds, 42.0);
        assert_eq!(pkt.temperature_celsius, 25.0);
        assert!(parser.get_data_frame().is_none());
        assert!(parser.get_response_frame().is_none());
    }

    #[test]
    fn test_serial_parser_parses_response_packet_split_across_calls() {
        // Marker is in the header for response packets; payload is just the response data.
        let payload = [1u8];
        let bytes = build_framed_packet(response_header(SET_DEVICE_CONFIG_MARKER), &payload);
        let mid = bytes.len() / 2;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes[..mid]);
        // When we first call it, it hasnt parsed the full packet yet
        assert!(parser.get_response_frame().is_none());

        parser.parse_bytes(&bytes[mid..]);
        let frame = parser.get_response_frame().expect("expected one response frame");
        assert_eq!(
            FIRMResponse::from_packet(&frame),
            FIRMResponse::SetDeviceConfig(true)
        );
        assert!(parser.get_response_frame().is_none());
        assert!(parser.get_data_frame().is_none());
    }

    #[test]
    fn test_serial_parser_rejects_bad_crc() {
        let payload = vec![0u8; 120];
        let mut bytes = build_framed_packet(data_header(), &payload);

        // Flip a payload bit so CRC no longer matches.
        let payload_start = HEADER_SIZE + LENGTH_FIELD_SIZE;
        bytes[payload_start] ^= 0x01;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);
        assert!(parser.get_data_frame().is_none());
        assert!(parser.get_response_frame().is_none());
    }
}
