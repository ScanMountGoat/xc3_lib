use glam::{vec4, Mat4, Vec3};

// Frustum culling algorithm adapted from https://bruop.github.io/frustum_culling/.
pub fn is_within_frustum(min_xyz: Vec3, max_xyz: Vec3, model_view_projection: Mat4) -> bool {
    let corners = [
        vec4(min_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, min_xyz.z, 1.0),
        vec4(min_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, min_xyz.y, max_xyz.z, 1.0),
        vec4(min_xyz.x, max_xyz.y, max_xyz.z, 1.0),
        vec4(max_xyz.x, max_xyz.y, max_xyz.z, 1.0),
    ];

    // In clip space, all points in the view frustum satisfy these inequalities:
    // -w <= x <= w
    // -w <= y <= w
    // 0 <= z <= w
    let within = |a, b, c| b >= a && b <= c;
    corners.into_iter().any(|c| {
        let corner = model_view_projection * c;
        within(-corner.w, corner.x, corner.w)
            && within(-corner.w, corner.y, corner.w)
            && within(0.0, corner.z, corner.w)
    })
}
