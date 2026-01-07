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

FIRMResponse = dict[str, object]

class FIRMDataPacket:
    """Represents a single FIRM packet."""

    timestamp_seconds: float
    """Timestamp in seconds since FIRM was powered on."""
    accel_x_meters_per_s2: float
    """Acceleration along the X-axis in meters per second squared."""
    accel_y_meters_per_s2: float
    """Acceleration along the Y-axis in meters per second squared."""
    accel_z_meters_per_s2: float
    """Acceleration along the Z-axis in meters per second squared."""
    gyro_x_radians_per_s: float
    """Angular velocity around the X-axis in radians per second."""
    gyro_y_radians_per_s: float
    """Angular velocity around the Y-axis in radians per second."""
    gyro_z_radians_per_s: float
    """Angular velocity around the Z-axis in radians per second."""
    pressure_pascals: float
    """Atmospheric pressure in pascals."""
    temperature_celsius: float
    """Temperature in degrees Celsius."""
    mag_x_microteslas: float
    """Magnetic field along the X-axis in microteslas."""
    mag_y_microteslas: float
    """Magnetic field along the Y-axis in microteslas."""
    mag_z_microteslas: float
    """Magnetic field along the Z-axis in microteslas."""

    pressure_altitude_meters: float
    """
    The pressure altitude based on the international standard atmosphere model.

    Call `FIRMClient.zero_out_pressure_altitude` to get a new reference to calculate the pressure
    altitude from.
    """

class FIRMClient:
    """Represents a client for communicating with the FIRM device.

    Args:
        port_name (str): The name of the serial port to connect to.
        baud_rate (int): The baud rate for the serial connection. This must match the baud rate set
            on FIRM. Default is 2,000,000.
        timeout (float): The timeout for serial read operations in seconds. Default is 0.1.
    """
    def __init__(
        self, port_name: str, baud_rate: int = 2_000_000, timeout: float = 0.1
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

    def zero_out_pressure_altitude(self) -> None: ...
    """Zeros the pressure altitude based on the current pressure reading from the given packet."""

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
