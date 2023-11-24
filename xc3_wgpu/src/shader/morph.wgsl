// TODO: Share type with model.wgsl?
struct VertexInput0 {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
}

struct MorphVertexDelta {
    position_delta: vec4<f32>,
    normal_delta: vec4<f32>,
    tangent_delta: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read_write> vertices: array<VertexInput0>;

// deltas[num_morphs][num_vertices]
@group(0) @binding(1)
var<storage, read> morph_deltas: array<MorphVertexDelta>;

@group(0) @binding(2)
var<storage, read> morph_weights: array<f32>;

@compute
@workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_index = global_id.x;
    let vertex_count = arrayLength(&vertices);
    if vertex_index < arrayLength(&vertices) {
        // Sum up each morph influence for this vertex.
        // Multiple targets can affect the same vertex,
        // so this needs to be done sequentially.
        let morph_count = arrayLength(&morph_weights);
        for (var i = 0u; i < morph_count; i += 1u) {
            let weight = morph_weights[i];

            let delta_index = i * vertex_count + vertex_index;
            if delta_index < arrayLength(&morph_deltas) {
                let delta = morph_deltas[delta_index];

                vertices[vertex_index].position += vec4(delta.position_delta.xyz * weight, 0.0);
                vertices[vertex_index].normal += vec4(delta.normal_delta.xyz * weight, 0.0);
                vertices[vertex_index].tangent += vec4(delta.tangent_delta.xyz * weight, 0.0);
            }
        }
    }
}
