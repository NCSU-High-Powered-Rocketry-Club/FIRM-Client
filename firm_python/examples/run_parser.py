from firm_client import FIRMClient


def main() -> None:
    port_name = "/dev/ttyACM0"  # Update this to your actual port
    baud_rate = 2_000_000
    initial_dt = 0

    with FIRMClient(port_name, baud_rate) as client:
        client.get_data_packets(block=True)  # Clear initial packets
        client.zero_out_pressure_altitude()
        while client.is_running():
            packets = client.get_data_packets(block=True)
            print(f"Received {len(packets)} packets")
            for packet in packets:
                # print(
                #     f"Timestamp: {packet.timestamp_seconds:.9f} s, "
                #     f"Accel: ({packet.accel_x_meters_per_s2:.2f}, "
                #     f"{packet.accel_y_meters_per_s2:.2f}, "
                #     f"{packet.accel_z_meters_per_s2:.2f}) m/s², "
                #     f"Gyro: ({packet.gyro_x_radians_per_s:.2f}, "
                #     f"{packet.gyro_y_radians_per_s:.2f}, "
                #     f"{packet.gyro_z_radians_per_s:.2f}) rad/s, "
                #     f"Pressure: {packet.pressure_pascals:.2f} Pa, "
                #     f"Temperature: {packet.temperature_celsius:.2f} °C, "
                #     f"Mag: ({packet.mag_x_microteslas:.2f}, "
                #     f"{packet.mag_y_microteslas:.2f}, "
                #     f"{packet.mag_z_microteslas:.2f}) µT"
                # )
                print(f"Pressure: {packet.pressure_pascals} Pa")
                print(f"Pressure alt meters: {packet.pressure_altitude_meters} m")
            print()
            print(
                f"Time since last packet: "
                f"{(packets[-1].timestamp_seconds - initial_dt) * 1e3:.9f} ms"
            )
            initial_dt = packets[-1].timestamp_seconds


if __name__ == "__main__":
    main()
