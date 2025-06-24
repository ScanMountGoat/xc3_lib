// "gTEtc" in "clustered" in xeno3/monolib/shader/shd_lgt.wishp.
@group(0) @binding(0)
var g_etc_buffer: texture_2d<f32>;

@group(0) @binding(1)
var g_depth: texture_2d<f32>;

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
fn fs_main(in: VertexOutput) -> @builtin(frag_depth) f32 {
    // Adapted from "unbranch_to_depth" in xeno3/monolib/shader/shd_post.
    let coords = vec2<u32>(in.uv * vec2<f32>(textureDimensions(g_etc_buffer)));

    // Extract the material ID from the first 3 bits.
    let g_etc_buffer = textureLoad(g_etc_buffer, coords, 0);
    let mat_id = u32(g_etc_buffer.w * 255.0 + 0.1) & 0x7u;

    // Avoid writing depth to unused fragments.
    // TODO: The in game check uses bit operations with stencil?
    let g_depth = textureLoad(g_depth, coords, 0).z;
    if g_depth != 1.0 {
        // Assume a Depth16 output format.
        // This creates an ID mask to use with depth function equals.
        return (f32(mat_id) + 1.0) / 65535.0;
    } else {
        return 0.0;
    }
}