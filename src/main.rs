mod extract;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("inspect-bloodprg") => {
            let path = args
                .next()
                .unwrap_or_else(|| "re/bin/BLOODPRG.EXE".to_string());
            let binary = commander_blood_tools::bloodprg::BloodPrg::parse_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&binary.inspect()?)?);
            Ok(())
        }
        Some("inspect-vm") => {
            #[derive(serde::Serialize)]
            struct VmInspection {
                tokens: Vec<commander_blood_tools::vm::VmToken>,
                line_states: Option<Vec<commander_blood_tools::vm::LineState>>,
                execution_trace: Option<commander_blood_tools::vm::ExecutionTrace>,
            }

            let cod_path = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-vm <SCRIPT.COD> [SCRIPT.VAR]"))?;
            let cod = std::fs::read(&cod_path)?;
            let tokens = commander_blood_tools::vm::walk(&cod, 0, cod.len());
            let var = args.next().map(std::fs::read).transpose()?;
            let line_states = var
                .as_ref()
                .map(|var| commander_blood_tools::vm::interpret_line_states(&cod, var));
            let execution_trace = var
                .as_ref()
                .map(|var| commander_blood_tools::vm::execute_trace(&cod, var));
            println!(
                "{}",
                serde_json::to_string_pretty(&VmInspection {
                    tokens,
                    line_states,
                    execution_trace,
                })?
            );
            Ok(())
        }
        Some("inspect-descript") => {
            let path = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-descript <DESCRIPT.DES>"))?;
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&db)?);
            Ok(())
        }
        Some("inspect-scripts") => {
            let iso_dir = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: inspect-scripts <iso-dir>"))?;
            let descript_path = std::path::Path::new(&iso_dir).join("DESCRIPT.DES");
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let hnm_music = db.hnm_music_map();
            let bundles =
                commander_blood_tools::script::parse_script_dir(&iso_dir, &db, &hnm_music)?;
            println!("{}", serde_json::to_string_pretty(&bundles)?);
            Ok(())
        }
        Some("inspect-character-combinations") => {
            let iso_dir = args.next().ok_or_else(|| {
                anyhow::anyhow!("usage: inspect-character-combinations <iso-dir>")
            })?;
            let descript_path = std::path::Path::new(&iso_dir).join("DESCRIPT.DES");
            let db = commander_blood_tools::descript::DescriptDb::parse_file(&descript_path)?;
            let hnm_music = db.hnm_music_map();
            let bundles =
                commander_blood_tools::script::parse_script_dir(&iso_dir, &db, &hnm_music)?;

            println!(
                "script\tactor\tactor_object_offset\tactor_talk_ref\tlocation_record\tbackground_hnm\tbackground_music\tsource"
            );
            for bundle in bundles {
                for context in bundle.character_contexts {
                    println!(
                        "{}\t{}\t0x{:04x}\t0x{:04x}\t{}\t{}\t{}\t{}",
                        context.script,
                        context.actor_record,
                        context.actor_object_offset,
                        context.actor_talk_ref,
                        context.location_record.as_deref().unwrap_or(""),
                        context.background_hnm.as_deref().unwrap_or(""),
                        context.background_music.as_deref().unwrap_or(""),
                        context.source
                    );
                }
            }
            Ok(())
        }
        Some("engine-play") => {
            let iso = args.next().ok_or_else(|| {
                anyhow::anyhow!("usage: engine-play <iso-dir> <asset-dir> <out.mp4> [SCRIPTn]")
            })?;
            let assets = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing asset-dir"))?;
            let out = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing out.mp4"))?;
            let script = args.next().unwrap_or_else(|| "SCRIPT1".to_string());
            run_engine_play(&iso, &assets, &out, &script)
        }
        Some("engine-window") => {
            let iso = args.next().ok_or_else(|| {
                anyhow::anyhow!("usage: engine-window <iso-dir> <asset-dir> [SCRIPTn]")
            })?;
            let assets = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("missing asset-dir"))?;
            let script = args.next().unwrap_or_else(|| "SCRIPT1".to_string());
            run_engine_window(&iso, &assets, &script)
        }
        _ => extract::run().map_err(|err| anyhow::anyhow!("{err}")),
    }
}

/// Headless real-time engine driver: run the runnable engine (`EngineState`) over a
/// script's dialogue scene flow and encode each stepped frame to an MP4 — the
/// engine playing the game, produced without a graphics window (the windowed
/// backend layers the same loop onto a display).
fn run_engine_play(iso: &str, assets: &str, out: &str, script: &str) -> anyhow::Result<()> {
    use commander_blood_tools::engine::{
        ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH, EngineState, MouseInput,
    };
    use std::io::Write;
    use std::path::Path;
    use std::process::{Command, Stdio};

    let rd = |ext: &str| std::fs::read(format!("{iso}/{script}.{ext}"));
    let (cod, var, dic, deb) = (rd("COD")?, rd("VAR")?, rd("DIC")?, rd("DEB")?);
    let descript =
        commander_blood_tools::descript::DescriptDb::parse_file(format!("{iso}/DESCRIPT.DES"))?;

    let mut engine = EngineState::new();
    engine.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, Path::new(assets));
    engine.dialogue_hold_frames = 20; // ~1.3s per line at 15 fps

    let total = (engine.dialogue_len().max(1) as u32) * engine.dialogue_hold_frames + 30;
    let mut ff = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            &format!("{ENGINE_SCREEN_WIDTH}x{ENGINE_SCREEN_HEIGHT}"),
            "-r",
            "15",
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            out,
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = ff.stdin.take().unwrap();
    let mut rgb = vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3];
    for _ in 0..total {
        engine.step(MouseInput::default());
        for (i, &idx) in engine.framebuffer.iter().enumerate() {
            let c = engine.scene_palette[idx as usize];
            rgb[i * 3..i * 3 + 3].copy_from_slice(&c);
        }
        stdin.write_all(&rgb)?;
    }
    drop(stdin);
    ff.wait()?;
    eprintln!("engine played {} lines -> {out}", engine.dialogue_len());
    Ok(())
}

/// Real-time windowed backend: present the runnable engine in a live window and
/// drive it with real mouse input (position steers the ship-nav compass; left
/// button toggles the on-ship view vs the dialogue scene). This is the interactive
/// presentation layer (winit + softbuffer, which runs under a virtual framebuffer)
/// over the same [`EngineState::step`] loop the headless `engine-play` driver uses.
fn run_engine_window(iso: &str, assets: &str, script: &str) -> anyhow::Result<()> {
    use commander_blood_tools::engine::{
        ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH, EngineState, MouseInput,
    };
    use std::num::NonZeroU32;
    use std::path::Path;
    use std::rc::Rc;
    use winit::dpi::LogicalSize;
    use winit::event::{ElementState, Event, MouseButton, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    let rd = |ext: &str| std::fs::read(format!("{iso}/{script}.{ext}"));
    let (cod, var, dic, deb) = (rd("COD")?, rd("VAR")?, rd("DIC")?, rd("DEB")?);
    let descript =
        commander_blood_tools::descript::DescriptDb::parse_file(format!("{iso}/DESCRIPT.DES"))?;

    let mut engine = EngineState::new();
    engine.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, Path::new(assets));
    engine.dialogue_hold_frames = 30;

    let event_loop = EventLoop::new()?;
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Commander Blood — engine")
            .with_inner_size(LogicalSize::new(
                (ENGINE_SCREEN_WIDTH * 2) as u32,
                (ENGINE_SCREEN_HEIGHT * 2) as u32,
            ))
            .build(&event_loop)?,
    );
    let context = softbuffer::Context::new(window.clone())
        .map_err(|e| anyhow::anyhow!("softbuffer context: {e}"))?;
    let mut surface = softbuffer::Surface::new(&context, window.clone())
        .map_err(|e| anyhow::anyhow!("softbuffer surface: {e}"))?;

    let (mut mx, mut my, mut buttons) = (0u16, 0u16, 0u16);
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                let size = window.inner_size();
                mx = ((position.x / size.width.max(1) as f64) * ENGINE_SCREEN_WIDTH as f64) as u16;
                my =
                    ((position.y / size.height.max(1) as f64) * ENGINE_SCREEN_HEIGHT as f64) as u16;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let down = state == ElementState::Pressed;
                    buttons = u16::from(down);
                    if down {
                        engine.on_ship = !engine.on_ship;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                engine.step(MouseInput {
                    x: mx,
                    y: my,
                    buttons,
                });
                let size = window.inner_size();
                let (w, h) = (size.width.max(1), size.height.max(1));
                surface
                    .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
                    .ok();
                if let Ok(mut buf) = surface.buffer_mut() {
                    // Nearest-neighbour scale the 320x200 indexed framebuffer to the
                    // window and pack ARGB.
                    for y in 0..h {
                        let sy = (y as usize * ENGINE_SCREEN_HEIGHT / h as usize)
                            .min(ENGINE_SCREEN_HEIGHT - 1);
                        for x in 0..w {
                            let sx = (x as usize * ENGINE_SCREEN_WIDTH / w as usize)
                                .min(ENGINE_SCREEN_WIDTH - 1);
                            let idx = engine.framebuffer[sy * ENGINE_SCREEN_WIDTH + sx];
                            let c = engine.scene_palette[idx as usize];
                            buf[(y * w + x) as usize] =
                                ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
                        }
                    }
                    buf.present().ok();
                }
                window.request_redraw();
            }
            _ => {}
        },
        _ => {}
    })?;
    Ok(())
}
