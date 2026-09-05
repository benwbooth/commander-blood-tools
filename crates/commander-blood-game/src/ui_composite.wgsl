const FULLSCREEN_VERTEX_COUNT: u32 = 6u;
const SCREEN_MINIMUM: f32 = -1.0;
const SCREEN_MAXIMUM: f32 = 1.0;
const TEXTURE_MINIMUM: f32 = 0.0;
const TEXTURE_MAXIMUM: f32 = 1.0;
const QUAD_DEPTH: f32 = 0.0;
const HOMOGENEOUS_DIVISOR: f32 = 1.0;
const SRGB_LINEAR_THRESHOLD: f32 = 0.04045;
const SRGB_LINEAR_DIVISOR: f32 = 12.92;
const SRGB_CURVE_OFFSET: f32 = 0.055;
const SRGB_CURVE_SCALE: f32 = 1.055;
const SRGB_CURVE_EXPONENT: f32 = 2.4;
const SRGB_INVERSE_EXPONENT: f32 = 1.0 / SRGB_CURVE_EXPONENT;
const SRGB_INVERSE_THRESHOLD: f32 = 0.0031308;
const SRGB_INVERSE_SCALE: f32 = 12.92;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var base_texture: texture_2d<f32>;
@group(0) @binding(1) var ui_overlay: texture_2d<f32>;
@group(0) @binding(2) var presentation_sampler: sampler;

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

fn linear_channel_to_srgb(channel: f32) -> f32 {
    if channel <= SRGB_INVERSE_THRESHOLD {
        return channel * SRGB_INVERSE_SCALE;
    }
    return SRGB_CURVE_SCALE * pow(channel, SRGB_INVERSE_EXPONENT) - SRGB_CURVE_OFFSET;
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_channel_to_srgb(color.r),
        linear_channel_to_srgb(color.g),
        linear_channel_to_srgb(color.b),
    );
}

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
fn fs_base(input: VertexOutput) -> @location(0) vec4<f32> {
    let base_uv = input.position.xy / vec2<f32>(textureDimensions(base_texture));
    let base = textureSample(base_texture, presentation_sampler, base_uv);
    return vec4<f32>(base.rgb, 1.0);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let base_uv = input.position.xy / vec2<f32>(textureDimensions(base_texture));
    let base = textureSample(base_texture, presentation_sampler, base_uv);
    let ui = textureSample(ui_overlay, presentation_sampler, input.uv);
    let base_srgb = linear_to_srgb(base.rgb);
    let ui_srgb = linear_to_srgb(ui.rgb);
    let mixed_srgb = mix(base_srgb, ui_srgb, ui.a);
    return vec4<f32>(srgb_to_linear(mixed_srgb), 1.0);
}
