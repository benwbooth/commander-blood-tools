const LOGICAL_WIDTH: f32 = 320.0;
const LOGICAL_HEIGHT: f32 = 200.0;

@group(0) @binding(0)
var panorama: texture_2d<u32>;

@group(0) @binding(1)
var palette: texture_2d<f32>;

struct StarVertexInput {
    @location(0) screen: vec2<f32>,
    @location(1) palette_index: u32,
};

struct IndexedVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) palette_index: u32,
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
fn vs_star(input: StarVertexInput) -> IndexedVertexOutput {
    var output: IndexedVertexOutput;
    output.position = vec4<f32>(logical_to_clip(input.screen), 0.0, 1.0);
    output.palette_index = input.palette_index;
    return output;
}

@fragment
fn fs_indexed(input: IndexedVertexOutput) -> @location(0) vec4<f32> {
    return textureLoad(palette, vec2<i32>(i32(input.palette_index), 0), 0);
}

@vertex
fn vs_panorama(@builtin(vertex_index) vertex_index: u32) -> PanoramaVertexOutput {
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    var output: PanoramaVertexOutput;
    output.position = vec4<f32>(x - 1.0, 1.0 - y, 0.0, 1.0);
    output.texture_coordinates = vec2<f32>(x, y) * 0.5;
    return output;
}

@fragment
fn fs_panorama(input: PanoramaVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(input.texture_coordinates * vec2<f32>(LOGICAL_WIDTH, LOGICAL_HEIGHT));
    let palette_index = textureLoad(panorama, pixel, 0).r;
    if palette_index == 0u {
        discard;
    }
    return textureLoad(palette, vec2<i32>(i32(palette_index), 0), 0);
}
