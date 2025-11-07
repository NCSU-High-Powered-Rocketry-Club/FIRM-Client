use crate::parser::SerialParser;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

/// JS-facing wrapper around the FIRM serial parser.
///
/// Exposed to JavaScript as the `FIRM` class.
///
/// # Arguments
///
/// - *None* - Constructed with default internal parser state.
///
/// # Returns
///
/// - `WasmSerialParser` - A new JS-visible parser wrapper.
#[wasm_bindgen(js_name = FIRM)]
pub struct WasmSerialParser {
    inner: SerialParser,
}

#[wasm_bindgen(js_class = FIRM)]
impl WasmSerialParser {
    /// Creates a new `FIRM` parser instance for use in JS.
    /// 
    /// # Arguments
    /// 
    /// - *None* - Initializes an empty internal parser.
    /// 
    /// # Returns
    /// 
    /// - `WasmSerialParser` - A new parser ready to accept bytes.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmSerialParser {
        WasmSerialParser {
            inner: SerialParser::new(),
        }
    }

    /// Feeds raw bytes from JS into the parser.
    /// 
    /// # Arguments
    /// 
    /// - `data` (`&[u8]`) - Raw bytes from the FIRM serial stream.
    /// 
    /// # Returns
    /// 
    /// - `()` - Parsed packets are stored internally for `get_packet`.
    #[wasm_bindgen]
    pub fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    /// Returns the next parsed packet as a JS value or `null`.
    /// 
    /// # Arguments
    /// 
    /// - *None* - Reads from the internal packet queue.
    /// 
    /// # Returns
    /// 
    /// - `JsValue` - A serialized packet object or `null` if none are available.
    #[wasm_bindgen]
    pub fn get_packet(&mut self) -> JsValue {
        match self.inner.get_packet() {
            Some(packet) => to_value(&packet).unwrap_or(JsValue::NULL),
            None => JsValue::NULL,
        }
    }
}
