use crate::utils::bytes_to_str;
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Represents the communication protocol used by the FIRM device.
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

pub const DEVICE_INFO_MARKER: u8 = 0x01;
pub const DEVICE_CONFIG_MARKER: u8 = 0x02;
pub const SET_DEVICE_CONFIG_MARKER: u8 = 0x03;
pub const REBOOT_MARKER: u8 = 0x04;
pub const CANCEL_MARKER: u8 = 0xFF;

pub const COMMAND_LENGTH: u8 = 64;
pub const CRC_LENGTH: usize = 2;
pub const DEVICE_NAME_LENGTH: usize = 32;
pub const DEVICE_ID_LENGTH: usize = 8;
pub const FIRMWARE_VERSION_LENGTH: usize = 8;
pub const FREQUENCY_LENGTH: usize = 2;

const GRAVITY_METERS_PER_SECONDS_SQUARED: f32 = 9.80665;

/// Represents a decoded FIRM telemetry packet with converted physical units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(get_all, freelist = 20, frozen))]
pub struct FIRMDataPacket {
    pub timestamp_seconds: f64,

    pub accel_x_meters_per_s2: f32,
    pub accel_y_meters_per_s2: f32,
    pub accel_z_meters_per_s2: f32,

    pub gyro_x_radians_per_s: f32,
    pub gyro_y_radians_per_s: f32,
    pub gyro_z_radians_per_s: f32,

    pub pressure_pascals: f32,
    pub temperature_celsius: f32,

    pub mag_x_microteslas: f32,
    pub mag_y_microteslas: f32,
    pub mag_z_microteslas: f32,

    pub pressure_altitude_meters: f32,
}

impl FIRMDataPacket {
    /// Constructs a `FIRMDataPacket` from a raw payload byte slice.
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

        // Scalars.
        let temperature_celsius: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let pressure_pascals: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Accelerometer values originally in g, converted to m/s².
        let accel_x_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;
        let accel_y_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;
        let accel_z_meters_per_s2: f32 =
            f32::from_le_bytes(four_bytes(bytes, &mut idx)) * GRAVITY_METERS_PER_SECONDS_SQUARED;

        // Gyroscope values in rad/s.
        let gyro_x_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let gyro_y_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let gyro_z_radians_per_s: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Magnetometer values in µT.
        let mag_x_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let mag_y_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));
        let mag_z_microteslas: f32 = f32::from_le_bytes(four_bytes(bytes, &mut idx));

        // Skip padding before timestamp.
        idx += 4;
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

        Self {
            timestamp_seconds,
            accel_x_meters_per_s2,
            accel_y_meters_per_s2,
            accel_z_meters_per_s2,
            gyro_x_radians_per_s,
            gyro_y_radians_per_s,
            gyro_z_radians_per_s,
            pressure_pascals,
            temperature_celsius,
            mag_x_microteslas,
            mag_y_microteslas,
            mag_z_microteslas,
            pressure_altitude_meters: 0.0,
        }
    }
}

/// Represents a response received from the FIRM hardware after sending a command.
/// It can contain anything from a simple status to actual data requested by the command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FIRMResponsePacket {
    GetDeviceInfo(DeviceInfo),
    GetDeviceConfig(DeviceConfig),
    SetDeviceConfig(bool),
    Cancel(bool),
    Error(String),
}

impl FIRMResponsePacket {
    /// Constructs a `FIRMResponsePacket` from a raw payload byte slice.
    /// The format of this payload byte slice is as follows: [COMMAND MARKER][DATA...]
    pub fn from_bytes(data: &[u8]) -> Self {
        match data[0] {
            DEVICE_INFO_MARKER => {
                // [DEVICE_INFO_MARKER][ID (8 bytes)][FIRMWARE_VERSION (8 bytes)][PADDING ...]
                let id_bytes = &data[1..1 + DEVICE_ID_LENGTH];
                let firmware_version_bytes =
                    &data[1 + DEVICE_ID_LENGTH..1 + DEVICE_ID_LENGTH + FIRMWARE_VERSION_LENGTH];
                let id = u64::from_le_bytes(id_bytes.try_into().unwrap());
                let firmware_version = bytes_to_str(firmware_version_bytes);

                let info = DeviceInfo {
                    id,
                    firmware_version,
                };
                FIRMResponsePacket::GetDeviceInfo(info)
            }
            DEVICE_CONFIG_MARKER => {
                // [DEVICE_CONFIG_MARKER][NAME (32 bytes)][FREQUENCY (2 bytes)][PROTOCOL (1 byte)]
                let name_bytes: [u8; DEVICE_NAME_LENGTH] =
                    data[1..DEVICE_NAME_LENGTH + 1].try_into().unwrap();
                let name = bytes_to_str(&name_bytes);
                let frequency = u16::from_le_bytes(
                    data[DEVICE_NAME_LENGTH + 1..DEVICE_NAME_LENGTH + 1 + FREQUENCY_LENGTH]
                        .try_into()
                        .unwrap(),
                );
                let protocol = match data[DEVICE_NAME_LENGTH + 1 + FREQUENCY_LENGTH] {
                    0x01 => DeviceProtocol::USB,
                    0x02 => DeviceProtocol::UART,
                    0x03 => DeviceProtocol::I2C,
                    0x04 => DeviceProtocol::SPI,
                    _ => DeviceProtocol::USB,
                };

                let config = DeviceConfig {
                    frequency,
                    protocol,
                    name,
                };

                FIRMResponsePacket::GetDeviceConfig(config)
            }
            SET_DEVICE_CONFIG_MARKER => {
                let success = data[1] == 1;
                FIRMResponsePacket::SetDeviceConfig(success)
            }
            CANCEL_MARKER => {
                let acknowledgement = data[1] == 1;
                FIRMResponsePacket::Cancel(acknowledgement)
            }
            _ => FIRMResponsePacket::Error("Unknown response marker".to_string()),
        }
    }
}
