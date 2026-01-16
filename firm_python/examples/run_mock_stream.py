from firm_client import FIRMClient


PORT = "COM8"
LOG_PATH = r"C:\Users\jackg\Downloads\LOG198.TXT"

BAUD_RATE = 2_000_000
SERIAL_TIMEOUT_SECONDS = 0.1

START_TIMEOUT_SECONDS = 5.0
REALTIME = True
SPEED = 1.0
CHUNK_SIZE = 1024


def main() -> None:
    client = FIRMClient(PORT, BAUD_RATE, SERIAL_TIMEOUT_SECONDS)
    try:
        client.start()

        print("Starting mock stream...")
        sent = client.stream_mock_log_file(
            LOG_PATH,
            realtime=REALTIME,
            speed=SPEED,
            chunk_size=CHUNK_SIZE,
            start_timeout_seconds=START_TIMEOUT_SECONDS,
        )
        print(f"Sent {sent} mock packets")
    finally:
        client.stop()


if __name__ == "__main__":
    main()
