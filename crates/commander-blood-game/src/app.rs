//! SDL host lifecycle for the modern game executable.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};
use commander_blood_formats::bloodprg::decode_bloodprg_bridge_resources;
use commander_blood_formats::manu3::decode_manu3;
use commander_blood_formats::palette::{
    MANU3_PALETTE_END, MANU3_PALETTE_START, decode_bloodprg_default_palette,
    decode_bloodprg_default_vga_palette,
};
use commander_blood_formats::panorama::BridgePanoramaArchive;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::mouse::MouseButton;

use crate::assets::{
    OriginalFrame, find_bloodprg_executable, find_bridge_panorama, find_title_image,
};
use crate::native::alien::{AlienInputAction, AlienMouseSample, AlienScene};
use crate::native::bloodprg::{
    BRIDGE_SPRITE_ENTITY_COUNT, BridgeScene, BridgeSceneInput, BridgeSpriteEntity,
    BridgeSteeringInteraction, GameLifecycleError, GameLifecycleState, InputAction, PointerButton,
    PointerButtons, ScriptClock, ShipProjectionResources, run_game_lifecycle,
};
use crate::native::manu3::animation::CursorPosition;
use crate::native::manu3::model::{Manu3FrameRequest, Manu3Model};
use crate::native::random::BloodPrng;
use crate::render::Renderer;
use crate::runtime::{
    ModernGameServices, OriginalGameData, OriginalGameDataPaths, RuntimeGameLifecycleHost,
    RuntimeInputHost, RuntimePlatformHost,
};

const DEFAULT_WINDOW_WIDTH: u32 = 1280;
const DEFAULT_WINDOW_HEIGHT: u32 = 960;
const MINIMUM_WINDOW_DIMENSION: i32 = 1;
const FIRST_RENDERED_FRAME: u64 = 1;
const PROGRAM_NAME_ARGUMENT_COUNT: usize = 1;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const ORIGINAL_SCREEN_WIDTH: f32 = 320.0;
const ORIGINAL_SCREEN_HEIGHT: f32 = 200.0;
const INITIAL_CURSOR: CursorPosition = CursorPosition { x: 160, y: 100 };
const ALIEN_DRIVER_WIDTH: u32 = 640;
const ALIEN_DRIVER_HEIGHT: u32 = 1_024;
const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;
const HOURS_PER_DAY: u64 = 24;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * HOURS_PER_DAY;
const DECIMAL_RADIX: u8 = 10;
const PACKED_BCD_DIGIT_SHIFT: u32 = 4;
const NO_MOUSE_MOTION: f32 = 0.0;
const MAXIMUM_BASE_SCENE_COUNT: usize = 1;
const UNIX_EPOCH_YEAR: u64 = 1_970;
const MONTH_COUNT: usize = 12;
const FEBRUARY_INDEX: usize = 1;
const LEAP_DAY: u16 = 1;
const ONE_BASED_CALENDAR_OFFSET: u16 = 1;
const MONTH_LENGTHS: [u16; MONTH_COUNT] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const LEAP_YEAR_INTERVAL: u64 = 4;
const LEAP_YEAR_CENTURY_INTERVAL: u64 = 100;
const LEAP_YEAR_CYCLE: u64 = 400;

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    data: Option<PathBuf>,
    write_data: Option<PathBuf>,
    asset: Option<PathBuf>,
    bloodprg: Option<PathBuf>,
    manu3: Option<PathBuf>,
    alien: Option<PathBuf>,
    bridge: bool,
    panorama: Option<PathBuf>,
    frame_limit: Option<u64>,
}

enum ParseOutcome {
    Run(Options),
    Help,
}

impl Options {
    fn uses_diagnostic_overrides(&self) -> bool {
        self.asset.is_some()
            || self.bloodprg.is_some()
            || self.manu3.is_some()
            || self.alien.is_some()
            || self.bridge
            || self.panorama.is_some()
    }

    fn parse() -> Result<ParseOutcome> {
        let mut options = Self::default();
        let mut arguments = std::env::args().skip(PROGRAM_NAME_ARGUMENT_COUNT);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--data" => {
                    options.data = Some(PathBuf::from(
                        arguments.next().context("--data requires a directory")?,
                    ));
                }
                "--write-data" => {
                    options.write_data = Some(PathBuf::from(
                        arguments
                            .next()
                            .context("--write-data requires a directory")?,
                    ));
                }
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
                "--alien" => {
                    options.alien = Some(PathBuf::from(
                        arguments.next().context("--alien requires an XDB path")?,
                    ));
                }
                "--bridge" => options.bridge = true,
                "--panorama" => {
                    options.panorama = Some(PathBuf::from(
                        arguments.next().context("--panorama requires a BIG path")?,
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
        "Usage: commander-blood [--data DIRECTORY] [--write-data DIRECTORY] [--asset IMAGE.LBM] [--manu3 MANU3.XDB | --alien ALIEN.XDB | --bridge] [--panorama TB.BIG] [--bloodprg BLOODPRG.EXE] [--frames COUNT]\n\
         \n\
         CBLOOD_DATA may point to the original game-data directory.\n\
         CBLOOD_ASSET_CACHE may select the versioned imported loose-asset directory.\n\
         CBLOOD_WRITE_DATA may point to the writable save-data directory.\n\
         CBLOOD_SCRIPT_SOURCE may select an editable re script-source directory."
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
    if !options.uses_diagnostic_overrides() {
        return run_production_game(&options);
    }
    let original_data = if options.data.is_some() || !options.uses_diagnostic_overrides() {
        let paths = OriginalGameDataPaths::discover(options.data.as_deref())?;
        Some(match options.write_data.as_deref() {
            Some(writable_root) => OriginalGameData::load_with_writable_root(paths, writable_root)?,
            None => OriginalGameData::load(paths)?,
        })
    } else {
        None
    };
    let path = match (options.asset.as_deref(), original_data.as_ref()) {
        (Some(path), _) => find_title_image(Some(path))?,
        (None, Some(data)) => data.paths().title().to_owned(),
        (None, None) => find_title_image(None)?,
    };
    let mut image = OriginalFrame::load_lbm(&path)?;
    let bridge_requested = options.bridge || options.panorama.is_some();
    let base_scene_count = usize::from(options.alien.is_some()) + usize::from(bridge_requested);
    if base_scene_count > MAXIMUM_BASE_SCENE_COUNT {
        bail!("--alien and --bridge select different base scenes");
    }
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
    let mut alien = options
        .alien
        .map(|path| {
            let kind = alien_kind(&path)?;
            let data = std::fs::read(&path)
                .with_context(|| format!("reading alien scene {}", path.display()))?;
            let asset = decode_alien_xdb(&data, kind)
                .with_context(|| format!("decoding alien scene {}", path.display()))?;
            Ok::<_, anyhow::Error>(AlienScene::from_asset(asset))
        })
        .transpose()?;
    let executable = if manu3.is_some() || bridge_requested {
        if let Some(path) = options.bloodprg.as_deref() {
            Some(std::fs::read(path).with_context(|| format!("reading {}", path.display()))?)
        } else if let Some(data) = original_data.as_ref() {
            Some(data.executable().to_vec())
        } else {
            let executable_path = find_bloodprg_executable(None)?;
            Some(
                std::fs::read(&executable_path)
                    .with_context(|| format!("reading {}", executable_path.display()))?,
            )
        }
    } else {
        None
    };
    let display_palette = executable
        .as_deref()
        .map(|executable| {
            decode_bloodprg_default_palette(executable)
                .context("decoding default palette from BLOODPRG.EXE")
        })
        .transpose()?;
    if manu3.is_some() {
        let palette = display_palette
            .as_ref()
            .context("MANU3 rendering requires the executable palette")?;
        image.install_palette_range(palette, MANU3_PALETTE_START..=MANU3_PALETTE_END);
    }
    let mut bridge = if bridge_requested {
        let executable = executable
            .as_deref()
            .context("bridge rendering requires BLOODPRG.EXE")?;
        let resources = decode_bloodprg_bridge_resources(executable)
            .context("decoding bridge projection resources from BLOODPRG.EXE")?;
        let panorama_path = match (options.panorama.as_deref(), original_data.as_ref()) {
            (Some(path), _) => find_bridge_panorama(Some(path))?,
            (None, Some(data)) => data.paths().bridge_panorama().to_owned(),
            (None, None) => find_bridge_panorama(None)?,
        };
        let panorama = BridgePanoramaArchive::decode(
            std::fs::read(&panorama_path)
                .with_context(|| format!("reading {}", panorama_path.display()))?
                .into_boxed_slice(),
        )
        .with_context(|| format!("decoding bridge panorama {}", panorama_path.display()))?;
        let mut random = BloodPrng::default();
        random.seed_from_clock_register(host_clock_seed_byte()?);
        Some(BridgeScene::new(
            panorama,
            ShipProjectionResources::from(resources),
            &mut random,
        )?)
    } else {
        None
    };
    let bridge_palette = if bridge.is_some() {
        Some(
            decode_bloodprg_default_vga_palette(
                executable
                    .as_deref()
                    .context("bridge rendering requires BLOODPRG.EXE")?,
            )
            .context("decoding native bridge palette from BLOODPRG.EXE")?,
        )
    } else {
        None
    };
    let manu3_palette = if manu3.is_some() {
        Some(
            decode_bloodprg_default_vga_palette(
                executable
                    .as_deref()
                    .context("MANU3 rendering requires BLOODPRG.EXE")?,
            )
            .context("decoding native MANU3 palette from BLOODPRG.EXE")?,
        )
    } else {
        None
    };

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
    let mut renderer = Renderer::new(
        &window,
        &image,
        manu3.as_ref(),
        manu3_palette.as_ref(),
        alien.as_ref().map(AlienScene::asset),
        bridge_palette.as_ref(),
    )?;
    let mut events = sdl.event_pump().map_err(anyhow::Error::msg)?;
    video.text_input().start(&window);
    let mut rendered_frames = u64::MIN;
    let mut input = RuntimeInputHost::new([INITIAL_CURSOR.x, INITIAL_CURSOR.y]);
    let mut bridge_horizontal_delta = NO_MOUSE_MOTION;
    let mut bridge_sprite_entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];

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
                    window_id, xrel, ..
                } if window_id == window.id() => {
                    let (width, height) = window.size();
                    bridge_horizontal_delta +=
                        map_horizontal_delta_to_original(width as f32, height as f32, xrel);
                }
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    input.queue_keycode(keycode);
                }
                Event::TextInput {
                    window_id, text, ..
                } if window_id == window.id() => {
                    input.queue_text(&text);
                }
                _ => {}
            }
        }

        let mouse = events.mouse_state();
        let (window_width, window_height) = window.size();
        let pointer_buttons = pointer_buttons(&mouse);
        let pointer_sample = input.poll_pointer(
            [window_width as f32, window_height as f32],
            [mouse.x(), mouse.y()],
            pointer_buttons,
        );
        input.update_pointer_buttons();
        let cursor = CursorPosition {
            x: pointer_sample.position[0],
            y: pointer_sample.position[1],
        };
        let dispatched_action = input.dispatch_next(false);
        if let (Some(scene), Some(action)) = (&mut alien, dispatched_action) {
            let alien_action = match action {
                InputAction::MovePrevious => AlienInputAction::IncreaseDepth,
                InputAction::MoveNext => AlienInputAction::DecreaseDepth,
                InputAction::Cancel => AlienInputAction::Interact,
                _ => AlienInputAction::None,
            };
            if alien_action != AlienInputAction::None {
                scene.control.queue_action(alien_action);
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
        let alien_frame = alien
            .as_mut()
            .map(|scene| scene.step(map_cursor_to_alien(cursor, pointer_buttons.bits())))
            .transpose()?;
        let bridge_frame = bridge
            .as_mut()
            .map(|scene| {
                scene.render_frame(
                    BridgeSceneInput {
                        horizontal_delta: bridge_horizontal_delta.round() as i32,
                        pointer_buttons: pointer_buttons.bits(),
                        interaction: BridgeSteeringInteraction::Free,
                    },
                    &mut bridge_sprite_entities,
                )
            })
            .transpose()?;
        bridge_horizontal_delta = NO_MOUSE_MOTION;
        renderer.render(triangles, alien_frame.as_ref(), bridge_frame.as_ref())?;
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

fn run_production_game(options: &Options) -> Result<()> {
    let paths = OriginalGameDataPaths::discover(options.data.as_deref())?;
    let data = match options.write_data.as_deref() {
        Some(writable_root) => OriginalGameData::load_with_writable_root(paths, writable_root)?,
        None => OriginalGameData::load(paths)?,
    };
    let data = crate::video_import::prepare_lossless_webm_derivatives(data)?;
    let clock = host_clock_sample()?;
    let sdl = sdl3::init().map_err(anyhow::Error::msg)?;
    let video = sdl.video().map_err(anyhow::Error::msg)?;
    let audio = sdl.audio().map_err(anyhow::Error::msg)?;
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
    let events = sdl.event_pump().map_err(anyhow::Error::msg)?;
    video.text_input().start(&window);

    let services = ModernGameServices::new(&window, data, clock.script)?;
    let platform = RuntimePlatformHost::new(&window, sdl.mouse(), events);
    let mut host = RuntimeGameLifecycleHost::new(
        services,
        platform,
        &audio,
        clock.packed_second,
        options.frame_limit,
    );
    let mut state = GameLifecycleState::default();
    match run_game_lifecycle(&mut state, &mut host) {
        Ok(_) => {}
        Err(GameLifecycleError::Runtime(error)) => {
            return Err(error.context("game runtime failed"));
        }
        Err(GameLifecycleError::Shutdown(error)) => {
            return Err(error.context("game shutdown failed"));
        }
        Err(GameLifecycleError::RuntimeAndShutdown { runtime, shutdown }) => {
            return Err(runtime.context(format!(
                "game runtime failed; shutdown also failed: {shutdown:#}"
            )));
        }
    }
    Ok(())
}

fn pointer_buttons(mouse: &sdl3::mouse::MouseState) -> PointerButtons {
    let mut buttons = PointerButtons::NONE.bits();
    if mouse.is_mouse_button_pressed(MouseButton::Left) {
        buttons |= PointerButton::Primary as u16;
    }
    if mouse.is_mouse_button_pressed(MouseButton::Right) {
        buttons |= PointerButton::Secondary as u16;
    }
    PointerButtons::from_bits(buttons)
}

fn host_clock_seed_byte() -> Result<u8> {
    Ok(host_clock_sample()?.packed_second)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HostClockSample {
    script: ScriptClock,
    packed_second: u8,
}

fn host_clock_sample() -> Result<HostClockSample> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("host clock precedes the Unix epoch")?;
    let elapsed_seconds = elapsed.as_secs();
    let second = u8::try_from(elapsed_seconds % SECONDS_PER_MINUTE)
        .expect("second within one minute fits u8");
    let hour = i16::try_from((elapsed_seconds % SECONDS_PER_DAY) / SECONDS_PER_HOUR)
        .expect("hour within one day fits i16");
    let (month, day) = utc_month_day(elapsed_seconds / SECONDS_PER_DAY);
    Ok(HostClockSample {
        script: ScriptClock { hour, day, month },
        packed_second: pack_clock_seconds(second),
    })
}

fn utc_month_day(mut days_since_epoch: u64) -> (i8, i8) {
    let mut year = UNIX_EPOCH_YEAR;
    loop {
        let year_days = u64::from(days_in_year(year));
        if days_since_epoch < year_days {
            break;
        }
        days_since_epoch -= year_days;
        year += 1;
    }

    for (month_index, base_length) in MONTH_LENGTHS.into_iter().enumerate() {
        let month_length =
            base_length + u16::from(month_index == FEBRUARY_INDEX && is_leap_year(year)) * LEAP_DAY;
        if days_since_epoch < u64::from(month_length) {
            let month = i8::try_from(month_index + usize::from(ONE_BASED_CALENDAR_OFFSET))
                .expect("one-based month fits i8");
            let day = i8::try_from(days_since_epoch + u64::from(ONE_BASED_CALENDAR_OFFSET))
                .expect("one-based day fits i8");
            return (month, day);
        }
        days_since_epoch -= u64::from(month_length);
    }
    unreachable!("validated Gregorian year contains every remaining day")
}

const fn days_in_year(year: u64) -> u16 {
    if is_leap_year(year) { 366 } else { 365 }
}

const fn is_leap_year(year: u64) -> bool {
    year.is_multiple_of(LEAP_YEAR_INTERVAL)
        && (!year.is_multiple_of(LEAP_YEAR_CENTURY_INTERVAL)
            || year.is_multiple_of(LEAP_YEAR_CYCLE))
}

fn pack_clock_seconds(seconds: u8) -> u8 {
    let tens = seconds / DECIMAL_RADIX;
    let ones = seconds % DECIMAL_RADIX;
    tens << PACKED_BCD_DIGIT_SHIFT | ones
}

fn alien_kind(path: &Path) -> Result<AlienXdbKind> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("alien XDB path has no UTF-8 file stem")?;
    match stem.to_ascii_lowercase().as_str() {
        "amer" => Ok(AlienXdbKind::Amer),
        "croolis" => Ok(AlienXdbKind::Croolis),
        "scrut" => Ok(AlienXdbKind::Scrut),
        _ => bail!("alien XDB must be named AMER.XDB, CROOLIS.XDB, or SCRUT.XDB"),
    }
}

fn map_cursor_to_alien(cursor: CursorPosition, buttons: u16) -> AlienMouseSample {
    AlienMouseSample {
        x: (u32::from(cursor.x as u16) * ALIEN_DRIVER_WIDTH / ORIGINAL_SCREEN_WIDTH as u32) as u16,
        y: (u32::from(cursor.y as u16) * ALIEN_DRIVER_HEIGHT / ORIGINAL_SCREEN_HEIGHT as u32)
            as u16,
        buttons,
    }
}

fn map_horizontal_delta_to_original(
    output_width: f32,
    output_height: f32,
    horizontal_delta: f32,
) -> f32 {
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    horizontal_delta * ORIGINAL_SCREEN_WIDTH / viewport_width
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDESCREEN_WIDTH: f32 = 1_920.0;
    const WIDESCREEN_HEIGHT: f32 = 1_080.0;
    const WIDESCREEN_VIEWPORT_WIDTH: f32 = 1_440.0;
    const FIRST_DOUBLE_DIGIT_SECOND: u8 = 10;
    const LAST_CLOCK_SECOND: u8 = 59;
    const FIRST_DOUBLE_DIGIT_BCD: u8 = 0x10;
    const LAST_CLOCK_SECOND_BCD: u8 = 0x59;
    const FIRST_DAY_OF_1971: u64 = 365;
    const LEAP_DAY_2000: u64 = 11_016;

    #[test]
    fn original_cursor_maps_to_the_alien_driver_range_without_pointer_warping() {
        assert_eq!(
            map_cursor_to_alien(INITIAL_CURSOR, PointerButton::Primary as u16),
            AlienMouseSample {
                x: 320,
                y: 512,
                buttons: PointerButton::Primary as u16,
            }
        );
        assert_eq!(
            map_cursor_to_alien(
                CursorPosition { x: 0, y: 0 },
                PointerButton::Secondary as u16,
            ),
            AlienMouseSample {
                x: 0,
                y: 0,
                buttons: PointerButton::Secondary as u16,
            }
        );
    }

    #[test]
    fn bridge_relative_motion_scales_to_the_original_logical_width() {
        assert_eq!(
            map_horizontal_delta_to_original(
                WIDESCREEN_WIDTH,
                WIDESCREEN_HEIGHT,
                WIDESCREEN_VIEWPORT_WIDTH,
            ),
            ORIGINAL_SCREEN_WIDTH
        );
    }

    #[test]
    fn alien_overlay_kind_comes_from_its_authored_filename() {
        assert_eq!(
            alien_kind(Path::new("AMER.XDB")).unwrap(),
            AlienXdbKind::Amer
        );
        assert_eq!(
            alien_kind(Path::new("croolis.xdb")).unwrap(),
            AlienXdbKind::Croolis
        );
        assert!(alien_kind(Path::new("unknown.xdb")).is_err());
    }

    #[test]
    fn host_seconds_use_the_pc_clocks_packed_decimal_byte() {
        assert_eq!(pack_clock_seconds(u8::MIN), u8::MIN);
        assert_eq!(
            pack_clock_seconds(FIRST_DOUBLE_DIGIT_SECOND),
            FIRST_DOUBLE_DIGIT_BCD
        );
        assert_eq!(pack_clock_seconds(LAST_CLOCK_SECOND), LAST_CLOCK_SECOND_BCD);
    }

    #[test]
    fn unix_days_map_to_the_script_month_and_day() {
        assert_eq!(utc_month_day(u64::MIN), (1, 1));
        assert_eq!(utc_month_day(FIRST_DAY_OF_1971), (1, 1));
        assert_eq!(utc_month_day(LEAP_DAY_2000), (2, 29));
    }
}
