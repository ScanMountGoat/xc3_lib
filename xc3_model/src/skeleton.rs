use glam::{vec3, Mat4, Quat};

/// See [Skeleton](xc3_lib::bc::Skeleton) and [Skinning](xc3_lib::mxmd::Skinning).
// TODO: Assume bones appear after their parents?
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// The hierarchy of bones in the skeleton.
    pub bones: Vec<Bone>,
}

/// A single node in the skeleton heirarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The name used by some animations to identify this bone.
    pub name: String,
    /// The local transform of the bone relative to its parent.
    pub transform: Mat4,
    /// The index of the parent [Bone] in [bones](struct.Skeleton.html#structfield.bones)
    /// or `None` if this is a root bone.
    pub parent_index: Option<usize>,
}

impl Skeleton {
    // TODO: Test this?
    pub fn from_skel(skeleton: &xc3_lib::bc::Skeleton, skinning: &xc3_lib::mxmd::Skinning) -> Self {
        // Start with the chr skeleton since it has parenting information.
        // The chr bones also tend to appear after their parents.
        // This makes accumulating transforms efficient when animating.
        // TODO: enforce this ordering?
        let mut bones: Vec<_> = skeleton
            .names
            .elements
            .iter()
            .zip(skeleton.transforms.iter())
            .zip(skeleton.parent_indices.elements.iter())
            .map(|((name, transform), parent)| Bone {
                name: name.name.clone(),
                transform: bone_transform(transform),
                parent_index: if *parent < 0 {
                    None
                } else {
                    Some(*parent as usize)
                },
            })
            .collect();

        // Merge the mxmd skeleton in case there are any missing bones.
        for (bone, transform) in skinning
            .bones
            .iter()
            .zip(skinning.inverse_bind_transforms.iter())
        {
            if !bones.iter().any(|b| b.name == bone.name) {
                // TODO: Parent index?
                // TODO: What to use for the transform?
                bones.push(Bone {
                    name: bone.name.clone(),
                    transform: Mat4::from_cols_array_2d(transform).inverse(),
                    parent_index: None,
                });
            }
        }

        // Add parenting and transform information for additional bones.
        // TODO: Does the mxmd have parenting information for all bones?
        if let Some(as_bone_data) = skinning
            .as_bone_data
            .as_ref()
            .and_then(|d| d.as_bone_data.as_ref())
        {
            for as_bone in &as_bone_data.bones {
                update_bone(
                    &mut bones,
                    skinning,
                    as_bone.bone_index,
                    as_bone.parent_index,
                );
            }
        }

        if let Some(unk4) = skinning
            .unk_offset4
            .as_ref()
            .and_then(|u| u.unk_offset4.as_ref())
        {
            for unk_bone in &unk4.bones {
                update_bone(
                    &mut bones,
                    skinning,
                    unk_bone.bone_index,
                    unk_bone.parent_index,
                );
            }
        }

        Self { bones }
    }

    /// The global accumulated transform for each bone in world space.
    ///
    /// This is the result of recursively applying the bone's transform to its parent.
    /// For inverse bind matrices, simply invert the world transforms.
    pub fn world_transforms(&self) -> Vec<Mat4> {
        let mut final_transforms: Vec<_> = self.bones.iter().map(|b| b.transform).collect();

        // TODO: Don't assume bones appear after their parents.
        for i in 0..final_transforms.len() {
            if let Some(parent) = self.bones[i].parent_index {
                final_transforms[i] = final_transforms[parent] * self.bones[i].transform;
            }
        }

        final_transforms
    }
}

fn update_bone(
    bones: &mut [Bone],
    skinning: &xc3_lib::mxmd::Skinning,
    bone_index: u16,
    parent_index: u16,
) {
    // TODO: Don't assume these bones are all parented?
    let bone_name = &skinning.bones[bone_index as usize].name;
    let parent_name = &skinning.bones[parent_index as usize].name;
    let parent_index = bones.iter().position(|b| &b.name == parent_name);

    if let Some(bone) = bones.iter_mut().find(|b| &b.name == bone_name) {
        let bone_world =
            Mat4::from_cols_array_2d(&skinning.inverse_bind_transforms[bone_index as usize])
                .inverse();
        // TODO: Is this the right transform?
        bone.transform = bone_world;
        bone.parent_index = parent_index;
    }
}

// TODO: Test the order of transforms.
fn bone_transform(b: &xc3_lib::bc::Transform) -> Mat4 {
    Mat4::from_translation(vec3(b.translation[0], b.translation[1], b.translation[2]))
        * Mat4::from_quat(Quat::from_array(b.rotation_quaternion))
        * Mat4::from_scale(vec3(b.scale[0], b.scale[1], b.scale[2]))
}

#[cfg(test)]
mod tests {
    // TODO: Test global/world transforms and inverse bind transforms
    #[test]
    fn test() {}
}
