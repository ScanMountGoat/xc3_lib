// "gTCol" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(0)
var g_color: texture_2d<f32>;

// "gTEtc" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(1)
var g_etc_buffer: texture_2d<f32>;

// "gTNom" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(2)
var g_normal: texture_2d<f32>;

@group(0) @binding(3)
var g_velocity: texture_2d<f32>;

// "gTDep" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(4)
var g_depth: texture_2d<f32>;

// "gTSpecularCol" in "clustered" in monolib/shader/shd_lgt.wishp.
@group(0) @binding(5)
var g_lgt_color: texture_2d<f32>;

@group(1) @binding(0)
var shared_sampler: sampler;

struct DebugSettings {
    index: vec4<u32>
}

struct Camera {
    view: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec4<f32>
}

@group(1) @binding(1)
var<uniform> camera: Camera;

@group(1) @binding(2)
var<uniform> debug_settings: DebugSettings;

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

// Standard GGX BRDF in "clustered" in monolib/shader/shd_lgt.wishp.
fn ggx_brdf(roughness: f32, n_dot_h: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let pi = 3.14159;
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denominator = (n_dot_h2) * (a2 - 1.0) + 1.0;
    return a2 / (pi * denominator * denominator);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let g_color = textureSample(g_color, shared_sampler, in.uv);
    let g_etc_buffer = textureSample(g_etc_buffer, shared_sampler, in.uv);
    let g_normal = textureSample(g_normal, shared_sampler, in.uv);
    let g_velocity = textureSample(g_velocity, shared_sampler, in.uv);
    let g_depth = textureSample(g_depth, shared_sampler, in.uv);
    let g_lgt_color = textureSample(g_lgt_color, shared_sampler, in.uv);

    let albedo = g_color.rgb;
    let metalness = g_etc_buffer.r;
    let glossiness = g_etc_buffer.g;
    
    // TODO: clamped using constant buffer?
    let roughness = clamp(1.0 - glossiness, 0.04, 0.995);

    let ambient_occlusion = g_normal.z;

    // Unpack the view space normals.
    let normal_x = g_normal.x * 2.0 - 1.0;
    let normal_y = g_normal.y * 2.0 - 1.0;
    let normal_z = sqrt(abs(1.0 - normal_x * normal_x - normal_y * normal_y));
    let normal = vec3(normal_x, normal_y, normal_z);

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

    let f0 = mix(vec3(0.04), albedo, metalness);

    let k_specular = f0;
    let k_diffuse = 1.0 - metalness;

    output = albedo * k_diffuse * diffuse_lighting + specular_lighting * k_specular * ambient_occlusion;

    // TODO: Depth test to avoid writing to all pixels?
    switch (debug_settings.index.x) {
        case 1u: {
            return g_color;
        }
        case 2u: {
            return g_etc_buffer;
        }
        case 3u: {
            return g_normal;
        }
        case 4u: {
            return g_velocity;
        }
        case 5u: {
            return g_depth;
        }
        case 6u: {
            return g_lgt_color;
        }
        default: {
            return vec4(output, 1.0);
        }
    }
}