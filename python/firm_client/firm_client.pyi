from typing import Optional

class FIRMPacket:
    timestamp_seconds: float
    accel_x_meters_per_s2: float
    accel_y_meters_per_s2: float
    accel_z_meters_per_s2: float
    gyro_x_radians_per_s: float
    gyro_y_radians_per_s: float
    gyro_z_radians_per_s: float
    pressure_pascals: float
    temperature_celsius: float
    mag_x_microteslas: float
    mag_y_microteslas: float
    mag_z_microteslas: float

class PyFIRMParser:
    def __new__(cls) -> "PyFIRMParser": ...
    def parse_bytes(self, data: bytes) -> None: ...
    def get_packet(self) -> Optional[FIRMPacket]: ...

class FirmCommandBuilder:
    @staticmethod
    def ping() -> bytes: ...
    @staticmethod
    def reset() -> bytes: ...
    @staticmethod
    def set_rate(rate_hz: int) -> bytes: ...
