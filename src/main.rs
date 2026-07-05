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

/// Real-time windowed backend: present the runnable engine in a live X11 window and
/// drive it with real mouse input (position steers the ship-nav compass; left button
/// toggles the on-ship view vs the dialogue scene). Uses raw X11 (x11rb) so it runs
/// on any X server, including a virtual framebuffer (Xvfb) — the interactive
/// presentation layer over the same `EngineState::step` loop `engine-play` uses.
fn run_engine_window(iso: &str, assets: &str, script: &str) -> anyhow::Result<()> {
    use commander_blood_tools::engine::{
        ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH, EngineState, MouseInput,
    };
    use std::path::Path;
    use std::time::Duration;
    use x11rb::connection::Connection;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        AtomEnum, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, ImageFormat, PropMode,
        WindowClass,
    };
    use x11rb::wrapper::ConnectionExt as _;

    let rd = |ext: &str| std::fs::read(format!("{iso}/{script}.{ext}"));
    let (cod, var, dic, deb) = (rd("COD")?, rd("VAR")?, rd("DIC")?, rd("DEB")?);
    let descript =
        commander_blood_tools::descript::DescriptDb::parse_file(format!("{iso}/DESCRIPT.DES"))?;
    let mut engine = EngineState::new();
    engine.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, Path::new(assets));
    engine.dialogue_hold_frames = 20;
    // Start in the star-map nav view; the playable loop below switches nav<->dialogue.
    engine.on_ship = true;
    // Load SCRIPT<n>'s dialogue into the engine (the destination's scene).
    let load_script = |engine: &mut EngineState, n: u32| {
        let r = |ext: &str| std::fs::read(format!("{iso}/SCRIPT{n}.{ext}"));
        if let (Ok(c), Ok(v), Ok(d), Ok(b)) = (r("COD"), r("VAR"), r("DIC"), r("DEB")) {
            engine.load_dialogue_scenes(&c, &v, &d, &b, &descript, Path::new(assets));
        }
    };

    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| anyhow::anyhow!("X11 connect: {e}"))?;
    let screen = &conn.setup().roots[screen_num];
    let (w, h) = (ENGINE_SCREEN_WIDTH as u16, ENGINE_SCREEN_HEIGHT as u16);
    let win = conn.generate_id()?;
    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        w,
        h,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new().event_mask(
            EventMask::EXPOSURE
                | EventMask::POINTER_MOTION
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::STRUCTURE_NOTIFY,
        ),
    )?;
    conn.change_property8(
        PropMode::REPLACE,
        win,
        u32::from(AtomEnum::WM_NAME),
        u32::from(AtomEnum::STRING),
        b"Commander Blood - engine",
    )?;
    conn.map_window(win)?;
    let gc = conn.generate_id()?;
    conn.create_gc(gc, win, &CreateGCAux::new())?;
    conn.flush()?;

    // 4 bytes/pixel Z-pixmap (little-endian BGRX for the common depth-24 visual).
    let mut image = vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 4];
    let (mut mx, mut my, mut buttons) = (0u16, 0u16, 0u16);
    let mut clicked = false;
    let mut frames_since_input = 0u32;
    loop {
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::MotionNotify(m) => {
                    mx = m.event_x.clamp(0, w as i16 - 1) as u16;
                    my = m.event_y.clamp(0, h as i16 - 1) as u16;
                }
                // Left button drives nav selection (via the engine); right button is a
                // manual nav<->dialogue view toggle for convenience.
                Event::ButtonPress(b) if b.detail == 1 => {
                    buttons = 1;
                    clicked = true; // latch so a fast press+release still reaches step()
                }
                Event::ButtonPress(b) if b.detail == 3 => engine.on_ship = !engine.on_ship,
                Event::ButtonRelease(b) if b.detail == 1 => buttons = 0,
                Event::DestroyNotify(_) => return Ok(()),
                _ => {}
            }
        }
        // A click that arrived and released within one frame still presents as pressed
        // for this step, so the engine's edge-triggered nav select fires.
        let step_buttons = if clicked { 1 } else { buttons };
        clicked = false;
        engine.step(MouseInput {
            x: mx,
            y: my,
            buttons: step_buttons,
        });
        // Playable loop: a nav-view click commits a destination → load its dialogue and
        // switch to the scene; when the dialogue finishes, return to the nav view.
        if let Some(heading) = engine.take_nav_selection() {
            let dest = (heading as u32 * 5 / 180).clamp(0, 4) + 1; // heading → SCRIPT1..5
            load_script(&mut engine, dest);
            engine.on_ship = false;
        } else if !engine.on_ship && engine.dialogue_finished() {
            engine.on_ship = true;
        }
        for (i, &idx) in engine.framebuffer.iter().enumerate() {
            let c = engine.scene_palette[idx as usize];
            image[i * 4] = c[2];
            image[i * 4 + 1] = c[1];
            image[i * 4 + 2] = c[0];
        }
        conn.put_image(
            ImageFormat::Z_PIXMAP,
            win,
            gc,
            w,
            h,
            0,
            0,
            0,
            screen.root_depth,
            &image,
        )?;
        conn.flush()?;
        std::thread::sleep(Duration::from_millis(66));
        // Headless-safety: exit after a bounded run if no display consumer.
        frames_since_input += 1;
        if frames_since_input > 100_000 {
            return Ok(());
        }
    }
}
