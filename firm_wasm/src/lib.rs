use firm_core::parser::{FIRMPacket, SerialParser};
use wasm_bindgen::prelude::*;

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
