use crate::commands::FIRMResponse;
use crate::firm_packet::FIRMPacket;
use crate::utils::crc16_ccitt;
use alloc::collections::VecDeque;
use alloc::vec::Vec;


/// Start byte sequence for packet identification. This is in little-endian format.
const PACKET_START_BYTES: [u8; 2] = [0x5A, 0xA5];

/// Start byte sequence for response identification. This is in little-endian format.
const RESPONSE_START_BYTES: [u8; 2] = [0xA5, 0x5A];

/// Size of the packet header in bytes.
const HEADER_SIZE: usize = core::mem::size_of_val(&PACKET_START_BYTES);

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

/// Streaming parser that accumulates serial bytes and produces `FIRMPacket` values.
pub struct SerialParser {
    /// Rolling buffer of unprocessed serial bytes.
    serial_bytes: Vec<u8>,
    /// Queue of fully decoded packets ready to be consumed.
    parsed_packets: VecDeque<FIRMPacket>,
    /// Queue of fully decoded command responses ready to be consumed.
    parsed_responses: VecDeque<FIRMResponse>,
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
            parsed_packets: VecDeque::new(),
            parsed_responses: VecDeque::new(),
        }
    }

    /// Feeds new bytes into the parser and queues any fully decoded packets or command
    /// responses. How this function works is that it appends incoming bytes to an internal
    /// buffer, then scans through that buffer looking for valid packets or responses. When
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
            let mut is_parsing_packet = false;
            let mut is_parsing_response = false;

            if &self.serial_bytes[pos..pos + HEADER_SIZE] == &PACKET_START_BYTES {
                is_parsing_packet = true;
            } else if &self.serial_bytes[pos..pos + HEADER_SIZE] == &RESPONSE_START_BYTES {
                is_parsing_response = true;
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

            if is_parsing_packet {
                let packet = FIRMPacket::from_bytes(payload_slice);
                self.parsed_packets.push_back(packet);
            } else if is_parsing_response {
                let response = FIRMResponse::from_bytes(payload_slice);
                self.parsed_responses.push_back(response);
            }

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
        self.parsed_packets.pop_front()
    }
}
