use firm_core::data_parser::{FIRMPacket, SerialParser};
use wasm_bindgen::prelude::*;
use firm_core::command_sender::FIRMCommand;

#[wasm_bindgen(js_name = JSFIRMParser)]
pub struct JSFIRMParser {
    inner: SerialParser,
}

#[wasm_bindgen(js_class = JSFIRMParser)]
impl JSFIRMParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JSFIRMParser {
        JSFIRMParser {
            inner: SerialParser::new(),
        }
    }

    #[wasm_bindgen]
    pub fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

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
        FIRMCommand::Ping.to_bytes()
    }

    /// Creates a Reset command.
    ///
    /// # Returns
    ///
    /// * `Uint8Array` - The serialized command bytes.
    pub fn reset() -> Vec<u8> {
        FIRMCommand::Reset.to_bytes()
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
        FIRMCommand::SetRate(rate_hz).to_bytes()
    }
}