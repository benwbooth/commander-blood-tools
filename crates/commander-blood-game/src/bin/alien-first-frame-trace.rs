//! Deterministic frame trace for original alien XDB parity audits.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};
use commander_blood_game::native::alien::{
    AlienFrameRenderStage, AlienMouseSample, AlienScene, AlienSceneRuntime,
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
const USAGE: &str = "alien-first-frame-trace MODULE XDB [RGBA-OUTPUT] [STAGE] \
                     [TIMING-SCALE] [FRAME-COUNT] [INPUT-CAMPAIGN] [TRACE-MODEL]";

#[derive(Clone, Copy)]
enum InputCampaign {
    Centered,
    Corners,
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
    let mut runtime = AlienSceneRuntime::enter(asset, timing_scale, INITIAL_FRAME_CLOCK);
    let mut selected_frame = None;
    for frame_number in 1..=frame_count {
        let key_events: &[u16] = if frame_number == frame_count {
            &[ESCAPE_KEY_EVENT]
        } else {
            &[]
        };
        let step = runtime
            .step(campaign_mouse(frame_number, input_campaign), key_events)
            .with_context(|| format!("rendering alien frame {frame_number}"))?;
        selected_frame = step.frame;
        if frame_number < frame_count && !runtime.is_running() {
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
        frame_count,
        input_campaign: input_campaign_name(input_campaign),
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
            campaign => bail!("unknown input campaign {campaign:?}; expected centered or corners"),
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
    }
}

fn campaign_mouse(frame_number: usize, campaign: InputCampaign) -> AlienMouseSample {
    if matches!(campaign, InputCampaign::Centered) {
        return AlienMouseSample {
            x: CENTERED_MOUSE_X,
            y: CENTERED_MOUSE_Y,
            buttons: u16::MIN,
        };
    }
    const MAXIMUM_MOUSE_X: u16 = 640;
    const MAXIMUM_MOUSE_Y: u16 = 1_024;

    let phase = frame_number.saturating_sub(1) & CORNERS_PHASE_MASK;
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
