const FRAME_WIDTH: f32 = 320.0;
const FRAME_HEIGHT: f32 = 200.0;

struct FrameOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
};

@group(0) @binding(0) var alien_frame: texture_2d<f32>;

@vertex
fn vs_frame(@builtin(vertex_index) vertex_index: u32) -> FrameOutput {
    let positions = array(
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
    );
    let texture_coordinates = array(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
    );
    var output: FrameOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.texture_coordinates = texture_coordinates[vertex_index];
    return output;
}

@fragment
fn fs_frame(input: FrameOutput) -> @location(0) vec4<f32> {
    let texel = clamp(
        vec2<i32>(floor(input.texture_coordinates * vec2<f32>(FRAME_WIDTH, FRAME_HEIGHT))),
        vec2<i32>(0),
        vec2<i32>(i32(FRAME_WIDTH) - 1, i32(FRAME_HEIGHT) - 1),
    );
    return textureLoad(alien_frame, texel, 0);
}
