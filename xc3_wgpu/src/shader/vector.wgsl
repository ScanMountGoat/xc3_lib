// PerScene values.
@group(0) @binding(0)
var<uniform> camera: super::camera::Camera;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // TODO: scale in screenspace?
    var out: VertexOutput;
    out.clip_position = camera.view_projection * vec4(in.position.xyz, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(in.color.rgb, 1.0);
}