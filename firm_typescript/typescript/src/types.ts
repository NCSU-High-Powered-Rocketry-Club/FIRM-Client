export interface FIRMPacket {
  /** Timestamp of the data packet in seconds. */
  timestamp_seconds: number;

  /** Ambient temperature measured in degrees Celsius. */
  temperature_celsius: number;
  /** Atmospheric pressure measured in Pascals. */
  pressure_pascals: number;

  /** Raw accelerometer reading for X-axis in Gs. */
  raw_acceleration_x_gs: number;
  /** Raw accelerometer reading for Y-axis in Gs. */
  raw_acceleration_y_gs: number;
  /** Raw accelerometer reading for Z-axis in Gs. */
  raw_acceleration_z_gs: number;

  /** Raw gyroscope reading for X-axis in degrees per second. */
  raw_angular_rate_x_deg_per_s: number;
  /** Raw gyroscope reading for Y-axis in degrees per second. */
  raw_angular_rate_y_deg_per_s: number;
  /** Raw gyroscope reading for Z-axis in degrees per second. */
  raw_angular_rate_z_deg_per_s: number;

  /** Magnetometer reading for X-axis in micro-Teslas. */
  magnetic_field_x_microteslas: number;
  /** Magnetometer reading for Y-axis in micro-Teslas. */
  magnetic_field_y_microteslas: number;
  /** Magnetometer reading for Z-axis in micro-Teslas. */
  magnetic_field_z_microteslas: number;

  /** Estimated position along the X-axis in meters. */
  est_position_x_meters: number;
  /** Estimated position along the Y-axis in meters. */
  est_position_y_meters: number;
  /** Estimated position along the Z-axis in meters. */
  est_position_z_meters: number;

  /** Estimated velocity along the X-axis in meters per second. */
  est_velocity_x_meters_per_s: number;
  /** Estimated velocity along the Y-axis in meters per second. */
  est_velocity_y_meters_per_s: number;
  /** Estimated velocity along the Z-axis in meters per second. */
  est_velocity_z_meters_per_s: number;

  /** Estimated acceleration along the X-axis in Gs. */
  est_acceleration_x_gs: number;
  /** Estimated acceleration along the Y-axis in Gs. */
  est_acceleration_y_gs: number;
  /** Estimated acceleration along the Z-axis in Gs. */
  est_acceleration_z_gs: number;

  /** Estimated angular rate around the X-axis in radians per second. */
  est_angular_rate_x_rad_per_s: number;
  /** Estimated angular rate around the Y-axis in radians per second. */
  est_angular_rate_y_rad_per_s: number;
  /** Estimated angular rate around the Z-axis in radians per second. */
  est_angular_rate_z_rad_per_s: number;

  /** Estimated orientation quaternion scalar component (W). */
  est_quaternion_w: number;
  /** Estimated orientation quaternion vector component (X). */
  est_quaternion_x: number;
  /** Estimated orientation quaternion vector component (Y). */
  est_quaternion_y: number;
  /** Estimated orientation quaternion vector component (Z). */
  est_quaternion_z: number;
}

export enum DeviceProtocol {
  USB = 1,
  UART = 2,
  I2C = 3,
  SPI = 4,
}

export interface DeviceInfo {
  id: string;
  firmware_version: string;
}

export interface DeviceConfig {
  name: string;
  frequency: number;
  protocol: DeviceProtocol;
}

export type FIRMResponse =
  | { GetDeviceInfo: DeviceInfo }
  | { GetDeviceConfig: DeviceConfig }
  | { SetDeviceConfig: boolean }
  | { SetMagnetometerCalibration: boolean }
  | { SetIMUCalibration: boolean }
  | { Mock: boolean }
  | { Cancel: boolean }
  | { Error: string };

