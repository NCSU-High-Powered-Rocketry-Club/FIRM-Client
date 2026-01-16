use crate::firm_packets::{FIRMDataPacket, FIRMResponsePacket};
use crate::framed_packet::Framed;
use crate::utils::crc16_ccitt;
use crate::constants::packet_constants::{PacketHeader, *};
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Streaming parser that accumulates serial bytes and queues wire-level frames.
pub struct SerialParser {
    /// Rolling buffer of unprocessed serial bytes.
    serial_bytes: Vec<u8>,
    /// Queue of framed data packets ready to be consumed.
    parsed_data_packets: VecDeque<FIRMDataPacket>,
    /// Queue of framed responses ready to be consumed.
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

        let mut position = 0usize;
        // Scan through the buffer looking for start words and valid packets.
        while position + 1 < self.serial_bytes.len() {
            // Need at least the 2-byte message id to consider a start.
            let potential_header =
                u16::from_le_bytes([self.serial_bytes[position], self.serial_bytes[position + 1]]);
            // TODO: when adding new packet types, extend this check to use a switch statement
            let is_data = potential_header == PacketHeader::Data as u16;
            let is_response = potential_header == PacketHeader::Response as u16;
            if !is_data && !is_response {
                position += 1;
                continue;
            }

            let header_start = position;

            // Need at least header+len+crc.
            if header_start + MIN_PACKET_SIZE > self.serial_bytes.len() {
                break;
            }

            let length_start = header_start + HEADER_SIZE + IDENTIFIER_SIZE;
            let length_bytes: [u8; LENGTH_SIZE] = self.serial_bytes[length_start..length_start + LENGTH_SIZE]
                .try_into()
                .unwrap();
            let length = u32::from_le_bytes(length_bytes) as usize;

            let payload_start = length_start + LENGTH_SIZE;
            let crc_start = payload_start + length;
            let packet_end = crc_start + CRC_SIZE;

            // If we don't have the full packet yet, wait for more bytes
            if packet_end > self.serial_bytes.len() {
                break;
            }

            // Compute CRC over [header][identifier][len][payload].
            let data_to_crc = &self.serial_bytes[header_start..crc_start];
            let data_crc = crc16_ccitt(data_to_crc);
            let crc_value = u16::from_le_bytes([
                self.serial_bytes[crc_start],
                self.serial_bytes[crc_start + 1],
            ]);

            // If CRC doesn't match, skip this start byte and keep looking
            if data_crc != crc_value {
                position += 1;
                continue;
            }

            let packet_bytes = &self.serial_bytes[header_start..packet_end];

            if is_data {
                // If we successfully parse, queue the frame, otherwise keep looking
                if let Ok(frame) = FIRMDataPacket::from_bytes(packet_bytes) {
                    self.parsed_data_packets.push_back(frame);
                } else {
                    position += 1;
                    continue;
                }
            } else {
                if let Ok(frame) = FIRMResponsePacket::from_bytes(packet_bytes) {
                    self.parsed_response_packets.push_back(frame);
                } else {
                    position += 1;
                    continue;
                }
            }

            position = packet_end;
        }

        // Drop all bytes that were processed, we keep only the tail for next call.
        self.serial_bytes = self.serial_bytes[position..].to_vec();
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
    /// - `Option<FIRMResponsePacket>` - `Some(frame)` if a frame is available, otherwise `None`.
    pub fn get_response_packet(&mut self) -> Option<FIRMResponsePacket> {
        self.parsed_response_packets.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::SerialParser;
    use crate::constants::command_constants::FIRMCommand;
    use crate::constants::packet_constants::{PacketHeader, *};
    use crate::framed_packet::FramedPacket;
    use crate::firm_packets::{FIRMResponse};

    fn build_framed_packet(header: u16, identifier: u16, payload: &[u8]) -> Vec<u8> {
        FramedPacket::new(header, identifier, payload.to_vec()).to_bytes()
    }

    fn data_header() -> (u16, u16) {
        (PacketHeader::Data as u16, 0)
    }

    fn response_header(marker: u16) -> (u16, u16) {
        (PacketHeader::Response as u16, marker)
    }

    #[test]
    fn test_serial_parser_parses_data_packet() {
        let mut payload = vec![0u8; 120];
        payload[0..8].copy_from_slice(&42.0f64.to_le_bytes());
        payload[8..12].copy_from_slice(&25.0f32.to_le_bytes());

        let (hdr, id) = data_header();
        let bytes = build_framed_packet(hdr, id, &payload);
        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);

        let packet = parser.get_data_packet().expect("expected one data frame");
        assert_eq!(packet.data().timestamp_seconds, 42.0);
        assert_eq!(packet.data().temperature_celsius, 25.0);
        assert!(parser.get_data_packet().is_none());
        assert!(parser.get_response_packet().is_none());
    }

    #[test]
    fn test_serial_parser_parses_response_packet_split_across_calls() {
        // Marker is in the identifier for response packets; payload is just the response data.
        let payload = [1u8];
        let (hdr, id) = response_header(FIRMCommand::SetDeviceConfig as u16);
        let bytes = build_framed_packet(hdr, id, &payload);
        let mid = bytes.len() / 2;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes[..mid]);
        // When we first call it, it hasnt parsed the full packet yet
        assert!(parser.get_response_packet().is_none());

        parser.parse_bytes(&bytes[mid..]);
        let frame = parser.get_response_packet().expect("expected one response frame");
        assert!(parser.get_response_packet().is_none());
        assert!(parser.get_data_packet().is_none());
    }

    #[test]
    fn test_serial_parser_rejects_bad_crc() {
        let payload = vec![0u8; 120];
        let (hdr, id) = data_header();
        let mut bytes = build_framed_packet(hdr, id, &payload);

        // Flip a payload bit so CRC no longer matches.
        let payload_start = HEADER_SIZE + IDENTIFIER_SIZE + LENGTH_SIZE;
        bytes[payload_start] ^= 0x01;

        let mut parser = SerialParser::new();
        parser.parse_bytes(&bytes);
        assert!(parser.get_data_packet().is_none());
        assert!(parser.get_response_packet().is_none());
    }
}
