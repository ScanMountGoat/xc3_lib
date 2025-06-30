// PerScene values.
@group(0) @binding(0)
var<uniform> camera: super::camera::Camera;

// PerGroup values for ModelGroup.
struct PerGroup {
    // TODO: Should this be with the model?
    // TODO: rename to has skeleton?
    enable_skinning: vec4<u32>,
}

@group(1) @binding(0)
var<uniform> per_group: PerGroup;

@group(1) @binding(1)
var<storage> animated_transforms: array<mat4x4<f32>>;

@group(1) @binding(2)
var<storage> animated_transforms_inv_transpose: array<mat4x4<f32>>;

// Remap 8-bit vertex skinning bone index -> skeleton bone index.
// This is necessary for models that have more than 256 bones in the skeleton.
@group(1) @binding(3)
var<storage> bone_indices_remap: array<u32>;

const TEXTURE_SAMPLER_COUNT: u32 = 32;

// PerMaterial values.
// Define all possible parameters even if unused.
// The "ubershader" approach makes it possible to generate WGSL bindings at build time.
@group(2) @binding(0)
var textures: binding_array<texture_2d<f32>, TEXTURE_SAMPLER_COUNT>;

@group(2) @binding(1)
var textures_d3: binding_array<texture_3d<f32>, TEXTURE_SAMPLER_COUNT>;

@group(2) @binding(2)
var textures_cube: binding_array<texture_cube<f32>, TEXTURE_SAMPLER_COUNT>;

@group(2) @binding(3)
var samplers: binding_array<sampler, TEXTURE_SAMPLER_COUNT>;

// TODO: move this to a separate pass?
@group(2) @binding(4)
var alpha_test_sampler: sampler;

struct OutputAssignment {
    has_channels: vec4<u32>,
    default_value: vec4<f32>
}

struct PerMaterial {
    // Shader database information.
    assignments: array<OutputAssignment, 6>,

    fur_params: FurShellParams,

    // Assume outline width is always set via a parameter or constant.
    outline_width: f32,

    alpha_test_ref: f32
}

struct FurShellParams {
    xyz_offset: vec3<f32>,
    instance_count: f32,
    shell_width: f32,
    alpha: f32
}

@group(2) @binding(5)
var<storage, read> per_material: PerMaterial;

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
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
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
            let skinning_bone_index = bone_indices[i];
            let bone_index = bone_indices_remap[skinning_bone_index];
            let skin_weight = skin_weights[i];

            position += skin_weight * (animated_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent_xyz += skin_weight * (animated_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            normal_xyz += skin_weight * (animated_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        }
    }

    // Transform any direction vectors by the camera transforms.
    // TODO: This assumes no scaling?
    position = (camera.view * vec4(position, 1.0)).xyz;
    normal_xyz = (camera.view * vec4(normalize(normal_xyz), 0.0)).xyz;
    tangent_xyz = (camera.view * vec4(normalize(tangent_xyz), 0.0)).xyz;

    var vertex_color = in1.vertex_color;

    if outline {
        // TODO: This is applied to work values in game?
        // TODO: Multiply by some other constant?
        let param = 2.0 * per_material.outline_width / camera.resolution.y;

        let outline_width = outline_width(in1.vertex_color, param, position.z, normal_xyz);
        position += normal_xyz * outline_width * 2.0;
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
    out.position = vec4(position, 1.0);

    // Some shaders have gTexA, gTexB, gTexC for up to 5 scaled versions of tex0.
    // This is handled in the fragment shader, so just return the attributes.
    out.tex01 = in1.tex01;
    out.tex23 = in1.tex23;
    out.tex45 = in1.tex45;
    out.tex67 = in1.tex67;
    out.tex8 = in1.tex8;

    out.vertex_color = vertex_color;

    out.normal = vec4(normal_xyz, in0.normal.w);
    out.tangent = vec4(tangent_xyz, in0.tangent.w);
    return out;
}

// Adapted from shd0001 GLSL from ch11021013.pcsmt (xc3). 
fn outline_width(vertex_color: vec4<f32>, param: f32, view_z: f32, normal: vec3<f32>) -> f32 {
    let f_line_width = vertex_color.w * param * -view_z / camera.projection[1][1];
    // TODO: Scaled by toon lighting using toon params?
    return f_line_width;
}

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
    out.position = vec4(position, 1.0);

    // Some shaders have gTexA, gTexB, gTexC for up to 5 scaled versions of tex0.
    // This is handled in the fragment shader, so just return the attributes.
    out.tex01 = in1.tex01;
    out.tex23 = in1.tex23;
    out.tex45 = in1.tex45;
    out.tex67 = in1.tex67;
    out.tex8 = in1.tex8;
    out.vertex_color = in1.vertex_color;
    out.normal = vec4(normal_xyz, in0.normal.w);
    out.tangent = vec4(tangent_xyz, in0.tangent.w);
    return out;
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, getCalcNormalMap.
fn apply_normal_map(normal_map: vec3<f32>, tangent: vec3<f32>, bitangent: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Normal mapping is a change of basis using the TBN vectors.
    let x = normal_map.x;
    let y = normal_map.y;
    let z = normal_map.z;
    return normalize(tangent * x + bitangent * y + normal * z);
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, createNormalMapTex_B5XY.
fn create_normal_map(nx: f32, ny: f32) -> vec3<f32> {
    // Remap the tangent space normal map to the correct range.
    // The additional offset determines the "neutral" normal map value.
    let x = 2.0 * nx - 1.0 - (1.0 / 256.0);
    let y = 2.0 * ny - 1.0 - (1.0 / 256.0);
    return vec3(x, y, normal_z(x, y));
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
    if per_material.assignments[1].has_channels.y != 0u {
        out.y = geometric_specular_aa(g_etc_buffer.y, view_normal);
    }
    return out;
}

// Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00028, getTextureMatrix.
// Scale parameters are converted to "matrices" for consistency.
fn transform_uv(uv: vec2<f32>, transform_u: vec4<f32>, transform_v: vec4<f32>) -> vec2<f32> {
    let v = vec4(uv, 0.0, 1.0);
    return vec2(dot(v, transform_u), dot(v, transform_v));
}

// TODO: This is slightly different for both Xenoblade X DE variants?
fn uv_parallax(vert: VertexOutput, ratio: f32) -> vec2<f32> {
    // TODO: How similar is this to traditional parallax mapping with a height map?
    let bitangent = cross(vert.normal.xyz, vert.tangent.xyz) * vert.tangent.w;
    let offset = vert.normal.x * vert.tangent.xy - vert.normal.x * bitangent.xy;

    return ratio * 0.7 * offset;
}

fn overlay_blend(a: f32, b: f32) -> f32 {
    // Trick to avoid a conditional branch from xenox/chr_fc/fc281011.camdo.
    // This is also used for Xenoblade X DE.
    let is_a_gt_half = clamp((a - 0.5) * 1000.0, 0.0, 1.0);
    let screen = 1.0 - 2.0 * (1.0 - a) * (1.0 - b);
    let multiply = 2.0 * a * b;
    return screen * is_a_gt_half + multiply * (1.0 - is_a_gt_half);
}

fn overlay_blend_xyz(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return vec3(overlay_blend(a.x, b.x), overlay_blend(a.y, b.y), overlay_blend(a.z, b.z));
}

fn overlay_blend2(a: f32, b: f32) -> f32 {
    // An overlay variant for xeno1/model/obj/oj110006.wimdo.
    // This is used for normals and other values.
    let ratio = clamp(pow(b, 4.0), 0.0, 1.0);
    let screen = 1.0 - 2.0 * (1.0 - a) * (1.0 - b);
    let multiply = 2.0 * a * b;
    return screen * ratio + multiply * (1.0 - ratio);
}

fn overlay_blend2_xyz(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return vec3(overlay_blend2(a.x, b.x), overlay_blend2(a.y, b.y), overlay_blend2(a.z, b.z));
}

fn fresnel_ratio(ratio: f32, n_dot_v: f32) -> f32 {
    // Adapted from xeno3/chr/ch/ch11021013.pcsmt, shd00016, getPixelCalcFresnel.
    return pow(1.0 - n_dot_v, ratio * 5.0);
}

fn fresnel_ratio_xyz(ratio: vec3<f32>, n_dot_v: f32) -> vec3<f32> {
    return vec3(fresnel_ratio(ratio.x, n_dot_v), fresnel_ratio(ratio.y, n_dot_v), fresnel_ratio(ratio.z, n_dot_v));
}

fn monochrome(x: f32, y: f32, z: f32, ratio: f32) -> vec3<f32> {
    let color = vec3(x, y, z);
    return mix(color, vec3(dot(color, vec3(0.01))), ratio);
}

fn monochrome_xyz_x(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>, ratio: vec3<f32>) -> vec3<f32> {
    return vec3(monochrome(x.x, y.x, z.x, ratio.x).x, monochrome(x.y, y.y, z.y, ratio.y).x, monochrome(x.z, y.z, z.z, ratio.z).x);
}

fn monochrome_xyz_y(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>, ratio: vec3<f32>) -> vec3<f32> {
    return vec3(monochrome(x.x, y.x, z.x, ratio.x).y, monochrome(x.y, y.y, z.y, ratio.y).y, monochrome(x.z, y.z, z.z, ratio.z).y);
}

fn monochrome_xyz_z(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>, ratio: vec3<f32>) -> vec3<f32> {
    return vec3(monochrome(x.x, y.x, z.x, ratio.x).z, monochrome(x.y, y.y, z.y, ratio.y).z, monochrome(x.z, y.z, z.z, ratio.z).z);
}

fn fragment_output(in: VertexOutput) -> FragmentOutput {
    let tangent = normalize(in.tangent.xyz);
    let vertex_normal = normalize(in.normal.xyz);

    let bitangent = cross(vertex_normal, tangent) * in.tangent.w;

    let tex0 = in.tex01.xy;
    let tex1 = in.tex01.zw;
    let tex2 = in.tex23.xy;
    let tex3 = in.tex23.zw;
    let tex4 = in.tex45.xy;
    let tex5 = in.tex45.zw;
    let tex6 = in.tex67.xy;
    let tex7 = in.tex67.zw;
    let tex8 = in.tex8.xy;

    // Required for reachability analysis to include these resources.
    let REMOVE_BEGIN = 0.0;
    var _unused = textureSample(textures[0], samplers[0], vec2(0.0));
    _unused = textureSample(textures_d3[0], samplers[0], vec3(0.0));
    _unused = textureSample(textures_cube[0], samplers[0], vec3(0.0));
    _unused = textureSample(textures[0], alpha_test_sampler, vec2(0.0));
    let REMOVE_END = 0.0;

    let ALPHA_TEST_DISCARD_GENERATED = 0.0;

    // The layout of G-Buffer textures is mostly fixed but assignments are not.
    // Each material in game can have a unique shader program.
    // Check the G-Buffer assignment database to simulate having unique shaders.
    let assignments = per_material.assignments;

    // Assume each G-Buffer texture and channel always has the same usage.
    var g_color = assignments[0].default_value;
    var g_etc_buffer = assignments[1].default_value;
    var g_normal = assignments[2].default_value;
    var g_velocity = assignments[3].default_value;
    var g_depth = assignments[4].default_value;
    var g_lgt_color = assignments[5].default_value;

    // Normals are in view space, so the view vector is simple.
    var normal = vertex_normal;
    let view = vec3(0.0, 0.0, 1.0);
    // TODO: This should use the normal after normal mapping.
    let n_dot_v = max(dot(view, normal), 0.0);

    let ASSIGN_VARS = 0.0;

    let ASSIGN_NORMAL_GENERATED = 0.0;
    
    // Not all materials and shaders use normal mapping.
    if assignments[2].has_channels.x != 0u || assignments[2].has_channels.y != 0u {
        var intensity = 1.0;
        let ASSIGN_NORMAL_INTENSITY_GENERATED = 0.0;
        intensity = pow(intensity, 0.7);
        let normal_map = create_normal_map(g_normal.x, g_normal.y) * vec3(intensity, intensity, 1.0);
        normal = apply_normal_map(normal_map, tangent, bitangent, vertex_normal);
    }
    let ao = g_normal.z;

    // TODO: front facing in calcNormalZAbs in pcmdo?

    let ASSIGN_COLOR_GENERATED = 0.0;
    let ASSIGN_ETC_GENERATED = 0.0;
    let ASSIGN_G_LGT_COLOR_GENERATED = 0.0;

    // The ordering here is the order of per material fragment shader outputs.
    // The input order for the deferred lighting pass is slightly different.
    var out: FragmentOutput;
    out.g_color = g_color;
    out.g_etc_buffer = mrt_etc_buffer(g_etc_buffer, normal);
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

    // TODO: Detect multiply by vertex color.
    var output = fragment_output(in);
    output.g_color = vec4(in.vertex_color.rgb, 0.0);
    return output;
}