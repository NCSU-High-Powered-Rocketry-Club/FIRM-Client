use crate::utils::bytes_to_str;
use crate::constants::command_constants::*;
use crate::framed_packet::{FramedPacket, Framed, FrameError};
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Represents the communication protocol used by the FIRM device.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyclass(eq, eq_int))]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum DeviceProtocol {
    USB = 1,
    UART = 2,
    I2C = 3,
    SPI = 4,
}

/// Represents the information of the FIRM device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyclass(get_all, set_all))]
pub struct DeviceInfo {
    pub firmware_version: String, // Max 8 characters
    #[cfg_attr(feature = "wasm", serde(serialize_with = "serialize_u64_as_string"))]
    // We need this because JS can't handle u64
    pub id: u64,
}

/// Serializes a u64 as a string for WASM compatibility. JS gets unhappy with
/// large integers, such as the device ID, so we serialize it as a string.
#[cfg(feature = "wasm")]
fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

/// Represents the configuration settings of the FIRM device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyclass(get_all, set_all))]
pub struct DeviceConfig {
    pub name: String, // Max 32 characters
    pub frequency: u16,
    pub protocol: DeviceProtocol,
}

/// Represents a decoded FIRM telemetry packet with converted physical units. In our Python code
/// it's called FIRMDataPacket, but to avoid confusion with the Rust packet struct
/// we name this FIRMData.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(name = "FIRMDataPacket", get_all, freelist = 20, frozen))]
pub struct FIRMData {
    pub timestamp_seconds: f64,

    pub temperature_celsius: f32,
    pub pressure_pascals: f32,

    pub raw_acceleration_x_gs: f32,
    pub raw_acceleration_y_gs: f32,
    pub raw_acceleration_z_gs: f32,

    pub raw_angular_rate_x_deg_per_s: f32,
    pub raw_angular_rate_y_deg_per_s: f32,
    pub raw_angular_rate_z_deg_per_s: f32,

    pub magnetic_field_x_microteslas: f32,
    pub magnetic_field_y_microteslas: f32,
    pub magnetic_field_z_microteslas: f32,

    pub est_position_x_meters: f32,
    pub est_position_y_meters: f32,
    pub est_position_z_meters: f32,

    pub est_velocity_x_meters_per_s: f32,
    pub est_velocity_y_meters_per_s: f32,
    pub est_velocity_z_meters_per_s: f32,

    pub est_acceleration_x_gs: f32,
    pub est_acceleration_y_gs: f32,
    pub est_acceleration_z_gs: f32,

    pub est_angular_rate_x_rad_per_s: f32,
    pub est_angular_rate_y_rad_per_s: f32,
    pub est_angular_rate_z_rad_per_s: f32,

    pub est_quaternion_w: f32,
    pub est_quaternion_x: f32,
    pub est_quaternion_y: f32,
    pub est_quaternion_z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FIRMResponse {
    GetDeviceInfo(DeviceInfo),
    GetDeviceConfig(DeviceConfig),
    SetDeviceConfig(bool),
    Mock(bool),
    Cancel(bool),
    Error(String),
}

/// Wire-level framed data packet.
///
/// This stores both the raw framed bytes and the decoded telemetry.
#[derive(Debug, Clone, PartialEq)]
pub struct FIRMDataPacket {
    frame: FramedPacket,
    data: FIRMData,
}

impl FIRMDataPacket {
    pub fn data(&self) -> &FIRMData {
        &self.data
    }
}

impl Framed for FIRMDataPacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        Ok(Self {
            data: FIRMData::from_bytes(frame.payload()),
            frame,
        })
    }
}

impl FIRMData {
    /// Constructs a `FIRMData` from a raw payload byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        fn four_bytes(bytes: &[u8], idx: &mut usize) -> [u8; 4] {
            let res = [
                bytes[*idx],
                bytes[*idx + 1],
                bytes[*idx + 2],
                bytes[*idx + 3],
            ];
            *idx += 4;
            res
        }

        let mut idx = 0;

        let timestamp_seconds: f64 = f64::from_le_bytes([
            bytes[idx],
            bytes[idx + 1],
            bytes[idx + 2],
            bytes[idx + 3],
            bytes[idx + 4],
            bytes[idx + 5],
            bytes[idx + 6],
            bytes[idx + 7],
        ]);
        idx += 8;

        let temperature_celsius: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let pressure_pascals: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let raw_acceleration_x_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let raw_acceleration_y_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let raw_acceleration_z_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let raw_angular_rate_x_deg_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let raw_angular_rate_y_deg_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let raw_angular_rate_z_deg_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let magnetic_field_x_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let magnetic_field_y_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let magnetic_field_z_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let est_position_x_meters: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_position_y_meters: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_position_z_meters: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let est_velocity_x_meters_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_velocity_y_meters_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_velocity_z_meters_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let est_acceleration_x_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_acceleration_y_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_acceleration_z_gs: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let est_angular_rate_x_rad_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_angular_rate_y_rad_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_angular_rate_z_rad_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        let est_quaternion_w: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_quaternion_x: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_quaternion_y: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let est_quaternion_z: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        Self {
            timestamp_seconds,
            temperature_celsius,
            pressure_pascals,

            raw_acceleration_x_gs,
            raw_acceleration_y_gs,
            raw_acceleration_z_gs,

            raw_angular_rate_x_deg_per_s,
            raw_angular_rate_y_deg_per_s,
            raw_angular_rate_z_deg_per_s,

            magnetic_field_x_microteslas,
            magnetic_field_y_microteslas,
            magnetic_field_z_microteslas,

            est_position_x_meters,
            est_position_y_meters,
            est_position_z_meters,

            est_velocity_x_meters_per_s,
            est_velocity_y_meters_per_s,
            est_velocity_z_meters_per_s,

            est_acceleration_x_gs,
            est_acceleration_y_gs,
            est_acceleration_z_gs,

            est_angular_rate_x_rad_per_s,
            est_angular_rate_y_rad_per_s,
            est_angular_rate_z_rad_per_s,

            est_quaternion_w,
            est_quaternion_x,
            est_quaternion_y,
            est_quaternion_z,
        }
    }
}

/// Wire-level framed response packet.
///
/// The response marker is stored in the identifier u16; the payload is marker-free.
#[derive(Debug, Clone, PartialEq)]
pub struct FIRMResponsePacket {
    frame: FramedPacket,
    command_type: FIRMCommand,
    response: FIRMResponse,
}

impl FIRMResponsePacket {
    pub fn command_type(&self) -> FIRMCommand {
        self.command_type
    }

    pub fn response(&self) -> &FIRMResponse {
        &self.response
    }
}

impl Framed for FIRMResponsePacket {
    fn frame(&self) -> &FramedPacket {
        &self.frame
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        let frame = FramedPacket::from_bytes(bytes)?;
        let marker = frame.identifier();
        let command_type = FIRMCommand::from_marker(marker)?;
        let response = FIRMResponse::from_command_and_bytes(command_type, frame.payload());

        Ok(Self {
            frame,
            command_type,
            response,
        })
    }
}

impl FIRMResponse {
    /// Constructs a decoded `FIRMResponse` from a command and raw payload bytes.
    pub fn from_command_and_bytes(command: FIRMCommand, data: &[u8]) -> Self {
        match command {
            FIRMCommand::GetDeviceInfo => {
                // [ID (8 bytes)][FIRMWARE_VERSION (8 bytes)][PADDING ...]
                let id_bytes = &data[0..DEVICE_ID_LENGTH];
                let firmware_version_bytes =
                    &data[DEVICE_ID_LENGTH..DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH];
                let id = u64::from_le_bytes(id_bytes.try_into().unwrap());
                let firmware_version = bytes_to_str(firmware_version_bytes);

                let info = DeviceInfo {
                    id,
                    firmware_version,
                };
                FIRMResponse::GetDeviceInfo(info)
            }
            FIRMCommand::GetDeviceConfig => {
                // [NAME (32 bytes)][FREQUENCY (2 bytes)][PROTOCOL (1 byte)]
                let name_bytes: [u8; DEVICE_NAME_LENGTH] =
                    data[0..DEVICE_NAME_LENGTH].try_into().unwrap();
                let name = bytes_to_str(&name_bytes);
                let frequency = u16::from_le_bytes(
                    data[DEVICE_NAME_LENGTH..DEVICE_NAME_LENGTH + FREQUENCY_LENGTH]
                        .try_into()
                        .unwrap(),
                );
                let protocol_byte = data[DEVICE_NAME_LENGTH + FREQUENCY_LENGTH];
                let protocol = match protocol_byte {
                    1 => DeviceProtocol::USB,
                    2 => DeviceProtocol::UART,
                    3 => DeviceProtocol::I2C,
                    4 => DeviceProtocol::SPI,
                    _ => DeviceProtocol::USB, // Fallback for invalid values
                };

                let config = DeviceConfig {
                    frequency,
                    protocol,
                    name,
                };

                FIRMResponse::GetDeviceConfig(config)
            }
            FIRMCommand::SetDeviceConfig => {
                let success = data.get(0) == Some(&1);
                FIRMResponse::SetDeviceConfig(success)
            }
            FIRMCommand::Mock => {
                let success = data.get(0) == Some(&1);
                FIRMResponse::Mock(success)
            }
            FIRMCommand::Cancel => {
                let acknowledgement = data.get(0) == Some(&1);
                FIRMResponse::Cancel(acknowledgement)
            }
            // Reboot currently has no decoded response type.
            FIRMCommand::Reboot => FIRMResponse::Error("No decoded response for Reboot".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceConfig, DeviceInfo, DeviceProtocol, FIRMData, FIRMResponse, FIRMResponsePacket};
    use crate::constants::command_constants::{DEVICE_ID_LENGTH, DEVICE_NAME_LENGTH, FIRMWARE_VERSION_LENGTH, FREQUENCY_LENGTH, FIRMCommand};
    use crate::constants::packet_constants::PacketHeader;
    use crate::framed_packet::{Framed, FramedPacket, FrameError};
    use crate::utils::str_to_bytes;

    fn resp_set_device_config(v: bool) -> FIRMResponse {
        FIRMResponse::SetDeviceConfig(v)
    }

    fn resp_mock(v: bool) -> FIRMResponse {
        FIRMResponse::Mock(v)
    }

    fn resp_cancel(v: bool) -> FIRMResponse {
        FIRMResponse::Cancel(v)
    }

    fn build_response_packet(marker: u16, payload: &[u8]) -> Result<FIRMResponsePacket, FrameError> {
        let bytes = FramedPacket::new(PacketHeader::Response as u16, marker, payload.to_vec()).to_bytes();
        FIRMResponsePacket::from_bytes(&bytes)
    }

    #[test]
    fn test_firm_data_packet_from_bytes() {
        let mut payload = [0u8; 120];
        let timestamp = 42.0f64;
        let temperature = 25.0f32;
        let pressure = 101325.0f32;
        payload[0..8].copy_from_slice(&timestamp.to_le_bytes());
        payload[8..12].copy_from_slice(&temperature.to_le_bytes());
        payload[12..16].copy_from_slice(&pressure.to_le_bytes());

        let pkt = FIRMData::from_bytes(&payload);
        assert_eq!(pkt.timestamp_seconds, timestamp);
        assert_eq!(pkt.temperature_celsius, temperature);
        assert_eq!(pkt.pressure_pascals, pressure);
    }

    #[test]
    fn test_firm_response_packet_from_bytes_get_device_info() {
        let mut payload = [0u8; DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH];

        let id = 0x1122334455667788u64;
        payload[0..DEVICE_ID_LENGTH].copy_from_slice(&id.to_le_bytes());

        let fw_bytes = str_to_bytes::<FIRMWARE_VERSION_LENGTH>("v1.2.3");
        payload[DEVICE_ID_LENGTH..DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH]
            .copy_from_slice(&fw_bytes);

        let pkt = build_response_packet(FIRMCommand::GetDeviceInfo as u16, &payload).unwrap();
        assert_eq!(
            pkt.response(),
            &FIRMResponse::GetDeviceInfo(DeviceInfo {
                firmware_version: "v1.2.3".to_string(),
                id,
            })
        );
        assert_eq!(pkt.command_type(), FIRMCommand::GetDeviceInfo);
    }

    #[test]
    fn test_firm_response_packet_from_bytes_get_device_config() {
        let mut payload = [0u8; DEVICE_NAME_LENGTH + FREQUENCY_LENGTH + 1];

        let name_bytes = str_to_bytes::<DEVICE_NAME_LENGTH>("MyDevice");
        payload[0..DEVICE_NAME_LENGTH].copy_from_slice(&name_bytes);

        let frequency: u16 = 50;
        payload[DEVICE_NAME_LENGTH..DEVICE_NAME_LENGTH + FREQUENCY_LENGTH]
            .copy_from_slice(&frequency.to_le_bytes());

        payload[DEVICE_NAME_LENGTH + FREQUENCY_LENGTH] = 0x03;

        let pkt = build_response_packet(FIRMCommand::GetDeviceConfig as u16, &payload).unwrap();
        assert_eq!(
            pkt.response(),
            &FIRMResponse::GetDeviceConfig(DeviceConfig {
                name: "MyDevice".to_string(),
                frequency,
                protocol: DeviceProtocol::I2C,
            })
        );
        assert_eq!(pkt.command_type(), FIRMCommand::GetDeviceConfig);
    }

    #[test]
    fn test_firm_response_packet_from_bytes_set_device_config() {
        let cases: &[(u16, FIRMCommand, fn(bool) -> FIRMResponse)] = &[
            (FIRMCommand::SetDeviceConfig as u16, FIRMCommand::SetDeviceConfig, resp_set_device_config),
            (FIRMCommand::Mock as u16, FIRMCommand::Mock, resp_mock),
            (FIRMCommand::Cancel as u16, FIRMCommand::Cancel, resp_cancel),
        ];

        for (marker, expected_command_type, mk_response) in cases {
            let pkt = build_response_packet(*marker, &[1u8]).unwrap();
            assert_eq!(pkt.response(), &mk_response(true));
            assert_eq!(pkt.command_type(), *expected_command_type);
        }
    }

    #[test]
    fn test_firm_response_packet_from_bytes_unknown_marker() {
        let payload = [0u8];
        let err = build_response_packet(0x00AB, &payload).unwrap_err();
        assert_eq!(err, FrameError::UnknownMarker(0x00AB));
    }
}
