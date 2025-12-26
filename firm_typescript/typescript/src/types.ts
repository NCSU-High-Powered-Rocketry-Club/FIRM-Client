export interface FIRMPacket {
  timestamp_seconds: number;
  accel_x_meters_per_s2: number;
  accel_y_meters_per_s2: number;
  accel_z_meters_per_s2: number;
  gyro_x_radians_per_s: number;
  gyro_y_radians_per_s: number;
  gyro_z_radians_per_s: number;
  pressure_pascals: number;
  temperature_celsius: number;
  mag_x_microteslas: number;
  mag_y_microteslas: number;
  mag_z_microteslas: number;
  pressure_altitude_meters: number;
}

export type DeviceProtocol = 'USB' | 'UART' | 'I2C' | 'SPI';

export interface DeviceInfo {
  name: string;
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
  | { RunIMUCalibration: boolean }
  | { RunMagnetometerCalibration: boolean }
  | { Cancel: boolean }
  | { Error: string };
