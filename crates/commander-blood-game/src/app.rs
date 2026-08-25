//! SDL host lifecycle for the modern game executable.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use commander_blood_formats::manu3::decode_manu3;
use commander_blood_formats::palette::{
    MANU3_PALETTE_END, MANU3_PALETTE_START, decode_bloodprg_default_palette,
};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;

use crate::assets::{OriginalFrame, find_bloodprg_executable, find_title_image};
use crate::native::manu3::animation::CursorPosition;
use crate::native::manu3::model::{Manu3FrameRequest, Manu3Model};
use crate::render::Renderer;

const DEFAULT_WINDOW_WIDTH: u32 = 1280;
const DEFAULT_WINDOW_HEIGHT: u32 = 960;
const MINIMUM_WINDOW_DIMENSION: i32 = 1;
const FIRST_RENDERED_FRAME: u64 = 1;
const PROGRAM_NAME_ARGUMENT_COUNT: usize = 1;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const ORIGINAL_SCREEN_WIDTH: f32 = 320.0;
const ORIGINAL_SCREEN_HEIGHT: f32 = 200.0;
const VIEWPORT_CENTER_DIVISOR: f32 = 2.0;
const INITIAL_CURSOR: CursorPosition = CursorPosition { x: 160, y: 100 };

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    asset: Option<PathBuf>,
    bloodprg: Option<PathBuf>,
    manu3: Option<PathBuf>,
    frame_limit: Option<u64>,
}

enum ParseOutcome {
    Run(Options),
    Help,
}

impl Options {
    fn parse() -> Result<ParseOutcome> {
        let mut options = Self::default();
        let mut arguments = std::env::args().skip(PROGRAM_NAME_ARGUMENT_COUNT);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--asset" => {
                    options.asset = Some(PathBuf::from(
                        arguments.next().context("--asset requires a path")?,
                    ));
                }
                "--frames" => {
                    let value = arguments.next().context("--frames requires a count")?;
                    options.frame_limit = Some(value.parse().context("invalid --frames count")?);
                }
                "--bloodprg" => {
                    options.bloodprg = Some(PathBuf::from(
                        arguments
                            .next()
                            .context("--bloodprg requires an executable path")?,
                    ));
                }
                "--manu3" => {
                    options.manu3 = Some(PathBuf::from(
                        arguments.next().context("--manu3 requires an XDB path")?,
                    ));
                }
                "--help" | "-h" => return Ok(ParseOutcome::Help),
                _ => bail!("unknown option: {argument}"),
            }
        }
        Ok(ParseOutcome::Run(options))
    }
}

fn print_usage() {
    println!(
        "Usage: commander-blood [--asset IMAGE.LBM] [--manu3 MANU3.XDB] [--bloodprg BLOODPRG.EXE] [--frames COUNT]\n\
         \n\
         CBLOOD_DATA may point to the original game-data directory."
    );
}

/// Start the SDL event loop and wgpu renderer for the modern game port.
pub fn run() -> Result<()> {
    let options = match Options::parse()? {
        ParseOutcome::Run(options) => options,
        ParseOutcome::Help => {
            print_usage();
            return Ok(());
        }
    };
    let path = find_title_image(options.asset.as_deref())?;
    let mut image = OriginalFrame::load_lbm(&path)?;
    let mut manu3 = options
        .manu3
        .map(|path| {
            let data = std::fs::read(&path)
                .with_context(|| format!("reading MANU3 model {}", path.display()))?;
            let asset = decode_manu3(&data)
                .with_context(|| format!("decoding MANU3 model {}", path.display()))?;
            Manu3Model::from_asset(asset).context("constructing MANU3 runtime model")
        })
        .transpose()?;
    if manu3.is_some() {
        let executable_path = find_bloodprg_executable(options.bloodprg.as_deref())?;
        let executable = std::fs::read(&executable_path)
            .with_context(|| format!("reading {}", executable_path.display()))?;
        let palette = decode_bloodprg_default_palette(&executable)
            .with_context(|| format!("decoding palette from {}", executable_path.display()))?;
        image.install_palette_range(&palette, MANU3_PALETTE_START..=MANU3_PALETTE_END);
    }

    let sdl = sdl3::init().map_err(anyhow::Error::msg)?;
    let video = sdl.video().map_err(anyhow::Error::msg)?;
    let window = video
        .window(
            "Commander Blood",
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
        )
        .position_centered()
        .resizable()
        .high_pixel_density()
        .metal_view()
        .build()
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let mut renderer = Renderer::new(&window, &image, manu3.as_ref())?;
    let mut events = sdl.event_pump().map_err(anyhow::Error::msg)?;
    let mut rendered_frames = u64::MIN;
    let mut cursor = INITIAL_CURSOR;

    'running: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::Window {
                    window_id,
                    win_event:
                        WindowEvent::PixelSizeChanged(width, height)
                        | WindowEvent::Resized(width, height),
                    ..
                } if window_id == window.id() => {
                    renderer.resize(
                        width.max(MINIMUM_WINDOW_DIMENSION) as u32,
                        height.max(MINIMUM_WINDOW_DIMENSION) as u32,
                    );
                }
                Event::MouseMotion {
                    window_id, x, y, ..
                } if window_id == window.id() => {
                    let (width, height) = window.size();
                    cursor = map_cursor_to_original(width as f32, height as f32, x, y);
                }
                _ => {}
            }
        }

        if let Some(model) = &mut manu3 {
            model.render_frame(Manu3FrameRequest {
                cursor,
                animation_selector: u16::MIN,
            })?;
        }
        let triangles = manu3
            .as_ref()
            .map(Manu3Model::render_triangles)
            .unwrap_or(&[]);
        renderer.render(triangles)?;
        rendered_frames = rendered_frames.saturating_add(FIRST_RENDERED_FRAME);
        if options
            .frame_limit
            .is_some_and(|limit| rendered_frames >= limit)
        {
            break;
        }
    }
    Ok(())
}

fn map_cursor_to_original(
    output_width: f32,
    output_height: f32,
    cursor_x: f32,
    cursor_y: f32,
) -> CursorPosition {
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    let viewport_height = ORIGINAL_DISPLAY_ASPECT_HEIGHT * scale;
    let viewport_x = (output_width - viewport_width) / VIEWPORT_CENTER_DIVISOR;
    let viewport_y = (output_height - viewport_height) / VIEWPORT_CENTER_DIVISOR;
    let x = ((cursor_x - viewport_x) * ORIGINAL_SCREEN_WIDTH / viewport_width)
        .clamp(0.0, ORIGINAL_SCREEN_WIDTH - 1.0);
    let y = ((cursor_y - viewport_y) * ORIGINAL_SCREEN_HEIGHT / viewport_height)
        .clamp(0.0, ORIGINAL_SCREEN_HEIGHT - 1.0);
    CursorPosition {
        x: x as i16,
        y: y as i16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDESCREEN_WIDTH: f32 = 1_920.0;
    const WIDESCREEN_HEIGHT: f32 = 1_080.0;

    #[test]
    fn mouse_coordinates_follow_the_letterboxed_original_display() {
        assert_eq!(
            map_cursor_to_original(
                WIDESCREEN_WIDTH,
                WIDESCREEN_HEIGHT,
                WIDESCREEN_WIDTH / VIEWPORT_CENTER_DIVISOR,
                WIDESCREEN_HEIGHT / VIEWPORT_CENTER_DIVISOR,
            ),
            INITIAL_CURSOR
        );
        assert_eq!(
            map_cursor_to_original(WIDESCREEN_WIDTH, WIDESCREEN_HEIGHT, 0.0, 0.0),
            CursorPosition { x: 0, y: 0 }
        );
    }
}
