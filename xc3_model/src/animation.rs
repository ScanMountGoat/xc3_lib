use glam::{vec4, Quat, Vec3, Vec4, Vec4Swizzles};
pub use xc3_lib::bc::{murmur3, BlendMode, PlayMode, SpaceMode};

pub struct Animation {
    pub name: String,
    pub space_mode: SpaceMode,
    pub play_mode: PlayMode,
    pub blend_mode: BlendMode,
    pub frames_per_second: f32,
    pub frame_count: u32,
    pub tracks: Vec<Track>,
}

pub struct Track {
    pub translation_keyframes: Vec<Keyframe>,
    pub rotation_keyframes: Vec<Keyframe>,
    pub scale_keyframes: Vec<Keyframe>,
    // TODO: Make this an enum instead?
    pub bone_index: Option<usize>,
    pub bone_hash: Option<u32>,
}

// TODO: Should this always be cubic?
// TODO: Separate type for vec3 and quaternion?
pub struct Keyframe {
    pub frame: f32,
    pub x_coeffs: Vec4,
    pub y_coeffs: Vec4,
    pub z_coeffs: Vec4,
    pub w_coeffs: Vec4,
}

impl Animation {
    pub fn from_anim(anim: &xc3_lib::bc::Anim) -> Self {
        Self {
            name: anim.binding.animation.name.clone(),
            space_mode: anim.binding.animation.space_mode,
            play_mode: anim.binding.animation.play_mode,
            blend_mode: anim.binding.animation.blend_mode,
            frames_per_second: anim.binding.animation.frames_per_second,
            frame_count: anim.binding.animation.frame_count,
            tracks: anim_tracks(anim),
        }
    }
}

fn anim_tracks(anim: &xc3_lib::bc::Anim) -> Vec<Track> {
    match &anim.binding.animation.data {
        xc3_lib::bc::AnimationData::Uncompressed(uncompressed) => {
            if let xc3_lib::bc::ExtraTrackAnimation::Uncompressed(extra) =
                &anim.binding.extra_track_animation
            {
                // TODO: Apply the root motion at each frame?
                let hashes = &extra.unk3.bone_name_hashes;
                let track_count = hashes.len();
                let transforms = &uncompressed.transforms.elements;
                hashes
                    .iter()
                    .enumerate()
                    .map(|(i, hash)| {
                        let mut translation_keyframes = Vec::new();
                        let mut rotation_keyframes = Vec::new();
                        let mut scale_keyframes = Vec::new();

                        // Assume each bone has a transform for each frame in order.
                        for frame in 0..anim.binding.animation.frame_count {
                            let index = frame as usize * track_count + i;
                            let next_index = (frame as usize + 1) * track_count + i;

                            // Convert to cubic instead of having separate interpolation types.
                            let translation = transforms[index].translation;
                            let next_translation =
                                transforms.get(next_index).map(|t| t.translation);
                            translation_keyframes.push(linear_to_cubic_keyframe(
                                frame,
                                translation,
                                next_translation,
                            ));

                            let rotation = transforms[index].rotation_quaternion;
                            let next_rotation =
                                transforms.get(next_index).map(|t| t.rotation_quaternion);
                            rotation_keyframes.push(linear_to_cubic_keyframe(
                                frame,
                                rotation,
                                next_rotation,
                            ));

                            let scale = transforms[index].scale;
                            let next_scale = transforms.get(next_index).map(|t| t.scale);
                            scale_keyframes
                                .push(linear_to_cubic_keyframe(frame, scale, next_scale));
                        }

                        Track {
                            translation_keyframes,
                            rotation_keyframes,
                            scale_keyframes,
                            bone_index: None,
                            bone_hash: Some(*hash),
                        }
                    })
                    .collect()
            } else {
                // TODO: error?
                Vec::new()
            }
        }
        xc3_lib::bc::AnimationData::Cubic(cubic) => {
            // TODO: Assigns bones to tracks?
            // TODO: Doesn't work for mio anim 0?
            // TODO: bone names replace the ordering of bones if present?

            // TODO: Some anims have more tracks than bones for bl200202?
            anim.binding
                .bone_track_indices
                .elements
                .iter()
                .enumerate()
                .filter_map(|(i, index)| {
                    let mut translation_keyframes = Vec::new();
                    let mut rotation_keyframes = Vec::new();
                    let mut scale_keyframes = Vec::new();

                    // TODO: How to handle index values of -1?
                    // TODO: Not all bones are being animated properly?
                    if *index >= 0 {
                        let track = &cubic.tracks.elements[*index as usize];

                        // TODO: Functions for these?
                        for keyframe in &track.translation.elements {
                            translation_keyframes.push(Keyframe {
                                frame: keyframe.frame,
                                x_coeffs: keyframe.x.into(),
                                y_coeffs: keyframe.y.into(),
                                z_coeffs: keyframe.z.into(),
                                w_coeffs: Vec4::ZERO,
                            });
                        }
                        for keyframe in &track.rotation.elements {
                            rotation_keyframes.push(Keyframe {
                                frame: keyframe.frame,
                                x_coeffs: keyframe.x.into(),
                                y_coeffs: keyframe.y.into(),
                                z_coeffs: keyframe.z.into(),
                                w_coeffs: keyframe.w.into(),
                            });
                        }
                        for keyframe in &track.scale.elements {
                            scale_keyframes.push(Keyframe {
                                frame: keyframe.frame,
                                x_coeffs: keyframe.x.into(),
                                y_coeffs: keyframe.y.into(),
                                z_coeffs: keyframe.z.into(),
                                w_coeffs: Vec4::ZERO,
                            });
                        }

                        // TODO: Map tracks to bones instead of creating a track for each bone?
                        Some(Track {
                            translation_keyframes,
                            rotation_keyframes,
                            scale_keyframes,
                            bone_index: Some(i),
                            bone_hash: None,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        xc3_lib::bc::AnimationData::Empty => {
            // TODO: how to handle this?
            Vec::new()
        }
        xc3_lib::bc::AnimationData::PackedCubic(cubic) => {
            // TODO: Does each of these tracks have a corresponding hash?
            // TODO: Also check the bone indices?
            if let xc3_lib::bc::ExtraTrackAnimation::PackedCubic(extra) =
                &anim.binding.extra_track_animation
            {
                cubic
                    .tracks
                    .elements
                    .iter()
                    .zip(extra.data.bone_name_hashes.iter())
                    .map(|(track, hash)| {
                        let translation_keyframes = packed_cubic_vec3_keyframes(
                            &track.translation,
                            &cubic.keyframes.elements,
                            &cubic.vectors.elements,
                        );
                        let rotation_keyframes = packed_cubic_vec4_keyframes(
                            &track.rotation,
                            &cubic.keyframes.elements,
                            &cubic.quaternions.elements,
                        );
                        let scale_keyframes = packed_cubic_vec3_keyframes(
                            &track.scale,
                            &cubic.keyframes.elements,
                            &cubic.vectors.elements,
                        );

                        Track {
                            translation_keyframes,
                            rotation_keyframes,
                            scale_keyframes,
                            bone_index: None,
                            bone_hash: Some(*hash),
                        }
                    })
                    .collect()
            } else {
                // TODO: error?
                Vec::new()
            }
        }
    }
}

fn linear_to_cubic_keyframe(
    frame: u32,
    current_frame: [f32; 4],
    next_frame: Option<[f32; 4]>,
) -> Keyframe {
    match next_frame {
        Some(next_frame) => {
            // Linearly interpolate between this frame and the next.
            // Assume the next frame is at frame + 1.0.
            let delta = Vec4::from(next_frame) - Vec4::from(current_frame);
            Keyframe {
                frame: frame as f32,
                x_coeffs: vec4(0.0, 0.0, delta.x, current_frame[0]),
                y_coeffs: vec4(0.0, 0.0, delta.y, current_frame[1]),
                z_coeffs: vec4(0.0, 0.0, delta.z, current_frame[2]),
                w_coeffs: vec4(0.0, 0.0, delta.w, current_frame[3]),
            }
        }
        None => Keyframe {
            frame: frame as f32,
            x_coeffs: vec4(0.0, 0.0, 0.0, current_frame[0]),
            y_coeffs: vec4(0.0, 0.0, 0.0, current_frame[1]),
            z_coeffs: vec4(0.0, 0.0, 0.0, current_frame[2]),
            w_coeffs: vec4(0.0, 0.0, 0.0, current_frame[3]),
        },
    }
}

fn packed_cubic_vec3_keyframes(
    sub_track: &xc3_lib::bc::SubTrack,
    keyframe_times: &[u16],
    coeffs: &[[f32; 4]],
) -> Vec<Keyframe> {
    (sub_track.keyframe_start_index..sub_track.keyframe_end_index)
        .enumerate()
        .map(|(i, keyframe_index)| {
            let start_index = sub_track.curves_start_index as usize + i * 3;
            Keyframe {
                frame: keyframe_times[keyframe_index as usize] as f32,
                x_coeffs: coeffs[start_index].into(),
                y_coeffs: coeffs[start_index + 1].into(),
                z_coeffs: coeffs[start_index + 2].into(),
                w_coeffs: Vec4::ZERO,
            }
        })
        .collect()
}

fn packed_cubic_vec4_keyframes(
    sub_track: &xc3_lib::bc::SubTrack,
    keyframe_times: &[u16],
    coeffs: &[[f32; 4]],
) -> Vec<Keyframe> {
    (sub_track.keyframe_start_index..sub_track.keyframe_end_index)
        .enumerate()
        .map(|(i, keyframe_index)| {
            let start_index = sub_track.curves_start_index as usize + i * 4;
            Keyframe {
                frame: keyframe_times[keyframe_index as usize] as f32,
                x_coeffs: coeffs[start_index].into(),
                y_coeffs: coeffs[start_index + 1].into(),
                z_coeffs: coeffs[start_index + 2].into(),
                w_coeffs: coeffs[start_index + 3].into(),
            }
        })
        .collect()
}

impl Track {
    pub fn sample_translation(&self, frame: f32) -> Vec3 {
        sample_keyframe_cubic(&self.translation_keyframes, frame).xyz()
    }

    pub fn sample_rotation(&self, frame: f32) -> Quat {
        Quat::from_array(sample_keyframe_cubic(&self.rotation_keyframes, frame).to_array())
    }

    pub fn sample_scale(&self, frame: f32) -> Vec3 {
        sample_keyframe_cubic(&self.scale_keyframes, frame).xyz()
    }
}

// TODO: Add tests for this.
fn sample_keyframe_cubic(values: &[Keyframe], frame: f32) -> Vec4 {
    // Assume the keyframes are in ascending order.
    // TODO: Avoid allocating here.
    let keyframes: Vec<_> = values.iter().map(|v| v.frame).collect();
    let (keyframe_index, x) = keyframe_index_position(&keyframes, frame);

    vec4(
        interpolate_cubic(values[keyframe_index].x_coeffs, x),
        interpolate_cubic(values[keyframe_index].y_coeffs, x),
        interpolate_cubic(values[keyframe_index].z_coeffs, x),
        interpolate_cubic(values[keyframe_index].w_coeffs, x),
    )
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

fn interpolate_cubic(coeffs: Vec4, x: f32) -> f32 {
    coeffs.x * (x * x * x) + coeffs.y * (x * x) + coeffs.z * x + coeffs.w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_cubic_values() {
        let coeffs = vec4(1.0, 2.0, 3.0, 4.0);
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
