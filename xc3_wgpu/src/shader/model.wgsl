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

// TODO: Multiple samplers?
@group(1) @binding(6)
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
    sampler_indices: vec4<u32>,
    channel_indices: vec4<u32>
}

@group(2) @binding(0)
var<uniform> gbuffer_assignments: array<GBufferAssignment, 3>;

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

fn assign_gbuffer_texture(assignment: GBufferAssignment, s_colors: array<vec4<f32>, 6>) -> vec4<f32> {
    let x = assign_channel(assignment.sampler_indices.x, assignment.channel_indices.x, s_colors);
    let y = assign_channel(assignment.sampler_indices.y, assignment.channel_indices.y, s_colors);
    let z = assign_channel(assignment.sampler_indices.z, assignment.channel_indices.z, s_colors);
    let w = assign_channel(assignment.sampler_indices.w, assignment.channel_indices.w, s_colors);
    return vec4(x, y, z, w);
}

fn assign_channel(sampler_index: u32, channel_index: u32, s_colors: array<vec4<f32>, 6>) -> f32 {
    // TODO: Is there a way to avoid needing a switch?
    switch (sampler_index) {
        case 0u: {
            return s_colors[0][channel_index];
        }
        case 1u: {
            return s_colors[1][channel_index];
        }
        case 2u: {
            return s_colors[2][channel_index];
        }
        case 3u: {
            return s_colors[3][channel_index];
        }
        case 4u: {
            return s_colors[4][channel_index];
        }
        case 5u: {
            return s_colors[5][channel_index];
        }
        default: {
            // TODO: Customize default values?
            return 0.0;
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

    let s_colors = array<vec4<f32>, 6>(s0_color, s1_color, s2_color, s3_color, s4_color, s5_color);

    // The layout of G-Buffer textures is fixed but assignments are not.
    // Each material in game can have a unique shader program.
    // The ordering here is the order of per material fragment shader outputs.
    // The input order for the deferred lighting pass is slightly different.
    let g0 = assign_gbuffer_texture(gbuffer_assignments[0], s_colors);
    let g1 = assign_gbuffer_texture(gbuffer_assignments[1], s_colors);
    let g2 = assign_gbuffer_texture(gbuffer_assignments[2], s_colors);

    // TODO: proper sRGB gamma conversion.
    // Each G-Buffer texture and channel always has the same usage.
    let albedo = pow(g0.xyz, vec3(2.2));
    let metalness = g1.x;
    let normal_map = g2.xy;
    let ambient_occlusion = g2.z;

    // TODO: Normalize vertex normals?
    let normal = apply_normal_map(normalize(in.normal.xyz), tangent, bitangent, normal_map);

    // Basic lambertion diffuse and phong specular for testing purposes.
    let diffuse_lighting = dot(view, normal) * 0.5 + 0.5;
    let reflection = reflect(-view, normal);
    let specular_lighting = pow(max(dot(view, reflection), 0.0), 8.0);

    // TODO: Proper metalness.
    var diffuse = albedo * ambient_occlusion * diffuse_lighting * (1.0 - metalness * 0.5);
    var specular = specular_lighting * mix(vec3(0.25), albedo, metalness);
    var color = diffuse + specular;

    // TODO: alpha?
    return vec4(color, 1.0);
}
