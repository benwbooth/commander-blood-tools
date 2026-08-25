//! SDL host lifecycle for the modern game executable.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;

use crate::assets::{OriginalFrame, find_title_image};
use crate::render::Renderer;

const DEFAULT_WINDOW_WIDTH: u32 = 1280;
const DEFAULT_WINDOW_HEIGHT: u32 = 960;
const MINIMUM_WINDOW_DIMENSION: i32 = 1;
const FIRST_RENDERED_FRAME: u64 = 1;
const PROGRAM_NAME_ARGUMENT_COUNT: usize = 1;

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    asset: Option<PathBuf>,
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
                "--help" | "-h" => return Ok(ParseOutcome::Help),
                _ => bail!("unknown option: {argument}"),
            }
        }
        Ok(ParseOutcome::Run(options))
    }
}

fn print_usage() {
    println!(
        "Usage: commander-blood [--asset BLOOD.LBM] [--frames COUNT]\n\
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
    let image = OriginalFrame::load_lbm(&path)?;

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
    let mut renderer = Renderer::new(&window, &image)?;
    let mut events = sdl.event_pump().map_err(anyhow::Error::msg)?;
    let mut rendered_frames = u64::MIN;

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
                _ => {}
            }
        }

        renderer.render()?;
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
