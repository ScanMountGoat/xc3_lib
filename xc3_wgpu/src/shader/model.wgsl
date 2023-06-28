struct Camera {
    view: mat4x4<f32>,
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

struct FragmentOutput {
    @location(0) g0: vec4<f32>,
    @location(1) g1: vec4<f32>,
    @location(2) g2: vec4<f32>,
    @location(3) g3: vec4<f32>,
    @location(4) g4: vec4<f32>,
    @location(5) g5: vec4<f32>,
    @location(6) g6: vec4<f32>,
}

// TODO: How to handle multiple inputs for each output channel?
// Texture and channel input for each output channel.
struct GBufferAssignment {
    sampler_indices: vec4<i32>,
    channel_indices: vec4<u32>
}

@group(2) @binding(0)
var<uniform> gbuffer_assignments: array<GBufferAssignment, 6>;

// TODO: Sort by frequency like pass, model, mesh, etc.
@group(3) @binding(0)
var<uniform> per_model: PerModel;

struct PerModel {
    matrix: mat4x4<f32>
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_projection * per_model.matrix * vec4(vertex.position.xyz, 1.0);
    out.position = vertex.position.xyz;
    out.uv1 = vertex.uv1.xy;
    out.vertex_color = vertex.vertex_color;
    // Transform any direction vectors by the instance transform.
    out.normal = (per_model.matrix * vec4(vertex.normal.xyz, 0.0)).xyz;
    out.tangent = vec4((per_model.matrix * vec4(vertex.tangent.xyz, 0.0)).xyz, vertex.tangent.w);
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
    let z = sqrt(abs(1.0 - (x * x) + (y * y)));

    // Normal mapping is a change of basis using the TBN vectors.
    let nor = vec3(x, y, z);
    let newNormal = tangent * nor.x + bitangent * nor.y + normal * nor.z;
    return normalize(newNormal);
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // TODO: Normalize vectors?
    let tangent = normalize(in.tangent.xyz);
    let vertex_normal = normalize(in.normal.xyz);
    // TODO: Flip the sign?
    let bitangent = cross(vertex_normal, tangent) * in.tangent.w;

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
    let g0 = assign_gbuffer_texture(gbuffer_assignments[0], s_colors, vec4(1.0));
    let g1 = assign_gbuffer_texture(gbuffer_assignments[1], s_colors, vec4(0.0));
    let g2 = assign_gbuffer_texture(gbuffer_assignments[2], s_colors, vec4(0.5, 0.5, 1.0, 0.0));
    let g3 = assign_gbuffer_texture(gbuffer_assignments[3], s_colors, vec4(0.0));
    let g4 = assign_gbuffer_texture(gbuffer_assignments[4], s_colors, vec4(0.0));
    let g5 = assign_gbuffer_texture(gbuffer_assignments[5], s_colors, vec4(0.0));

    // TODO: How much of this goes into deferred?
    // Assume each G-Buffer texture and channel always has the same usage.
    let normal_map = g2.xy;

    // Not all materials and shaders use normal mapping.
    // TODO: Is this a good way to check for this?
    var normal = vertex_normal;
    if (gbuffer_assignments[2].sampler_indices.x != -1 && gbuffer_assignments[2].sampler_indices.y != -1) {
        normal = apply_normal_map(vertex_normal, tangent, bitangent, normal_map);
    }

    // TODO: Are in game normals in view space?
    let view_normal = camera.view * vec4(normal.xyz, 0.0);

    // TODO: alpha?
    // TODO: How much shading is done in this pass?
    var out: FragmentOutput;
    out.g0 = g0;
    out.g1 = g1;
    out.g2 = vec4(normalize(view_normal).xy * 0.5 + 0.5, g2.zw);
    out.g3 = g3;
    out.g4 = g4;
    out.g5 = g5;
    out.g6 = vec4(0.0);
    return out;
}