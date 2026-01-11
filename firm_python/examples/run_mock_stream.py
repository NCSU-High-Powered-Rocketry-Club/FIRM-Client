from firm_client import FIRMClient


# --- User settings (edit these before running) ---
PORT = "COM6"  # e.g. "COM6" on Windows, "/dev/ttyUSB0" on Linux
LOG_PATH = r"C:\path\to\your\log.bin"

BAUD_RATE = 2_000_000
SERIAL_TIMEOUT_SECONDS = 0.1

START_TIMEOUT_SECONDS = 5.0  # wait for mock-mode acknowledgement
REALTIME = True  # False = send as fast as possible
SPEED = 1.0  # 1.0 = real-time, 2.0 = 2x faster
CHUNK_SIZE = 8192


def main() -> None:
    with FIRMClient(PORT, BAUD_RATE, SERIAL_TIMEOUT_SECONDS) as client:
        sent = client.stream_mock_log_file(
            LOG_PATH,
            realtime=REALTIME,
            speed=SPEED,
            chunk_size=CHUNK_SIZE,
            start_timeout_seconds=START_TIMEOUT_SECONDS,
        )

    print(f"Sent {sent} mock packets")


if __name__ == "__main__":
    main()
