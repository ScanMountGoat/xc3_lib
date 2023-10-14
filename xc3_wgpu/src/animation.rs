use std::collections::HashMap;

use glam::{vec3, Mat4, Quat, Vec3};
use log::error;
use xc3_lib::bc::murmur3;

pub fn animate_skeleton(
    skeleton: &xc3_model::Skeleton,
    anim: &xc3_lib::bc::Anim,
    frame: f32,
) -> [Mat4; 256] {
    // TODO: Is it worth precomputing this?
    let hash_to_index: HashMap<_, _> = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(i, b)| (murmur3(b.name.as_bytes()), i))
        .collect();

    // Just create a copy of the skeleton to simplify the code for now.
    let mut animated_skeleton = skeleton.clone();

    // TODO: Load all key frames?
    match &anim.binding.animation.data {
        xc3_lib::bc::AnimationData::Uncompressed(uncompressed) => {
            if let xc3_lib::bc::ExtraTrackAnimation::Uncompressed(extra) =
                &anim.binding.extra_track_animation
            {
                // TODO: Are the transforms in order for each hash for each frame?
                // TODO: Correctly interpolate keyframes.
                let frame_index = frame as usize;
                let start_index = frame_index * extra.unk3.bone_name_hashes.len();

                // TODO: Apply the root motion at each frame?
                for (transform, hash) in uncompressed
                    .transforms
                    .elements
                    .iter()
                    .skip(start_index)
                    .zip(extra.unk3.bone_name_hashes.iter())
                {
                    if let Some(bone_index) = hash_to_index.get(hash) {
                        let translation = vec3(
                            transform.translation[0],
                            transform.translation[1],
                            transform.translation[2],
                        );
                        let rotation = Quat::from_xyzw(
                            transform.rotation_quaternion[0],
                            transform.rotation_quaternion[1],
                            transform.rotation_quaternion[2],
                            transform.rotation_quaternion[3],
                        );
                        let scale =
                            vec3(transform.scale[0], transform.scale[1], transform.scale[2]);

                        let transform = Mat4::from_translation(translation)
                            * Mat4::from_quat(rotation)
                            * Mat4::from_scale(scale);
                        animated_skeleton.bones[*bone_index].transform = transform;
                    } else {
                        error!("No matching bone for hash {hash:x}");
                    }
                }
            }
        }
        xc3_lib::bc::AnimationData::Cubic(cubic) => {
            for (track, bone_index) in cubic
                .tracks
                .elements
                .iter()
                .zip(anim.binding.bone_indices.elements.iter())
            {
                // TODO: How to handle index values of -1?
                // TODO: Not all bones are being animated properly?
                if *bone_index >= 0 {
                    let translation = sample_vec3_cubic(&track.translation.elements, frame);
                    let rotation = sample_quat_cubic(&track.rotation.elements, frame);
                    let scale = sample_vec3_cubic(&track.scale.elements, frame);

                    let bone_name = &anim.binding.bone_names.elements[*bone_index as usize].name;

                    if let Some(bone_index) = animated_skeleton
                        .bones
                        .iter()
                        .position(|b| &b.name == bone_name)
                    {
                        let transform = Mat4::from_translation(translation)
                            * Mat4::from_quat(rotation)
                            * Mat4::from_scale(scale);
                        animated_skeleton.bones[bone_index].transform = transform;
                    } else {
                        error!("No matching bone for {:?}", bone_name);
                    }
                }
            }
        }
        xc3_lib::bc::AnimationData::Unk2 => todo!(),
        xc3_lib::bc::AnimationData::PackedCubic(cubic) => {
            // TODO: Does each of these tracks have a corresponding hash?
            // TODO: Also check the bone indices?
            if let xc3_lib::bc::ExtraTrackAnimation::PackedCubic(extra) =
                &anim.binding.extra_track_animation
            {
                for (track, hash) in cubic
                    .tracks
                    .elements
                    .iter()
                    .zip(extra.data.bone_name_hashes.elements.iter())
                {
                    // Interpolate based on the current frame.
                    // TODO: Correctly account for animation speed here?
                    let translation = sample_vec3_packed_cubic(cubic, &track.translation, frame);
                    let rotation = sample_quat_packed_cubic(cubic, &track.rotation, frame);
                    let scale = sample_vec3_packed_cubic(cubic, &track.scale, frame);

                    if let Some(bone_index) = hash_to_index.get(hash) {
                        // TODO: Does every track start at time 0?
                        let transform = Mat4::from_translation(translation)
                            * Mat4::from_quat(rotation)
                            * Mat4::from_scale(scale);
                        animated_skeleton.bones[*bone_index].transform = transform;
                    } else {
                        error!("No matching bone for hash {hash:x}");
                    }
                }
            }
        }
    }

    let rest_pose_world = skeleton.world_transforms();
    let animated_world = animated_skeleton.world_transforms();

    let mut animated_transforms = [Mat4::IDENTITY; 256];
    for i in (0..skeleton.bones.len()).take(animated_transforms.len()) {
        animated_transforms[i] = animated_world[i] * rest_pose_world[i].inverse();
    }

    animated_transforms
}

// TODO: Add tests for this.
fn sample_vec3_cubic(values: &[xc3_lib::bc::KeyFrameCubicVec3], frame: f32) -> Vec3 {
    // Assume the keyframes are in ascending order.
    // TODO: Avoid allocating here.
    let keyframes: Vec<_> = values.iter().map(|v| v.frame).collect();
    let (keyframe_index, x) = keyframe_index_position(&keyframes, frame);

    vec3(
        interpolate_cubic(values[keyframe_index].x, x),
        interpolate_cubic(values[keyframe_index].y, x),
        interpolate_cubic(values[keyframe_index].z, x),
    )
}

// TODO: Add tests for this.
fn sample_quat_cubic(values: &[xc3_lib::bc::KeyFrameCubicQuaternion], frame: f32) -> Quat {
    // Assume the keyframes are in ascending order.
    // TODO: Avoid allocating here.
    let keyframes: Vec<_> = values.iter().map(|v| v.frame).collect();
    let (keyframe_index, x) = keyframe_index_position(&keyframes, frame);

    Quat::from_xyzw(
        interpolate_cubic(values[keyframe_index].x, x),
        interpolate_cubic(values[keyframe_index].y, x),
        interpolate_cubic(values[keyframe_index].z, x),
        interpolate_cubic(values[keyframe_index].w, x),
    )
}

fn sample_vec3_packed_cubic(
    cubic: &xc3_lib::bc::PackedCubic,
    sub_track: &xc3_lib::bc::SubTrack,
    frame: f32,
) -> Vec3 {
    let [x, y, z, _] = sample_packed_cubic(
        &cubic.keyframes.elements,
        &cubic.vectors.elements,
        3,
        sub_track,
        frame,
    );
    vec3(x, y, z)
}

fn sample_quat_packed_cubic(
    cubic: &xc3_lib::bc::PackedCubic,
    sub_track: &xc3_lib::bc::SubTrack,
    frame: f32,
) -> Quat {
    let [x, y, z, w] = sample_packed_cubic(
        &cubic.keyframes.elements,
        &cubic.quaternions.elements,
        4,
        sub_track,
        frame,
    );
    Quat::from_xyzw(x, y, z, w)
}

// TODO: Add tests for this.
fn sample_packed_cubic(
    keyframes: &[u16],
    coeffs: &[[f32; 4]],
    component_count: usize,
    sub_track: &xc3_lib::bc::SubTrack,
    frame: f32,
) -> [f32; 4] {
    let track_keyframes =
        &keyframes[sub_track.keyframe_start_index as usize..sub_track.keyframe_end_index as usize];

    let (keyframe_index, x) = keyframe_index_position(track_keyframes, frame);

    let start_index = sub_track.curves_start_index as usize + keyframe_index * component_count;

    let mut value = [0.0; 4];
    for c in 0..component_count {
        value[c] = interpolate_cubic(coeffs[start_index + c], x)
    }

    value
}

fn keyframe_index_position<K>(keyframes: &[K], frame: f32) -> (usize, f32)
where
    K: Into<f32> + PartialEq + Copy,
{
    // Find the keyframe range and position within that range.
    // Assume keyframes are in ascending order.
    // TODO: Is there a way to make this not O(N)?
    let mut keyframe_index = 0;
    let mut position = 0.0;

    for i in 0..keyframes.len() {
        // TODO: Find a cleaner way to handle the final frame.
        let current_frame = keyframes[i];
        let next_frame = *keyframes.get(i + 1).unwrap_or(&current_frame);
        let frame_range = current_frame.into()..=next_frame.into();

        if frame_range.contains(&frame)
            || (current_frame == next_frame && frame > current_frame.into())
        {
            keyframe_index = i;
            // The final keyframe should persist for the rest of the animation.
            position = frame.min(next_frame.into()) - current_frame.into();
        }
    }

    (keyframe_index, position)
}

fn interpolate_cubic(coeffs: [f32; 4], x: f32) -> f32 {
    coeffs[0] * (x * x * x) + coeffs[1] * (x * x) + coeffs[2] * x + coeffs[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_cubic_values() {
        let coeffs = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(4.0, interpolate_cubic(coeffs, 0.0));
        assert_eq!(10.0, interpolate_cubic(coeffs, 1.0));
        assert_eq!(26.0, interpolate_cubic(coeffs, 2.0));
        assert_eq!(58.0, interpolate_cubic(coeffs, 3.0));
    }

    #[test]
    fn index_position_first_keyframe() {
        assert_eq!((0, 0.0), keyframe_index_position(&[0u16, 5u16, 9u16], 0.0));
        assert_eq!((0, 2.5), keyframe_index_position(&[0u16, 5u16, 9u16], 2.5));
        assert_eq!((0, 4.9), keyframe_index_position(&[0u16, 5u16, 9u16], 4.9));
    }

    #[test]
    fn index_position_second_keyframe() {
        assert_eq!((1, 0.0), keyframe_index_position(&[0u16, 5u16, 9u16], 5.0));
        assert_eq!((1, 2.0), keyframe_index_position(&[0u16, 5u16, 9u16], 7.0));
        assert_eq!((1, 3.5), keyframe_index_position(&[0u16, 5u16, 9u16], 8.5));
    }

    #[test]
    fn index_position_last_keyframe() {
        // This should clamp to the final keyframe instead of extrapolating.
        assert_eq!((2, 0.0), keyframe_index_position(&[0u16, 5u16, 9u16], 10.0));
        assert_eq!((2, 0.0), keyframe_index_position(&[0u16, 5u16, 9u16], 12.5));
    }
}
