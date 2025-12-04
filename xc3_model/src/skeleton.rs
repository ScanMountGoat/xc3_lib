use glam::{Mat4, Quat, vec3};
use log::{error, warn};
use xc3_lib::hkt::Hkt;

use crate::Transform;

/// See [Skeleton](xc3_lib::bc::skel::Skeleton) and [Skinning](xc3_lib::mxmd::Skinning).
// TODO: Assume bones appear after their parents?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Skeleton {
    /// The hierarchy of bones in the skeleton.
    pub bones: Vec<Bone>,
}

/// A single node in the skeleton heirarchy.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct Bone {
    /// The name used by some animations to identify this bone.
    pub name: String,
    /// The local transform of the bone relative to its parent.
    pub transform: Transform,
    /// The index of the parent [Bone] in [bones](struct.Skeleton.html#structfield.bones)
    /// or `None` if this is a root bone.
    pub parent_index: Option<usize>,
}

impl Skeleton {
    // TODO: Test this?
    pub fn from_skeleton(
        skeleton: &xc3_lib::bc::skel::Skeleton,
        skinning: Option<&xc3_lib::mxmd::Skinning>,
    ) -> Self {
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
                parent_index: (*parent).try_into().ok(),
            })
            .collect();

        // Add additional MT_ bones.
        for ((name, transform), parent) in skeleton
            .mt_names
            .iter()
            .zip(skeleton.mt_transforms.iter())
            .zip(skeleton.mt_parent_indices.iter())
        {
            bones.push(Bone {
                name: name.name.clone(),
                transform: bone_transform(transform),
                parent_index: (*parent).try_into().ok(),
            });
        }

        // Add defaults for any missing bones.
        if let Some(skinning) = skinning {
            let root_bone_index = bones.iter().position(|b| b.name == skeleton.root_bone_name);
            for (i, bone) in skinning.bones.iter().enumerate() {
                if !bones.iter().any(|b| b.name == bone.name) {
                    let transform = skinning
                        .inverse_bind_transforms
                        .get(i)
                        .map(|transform| Mat4::from_cols_array_2d(transform).inverse())
                        .unwrap_or(Mat4::IDENTITY);

                    // Some bones have no explicitly defined parents.
                    bones.push(Bone {
                        name: bone.name.clone(),
                        transform: Transform::from_matrix(transform),
                        parent_index: root_bone_index,
                    });
                }
            }

            // Add parenting and transform information for additional bones.
            if let Some(as_bone_data) = skinning
                .as_bone_data
                .as_ref()
                .and_then(|d| d.as_bone_data.as_ref())
            {
                for as_bone in &as_bone_data.bones {
                    // TODO: Why is using the translation and rotation not always accurate?
                    // TODO: Is there a flag or value that affects the rotation?
                    let transform = infer_transform(
                        skinning,
                        as_bone.bone_index as usize,
                        as_bone.parent_index as usize,
                    );
                    update_bone(
                        &mut bones,
                        skinning,
                        as_bone.bone_index,
                        as_bone.parent_index,
                        transform,
                    );
                }
            }

            if let Some(unk4) = skinning
                .unk_offset4
                .as_ref()
                .and_then(|u| u.unk_offset4.as_ref())
            {
                for unk_bone in &unk4.bones {
                    let transform = infer_transform(
                        skinning,
                        unk_bone.bone_index as usize,
                        unk_bone.parent_index as usize,
                    );

                    update_bone(
                        &mut bones,
                        skinning,
                        unk_bone.bone_index,
                        unk_bone.parent_index,
                        transform,
                    );
                }
            }
        }

        // Check ordering constraints to enable more efficient animation code.
        for (i, bone) in bones.iter().enumerate() {
            if let Some(p) = bone.parent_index
                && i < p
            {
                warn!("Bone {i} appears before parent {p} and will not animate properly.")
            }
        }

        // The way skeleton creation is defined above should only produce a single root.
        // A single root improves compatibility with other programs.
        let root_bone_count = bones.iter().filter(|b| b.parent_index.is_none()).count();
        if root_bone_count > 1 {
            error!("Skeleton contains {root_bone_count} root bones.")
        }

        Self { bones }
    }

    // TODO: Test this?
    pub fn from_legacy_skeleton(hkt: &Hkt, models: &xc3_lib::mxmd::legacy::Models) -> Self {
        // TODO: make the hkt optional since the skinning has most parenting information?
        let mut bones: Vec<_> = hkt
            .names
            .iter()
            .zip(hkt.parent_indices.iter())
            .zip(hkt.transforms.iter())
            .map(|((name, parent_index), transform)| Bone {
                name: name.name.clone(),
                transform: Transform {
                    translation: vec3(
                        transform.translation[0],
                        transform.translation[1],
                        transform.translation[2],
                    ),
                    rotation: Quat::from_array(transform.rotation_quaternion),
                    scale: vec3(transform.scale[0], transform.scale[1], transform.scale[2]),
                },
                parent_index: (*parent_index).try_into().ok(),
            })
            .collect();

        // Add any missing bones from the skinning information.
        // TODO: Does the bone order need to match the skeleton for animations to work?
        for name in &models.bone_names {
            if !bones.iter().any(|b| b.name == name.name)
                && let Some(skinning_bone) = models.bones.iter().find(|b| b.name == name.name)
            {
                let transform = Mat4::from_cols_array_2d(&skinning_bone.transform);
                let (scale, rotation, translation) = transform.to_scale_rotation_translation();
                bones.push(Bone {
                    name: name.name.clone(),
                    transform: Transform {
                        translation,
                        rotation,
                        scale,
                    },
                    parent_index: None,
                });
            }
        }

        // Apply parenting information from the skinning.
        for skinning_bone in &models.bones {
            let parent_index = find_legacy_parent_index(models, &bones, skinning_bone);
            if let Some(bone) = bones.iter_mut().find(|b| b.name == skinning_bone.name) {
                // Don't affect parenting for already parented bones.
                // TODO: Why are some bones missing a parent in the skinning?
                bone.parent_index = bone.parent_index.or(parent_index);
            }
        }

        Self { bones }
    }

    /// The global transform for each bone in model space
    /// by recursively applying the parent transform.
    ///
    /// This is also known as the bone's "rest pose" or "bind pose".
    /// For inverse bind matrices, convert the transforms to a matrix and invert.
    pub fn model_space_transforms(&self) -> Vec<Transform> {
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

fn find_legacy_parent_index(
    models: &xc3_lib::mxmd::legacy::Models,
    bones: &[Bone],
    skinning_bone: &xc3_lib::mxmd::legacy::Bone,
) -> Option<usize> {
    // Convert skinning parent index to skeleton parent index.
    let parent_index = usize::try_from(skinning_bone.parent_index).ok()?;
    let parent_name = models.bones.get(parent_index).map(|b| &b.name)?;
    bones.iter().position(|b| &b.name == parent_name)
}

/// Merge all bones in `skeletons` into a single [Skeleton].
pub fn merge_skeletons(skeletons: &[Skeleton]) -> Option<Skeleton> {
    let (base, skeletons) = skeletons.split_first()?;
    let mut combined = base.clone();

    // Merge each bone instead of finding the skeleton with more bones.
    // This is necessary since model skinning can define additional bones.
    for skeleton in skeletons {
        for bone in &skeleton.bones {
            if !combined.bones.iter().any(|b| b.name == bone.name) {
                // Assume bones appear after their parents.
                // TODO: Do this in two passes to avoid this assumption?
                let parent_index = bone
                    .parent_index
                    .and_then(|i| skeleton.bones.get(i))
                    .and_then(|p| combined.bones.iter().position(|b| b.name == p.name));
                combined.bones.push(Bone {
                    parent_index,
                    ..bone.clone()
                });
            }
        }
    }

    Some(combined)
}

fn infer_transform(
    skinning: &xc3_lib::mxmd::Skinning,
    bone_index: usize,
    parent_index: usize,
) -> Mat4 {
    // The transform can be inferred from accumulated transforms.
    let transform = skinning
        .inverse_bind_transforms
        .get(bone_index)
        .map(|transform| Mat4::from_cols_array_2d(transform).inverse())
        .unwrap_or(Mat4::IDENTITY);

    if let Some(parent_inverse) = skinning
        .inverse_bind_transforms
        .get(parent_index)
        .map(Mat4::from_cols_array_2d)
    {
        parent_inverse * transform
    } else {
        transform
    }
}

fn update_bone(
    bones: &mut [Bone],
    skinning: &xc3_lib::mxmd::Skinning,
    bone_index: u16,
    parent_index: u16,
    transform: Mat4,
) {
    // TODO: Don't assume these bones are all parented?
    let bone_name = &skinning.bones[bone_index as usize].name;
    let parent_name = &skinning.bones[parent_index as usize].name;
    let parent_index = bones.iter().position(|b| &b.name == parent_name);

    if let Some(bone) = bones.iter_mut().find(|b| &b.name == bone_name) {
        bone.transform = Transform::from_matrix(transform);
        bone.parent_index = parent_index;
    }
}

fn bone_transform(b: &xc3_lib::bc::Transform) -> Transform {
    Transform {
        translation: vec3(b.translation[0], b.translation[1], b.translation[2]),
        rotation: Quat::from_array(b.rotation_quaternion),
        scale: vec3(b.scale[0], b.scale[1], b.scale[2]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Test global/world transforms and inverse bind transforms

    #[test]
    fn merge_skeletons_empty() {
        assert!(merge_skeletons(&[]).is_none());
    }

    #[test]
    fn merge_single_skeletons() {
        assert_eq!(
            Some(Skeleton {
                bones: vec![
                    Bone {
                        name: "a".to_string(),
                        transform: Transform::IDENTITY,
                        parent_index: None
                    },
                    Bone {
                        name: "b".to_string(),
                        transform: Transform {
                            scale: vec3(2.0, 2.0, 2.0),
                            ..Transform::IDENTITY
                        },
                        parent_index: Some(0)
                    },
                ]
            }),
            merge_skeletons(&[Skeleton {
                bones: vec![
                    Bone {
                        name: "a".to_string(),
                        transform: Transform::IDENTITY,
                        parent_index: None
                    },
                    Bone {
                        name: "b".to_string(),
                        transform: Transform {
                            scale: vec3(2.0, 2.0, 2.0),
                            ..Transform::IDENTITY
                        },
                        parent_index: Some(0)
                    }
                ]
            }])
        );
    }

    #[test]
    fn merge_two_skeletons() {
        assert_eq!(
            Some(Skeleton {
                bones: vec![
                    Bone {
                        name: "a".to_string(),
                        transform: Transform::IDENTITY,
                        parent_index: None
                    },
                    Bone {
                        name: "b".to_string(),
                        transform: Transform {
                            scale: vec3(2.0, 2.0, 2.0),
                            ..Transform::IDENTITY
                        },
                        parent_index: None
                    },
                    Bone {
                        name: "c".to_string(),
                        transform: Transform {
                            scale: vec3(3.0, 3.0, 3.0),
                            ..Transform::IDENTITY
                        },
                        parent_index: Some(1)
                    }
                ]
            }),
            merge_skeletons(&[
                Skeleton {
                    bones: vec![Bone {
                        name: "a".to_string(),
                        transform: Transform::IDENTITY,
                        parent_index: None
                    }]
                },
                Skeleton {
                    bones: vec![
                        Bone {
                            name: "b".to_string(),
                            transform: Transform {
                                scale: vec3(2.0, 2.0, 2.0),
                                ..Transform::IDENTITY
                            },
                            parent_index: None
                        },
                        Bone {
                            name: "a".to_string(),
                            transform: Transform {
                                scale: vec3(-1.0, -1.0, -1.0),
                                ..Transform::IDENTITY
                            },
                            parent_index: None
                        },
                        Bone {
                            name: "c".to_string(),
                            transform: Transform {
                                scale: vec3(3.0, 3.0, 3.0),
                                ..Transform::IDENTITY
                            },
                            parent_index: Some(0)
                        }
                    ]
                }
            ])
        );
    }
}
