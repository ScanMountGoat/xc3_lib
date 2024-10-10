// PerPass resources.
struct DebugSettings {
    render_mode: u32
}

@group(0) @binding(0)
var<uniform> debug_settings: DebugSettings;

struct RenderSettings {
    mat_id: u32,
}

// PerPass resources that may change when resized.
// TODO: Where to put this since it's only used for the toon shader?
@group(0) @binding(1)
var g_toon_grad: texture_2d<f32>;

@group(0) @binding(2)
var shared_sampler: sampler;

// "gTCol" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(1) @binding(0)
var g_color: texture_2d<f32>;

// "gTEtc" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(1) @binding(1)
var g_etc_buffer: texture_2d<f32>;

// "gTNom" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(1) @binding(2)
var g_normal: texture_2d<f32>;

@group(1) @binding(3)
var g_velocity: texture_2d<f32>;

// "gTDep" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(1) @binding(4)
var g_depth: texture_2d<f32>;

// TODO: the output at index 5 can be specular color or emission?
@group(1) @binding(5)
var g_lgt_color: texture_2d<f32>;

// "gTSpecularCol" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(1) @binding(6)
var g_specular_color: texture_2d<f32>;

// PerDraw resources for each material ID type.
@group(2) @binding(0)
var<uniform> render_settings: RenderSettings;

// TODO: Create uniform arrays with max length 256 for these lights?
// TODO: Hardcode lighting from the character menu in xc3 for now?
// Uniform structs from program "clustered" from monolib/shader/shd_lgt.wishp.
struct PointLight {
    col: vec4<f32>,
    position: vec4<f32>,
    etc: vec4<f32>
}

struct SpotLight {
    col: vec4<f32>,
    position: vec4<f32>,
    etc: vec4<f32>,
    vector: vec4<f32>
}

struct ToonData {
    head_left_dir: vec4<f32>,
    head_up_dir: vec4<f32>,
    head_forward_dir: vec4<f32>,
}

// wgpu recommends @invariant for position with depth func equals.
struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32
}

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

// Standard GGX BRDF in "clustered" in monolib/shader/shd_lgt.wishp.
fn ggx_brdf(roughness: f32, n_dot_h: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let pi = 3.14159;
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denominator = (n_dot_h2) * (a2 - 1.0) + 1.0;
    return a2 / (pi * denominator * denominator);
}

fn mat_id_depth(id: u32) -> f32 {
    // Assume a Depth16 depth buffer.
    // wgpu maps [0.0, 1.0] to depth instead of [-1.0, 1.0].
    return f32(id + 1u) / 65535.0;
}

fn unpack_normal(g_normal: vec2<f32>) -> vec3<f32> {
    // Unpack the view space normals.
    let normal_x = g_normal.x * 2.0 - 1.0;
    let normal_y = g_normal.y * 2.0 - 1.0;
    let normal_z = sqrt(abs(1.0 - normal_x * normal_x - normal_y * normal_y));
    return vec3(normal_x, normal_y, normal_z);
}

// Adapted from "unbranch_to_depth" in xeno3/monolib/shader/shd_post.
fn unpack_etc_flags(etc_a: f32) -> u32 {
    // This is also the same logic used in the deferred shaders.
    return u32(etc_a * 255.0 + 0.1);
}

fn calculate_color(uv: vec2<f32>) -> vec4<f32> {
    let g_color = textureSample(g_color, shared_sampler, uv);
    let g_etc_buffer = textureSample(g_etc_buffer, shared_sampler, uv);
    let g_normal = textureSample(g_normal, shared_sampler, uv);
    let g_velocity = textureSample(g_velocity, shared_sampler, uv);
    let g_depth = textureSample(g_depth, shared_sampler, uv);
    let g_lgt_color = textureSample(g_lgt_color, shared_sampler, uv);

    let albedo = g_color.rgb;
    let metalness = g_etc_buffer.r;
    let glossiness = g_etc_buffer.g;
    
    // TODO: clamped using constant buffer?
    let roughness = clamp(1.0 - glossiness, 0.04, 0.995);

    let normal = unpack_normal(g_normal.xy);
    let ambient_occlusion = g_normal.z;

    var output = vec3(0.0);

    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);
    let reflection = reflect(view, normal);

    let n_dot_v = max(dot(view, normal), 0.0);
    // TODO: Calculate this from lighting vectors.
    let n_dot_h = n_dot_v;
    
    // Basic lambertian diffuse for testing purposes.
    let diffuse_indirect = 0.35 * ambient_occlusion;
    let diffuse_direct = 1.0;
    let diffuse_lighting = mix(diffuse_indirect, diffuse_direct, n_dot_v);

    let ggx = ggx_brdf(roughness, n_dot_h);

    // TODO: ambient specular using BRDF map?
    let specular_lighting = ggx + 0.25;

    // TODO: fresnel?
    let f0 = mix(vec3(0.08), albedo, metalness);

    let k_specular = f0;
    let k_diffuse = 1.0 - metalness;

    output = albedo * k_diffuse * diffuse_lighting + specular_lighting * k_specular * ambient_occlusion;
    output += g_lgt_color.rgb;

    return vec4(output, 1.0);
}

fn calculate_toon_color(uv: vec2<f32>) -> vec4<f32> {
    let g_color = textureSample(g_color, shared_sampler, uv);
    let g_etc_buffer = textureSample(g_etc_buffer, shared_sampler, uv);
    let g_normal = textureSample(g_normal, shared_sampler, uv);
    let g_velocity = textureSample(g_velocity, shared_sampler, uv);
    let g_depth = textureSample(g_depth, shared_sampler, uv);
    let g_specular_color = textureSample(g_specular_color, shared_sampler, uv);

    let albedo = g_color.rgb;
    let metalness = g_etc_buffer.r; // TODO: different for toon?
    let glossiness = g_etc_buffer.g;

    let flags = unpack_etc_flags(g_etc_buffer.a);

    // Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, createBitInfo,
    let b_specular_col = (flags & (1u << 4u)) != 0u;
    
    // TODO: clamped using constant buffer?
    let roughness = clamp(1.0 - glossiness, 0.04, 0.995);

    let normal = unpack_normal(g_normal.xy);
    let ambient_occlusion = g_normal.z;

    var output = vec3(0.0);

    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);
    let reflection = reflect(view, normal);

    let n_dot_v = max(dot(view, normal), 0.0);
    // TODO: Calculate this from lighting vectors.
    let n_dot_h = n_dot_v;
    
    // Basic lambertian diffuse for testing purposes.
    // TODO: Toon shading in game has many more parameters.
    let diffuse_indirect = 0.25 * ambient_occlusion;
    let diffuse_direct = 1.0;
    let diffuse_lighting = mix(diffuse_indirect, diffuse_direct, n_dot_v);

    // TODO: What is the default specular color?
    var specular_color = vec3(0.08);
    if b_specular_col {
        specular_color = g_specular_color.rgb;
    }

    // TODO: Does toon shading use ggx?
    let ggx = ggx_brdf(roughness, n_dot_h);

    // TODO: ambient specular using BRDF map?
    let specular_lighting = ggx;

    // TODO: fresnel?
    let f0 = mix(specular_color, albedo, metalness);
    // TODO: specular intensity when using specular map?
    let k_specular = f0;
    let k_diffuse = 1.0 - metalness;

    // TODO: Correctly use both gradients to fix massive melee mythra hair.
    // TODO: Are these the right gradients?
    let toon_v = toon_grad_v(g_etc_buffer.z);
    let toon_diffuse = textureSample(g_toon_grad, shared_sampler, vec2(diffuse_lighting, toon_v)).rgb;
    let toon_specular = specular_lighting;

    output = albedo * k_diffuse * toon_diffuse + toon_specular * k_specular * ambient_occlusion;

    return vec4(output, 1.0);
}

// TODO: entry points for each of the mat id types in game.
// 0, ouroboros core?
// 1, PBR
// 2, TOON
// 3, vegetation and colony 9 tents (sss?) also bathtub from xc2?
// 4, core crystal and levialord fins?
// 5, HAIR (xc3 only)

// Adapted from slct 0 nvsd 8 in xeno3/monolib/shader/shd_lgt.wishp.
fn toon_grad_v(etc_z: f32) -> f32 {
    // etc.z is often stored as row / 255.0 in the material work values.
    // Recover the row using round(etc.z * 255.0).
    let row_index = floor(etc_z * 255.0 + 0.5);
    // Select the center of a row from the 256x256 toon gradients texture.
    return (row_index + 0.5) / 256.0;
}

// Each material type is "masked" using depth function equals.
// TODO: How to share code between these?
@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = calculate_color(in.uv);
    out.depth = mat_id_depth(render_settings.mat_id);
    return out;
}

@fragment
fn fs_toon(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = calculate_toon_color(in.uv);
    out.depth = mat_id_depth(render_settings.mat_id);
    return out;
}

@fragment
fn fs_debug(in: VertexOutput) -> FragmentOutput {
    let g_color = textureSample(g_color, shared_sampler, in.uv);
    let g_etc_buffer = textureSample(g_etc_buffer, shared_sampler, in.uv);
    let g_normal = textureSample(g_normal, shared_sampler, in.uv);
    let g_velocity = textureSample(g_velocity, shared_sampler, in.uv);
    let g_depth = textureSample(g_depth, shared_sampler, in.uv);
    let g_lgt_color = textureSample(g_lgt_color, shared_sampler, in.uv);
    let g_specular_color = textureSample(g_specular_color, shared_sampler, in.uv);

    var out: FragmentOutput;
    switch (debug_settings.render_mode) {
        case 1u: {
            out.color = g_color;
        }
        case 2u: {
            out.color = g_etc_buffer;
        }
        case 3u: {
            out.color = g_normal;
        }
        case 4u: {
            out.color = g_velocity;
        }
        case 5u: {
            out.color = g_depth;
        }
        case 6u: {
            out.color = g_lgt_color;
        }
        case 7u: {
            out.color = g_specular_color;
        }
        default: {
            out.color = vec4(0.0);
        }
    }

    return out;
}