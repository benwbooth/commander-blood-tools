//! Deterministic frame trace for original alien XDB parity audits.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::{AlienBehaviorMethod, AlienXdbKind, decode_alien_xdb};
use commander_blood_game::native::alien::{
    AlienFrameRenderStage, AlienMouseSample, AlienScene, AlienSceneRuntime, AlienWaveSelection,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const DEFAULT_TIMING_SCALE: u16 = 7;
const INITIAL_FRAME_CLOCK: u32 = 0;
const CENTERED_MOUSE_X: u16 = 320;
const CENTERED_MOUSE_Y: u16 = 512;
const ESCAPE_KEY_EVENT: u16 = 0x011b;
const CORNERS_PHASE_MASK: usize = 7;
const CORNERS_LEFT_PHASE: usize = 1;
const CORNERS_RIGHT_PHASE: usize = 2;
const CORNERS_TOP_PHASE: usize = 3;
const CORNERS_BOTTOM_PHASE: usize = 4;
const CORNERS_TOP_LEFT_PHASE: usize = 5;
const CORNERS_BOTTOM_RIGHT_PHASE: usize = 6;
const COLLECTION_TARGET: u16 = 10;
const USAGE: &str = "alien-first-frame-trace MODULE XDB [RGBA-OUTPUT] [STAGE] \
                     [TIMING-SCALE] [FRAME-COUNT] [INPUT-CAMPAIGN] [TRACE-MODEL]";

#[derive(Clone, Copy)]
enum InputCampaign {
    Centered,
    Corners,
    Forward,
    ForwardCorners,
    Navigate,
}

#[derive(Debug, PartialEq)]
enum FlightInput {
    Move(u16, u16),
    Key(u16),
}

impl FlightInput {
    fn apply(self, mouse: &mut AlienMouseSample, actions: &mut Vec<String>) -> Option<u16> {
        match self {
            Self::Move(x, y) => {
                mouse.x = x * 2;
                mouse.y = (u32::from(y) * 1024 / 200) as u16;
                actions.push(format!("move {x} {y}"));
                None
            }
            Self::Key(key) => {
                actions.push(format!("key {}", key >> 8));
                Some(key)
            }
        }
    }
}

fn steering_input(
    frame: usize,
    delta: [f64; 3],
    pan: i16,
    velocity: i16,
    backward: bool,
) -> Option<FlightInput> {
    // await-alien consumes the first neutral frame before replay begins.
    if frame == 1 {
        return None;
    }
    if frame.is_multiple_of(8) {
        let direction = if backward { 1.0 } else { -1.0 };
        let desired_pan =
            (delta[0] * direction).atan2(delta[2] * direction) * 2048.0 / std::f64::consts::PI;
        let error = (desired_pan - f64::from(pan) + 2048.0).rem_euclid(4096.0) - 2048.0;
        let desired_pitch =
            (-delta[1] * direction).atan2(delta[0].hypot(delta[2])) * 2048.0 / std::f64::consts::PI;
        let x = (160.0 + error * 0.4).round().clamp(0.0, 319.0) as u16;
        let y = ((512.0 - desired_pitch * 0.5) * 200.0 / 1024.0)
            .round()
            .clamp(0.0, 199.0) as u16;
        Some(FlightInput::Move(x, y))
    } else {
        let distance = delta.iter().map(|x| x * x).sum::<f64>().sqrt();
        // Back into a Manta to avoid its forward collision impulse.
        let desired_speed = (distance * 0.2).clamp(12.0, 180.0) * if backward { -1.0 } else { 1.0 };
        Some(FlightInput::Key(if f64::from(velocity) < desired_speed {
            0x4800
        } else {
            0x5000
        }))
    }
}

fn navigation_input(
    scene: &AlienScene,
    frame: usize,
    rings: &[usize],
    waves: &[usize],
) -> Result<Option<FlightInput>> {
    let backward = scene.callback_state.wave_selection != AlienWaveSelection::Disabled;
    let targets = if backward { waves } else { rings };
    let target_index = *targets.first().context("no collection target")?;
    let target = scene
        .models
        .get(target_index)
        .and_then(|model| model.nodes.first())
        .context("collection target has no root node")?
        .local_position;
    let delta = std::array::from_fn(|i| {
        let center = -(target[i] as i16 as f64) + if i == 1 { 124.0 } else { 0.0 };
        center - f64::from(scene.camera.view[i])
    });
    Ok(steering_input(
        frame,
        delta,
        scene.control.pan,
        scene.control.depth_velocity,
        backward,
    ))
}

#[derive(Serialize)]
struct FrameTrace {
    module: &'static str,
    xdb_file: String,
    xdb_bytes: usize,
    xdb_sha256: String,
    rgba_bytes: usize,
    rgba_sha256: String,
    render_stage: &'static str,
    timing_scale: u16,
    returned_timing_scale: u16,
    wave_selection: String,
    wave_target_positions: Vec<[i32; 3]>,
    ring_target_positions: Vec<[i32; 3]>,
    input_actions: Option<Vec<String>>,
    frame_count: usize,
    input_campaign: &'static str,
    camera_matrix: [[i32; 3]; 3],
    camera_position: [i32; 3],
    camera_view: [i16; 3],
    camera_result: [i32; 3],
    camera_pitch: i16,
    camera_pan: i16,
    camera_secondary_pan: i16,
    camera_depth_step: i16,
    primary_screens: Vec<[i16; 2]>,
    primary_clip_flags: Vec<u16>,
    primary_render_requested: bool,
    primary_triangles: Vec<PrimaryTriangleTrace>,
    model: Option<ModelTrace>,
}

#[derive(Serialize)]
struct PrimaryTriangleTrace {
    face_index: usize,
    activation_column: usize,
    screens: [[i16; 2]; 3],
}

#[derive(Serialize)]
struct ModelTrace {
    model_index: usize,
    root_matrix: [[i32; 3]; 3],
    root_translation: [i32; 3],
    nodes: Vec<NodeTrace>,
    projected_vertices: Vec<ProjectedVertexTrace>,
    object_positions: Vec<[i16; 3]>,
    texture_coordinates: Vec<[i16; 2]>,
}

#[derive(Serialize)]
struct NodeTrace {
    first_vertex: usize,
    vertex_count: usize,
    matrix: [[i32; 3]; 3],
    translation: [i32; 3],
    local_position: [i32; 3],
    angles: [u16; 3],
    radial_offset: i16,
}

#[derive(Serialize)]
struct ProjectedVertexTrace {
    screen: [i16; 2],
    depth: i32,
    clip_flags: u16,
}

fn main() -> Result<()> {
    let (
        kind,
        module,
        xdb,
        rgba_output,
        render_stage,
        timing_scale,
        frame_count,
        input_campaign,
        trace_model,
    ) = arguments()?;
    let data = std::fs::read(&xdb).with_context(|| format!("reading {}", xdb.display()))?;
    let asset = decode_alien_xdb(&data, kind)
        .with_context(|| format!("decoding original alien overlay {}", xdb.display()))?;
    let wave_models: Vec<usize> = asset
        .models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| (model.behavior == AlienBehaviorMethod::Wave).then_some(index))
        .collect();
    let ring_models: Vec<usize> = asset
        .models
        .iter()
        .enumerate()
        .filter_map(|(index, model)| {
            (model.behavior == AlienBehaviorMethod::RingAnimation).then_some(index)
        })
        .collect();
    let mut runtime = AlienSceneRuntime::enter(asset, timing_scale, INITIAL_FRAME_CLOCK);
    let mut selected_frame = None;
    let mut input_actions = Vec::new();
    let mut mouse = AlienMouseSample {
        x: CENTERED_MOUSE_X,
        y: CENTERED_MOUSE_Y,
        buttons: 0,
    };
    let mut observed_frames = 0;
    for frame_number in 1..=frame_count {
        let navigating = matches!(input_campaign, InputCampaign::Navigate);
        let key = if frame_number == frame_count
            || (runtime.timing_scale() >= COLLECTION_TARGET && navigating)
        {
            if navigating {
                FlightInput::Key(ESCAPE_KEY_EVENT).apply(&mut mouse, &mut input_actions);
            }
            Some(ESCAPE_KEY_EVENT)
        } else if navigating {
            navigation_input(runtime.scene(), frame_number, &ring_models, &wave_models)?
                .and_then(|input| input.apply(&mut mouse, &mut input_actions))
        } else if matches!(
            input_campaign,
            InputCampaign::Forward | InputCampaign::ForwardCorners
        ) {
            Some(0x4800)
        } else {
            None
        };
        if !navigating {
            mouse = campaign_mouse(frame_number, input_campaign);
        }
        let key_events = key.map(|key| [key]);
        let step = runtime
            .step(
                mouse,
                key_events.as_ref().map_or(&[], |keys| keys.as_slice()),
            )
            .with_context(|| format!("rendering alien frame {frame_number}"))?;
        observed_frames = frame_number;
        selected_frame = step.frame;
        if frame_number < frame_count && !runtime.is_running() {
            if matches!(input_campaign, InputCampaign::Navigate) {
                break;
            }
            bail!("alien overlay stopped before requested frame {frame_count}");
        }
    }
    let frame = selected_frame.context("alien overlay stopped before rendering a frame")?;
    let pixels = runtime
        .scene()
        .rasterize_frame_stage(&frame, render_stage)
        .context("rasterizing selected alien frame stage")?;
    if let Some(path) = rgba_output {
        std::fs::write(&path, &pixels).with_context(|| format!("writing {}", path.display()))?;
    }
    let scene = runtime.scene();
    let trace = FrameTrace {
        module,
        xdb_file: xdb.display().to_string(),
        xdb_bytes: data.len(),
        xdb_sha256: sha256(&data),
        rgba_bytes: pixels.len(),
        rgba_sha256: sha256(&pixels),
        render_stage: render_stage_name(render_stage),
        timing_scale,
        frame_count: observed_frames,
        input_campaign: input_campaign_name(input_campaign),
        returned_timing_scale: runtime.timing_scale(),
        wave_selection: format!("{:?}", scene.callback_state.wave_selection),
        wave_target_positions: wave_models
            .iter()
            .map(|&index| scene.models[index].nodes[0].local_position)
            .collect(),
        ring_target_positions: ring_models
            .iter()
            .map(|&index| scene.models[index].nodes[0].local_position)
            .collect(),
        input_actions: matches!(input_campaign, InputCampaign::Navigate).then_some(input_actions),
        camera_matrix: scene.camera.matrix,
        camera_position: scene.camera.position,
        camera_view: scene.camera.view,
        camera_result: scene.camera.transformed_view,
        camera_pitch: scene.control.pitch,
        camera_pan: scene.control.pan,
        camera_secondary_pan: scene.control.secondary_pan,
        camera_depth_step: scene.control.depth_velocity,
        primary_screens: scene
            .primary
            .projected_vertices
            .iter()
            .map(|vertex| vertex.screen)
            .collect(),
        primary_clip_flags: scene
            .primary
            .projected_vertices
            .iter()
            .map(|vertex| vertex.clip_flags)
            .collect(),
        primary_render_requested: frame.primary.render_requested,
        primary_triangles: frame
            .geometry
            .primary_triangles
            .iter()
            .map(|triangle| PrimaryTriangleTrace {
                face_index: triangle.source.face_index,
                activation_column: triangle.activation_column(),
                screens: triangle.vertices.map(|vertex| vertex.screen),
            })
            .collect(),
        model: trace_model
            .map(|model_index| model_trace(scene, model_index))
            .transpose()?,
    };
    println!("{}", serde_json::to_string(&trace)?);
    Ok(())
}

fn arguments() -> Result<(
    AlienXdbKind,
    &'static str,
    PathBuf,
    Option<PathBuf>,
    AlienFrameRenderStage,
    u16,
    usize,
    InputCampaign,
    Option<usize>,
)> {
    let mut arguments = std::env::args_os().skip(1);
    let module = arguments
        .next()
        .with_context(|| format!("usage: {USAGE}"))?;
    let module = module
        .to_str()
        .context("alien module must be valid UTF-8")?;
    let (kind, module) = match module.to_ascii_lowercase().as_str() {
        "amer" => (AlienXdbKind::Amer, "amer"),
        "croolis" => (AlienXdbKind::Croolis, "croolis"),
        "scrut" => (AlienXdbKind::Scrut, "scrut"),
        _ => bail!("unknown alien module {module:?}; expected amer, croolis, or scrut"),
    };
    let xdb_argument = arguments
        .next()
        .with_context(|| format!("usage: {USAGE}"))?;
    let xdb = Path::new(&xdb_argument).to_owned();
    let rgba_output = arguments.next().map(PathBuf::from);
    let render_stage = match arguments.next() {
        None => AlienFrameRenderStage::Full,
        Some(stage) => match stage.to_string_lossy().to_ascii_lowercase().as_str() {
            "primary" => AlienFrameRenderStage::Primary,
            "stars" | "starfield" => AlienFrameRenderStage::Starfield,
            "full" => AlienFrameRenderStage::Full,
            stage if stage.starts_with("models:") => {
                let count = stage["models:".len()..]
                    .parse::<usize>()
                    .with_context(|| format!("invalid behavior-model count in {stage:?}"))?;
                AlienFrameRenderStage::Models(count)
            }
            stage => bail!(
                "unknown render stage {stage:?}; expected primary, stars, models:COUNT, or full"
            ),
        },
    };
    let timing_scale = arguments
        .next()
        .map(|value| {
            value
                .to_string_lossy()
                .parse::<u16>()
                .context("timing scale must fit an unsigned 16-bit word")
        })
        .transpose()?
        .unwrap_or(DEFAULT_TIMING_SCALE);
    let frame_count = arguments
        .next()
        .map(|value| {
            value
                .to_string_lossy()
                .parse::<usize>()
                .context("frame count must be positive")
        })
        .transpose()?
        .unwrap_or(1);
    if frame_count == 0 {
        bail!("frame count must be positive");
    }
    let input_campaign = match arguments.next() {
        None => InputCampaign::Centered,
        Some(campaign) => match campaign.to_string_lossy().to_ascii_lowercase().as_str() {
            "centered" => InputCampaign::Centered,
            "corners" => InputCampaign::Corners,
            "forward" => InputCampaign::Forward,
            "forward-corners" => InputCampaign::ForwardCorners,
            "navigate" => InputCampaign::Navigate,
            campaign => bail!(
                "unknown input campaign {campaign:?}; expected centered, corners, forward, forward-corners, or navigate"
            ),
        },
    };
    let trace_model = arguments
        .next()
        .map(|value| {
            value
                .to_string_lossy()
                .parse::<usize>()
                .context("trace model must be a zero-based index")
        })
        .transpose()?;
    if arguments.next().is_some() {
        bail!("usage: {USAGE}");
    }
    Ok((
        kind,
        module,
        xdb,
        rgba_output,
        render_stage,
        timing_scale,
        frame_count,
        input_campaign,
        trace_model,
    ))
}

const fn render_stage_name(stage: AlienFrameRenderStage) -> &'static str {
    match stage {
        AlienFrameRenderStage::Primary => "primary",
        AlienFrameRenderStage::Starfield => "stars",
        AlienFrameRenderStage::Models(_) => "models",
        AlienFrameRenderStage::Full => "full",
    }
}

const fn input_campaign_name(campaign: InputCampaign) -> &'static str {
    match campaign {
        InputCampaign::Centered => "centered",
        InputCampaign::Corners => "corners",
        InputCampaign::Forward => "forward",
        InputCampaign::ForwardCorners => "forward-corners",
        InputCampaign::Navigate => "navigate",
    }
}

fn campaign_mouse(frame_number: usize, campaign: InputCampaign) -> AlienMouseSample {
    if matches!(campaign, InputCampaign::Centered | InputCampaign::Forward) {
        return AlienMouseSample {
            x: CENTERED_MOUSE_X,
            y: CENTERED_MOUSE_Y,
            buttons: u16::MIN,
        };
    }
    const MAXIMUM_MOUSE_X: u16 = 640;
    const MAXIMUM_MOUSE_Y: u16 = 1_024;

    let frame = if matches!(campaign, InputCampaign::ForwardCorners) {
        frame_number / 50
    } else {
        frame_number
    };
    let phase = frame.saturating_sub(1) & CORNERS_PHASE_MASK;
    let mut x = CENTERED_MOUSE_X;
    let mut y = CENTERED_MOUSE_Y;
    if matches!(phase, CORNERS_LEFT_PHASE | CORNERS_TOP_LEFT_PHASE) {
        x = u16::MIN;
    } else if matches!(phase, CORNERS_RIGHT_PHASE | CORNERS_BOTTOM_RIGHT_PHASE) {
        x = MAXIMUM_MOUSE_X;
    }
    if matches!(phase, CORNERS_TOP_PHASE | CORNERS_TOP_LEFT_PHASE) {
        y = u16::MIN;
    } else if matches!(phase, CORNERS_BOTTOM_PHASE | CORNERS_BOTTOM_RIGHT_PHASE) {
        y = MAXIMUM_MOUSE_Y;
    }
    AlienMouseSample {
        x,
        y,
        buttons: u16::MIN,
    }
}

fn model_trace(scene: &AlienScene, model_index: usize) -> Result<ModelTrace> {
    let pose = scene
        .models
        .get(model_index)
        .with_context(|| format!("alien scene has no model {model_index}"))?;
    Ok(ModelTrace {
        model_index,
        root_matrix: pose.root.matrix,
        root_translation: pose.root.translation,
        nodes: pose
            .nodes
            .iter()
            .map(|node| NodeTrace {
                first_vertex: node.first_vertex,
                vertex_count: node.vertex_count,
                matrix: node.transform.matrix,
                translation: node.transform.translation,
                local_position: node.local_position,
                angles: node.angles,
                radial_offset: node.radial_offset,
            })
            .collect(),
        projected_vertices: pose
            .projected_vertices
            .iter()
            .map(|vertex| ProjectedVertexTrace {
                screen: vertex.screen,
                depth: vertex.depth,
                clip_flags: vertex.clip_flags,
            })
            .collect(),
        object_positions: pose.object_positions.clone(),
        texture_coordinates: pose.texture_coordinates.clone(),
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flight_plan_starts_neutral_and_uses_opposite_delivery_thrust() {
        let delta = [0.0, 0.0, -1000.0];
        assert_eq!(steering_input(1, delta, 0, 0, false), None);
        assert_eq!(
            steering_input(2, delta, 0, 0, false),
            Some(FlightInput::Key(0x4800))
        );
        assert_eq!(
            steering_input(2, delta, 0, 0, true),
            Some(FlightInput::Key(0x5000))
        );
        assert_eq!(
            steering_input(8, delta, 0, 0, false),
            Some(FlightInput::Move(160, 100))
        );
    }

    #[test]
    fn flight_inputs_round_trip_to_logical_scenario_coordinates() {
        let mut mouse = campaign_mouse(1, InputCampaign::Centered);
        let mut actions = Vec::new();
        assert_eq!(
            FlightInput::Move(319, 199).apply(&mut mouse, &mut actions),
            None
        );
        assert_eq!((mouse.x, mouse.y), (638, 1018));
        assert_eq!(
            FlightInput::Key(ESCAPE_KEY_EVENT).apply(&mut mouse, &mut actions),
            Some(ESCAPE_KEY_EVENT)
        );
        assert_eq!(actions, ["move 319 199", "key 1"]);
    }
}
