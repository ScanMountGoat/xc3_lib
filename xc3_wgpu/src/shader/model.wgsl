// PerScene values.
struct Camera {
    view: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec4<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

// PerGroup values for ModelGroup.
struct PerGroup {
    // TODO: Should this be with the model?
    // TODO: rename to has skeleton?
    enable_skinning: vec4<u32>,
    // TODO: Is 256 the max bone count if index attributes use u8?
    // animated_bone_world * bone_world.inv()
    // i.e. animated_world * inverse_bind
    animated_transforms: array<mat4x4<f32>, 256>,
    animated_transforms_inv_transpose: array<mat4x4<f32>, 256>,
}

@group(1) @binding(0)
var<uniform> per_group: PerGroup;

@group(1) @binding(1)
var<storage> bone_indices: array<vec4<u32>>;

@group(1) @binding(2)
var<storage> skin_weights: array<vec4<f32>>;

// PerMaterial values.
// Define all possible parameters even if unused.
// The "ubershader" approach makes it possible to generate WGSL bindings at build time.
@group(2) @binding(0)
var s0: texture_2d<f32>;

@group(2) @binding(1)
var s1: texture_2d<f32>;

@group(2) @binding(2)
var s2: texture_2d<f32>;

@group(2) @binding(3)
var s3: texture_2d<f32>;

@group(2) @binding(4)
var s4: texture_2d<f32>;

@group(2) @binding(5)
var s5: texture_2d<f32>;

@group(2) @binding(6)
var s6: texture_2d<f32>;

@group(2) @binding(7)
var s7: texture_2d<f32>;

@group(2) @binding(8)
var s8: texture_2d<f32>;

@group(2) @binding(9)
var s9: texture_2d<f32>;

@group(2) @binding(10)
var s0_sampler: sampler;

@group(2) @binding(11)
var s1_sampler: sampler;

@group(2) @binding(12)
var s2_sampler: sampler;

@group(2) @binding(13)
var s3_sampler: sampler;

@group(2) @binding(14)
var s4_sampler: sampler;

@group(2) @binding(15)
var s5_sampler: sampler;

@group(2) @binding(16)
var s6_sampler: sampler;

@group(2) @binding(17)
var s7_sampler: sampler;

@group(2) @binding(18)
var s8_sampler: sampler;

@group(2) @binding(19)
var s9_sampler: sampler;

// TODO: How to handle multiple inputs for each output channel?
// Texture and channel input for each output channel.
struct OutputAssignment {
    sampler_indices: vec4<i32>,
    channel_indices: vec4<u32>
}

struct PerMaterial {
    mat_color: vec4<f32>,
    // TODO: Make a struct with named fields instead of arrays?
    output_assignments: array<OutputAssignment, 6>,
    // Parameters, constants, and defaults if no texture is assigned.
    gbuffer_defaults: array<vec4<f32>, 6>,
    texture_scale: array<vec4<f32>, 10>,
    // texture index, channel, index, 0, 0
    alpha_test_texture: vec4<i32>,
    alpha_test_ref: vec4<f32>,
    // Workaround for BC4 swizzle mask.
    is_single_channel: array<vec4<u32>, 10>,
}

@group(2) @binding(20)
var<uniform> per_material: PerMaterial;

// PerMesh values.
struct PerMesh {
    // start_index, 0, 0, 0
    weight_group_indices: vec4<u32>
}

@group(3) @binding(2)
var<uniform> per_mesh: PerMesh;

// Define all possible attributes even if unused.
// This avoids needing separate shaders.
struct VertexInput0 {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

// Store attributes unaffected by skinning or morphs separately.
struct VertexInput1 {
    @location(3) vertex_color: vec4<f32>,
    @location(4) tex0: vec3<f32>,
    @location(5) weight_index: u32,
}

struct InstanceInput {
    @location(7) model_matrix_0: vec4<f32>,
    @location(8) model_matrix_1: vec4<f32>,
    @location(9) model_matrix_2: vec4<f32>,
    @location(10) model_matrix_3: vec4<f32>,
}

// wgpu recommends @invariant for position with depth func equals.
struct VertexOutput {
    @builtin(position) @invariant clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) tex0: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) vertex_color: vec4<f32>,
}

struct FragmentOutput {
    @location(0) g_color: vec4<f32>,
    @location(1) g_etc_buffer: vec4<f32>,
    @location(2) g_normal: vec4<f32>,
    @location(3) g_velocity: vec4<f32>,
    @location(4) g_depth: vec4<f32>,
    @location(5) g_lgt_color: vec4<f32>,
}

fn vertex_output(in0: VertexInput0, in1: VertexInput1, instance: InstanceInput, outline: bool) -> VertexOutput {
    var out: VertexOutput;

    // Linear blend skinning.
    var position = in0.position.xyz;
    var normal_xyz = in0.normal.xyz;
    var tangent_xyz = in0.tangent.xyz;

    if per_group.enable_skinning.x == 1u {
        position = vec3(0.0);
        normal_xyz = vec3(0.0);
        tangent_xyz = vec3(0.0);

        // Weights require an extra layer of indirection.
        // This is done in game using a buffer of bone transforms with weights already applied.
        // The "nWgtIdx" selects a transform combining up to 4 bone transforms and the camera transform.
        // Compute the transforms here instead for simplicity.
        var weights_index = in1.weight_index + per_mesh.weight_group_indices.x;
        let bone_indices = bone_indices[weights_index];
        let skin_weights = skin_weights[weights_index];

        for (var i = 0u; i < 4u; i += 1u) {
            let bone_index = bone_indices[i];
            let skin_weight = skin_weights[i];

            position += skin_weight * (per_group.animated_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent_xyz += skin_weight * (per_group.animated_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            normal_xyz += skin_weight * (per_group.animated_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        }
    }

    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    if outline {
        // TODO: What to use for the param here?
        let outline_width = outline_width(in1.vertex_color, 0.007351, position.z, normal_xyz);
        position += normal_xyz * outline_width;
    }

    out.clip_position = camera.view_projection * model_matrix * vec4(position, 1.0);
    out.position = out.clip_position.xyz;

    // TODO: The zw components should contain a scaled version?
    // TODO: How to detect what the scale values should be?
    // TODO: How to detect whether a texture uses the scaled UVs or not?
    // TODO: Are the scaled UVs always in zw?
    // TODO: sometimes tex0.xy is multiplied by a matrix?
    // TODO: some shaders have gTexA, gTexB, gTexC for up to 5 scaled versions of tex0?
    // TODO: frag shader just accesses the attributes without modifying them?
    out.tex0 = in1.tex0.xy;

    // TODO: Find a better way to only force vertex color for outlines.
    if outline {
        out.vertex_color = in1.vertex_color;
    } else {
        out.vertex_color = vec4(1.0);
    }
    // Transform any direction vectors by the instance transform.
    // TODO: This assumes no scaling?
    out.normal = (model_matrix * vec4(normal_xyz, 0.0)).xyz;
    out.tangent = (model_matrix * vec4(tangent_xyz, 0.0)).xyz;
    out.bitangent = cross(out.normal, out.tangent) * in0.tangent.w;
    return out;
}

// Adapted from shd0001 GLSL from ch11021013.pcsmt (xc3). 
fn outline_width(vertex_color: vec4<f32>, param: f32, view_z: f32, normal: vec3<f32>) -> f32 {
    // TODO: Is this scaled to have a fixed width in screen space?
    // TODO: is the param always gWrkFl4[0].w?
    // TODO: Scaled by toon lighting using toon params?
    let f_line_width = vertex_color.w * param;
    return f_line_width;
}

@vertex
fn vs_main(in0: VertexInput0, in1: VertexInput1, instance: InstanceInput) -> VertexOutput {
    return vertex_output(in0, in1, instance, false);
}

@vertex
fn vs_outline_main(in0: VertexInput0, in1: VertexInput1, instance: InstanceInput) -> VertexOutput {
    return vertex_output(in0, in1, instance, true);
}

fn assign_texture(assignment: OutputAssignment, s_colors: array<vec4<f32>, 10>, default_value: vec4<f32>) -> vec4<f32> {
    let x = assign_channel(assignment.sampler_indices.x, assignment.channel_indices.x, s_colors, default_value.x);
    let y = assign_channel(assignment.sampler_indices.y, assignment.channel_indices.y, s_colors, default_value.y);
    let z = assign_channel(assignment.sampler_indices.z, assignment.channel_indices.z, s_colors, default_value.z);
    let w = assign_channel(assignment.sampler_indices.w, assignment.channel_indices.w, s_colors, default_value.w);
    return vec4(x, y, z, w);
}

fn assign_channel(sampler_index: i32, channel_index: u32, s_colors: array<vec4<f32>, 10>, default_value: f32) -> f32 {
    // Workaround for BC4 swizzle mask of RRR1.
    var channel = channel_index;
    if sampler_index >= 0 {
        if per_material.is_single_channel[sampler_index].x == 1u {
            channel = 0u;
        }
    }

    // TODO: Is there a way to avoid needing a switch?
    switch (sampler_index) {
        case 0: {
            return s_colors[0][channel];
        }
        case 1: {
            return s_colors[1][channel];
        }
        case 2: {
            return s_colors[2][channel];
        }
        case 3: {
            return s_colors[3][channel];
        }
        case 4: {
            return s_colors[4][channel];
        }
        case 5: {
            return s_colors[5][channel];
        }
        case 6: {
            return s_colors[6][channel];
        }
        case 7: {
            return s_colors[7][channel];
        }
        case 8: {
            return s_colors[8][channel];
        }
        case 9: {
            return s_colors[9][channel];
        }
        default: {
            return default_value;
        }
    }
}

// Adapted from shd00036 GLSL from ch11021013.pcsmt (xc3). 
fn apply_normal_map(normal: vec3<f32>, tangent: vec3<f32>, bitangent: vec3<f32>, normal_map: vec2<f32>) -> vec3<f32> {
    // Remap the tangent space normal map to the correct range.
    // The additional offset determines the "neutral" normal map value.
    let x = 2.0 * normal_map.x - 1.0 - (1.0 / 256.0);
    let y = 2.0 * normal_map.y - 1.0 - (1.0 / 256.0);

    // Calculate z based on the fact that x*x + y*y + z*z = 1.
    let z = sqrt(abs(1.0 - (x * x) + (y * y)));

    // Normal mapping is a change of basis using the TBN vectors.
    return normalize(tangent * x + bitangent * y + normal * z);
}

// Adapted from shd00036 GLSL from ch11021013.pcsmt (xc3). 
fn geometric_specular_aa(shininess: f32, normal: vec3<f32>) -> f32 {
    let sigma2 = 0.25;
    let kappa = 0.18;
    let roughness = 1.0 - shininess;
    let roughness2 = roughness * roughness;
    let dndu = dpdx(normal);
    let dndv = dpdy(normal);
    let variance = sigma2 * (dot(dndu, dndu) + dot(dndv, dndv));
    let kernelRoughness2 = min(2.0 * variance, kappa);
    let filteredRoughness2 = saturate(roughness2 + kernelRoughness2);
    return (1.0 - sqrt(filteredRoughness2));
}

// Adapted from shd00036 GLSL from ch11021013.pcsmt (xc3). 
// TODO: What is this conversion doing?
fn mrt_depth(depth: f32, param: f32) -> vec4<f32> {
    var o = vec2(depth * 8.0, floor(depth * 8.0) / 255.0);
    let t = floor(o);
    return vec4(o.xy - t.xy, t.y / 255.0, param);
}

// Adapted from shd00036 GLSL from ch11021013.pcsmt (xc3). 
// TODO: What is this conversion doing?
fn mrt_normal(normal: vec3<f32>, ao: f32) -> vec4<f32> {
    let temp = normal * vec3(0.5, 0.5, 1000.0) + vec3(0.5);
    return vec4(temp.xy, ao, temp.z);
}

// Adapted from shd00036 GLSL from ch11021013.pcsmt (xc3). 
fn mrt_etc_buffer(g_etc_buffer: vec4<f32>, view_normal: vec3<f32>) -> vec4<f32> {
    var out = g_etc_buffer;
    // Antialiasing isn't necessary for parameters or constants.
    if per_material.output_assignments[1].sampler_indices.y != -1 {
        out.y = geometric_specular_aa(g_etc_buffer.y, view_normal);
    }
    return out;
}

// TODO: schlick fresnel approximation for shading some toon meshes?
// TODO: Is it reliable to detect this as U_Mate.gWrkFl4[i].c * 5?
// TODO: what color is used for blending?

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let tangent = normalize(in.tangent);
    let vertex_normal = normalize(in.normal.xyz);
    let bitangent = normalize(in.bitangent);

    let s0_color = textureSample(s0, s0_sampler, in.tex0 * per_material.texture_scale[0].xy);
    let s1_color = textureSample(s1, s1_sampler, in.tex0 * per_material.texture_scale[1].xy);
    let s2_color = textureSample(s2, s2_sampler, in.tex0 * per_material.texture_scale[2].xy);
    let s3_color = textureSample(s3, s3_sampler, in.tex0 * per_material.texture_scale[3].xy);
    let s4_color = textureSample(s4, s4_sampler, in.tex0 * per_material.texture_scale[4].xy);
    let s5_color = textureSample(s5, s5_sampler, in.tex0 * per_material.texture_scale[5].xy);
    let s6_color = textureSample(s6, s6_sampler, in.tex0 * per_material.texture_scale[6].xy);
    let s7_color = textureSample(s7, s7_sampler, in.tex0 * per_material.texture_scale[7].xy);
    let s8_color = textureSample(s8, s8_sampler, in.tex0 * per_material.texture_scale[8].xy);
    let s9_color = textureSample(s9, s9_sampler, in.tex0 * per_material.texture_scale[9].xy);

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

    // An index of -1 disables alpha testing.
    let alpha_texture = per_material.alpha_test_texture.x;
    let alpha_texture_channel = u32(per_material.alpha_test_texture.y);
    // Workaround for not being able to use a non constant index.
    if assign_channel(alpha_texture, alpha_texture_channel, s_colors, 1.0) < per_material.alpha_test_ref.x {
        // TODO: incorrect reference alpha for comparison?
        discard;
    }

    // The layout of G-Buffer textures is fixed but assignments are not.
    // Each material in game can have a unique shader program.
    // Check the G-Buffer assignment database to simulate having unique shaders.
    // TODO: How to properly handle missing assignments?
    let assignments = per_material.output_assignments;
    // Defaults incorporate constants, parameters, and default values.
    let defaults = per_material.gbuffer_defaults;
    let g_color = assign_texture(assignments[0], s_colors, defaults[0]);
    let g_etc_buffer = assign_texture(assignments[1], s_colors, defaults[1]);
    let g_normal = assign_texture(assignments[2], s_colors, defaults[2]);
    let g_velocity = assign_texture(assignments[3], s_colors, defaults[3]);
    let g_depth = assign_texture(assignments[4], s_colors, defaults[4]);
    let g_lgt_color = assign_texture(assignments[5], s_colors, defaults[5]);

    // Assume each G-Buffer texture and channel always has the same usage.
    let normal_map = g_normal.xy;

    // Not all materials and shaders use normal mapping.
    // TODO: Is this a good way to check for this?
    var normal = vertex_normal;
    if assignments[2].sampler_indices.x != -1 && assignments[2].sampler_indices.y != -1 {
        normal = apply_normal_map(vertex_normal, tangent, bitangent, normal_map);
    }

    // In game normals in view space.
    let view_normal = normalize((camera.view * vec4(normal.xyz, 0.0)).xyz);

    // TODO: How to detect if vertex color is actually color?
    // TODO: Some outlines aren't using vertex color?

    // The ordering here is the order of per material fragment shader outputs.
    // The input order for the deferred lighting pass is slightly different.
    // TODO: alpha?
    // TODO: How much shading is done in this pass?
    // TODO: Is it ok to always apply gMatCol like this?
    var out: FragmentOutput;
    out.g_color = g_color * vec4(per_material.mat_color.rgb * in.vertex_color.rgb, 1.0);
    out.g_etc_buffer = mrt_etc_buffer(g_etc_buffer, view_normal);
    out.g_normal = mrt_normal(view_normal, g_normal.z);
    out.g_velocity = g_velocity;
    out.g_depth = mrt_depth(in.position.z, 0.0);
    out.g_lgt_color = g_lgt_color;
    return out;
}