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

@group(1) @binding(1)
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

    switch (debug_settings.index.x) {
        case 0u: {
            return g0;
        }
        case 1u: {
            return g1;
        }
        case 2u: {
            return g2;
        }
        case 3u: {
            return g3;
        }
        case 4u: {
            return g4;
        }
        case 5u: {
            return g5;
        }
        default: {
            return g0;
        }
    }
}