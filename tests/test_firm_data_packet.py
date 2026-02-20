import inspect

from firm_client import FIRMDataPacket


def test_firm_data_packet_constructor() -> None:
    packet = FIRMDataPacket(
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0,
        11.0,
        12.0,
        13.0,
        14.0,
        15.0,
        16.0,
        17.0,
        18.0,
        19.0,
        20.0,
        21.0,
        22.0,
        23.0,
        24.0,
        25.0,
        26.0,
        27.0,
        28.0,
    )

    assert packet.timestamp_seconds == 1.0
    assert packet.temperature_celsius == 2.0
    assert packet.pressure_pascals == 3.0
    assert packet.raw_acceleration_x_gs == 4.0
    assert packet.raw_acceleration_y_gs == 5.0
    assert packet.raw_acceleration_z_gs == 6.0
    assert packet.raw_angular_rate_x_deg_per_s == 7.0
    assert packet.raw_angular_rate_y_deg_per_s == 8.0
    assert packet.raw_angular_rate_z_deg_per_s == 9.0
    assert packet.magnetic_field_x_microteslas == 10.0
    assert packet.magnetic_field_y_microteslas == 11.0
    assert packet.magnetic_field_z_microteslas == 12.0
    assert packet.est_position_x_meters == 13.0
    assert packet.est_position_y_meters == 14.0
    assert packet.est_position_z_meters == 15.0
    assert packet.est_velocity_x_meters_per_s == 16.0
    assert packet.est_velocity_y_meters_per_s == 17.0
    assert packet.est_velocity_z_meters_per_s == 18.0
    assert packet.est_acceleration_x_gs == 19.0
    assert packet.est_acceleration_y_gs == 20.0
    assert packet.est_acceleration_z_gs == 21.0
    assert packet.est_angular_rate_x_rad_per_s == 22.0
    assert packet.est_angular_rate_y_rad_per_s == 23.0
    assert packet.est_angular_rate_z_rad_per_s == 24.0
    assert packet.est_quaternion_w == 25.0
    assert packet.est_quaternion_x == 26.0
    assert packet.est_quaternion_y == 27.0
    assert packet.est_quaternion_z == 28.0


def test_firm_data_packet_default_zero() -> None:
    firm_data_packet = FIRMDataPacket.default_zero()

    assert firm_data_packet.timestamp_seconds == 0.0
    assert firm_data_packet.temperature_celsius == 0.0
    assert firm_data_packet.pressure_pascals == 0.0
    assert firm_data_packet.raw_acceleration_x_gs == 0.0
    assert firm_data_packet.raw_acceleration_y_gs == 0.0
    assert firm_data_packet.raw_acceleration_z_gs == 0.0
    assert firm_data_packet.raw_angular_rate_x_deg_per_s == 0.0
    assert firm_data_packet.raw_angular_rate_y_deg_per_s == 0.0
    assert firm_data_packet.raw_angular_rate_z_deg_per_s == 0.0
    assert firm_data_packet.magnetic_field_x_microteslas == 0.0
    assert firm_data_packet.magnetic_field_y_microteslas == 0.0
    assert firm_data_packet.magnetic_field_z_microteslas == 0.0
    assert firm_data_packet.est_position_x_meters == 0.0
    assert firm_data_packet.est_position_y_meters == 0.0
    assert firm_data_packet.est_position_z_meters == 0.0
    assert firm_data_packet.est_velocity_x_meters_per_s == 0.0
    assert firm_data_packet.est_velocity_y_meters_per_s == 0.0
    assert firm_data_packet.est_velocity_z_meters_per_s == 0.0
    assert firm_data_packet.est_acceleration_x_gs == 0.0
    assert firm_data_packet.est_acceleration_y_gs == 0.0
    assert firm_data_packet.est_acceleration_z_gs == 0.0
    assert firm_data_packet.est_angular_rate_x_rad_per_s == 0.0
    assert firm_data_packet.est_angular_rate_y_rad_per_s == 0.0
    assert firm_data_packet.est_angular_rate_z_rad_per_s == 0.0
    assert firm_data_packet.est_quaternion_w == 1.0
    assert firm_data_packet.est_quaternion_x == 0.0
    assert firm_data_packet.est_quaternion_y == 0.0
    assert firm_data_packet.est_quaternion_z == 0.0


def test_firm_data_packet_struct_fields() -> None:
    fields = FIRMDataPacket.__struct_fields__

    assert isinstance(fields, list)

    sig = inspect.signature(FIRMDataPacket)
    constructor_params = list(sig.parameters.keys())

    assert fields == constructor_params


def test_firm_data_packet_as_dict() -> None:
    packet = FIRMDataPacket(
        timestamp_seconds=1.0,
        temperature_celsius=2.0,
        pressure_pascals=3.0,
        raw_acceleration_x_gs=4.0,
        raw_acceleration_y_gs=5.0,
        raw_acceleration_z_gs=6.0,
        raw_angular_rate_x_deg_per_s=7.0,
        raw_angular_rate_y_deg_per_s=8.0,
        raw_angular_rate_z_deg_per_s=9.0,
        magnetic_field_x_microteslas=10.0,
        magnetic_field_y_microteslas=11.0,
        magnetic_field_z_microteslas=12.0,
        est_position_x_meters=13.0,
        est_position_y_meters=14.0,
        est_position_z_meters=15.0,
        est_velocity_x_meters_per_s=16.0,
        est_velocity_y_meters_per_s=17.0,
        est_velocity_z_meters_per_s=18.0,
        est_acceleration_x_gs=19.0,
        est_acceleration_y_gs=20.0,
        est_acceleration_z_gs=21.0,
        est_angular_rate_x_rad_per_s=22.0,
        est_angular_rate_y_rad_per_s=23.0,
        est_angular_rate_z_rad_per_s=24.0,
        est_quaternion_w=25.0,
        est_quaternion_x=26.0,
        est_quaternion_y=27.0,
        est_quaternion_z=28.0,
    )

    data_dict = packet.as_dict()

    assert isinstance(data_dict, dict)

    assert set(data_dict.keys()) == set(FIRMDataPacket.__struct_fields__)

    assert data_dict["timestamp_seconds"] == 1.0
    assert data_dict["temperature_celsius"] == 2.0
    assert data_dict["est_quaternion_z"] == 28.0

    # Make sure modifying the dict does not affect the original packet
    data_dict["timestamp_seconds"] = 999.9
    assert packet.timestamp_seconds == 1.0
