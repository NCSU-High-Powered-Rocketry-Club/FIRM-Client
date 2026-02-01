import argparse
import csv
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

from firm_client import FIRMClient


SERIAL_TIMEOUT_SECONDS = 0.05
START_TIMEOUT_SECONDS = 5.0
REALTIME = True
SPEED = 1.0
CHUNK_SIZE = 80_000
DRAIN_SECONDS = 1.0


FIELDS: List[str] = [
    "timestamp_seconds",
    "temperature_celsius",
    "pressure_pascals",
    "raw_acceleration_x_gs",
    "raw_acceleration_y_gs",
    "raw_acceleration_z_gs",
    "raw_angular_rate_x_deg_per_s",
    "raw_angular_rate_y_deg_per_s",
    "raw_angular_rate_z_deg_per_s",
    "magnetic_field_x_microteslas",
    "magnetic_field_y_microteslas",
    "magnetic_field_z_microteslas",
    "est_position_x_meters",
    "est_position_y_meters",
    "est_position_z_meters",
    "est_velocity_x_meters_per_s",
    "est_velocity_y_meters_per_s",
    "est_velocity_z_meters_per_s",
    "est_acceleration_x_gs",
    "est_acceleration_y_gs",
    "est_acceleration_z_gs",
    "est_angular_rate_x_rad_per_s",
    "est_angular_rate_y_rad_per_s",
    "est_angular_rate_z_rad_per_s",
    "est_quaternion_w",
    "est_quaternion_x",
    "est_quaternion_y",
    "est_quaternion_z",
]


def _row_from_packet(pkt: Any) -> Dict[str, Any]:
    return {f: getattr(pkt, f) for f in FIELDS}


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Stream a mock FIRM log file asynchronously and save FIRM output packets to CSV"
    )
    parser.add_argument("port", help='Serial port name (e.g., "COM8" or "/dev/ttyACM0")')
    parser.add_argument("log_path", help="Path to the FIRM log file to stream")
    parser.add_argument("--out", required=True, help="Output CSV path")
    parser.add_argument(
        "-b",
        "--baud-rate",
        type=int,
        default=2_000_000,
        help="Baud rate (default: 2000000)",
    )
    parser.add_argument(
        "--serial-timeout-seconds",
        type=float,
        default=SERIAL_TIMEOUT_SECONDS,
        help=f"Read timeout for serial polling (default: {SERIAL_TIMEOUT_SECONDS})",
    )
    parser.add_argument(
        "--start-timeout-seconds",
        type=float,
        default=START_TIMEOUT_SECONDS,
        help=f"Timeout waiting for mock ack (default: {START_TIMEOUT_SECONDS})",
    )
    parser.add_argument(
        "--realtime",
        action=argparse.BooleanOptionalAction,
        default=REALTIME,
        help=f"Pace the stream by timestamps (default: {REALTIME})",
    )
    parser.add_argument(
        "--speed",
        type=float,
        default=SPEED,
        help=f"Playback speed multiplier when realtime (default: {SPEED})",
    )
    parser.add_argument(
        "--chunk-size",
        type=int,
        default=CHUNK_SIZE,
        help=f"File read chunk size (default: {CHUNK_SIZE})",
    )
    parser.add_argument(
        "--drain-seconds",
        type=float,
        default=DRAIN_SECONDS,
        help=f"How long to keep logging after stream ends (default: {DRAIN_SECONDS})",
    )
    args = parser.parse_args()

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    total_rows = 0
    sent_packets: Optional[int] = None
    mock_finished_wall: Optional[float] = None

    start_wall = time.time()

    with FIRMClient(args.port, args.baud_rate, args.serial_timeout_seconds) as client:
        # Clear any initial packets (blocking read to drain).
        client.get_data_packets(block=True)

        print("Starting async mock stream...")
        client.start_mock_log_stream(
            args.log_path,
            realtime=args.realtime,
            speed=args.speed,
            chunk_size=args.chunk_size,
            start_timeout_seconds=args.start_timeout_seconds,
            cancel_on_finish=True,
        )

        with out_path.open("w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=FIELDS)
            writer.writeheader()

            while True:
                # Poll streamer completion once until it finishes.
                if sent_packets is None:
                    sent_packets = client.poll_mock_log_stream()

                    # `poll_*` is our single source of truth for "still streaming":
                    # - None => still running
                    # - int  => finished
                    if sent_packets is not None and mock_finished_wall is None:
                        mock_finished_wall = time.time()

                # Keep reading device output packets.
                packets = client.get_data_packets(block=False)
                for pkt in packets:
                    writer.writerow(_row_from_packet(pkt))
                    total_rows += 1

                # Once the stream is finished, keep logging for a fixed drain window.
                if mock_finished_wall is not None:
                    if time.time() - mock_finished_wall >= float(args.drain_seconds):
                        break

                # Avoid busy loop.
                time.sleep(0.001)

        # Best-effort join, in case we exited before the poll observed completion.
        if sent_packets is None:
            sent_packets = client.join_mock_log_stream()

    elapsed = time.time() - start_wall
    print(f"Wrote {total_rows} FIRM output packets to: {out_path}")
    if sent_packets is not None:
        print(f"Mock packets sent: {sent_packets}")
    print(f"Total time: {elapsed:.2f} seconds")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
