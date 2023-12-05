// "gTEtc" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(0)
var g_etc_buffer: texture_2d<f32>;

@group(0) @binding(1)
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

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // Adapted from "unbranch_to_depth" in monolib/shader/shd_post.
    // Extract the material ID from the first 3 bits.
    let g_etc_buffer = textureSample(g_etc_buffer, shared_sampler, in.uv);
    let mat_id = u32(g_etc_buffer.a * 255.0 + 0.1) & 0x7u;

    // Assume a Depth16 output format.
    // This creates an ID mask to use with depth function equals.
    var output: FragmentOutput;
    output.depth = (f32(mat_id) + 1.0) / 65535.0;
    // Avoid writing depth to unused fragments.
    // TODO: The in game check uses the model depth buffer.
    if g_etc_buffer.a == 0.0 {
        output.depth = 0.0;
    }
    return output;
}