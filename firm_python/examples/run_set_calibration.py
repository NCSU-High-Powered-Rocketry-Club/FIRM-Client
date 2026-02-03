import argparse
from firm_client import FIRMClient

# Example calibration values
# Offsets: [x, y, z]
MAG_OFFSETS = (0.01, -0.02, 0.005)
IMU_OFFSETS = (0.0, 0.0, 0.0)

# Scale matrices: 3x3 row-major
IDENTITY_SCALE = (
    1.0,
    0.0,
    0.0,
    0.0,
    1.0,
    0.0,
    0.0,
    0.0,
    1.0,
)


def main() -> None:
    parser = argparse.ArgumentParser(description="Set FIRM calibration constants")
    parser.add_argument("port", help='Serial port (e.g. "COM8" or "/dev/ttyACM0")')
    parser.add_argument(
        "--baud",
        type=int,
        default=2_000_000,
        help="Serial baud rate (default: 2000000)",
    )
    args = parser.parse_args()

    with FIRMClient(args.port, args.baud) as client:
        print("Setting magnetometer calibration...")
        ok = client.set_magnetometer_calibration(
            MAG_OFFSETS,
            IDENTITY_SCALE,
        )
        print(f"Magnetometer calibration {'OK' if ok else 'FAILED'}")

        print("Setting IMU calibration...")
        ok = client.set_imu_calibration(
            IMU_OFFSETS,
            IDENTITY_SCALE,
        )
        print(f"IMU calibration {'OK' if ok else 'FAILED'}")


if __name__ == "__main__":
    main()
