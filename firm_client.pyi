from typing import Type
from types import TracebackType
from enum import IntEnum


class DeviceProtocol(IntEnum):
    USB: int
    UART: int
    I2C: int
    SPI: int

class DeviceInfo:
    firmware_version: str
    id: int

class DeviceConfig:
    name: str
    frequency: int
    protocol: DeviceProtocol

# TODO I dont like this
FIRMResponse = dict[str, object]

class FIRMDataPacket:
    """Represents a data packet received from the FIRM device."""

    timestamp_seconds: float
    """Timestamp of the data packet in seconds."""

    temperature_celsius: float
    """Ambient temperature measured in degrees Celsius."""
    pressure_pascals: float
    """Atmospheric pressure measured in Pascals."""

    raw_acceleration_x_gs: float
    """Raw accelerometer reading for X-axis in Gs."""
    raw_acceleration_y_gs: float
    """Raw accelerometer reading for Y-axis in Gs."""
    raw_acceleration_z_gs: float
    """Raw accelerometer reading for Z-axis in Gs."""

    raw_angular_rate_x_deg_per_s: float
    """Raw gyroscope reading for X-axis in degrees per second."""
    raw_angular_rate_y_deg_per_s: float
    """Raw gyroscope reading for Y-axis in degrees per second."""
    raw_angular_rate_z_deg_per_s: float
    """Raw gyroscope reading for Z-axis in degrees per second."""

    magnetic_field_x_microteslas: float
    """Magnetometer reading for X-axis in micro-Teslas."""
    magnetic_field_y_microteslas: float
    """Magnetometer reading for Y-axis in micro-Teslas."""
    magnetic_field_z_microteslas: float
    """Magnetometer reading for Z-axis in micro-Teslas."""

    est_position_x_meters: float
    """Estimated position along the X-axis in meters."""
    est_position_y_meters: float
    """Estimated position along the Y-axis in meters."""
    est_position_z_meters: float
    """Estimated position along the Z-axis in meters."""

    est_velocity_x_meters_per_s: float
    """Estimated velocity along the X-axis in meters per second."""
    est_velocity_y_meters_per_s: float
    """Estimated velocity along the Y-axis in meters per second."""
    est_velocity_z_meters_per_s: float
    """Estimated velocity along the Z-axis in meters per second."""

    est_acceleration_x_gs: float
    """Estimated acceleration along the X-axis in Gs."""
    est_acceleration_y_gs: float
    """Estimated acceleration along the Y-axis in Gs."""
    est_acceleration_z_gs: float
    """Estimated acceleration along the Z-axis in Gs."""

    est_angular_rate_x_rad_per_s: float
    """Estimated angular rate around the X-axis in radians per second."""
    est_angular_rate_y_rad_per_s: float
    """Estimated angular rate around the Y-axis in radians per second."""
    est_angular_rate_z_rad_per_s: float
    """Estimated angular rate around the Z-axis in radians per second."""

    est_quaternion_w: float
    """Estimated orientation quaternion scalar component (W)."""
    est_quaternion_x: float
    """Estimated orientation quaternion vector component (X)."""
    est_quaternion_y: float
    """Estimated orientation quaternion vector component (Y)."""
    est_quaternion_z: float
    """Estimated orientation quaternion vector component (Z)."""

class FIRMClient:
    """Represents a client for communicating with the FIRM device.

    Args:
        port_name (str): The name of the serial port to connect to.
        baud_rate (int): The baud rate for the serial connection. This must match the baud rate set
            on FIRM. Default is 112,500.
        timeout (float): The timeout for serial read operations in seconds. Default is 0.1.
    """
    def __init__(
        self, port_name: str, baud_rate: int = 112_500, timeout: float = 0.1
    ) -> None: ...
    def start(self) -> None: ...
    """Starts the client by starting a thread to read data from the FIRM device."""

    def stop(self) -> None: ...
    """Stops the client by stopping the data reading thread and closing the serial port."""

    def get_data_packets(self, block: bool = False) -> list[FIRMDataPacket]: ...
    """Retrieves available data packets from the FIRM device.
    
    Args:
        block (bool): If True, blocks until at least one packet is available. Default is
            False.
    """

    def is_running(self) -> bool: ...
    """Return True if the client is currently running and reading data."""

    def send_command_bytes(self, command_bytes: bytes) -> None: ...
    """Sends raw command bytes to the device."""

    def get_responses(self, block: bool = False) -> list[FIRMResponse]: ...
    """Retrieves parsed command responses (as dicts)."""

    def get_device_info(self, timeout_seconds: float = 5.0) -> DeviceInfo | None: ...
    """Requests device info and waits up to timeout_seconds."""

    def get_device_config(
        self, timeout_seconds: float = 5.0
    ) -> DeviceConfig | None: ...
    """Requests device configuration and waits up to timeout_seconds."""

    def set_device_config(
        self,
        name: str,
        frequency: int,
        protocol: DeviceProtocol,
        timeout_seconds: float = 5.0,
    ) -> bool: ...
    """Sets device config and waits up to timeout_seconds for acknowledgement."""

    def cancel(self, timeout_seconds: float = 5.0) -> bool: ...
    """Sends cancel and waits up to timeout_seconds for acknowledgement."""

    def reboot(self) -> None: ...
    """Sends reboot command."""

    def __enter__(self) -> "FIRMClient": ...
    """Context manager which simply calls .start()"""

    def __exit__(
        self,
        exc_type: Type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...
    """Context manager which simply calls .stop()"""
