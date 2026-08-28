const LOGICAL_WIDTH: f32 = 320.0;
const LOGICAL_HEIGHT: f32 = 200.0;
const FULLSCREEN_TRIANGLE_SCALE: f32 = 2.0;

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

@vertex
fn vs_star(input: StarVertexInput) -> ColorVertexOutput {
    var output: ColorVertexOutput;
    output.position = vec4<f32>(logical_to_clip(input.screen), 0.0, 1.0);
    output.color = input.color;
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
