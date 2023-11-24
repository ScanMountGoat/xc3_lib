// TODO: Share type with model.wgsl?
struct VertexInput0 {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

// Store a flattened list of vertex updates.
// This allows updating the morph weights separately.
// Each morph target can also have a different vertex count.
// Including the vertex index allows for "sparse" data.
struct MorphVertexDelta {
    position_delta: vec4<f32>,
    normal_delta: vec3<f32>,
    morph_index: u32,
    tangent_delta: vec3<f32>,
    vertex_index: u32,
}

@group(0) @binding(0)
var<storage, read> input: array<VertexInput0>;

@group(0) @binding(1)
var<storage, read_write> output: array<VertexInput0>;

@group(0) @binding(2)
var<storage, read> morph_deltas: array<MorphVertexDelta>;

@group(0) @binding(3)
var<storage, read> morph_weights: array<f32>;

@compute
@workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if index < arrayLength(&morph_deltas) {
        let delta = morph_deltas[index];

        let morph_index = delta.morph_index;
        if morph_index < arrayLength(&morph_weights) {
            let weight = morph_weights[morph_index];

            let vertex_index = delta.vertex_index;
            if vertex_index < arrayLength(&input) && vertex_index < arrayLength(&output) {
                let vertex = input[vertex_index];
                output[vertex_index].position = vec4(vertex.position.xyz + delta.position_delta.xyz * weight, 0.0);
                output[vertex_index].normal = vec4(vertex.normal.xyz + delta.normal_delta.xyz * weight, 0.0);
                output[vertex_index].tangent = vec4(vertex.tangent.xyz + delta.tangent_delta.xyz * weight, vertex.tangent.w);
            }
        }
    }
}
