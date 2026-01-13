use crate::firm_packets::{FIRMDataPacket, FIRMResponsePacket};
use crate::utils::crc16_ccitt;
use crate::constants::data_parser_constants::*;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Streaming parser that accumulates serial bytes and produces `FIRMPacket` values.
pub struct SerialParser {
    /// Rolling buffer of unprocessed serial bytes.
    serial_bytes: Vec<u8>,
    /// Queue of fully decoded packets ready to be consumed.
    parsed_data_packets: VecDeque<FIRMDataPacket>,
    /// Queue of fully decoded command responses ready to be consumed.
    parsed_response_packets: VecDeque<FIRMResponsePacket>,
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
            parsed_data_packets: VecDeque::new(),
            parsed_response_packets: VecDeque::new(),
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
        while pos < self.serial_bytes.len().saturating_sub(1) {
            let mut is_parsing_data_packet = false;
            let mut is_parsing_response_packet = false;

            if &self.serial_bytes[pos..pos + HEADER_SIZE] == &DATA_PACKET_START_BYTES {
                is_parsing_data_packet = true;
            } else if &self.serial_bytes[pos..pos + HEADER_SIZE] == &RESPONSE_PACKET_START_BYTES {
                is_parsing_response_packet = true;
            } else {
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
            // We know that length_bytes is 2 bytes long
            let length = u16::from_le_bytes([length_bytes[0], length_bytes[1]]);

            let payload_start = length_start + LENGTH_FIELD_SIZE + PADDING_BEFORE_PAYLOAD_SIZE;
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

            if is_parsing_data_packet {
                let packet = FIRMDataPacket::from_bytes(payload_slice);
                self.parsed_data_packets.push_back(packet);
            } else if is_parsing_response_packet {
                let response = FIRMResponsePacket::from_bytes(payload_slice);
                self.parsed_response_packets.push_back(response);
            }

            // Advance past this full packet and continue scanning.
            pos = crc_start + CRC_SIZE + PADDING_AFTER_CRC_SIZE;
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
    /// - `Option<FIRMDataPacket>` - `Some(packet)` if a packet is available, otherwise `None`.
    pub fn get_data_packet(&mut self) -> Option<FIRMDataPacket> {
        self.parsed_data_packets.pop_front()
    }

    /// Pops the next parsed command response from the internal queue, if available.
    ///
    /// # Arguments
    ///
    /// - *None* - Operates on the parser's existing queued responses.
    ///
    /// # Returns
    ///
    /// - `Option<FIRMResponsePacket>` - `Some(response)` if a response is available, otherwise `None`.
    pub fn get_response_packet(&mut self) -> Option<FIRMResponsePacket> {
        self.parsed_response_packets.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::SerialParser;
    use crate::constants::command_constants::SET_DEVICE_CONFIG_MARKER;
    use crate::constants::data_parser_constants::*;
    use crate::firm_packets::FIRMResponsePacket;
    use crate::utils::crc16_ccitt;

    fn build_framed_packet(header: [u8; 2], payload: &[u8; PAYLOAD_LENGTH]) -> Vec<u8> {
        let mut out = Vec::with_capacity(FULL_PACKET_SIZE);
        out.extend_from_slice(&header);
        out.extend_from_slice(&(PAYLOAD_LENGTH as u16).to_le_bytes());
        out.extend_from_slice(&[0u8; PADDING_BEFORE_PAYLOAD_SIZE]);
        out.extend_from_slice(payload);

        let crc = crc16_ccitt(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&[0u8; PADDING_AFTER_CRC_SIZE]);
        out
    }

    #[test]
    fn test_serial_parser_parses_data_packet() {
        let mut payload = [0u8; PAYLOAD_LENGTH];
        payload[0..8].copy_from_slice(&42.0f64.to_le_bytes());
        payload[8..12].copy_from_slice(&25.0f32.to_le_bytes());

        let bytes = build_framed_packet(DATA_PACKET_START_BYTES, &payload);
        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);

        let pkt = parser.get_data_packet().expect("expected one data packet");
        assert_eq!(pkt.timestamp_seconds, 42.0);
        assert_eq!(pkt.temperature_celsius, 25.0);
        assert!(parser.get_data_packet().is_none());
        assert!(parser.get_response_packet().is_none());
    }

    #[test]
    fn test_serial_parser_parses_response_packet_split_across_calls() {
        let mut payload = [0u8; PAYLOAD_LENGTH];
        payload[0] = SET_DEVICE_CONFIG_MARKER;
        payload[1] = 1;

        let bytes = build_framed_packet(RESPONSE_PACKET_START_BYTES, &payload);
        let mid = bytes.len() / 2;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes[..mid]);
        // When we first call it, it hasnt parsed the full packet yet
        assert!(parser.get_response_packet().is_none());

        parser.parse_bytes(&bytes[mid..]);
        assert_eq!(
            parser.get_response_packet(),
            Some(FIRMResponsePacket::SetDeviceConfig(true))
        );
        assert!(parser.get_response_packet().is_none());
        assert!(parser.get_data_packet().is_none());
    }

    #[test]
    fn test_serial_parser_rejects_bad_crc() {
        let payload = [0u8; PAYLOAD_LENGTH];
        let mut bytes = build_framed_packet(DATA_PACKET_START_BYTES, &payload);

        let payload_start = HEADER_SIZE + LENGTH_FIELD_SIZE + PADDING_BEFORE_PAYLOAD_SIZE;
        bytes[payload_start] ^= 0x01;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);
        assert!(parser.get_data_packet().is_none());
        assert!(parser.get_response_packet().is_none());
    }
}
