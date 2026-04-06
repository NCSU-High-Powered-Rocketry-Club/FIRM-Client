import time
from typing import List, Tuple

import firm_client

try:
    import numpy as np
except ImportError as exc:  # pragma: no cover - runtime check only
    raise SystemExit(
        "This script requires numpy. Install it with: pip install numpy"
    ) from exc


PORT = "COM22"  # Update as needed (e.g., "COM12" or "/dev/ttyACM0")
BAUD_RATE = 2_000_000

INTERVAL_COUNT = 26
COLLECTION_SECONDS = 10.0
PAUSE_SECONDS = 10.0


def _collect_accel_samples(
    client: firm_client.FIRMClient, duration_seconds: float
) -> List[Tuple[float, float, float]]:
    samples: List[Tuple[float, float, float]] = []
    end_time = time.time() + duration_seconds

    while time.time() < end_time:
        packets = client.get_data_packets(block=True)
        for packet in packets:
            samples.append(
                (
                    packet.raw_acceleration_x_gs,
                    packet.raw_acceleration_y_gs,
                    packet.raw_acceleration_z_gs,
                )
            )

    return samples


def _collect_accel_samples_and_gyro_mean(
    client: firm_client.FIRMClient, duration_seconds: float
) -> Tuple[List[Tuple[float, float, float]], Tuple[float, float, float] | None]:
    accel_samples: List[Tuple[float, float, float]] = []
    gyro_sum_x = 0.0
    gyro_sum_y = 0.0
    gyro_sum_z = 0.0
    gyro_count = 0

    end_time = time.time() + duration_seconds
    while time.time() < end_time:
        packets = client.get_data_packets(block=True)
        for packet in packets:
            accel_samples.append(
                (
                    packet.raw_acceleration_x_gs,
                    packet.raw_acceleration_y_gs,
                    packet.raw_acceleration_z_gs,
                )
            )

            gyro_sum_x += packet.raw_angular_rate_x_deg_per_s
            gyro_sum_y += packet.raw_angular_rate_y_deg_per_s
            gyro_sum_z += packet.raw_angular_rate_z_deg_per_s
            gyro_count += 1

    if gyro_count == 0:
        return accel_samples, None

    gyro_mean = (
        gyro_sum_x / gyro_count,
        gyro_sum_y / gyro_count,
        gyro_sum_z / gyro_count,
    )
    return accel_samples, gyro_mean


def _discard_packets(client: firm_client.FIRMClient, duration_seconds: float) -> None:
    end_time = time.time() + duration_seconds
    while time.time() < end_time:
        client.get_data_packets(block=True)


def _fit_ellipsoid(
    samples: List[Tuple[float, float, float]]
) -> Tuple[np.ndarray, np.ndarray, float] | None:
    n = len(samples)
    if n < 10:
        return None

    d_matrix = np.zeros((n, 9), dtype=np.float64)
    ones = np.ones((n, 1), dtype=np.float64)

    for idx, (x, y, z) in enumerate(samples):
        d_matrix[idx, 0] = x * x
        d_matrix[idx, 1] = y * y
        d_matrix[idx, 2] = z * z
        d_matrix[idx, 3] = 2.0 * x * y
        d_matrix[idx, 4] = 2.0 * x * z
        d_matrix[idx, 5] = 2.0 * y * z
        d_matrix[idx, 6] = 2.0 * x
        d_matrix[idx, 7] = 2.0 * y
        d_matrix[idx, 8] = 2.0 * z

    solution, *_ = np.linalg.lstsq(d_matrix, ones, rcond=1e-6)
    solution = solution.flatten()

    a, b, c, d, e, f, g, h, i_val = solution
    q_matrix = np.array([[a, d, e], [d, b, f], [e, f, c]], dtype=np.float64)
    u_vec = np.array([g, h, i_val], dtype=np.float64)

    try:
        q_inv = np.linalg.inv(q_matrix)
    except np.linalg.LinAlgError:
        return None

    center = -q_inv @ u_vec
    term = center.T @ q_matrix @ center
    radius_sq = 1.0 + term
    if radius_sq <= 0.0:
        return None

    field_strength = float(np.sqrt(radius_sq))

    eigenvalues, eigenvectors = np.linalg.eigh(q_matrix)
    if np.any(eigenvalues < 0.0):
        return None

    d_sqrt = np.diag(np.sqrt(eigenvalues))
    soft_iron = (eigenvectors @ d_sqrt @ eigenvectors.T) * (1.0 / field_strength)

    return center, soft_iron, field_strength


def main() -> None:
    client = firm_client.FIRMClient(PORT, BAUD_RATE)
    client.start()

    try:
        client.get_data_packets(block=True)  # Clear initial packets

        all_samples: List[Tuple[float, float, float]] = []
        gyro_sum_x = 0.0
        gyro_sum_y = 0.0
        gyro_sum_z = 0.0
        gyro_count = 0

        print("Starting accelerometer calibration (gravity ellipsoid).")
        for idx in range(INTERVAL_COUNT):
            print(
                f"Interval {idx + 1}/{INTERVAL_COUNT}: collecting for {COLLECTION_SECONDS:.0f} seconds..."
            )
            interval_samples, interval_gyro_mean = _collect_accel_samples_and_gyro_mean(
                client, COLLECTION_SECONDS
            )
            all_samples.extend(interval_samples)
            print(f"  Collected {len(interval_samples)} accel samples.")

            if interval_gyro_mean is not None:
                gx, gy, gz = interval_gyro_mean
                gyro_sum_x += gx
                gyro_sum_y += gy
                gyro_sum_z += gz
                gyro_count += 1

            if idx < INTERVAL_COUNT - 1:
                print(
                    f"Pause {PAUSE_SECONDS:.0f} seconds to move the mount (no calibration data logged)."
                )
                _discard_packets(client, PAUSE_SECONDS)

        result = _fit_ellipsoid(all_samples)
        if result is None:
            print("Calibration failed: insufficient or invalid data.")
            return

        offsets, scale_matrix, field_strength = result
        offsets_list = offsets.tolist()
        scale_list = scale_matrix.flatten().tolist()

        gyro_offsets: Tuple[float, float, float] | None
        if gyro_count == 0:
            gyro_offsets = None
        else:
            gyro_offsets = (
                gyro_sum_x / gyro_count,
                gyro_sum_y / gyro_count,
                gyro_sum_z / gyro_count,
            )
        gyro_identity_scale = [
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ]

        print("\nFinal calibration coefficients:")
        print(f"Accel offsets (x,y,z) [calibrated = raw - offset]: {offsets_list}")
        print(f"Accel scale matrix (row-major): {scale_list}")
        print(f"Estimated field strength: {field_strength:.6f} g")

        if gyro_offsets is None:
            print("Gyro offsets: unavailable (no gyro samples collected).")
        else:
            print(
                "Gyro offsets (x,y,z) deg/s [calibrated = raw - offset]: "
                f"{list(gyro_offsets)}"
            )
            print(f"Gyro scale matrix (row-major, identity): {gyro_identity_scale}")
        print("\nTo apply to the device, use set_imu_calibration(offsets, scale_matrix).")

    finally:
        client.stop()


if __name__ == "__main__":
    main()
