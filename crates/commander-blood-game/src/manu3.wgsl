const ORIGINAL_WIDTH: f32 = 320.0;
const ORIGINAL_HEIGHT: f32 = 200.0;
const NDC_EXTENT: f32 = 2.0;
const NDC_LEFT: f32 = -1.0;
const NDC_TOP: f32 = 1.0;

struct VertexInput {
    @location(0) screen: vec2<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) depth: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
};

@group(0) @binding(0) var hand_texture: texture_2d<u32>;
@group(0) @binding(1) var scene_palette: texture_2d<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let x = NDC_LEFT + input.screen.x * NDC_EXTENT / ORIGINAL_WIDTH;
    let y = NDC_TOP - input.screen.y * NDC_EXTENT / ORIGINAL_HEIGHT;
    output.position = vec4<f32>(x, y, input.depth, 1.0);
    output.texture_coordinates = input.texture_coordinates;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = textureDimensions(hand_texture);
    let texel = clamp(
        vec2<i32>(floor(input.texture_coordinates)),
        vec2<i32>(0),
        vec2<i32>(texture_size) - vec2<i32>(1),
    );
    let palette_index = textureLoad(hand_texture, texel, 0).r;
    return textureLoad(scene_palette, vec2<i32>(i32(palette_index), 0), 0);
}
