mod extract;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

/// Locate a directory that holds the game data (`DESCRIPT.DES` + `SCRIPT1.COD`).
/// Checked in order: the `CBLOOD_DATA` env var, then the usual extracted-ISO
/// locations in this tree. Used so a bare `cargo run` can launch the game
/// window without the caller having to spell out the data paths.
fn default_data_dir() -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("CBLOOD_DATA") {
        candidates.push(dir.into());
    }
    candidates.push("commander-blood-audio/_tmp_iso".into());
    candidates.push("output".into());
    candidates
        .into_iter()
        .find(|d| d.join("DESCRIPT.DES").is_file() && d.join("SCRIPT1.COD").is_file())
}

fn run() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        // Bare `cargo run` (no subcommand): launch the live game window using the
        // auto-detected data directory. Set CBLOOD_DATA to override the location.
        None => {
            let dir = default_data_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "no game data found (looked for DESCRIPT.DES + SCRIPT1.COD in \
                     $CBLOOD_DATA, commander-blood-audio/_tmp_iso, output).\n\
                     Run `cargo run -- engine-window <iso-dir> <asset-dir>` with an \
                     extracted ISO, or set CBLOOD_DATA to that directory."
                )
            })?;
            let dir = dir.to_string_lossy().into_owned();
            run_engine_window(&dir, &dir, "SCRIPT1")
        }
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

    // Scene background music (.voc, which ffmpeg reads directly), resolved via the
    // script's location→music the same way the video pipeline does, so the video isn't
    // silent. (The DESCRIPT music map is keyed by scene/location HNMs, not talk HNMs.)
    let music_voc = extract::script_background_music(Path::new(iso), script)
        .map(|m| format!("{assets}/mu/{m}.voc"))
        .filter(|p| Path::new(p).exists());

    let size = format!("{ENGINE_SCREEN_WIDTH}x{ENGINE_SCREEN_HEIGHT}");
    let mut args: Vec<String> = ["-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", &size,
        "-r", "15", "-i", "-"].iter().map(|s| s.to_string()).collect();
    if let Some(m) = &music_voc {
        args.push("-i".into());
        args.push(m.clone());
    }
    args.extend(["-c:v", "libx264", "-crf", "18", "-pix_fmt", "yuv420p"].iter().map(|s| s.to_string()));
    if music_voc.is_some() {
        args.extend(["-c:a", "aac", "-shortest"].iter().map(|s| s.to_string()));
    }
    args.push(out.to_string());
    if let Some(m) = &music_voc {
        eprintln!("scene music: {m}");
    }
    let mut ff = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = ff.stdin.take().unwrap();
    let mut rgb = vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3];
    // Length-scaled per-line hold means a fixed total is wrong: step until the dialogue
    // finishes, then hold the last line briefly, capped for safety.
    let cap = engine.dialogue_len() as u32 * 260 + 90;
    let mut done_at: Option<u32> = None;
    for i in 0..cap {
        engine.step(MouseInput::default());
        for (j, &idx) in engine.framebuffer.iter().enumerate() {
            let c = engine.scene_palette[idx as usize];
            rgb[j * 3..j * 3 + 3].copy_from_slice(&c);
        }
        stdin.write_all(&rgb)?;
        if engine.dialogue_finished() {
            let d = *done_at.get_or_insert(i);
            if i - d > 45 {
                break;
            }
        }
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
    // Parse every script's FULL decoded dialogue (all characters' speech events, ~3400
    // lines) so the engine can play the whole content, not just execute_trace's single
    // linear branch.
    let hnm_music = descript.hnm_music_map();
    let bundles = commander_blood_tools::script::parse_script_dir(iso, &descript, &hnm_music)
        .unwrap_or_default();
    let mut engine = EngineState::new();
    engine.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, Path::new(assets));
    engine.dialogue_hold_frames = 20;
    // The real star-map sprites: CARTE pyramids + BORXX orb for the nav view.
    if let (Ok(carte), Ok(borxx)) = (
        std::fs::read(format!("{iso}/CARTE.SPR")),
        std::fs::read(format!("{iso}/BORXX.SPR")),
    ) {
        engine.load_nav_sprites(&carte, &borxx);
    }
    // Show the decoded title/box art (BLOOD.LBM) first; a click or key dismisses it.
    engine.load_title(Path::new(iso));
    // Play the startup intro videos next (logos + intro cutscene), like the real game.
    // The DESCRIPT `present` record supplies the CRYO publisher-credit overlay.
    engine.load_intro(Path::new(assets), &descript);
    // The alien-examination screen (Scruter Jo): press 'c' to toggle it in nav.
    engine.load_alien_view(Path::new(assets), "scrut");
    // The comms "Hate TV" screen: press 't' to toggle, left/right to change channel.
    engine.load_tv_channels(Path::new(assets), "tv");
    // The cyberspace hyperspace-tunnel screen: press 'y' to toggle.
    engine.load_cyberspace(Path::new(assets));
    // The ship-bridge hub: press 'b' to open; click a station icon to enter its screen.
    engine.load_bridge(Path::new(iso));
    // The real navigation star-map background (CHART.FD) for the ship-nav view.
    engine.load_nav_chart(Path::new(iso));
    engine.load_console_font(Path::new(iso));
    engine.load_cryobox(Path::new(assets));
    // The console TELEPHONE option: the video-phone call screen (BAPPEL call widget +
    // the crew's talk-head HNMs as the live call feed).
    engine.load_telephone(Path::new(iso), Path::new(assets));
    // The choose-a-location nav list: the free-choice destinations (SCRIPT3/4/5), each
    // labelled by its first speaking character (the location's host) and carrying that
    // script's full decoded dialogue. Clicking one on the star-map visits that location.
    // (Index i in the list maps to SCRIPT<3+i>, so main.rs plays it with scene music.)
    let script_destination =
        |n: u32| -> Option<(String, Vec<(String, Option<std::path::PathBuf>)>)> {
            let bundle = bundles.iter().find(|bu| bu.script == format!("SCRIPT{n}"))?;
            let lines: Vec<(String, Option<std::path::PathBuf>)> = bundle
                .speech_events
                .iter()
                .filter(|e| !e.text.trim().is_empty())
                .map(|e| {
                    let scene = e
                        .background_hnm
                        .as_ref()
                        .map(|h| std::path::PathBuf::from(format!("{assets}/sq/{h}")))
                        .filter(|p| p.exists());
                    (e.text.clone(), scene)
                })
                .collect();
            if lines.is_empty() {
                return None;
            }
            let label = bundle
                .speech_events
                .iter()
                .find_map(|e| e.actor_record.clone())
                .unwrap_or_else(|| format!("SCRIPT{n}"))
                .to_uppercase();
            Some((label, lines))
        };
    engine.set_nav_destinations((3..=5).filter_map(script_destination).collect());
    // The boot reel's music (`mu/blintr.voc` — "BLood INTRo"): starts with the intro
    // and is stopped when the intro hands off to the game.
    let mut intro_music_started = false;
    // The game plays the SCRIPT1 console tutorial automatically once the intro ends (it
    // then chains to SCRIPT2 via its decoded D2 handoff, after which control returns to
    // the nav view for free destination choice). Fire it exactly once.
    let mut tutorial_played = false;
    // After the intro, start in the star-map nav view; the loop switches nav<->dialogue.
    engine.on_ship = true;
    // Scene music: the game plays each location's background music (.voc). Decoded
    // with our own VOC parser and played through cpal (cross-platform, in-process) —
    // best-effort, the engine stays silent without an output device.
    struct Music(Option<commander_blood_tools::audio::MusicPlayer>);
    impl Music {
        fn play(&mut self, voc_path: &str) {
            self.stop();
            self.0 = std::fs::read(voc_path)
                .ok()
                .and_then(|data| commander_blood_tools::snd::parse_voc_pcm(&data))
                .and_then(|(pcm, rate)| {
                    commander_blood_tools::audio::MusicPlayer::start(pcm, rate)
                });
        }
        fn stop(&mut self) {
            if let Some(mut p) = self.0.take() {
                p.stop();
            }
        }
    }
    let mut music = Music(None);

    // Per-line voice: when the dialogue line changes, play the speaker's SND clip
    // for that line (bank from the speaker's DESCRIPT record, clip index from the
    // A6 voice selector via the decoded one-based mapping). One voice at a time.
    let mut voice: Option<commander_blood_tools::audio::MusicPlayer> = None;
    let mut voice_line: Option<usize> = None;
    let mut snd_cache: std::collections::HashMap<std::path::PathBuf, commander_blood_tools::snd::SndBank> =
        std::collections::HashMap::new();

    // Subtitle chatter: the game plays sn/tb.snd clip 0 once per fully-revealed
    // subtitle line (@0x94BA). Track the reveal edge and fire it.
    let tb_snd = commander_blood_tools::snd::SndBank::read(
        std::path::Path::new(&format!("{assets}/sn/tb.snd")),
    )
    .ok();
    let mut chatter: Option<commander_blood_tools::audio::MusicPlayer> = None;
    let mut chatter_done_line: Option<usize> = None;

    // Load SCRIPT<n>'s dialogue into the engine (the destination's scene) and start
    // that scene's background music, as the game does per location.
    // The location/dialogue script the player is currently in (0 = none, on the nav) —
    // tracked here (the engine doesn't own it) so a save records where to resume. A Cell
    // lets the `load_script` closure update it through a shared borrow.
    let current_script = std::cell::Cell::new(0u32);
    let load_script = |engine: &mut EngineState, music: &mut Music, n: u32| {
        current_script.set(n);
        let r = |ext: &str| std::fs::read(format!("{iso}/SCRIPT{n}.{ext}"));
        if let (Ok(c), Ok(v), Ok(d), Ok(b)) = (r("COD"), r("VAR"), r("DIC"), r("DEB")) {
            // load_dialogue_scenes sets up the scene + the D2 chaining decision.
            engine.load_dialogue_scenes(&c, &v, &d, &b, &descript, Path::new(assets));
            // Then play the FULL decoded dialogue (every character's speech events, each
            // over its location background) instead of the single linear execute_trace path.
            if let Some(bundle) = bundles.iter().find(|bu| bu.script == format!("SCRIPT{n}")) {
                let lines: Vec<(String, Option<std::path::PathBuf>)> = bundle
                    .speech_events
                    .iter()
                    .filter(|e| !e.text.trim().is_empty())
                    .map(|e| {
                        let scene = e
                            .background_hnm
                            .as_ref()
                            .map(|h| std::path::PathBuf::from(format!("{assets}/sq/{h}")))
                            .filter(|p| p.exists());
                        (e.text.clone(), scene)
                    })
                    .collect();
                if !lines.is_empty() {
                    engine.set_speech_dialogue(lines);
                }
            }
            if let Some(m) = extract::script_background_music(Path::new(iso), &format!("SCRIPT{n}"))
            {
                let voc = format!("{assets}/mu/{m}.voc");
                if Path::new(&voc).exists() {
                    music.play(&voc);
                }
            }
        }
    };

    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| anyhow::anyhow!("X11 connect: {e}"))?;
    let screen = &conn.setup().roots[screen_num];
    // Source is the 320x200 engine framebuffer; the window is larger and resizable,
    // with the framebuffer scaled to fit while preserving the 320:200 (8:5) aspect.
    let (src_w, src_h) = (ENGINE_SCREEN_WIDTH, ENGINE_SCREEN_HEIGHT);
    let (mut win_w, mut win_h) = (960u16, 600u16); // 3x, aspect-correct
    let win = conn.generate_id()?;
    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        win_w,
        win_h,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new().event_mask(
            EventMask::EXPOSURE
                | EventMask::POINTER_MOTION
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::KEY_PRESS
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

    // 4 bytes/pixel Z-pixmap (little-endian BGRX for the common depth-24 visual),
    // sized to the (resizable) window.
    let mut image = vec![0u8; win_w as usize * win_h as usize * 4];
    let (mut mx, mut my, mut buttons) = (0u16, 0u16, 0u16);
    let mut clicked = false;
    let mut frames_since_input = 0u32;
    // Conservative X11 request-size cap (safe even without the big-requests extension);
    // put_image is chunked into row-strips under this.
    let max_req = 262_144usize;
    loop {
        // Aspect-preserving fit: largest integer scale of 320x200 that fits the window,
        // centered (letterboxed). Used for both drawing and mouse-coord mapping.
        let scale = ((win_w as usize / src_w).min(win_h as usize / src_h)).max(1);
        let (dst_w, dst_h) = (src_w * scale, src_h * scale);
        let off_x = (win_w as usize).saturating_sub(dst_w) / 2;
        let off_y = (win_h as usize).saturating_sub(dst_h) / 2;
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::MotionNotify(m) => {
                    // Map window coords back through the letterbox+scale to source pixels.
                    let ex = (m.event_x as isize - off_x as isize)
                        .clamp(0, dst_w as isize - 1) as usize;
                    let ey = (m.event_y as isize - off_y as isize)
                        .clamp(0, dst_h as isize - 1) as usize;
                    mx = (ex / scale).min(src_w - 1) as u16;
                    my = (ey / scale).min(src_h - 1) as u16;
                }
                // Any click/key dismisses the title screen first (advance to the intro).
                Event::ButtonPress(_) | Event::KeyPress(_) if engine.title_active() => {
                    engine.dismiss_title();
                }
                // On the bridge hub, a click on a station icon opens its screen.
                Event::ButtonPress(b) if engine.bridge_active && b.detail == 1 => {
                    // The ship-console menu (decoded console -> VM object dispatch):
                    // HONK (0) opens the cook's daily-fare menu (SCRIPT1); TELEPHONE (1)
                    // the video-phone; CRYOBOX (2) the cryo-chamber. MENU/OPTION not decoded.
                    match engine.console_menu_click(mx, my) {
                        Some(0) => {
                            engine.bridge_active = false;
                            load_script(&mut engine, &mut music, 1);
                            engine.on_ship = false;
                        }
                        Some(1) => {
                            engine.bridge_active = false;
                            engine.phone_active = true;
                        }
                        Some(2) => {
                            engine.bridge_active = false;
                            engine.cryobox_active = true;
                        }
                        Some(_) => {} // MENU/OPTION: functions not yet decoded
                        None => {
                            if let Some(station) = engine.bridge_click(mx, my) {
                                engine.bridge_active = false;
                                use commander_blood_tools::engine::BridgeStation::*;
                                match station {
                                    Nav => engine.on_ship = true,
                                    Comms => engine.tv_active = true,
                                    Cyberspace => engine.cyber_active = true,
                                }
                            }
                        }
                    }
                }
                // On the TV screen, left/right buttons change channel (must precede the
                // generic nav-button handlers below).
                Event::ButtonPress(b) if engine.tv_active && (b.detail == 1 || b.detail == 3) => {
                    engine.switch_tv_channel(if b.detail == 1 { 1 } else { -1 });
                }
                // On the video-phone: a left click on a contact connects the call (dialling)
                // or hangs it up (connected); the right button cycles the dialled contact.
                Event::ButtonPress(b) if engine.phone_active && b.detail == 1 => {
                    if engine.phone_connected() {
                        engine.phone_hangup();
                    } else if let Some(i) = engine.phone_contact_click(mx, my) {
                        engine.phone_connect(i);
                    }
                }
                Event::ButtonPress(b) if engine.phone_active && b.detail == 3 => {
                    engine.phone_cycle_contact(1);
                }
                // On the nav star-map, a left click on a destination in the
                // choose-a-location list visits it (loads SCRIPT<3+i> — that location's
                // character dialogue with its scene music).
                Event::ButtonPress(b)
                    if b.detail == 1
                        && engine.nav_view_active()
                        && engine.nav_destination_click(mx, my).is_some() =>
                {
                    if let Some(i) = engine.nav_destination_click(mx, my) {
                        load_script(&mut engine, &mut music, 3 + i as u32);
                        engine.on_ship = false;
                    }
                }
                // Left button otherwise drives compass nav selection (via the engine);
                // right button is a manual nav<->dialogue view toggle for convenience.
                Event::ButtonPress(b) if b.detail == 1 => {
                    buttons = 1;
                    clicked = true; // latch so a fast press+release still reaches step()
                }
                Event::ButtonPress(b) if b.detail == 3 => engine.on_ship = !engine.on_ship,
                // Keyboard loop controls: Escape (keycode 9) returns to the nav view.
                Event::KeyPress(k) if k.detail == 9 => {
                    // A visited world-location screen closes back to the nav view first.
                    if engine.world_location_active() {
                        engine.leave_world();
                    } else if engine.phone_active && engine.phone_connected() {
                        // A connected call hangs up first, back to the phone's dial screen.
                        engine.phone_hangup();
                    } else {
                        engine.alien_view_active = false;
                        engine.tv_active = false;
                        engine.cyber_active = false;
                        engine.cryobox_active = false;
                        engine.phone_active = false;
                        // Esc from a screen returns to the bridge hub.
                        engine.bridge_active = true;
                    }
                }
                // F5 (keycode 71): save the game to blood.sav (the port's save format).
                Event::KeyPress(k) if k.detail == 71 => {
                    let save = engine.capture_save(current_script.get());
                    match save.write(Path::new("blood.sav")) {
                        Ok(()) => println!(
                            "saved blood.sav ({:?}, script {}, line {})",
                            save.screen, save.script, save.dialogue_cursor
                        ),
                        Err(e) => eprintln!("save failed: {e}"),
                    }
                }
                // F9 (keycode 75): load blood.sav and resume that saved state.
                Event::KeyPress(k) if k.detail == 75 => {
                    use commander_blood_tools::save::{SaveScreen, SaveState};
                    if let Some(save) = SaveState::read(Path::new("blood.sav")) {
                        // If the save was mid-dialogue, reload that location's script first
                        // so the resumed cursor lands on a real line; then apply the view.
                        if save.screen == SaveScreen::Dialogue && save.script != 0 {
                            load_script(&mut engine, &mut music, save.script);
                        }
                        engine.restore_save(&save);
                        println!(
                            "loaded blood.sav ({:?}, script {}, line {})",
                            save.screen, save.script, save.dialogue_cursor
                        );
                    } else {
                        eprintln!("no valid blood.sav to load");
                    }
                }
                // 'v' (keycode 55): visit the nav destination the compass targets —
                // shows that world's decoded fd/ location background. While visiting,
                // 'v' cycles forward through the world's rooms.
                Event::KeyPress(k) if k.detail == 55 => {
                    if engine.world_location_active() {
                        engine.cycle_world_room(1);
                    } else if let Some(world) = engine.targeted_world_name() {
                        if engine.visit_world(world, Path::new(assets)) {
                            // Overlay the world's decoded .ext object positions (from the ISO).
                            if let Ok(ext) = std::fs::read(format!("{iso}/{}.EXT", world.to_uppercase())) {
                                engine.set_world_ext(&ext);
                            }
                        }
                    }
                }
                // 'c' (keycode 54): toggle the alien-examination screen (plays the
                // scrutinizer intro on entry).
                Event::KeyPress(k) if k.detail == 54 => {
                    engine.alien_view_active = !engine.alien_view_active;
                    if engine.alien_view_active {
                        engine.arm_alien_intro();
                    }
                }
                // 't' (keycode 28): toggle the comms/TV screen.
                Event::KeyPress(k) if k.detail == 28 => {
                    engine.tv_active = !engine.tv_active;
                }
                // 'y' (keycode 29): toggle the cyberspace tunnel screen.
                Event::KeyPress(k) if k.detail == 29 => {
                    engine.cyber_active = !engine.cyber_active;
                }
                // 'b' (keycode 56): open the ship-bridge hub (click stations to enter).
                Event::KeyPress(k) if k.detail == 56 => {
                    engine.alien_view_active = false;
                    engine.tv_active = false;
                    engine.cyber_active = false;
                    engine.bridge_active = !engine.bridge_active;
                }
                Event::ButtonRelease(b) if b.detail == 1 => buttons = 0,
                // Window resized: track the new size and re-alloc the image buffer.
                Event::ConfigureNotify(c) => {
                    if c.width > 0 && c.height > 0 && (c.width != win_w || c.height != win_h) {
                        win_w = c.width;
                        win_h = c.height;
                        image = vec![0u8; win_w as usize * win_h as usize * 4];
                    }
                }
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
        // Suppressed while the startup intro is still playing.
        if engine.intro_active() {
            // Intro playing: start its music once; no nav/dialogue transitions yet.
            if !intro_music_started {
                intro_music_started = true;
                music.play(&format!("{assets}/mu/blintr.voc"));
            }
        } else if !tutorial_played {
            // Intro just ended: silence the reel music, then start the SCRIPT1 tutorial
            // automatically (the game's flow — it chains to SCRIPT2 via D2, then frees the
            // nav for SCRIPT3/4/5).
            tutorial_played = true;
            if intro_music_started {
                intro_music_started = false;
                music.stop();
            }
            load_script(&mut engine, &mut music, 1);
            engine.on_ship = false;
        } else if let Some(heading) = engine.take_nav_selection() {
            // SCRIPT1/2 are the forced tutorial + first encounter (played after the intro
            // and chained). The nav offers the free-choice destinations: SCRIPT3/4/5.
            let dest = (heading as u32 * 3 / 180).clamp(0, 2) + 3; // heading → SCRIPT3..5
            load_script(&mut engine, &mut music, dest);
            engine.on_ship = false;
        } else if !engine.on_ship && engine.dialogue_finished() {
            // Scene finished: follow the decoded D2 handoff if the script requested a
            // successor profile (profile_index → SCRIPT<index+1>), chaining scene to
            // scene like the game; otherwise return to the nav view.
            match engine.pending_next_scene() {
                Some(profile) => {
                    voice = None;
                    voice_line = None;
                    chatter_done_line = None;
                    load_script(&mut engine, &mut music, u32::from(profile) + 1);
                }
                None => {
                    engine.on_ship = true;
                    music.stop();
                    voice = None;
                    voice_line = None;
                    current_script.set(0); // back on the nav — no active location
                }
            }
        }
        // Speak the current line once when playback reaches it.
        if !engine.on_ship && voice_line != Some(engine.dialogue_cursor()) {
            voice_line = Some(engine.dialogue_cursor());
            voice = None;
            if let Some((bank_path, selector)) = engine.current_voice() {
                let bank = snd_cache.entry(bank_path.clone()).or_insert_with(|| {
                    commander_blood_tools::snd::SndBank::read(&bank_path).unwrap_or_else(|_| {
                        commander_blood_tools::snd::SndBank::parse(&[0, 0, 0, 0, 0, 0])
                            .expect("empty bank")
                    })
                });
                if let Some(idx) = commander_blood_tools::vm::text_selector_voice_clip_index(
                    selector,
                    bank.clip_count(),
                ) {
                    if let Some(clip) = bank.clip(idx) {
                        voice = commander_blood_tools::audio::MusicPlayer::start_once(
                            clip.pcm.clone(),
                            clip.sample_rate,
                        );
                    }
                }
            }
        }
        let _ = &voice; // keep the stream alive while the line plays
        // Subtitle chatter: when the current line finishes revealing, play tb.snd
        // clip 0 once (per decoded @0x94BA behaviour).
        if !engine.on_ship {
            let line = engine.dialogue_cursor();
            if let Some((revealed, total)) = engine.subtitle_reveal_progress() {
                if revealed >= total && chatter_done_line != Some(line) {
                    chatter_done_line = Some(line);
                    if let Some(bank) = &tb_snd {
                        if let Some(clip) = bank.clip(0).filter(|c| !c.pcm.is_empty()) {
                            chatter = commander_blood_tools::audio::MusicPlayer::start_once(
                                clip.pcm.clone(),
                                clip.sample_rate,
                            );
                        }
                    }
                }
            }
        }
        let _ = &chatter;
        // Clear the whole window (letterbox borders + a full erase so nothing from the
        // previous frame can bleed through), then scale the framebuffer in.
        for b in image.iter_mut() {
            *b = 0;
        }
        let stride = win_w as usize * 4;
        for sy in 0..src_h {
            let src_row = sy * src_w;
            for row in 0..scale {
                let dy = off_y + sy * scale + row;
                if dy >= win_h as usize {
                    break;
                }
                let mut di = dy * stride + off_x * 4;
                for sx in 0..src_w {
                    let c = engine.scene_palette[engine.framebuffer[src_row + sx] as usize];
                    for _ in 0..scale {
                        if di + 2 < image.len() {
                            image[di] = c[2];
                            image[di + 1] = c[1];
                            image[di + 2] = c[0];
                        }
                        di += 4;
                    }
                }
            }
        }
        // put_image is one request; chunk by row-strips so a large window stays under
        // the server's maximum request size.
        let row_bytes = win_w as usize * 4;
        let max_rows = (max_req.saturating_sub(64) / row_bytes.max(1)).max(1);
        let mut y = 0usize;
        while y < win_h as usize {
            let rows = max_rows.min(win_h as usize - y);
            let start = y * row_bytes;
            conn.put_image(
                ImageFormat::Z_PIXMAP,
                win,
                gc,
                win_w,
                rows as u16,
                0,
                y as i16,
                0,
                screen.root_depth,
                &image[start..start + rows * row_bytes],
            )?;
            y += rows;
        }
        conn.flush()?;
        std::thread::sleep(Duration::from_millis(66));
        // Headless-safety: exit after a bounded run if no display consumer.
        frames_since_input += 1;
        if frames_since_input > 100_000 {
            return Ok(());
        }
    }
}
