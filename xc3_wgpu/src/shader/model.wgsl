// PerScene values.
struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec4<f32>,
    resolution: vec2<f32>
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
// TODO: Better way to store multiple layers?
// TODO: More than 3 layers?
// TODO: store texture channel or float for layer weights
struct SamplerAssignment {
    sampler_indices: vec4<i32>,
    channel_indices: vec4<u32>,
}

// TODO: Support attributes other than vColor.
// Attribute and channel input for each output channel.
struct AttributeAssignment {
    // TODO: proper attribute selection similar to textures?
    channel_indices: vec4<i32>
}

struct OutputAssignment {
    samplers1: SamplerAssignment,
    // wimdo and wismhd models use up to 4 additional layers.
    samplers2: SamplerAssignment,
    samplers3: SamplerAssignment,
    samplers4: SamplerAssignment,
    samplers5: SamplerAssignment,
    layer2: TextureLayers,
    layer3: TextureLayers,
    layer4: TextureLayers,
    layer5: TextureLayers,
    attributes1: AttributeAssignment,
    attributes2: AttributeAssignment,
    attributes3: AttributeAssignment,
    attributes4: AttributeAssignment,
    attributes5: AttributeAssignment,
    default_value: vec4<f32>
}

struct TextureLayers {
    // Layer blending for xyzw channels.
    sampler_indices: vec4<i32>,
    channel_indices: vec4<u32>,
    default_weights: vec4<f32>,
    value: vec4<f32>,
    blend_mode: vec4<i32>,
    is_fresnel: vec4<u32>,

}

struct PerMaterial {
    mat_color: vec4<f32>,

    // Shader database information.
    assignments: array<OutputAssignment, 6>,

    // TODO: Split into separate fields since this uses encase.
    // texture index, channel, index, 0, 0
    alpha_test_texture: vec4<i32>,

    // Store one transform per texture.
    // Assume shaders access textures at most once.
    texture_transforms: array<array<vec4<f32>, 2>, 10>,
    // texcoord index, single channel (BC4 swizzle mask), 0, 0
    texture_info: array<vec4<u32>, 10>,

    fur_params: FurShellParams,

    // Assume outline width is always set via a parameter or constant.
    outline_width: f32
}

struct FurShellParams {
    xyz_offset: vec3<f32>,
    instance_count: f32,
    shell_width: f32,
    alpha: f32
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

// TODO: Avoid storing skin weights per mesh?
@group(3) @binding(3)
var<storage> bone_indices: array<vec4<u32>>;

@group(3) @binding(4)
var<storage> skin_weights: array<vec4<f32>>;

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
    @location(4) weight_index: vec4<u32>,
    @location(5) tex01: vec4<f32>,
    @location(6) tex23: vec4<f32>,
    @location(7) tex45: vec4<f32>,
    @location(8) tex67: vec4<f32>,
    @location(9) tex8: vec4<f32>,
}

struct InstanceInput {
    @location(10) model_matrix_0: vec4<f32>,
    @location(11) model_matrix_1: vec4<f32>,
    @location(12) model_matrix_2: vec4<f32>,
    @location(13) model_matrix_3: vec4<f32>,
}

// wgpu recommends @invariant for position with depth func equals.
struct VertexOutput {
    @builtin(position) @invariant clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) vertex_color: vec4<f32>,
    @location(4) tex01: vec4<f32>,
    @location(5) tex23: vec4<f32>,
    @location(6) tex45: vec4<f32>,
    @location(7) tex67: vec4<f32>,
    @location(8) tex8: vec4<f32>,
}

struct FragmentOutput {
    @location(0) g_color: vec4<f32>,
    @location(1) g_etc_buffer: vec4<f32>,
    @location(2) g_normal: vec4<f32>,
    @location(3) g_velocity: vec4<f32>,
    @location(4) g_depth: vec4<f32>,
    @location(5) g_lgt_color: vec4<f32>,
}

fn vertex_output(in0: VertexInput0, in1: VertexInput1, instance_index: u32, outline: bool) -> VertexOutput {
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
        let bone_indices = bone_indices[weights_index.x];
        let skin_weights = skin_weights[weights_index.x];

        for (var i = 0u; i < 4u; i += 1u) {
            let bone_index = bone_indices[i];
            let skin_weight = skin_weights[i];

            position += skin_weight * (per_group.animated_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent_xyz += skin_weight * (per_group.animated_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            normal_xyz += skin_weight * (per_group.animated_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        }
    }

    // Transform any direction vectors by the camera transforms.
    // TODO: This assumes no scaling?
    position = (camera.view * vec4(position, 1.0)).xyz;
    normal_xyz = (camera.view * vec4(normal_xyz, 0.0)).xyz;
    tangent_xyz = (camera.view * vec4(tangent_xyz, 0.0)).xyz;

    var vertex_color = in1.vertex_color;

    if outline {
        // TODO: This is applied to work values in game?
        // TODO: Multiply by some other constant?
        let param = 2.0 * per_material.outline_width / camera.resolution.y;

        let outline_width = outline_width(in1.vertex_color, param, position.z, normal_xyz);
        position += normal_xyz * outline_width * 8.0;
        // TODO: set vertex alpha to line width?
        // vertex_color.a = outline_width;
    }

    if per_material.fur_params.instance_count > 0.0 {
        let instance = f32(instance_index) + 1.0;
        let fur_shell_width = instance * per_material.fur_params.shell_width;
        position += normal_xyz * fur_shell_width;

        // This is only a vertical offset in practice.
        let param = instance * (1.0 / per_material.fur_params.instance_count);
        let xyz_offset = (param * param * param) * per_material.fur_params.xyz_offset;

        position += xyz_offset;

        // Outer shells are more transparent than inner shells.
        let alpha_factor = f32(instance_index) * per_material.fur_params.alpha;
        vertex_color.a = 1.0 - clamp(alpha_factor, 0.0, 1.0);
    }

    out.clip_position = camera.projection * vec4(position, 1.0);
    out.position = out.clip_position.xyz;

    // Some shaders have gTexA, gTexB, gTexC for up to 5 scaled versions of tex0.
    // This is handled in the fragment shader, so just return the attributes.
    out.tex01 = in1.tex01;
    out.tex23 = in1.tex23;
    out.tex45 = in1.tex45;
    out.tex67 = in1.tex67;
    out.tex8 = in1.tex8;

    out.vertex_color = vertex_color;

    out.normal = normal_xyz;
    out.tangent = vec4(tangent_xyz, in0.tangent.w);
    return out;
}

// Adapted from shd0001 GLSL from ch11021013.pcsmt (xc3). 
fn outline_width(vertex_color: vec4<f32>, param: f32, view_z: f32, normal: vec3<f32>) -> f32 {
    let f_line_width = vertex_color.w * param * -view_z / camera.projection[1][1];
    // TODO: Scaled by toon lighting using toon params?
    return f_line_width;
}

// TODO: separate shader for instancing stage models?
@vertex
fn vs_main(in0: VertexInput0, in1: VertexInput1, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    return vertex_output(in0, in1, instance_index, false);
}

@vertex
fn vs_outline_main(in0: VertexInput0, in1: VertexInput1, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    return vertex_output(in0, in1, instance_index, true);
}

@vertex
fn vs_main_instanced_static(in0: VertexInput0, in1: VertexInput1, instance: InstanceInput) -> VertexOutput {
    // Simplified vertex shader for static stage meshes
    var out: VertexOutput;

    let instance_transform = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    // Transform any direction vectors by the instance and camera transforms.
    // TODO: This assumes no scaling?
    let model_view_matrix = camera.view * instance_transform;
    let position = (model_view_matrix * vec4(in0.position.xyz, 1.0)).xyz;
    let normal_xyz = (model_view_matrix * vec4(in0.normal.xyz, 0.0)).xyz;
    let tangent_xyz = (model_view_matrix * vec4(in0.tangent.xyz, 0.0)).xyz;

    out.clip_position = camera.projection * vec4(position, 1.0);
    out.position = out.clip_position.xyz;

    // Some shaders have gTexA, gTexB, gTexC for up to 5 scaled versions of tex0.
    // This is handled in the fragment shader, so just return the attributes.
    out.tex01 = in1.tex01;
    out.tex23 = in1.tex23;
    out.tex45 = in1.tex45;
    out.tex67 = in1.tex67;
    out.tex8 = in1.tex8;
    out.vertex_color = in1.vertex_color;
    out.normal = normal_xyz;
    out.tangent = vec4(tangent_xyz, in0.tangent.w);
    return out;
}

fn assign_texture(a: OutputAssignment, s_colors: array<vec4<f32>, 10>, vcolor: vec4<f32>) -> vec4<f32> {
    let x = assign_channel(a.samplers1.sampler_indices.x, a.samplers1.channel_indices.x, a.attributes1.channel_indices.x, s_colors, vcolor, a.default_value.x);
    let y = assign_channel(a.samplers1.sampler_indices.y, a.samplers1.channel_indices.y, a.attributes1.channel_indices.y, s_colors, vcolor, a.default_value.y);
    let z = assign_channel(a.samplers1.sampler_indices.z, a.samplers1.channel_indices.z, a.attributes1.channel_indices.z, s_colors, vcolor, a.default_value.z);
    let w = assign_channel(a.samplers1.sampler_indices.w, a.samplers1.channel_indices.w, a.attributes1.channel_indices.w, s_colors, vcolor, a.default_value.w);
    return vec4(x, y, z, w);
}

fn assign_texture_layer(o: OutputAssignment, layer_index: u32, s_colors: array<vec4<f32>, 10>, vcolor: vec4<f32>, default_value: vec4<f32>) -> vec4<f32> {
    var s = o.samplers1;
    var a = o.attributes1;
    switch (layer_index) {
        case 0u: {
            s = o.samplers2;
            a = o.attributes2;
        }
        case 1u: {
            s = o.samplers3;
            a = o.attributes3;
        }
        case 2u: {
            s = o.samplers4;
            a = o.attributes4;
        }
        case 3u: {
            s = o.samplers5;
            a = o.attributes5;
        }
        default: {
            s = o.samplers1;
            a = o.attributes1;
        }
    }

    let x = assign_channel(s.sampler_indices.x, s.channel_indices.x, a.channel_indices.x, s_colors, vcolor, default_value.x);
    let y = assign_channel(s.sampler_indices.y, s.channel_indices.y, a.channel_indices.y, s_colors, vcolor, default_value.y);
    let z = assign_channel(s.sampler_indices.z, s.channel_indices.z, a.channel_indices.z, s_colors, vcolor, default_value.z);
    let w = assign_channel(s.sampler_indices.w, s.channel_indices.w, a.channel_indices.w, s_colors, vcolor, default_value.w);
    return vec4(x, y, z, w);
}

fn assign_channel(sampler_index: i32, channel_index: u32, attribute_channel_index: i32, s_colors: array<vec4<f32>, 10>, vcolor: vec4<f32>, default_value: f32) -> f32 {
    if attribute_channel_index >= 0 {
        return vcolor[attribute_channel_index];
    }
    
    // Workaround for BC4 swizzle mask of RRR1.
    var channel = channel_index;
    if sampler_index >= 0 {
        if per_material.texture_info[sampler_index].y == 1u {
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

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, getCalcNormalMap,
fn apply_normal_map(normal_map: vec3<f32>, tangent: vec3<f32>, bitangent: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Normal mapping is a change of basis using the TBN vectors.
    let x = normal_map.x;
    let y = normal_map.y;
    let z = normal_map.z;
    return normalize(tangent * x + bitangent * y + normal * z);
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, createNormalMapTex_B5XY.
fn create_normal_map(col: vec2<f32>) -> vec3<f32> {
    // Remap the tangent space normal map to the correct range.
    // The additional offset determines the "neutral" normal map value.
    let x = 2.0 * col.x - 1.0 - (1.0 / 256.0);
    let y = 2.0 * col.y - 1.0 - (1.0 / 256.0);

    let z = normal_z(x, y);

    return vec3(x, y, z);
}

fn normal_z(x: f32, y: f32) -> f32 {
    // Calculate z based on the fact that x*x + y*y + z*z = 1.
    return sqrt(abs(1.0 - (x * x) + (y * y)));
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, getPixelCalcAddNormal.
// This appears to match "Reoriented Normal Mapping (RNM)" described here:
// https://blog.selfshadow.com/publications/blending-in-detail/
fn add_normal_maps(n1: vec3<f32>, n2: vec3<f32>, ratio: f32) -> vec3<f32> {
    let t = n1.xyz + vec3(0.0, 0.0, 1.0);
    let u = n2.xyz * vec3(-1.0, -1.0, 1.0);
    let r = t * dot(t, u) - u * t.z;
    return normalize(mix(n1, normalize(r), ratio));
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, calcGeometricSpecularAA.
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

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, setMrtDepth,
// TODO: What is this conversion doing?
fn mrt_depth(depth: f32, param: f32) -> vec4<f32> {
    var o = vec2(depth * 8.0, floor(depth * 8.0) / 255.0);
    let t = floor(o);
    return vec4(o.xy - t.xy, t.y / 255.0, param);
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, setMrtNormal,
// TODO: What is this conversion doing?
fn mrt_normal(normal: vec3<f32>, ao: f32) -> vec4<f32> {
    let temp = normal * vec3(0.5, 0.5, 1000.0) + vec3(0.5);
    return vec4(temp.xy, ao, temp.z);
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00036, setMrtEtcBuffer,
fn mrt_etc_buffer(g_etc_buffer: vec4<f32>, view_normal: vec3<f32>) -> vec4<f32> {
    var out = g_etc_buffer;
    // Antialiasing isn't necessary for parameters or constants.
    if per_material.assignments[1].samplers1.sampler_indices.y != -1 {
        out.y = geometric_specular_aa(g_etc_buffer.y, view_normal);
    }
    return out;
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, getTextureMatrix.
// Scale parameters are converted to "matrices" for consistency.
fn transform_uv(uv: vec2<f32>, matrix: array<vec4<f32>, 2>) -> vec2<f32> {
    let v = vec4(uv, 0.0, 1.0);
    return vec2(dot(v, matrix[0]), dot(v, matrix[1]));
}

fn select_uv(vert: VertexOutput, index: u32) -> vec2<f32> {
    var uvs = vert.tex01.xy;
    switch (per_material.texture_info[index].x) {
        case 0u: {
            uvs = vert.tex01.xy;
        }
        case 1u: {
            uvs = vert.tex01.zw;
        }
        case 2u: {
            uvs = vert.tex23.xy;
        }
        case 3u: {
            uvs = vert.tex23.zw;
        }
        case 4u: {
            uvs = vert.tex45.xy;
        }
        case 5u: {
            uvs = vert.tex45.zw;
        }
        case 6u: {
            uvs = vert.tex67.xy;
        }
        case 7u: {
            uvs = vert.tex67.zw;
        }
        case 8u: {
            uvs = vert.tex8.xy;
        }
        default: {
        }
    }
    return transform_uv(uvs, per_material.texture_transforms[index]);
}

fn overlay_blend(a: f32, b: f32) -> f32 {
    // Trick to avoid a conditional branch from xenox/chr_fc/fc281011.camdo.
    let is_a_gt_half = clamp((a - 0.5) * 1000.0, 0.0, 1.0);
    let screen = 1.0 - 2.0 * (1.0 - a) * (1.0 - b);
    let multiply = 2.0 * a * b;
    return screen * is_a_gt_half + multiply * (1.0 - is_a_gt_half);
}

fn blend_layer(a: vec4<f32>, b: vec4<f32>, ratio: vec4<f32>, n_dot_v: f32, mode: vec4<i32>) -> vec4<f32> {
    // Color layers typically have the same blend mode for all channels.
    // Blend each channel individually to properly blend params.
    var result = a;
    for (var i = 0u; i < 4u; i++) {
        switch (mode[i]) {
            case 0: {
                result[i] = mix(a[i], b[i], ratio[i]);
            }
            case 1: {
                result[i] = mix(a[i], a[i] * b[i], ratio[i]);
            }
            case 2: {
                result[i] = a[i] + b[i] * ratio[i];
            }
            case 3: {
                // Ensure that z blending does not affect normals.
                let a_normal = vec3(a.xy, normal_z(a.x, a.y));
                let b_normal = create_normal_map(b.xy);
                // Assume this mode applies to all relevant channels.
                let blended = add_normal_maps(a_normal, b_normal, ratio[i]);
                result.x = blended.x;
                result.y = blended.y;
            }
            case 4: {
                result[i] = mix(a[i], overlay_blend(a[i], b[i]), ratio[i]);
            }
            default: {
                result[i] = a[i];
            }
        }
    }
    return result;
}

fn blend_texture_layer(current: vec4<f32>, assignments: OutputAssignment, s_colors: array<vec4<f32>, 10>, vcolor: vec4<f32>, layer_index: u32, n_dot_v: f32) -> vec4<f32> {
    var layers: TextureLayers;
    var attribute_channel_indices = vec4(-1);
    switch (layer_index) {
        case 0u: {
            layers = assignments.layer2;
        }
        case 1u: {
            layers = assignments.layer3;
        }
        case 2u: {
            layers = assignments.layer4;
        }
        case 3u: {
            layers = assignments.layer5;
        }
        default: {
        }
    }

    var ratio = vec4(0.0);
    for (var i = 0; i < 4; i++) {
        let blend_mode = layers.blend_mode[i];
        let sampler_index = layers.sampler_indices[i];
        let channel_index = layers.channel_indices[i];
        var default_weight = layers.default_weights[i];
        let is_fresnel = layers.is_fresnel[i] != 0u;

        if blend_mode != -1 {
            ratio[i] = assign_channel(sampler_index, channel_index, -1, s_colors, vec4(0.0), default_weight);
            if is_fresnel {
                // Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00016, getPixelCalcFresnel.
                ratio[i] = pow(1.0 - n_dot_v, ratio[i] * 5.0);
            }
        }
    }
    let b = assign_texture_layer(assignments, layer_index, s_colors, vcolor, layers.value);

    return blend_layer(current, b, ratio, n_dot_v, layers.blend_mode);
}

fn fragment_output(in: VertexOutput) -> FragmentOutput {
    let tangent = normalize(in.tangent.xyz);
    let vertex_normal = normalize(in.normal.xyz);

    let bitangent = cross(vertex_normal, tangent) * in.tangent.w;

    let s0_color = textureSample(s0, s0_sampler, select_uv(in, 0u));
    let s1_color = textureSample(s1, s1_sampler, select_uv(in, 1u));
    let s2_color = textureSample(s2, s2_sampler, select_uv(in, 2u));
    let s3_color = textureSample(s3, s3_sampler, select_uv(in, 3u));
    let s4_color = textureSample(s4, s4_sampler, select_uv(in, 4u));
    let s5_color = textureSample(s5, s5_sampler, select_uv(in, 5u));
    let s6_color = textureSample(s6, s6_sampler, select_uv(in, 6u));
    let s7_color = textureSample(s7, s7_sampler, select_uv(in, 7u));
    let s8_color = textureSample(s8, s8_sampler, select_uv(in, 8u));
    let s9_color = textureSample(s9, s9_sampler, select_uv(in, 9u));

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
    if assign_channel(alpha_texture, alpha_texture_channel, -1, s_colors, vec4(1.0), 1.0) < 0.5 {
        // TODO: incorrect reference alpha for comparison?
        discard;
    }

    // The layout of G-Buffer textures is fixed but assignments are not.
    // Each material in game can have a unique shader program.
    // Check the G-Buffer assignment database to simulate having unique shaders.
    // TODO: How to properly handle missing assignments?
    let assignments = per_material.assignments;

    // Defaults incorporate constants, parameters, and default values.
    // Assume each G-Buffer texture and channel always has the same usage.
    let g_color = assign_texture(assignments[0], s_colors, in.vertex_color);
    let g_etc_buffer = assign_texture(assignments[1], s_colors, in.vertex_color);
    let g_normal = assign_texture(assignments[2], s_colors, in.vertex_color);
    let g_velocity = assign_texture(assignments[3], s_colors, in.vertex_color);
    let g_depth = assign_texture(assignments[4], s_colors, in.vertex_color);
    let g_lgt_color = assign_texture(assignments[5], s_colors, in.vertex_color);


    // Not all materials and shaders use normal mapping.
    // TODO: Is this a good way to check for this?
    var normal = vertex_normal;
    var ao = g_normal.z;
    if assignments[2].samplers1.sampler_indices.x != -1 && assignments[2].samplers1.sampler_indices.y != -1 {

        // Normal layers never use fresnel blending, so just use a default for dot(N, V).
        // This avoids needing to define N before layering normal maps.
        var normal_map = create_normal_map(g_normal.xy);
        for (var i = 0u; i < 4u; i++) {
            normal_map = blend_texture_layer(vec4(normal_map, 0.0), assignments[2], s_colors, in.vertex_color, i, 1.0).xyz;
            // Ensure that z blending does not affect normals.
            ao = normal_map.z;
            normal_map.z = normal_z(normal_map.x, normal_map.y);
        }

        normal = apply_normal_map(normal_map, tangent, bitangent, vertex_normal);
    }

    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);
    let n_dot_v = max(dot(view, normal), 0.0);

    // Blend color layers.
    var color = g_color.rgb;
    for (var i = 0u; i < 4u; i++) {
        color = blend_texture_layer(vec4(color, 1.0), assignments[0], s_colors, in.vertex_color, i, n_dot_v).rgb;
    }

    // Blend parameter layers.
    var etc_buffer = g_etc_buffer;
    for (var i = 0u; i < 4u; i++) {
        etc_buffer = blend_texture_layer(etc_buffer, assignments[1], s_colors, in.vertex_color, i, n_dot_v);
    }

    // TODO: How to detect if vertex color is actually color?

    // The ordering here is the order of per material fragment shader outputs.
    // The input order for the deferred lighting pass is slightly different.
    // TODO: alpha?
    // TODO: Is it ok to always apply gMatCol like this?
    // TODO: Detect multiply by vertex color and gMatCol.
    // TODO: Just detect if gMatCol is part of the technique parameters?
    var out: FragmentOutput;
    out.g_color = vec4(color, g_color.a * in.vertex_color.a) * per_material.mat_color;
    out.g_etc_buffer = mrt_etc_buffer(etc_buffer, normal);
    out.g_normal = mrt_normal(normal, ao);
    out.g_velocity = g_velocity;
    out.g_depth = mrt_depth(in.position.z, g_depth.w);
    out.g_lgt_color = g_lgt_color;
    return out;
}

@fragment
fn fs_alpha(in: VertexOutput) -> @location(0) vec4<f32> {
    let output = fragment_output(in);
    return output.g_color;
}

// TODO: Separate entry for depth prepass.
// TODO: depth func needs to be changed if using prepass?

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    return fragment_output(in);
}

@fragment
fn fs_outline(in: VertexOutput) -> FragmentOutput {
    if in.vertex_color.a <= 0.0 {
        discard;
    }

    // TODO: Detect multiply by vertex color and gMatCol.
    var output = fragment_output(in);
    output.g_color = vec4(in.vertex_color.rgb * per_material.mat_color.rgb, 0.0);
    return output;
}