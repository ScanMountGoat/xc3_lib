//! Utilities for working with animation data.
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::*;

use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles, vec3, vec4};
use log::error;
use ordered_float::OrderedFloat;
pub use xc3_lib::bc::anim::{BlendMode, PlayMode, SpaceMode};
pub use xc3_lib::hash::murmur3;

use crate::{Skeleton, Transform};

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
    // TODO: make this a vec instead?
    pub morph_tracks: Option<MorphTracks>,
    /// Translation at each frame for the skeleton root.
    pub root_translation: Option<Vec<Vec3>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    // TODO: Are fractional keyframes used in practice?
    pub translation_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub rotation_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub scale_keyframes: BTreeMap<OrderedFloat<f32>, Keyframe>,
    pub bone_index: BoneIndex,
}

/// Index for selecting the appropriate bone in a [Skeleton].
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

// TODO: Store this as a track for each index?
#[derive(Debug, PartialEq, Clone)]
pub struct MorphTracks {
    pub track_indices: Vec<i16>,
    pub track_values: Vec<f32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FCurves {
    // TODO: also store keyframes?
    // TODO: methods to return values per channel to work efficiently in Blender?
    pub translation: BTreeMap<String, Vec<Vec3>>,
    pub rotation: BTreeMap<String, Vec<Quat>>,
    pub scale: BTreeMap<String, Vec<Vec3>>,
}

impl Animation {
    // TODO: Error type instead of ignoring invalid data?
    // TODO: better logging for invalid data
    pub fn from_anim(anim: &xc3_lib::bc::anim::Anim) -> Self {
        Self {
            name: anim.binding.animation.name.clone(),
            space_mode: anim.binding.animation.space_mode,
            play_mode: anim.binding.animation.play_mode,
            blend_mode: anim.binding.animation.blend_mode,
            frames_per_second: anim.binding.animation.frames_per_second,
            frame_count: anim.binding.animation.frame_count,
            tracks: anim_tracks(anim),
            morph_tracks: morph_tracks(anim),
            root_translation: root_translation(anim),
        }
    }

    /// Convert `current_time_seconds` to frames based on the animation parameters.
    pub fn current_frame(&self, current_time_seconds: f32) -> f32 {
        // TODO: add option to force looping?
        let frame = current_time_seconds * self.frames_per_second;
        let final_frame = self.frame_count.saturating_sub(1) as f32;
        match self.play_mode {
            PlayMode::Loop => frame.rem_euclid(final_frame),
            PlayMode::Single => frame,
        }
    }

    // TODO: Tests for this.
    /// Compute the matrix for each bone in `skeleton`
    /// that transforms a vertex in model space to its animated position in model space.
    ///
    /// This also applies any extra translations to the root bone if present.
    ///
    /// This can be used in a vertex shader to apply linear blend skinning
    /// by transforming the vertex by up to 4 skinning matrices
    /// and blending with vertex skin weights.
    ///
    /// The in game skinning code precomputes a slightly different matrix.
    /// See [Weights](crate::skinning::Weights) for details.
    pub fn skinning_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Mat4> {
        let anim_transforms = self.model_space_transforms(skeleton, frame);
        let bind_transforms = skeleton.model_space_transforms();

        let mut animated_transforms = vec![Mat4::IDENTITY; skeleton.bones.len()];
        for i in (0..skeleton.bones.len()).take(animated_transforms.len()) {
            let inverse_bind = bind_transforms[i].to_matrix().inverse();
            animated_transforms[i] = anim_transforms[i].to_matrix() * inverse_bind;
        }

        animated_transforms
    }

    /// Compute the the animated transform in model space for each bone in `skeleton`.
    ///
    /// This also applies any extra translations to the root bone if present.
    ///
    /// See [Skeleton::model_space_transforms] for the transforms without animations applied.
    pub fn model_space_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Transform> {
        let mut animated_transforms = self.animated_transforms(skeleton, frame);
        self.apply_root_motion(&mut animated_transforms, frame);

        let mut anim_model_space = skeleton.model_space_transforms();
        self.apply_animation_transforms(&mut anim_model_space, skeleton, &animated_transforms);

        anim_model_space
    }

    fn apply_animation_transforms(
        &self,
        anim_model_space: &mut [Transform],
        skeleton: &Skeleton,
        animated_transforms: &[Option<Transform>],
    ) {
        // Assume parents appear before their children.
        for i in 0..anim_model_space.len() {
            match animated_transforms[i] {
                Some(transform) => {
                    anim_model_space[i] = match self.space_mode {
                        SpaceMode::Local => {
                            // Local space is relative to the parent bone.
                            if let Some(parent) = skeleton.bones[i].parent_index {
                                anim_model_space[parent] * transform
                            } else {
                                transform
                            }
                        }
                        SpaceMode::Model => {
                            // Model space is relative to the model root.
                            // This is faster to compute but rarely used by animation files.
                            transform
                        }
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

    fn animated_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Option<Transform>> {
        // TODO: Is it worth precomputing this?
        let hash_to_index: BTreeMap<_, _> = skeleton
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (murmur3(b.name.as_bytes()), i))
            .collect();

        // Keep track of which bones have animations applied.
        let mut animated_transforms = vec![None; skeleton.bones.len()];

        for track in &self.tracks {
            if let Some(bone_index) = track_bone_index(track, skeleton, &hash_to_index) {
                if let Some(transform) = track.sample_transform(frame, self.frame_count) {
                    if bone_index < skeleton.bones.len() {
                        animated_transforms[bone_index] = Some(apply_transform(
                            skeleton.bones[bone_index].transform,
                            transform,
                            self.blend_mode,
                        ));
                    } else {
                        // TODO: Why does this happen?
                        error!(
                            "Bone index {bone_index} out of range for length {}",
                            skeleton.bones.len()
                        );
                    }
                }
            } else {
                // TODO: Why does this happen?
                error!("No matching bone for {:?}", track.bone_index);
            }
        }
        animated_transforms
    }

    fn apply_root_motion(&self, animated_transforms: &mut [Option<Transform>], frame: f32) {
        if let Some(translations) = &self.root_translation {
            let (current, next, factor) = frame_next_frame_factor(frame, self.frame_count);
            let current_translation = translations.get(current).copied().unwrap_or(Vec3::ZERO);
            let next_translation = translations.get(next).copied().unwrap_or(Vec3::ZERO);

            let translation = current_translation.lerp(next_translation, factor);

            if let Some(root) = animated_transforms.first_mut() {
                match root {
                    Some(transform) => {
                        transform.translation += translation;
                    }
                    None => {
                        *root = Some(Transform {
                            translation,
                            ..Transform::IDENTITY
                        })
                    }
                }
            }
        }
    }

    /// Identical to [Self::model_space_transforms] but each transform is relative to the parent bone's transform.
    pub fn local_space_transforms(&self, skeleton: &Skeleton, frame: f32) -> Vec<Mat4> {
        let transforms = self.model_space_transforms(skeleton, frame);
        transforms
            .iter()
            .zip(skeleton.bones.iter())
            .map(|(transform, bone)| match bone.parent_index {
                Some(p) => transforms[p].to_matrix().inverse() * transform.to_matrix(),
                None => transform.to_matrix(),
            })
            .collect()
    }

    // TODO: Can these parameters be simplified or use a different type?
    /// Compute the the animated morph weights for each controller in `morph_controller_names`.
    pub fn morph_weights(
        &self,
        morph_controller_names: &[String],
        animation_morph_names: &[String],
        morph_target_controller_indices: &[usize],
        frame: f32,
    ) -> Vec<f32> {
        let (frame_index, next_frame_index, factor) =
            frame_next_frame_factor(frame, self.frame_count);

        // Default to the basis values if no morph animation is present.
        let mut weights = vec![0.0f32; morph_controller_names.len()];

        if let Some(morphs) = &self.morph_tracks {
            for (i, track_index) in morphs.track_indices.iter().enumerate() {
                // TODO: The counts and indices match up but don't select the right names?
                let name = &animation_morph_names[i];

                // TODO: This part isn't correct?
                if let Some(target_index) = morph_target_controller_indices
                    .iter()
                    .position(|t| morph_controller_names[*t] == *name)
                {
                    // TODO: Why is this sometimes out of range?
                    // TODO: log errors?
                    let len = weights.len();
                    if let Some(weight) = weights.get_mut(target_index % len) {
                        if let Ok(track_index) = usize::try_from(*track_index) {
                            // TODO: Is this how to handle multiple frames?
                            let frame_value = morphs.track_values.get(track_index * frame_index);

                            let next_frame_value =
                                morphs.track_values.get(track_index * next_frame_index);
                            if let Some(value) = frame_value {
                                *weight = match next_frame_value {
                                    Some(next_value) => {
                                        *value * (1.0 - factor) + *next_value * factor
                                    }
                                    None => *value,
                                }
                            }
                        }
                    }
                }
            }
        }
        weights
    }

    /// Calculate animation values relative to the bone's parent and "rest pose" or "bind pose".
    ///
    /// If `use_blender_coordinates` is `true`, the resulting values will match Blender's conventions.
    /// Bones will point along the y-axis instead of the x-axis and with z-axis for up instead of the y-axis.
    pub fn fcurves(&self, skeleton: &Skeleton, use_blender_coordinates: bool) -> FCurves {
        let bind_transforms: Vec<_> = skeleton
            .model_space_transforms()
            .into_iter()
            .map(|t| {
                if use_blender_coordinates {
                    xenoblade_to_blender(t.to_matrix())
                } else {
                    t.to_matrix()
                }
            })
            .collect();

        let animated_bone_names = animated_bone_names(self, skeleton);

        let mut translation_points = BTreeMap::new();
        let mut rotation_points = BTreeMap::new();
        let mut scale_points = BTreeMap::new();

        for frame in 0..self.frame_count {
            let transforms = self.local_space_transforms(skeleton, frame as f32);

            let mut animated_transforms = bind_transforms.clone();

            for i in 0..animated_transforms.len() {
                let bone = &skeleton.bones[i];
                if animated_bone_names.contains(bone.name.as_str()) {
                    let matrix = transforms[i];
                    if let Some(parent_index) = bone.parent_index {
                        let transform = if use_blender_coordinates {
                            blender_transform(matrix)
                        } else {
                            matrix
                        };
                        animated_transforms[i] = animated_transforms[parent_index] * transform;
                    } else {
                        animated_transforms[i] = if use_blender_coordinates {
                            xenoblade_to_blender(matrix)
                        } else {
                            matrix
                        };
                    }

                    // Find the transform relative to the parent and "rest pose" or "bind pose".
                    // This matches the UI values used in Blender for posing bones.
                    // TODO: Add tests for calculating this.
                    let basis_transform = if let Some(parent_index) = bone.parent_index {
                        let rest_local =
                            bind_transforms[parent_index].inverse() * bind_transforms[i];
                        let local =
                            animated_transforms[parent_index].inverse() * animated_transforms[i];
                        rest_local.inverse() * local
                    } else {
                        // Equivalent to above with parent transform set to identity.
                        bind_transforms[i].inverse() * animated_transforms[i]
                    };

                    let (s, r, t) = basis_transform.to_scale_rotation_translation();
                    insert_fcurve_point(&mut translation_points, &bone.name, t);
                    insert_fcurve_point(&mut rotation_points, &bone.name, r);
                    insert_fcurve_point(&mut scale_points, &bone.name, s);
                }
            }
        }

        FCurves {
            translation: translation_points,
            rotation: rotation_points,
            scale: scale_points,
        }
    }
}

fn track_bone_index(
    track: &Track,
    skeleton: &Skeleton,
    hash_to_index: &BTreeMap<u32, usize>,
) -> Option<usize> {
    match &track.bone_index {
        BoneIndex::Index(i) => Some(*i),
        BoneIndex::Hash(hash) => hash_to_index.get(hash).copied(),
        BoneIndex::Name(name) => skeleton.bones.iter().position(|b| &b.name == name),
    }
}

fn xenoblade_to_blender(m: Mat4) -> Mat4 {
    // Hard code these matrices for better precision.
    // rotate x -90 degrees
    let y_up_to_z_up = Mat4::from_cols_array_2d(&[
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    // rotate z -90 degrees.
    let x_major_to_y_major = Mat4::from_cols_array_2d(&[
        [0.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    y_up_to_z_up * m * x_major_to_y_major
}

fn insert_fcurve_point<T: Copy>(points: &mut BTreeMap<String, Vec<T>>, name: &str, t: T) {
    points
        .entry(name.to_string())
        .and_modify(|f| {
            f.push(t);
        })
        .or_insert(vec![t]);
}

fn frame_next_frame_factor(frame: f32, frame_count: u32) -> (usize, usize, f32) {
    let frame_index = frame as usize;
    let factor = frame.fract();
    let next_frame_index = frame.ceil() as usize;
    let final_frame_index = frame_count.saturating_sub(1) as usize;
    (
        frame_index.min(final_frame_index),
        next_frame_index.min(final_frame_index),
        factor,
    )
}

fn anim_tracks(anim: &xc3_lib::bc::anim::Anim) -> Vec<Track> {
    // Tracks are assigned to bones using indices, names, or name hashes.
    // Tracks have optional data depending on the anim type and game version.
    // This makes the conversion to a unified format somewhat complex.
    let (bone_names, hashes) = names_hashes(anim);

    match &anim.binding.animation.data {
        xc3_lib::bc::anim::AnimationData::Uncompressed(uncompressed) => {
            // The transforms do not contain values for unused tracks.
            let track_count = anim
                .binding
                .bone_track_indices
                .elements
                .iter()
                .filter(|i| **i != -1)
                .count();

            let transforms = &uncompressed.transforms;

            anim.binding
                .bone_track_indices
                .elements
                .iter()
                .filter(|i| **i != -1)
                .map(|i| {
                    let mut translation_keyframes = BTreeMap::new();
                    let mut rotation_keyframes = BTreeMap::new();
                    let mut scale_keyframes = BTreeMap::new();

                    for frame in 0..anim.binding.animation.frame_count {
                        let index = frame as usize * track_count + *i as usize;
                        let next_index = (frame as usize + 1) * track_count + *i as usize;

                        if let Some(transform) = transforms.get(index) {
                            // Convert to cubic instead of having separate interpolation types.
                            let translation = transform.translation;
                            let next_translation =
                                transforms.get(next_index).map(|t| t.translation);
                            translation_keyframes.insert(
                                (frame as f32).into(),
                                linear_to_cubic_keyframe(translation, next_translation),
                            );

                            let rotation = transform.rotation_quaternion;
                            let next_rotation =
                                transforms.get(next_index).map(|t| t.rotation_quaternion);
                            rotation_keyframes.insert(
                                (frame as f32).into(),
                                linear_to_cubic_keyframe(rotation, next_rotation),
                            );

                            let scale = transform.scale;
                            let next_scale = transforms.get(next_index).map(|t| t.scale);
                            scale_keyframes.insert(
                                (frame as f32).into(),
                                linear_to_cubic_keyframe(scale, next_scale),
                            );
                        } else {
                            error!(
                                "Uncompressed transform index {index} out of range for length {}",
                                transforms.len()
                            );
                        }
                    }

                    let bone_index = bone_index(*i as usize, bone_names, hashes);

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
                    usize::try_from(*index).ok().and_then(|index| {
                        // TODO: How to handle invalid indices?
                        if let Some(track) = cubic.tracks.elements.get(index) {
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

                            let bone_index = bone_index(i, bone_names, hashes);

                            Some(Track {
                                translation_keyframes,
                                rotation_keyframes,
                                scale_keyframes,
                                bone_index,
                            })
                        } else {
                            error!(
                                "Cubic track index {index} out of range for length {}",
                                cubic.tracks.elements.len()
                            );
                            None
                        }
                    })
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

                    let bone_index = bone_index(*i as usize, bone_names, hashes);

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

fn morph_tracks(anim: &xc3_lib::bc::anim::Anim) -> Option<MorphTracks> {
    match &anim.binding.inner {
        xc3_lib::bc::anim::AnimationBindingInner::Unk1(unk1) => {
            // TODO: Does this ever have more than 1 element?
            let extra = unk1.extra_track_bindings.first()?;

            Some(MorphTracks {
                track_indices: match &extra.track_indices {
                    xc3_lib::bc::BcListCount::List(list) => list.clone(),
                    xc3_lib::bc::BcListCount::NullOffsetCount(_) => Vec::new(),
                },
                track_values: extra
                    .extra_track_animation
                    .as_ref()
                    .map(|extra| match &extra.data {
                        xc3_lib::bc::anim::ExtraAnimationData::Uncompressed(values) => {
                            values.elements.clone()
                        }
                        xc3_lib::bc::anim::ExtraAnimationData::Cubic(_cubic) => Vec::new(),
                    })
                    .unwrap_or_default(),
            })
        }
        // TODO: Does these also contain morph animations?
        xc3_lib::bc::anim::AnimationBindingInner::Unk2(_) => None,
        xc3_lib::bc::anim::AnimationBindingInner::Unk3(_) => None,
        xc3_lib::bc::anim::AnimationBindingInner::Unk4(_) => None,
        xc3_lib::bc::anim::AnimationBindingInner::Unk5(_) => None,
    }
}

fn names_hashes(
    anim: &xc3_lib::bc::anim::Anim,
) -> (Option<&Vec<xc3_lib::bc::StringOffset>>, Option<&Vec<u32>>) {
    match &anim.binding.inner {
        xc3_lib::bc::anim::AnimationBindingInner::Unk1(_) => (None, None),
        xc3_lib::bc::anim::AnimationBindingInner::Unk2(inner) => {
            (Some(&inner.bone_names.elements), None)
        }
        xc3_lib::bc::anim::AnimationBindingInner::Unk3(inner) => (
            Some(&inner.bone_names.elements),
            extra_track_hashes(&inner.extra_track_data),
        ),
        xc3_lib::bc::anim::AnimationBindingInner::Unk4(inner) => (
            Some(&inner.bone_names.elements),
            extra_track_hashes(&inner.extra_track_data),
        ),
        xc3_lib::bc::anim::AnimationBindingInner::Unk5(inner) => (
            Some(&inner.bone_names.elements),
            extra_track_hashes(&inner.extra_track_data),
        ),
    }
}

fn root_translation(anim: &xc3_lib::bc::anim::Anim) -> Option<Vec<Vec3>> {
    anim.binding.animation.locomotion.as_ref().map(|l| {
        l.translation
            .iter()
            .map(|v| vec3(v[0], v[1], v[2]))
            .collect()
    })
}

fn bone_index(
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
    /// Sample the translation at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_translation(&self, frame: f32, frame_count: u32) -> Option<Vec3> {
        sample_keyframe_cubic(&self.translation_keyframes, frame, frame_count).map(|t| t.xyz())
    }

    /// Sample the rotation at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_rotation(&self, frame: f32, frame_count: u32) -> Option<Quat> {
        let rotation = sample_keyframe_cubic(&self.rotation_keyframes, frame, frame_count)?;
        Some(Quat::from_array(rotation.to_array()))
    }

    /// Sample the scale at `frame` using the appropriate interpolation between frames.
    /// Returns `None` if the animation is empty.
    pub fn sample_scale(&self, frame: f32, frame_count: u32) -> Option<Vec3> {
        sample_keyframe_cubic(&self.scale_keyframes, frame, frame_count).map(|s| s.xyz())
    }

    /// Sample and combine transformation matrices for scale -> rotation -> translation (TRS).
    /// Returns `None` if the animation is empty.
    pub fn sample_transform(&self, frame: f32, frame_count: u32) -> Option<Transform> {
        let translation = self.sample_translation(frame, frame_count)?;
        let rotation = self.sample_rotation(frame, frame_count)?;
        let scale = self.sample_scale(frame, frame_count)?;

        Some(Transform {
            translation,
            rotation,
            scale,
        })
    }
}

// TODO: Add tests for this.
fn sample_keyframe_cubic(
    keyframes: &BTreeMap<OrderedFloat<f32>, Keyframe>,
    frame: f32,
    frame_count: u32,
) -> Option<Vec4> {
    let (keyframe, x) = keyframe_position(keyframes, frame, frame_count)?;

    Some(vec4(
        interpolate_cubic(keyframe.x_coeffs, x),
        interpolate_cubic(keyframe.y_coeffs, x),
        interpolate_cubic(keyframe.z_coeffs, x),
        interpolate_cubic(keyframe.w_coeffs, x),
    ))
}

fn keyframe_position(
    keyframes: &BTreeMap<OrderedFloat<f32>, Keyframe>,
    frame: f32,
    frame_count: u32,
) -> Option<(&Keyframe, f32)> {
    // Find the keyframe range containing the desired frame.
    // Use a workaround for tree lower/upper bound not being stable.
    let key = OrderedFloat::<f32>::from(frame);
    let mut before = keyframes.range((Unbounded, Included(key)));
    let mut after = keyframes.range((Excluded(key), Unbounded));

    let (previous_frame, keyframe) = before.next_back()?;
    // The final keyframe should persist for the rest of the animation.
    let next_frame = after
        .next()
        .map(|(f, _)| f.0)
        .unwrap_or(frame_count.saturating_sub(1) as f32);

    let position = frame.min(next_frame) - previous_frame.0;

    Some((keyframe, position))
}

fn interpolate_cubic(coeffs: Vec4, x: f32) -> f32 {
    coeffs.x * (x * x * x) + coeffs.y * (x * x) + coeffs.z * x + coeffs.w
}

fn apply_transform(target: Transform, source: Transform, blend_mode: BlendMode) -> Transform {
    // TODO: Is this the correct way to implement additive blending?
    match blend_mode {
        BlendMode::Blend => source,
        BlendMode::Add => target * source,
    }
}

fn animated_bone_names<'a>(animation: &'a Animation, skeleton: &'a Skeleton) -> BTreeSet<&'a str> {
    let hash_to_name: BTreeMap<u32, &str> = skeleton
        .bones
        .iter()
        .map(|b| (murmur3(b.name.as_bytes()), b.name.as_str()))
        .collect();

    let mut names: BTreeSet<_> = animation
        .tracks
        .iter()
        .filter_map(|t| match &t.bone_index {
            BoneIndex::Index(i) => skeleton.bones.get(*i).map(|b| b.name.as_str()),
            BoneIndex::Hash(hash) => hash_to_name.get(hash).copied(),
            BoneIndex::Name(n) => Some(n.as_str()),
        })
        .collect();

    // Include the root motion even if there is no track for the root bone.
    if animation.root_translation.is_some() {
        if let Some(root_bone) = skeleton.bones.first() {
            names.insert(&root_bone.name);
        }
    }

    names
}

fn blender_transform(m: Mat4) -> Mat4 {
    // In game, the bone's x-axis points from parent to child.
    // In Blender, the bone's y-axis points from parent to child.
    // https://en.wikipedia.org/wiki/Matrix_similarity
    // Perform the transformation m in Xenoblade's basis and convert back to Blender.
    let p = Mat4::from_cols_array_2d(&[
        [0.0, -1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ])
    .transpose();
    p * m * p.inverse()
}

#[cfg(test)]
mod tests {
    use glam::quat;

    use crate::Bone;

    use super::*;

    macro_rules! assert_matrix_relative_eq {
        ($a:expr, $b:expr) => {
            assert!(
                $a.to_cols_array()
                    .iter()
                    .zip($b.to_cols_array().iter())
                    .all(|(a, b)| approx::relative_eq!(a, b, epsilon = 0.0001f32)),
                "Matrices not equal to within 0.0001.\nleft = {:?}\nright = {:?}",
                $a,
                $b
            )
        };
    }

    fn keys(frames: &[f32]) -> BTreeMap<OrderedFloat<f32>, Keyframe> {
        frames
            .iter()
            .map(|frame| {
                (
                    (*frame).into(),
                    Keyframe {
                        x_coeffs: Vec4::splat(*frame),
                        y_coeffs: Vec4::splat(*frame),
                        z_coeffs: Vec4::splat(*frame),
                        w_coeffs: Vec4::splat(*frame),
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
    fn index_position_no_keyframes() {
        let keyframes = keys(&[]);
        assert_eq!(None, keyframe_position(&keyframes, 0.0, 0));
        assert_eq!(None, keyframe_position(&keyframes, 2.5, 0));
        assert_eq!(None, keyframe_position(&keyframes, 4.9, 0));
    }

    #[test]
    fn index_position_first_keyframe() {
        let keyframes = keys(&[0.0, 5.0, 9.0]);
        assert_eq!(
            Some((&keyframes[&0.0.into()], 0.0)),
            keyframe_position(&keyframes, 0.0, 11)
        );
        assert_eq!(
            Some((&keyframes[&0.0.into()], 2.5)),
            keyframe_position(&keyframes, 2.5, 11)
        );
        assert_eq!(
            Some((&keyframes[&0.0.into()], 4.9)),
            keyframe_position(&keyframes, 4.9, 11)
        );
    }

    #[test]
    fn index_position_second_keyframe() {
        let keyframes = keys(&[0.0, 5.0, 9.0]);
        assert_eq!(
            Some((&keyframes[&5.0.into()], 0.0)),
            keyframe_position(&keyframes, 5.0, 11)
        );
        assert_eq!(
            Some((&keyframes[&5.0.into()], 2.0)),
            keyframe_position(&keyframes, 7.0, 11)
        );
        assert_eq!(
            Some((&keyframes[&5.0.into()], 3.5)),
            keyframe_position(&keyframes, 8.5, 11)
        );
    }

    #[test]
    fn index_position_last_keyframe() {
        // This should extrapolate.
        // The final keyframe may not be at the final animation frame.
        let keyframes = keys(&[0.0, 5.0, 9.0]);
        assert_eq!(
            Some((&keyframes[&9.0.into()], 0.0)),
            keyframe_position(&keyframes, 9.0, 11)
        );
        assert_eq!(
            Some((&keyframes[&9.0.into()], 1.0)),
            keyframe_position(&keyframes, 10.0, 11)
        );
        assert_eq!(
            Some((&keyframes[&9.0.into()], 1.0)),
            keyframe_position(&keyframes, 12.5, 11)
        );
    }

    #[test]
    fn model_space_transforms_empty() {
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Local,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: Vec::new(),
            morph_tracks: None,
            root_translation: None,
        };

        assert!(
            animation
                .model_space_transforms(&Skeleton { bones: Vec::new() }, 0.0)
                .is_empty()
        );
    }

    #[test]
    fn model_space_transforms_root_motion_empty() {
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Local,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: Vec::new(),
            morph_tracks: None,
            root_translation: Some(vec![Vec3::ONE]),
        };

        assert!(
            animation
                .model_space_transforms(&Skeleton { bones: Vec::new() }, 0.0)
                .is_empty()
        );
    }

    fn keyframe(x: f32, y: f32, z: f32, w: f32) -> (OrderedFloat<f32>, Keyframe) {
        // Crate a keyframe with a constant value.
        (
            0.0.into(),
            Keyframe {
                x_coeffs: vec4(0.0, 0.0, 0.0, x),
                y_coeffs: vec4(0.0, 0.0, 0.0, y),
                z_coeffs: vec4(0.0, 0.0, 0.0, z),
                w_coeffs: vec4(0.0, 0.0, 0.0, w),
            },
        )
    }

    // TODO: test additive blending.
    #[test]
    fn model_space_transforms_local_blend() {
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Local,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: None,
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let transforms = animation.model_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[0].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [2.0, 4.0, 6.0, 1.0],
            ]),
            transforms[1].to_matrix()
        );
    }

    #[test]
    fn model_space_transforms_local_blend_constrain_scale() {
        // Test that the original bone distance is preserved even when scaling.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Local,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.5, 1.5, 1.5, 0.0)].into(),
                    bone_index: BoneIndex::Name("a_L".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.5, 1.5, 1.5, 0.0)].into(),
                    bone_index: BoneIndex::Name("b_L".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(0.75, 0.75, 0.75, 0.0)].into(),
                    bone_index: BoneIndex::Name("a_R".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(0.5, 0.5, 0.5, 0.0)].into(),
                    bone_index: BoneIndex::Name("b_R".to_string()),
                },
            ],
            morph_tracks: None,
            root_translation: None,
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "root".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "a_L".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
                Bone {
                    name: "b_L".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(1),
                },
                Bone {
                    name: "a_R".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
                Bone {
                    name: "b_R".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(3),
                },
            ],
        };

        // Scaling bones preserves the relative distance with the parent.
        // This was tested visually in Xenoblade 2 with bl000101.wimdo.
        // The behavior seems to be the same for Xenoblade 1 DE and Xenoblade 3.
        let transforms = animation.model_space_transforms(&skeleton, 0.0);
        assert_eq!(5, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]),
            transforms[0].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.5, 0.0, 0.0, 0.0],
                [0.0, 1.5, 0.0, 0.0],
                [0.0, 0.0, 1.5, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[1].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [2.25, 0.0, 0.0, 0.0],
                [0.0, 2.25, 0.0, 0.0],
                [0.0, 0.0, 2.25, 0.0],
                [2.0, 4.0, 6.0, 1.0],
            ]),
            transforms[2].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [0.75, 0.0, 0.0, 0.0],
                [0.0, 0.75, 0.0, 0.0],
                [0.0, 0.0, 0.75, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[3].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [0.375, 0.0, 0.0, 0.0],
                [0.0, 0.375, 0.0, 0.0],
                [0.0, 0.0, 0.375, 0.0],
                [2.0, 4.0, 6.0, 1.0],
            ]),
            transforms[4].to_matrix()
        );
    }

    #[test]
    fn model_space_transforms_model_blend() {
        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: None,
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let transforms = animation.model_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[0].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [10.0, 20.0, 30.0, 1.0],
            ]),
            transforms[1].to_matrix()
        );
    }

    #[test]
    fn model_space_transforms_model_root_motion_blend() {
        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: Some(vec![vec3(0.25, 0.5, 0.75)]),
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let transforms = animation.model_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.25, 2.5, 3.75, 1.0],
            ]),
            transforms[0].to_matrix()
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [10.0, 20.0, 30.0, 1.0],
            ]),
            transforms[1].to_matrix()
        );
    }

    #[test]
    fn local_space_transforms_model_blend() {
        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 0.0, 0.0, 1.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: None,
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let transforms = animation.local_space_transforms(&skeleton, 0.0);
        assert_eq!(2, transforms.len());
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            transforms[0]
        );
        assert_matrix_relative_eq!(
            Mat4::from_cols_array_2d(&[
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [9.0, 18.0, 27.0, 1.0],
            ]),
            transforms[1]
        );
    }

    #[test]
    fn fcurves_xenoblade() {
        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(1.0, 0.0, 0.0, 0.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 1.0, 0.0, 0.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: Some(vec![vec3(0.25, 0.5, 0.75)]),
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let fcurves = animation.fcurves(&skeleton, false);
        assert_eq!(
            FCurves {
                translation: [
                    ("a".to_string(), vec![vec3(1.25, 2.5, 3.75)]),
                    ("b".to_string(), vec![vec3(8.75, -17.5, -26.25)])
                ]
                .into(),
                rotation: [
                    ("a".to_string(), vec![quat(1.0, 0.0, 0.0, 0.0)]),
                    ("b".to_string(), vec![quat(0.0, 0.0, 1.0, 0.0)])
                ]
                .into(),
                scale: [
                    ("a".to_string(), vec![vec3(1.0, 1.0, 1.0)]),
                    ("b".to_string(), vec![vec3(1.0, 1.0, 1.0)])
                ]
                .into()
            },
            fcurves
        );
    }

    #[test]
    fn fcurves_xenoblade_no_root_track() {
        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![Track {
                translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                rotation_keyframes: [keyframe(0.0, 1.0, 0.0, 0.0)].into(),
                scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                bone_index: BoneIndex::Index(1),
            }],
            morph_tracks: None,
            root_translation: Some(vec![vec3(0.25, 0.5, 0.75)]),
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let fcurves = animation.fcurves(&skeleton, false);
        assert_eq!(
            FCurves {
                translation: [
                    ("a".to_string(), vec![vec3(0.25, 0.5, 0.75)]),
                    ("b".to_string(), vec![vec3(9.75, 19.5, 29.25)])
                ]
                .into(),
                rotation: [
                    ("a".to_string(), vec![quat(0.0, 0.0, 0.0, 1.0)]),
                    ("b".to_string(), vec![quat(0.0, 1.0, 0.0, 0.0)])
                ]
                .into(),
                scale: [
                    ("a".to_string(), vec![vec3(1.0, 1.0, 1.0)]),
                    ("b".to_string(), vec![vec3(1.0, 1.0, 1.0)])
                ]
                .into()
            },
            fcurves
        );
    }

    #[test]
    fn fcurves_blender() {
        // Crate a keyframe with a constant value.
        let keyframe = |x, y, z, w| {
            (
                0.0.into(),
                Keyframe {
                    x_coeffs: vec4(0.0, 0.0, 0.0, x),
                    y_coeffs: vec4(0.0, 0.0, 0.0, y),
                    z_coeffs: vec4(0.0, 0.0, 0.0, z),
                    w_coeffs: vec4(0.0, 0.0, 0.0, w),
                },
            )
        };

        // Model space animations update the model space transforms directly.
        let animation = Animation {
            name: String::new(),
            space_mode: SpaceMode::Model,
            play_mode: PlayMode::Single,
            blend_mode: BlendMode::Blend,
            frames_per_second: 30.0,
            frame_count: 1,
            tracks: vec![
                Track {
                    translation_keyframes: [keyframe(1.0, 2.0, 3.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(1.0, 0.0, 0.0, 0.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Name("a".to_string()),
                },
                Track {
                    translation_keyframes: [keyframe(10.0, 20.0, 30.0, 0.0)].into(),
                    rotation_keyframes: [keyframe(0.0, 1.0, 0.0, 0.0)].into(),
                    scale_keyframes: [keyframe(1.0, 1.0, 1.0, 0.0)].into(),
                    bone_index: BoneIndex::Index(1),
                },
            ],
            morph_tracks: None,
            root_translation: Some(vec![vec3(0.25, 0.5, 0.75)]),
        };

        let skeleton = Skeleton {
            bones: vec![
                Bone {
                    name: "a".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: None,
                },
                Bone {
                    name: "b".to_string(),
                    transform: Transform::IDENTITY,
                    parent_index: Some(0),
                },
            ],
        };

        let fcurves = animation.fcurves(&skeleton, true);
        assert_eq!(
            FCurves {
                translation: [
                    ("a".to_string(), vec![vec3(-2.5, 1.25, 3.75)]),
                    ("b".to_string(), vec![vec3(17.5, 8.75, -26.25)])
                ]
                .into(),
                rotation: [
                    ("a".to_string(), vec![quat(0.0, 1.0, 0.0, 0.0)]),
                    ("b".to_string(), vec![quat(0.0, 0.0, 1.0, 0.0)])
                ]
                .into(),
                scale: [
                    ("a".to_string(), vec![vec3(1.0, 1.0, 1.0)]),
                    ("b".to_string(), vec![vec3(1.0, 1.0, 1.0)])
                ]
                .into()
            },
            fcurves
        );
    }
}
