import time
from firm_client import FIRMClient


PORT = "COM8"
LOG_PATH = "C:\\Users\\jackg\\Downloads\\LOG1.TXT"
BAUD_RATE = 2_000_000

SERIAL_TIMEOUT_SECONDS = 0.1
START_TIMEOUT_SECONDS = 5.0
REALTIME = True
SPEED = 2
CHUNK_SIZE = 80_000


def main() -> None:
    client = FIRMClient(PORT, BAUD_RATE, SERIAL_TIMEOUT_SECONDS)
    start_time = time.time()
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
        end_time = time.time()
        print(f"Total time: {end_time - start_time:.2f} seconds")


if __name__ == "__main__":
    main()
