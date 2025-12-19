use firm_core::data_parser::{FIRMPacket, SerialParser};
use firm_core::command_builder::{FIRMCommand, DeviceConfig, DeviceProtocol};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FIRMCommandBuilder;

#[wasm_bindgen]
impl FIRMCommandBuilder {
    pub fn build_get_device_info() -> Vec<u8> {
        FIRMCommand::GetDeviceInfo.to_bytes()
    }

    pub fn build_get_device_config() -> Vec<u8> {
        FIRMCommand::GetDeviceConfig.to_bytes()
    }

    pub fn build_set_device_config(name: String, frequency: u16, protocol: u8) -> Vec<u8> {
        let protocol_enum: DeviceProtocol = match protocol {
            1 => DeviceProtocol::USB,
            2 => DeviceProtocol::UART,
            3 => DeviceProtocol::I2C,
            4 => DeviceProtocol::SPI,
            _ => DeviceProtocol::USB, // Default
        };
        
        let config = DeviceConfig {
            name,
            frequency,
            protocol: protocol_enum,
        };
        
        FIRMCommand::SetDeviceConfig(config).to_bytes()
    }

    pub fn build_run_imu_calibration() -> Vec<u8> {
        FIRMCommand::RunIMUCalibration.to_bytes()
    }

    pub fn build_run_magnetometer_calibration() -> Vec<u8> {
        FIRMCommand::RunMagnetometerCalibration.to_bytes()
    }

    pub fn build_download_log_file(file_id: u32) -> Vec<u8> {
        FIRMCommand::DownloadLogFile(file_id).to_bytes()
    }

    pub fn build_reboot() -> Vec<u8> {
        FIRMCommand::Reboot.to_bytes()
    }
}

#[wasm_bindgen(js_name = FIRMParser)]
pub struct FIRMParser {
    inner: SerialParser,
}

#[wasm_bindgen(js_class = FIRMParser)]
impl FIRMParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FIRMParser {
        FIRMParser {
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
