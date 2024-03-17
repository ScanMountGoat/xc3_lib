//! Utilities for working with animation data.
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound::*;

use glam::{vec4, Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use log::error;
use ordered_float::OrderedFloat;
pub use xc3_lib::bc::anim::{BlendMode, PlayMode, SpaceMode};
pub use xc3_lib::hash::murmur3;

use crate::Skeleton;

#[derive(Debug, PartialEq, Clone)]
pub struct Animation {
    pub name: String,
    /// The space for transforms in [tracks](#structfield.tracks).
    pub space_mode: SpaceMode,
    pub play_mode: PlayMode,
    pub blend_mode: BlendMode,
    pub frames_per_second: f32,
    pub frame_count: u32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    // TODO: Are fractional keyframes used in practice?
    pub translation_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub rotation_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub scale_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub bone_index: BoneIndex,
}

/// Index for selecting the appropriate bone in a [Skeleton](crate::skeleton::Skeleton).
#[derive(Debug, PartialEq, Clone)]
pub enum BoneIndex {
    /// Index into [bones](../skeleton/struct.Skeleton.html#structfield.bones).
    /// Used for Xenoblade 2 animations.
    Index(usize),
    /// [murmur3] hash of the bone in [bones](../skeleton/struct.Skeleton.html#structfield.bones).
    /// Used for Xenoblade 1 DE and Xenoblade 3 animations.
    Hash(u32),
    /// Name of the bone in [bones](../skeleton/struct.Skeleton.html#structfield.bones).
    /// Used for Xenoblade 1 DE and Xenoblade 3 animations.
    Name(String),
}

// TODO: Should this always be cubic?
// TODO: Separate type for vec3 and quaternion?
#[derive(Debug, PartialEq, Clone)]
pub struct Keyframe {
    pub x_coeffs: Vec4,
    pub y_coeffs: Vec4,
    pub z_coeffs: Vec4,
    pub w_coeffs: Vec4,
}

impl Animation {
    pub fn from_anim(anim: &xc3_lib::bc::anim::Anim) -> Self {
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

    /// Convert `current_time_seconds` to frames based on the animation parameters.
    pub fn current_frame(&self, current_time_seconds: f32) -> f32 {
        // TODO: looping?
        current_time_seconds * self.frames_per_second
    }

    // TODO: Tests for this.
    /// Compute the matrix for each bone in `skeleton`
    /// that transforms a vertex in model space to its animated position in model space.
    ///
    /// This can be used in a vertex shader to apply linear blend skinning
    /// by transforming the vertex by up to 4 skinning matrices
    /// and blending with vertex skin weights.
    ///
    /// The in game skinning code precomputes a slightly different matrix.
    /// See [Weights](crate::Weights) for details.
    pub fn skinning_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Mat4> {
        let anim_transforms = self.model_space_transforms(skeleton, frame);
        let bind_transforms = skeleton.model_space_transforms();

        let mut animated_transforms = vec![Mat4::IDENTITY; skeleton.bones.len()];
        for i in (0..skeleton.bones.len()).take(animated_transforms.len()) {
            let inverse_bind = bind_transforms[i].inverse();
            animated_transforms[i] = anim_transforms[i] * inverse_bind;
        }

        animated_transforms
    }

    // TODO: Tests for this.
    /// Compute the the animated transform in model space for each bone in `skeleton`.
    ///
    /// See [Skeleton::model_space_transforms] for the transforms without animations applied.
    pub fn model_space_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Mat4> {
        // TODO: Is it worth precomputing this?
        let hash_to_index: HashMap<_, _> = skeleton
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (murmur3(b.name.as_bytes()), i))
            .collect();

        // Keep track of which bones have animations applied.
        let mut animated_transforms = vec![None; skeleton.bones.len()];

        for track in &self.tracks {
            if let Some(bone_index) = match &track.bone_index {
                BoneIndex::Index(i) => Some(*i),
                BoneIndex::Hash(hash) => hash_to_index.get(hash).copied(),
                BoneIndex::Name(name) => skeleton.bones.iter().position(|b| &b.name == name),
            } {
                let translation = track.sample_translation(frame);
                let rotation = track.sample_rotation(frame);
                let scale = track.sample_scale(frame);

                let transform = Mat4::from_translation(translation)
                    * Mat4::from_quat(rotation)
                    * Mat4::from_scale(scale);

                if bone_index < skeleton.bones.len() {
                    animated_transforms[bone_index] = Some(apply_transform(
                        skeleton.bones[bone_index].transform,
                        transform,
                        self.blend_mode,
                    ));
                } else {
                    // TODO: Why does this happen?
                    error!(
                        "Bone index {bone_index} out of range for {} bones",
                        skeleton.bones.len()
                    );
                }
            } else {
                // TODO: Why does this happen?
                error!("No matching bone for {:?}", track.bone_index);
            }
        }

        let rest_pose_model_space = skeleton.model_space_transforms();

        // Assume parents appear before their children.
        // TODO: Does this code correctly handle all cases?
        let mut anim_model_space = rest_pose_model_space.clone();
        match self.space_mode {
            SpaceMode::Local => {
                for i in 0..anim_model_space.len() {
                    match animated_transforms[i] {
                        Some(transform) => {
                            // Local space is relative to the parent bone.
                            if let Some(parent) = skeleton.bones[i].parent_index {
                                anim_model_space[i] = anim_model_space[parent] * transform;
                            } else {
                                anim_model_space[i] = transform;
                            }
                        }
                        None => {
                            if let Some(parent) = skeleton.bones[i].parent_index {
                                anim_model_space[i] =
                                    anim_model_space[parent] * skeleton.bones[i].transform;
                            }
                        }
                    }
                }
            }
            SpaceMode::Model => {
                for i in 0..anim_model_space.len() {
                    // Model space is relative to the model root.
                    // This is faster to compute but rarely used by animation files.
                    match animated_transforms[i] {
                        Some(transform) => {
                            anim_model_space[i] = transform;
                        }
                        None => {
                            if let Some(parent) = skeleton.bones[i].parent_index {
                                anim_model_space[i] =
                                    anim_model_space[parent] * skeleton.bones[i].transform;
                            }
                        }
                    }
                }
            }
        }

        anim_model_space
    }
}

fn anim_tracks(anim: &xc3_lib::bc::anim::Anim) -> Vec<Track> {
    // Tracks are assigned to bones using indices, names, or name hashes.
    // Tracks have optional data depending on the anim type and game version.
    // This makes the conversion to a unified format somewhat complex.
    let (bone_names, hashes) = names_hashes(anim);

    match &anim.binding.animation.data {
        xc3_lib::bc::anim::AnimationData::Uncompressed(uncompressed) => {
            // TODO: Apply the root motion at each frame?
            let track_count = anim.binding.bone_track_indices.elements.len();
            let transforms = &uncompressed.transforms;

            // TODO: Are these always the elements 0..N-1?
            anim.binding
                .bone_track_indices
                .elements
                .iter()
                .map(|i| {
                    let mut translation_keyframes = BTreeMap::new();
                    let mut rotation_keyframes = BTreeMap::new();
                    let mut scale_keyframes = BTreeMap::new();

                    for frame in 0..anim.binding.animation.frame_count {
                        let index = frame as usize * track_count + *i as usize;
                        let next_index = (frame as usize + 1) * track_count + *i as usize;

                        // Convert to cubic instead of having separate interpolation types.
                        let translation = transforms[index].translation;
                        let next_translation = transforms.get(next_index).map(|t| t.translation);
                        translation_keyframes.insert(
                            (frame as f32).into(),
                            linear_to_cubic_keyframe(translation, next_translation),
                        );

                        let rotation = transforms[index].rotation_quaternion;
                        let next_rotation =
                            transforms.get(next_index).map(|t| t.rotation_quaternion);
                        rotation_keyframes.insert(
                            (frame as f32).into(),
                            linear_to_cubic_keyframe(rotation, next_rotation),
                        );

                        let scale = transforms[index].scale;
                        let next_scale = transforms.get(next_index).map(|t| t.scale);
                        scale_keyframes.insert(
                            (frame as f32).into(),
                            linear_to_cubic_keyframe(scale, next_scale),
                        );
                    }

                    let bone_index = track_bone_index(*i as usize, bone_names, hashes);

                    Track {
                        translation_keyframes,
                        rotation_keyframes,
                        scale_keyframes,
                        bone_index,
                    }
                })
                .collect()
        }

        xc3_lib::bc::anim::AnimationData::Cubic(cubic) => {
            // TODO: Some anims have more tracks than bones for bl200202?
            anim.binding
                .bone_track_indices
                .elements
                .iter()
                .enumerate()
                .filter_map(|(i, index)| {
                    let mut translation_keyframes = BTreeMap::new();
                    let mut rotation_keyframes = BTreeMap::new();
                    let mut scale_keyframes = BTreeMap::new();

                    // TODO: How to handle index values of -1?
                    if *index >= 0 {
                        let track = &cubic.tracks.elements[*index as usize];

                        // TODO: Functions for these?
                        for keyframe in &track.translation.elements {
                            translation_keyframes.insert(
                                keyframe.frame.into(),
                                Keyframe {
                                    x_coeffs: keyframe.x.into(),
                                    y_coeffs: keyframe.y.into(),
                                    z_coeffs: keyframe.z.into(),
                                    w_coeffs: Vec4::ZERO,
                                },
                            );
                        }
                        for keyframe in &track.rotation.elements {
                            rotation_keyframes.insert(
                                keyframe.frame.into(),
                                Keyframe {
                                    x_coeffs: keyframe.x.into(),
                                    y_coeffs: keyframe.y.into(),
                                    z_coeffs: keyframe.z.into(),
                                    w_coeffs: keyframe.w.into(),
                                },
                            );
                        }
                        for keyframe in &track.scale.elements {
                            scale_keyframes.insert(
                                keyframe.frame.into(),
                                Keyframe {
                                    x_coeffs: keyframe.x.into(),
                                    y_coeffs: keyframe.y.into(),
                                    z_coeffs: keyframe.z.into(),
                                    w_coeffs: Vec4::ZERO,
                                },
                            );
                        }

                        let bone_index = track_bone_index(i, bone_names, hashes);

                        Some(Track {
                            translation_keyframes,
                            rotation_keyframes,
                            scale_keyframes,
                            bone_index,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        xc3_lib::bc::anim::AnimationData::Empty => {
            // TODO: how to handle this?
            Vec::new()
        }
        xc3_lib::bc::anim::AnimationData::PackedCubic(cubic) => {
            anim.binding
                .bone_track_indices
                .elements
                .iter()
                .map(|i| {
                    // TODO: Will the index ever be negative?
                    let track = &cubic.tracks.elements[*i as usize];

                    let translation_keyframes = packed_cubic_vec3_keyframes(
                        &track.translation,
                        &cubic.keyframes.elements,
                        &cubic.vectors,
                    );
                    let rotation_keyframes = packed_cubic_vec4_keyframes(
                        &track.rotation,
                        &cubic.keyframes.elements,
                        &cubic.quaternions.elements,
                    );
                    let scale_keyframes = packed_cubic_vec3_keyframes(
                        &track.scale,
                        &cubic.keyframes.elements,
                        &cubic.vectors,
                    );

                    let bone_index = track_bone_index(*i as usize, bone_names, hashes);

                    Track {
                        translation_keyframes,
                        rotation_keyframes,
                        scale_keyframes,
                        bone_index,
                    }
                })
                .collect()
        }
    }
}

fn names_hashes(
    anim: &xc3_lib::bc::anim::Anim,
) -> (Option<&Vec<xc3_lib::bc::StringOffset>>, Option<&Vec<u32>>) {
    match &anim.binding.inner {
        xc3_lib::bc::anim::AnimationBindingInner::Unk1(_) => (None, None),
        xc3_lib::bc::anim::AnimationBindingInner::Unk2(inner) => (Some(&inner.bone_names), None),
        xc3_lib::bc::anim::AnimationBindingInner::Unk3(inner) => (
            Some(&inner.bone_names),
            extra_track_hashes(&inner.extra_track_data),
        ),
        xc3_lib::bc::anim::AnimationBindingInner::Unk4(inner) => (
            Some(&inner.bone_names),
            extra_track_hashes(&inner.extra_track_data),
        ),
    }
}

fn track_bone_index(
    i: usize,
    bone_names: Option<&Vec<xc3_lib::bc::StringOffset>>,
    hashes: Option<&Vec<u32>>,
) -> BoneIndex {
    // Some XC1 and XC3 animations don't use the chr bone ordering.
    // XC2 always uses the index directly since it doesn't store names.
    if let Some(name) = bone_names.and_then(|names| names.get(i)) {
        BoneIndex::Name(name.name.clone())
    } else if let Some(hash) = hashes.and_then(|hashes| hashes.get(i)).copied() {
        BoneIndex::Hash(hash)
    } else {
        BoneIndex::Index(i)
    }
}

fn extra_track_hashes(data: &xc3_lib::bc::anim::ExtraTrackData) -> Option<&Vec<u32>> {
    match data {
        xc3_lib::bc::anim::ExtraTrackData::Uncompressed(extra) => {
            Some(&extra.hashes.bone_name_hashes)
        }
        xc3_lib::bc::anim::ExtraTrackData::Cubic(_) => None,
        xc3_lib::bc::anim::ExtraTrackData::Empty => None,
        xc3_lib::bc::anim::ExtraTrackData::PackedCubic(extra) => {
            Some(&extra.hashes.bone_name_hashes)
        }
    }
}

fn linear_to_cubic_keyframe(current_frame: [f32; 4], next_frame: Option<[f32; 4]>) -> Keyframe {
    match next_frame {
        Some(next_frame) => {
            // Linearly interpolate between this frame and the next.
            // Assume the next frame is at frame + 1.0.
            let delta = Vec4::from(next_frame) - Vec4::from(current_frame);
            Keyframe {
                x_coeffs: vec4(0.0, 0.0, delta.x, current_frame[0]),
                y_coeffs: vec4(0.0, 0.0, delta.y, current_frame[1]),
                z_coeffs: vec4(0.0, 0.0, delta.z, current_frame[2]),
                w_coeffs: vec4(0.0, 0.0, delta.w, current_frame[3]),
            }
        }
        None => Keyframe {
            x_coeffs: vec4(0.0, 0.0, 0.0, current_frame[0]),
            y_coeffs: vec4(0.0, 0.0, 0.0, current_frame[1]),
            z_coeffs: vec4(0.0, 0.0, 0.0, current_frame[2]),
            w_coeffs: vec4(0.0, 0.0, 0.0, current_frame[3]),
        },
    }
}

fn packed_cubic_vec3_keyframes(
    sub_track: &xc3_lib::bc::anim::SubTrack,
    keyframe_times: &[u16],
    coeffs: &[[f32; 4]],
) -> BTreeMap<OrderedFloat<f32>, Keyframe> {
    (sub_track.keyframe_start_index..sub_track.keyframe_end_index)
        .enumerate()
        .map(|(i, keyframe_index)| {
            let start_index = sub_track.curves_start_index as usize + i * 3;
            (
                (keyframe_times[keyframe_index as usize] as f32).into(),
                Keyframe {
                    x_coeffs: coeffs[start_index].into(),
                    y_coeffs: coeffs[start_index + 1].into(),
                    z_coeffs: coeffs[start_index + 2].into(),
                    w_coeffs: Vec4::ZERO,
                },
            )
        })
        .collect()
}

fn packed_cubic_vec4_keyframes(
    sub_track: &xc3_lib::bc::anim::SubTrack,
    keyframe_times: &[u16],
    coeffs: &[[f32; 4]],
) -> BTreeMap<OrderedFloat<f32>, Keyframe> {
    (sub_track.keyframe_start_index..sub_track.keyframe_end_index)
        .enumerate()
        .map(|(i, keyframe_index)| {
            let start_index = sub_track.curves_start_index as usize + i * 4;
            (
                (keyframe_times[keyframe_index as usize] as f32).into(),
                Keyframe {
                    x_coeffs: coeffs[start_index].into(),
                    y_coeffs: coeffs[start_index + 1].into(),
                    z_coeffs: coeffs[start_index + 2].into(),
                    w_coeffs: coeffs[start_index + 3].into(),
                },
            )
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
fn sample_keyframe_cubic(keyframes: &BTreeMap<OrderedFloat<f32>, Keyframe>, frame: f32) -> Vec4 {
    let (keyframe, x) = keyframe_position(keyframes, frame);

    vec4(
        interpolate_cubic(keyframe.x_coeffs, x),
        interpolate_cubic(keyframe.y_coeffs, x),
        interpolate_cubic(keyframe.z_coeffs, x),
        interpolate_cubic(keyframe.w_coeffs, x),
    )
}

fn keyframe_position(
    keyframes: &BTreeMap<OrderedFloat<f32>, Keyframe>,
    frame: f32,
) -> (&Keyframe, f32) {
    // Find the keyframe range containing the desired frame.
    // Use a workaround for tree lower/upper bound not being stable.
    let key = OrderedFloat::<f32>::from(frame);
    let mut before = keyframes.range((Unbounded, Included(key)));
    let mut after = keyframes.range((Excluded(key), Unbounded));

    let (previous_frame, keyframe) = before.next_back().unwrap();
    let (next_frame, _) = after.next().unwrap_or((previous_frame, keyframe));

    // The final keyframe should persist for the rest of the animation.
    let position = frame.min(next_frame.0) - previous_frame.0;

    (keyframe, position)
}

fn interpolate_cubic(coeffs: Vec4, x: f32) -> f32 {
    coeffs.x * (x * x * x) + coeffs.y * (x * x) + coeffs.z * x + coeffs.w
}

fn apply_transform(target: Mat4, source: Mat4, blend_mode: BlendMode) -> Mat4 {
    // TODO: Is this the correct way to implement additive blending?
    match blend_mode {
        BlendMode::Blend => source,
        BlendMode::Add => target * source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(frames: [f32; 3]) -> BTreeMap<OrderedFloat<f32>, Keyframe> {
        frames
            .into_iter()
            .map(|frame| {
                (
                    frame.into(),
                    Keyframe {
                        x_coeffs: Vec4::splat(frame),
                        y_coeffs: Vec4::splat(frame),
                        z_coeffs: Vec4::splat(frame),
                        w_coeffs: Vec4::splat(frame),
                    },
                )
            })
            .collect()
    }

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
        let keyframes = keys([0.0, 5.0, 9.0]);
        assert_eq!(
            (&keyframes[&0.0.into()], 0.0),
            keyframe_position(&keyframes, 0.0)
        );
        assert_eq!(
            (&keyframes[&0.0.into()], 2.5),
            keyframe_position(&keyframes, 2.5)
        );
        assert_eq!(
            (&keyframes[&0.0.into()], 4.9),
            keyframe_position(&keyframes, 4.9)
        );
    }

    #[test]
    fn index_position_second_keyframe() {
        let keyframes = keys([0.0, 5.0, 9.0]);
        assert_eq!(
            (&keyframes[&5.0.into()], 0.0),
            keyframe_position(&keyframes, 5.0)
        );
        assert_eq!(
            (&keyframes[&5.0.into()], 2.0),
            keyframe_position(&keyframes, 7.0)
        );
        assert_eq!(
            (&keyframes[&5.0.into()], 3.5),
            keyframe_position(&keyframes, 8.5)
        );
    }

    #[test]
    fn index_position_last_keyframe() {
        // This should clamp to the final keyframe instead of extrapolating.
        let keyframes = keys([0.0, 5.0, 9.0]);
        assert_eq!(
            (&keyframes[&9.0.into()], 0.0),
            keyframe_position(&keyframes, 10.0)
        );
        assert_eq!(
            (&keyframes[&9.0.into()], 0.0),
            keyframe_position(&keyframes, 12.5)
        );
    }
}
