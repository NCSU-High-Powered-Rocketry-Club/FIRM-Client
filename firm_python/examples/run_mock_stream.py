import argparse
import time
from firm_client import FIRMClient


SERIAL_TIMEOUT_SECONDS = 0.1
START_TIMEOUT_SECONDS = 5.0
REALTIME = True
SPEED = 1.0
CHUNK_SIZE = 80000


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Stream mock FIRM log file to a device"
    )
    parser.add_argument("port", help="Serial port name (e.g., COM8 or /dev/ttyACM0)")
    parser.add_argument("log_path", help="Path to the FIRM log file to stream")
    parser.add_argument(
        "-b",
        "--baud-rate",
        type=int,
        default=2_000_000,
        help="Baud rate for serial communication (default: 2000000)",
    )
    args = parser.parse_args()

    client = FIRMClient(args.port, args.baud_rate, SERIAL_TIMEOUT_SECONDS)
    start_time = time.time()
    try:
        client.start()

        print("Starting mock stream...")
        sent = client.stream_mock_log_file(
            args.log_path,
            realtime=REALTIME,
            speed=SPEED,
            chunk_size=CHUNK_SIZE,
            start_timeout_seconds=START_TIMEOUT_SECONDS,
        )
        print(f"Sent {sent} mock packets")
    finally:
        client.stop()
        end_time = time.time()
        print(f"Total time: {end_time - start_time:.2f} seconds")


if __name__ == "__main__":
    main()
