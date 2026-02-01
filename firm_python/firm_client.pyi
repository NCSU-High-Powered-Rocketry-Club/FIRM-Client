from __future__ import annotations

from enum import IntEnum
from types import TracebackType
from typing import Any, Optional, Type


class DeviceProtocol(IntEnum):
    """Enum of the supported device communication protocols."""
    USB: int
    UART: int
    I2C: int
    SPI: int


class DeviceInfo:
    """Represents information about the FIRM device."""
    firmware_version: str
    id: int


class DeviceConfig:
    """Represents the configuration of the FIRM device."""
    name: str
    frequency: int
    protocol: DeviceProtocol


class FIRMData:
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


class MockDeviceHandle:
    """Handle for controlling an in-process mock device."""

    def inject_response(self, identifier: int, payload: bytes | bytearray) -> None: ...
    """Inject a response packet (identifier + raw payload bytes) into the mock device."""

    def wait_for_command_identifier(self, timeout_seconds: float) -> int | None: ...
    """Wait up to timeout_seconds for a command to be observed; returns its identifier or None."""


class FIRMClient:
    """Client for communicating with the FIRM device.

    Args:
        port_name: The name of the serial port to connect to.
        baud_rate: The baud rate for the serial connection. Default is 2,000,000.
        timeout: Read timeout used when get_data_packets(block=True). Default is 0.1 seconds.
    """

    def __init__(self, port_name: str, baud_rate: int = 2_000_000, timeout: float = 0.1) -> None: ...

    @staticmethod
    def new_mock(timeout: float = 0.1) -> tuple[FIRMClient, MockDeviceHandle]: ...
    """Create a client + mock device pair for testing."""

    def start(self) -> None: ...
    """Start the background reader thread."""

    def stop(self) -> None: ...
    """Stop the background reader thread and close the serial port."""

    def get_data_packets(self, block: bool = False) -> list[FIRMData]: ...
    """Retrieve currently-available data packets.

    Args:
        block: If True, blocks up to `timeout` (from __init__) waiting for packets.
    """

    def get_device_info(self, timeout_seconds: float = 5.0) -> DeviceInfo | None: ...
    """Request device info and wait up to timeout_seconds."""

    def get_device_config(self, timeout_seconds: float = 5.0) -> DeviceConfig | None: ...
    """Request device configuration and wait up to timeout_seconds."""

    def set_device_config(
        self,
        name: str,
        frequency: int,
        protocol: DeviceProtocol,
        timeout_seconds: float = 5.0,
    ) -> bool: ...
    """Set device config and wait up to timeout_seconds for acknowledgement."""

    def cancel(self, timeout_seconds: float = 5.0) -> bool: ...
    """Send cancel and wait up to timeout_seconds for acknowledgement."""

    def reboot(self) -> None: ...
    """Send reboot command."""

    def stream_mock_log_file(
        self,
        log_path: str,
        realtime: bool = True,
        speed: float = 1.0,
        chunk_size: int = 8192,
        start_timeout_seconds: float = 5.0,
    ) -> int: ...
    """Stream an entire mock log file synchronously and return bytes sent."""

    def start_mock_log_stream(
        self,
        log_path: str,
        realtime: bool = True,
        speed: float = 1.0,
        chunk_size: int = 8192,
        start_timeout_seconds: float = 5.0,
        cancel_on_finish: bool = True,
    ) -> None: ...
    """Start streaming a mock log file asynchronously in the background."""

    def stop_mock_log_stream(self, cancel_device: bool = True) -> None: ...
    """Stop the async mock log stream. Optionally cancel the device."""

    def poll_mock_log_stream(self) -> int | None: ...
    """Non-blocking: returns None if still streaming; otherwise bytes sent."""

    def join_mock_log_stream(self) -> int | None: ...
    """Blocking join for the async mock log stream; returns bytes sent or None."""

    def is_running(self) -> bool: ...
    """True if the client reader thread is running."""

    def __enter__(self) -> FIRMClient: ...
    """Context manager which calls start()."""

    def __exit__(
        self,
        exc_type: Type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...
    """Context manager which calls stop()."""
