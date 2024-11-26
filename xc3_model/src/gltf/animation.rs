use super::{buffer::WriteBytes, CreateGltfError, GltfData};
use crate::animation::{Animation, BoneIndex};
use gltf::json::validation::Checked::Valid;
use xc3_lib::hash::murmur3;

pub fn add_animations(
    data: &mut GltfData,
    animations: &[Animation],
    root_bone_node_index: u32,
) -> Result<(), CreateGltfError> {
    for animation in animations {
        let mut samplers = Vec::new();
        let mut channels = Vec::new();

        // Baked tracks can share keyframe times.
        let keyframe_times: Vec<_> = (0..animation.frame_count).map(|i| i as f32).collect();
        let input = data.buffers.add_values(
            &keyframe_times,
            gltf::json::accessor::Type::Scalar,
            gltf::json::accessor::ComponentType::F32,
            None,
            (
                Some(serde_json::json!([0.0])),
                Some(serde_json::json!([
                    animation.frame_count.saturating_sub(1) as f32
                ])),
            ),
            false,
        )?;

        for track in &animation.tracks {
            // TODO: These transforms aren't correct.
            // TODO: avoid unwrap
            let translations: Vec<_> = keyframe_times
                .iter()
                .map(|i| track.sample_translation(*i, animation.frame_count).unwrap())
                .collect();
            let rotations: Vec<_> = keyframe_times
                .iter()
                .map(|i| track.sample_rotation(*i, animation.frame_count).unwrap())
                .collect();
            let scales: Vec<_> = keyframe_times
                .iter()
                .map(|i| track.sample_scale(*i, animation.frame_count).unwrap())
                .collect();

            // TODO: Is there a more reliable way to find bone nodes?
            let bone_index = match &track.bone_index {
                BoneIndex::Index(i) => *i as u32,
                BoneIndex::Hash(hash) => data
                    .nodes
                    .iter()
                    .skip(root_bone_node_index as usize)
                    .position(|n| n.name.as_ref().map(|n| murmur3(n.as_bytes())) == Some(*hash))
                    .unwrap_or_default() as u32,
                BoneIndex::Name(name) => data
                    .nodes
                    .iter()
                    .skip(root_bone_node_index as usize)
                    .position(|n| n.name.as_ref() == Some(name))
                    .unwrap_or_default() as u32,
            };
            let node = gltf::json::Index::new(root_bone_node_index + bone_index);

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
