#![cfg(feature = "wasm")]

use crate::command_sender::FirmCommand;
use crate::parser::{FIRMPacket, SerialParser};
use wasm_bindgen::prelude::*;

/// JS-facing wrapper around the FIRM serial parser.
///
/// Exposed to JavaScript / TypeScript as the `JSFIRMParser` class.
#[wasm_bindgen(js_name = JSFIRMParser)]
pub struct JSFIRMParser {
    /// Internal Rust streaming parser.
    inner: SerialParser,
}

#[wasm_bindgen(js_class = JSFIRMParser)]
impl JSFIRMParser {
    /// Creates a new `JSFIRMParser` instance for use in JS/TS.
    ///
    /// # Returns
    ///
    /// A parser with an empty internal buffer and no queued packets.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JSFIRMParser {
        JSFIRMParser {
            inner: SerialParser::new(),
        }
    }

    /// Feeds raw bytes from JavaScript into the parser.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes from the FIRM serial stream, typically a `Uint8Array`.
    #[wasm_bindgen]
    pub fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    /// Returns the next parsed packet, if one is available.
    ///
    /// # Returns
    ///
    /// * `Some(FIRMPacket)` – The next parsed packet.
    /// * `None` – If the internal queue is empty.
    #[wasm_bindgen]
    pub fn get_packet(&mut self) -> Option<FIRMPacket> {
        self.inner.get_packet()
    }
}

/// Helper class to construct FIRM commands.
#[wasm_bindgen]
pub struct FirmCommandBuilder;

#[wasm_bindgen]
impl FirmCommandBuilder {
    /// Creates a Ping command.
    ///
    /// # Returns
    ///
    /// * `Uint8Array` - The serialized command bytes.
    pub fn ping() -> Vec<u8> {
        FirmCommand::Ping.to_bytes()
    }

    /// Creates a Reset command.
    ///
    /// # Returns
    ///
    /// * `Uint8Array` - The serialized command bytes.
    pub fn reset() -> Vec<u8> {
        FirmCommand::Reset.to_bytes()
    }

    /// Creates a SetRate command.
    ///
    /// # Arguments
    ///
    /// * `rate_hz` - The desired rate in Hertz.
    ///
    /// # Returns
    ///
    /// * `Uint8Array` - The serialized command bytes.
    pub fn set_rate(rate_hz: u32) -> Vec<u8> {
        FirmCommand::SetRate(rate_hz).to_bytes()
    }
}
