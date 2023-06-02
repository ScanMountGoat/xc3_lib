struct Camera {
    view_projection: mat4x4<f32>,
    position: vec4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

// PerMaterial values.
// Define all possible parameters even if unused.
// The "ubershader" approach makes it possible to generate WGSL bindings at build time.
@group(1) @binding(0)
var s0: texture_2d<f32>;

@group(1) @binding(1)
var s1: texture_2d<f32>;

@group(1) @binding(2)
var s2: texture_2d<f32>;

@group(1) @binding(3)
var s3: texture_2d<f32>;

@group(1) @binding(4)
var s4: texture_2d<f32>;

@group(1) @binding(5)
var s5: texture_2d<f32>;

@group(1) @binding(6)
var s6: texture_2d<f32>;

@group(1) @binding(7)
var s7: texture_2d<f32>;

@group(1) @binding(8)
var s8: texture_2d<f32>;

@group(1) @binding(9)
var s9: texture_2d<f32>;

// TODO: Multiple samplers?
@group(1) @binding(10)
var shared_sampler: sampler;

// TODO: Define all possible attributes and have accessors fill them in.
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(2) weight_index: u32,
    @location(3) vertex_color: vec4<f32>,
    @location(4) normal: vec4<f32>,
    @location(5) tangent: vec4<f32>,
    @location(6) uv1: vec4<f32>, // TODO: padding?
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec4<f32>,
    @location(4) vertex_color: vec4<f32>,
}

// TODO: How to handle multiple inputs for each output channel?
// Texture and channel input for each output channel.
struct GBufferAssignment {
    sampler_indices: vec4<i32>,
    channel_indices: vec4<u32>
}

@group(2) @binding(0)
var<uniform> gbuffer_assignments: array<GBufferAssignment, 6>;

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_projection * vec4<f32>(in.position.xyz, 1.0);
    out.position = in.position.xyz;
    out.uv1 = in.uv1.xy;
    out.normal = in.normal.xyz;
    out.tangent = in.tangent;
    out.vertex_color = in.vertex_color;
    return out;
}

fn assign_gbuffer_texture(assignment: GBufferAssignment, s_colors: array<vec4<f32>, 10>, default_value: vec4<f32>) -> vec4<f32> {
    let x = assign_channel(assignment.sampler_indices.x, assignment.channel_indices.x, s_colors, default_value.x);
    let y = assign_channel(assignment.sampler_indices.y, assignment.channel_indices.y, s_colors, default_value.y);
    let z = assign_channel(assignment.sampler_indices.z, assignment.channel_indices.z, s_colors, default_value.z);
    let w = assign_channel(assignment.sampler_indices.w, assignment.channel_indices.w, s_colors, default_value.w);
    return vec4(x, y, z, w);
}

fn assign_channel(sampler_index: i32, channel_index: u32, s_colors: array<vec4<f32>, 10>, default_value: f32) -> f32 {
    // TODO: Is there a way to avoid needing a switch?
    switch (sampler_index) {
        case 0: {
            return s_colors[0][channel_index];
        }
        case 1: {
            return s_colors[1][channel_index];
        }
        case 2: {
            return s_colors[2][channel_index];
        }
        case 3: {
            return s_colors[3][channel_index];
        }
        case 4: {
            return s_colors[4][channel_index];
        }
        case 5: {
            return s_colors[5][channel_index];
        }
        case 6: {
            return s_colors[6][channel_index];
        }
        case 7: {
            return s_colors[7][channel_index];
        }
        case 8: {
            return s_colors[8][channel_index];
        }
        case 9: {
            return s_colors[9][channel_index];
        }
        default: {
            return default_value;
        }
    }
}

// TODO: Is it worth porting the in game code for this?
fn apply_normal_map(normal: vec3<f32>, tangent: vec3<f32>, bitangent: vec3<f32>, normal_map: vec2<f32>) -> vec3<f32> {
    // Remap the tangent space normal map to the correct range.
    let x = 2.0 * normal_map.x - 1.0;
    let y = 2.0 * normal_map.y - 1.0;

    // Calculate z based on the fact that x*x + y*y + z*z = 1.
    // Clamp to ensure z is positive.
    let z = sqrt(max(1.0 - (x * x) + (y * y), 0.0));

    // Normal mapping is a change of basis using the TBN vectors.
    let nor = vec3(x, y, z);
    let newNormal = tangent * nor.x + bitangent * nor.y + normal * nor.z;
    return normalize(newNormal);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view = normalize(camera.position.xyz - in.position.xyz);

    // TODO: Normalize vectors?
    let tangent = normalize(in.tangent.xyz);
    // TODO: Flip the sign?
    let bitangent = cross(normalize(in.normal.xyz), normalize(in.tangent.xyz)) * in.tangent.w;

    // TODO: Handle missing samplers?
    let s0_color = textureSample(s0, shared_sampler, in.uv1);
    let s1_color = textureSample(s1, shared_sampler, in.uv1);
    let s2_color = textureSample(s2, shared_sampler, in.uv1);
    let s3_color = textureSample(s3, shared_sampler, in.uv1);
    let s4_color = textureSample(s4, shared_sampler, in.uv1);
    let s5_color = textureSample(s5, shared_sampler, in.uv1);
    let s6_color = textureSample(s6, shared_sampler, in.uv1);
    let s7_color = textureSample(s7, shared_sampler, in.uv1);
    let s8_color = textureSample(s8, shared_sampler, in.uv1);
    let s9_color = textureSample(s9, shared_sampler, in.uv1);

    let s_colors = array<vec4<f32>, 10>(
        s0_color, 
        s1_color, 
        s2_color, 
        s3_color, 
        s4_color, 
        s5_color,
        s6_color,
        s7_color,
        s8_color,
        s9_color,
    );

    // The layout of G-Buffer textures is fixed but assignments are not.
    // Each material in game can have a unique shader program.
    // The ordering here is the order of per material fragment shader outputs.
    // The input order for the deferred lighting pass is slightly different.
    // TODO: How to properly handle missing assignments?
    let g0 = assign_gbuffer_texture(gbuffer_assignments[0], s_colors, vec4(0.0));
    let g1 = assign_gbuffer_texture(gbuffer_assignments[1], s_colors, vec4(0.0));
    let g2 = assign_gbuffer_texture(gbuffer_assignments[2], s_colors, vec4(0.0));
    let g3 = assign_gbuffer_texture(gbuffer_assignments[3], s_colors, vec4(0.0));
    let g4 = assign_gbuffer_texture(gbuffer_assignments[4], s_colors, vec4(0.0));
    let g5 = assign_gbuffer_texture(gbuffer_assignments[5], s_colors, vec4(0.0));

    // TODO: proper sRGB gamma conversion.
    // Each G-Buffer texture and channel always has the same usage.
    let albedo = pow(g0.xyz, vec3(2.2));
    let metalness = g1.x;
    let normal_map = g2.xy;
    let ambient_occlusion = g2.z;
    let emission = g5.xyz;

    // TODO: Normalize vertex normals?
    let normal = apply_normal_map(normalize(in.normal.xyz), tangent, bitangent, normal_map);

    // Basic lambertian diffuse and phong specular for testing purposes.
    let diffuse_lighting = dot(view, normal) * 0.5 + 0.5;
    let reflection = reflect(-view, normal);
    let specular_lighting = pow(max(dot(view, reflection), 0.0), 8.0);

    // TODO: Proper metalness.
    // TODO: Ambient occlusion can be set via material or constant?
    var diffuse = albedo * diffuse_lighting * (1.0 - metalness * 0.5);
    var specular = specular_lighting * mix(vec3(0.25), albedo, metalness);
    var color = diffuse + specular;

    // TODO: alpha?
    return vec4(color, 1.0);
}
