use firm_core::client_packets::FIRMCommandPacket;
use firm_core::data_parser::SerialParser;
use firm_core::firm_packets::{DeviceConfig, DeviceProtocol, FIRMData, FIRMResponse};
use firm_core::framed_packet::Framed;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FIRMCommandBuilder;

#[wasm_bindgen]
impl FIRMCommandBuilder {
    pub fn build_get_device_info() -> Vec<u8> {
        FIRMCommandPacket::get_device_info().to_bytes()
    }

    pub fn build_get_device_config() -> Vec<u8> {
        FIRMCommandPacket::get_device_config().to_bytes()
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

        FIRMCommandPacket::set_device_config(config).to_bytes()
    }

    pub fn build_cancel() -> Vec<u8> {
        FIRMCommandPacket::cancel().to_bytes()
    }

    pub fn build_reboot() -> Vec<u8> {
        FIRMCommandPacket::reboot().to_bytes()
    }
}

#[wasm_bindgen(js_name = FIRMDataParser)]
pub struct FIRMDataParser {
    inner: SerialParser,
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
        match self.inner.get_data_frame() {
            Some(frame) => {
                let decoded = FIRMData::from_bytes(frame.payload());
                serde_wasm_bindgen::to_value(&decoded).unwrap()
            }
            None => JsValue::NULL,
        }
    }

    #[wasm_bindgen]
    pub fn get_response(&mut self) -> JsValue {
        match self.inner.get_response_frame() {
            Some(frame) => {
                let decoded = FIRMResponse::from_packet(&frame);
                serde_wasm_bindgen::to_value(&decoded).unwrap()
            }
            None => JsValue::NULL,
        }
    }
}
