// PerScene values.
struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

// PerDraw values.
struct Uniforms {
    color: vec4<f32>
}

@group(1) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return camera.view_projection * vec4(in.position.xyz, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(uniforms.color);
}