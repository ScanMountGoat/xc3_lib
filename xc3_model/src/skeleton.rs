use glam::{vec3, Mat4, Quat};

// TODO: Assume bones appear after their parents?
#[derive(Debug)]
pub struct Skeleton {
    /// The hierarchy of bones in the skeleton.
    pub bones: Vec<Bone>,
}

#[derive(Debug)]
pub struct Bone {
    pub name: String,
    /// The local transform of the bone relative to its parent.
    pub transform: Mat4,
    /// The index of the parent [Bone] in [bones](struct.Skeleton.html#structfield.bones)
    /// or `None` if this is a root bone.
    pub parent_index: Option<usize>,
}

impl Skeleton {
    // TODO: Test this?
    // TODO: Also accept mxmd skeleton?
    pub fn from_skel(skel: &xc3_lib::sar1::Skel) -> Self {
        Self {
            bones: skel
                .names
                .elements
                .iter()
                .zip(skel.transforms.elements.iter())
                .zip(skel.parents.elements.iter())
                .map(|((name, transform), parent)| Bone {
                    name: name.name.clone(),
                    transform: bone_transform(transform),
                    parent_index: if *parent < 0 {
                        None
                    } else {
                        Some(*parent as usize)
                    },
                })
                .collect(),
        }
    }
}

// TODO: Test the order of transforms.
fn bone_transform(b: &xc3_lib::sar1::Transform) -> Mat4 {
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
