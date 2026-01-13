use alloc::vec::Vec;

use crate::utils::crc16_ccitt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    TooShort,
    LengthMismatch { expected: usize, got: usize },
    BadCrc { expected: u16, got: u16 },
    UnknownMarker(u8),
}

/// Trait implemented by all packet types that are framed using `FramedPacket`.
pub trait Framed: Sized {
    fn frame(&self) -> &FramedPacket;

    fn to_bytes(&self) -> Vec<u8> {
        self.frame().to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError>;
}

/// Shared packet framing for the wire format:
/// `[header(4)][length(4)][payload(len)][crc(2)]`.
///
/// CRC is computed over everything before the CRC: `header + len + payload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedPacket {
    header: [u8; 4],
    payload: Vec<u8>,
    crc: u16,
}

impl FramedPacket {
    // TODO: move these constants to constants.rs
    pub const HEADER_SIZE: usize = 4;
    pub const LENGTH_SIZE: usize = 4;
    pub const CRC_SIZE: usize = 2;
    pub const MIN_SIZE: usize = Self::HEADER_SIZE + Self::LENGTH_SIZE + Self::CRC_SIZE;

    pub fn new(header: [u8; 4], payload: Vec<u8>) -> Self {
        let crc = Self::compute_crc(header, payload.len() as u32, &payload);
        Self { header, payload, crc }
    }

    /// Creates a packet from already-validated parts.
    ///
    /// This does not recompute or validate the CRC.
    pub fn from_parts(header: [u8; Self::HEADER_SIZE], payload: Vec<u8>, crc: u16) -> Self {
        Self { header, payload, crc }
    }

    pub fn header(&self) -> &[u8; Self::HEADER_SIZE] {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn crc(&self) -> u16 {
        self.crc
    }

    pub fn len(&self) -> u32 {
        self.payload.len() as u32
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let len = self.payload.len() as u32;
        let mut out = Vec::with_capacity(Self::HEADER_SIZE + Self::LENGTH_SIZE + self.payload.len() + Self::CRC_SIZE);
        out.extend_from_slice(&self.header);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&self.crc.to_le_bytes());
        out
    }

    /// Parses a single framed packet from `bytes`, requiring that `bytes` contains
    /// exactly one full frame.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < Self::MIN_SIZE {
            return Err(FrameError::TooShort);
        }

        let header: [u8; Self::HEADER_SIZE] = bytes[0..Self::HEADER_SIZE].try_into().unwrap();
        let len = u32::from_le_bytes(bytes[Self::HEADER_SIZE..Self::HEADER_SIZE + Self::LENGTH_SIZE].try_into().unwrap()) as usize;

        let expected = Self::HEADER_SIZE + Self::LENGTH_SIZE + len + Self::CRC_SIZE;
        if bytes.len() != expected {
            return Err(FrameError::LengthMismatch { expected, got: bytes.len() });
        }

        let payload_start = Self::HEADER_SIZE + Self::LENGTH_SIZE;
        let payload_end = payload_start + len;
        let payload = bytes[payload_start..payload_end].to_vec();

        let got_crc = u16::from_le_bytes(bytes[payload_end..payload_end + Self::CRC_SIZE].try_into().unwrap());
        let expected_crc = Self::compute_crc(header, len as u32, &payload);
        if got_crc != expected_crc {
            return Err(FrameError::BadCrc { expected: expected_crc, got: got_crc });
        }

        Ok(Self { header, payload, crc: got_crc })
    }

    /// Computes CRC over `[header][length][payload]`.
    pub fn compute_crc(header: [u8; Self::HEADER_SIZE], len: u32, payload: &[u8]) -> u16 {
        let mut crc_input = Vec::with_capacity(Self::HEADER_SIZE + Self::LENGTH_SIZE + payload.len());
        crc_input.extend_from_slice(&header);
        crc_input.extend_from_slice(&len.to_le_bytes());
        crc_input.extend_from_slice(payload);
        crc16_ccitt(&crc_input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framed_packet_roundtrip() {
        let header = [0x5A, 0xA5, 0x00, 0x00];
        let payload = vec![1u8, 2, 3, 4, 5];
        let pkt = FramedPacket::new(header, payload.clone());
        let bytes = pkt.to_bytes();

        let parsed = FramedPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header(), &header);
        assert_eq!(parsed.payload(), payload.as_slice());
        assert_eq!(parsed.crc(), pkt.crc());
    }
}
