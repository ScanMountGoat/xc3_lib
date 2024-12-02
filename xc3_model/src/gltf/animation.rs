use super::{buffer::WriteBytes, CreateGltfError, GltfData};
use crate::{animation::Animation, Skeleton};
use gltf::json::validation::Checked::Valid;
use xc3_lib::hash::murmur3;

pub fn add_animations(
    data: &mut GltfData,
    animations: &[Animation],
    skeleton: &Skeleton,
    root_bone_node_index: u32,
) -> Result<(), CreateGltfError> {
    for animation in animations {
        let mut samplers = Vec::new();
        let mut channels = Vec::new();

        // Baked tracks can share keyframe times in seconds.
        let keyframe_times: Vec<_> = (0..animation.frame_count)
            .map(|i| i as f32 / animation.frames_per_second)
            .collect();
        let input = data.buffers.add_values(
            &keyframe_times,
            gltf::json::accessor::Type::Scalar,
            gltf::json::accessor::ComponentType::F32,
            None,
            (
                keyframe_times
                    .iter()
                    .copied()
                    .reduce(f32::min)
                    .map(|v| serde_json::json!([v])),
                keyframe_times
                    .iter()
                    .copied()
                    .reduce(f32::max)
                    .map(|v| serde_json::json!([v])),
            ),
            false,
        )?;

        // Calculate transforms to handle animation spaces and blending.
        // TODO: Is there a more efficient way to calculate this?
        let frame_bone_transforms: Vec<_> = (0..animation.frame_count)
            .map(|i| animation.local_space_transforms(skeleton, i as f32))
            .collect();

        // Assume each bone has at most one track.
        // TODO: how to handle missing bones?
        let animated_bone_indices: Vec<_> = animation
            .tracks
            .iter()
            .filter_map(|t| match &t.bone_index {
                crate::animation::BoneIndex::Index(i) => Some(*i),
                crate::animation::BoneIndex::Hash(hash) => skeleton
                    .bones
                    .iter()
                    .position(|b| murmur3(b.name.as_bytes()) == *hash),
                crate::animation::BoneIndex::Name(name) => {
                    skeleton.bones.iter().position(|b| &b.name == name)
                }
            })
            .collect();

        for bone in animated_bone_indices {
            let mut translations = Vec::new();
            let mut rotations = Vec::new();
            let mut scales = Vec::new();

            for bone_transforms in &frame_bone_transforms {
                let (s, r, t) = bone_transforms[bone].to_scale_rotation_translation();
                translations.push(t);
                rotations.push(r);
                scales.push(s);
            }

            // Assume bone nodes match the skeleton bone ordering.
            let node = gltf::json::Index::new(root_bone_node_index + bone as u32);

            add_channel(
                data,
                &mut samplers,
                &mut channels,
                &translations,
                input,
                node,
                gltf::json::animation::Property::Translation,
                gltf::json::accessor::Type::Vec3,
            )?;
            add_channel(
                data,
                &mut samplers,
                &mut channels,
                &rotations,
                input,
                node,
                gltf::json::animation::Property::Rotation,
                gltf::json::accessor::Type::Vec4,
            )?;
            add_channel(
                data,
                &mut samplers,
                &mut channels,
                &scales,
                input,
                node,
                gltf::json::animation::Property::Scale,
                gltf::json::accessor::Type::Vec3,
            )?;
        }

        data.animations.push(gltf::json::Animation {
            extensions: None,
            extras: None,
            channels,
            name: Some(animation.name.clone()),
            samplers,
        });
    }

    Ok(())
}

fn add_channel<T: WriteBytes>(
    data: &mut GltfData,
    samplers: &mut Vec<gltf_json::animation::Sampler>,
    channels: &mut Vec<gltf_json::animation::Channel>,
    values: &[T],
    input: gltf_json::Index<gltf_json::Accessor>,
    node: gltf_json::Index<gltf_json::Node>,
    property: gltf::json::animation::Property,
    component_type: gltf::json::accessor::Type,
) -> Result<(), CreateGltfError> {
    let output = data.buffers.add_values(
        values,
        component_type,
        gltf::json::accessor::ComponentType::F32,
        None,
        (None, None),
        false,
    )?;

    let sampler = gltf::json::animation::Sampler {
        extensions: None,
        extras: None,
        input,
        interpolation: Valid(gltf::json::animation::Interpolation::Linear),
        output,
    };
    let sampler_index = gltf::json::Index::new(samplers.len() as u32);
    samplers.push(sampler);

    let channel = gltf::json::animation::Channel {
        sampler: sampler_index,
        target: gltf::json::animation::Target {
            extensions: None,
            extras: None,
            node,
            path: Valid(property),
        },
        extensions: None,
        extras: None,
    };
    channels.push(channel);

    Ok(())
}
