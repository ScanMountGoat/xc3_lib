use glam::{Vec3, vec3};

/// Frame the region defined by `min_xyz` and `max_xyz`
/// assuming a camera pointing toward the positive z-axis.
pub fn frame_bounds_translation(min_xyz: Vec3, max_xyz: Vec3, fov_y: f32, aspect: f32) -> Vec3 {
    let center = (min_xyz + max_xyz) / 2.0;
    let bounds_size = max_xyz - min_xyz;

    // Find the base of the triangle based on FOV and model width or height.
    // Take the max to frame both horizontally and vertically.
    // TODO: Take near clip into account.
    let fov_x = 2.0 * (aspect * (fov_y * 0.5).tan()).atan();
    let distance_y = (bounds_size.y * 0.5) / (fov_y * 0.5).tan();
    let distance_x = (bounds_size.x * 0.5) / (fov_x * 0.5).tan();
    let distance = distance_x.max(distance_y);

    // This distance should be from the closest object depth.
    vec3(-center.x, -center.y, min_xyz.z - distance)
}
