//! Deterministic first-frame trace for original alien XDB parity audits.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};
use commander_blood_game::native::alien::{
    AlienFrameRenderStage, AlienMouseSample, AlienSceneRuntime,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const TIMING_SCALE: u16 = 7;
const INITIAL_FRAME_CLOCK: u32 = 0;
const CENTERED_MOUSE_X: u16 = 320;
const CENTERED_MOUSE_Y: u16 = 512;
const ESCAPE_KEY_EVENT: u16 = 0x011b;

#[derive(Serialize)]
struct FrameTrace {
    module: &'static str,
    xdb_file: String,
    xdb_bytes: usize,
    xdb_sha256: String,
    rgba_bytes: usize,
    rgba_sha256: String,
    render_stage: &'static str,
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
}

#[derive(Serialize)]
struct PrimaryTriangleTrace {
    face_index: usize,
    activation_column: usize,
    screens: [[i16; 2]; 3],
}

fn main() -> Result<()> {
    let (kind, module, xdb, rgba_output, render_stage) = arguments()?;
    let data = std::fs::read(&xdb).with_context(|| format!("reading {}", xdb.display()))?;
    let asset = decode_alien_xdb(&data, kind)
        .with_context(|| format!("decoding original alien overlay {}", xdb.display()))?;
    let mut runtime = AlienSceneRuntime::enter(asset, TIMING_SCALE, INITIAL_FRAME_CLOCK);
    let step = runtime
        .step(
            AlienMouseSample {
                x: CENTERED_MOUSE_X,
                y: CENTERED_MOUSE_Y,
                buttons: u16::MIN,
            },
            &[ESCAPE_KEY_EVENT],
        )
        .context("rendering first alien frame")?;
    let frame = step
        .frame
        .context("alien overlay stopped before rendering its first frame")?;
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
)> {
    let mut arguments = std::env::args_os().skip(1);
    let module = arguments
        .next()
        .context("usage: alien-first-frame-trace MODULE XDB [RGBA-OUTPUT] [STAGE]")?;
    let module = module
        .to_str()
        .context("alien module must be valid UTF-8")?;
    let (kind, module) = match module.to_ascii_lowercase().as_str() {
        "amer" => (AlienXdbKind::Amer, "amer"),
        "croolis" => (AlienXdbKind::Croolis, "croolis"),
        "scrut" => (AlienXdbKind::Scrut, "scrut"),
        _ => bail!("unknown alien module {module:?}; expected amer, croolis, or scrut"),
    };
    let xdb = Path::new(
        &arguments
            .next()
            .context("usage: alien-first-frame-trace MODULE XDB [RGBA-OUTPUT] [STAGE]")?,
    )
    .to_owned();
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
    if arguments.next().is_some() {
        bail!("usage: alien-first-frame-trace MODULE XDB [RGBA-OUTPUT] [STAGE]");
    }
    Ok((kind, module, xdb, rgba_output, render_stage))
}

const fn render_stage_name(stage: AlienFrameRenderStage) -> &'static str {
    match stage {
        AlienFrameRenderStage::Primary => "primary",
        AlienFrameRenderStage::Starfield => "stars",
        AlienFrameRenderStage::Models(_) => "models",
        AlienFrameRenderStage::Full => "full",
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
