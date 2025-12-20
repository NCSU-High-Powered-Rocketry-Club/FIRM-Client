use serde::Serialize;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Standard gravity in m/s².
const GRAVITY_METERS_PER_SECONDS_SQUARED: f32 = 9.80665;

/// Represents a decoded FIRM telemetry packet with converted physical units.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "python", pyo3::pyclass(get_all, freelist = 20, frozen))]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct FIRMPacket {
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub timestamp_seconds: f64,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_x_meters_per_s2: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_y_meters_per_s2: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub accel_z_meters_per_s2: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_x_radians_per_s: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_y_radians_per_s: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub gyro_z_radians_per_s: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub pressure_pascals: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub temperature_celsius: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_x_microteslas: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_y_microteslas: f32,
    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub mag_z_microteslas: f32,

    #[cfg_attr(feature = "wasm", wasm_bindgen(readonly))]
    pub pressure_altitude_meters: f32,
}

impl FIRMPacket {
    /// Constructs a `FIRMPacket` from a raw payload byte slice.
    /// 
    /// # Arguments
    /// 
    /// - `bytes` (`&[u8]`) - Raw payload bytes in the FIRM on-wire format.
    /// 
    /// # Returns
    /// 
    /// - `Self` - Parsed packet with converted sensor and timestamp values.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        /// Reads 4 bytes from `bytes` at `idx` and advances the index.
        /// 
        /// # Arguments
        /// 
        /// - `bytes` (`&[u8]`) - Source byte slice to read from.
        /// - `idx` (`&mut usize`) - Current read offset, updated in place.
        /// 
        /// # Returns
        /// 
        /// - `[u8; 4]` - Four-byte chunk starting at the current index.
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
