use glam::{Vec3, Vec4Swizzles};

use crate::CameraData;

// TODO: Tests for this?
// Fast plane extraction (Gribb/Hartmann) and math from here:
// https://www.gamedevs.org/uploads/fast-extraction-viewing-frustum-planes-from-world-view-projection-matrix.pdf
pub fn is_within_frustum(min_xyz: Vec3, max_xyz: Vec3, camera: &CameraData) -> bool {
    // World space clipping planes.
    // Use the Direct3D example since WebGPU clip space Z is in the range 0.0 to 1.0.
    let matrix = camera.view_projection;
    let left = matrix.row(3) + matrix.row(0);
    let right = matrix.row(3) - matrix.row(0);
    let bottom = matrix.row(3) + matrix.row(1);
    let top = matrix.row(3) - matrix.row(1);
    let near = matrix.row(3);
    let far = matrix.row(3) - matrix.row(2);

    // Normalize the planes.
    let planes = [
        left / left.xyz().length(),
        right / right.xyz().length(),
        bottom / bottom.xyz().length(),
        top / top.xyz().length(),
        near / near.xyz().length(),
        far / far.xyz().length(),
    ];

    // Convert the bounding box to a bounding sphere for more reliable culling.
    let center = (min_xyz + max_xyz) / 2.0;
    let radius = max_xyz.distance(center);

    for plane in planes {
        let signed_distance = plane.dot(center.extend(1.0));
        if signed_distance < -radius {
            return false;
        }
    }

    true
}
