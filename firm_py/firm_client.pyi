from typing import Type
from types import TracebackType

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

class FirmClient:
    def __init__(self, port_name: str, baud_rate: int = 115200) -> None: ...
    def start(self) -> None: ...
    def stop(self) -> None: ...
    def get_packets(self) -> list[FIRMPacket]: ...
    def is_running(self) -> bool: ...
    def __enter__(self) -> "FirmClient": ...
    def __exit__(
        self,
        exc_type: Type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...
