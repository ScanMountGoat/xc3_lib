use std::ops::Mul;

use glam::{Mat4, Quat, Vec3};

/// A decomposed transform as scale -> rotation -> translation (TRS).
///
/// Scale does not affect translation when multiplying [Transform].
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn to_matrix(self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
    }

    pub fn from_matrix(value: Mat4) -> Self {
        let (scale, rotation, translation) = value.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

impl Mul<Transform> for Transform {
    type Output = Transform;

    fn mul(self, rhs: Transform) -> Self::Output {
        // anm::TransformUtil::mul in the Xenoblade 2 binary.
        Transform {
            translation: self.rotation.mul_vec3(rhs.translation) + self.translation,
            rotation: self.rotation * rhs.rotation,
            scale: self.scale * rhs.scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{quat, vec3};

    #[test]
    fn transform_to_matrix() {
        assert_eq!(
            Mat4::from_cols_array_2d(&[
                [4.0, 0.0, 0.0, 0.0],
                [0.0, -5.0, 0.0, 0.0],
                [0.0, 0.0, -6.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]),
            Transform {
                translation: vec3(1.0, 2.0, 3.0),
                rotation: quat(1.0, 0.0, 0.0, 0.0),
                scale: vec3(4.0, 5.0, 6.0),
            }
            .to_matrix()
        );
    }

    #[test]
    fn transform_from_matrix() {
        assert_eq!(
            Transform {
                translation: vec3(1.0, 2.0, 3.0),
                rotation: quat(1.0, 0.0, 0.0, 0.0),
                scale: vec3(4.0, 5.0, 6.0),
            },
            Transform::from_matrix(Mat4::from_cols_array_2d(&[
                [4.0, 0.0, 0.0, 0.0],
                [0.0, -5.0, 0.0, 0.0],
                [0.0, 0.0, -6.0, 0.0],
                [1.0, 2.0, 3.0, 1.0],
            ]))
        );
    }
}
