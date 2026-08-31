const LOGICAL_WIDTH: f32 = 320.0;
const LOGICAL_HEIGHT: f32 = 200.0;
const FULLSCREEN_TRIANGLE_SCALE: f32 = 2.0;
const SRGB_LINEAR_THRESHOLD: f32 = 0.04045;
const SRGB_LINEAR_DIVISOR: f32 = 12.92;
const SRGB_CURVE_OFFSET: f32 = 0.055;
const SRGB_CURVE_SCALE: f32 = 1.055;
const SRGB_CURVE_EXPONENT: f32 = 2.4;

@group(0) @binding(0)
var panorama: texture_2d<f32>;

struct StarVertexInput {
    @location(0) screen: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct ColorVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
};

struct PanoramaVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
};

fn logical_to_clip(screen: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        screen.x * 2.0 / LOGICAL_WIDTH - 1.0,
        1.0 - screen.y * 2.0 / LOGICAL_HEIGHT,
    );
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= SRGB_LINEAR_THRESHOLD {
        return channel / SRGB_LINEAR_DIVISOR;
    }
    return pow(
        (channel + SRGB_CURVE_OFFSET) / SRGB_CURVE_SCALE,
        SRGB_CURVE_EXPONENT,
    );
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
    );
}

@vertex
fn vs_star(input: StarVertexInput) -> ColorVertexOutput {
    var output: ColorVertexOutput;
    output.position = vec4<f32>(logical_to_clip(input.screen), 0.0, 1.0);
    output.color = vec4<f32>(srgb_to_linear(input.color.rgb), input.color.a);
    return output;
}

@fragment
fn fs_color(input: ColorVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@vertex
fn vs_panorama(@builtin(vertex_index) vertex_index: u32) -> PanoramaVertexOutput {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var output: PanoramaVertexOutput;
    output.position = vec4<f32>(
        x * FULLSCREEN_TRIANGLE_SCALE - 1.0,
        1.0 - y * FULLSCREEN_TRIANGLE_SCALE,
        0.0,
        1.0,
    );
    output.texture_coordinates = vec2<f32>(x, y);
    return output;
}

@fragment
fn fs_panorama(input: PanoramaVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.texture_coordinates * vec2<f32>(LOGICAL_WIDTH, LOGICAL_HEIGHT));
    let color = textureLoad(panorama, pixel, 0);
    if color.a == 0.0 {
        discard;
    }
    return color;
}
