const SPECIFIC_HEAT_RATIO_AIR: f32 = 1.4;
const SPECIFIC_GAS_CONSTANT_AIR_J_PER_KG_K: f32 = 287.05;
const CELSIUS_TO_KELVIN_OFFSET: f32 = 273.15;
const RAD_TO_DEG: f32 = 180.0 / core::f32::consts::PI;

/// Rotates a body-frame acceleration vector using an orientation quaternion.
///
/// The quaternion is normalized internally, and treated as a world-to-body
/// rotation estimate, so we apply its conjugate to map body vectors to world.
pub fn derive_rotated_raw_acceleration(
    raw_acceleration_x_gs: f32,
    raw_acceleration_y_gs: f32,
    raw_acceleration_z_gs: f32,
    est_quaternion_w: f32,
    est_quaternion_x: f32,
    est_quaternion_y: f32,
    est_quaternion_z: f32,
) -> (f32, f32, f32) {
    if !raw_acceleration_x_gs.is_finite()
        || !raw_acceleration_y_gs.is_finite()
        || !raw_acceleration_z_gs.is_finite()
        || !est_quaternion_w.is_finite()
        || !est_quaternion_x.is_finite()
        || !est_quaternion_y.is_finite()
        || !est_quaternion_z.is_finite()
    {
        return (0.0, 0.0, 0.0);
    }

    let quaternion_norm = (est_quaternion_w * est_quaternion_w
        + est_quaternion_x * est_quaternion_x
        + est_quaternion_y * est_quaternion_y
        + est_quaternion_z * est_quaternion_z)
        .sqrt();

    if quaternion_norm <= f32::EPSILON {
        return (
            raw_acceleration_x_gs,
            raw_acceleration_y_gs,
            raw_acceleration_z_gs,
        );
    }

    let qw = est_quaternion_w / quaternion_norm;
    // Use conjugate(q) to rotate body -> world when q is world -> body.
    let qx = -est_quaternion_x / quaternion_norm;
    let qy = -est_quaternion_y / quaternion_norm;
    let qz = -est_quaternion_z / quaternion_norm;

    let vx = raw_acceleration_x_gs;
    let vy = raw_acceleration_y_gs;
    let vz = raw_acceleration_z_gs;

    // Optimized quaternion-vector rotation:
    // t = 2 * cross(q.xyz, v)
    // v' = v + qw * t + cross(q.xyz, t)
    let tx = 2.0 * (qy * vz - qz * vy);
    let ty = 2.0 * (qz * vx - qx * vz);
    let tz = 2.0 * (qx * vy - qy * vx);

    let rotated_x = vx + qw * tx + (qy * tz - qz * ty);
    let rotated_y = vy + qw * ty + (qz * tx - qx * tz);
    let rotated_z = vz + qw * tz + (qx * ty - qy * tx);

    (rotated_x, rotated_y, rotated_z)
}

/// Computes total tilt angle (degrees) between an acceleration vector and +Z.
pub fn derive_tilt_angle_degrees(
    raw_acceleration_x_gs: f32,
    raw_acceleration_y_gs: f32,
    raw_acceleration_z_gs: f32,
) -> f32 {
    if !raw_acceleration_x_gs.is_finite()
        || !raw_acceleration_y_gs.is_finite()
        || !raw_acceleration_z_gs.is_finite()
    {
        return 0.0;
    }

    let magnitude = (raw_acceleration_x_gs * raw_acceleration_x_gs
        + raw_acceleration_y_gs * raw_acceleration_y_gs
        + raw_acceleration_z_gs * raw_acceleration_z_gs)
        .sqrt();

    if magnitude <= f32::EPSILON {
        return 0.0;
    }

    let cos_theta = (raw_acceleration_z_gs / magnitude).clamp(-1.0, 1.0);
    cos_theta.acos() * RAD_TO_DEG
}

/// Computes Mach number from estimated velocity magnitude and ambient temperature.
pub fn derive_mach_number(
    est_velocity_x_meters_per_s: f32,
    est_velocity_y_meters_per_s: f32,
    est_velocity_z_meters_per_s: f32,
    temperature_celsius: f32,
) -> f32 {
    if !est_velocity_x_meters_per_s.is_finite()
        || !est_velocity_y_meters_per_s.is_finite()
        || !est_velocity_z_meters_per_s.is_finite()
        || !temperature_celsius.is_finite()
    {
        return 0.0;
    }

    let temperature_kelvin = temperature_celsius + CELSIUS_TO_KELVIN_OFFSET;
    if temperature_kelvin <= 0.0 {
        return 0.0;
    }

    let speed_of_sound_m_per_s =
        (SPECIFIC_HEAT_RATIO_AIR * SPECIFIC_GAS_CONSTANT_AIR_J_PER_KG_K * temperature_kelvin)
            .sqrt();
    if speed_of_sound_m_per_s <= f32::EPSILON {
        return 0.0;
    }

    let speed_m_per_s = (est_velocity_x_meters_per_s * est_velocity_x_meters_per_s
        + est_velocity_y_meters_per_s * est_velocity_y_meters_per_s
        + est_velocity_z_meters_per_s * est_velocity_z_meters_per_s)
        .sqrt();

    speed_m_per_s / speed_of_sound_m_per_s
}

#[cfg(test)]
mod tests {
    use super::{derive_mach_number, derive_rotated_raw_acceleration, derive_tilt_angle_degrees};

    fn rotate_with_quaternion(
        vx: f32,
        vy: f32,
        vz: f32,
        qw: f32,
        qx: f32,
        qy: f32,
        qz: f32,
    ) -> (f32, f32, f32) {
        let tx = 2.0 * (qy * vz - qz * vy);
        let ty = 2.0 * (qz * vx - qx * vz);
        let tz = 2.0 * (qx * vy - qy * vx);

        let rotated_x = vx + qw * tx + (qy * tz - qz * ty);
        let rotated_y = vy + qw * ty + (qz * tx - qx * tz);
        let rotated_z = vz + qw * tz + (qx * ty - qy * tx);

        (rotated_x, rotated_y, rotated_z)
    }

    #[test]
    fn test_derive_rotated_raw_acceleration_identity_quaternion() {
        let (x, y, z) = derive_rotated_raw_acceleration(0.1, -0.2, 1.0, 1.0, 0.0, 0.0, 0.0);
        assert!((x - 0.1).abs() < 1e-6);
        assert!((y + 0.2).abs() < 1e-6);
        assert!((z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_derive_rotated_raw_acceleration_undoes_world_to_body_rotation() {
        // Simulate gravity in world frame and a 45 deg tilt about Y represented as world->body.
        let world_gravity = (0.0f32, 0.0f32, 1.0f32);
        let half_angle = core::f32::consts::FRAC_PI_4 / 2.0;
        let qw = half_angle.cos();
        let qx = 0.0;
        let qy = half_angle.sin();
        let qz = 0.0;

        // Sensor/body reading is world gravity rotated by world->body quaternion.
        let (body_x, body_y, body_z) = rotate_with_quaternion(
            world_gravity.0,
            world_gravity.1,
            world_gravity.2,
            qw,
            qx,
            qy,
            qz,
        );

        // Deriver should rotate body reading back into world frame.
        let (world_x, world_y, world_z) =
            derive_rotated_raw_acceleration(body_x, body_y, body_z, qw, qx, qy, qz);

        assert!(world_x.abs() < 1e-5);
        assert!(world_y.abs() < 1e-5);
        assert!((world_z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_derive_tilt_angle_degrees_for_aligned_and_perpendicular() {
        let aligned = derive_tilt_angle_degrees(0.0, 0.0, 1.0);
        assert!(aligned.abs() < 1e-6);

        let perpendicular = derive_tilt_angle_degrees(1.0, 0.0, 0.0);
        assert!((perpendicular - 90.0).abs() < 1e-5);
    }

    #[test]
    fn test_derive_mach_number_zero_velocity() {
        let mach = derive_mach_number(0.0, 0.0, 0.0, 20.0);
        assert!(mach.abs() < 1e-6);
    }
}
