use std::collections::HashMap;

use glam::Mat4;
use log::error;
use xc3_model::animation::{murmur3, Animation, BlendMode, BoneIndex};

pub fn animate_skeleton(
    skeleton: &xc3_model::Skeleton,
    animation: &Animation,
    current_time_seconds: f32,
) -> [Mat4; 256] {
    let frame = animation.current_frame(current_time_seconds);

    // TODO: Is it worth precomputing this?
    let hash_to_index: HashMap<_, _> = skeleton
        .bones
        .iter()
        .enumerate()
        .map(|(i, b)| (murmur3(b.name.as_bytes()), i))
        .collect();

    // Keep track of which bones have animations applied.
    let mut animated_transforms = vec![None; skeleton.bones.len()];

    for track in &animation.tracks {
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
                    animation.blend_mode,
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
            error!("No matching bone for index {:?}", track.bone_index);
        }
    }

    let rest_pose_world = skeleton.world_transforms();

    // Assume parents appear before their children.
    // TODO: Does this code correctly handle all cases?
    let mut anim_world = rest_pose_world.clone();
    match animation.space_mode {
        xc3_model::animation::SpaceMode::Local => {
            for i in 0..anim_world.len() {
                match animated_transforms[i] {
                    Some(transform) => {
                        // Local space is relative to the parent bone.
                        if let Some(parent) = skeleton.bones[i].parent_index {
                            anim_world[i] = anim_world[parent] * transform;
                        } else {
                            anim_world[i] = transform;
                        }
                    }
                    None => {
                        if let Some(parent) = skeleton.bones[i].parent_index {
                            anim_world[i] = anim_world[parent] * skeleton.bones[i].transform;
                        }
                    }
                }
            }
        }
        xc3_model::animation::SpaceMode::Model => {
            for i in 0..anim_world.len() {
                match animated_transforms[i] {
                    Some(transform) => {
                        // Model space is relative to the model root.
                        // TODO: Is it worth distinguishing between model and world space?
                        anim_world[i] = transform;
                    }
                    None => {
                        if let Some(parent) = skeleton.bones[i].parent_index {
                            anim_world[i] = anim_world[parent] * skeleton.bones[i].transform;
                        }
                    }
                }
            }
        }
    }

    let mut animated_transforms = [Mat4::IDENTITY; 256];
    for i in (0..skeleton.bones.len()).take(animated_transforms.len()) {
        let inverse_bind = rest_pose_world[i].inverse();
        animated_transforms[i] = anim_world[i] * inverse_bind;
    }

    animated_transforms
}

fn apply_transform(target: Mat4, source: Mat4, blend_mode: BlendMode) -> Mat4 {
    // TODO: Is this the correct way to implement additive blending?
    match blend_mode {
        BlendMode::Blend => source,
        BlendMode::Add => target * source,
    }
}
