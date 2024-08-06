@group(0) @binding(0)
var g_color: texture_2d<f32>;

@group(0) @binding(1)
var g_depth: texture_2d<f32>;

@group(0) @binding(2)
var shared_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    // A fullscreen triangle using index calculations.
    var out: VertexOutput;
    let x = f32((i32(in_vertex_index) << 1u) & 2);
    let y = f32(i32(in_vertex_index & 2u));
    out.position = vec4(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2(x, 1.0 - y);
    return out;
}

fn closest(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    if distance(a, c) < distance(b, c) {
        return a;
    } else {
        return b;
    }
}

fn unpack_depth(depth: vec3<f32>) -> f32 {
    return depth.z * 8128.125 + depth.y * 31.875 + depth.x * 0.125;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Adapted from "snnFilterFast" in xeno3/monolib/shader/shd_post.
    // Symmetric nearest neighbor (SNN) is an edge preserving blur kernel.
    // The effect is similar to the Kuwahara or "oil paint" filter.
    let c = textureSample(g_color, shared_sampler, in.uv).rgb;

    let gt_dep = textureSample(g_depth, shared_sampler, in.uv).xyz;
    let depth = unpack_depth(gt_dep);

    // TODO: Should these parameters change with the camera?
    let depth_remapped = clamp((depth - 1.43352) / (8.60111 - 1.43352 + 0.0001), 0.0, 1.0);
    let depth_scale = 2.0 / 3.0 * (1.0 - depth_remapped);
    // This is only horizontal since uvOffset.y is 0.0.
    let scale = vec2(depth_scale, 0.0);

    // Calculate offsets in terms of pixels.
    let offset = 1.0 / vec2<f32>(textureDimensions(g_color)) * scale;

    let c1 = textureSample(g_color, shared_sampler, in.uv + offset * 6.5).rgb;
    let c2 = textureSample(g_color, shared_sampler, in.uv + offset * -6.5).rgb;
    let c3 = textureSample(g_color, shared_sampler, in.uv + offset * 4.5).rgb;
    let c4 = textureSample(g_color, shared_sampler, in.uv + offset * -4.5).rgb;
    let c5 = textureSample(g_color, shared_sampler, in.uv + offset * 2.5).rgb;
    let c6 = textureSample(g_color, shared_sampler, in.uv + offset * -2.5).rgb;

    let sum = 2.0 * (closest(c1, c2, c) + closest(c3, c4, c) + closest(c5, c6, c)) + c;
    return vec4(sum / 7.0, 1.0);
}