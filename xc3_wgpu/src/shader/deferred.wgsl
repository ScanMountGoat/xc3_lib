@group(0) @binding(0)
var g0: texture_2d<f32>;

@group(0) @binding(1)
var g1: texture_2d<f32>;

@group(0) @binding(2)
var g2: texture_2d<f32>;

@group(0) @binding(3)
var g3: texture_2d<f32>;

@group(0) @binding(4)
var g4: texture_2d<f32>;

@group(0) @binding(5)
var g5: texture_2d<f32>;

@group(0) @binding(6)
var g6: texture_2d<f32>;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let g0 = textureSample(g0, shared_sampler, in.uv);
    let g1 = textureSample(g1, shared_sampler, in.uv);
    let g2 = textureSample(g2, shared_sampler, in.uv);
    let g3 = textureSample(g3, shared_sampler, in.uv);
    let g4 = textureSample(g4, shared_sampler, in.uv);
    let g5 = textureSample(g5, shared_sampler, in.uv);

    let albedo = g0.rgb;
    let metalness = g1.r;
    let glossiness = g1.g;
    let ambient_occlusion = g2.z;

    // Unpack the view space normals.
    let normal_x = g2.x * 2.0 - 1.0;
    let normal_y = g2.y * 2.0 - 1.0;
    let normal_z = sqrt(abs(1.0 - normal_x * normal_x - normal_y * normal_y));
    let normal = vec3(normal_x, normal_y, normal_z);

    var output = vec3(0.0);

    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);
    let reflection = reflect(view, normal);

    // Basic lambertian diffuse and ggx specular for testing purposes.
    let n_dot_v = max(dot(view, normal), 0.0);
    let diffuse_indirect = 0.35 * ambient_occlusion;
    let diffuse_direct = 1.0;
    let diffuse_lighting = mix(diffuse_indirect, diffuse_direct, n_dot_v);

    let a = max(1.0 - glossiness, 0.01);
    let a2 = a * a;
    let pi = 3.14159;
    let n_dot_h2 = n_dot_v * n_dot_v;
    let denominator = ((n_dot_h2) * (a2 - 1.0) + 1.0);
    let ggx = a2 / (pi * denominator * denominator);
    let specular_lighting = ggx * 0.15;

    let specular_color = mix(albedo, vec3(1.0), metalness) * ambient_occlusion;

    output = albedo * diffuse_lighting + specular_lighting * specular_color;

    switch (debug_settings.index.x) {
        case 1u: {
            return g0;
        }
        case 2u: {
            return g1;
        }
        case 3u: {
            return g2;
        }
        case 4u: {
            return g3;
        }
        case 5u: {
            return g4;
        }
        case 6u: {
            return g5;
        }
        default: {
            return vec4(output, 1.0);
        }
    }
}