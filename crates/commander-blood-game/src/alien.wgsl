const ORIGINAL_WIDTH: f32 = 320.0;
const ORIGINAL_HEIGHT: f32 = 200.0;
const NDC_EXTENT: f32 = 2.0;
const NDC_LEFT: f32 = -1.0;
const NDC_TOP: f32 = 1.0;

struct TexturedInput {
    @location(0) screen: vec2<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) depth: f32,
};

struct TexturedOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
};

struct StarInput {
    @location(0) screen: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct StarOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
};

@group(0) @binding(0) var alien_texture: texture_2d<f32>;

fn screen_position(screen: vec2<f32>, depth: f32) -> vec4<f32> {
    let x = NDC_LEFT + screen.x * NDC_EXTENT / ORIGINAL_WIDTH;
    let y = NDC_TOP - screen.y * NDC_EXTENT / ORIGINAL_HEIGHT;
    return vec4<f32>(x, y, depth, 1.0);
}

@vertex
fn vs_textured(input: TexturedInput) -> TexturedOutput {
    var output: TexturedOutput;
    output.position = screen_position(input.screen, input.depth);
    output.texture_coordinates = input.texture_coordinates;
    return output;
}

@fragment
fn fs_textured(input: TexturedOutput) -> @location(0) vec4<f32> {
    let texture_size = textureDimensions(alien_texture);
    let texel = clamp(
        vec2<i32>(floor(input.texture_coordinates)),
        vec2<i32>(0),
        vec2<i32>(texture_size) - vec2<i32>(1),
    );
    return textureLoad(alien_texture, texel, 0);
}

@vertex
fn vs_star(input: StarInput) -> StarOutput {
    var output: StarOutput;
    output.position = screen_position(input.screen, 0.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_star(input: StarOutput) -> @location(0) vec4<f32> {
    return input.color;
}
