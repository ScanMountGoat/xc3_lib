use glam::Mat4;
use xc3_model::animation::Animation;

pub fn animated_skinning_transforms(
    skeleton: &xc3_model::Skeleton,
    animation: &Animation,
    current_time_seconds: f32,
) -> [Mat4; 256] {
    let frame = animation.current_frame(current_time_seconds);
    let mut transforms = animation.skinning_transforms(skeleton, frame);
    transforms.resize(256, Mat4::IDENTITY);
    transforms.try_into().unwrap()
}
