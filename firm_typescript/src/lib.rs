use firm_core::client_packets::{FIRMCommandPacket, FIRMMockPacket};
use firm_core::constants::mock::{FIRMMockPacketType, HEADER_TOTAL_SIZE};
use firm_core::data_parser::SerialParser;
use firm_core::firm_packets::{DeviceConfig, DeviceProtocol};
use firm_core::framed_packet::Framed;
use firm_core::mock::LogParser;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FIRMCommandBuilder;

#[wasm_bindgen]
impl FIRMCommandBuilder {
    pub fn build_get_device_info() -> Vec<u8> {
        FIRMCommandPacket::build_get_device_info_command().to_bytes()
    }

    pub fn build_get_device_config() -> Vec<u8> {
        FIRMCommandPacket::build_get_device_config_command().to_bytes()
    }

    pub fn build_set_device_config(
        name: String,
        frequency: u16,
        protocol: DeviceProtocol,
    ) -> Vec<u8> {
        let config = DeviceConfig {
            name,
            frequency,
            protocol,
        };

        FIRMCommandPacket::build_set_device_config_command(config).to_bytes()
    }

    pub fn build_cancel() -> Vec<u8> {
        FIRMCommandPacket::build_cancel_command().to_bytes()
    }

    pub fn build_reboot() -> Vec<u8> {
        FIRMCommandPacket::build_reboot_command().to_bytes()
    }

    pub fn build_mock() -> Vec<u8> {
        FIRMCommandPacket::build_mock_command().to_bytes()
    }
}

#[wasm_bindgen]
pub fn mock_header_size() -> usize {
    HEADER_TOTAL_SIZE
}

#[wasm_bindgen(js_name = FIRMDataParser)]
pub struct FIRMDataParser {
    inner: SerialParser,
}

impl Default for FIRMDataParser {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FIRMDataParser)]
impl FIRMDataParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FIRMDataParser {
        FIRMDataParser {
            inner: SerialParser::new(),
        }
    }

    #[wasm_bindgen]
    pub fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    #[wasm_bindgen]
    pub fn get_packet(&mut self) -> JsValue {
        match self.inner.get_data_packet() {
            Some(frame) => serde_wasm_bindgen::to_value(frame.data()).unwrap(),
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn get_response(&mut self) -> JsValue {
        match self.inner.get_response_packet() {
            Some(frame) => serde_wasm_bindgen::to_value(frame.response()).unwrap(),
            None => JsValue::NULL,
        }
    }
}

#[wasm_bindgen(js_name = MockLogParser)]
pub struct MockLogParser {
    inner: LogParser,
}

impl Default for MockLogParser {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = MockLogParser)]
impl MockLogParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> MockLogParser {
        MockLogParser {
            inner: LogParser::new(),
        }
    }

    #[wasm_bindgen]
    pub fn read_header(&mut self, header: &[u8]) {
        self.inner.read_header(header);
    }

    #[wasm_bindgen]
    pub fn parse_bytes(&mut self, data: &[u8]) {
        self.inner.parse_bytes(data);
    }

    #[wasm_bindgen]
    pub fn get_packet_with_delay(&mut self) -> JsValue {
        match self.inner.get_packet_and_time_delay() {
            Some((pkt, delay_seconds)) => {
                let bytes = pkt.to_bytes();
                let obj = Object::new();
                let _ = Reflect::set(
                    &obj,
                    &"bytes".into(),
                    &Uint8Array::from(bytes.as_slice()).into(),
                );
                let _ = Reflect::set(
                    &obj,
                    &"delaySeconds".into(),
                    &JsValue::from_f64(delay_seconds),
                );
                obj.into()
            }
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn build_header_packet(&self, header: &[u8]) -> Vec<u8> {
        FIRMMockPacket::new(FIRMMockPacketType::HeaderPacket, header.to_vec()).to_bytes()
    }
}
