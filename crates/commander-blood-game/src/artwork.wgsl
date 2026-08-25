const FULLSCREEN_VERTEX_COUNT: u32 = 6u;
const SCREEN_MINIMUM: f32 = -1.0;
const SCREEN_MAXIMUM: f32 = 1.0;
const TEXTURE_MINIMUM: f32 = 0.0;
const TEXTURE_MAXIMUM: f32 = 1.0;
const QUAD_DEPTH: f32 = 0.0;
const HOMOGENEOUS_DIVISOR: f32 = 1.0;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var artwork: texture_2d<f32>;
@group(0) @binding(1) var artwork_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, FULLSCREEN_VERTEX_COUNT>(
        vec2<f32>(SCREEN_MINIMUM, SCREEN_MINIMUM),
        vec2<f32>(SCREEN_MAXIMUM, SCREEN_MINIMUM),
        vec2<f32>(SCREEN_MAXIMUM, SCREEN_MAXIMUM),
        vec2<f32>(SCREEN_MINIMUM, SCREEN_MINIMUM),
        vec2<f32>(SCREEN_MAXIMUM, SCREEN_MAXIMUM),
        vec2<f32>(SCREEN_MINIMUM, SCREEN_MAXIMUM),
    );
    let texture_coordinates = array<vec2<f32>, FULLSCREEN_VERTEX_COUNT>(
        vec2<f32>(TEXTURE_MINIMUM, TEXTURE_MAXIMUM),
        vec2<f32>(TEXTURE_MAXIMUM, TEXTURE_MAXIMUM),
        vec2<f32>(TEXTURE_MAXIMUM, TEXTURE_MINIMUM),
        vec2<f32>(TEXTURE_MINIMUM, TEXTURE_MAXIMUM),
        vec2<f32>(TEXTURE_MAXIMUM, TEXTURE_MINIMUM),
        vec2<f32>(TEXTURE_MINIMUM, TEXTURE_MINIMUM),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(
        positions[vertex_index],
        QUAD_DEPTH,
        HOMOGENEOUS_DIVISOR,
    );
    output.uv = texture_coordinates[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(artwork, artwork_sampler, input.uv);
}
