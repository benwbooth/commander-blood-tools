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

/// Resolve a dialogue-scene HNM by name across the asset subdirs. Location backgrounds
/// live in `pl/` (e.g. `pterra10.hnm`), cutscenes in `sq/`, character talk-heads in `pe/`,
/// objects in `ob/` — the DEB's `background_hnm` names don't carry the dir, so search them
/// all and return the first that exists.
fn resolve_scene_hnm(assets: &str, h: &str) -> Option<std::path::PathBuf> {
    for sub in ["pl", "sq", "pe", "ob"] {
        let p = std::path::PathBuf::from(format!("{assets}/{sub}/{h}"));
        if p.exists() {
            return Some(p);
        }
    }
    None
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
        Some("decompile-scripts") => {
            // Static decompilation of every SCRIPTn.COD into a readable BASIC-like
            // listing (decompiled/SCRIPTn.bas) using the faithfully-decoded VM
            // semantics — the human-readable form of the game's script logic.
            let iso_dir = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("usage: decompile-scripts <iso-dir> [out-dir]"))?;
            let out_dir = args.next().unwrap_or_else(|| "decompiled".to_string());
            std::fs::create_dir_all(&out_dir)?;
            for n in 1..=5u32 {
                let cod = match std::fs::read(format!("{iso_dir}/SCRIPT{n}.COD")) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let dic_raw = std::fs::read(format!("{iso_dir}/SCRIPT{n}.DIC"))?;
                let dic = commander_blood_tools::script::parse_dictionary(&dic_raw);
                let deb = std::fs::read(format!("{iso_dir}/SCRIPT{n}.DEB")).unwrap_or_default();
                let names = commander_blood_tools::engine::deb_actor_name_map(&deb);
                let listing = commander_blood_tools::vm::decompile_script(&cod, &dic, &names);
                let out = format!("{out_dir}/SCRIPT{n}.bas");
                std::fs::write(&out, &listing)?;
                println!("wrote {out} ({} lines)", listing.lines().count());
            }
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
    use commander_blood_tools::concept_menu;
    use commander_blood_tools::engine::{
        ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH, EngineState, MouseInput,
    };
    use std::path::Path;
    use std::time::Duration;
    use x11rb::connection::Connection;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConnectionExt, CreateGCAux, CreateWindowAux,
        EventMask, GrabMode, ImageFormat, PropMode, WindowClass,
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
    // Boot straight into the intro logos + cutscene, exactly as the real game does
    // (MINDSCAPE → Microfolie's → intro cutscene → CRYO credit → crew showcase) — NOT a
    // static box-art title screen (the real game has none; `BLOOD.LBM` box art isn't part
    // of the boot). The DESCRIPT `present` record supplies the CRYO publisher-credit overlay.
    engine.load_intro(Path::new(assets), &descript);
    // The alien-examination screen (Scruter Jo): press 'c' to toggle it in nav.
    engine.load_alien_view(Path::new(assets), "scrut");
    // The comms "Hate TV" screen: press 't' to toggle, left/right to change channel.
    // The real PROGRAMMING comes from the DESCRIPT broadcast records (hatetv / microkid,
    // self-identified by their "…watching…" subtitles) — chained clips + music + cues;
    // the raw tv* HNMs remain as a fallback when the records' clips are missing.
    engine.load_tv_programs(&descript, Path::new(assets));
    engine.load_tv_channels(Path::new(assets), "tv");
    // The cyberspace hyperspace-tunnel screen: press 'y' to toggle.
    engine.load_cyberspace(Path::new(assets));
    // The ship bridge: the TB.BIG panorama + the captured pointing-hand cursor
    // (real-renderer output; regenerate with runtime_boot BRIDGEPROBE HANDATLAS).
    engine.load_bridge(Path::new(iso));
    engine.load_hand_atlas(Path::new("accuracy/captures/bridge/hand"));
    // The real navigation star-map background (CHART.FD) for the ship-nav view.
    engine.load_nav_chart(Path::new(iso));

    engine.load_console_font(Path::new(iso));
    engine.load_cryobox(Path::new(assets));
    // The console TELEPHONE option: the video-phone call screen (BAPPEL call widget +
    // the crew's talk-head HNMs as the live call feed).
    engine.load_telephone(Path::new(iso), Path::new(assets));
    // The game-ending finale (sq/fin.hnm) — plays once the player has visited every
    // free-choice destination (the bookend to the intro).
    engine.load_ending(Path::new(assets));
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
                        .and_then(|h| resolve_scene_hnm(assets, h));
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
    // The intro music is tied to a specific clip by the DESCRIPT data (the `present` record's
    // Music plays with its cliptoot.hnm cinematic, NOT the logo reel) — so we start each clip's
    // music when the clip BEGINS and keep the logos silent. Track the last clip we started music
    // for so a given clip's music fires exactly once.
    let mut intro_music_clip: Option<usize> = None;
    // The TV channel's broadcast music (hatetv.voc / balise.voc, from the channel's DESCRIPT
    // record): one per-frame watcher covers every open/close/switch path.
    let mut tv_music_playing: Option<String> = None;
    // Whether the nav target-list music (`mu\tablo2.voc`, decoded handler-4 toggle @0x886C)
    // is currently on.
    let mut nav_music_on = false;
    // The game plays the SCRIPT1 console tutorial automatically once the intro ends (it
    // then chains to SCRIPT2 via its decoded D2 handoff, after which control returns to
    // the nav view for free destination choice). Fire it exactly once.
    let mut tutorial_played = false;
    // The free-choice destinations (SCRIPT3/4/5) drive completion: visiting all of them
    // plays the ending finale. Tracked via the decoded entity state machine
    // (engine.progress), which registers each and marks it visited.
    let free_choice_scripts: [u32; 3] = [3, 4, 5];
    for n in free_choice_scripts {
        engine.progress.register(&format!("SCRIPT{n}"), n as u16);
    }
    let mut ending_started = false;
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
    // Chatter burble state (decoded @0xB898): 4-tick throttle ([0xB2F]), last random pick
    // ([0xC4D], never repeated back-to-back), and a simple LCG for the roll.
    let mut chatter_throttle: u32 = 0;
    let mut chatter_prev: Option<u32> = None;
    let mut chatter_seed: u32 = 0x1234_5678;

    // Load SCRIPT<n>'s dialogue into the engine (the destination's scene) and start
    // that scene's background music, as the game does per location.
    // The location/dialogue script the player is currently in (0 = none, on the nav) —
    // tracked here (the engine doesn't own it) so a save records where to resume. A Cell
    // lets the `load_script` closure update it through a shared borrow.
    let current_script = std::cell::Cell::new(0u32);
    // The FAITHFUL script VM (SCRIPT1 wired; see src/vm.rs VmMachine): the engine
    // plays exactly the lines the game's own bytecode emits — console button clicks
    // start actor presentations, and the script's self-modifying state drives
    // rotation/progression. Shared with the load_script closure via RefCell.
    let script_vm: std::cell::RefCell<Option<commander_blood_tools::vm::VmMachine>> =
        std::cell::RefCell::new(None);
    let vm_lines: std::cell::RefCell<
        std::collections::HashMap<usize, (String, Option<std::path::PathBuf>)>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
    // Concept word -> DIC offset (A3 operands are DIC word offsets).
    let dic_word_offset: std::cell::RefCell<std::collections::HashMap<String, u16>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    // Collect a VM frame's output: dialogue lines (mapped through vm_lines) and
    // any D2 profile handoff.
    let vm_collect = |m: &mut commander_blood_tools::vm::VmMachine,
                      map: &std::collections::HashMap<usize, (String, Option<std::path::PathBuf>)>|
     -> (Vec<(String, Option<std::path::PathBuf>)>, Option<i16>) {
        let mut lines = Vec::new();
        let mut profile = None;
        for ev in m.run_frame() {
            match ev {
                commander_blood_tools::vm::VmEvent::Text { offset } => {
                    if let Some(l) = map.get(&offset) {
                        lines.push(l.clone());
                    }
                }
                commander_blood_tools::vm::VmEvent::ProfileRequest(p) => profile = Some(p),
                _ => {}
            }
        }
        (lines, profile)
    };
    let load_script = |engine: &mut EngineState, music: &mut Music, n: u32| {
        current_script.set(n);
        let r = |ext: &str| std::fs::read(format!("{iso}/SCRIPT{n}.{ext}"));
        if let (Ok(c), Ok(v), Ok(d), Ok(b)) = (r("COD"), r("VAR"), r("DIC"), r("DEB")) {
            // load_dialogue_scenes sets up the scene + the D2 chaining decision.
            engine.load_dialogue_scenes(&c, &v, &d, &b, &descript, Path::new(assets));
            // Load the script's decoded concept-menu stack (the game's gs:0x6772 menu
            // system, src/bas_vm.rs) from its .BAS so conversations can present their
            // real topic menus (seeded at the entry menu; navigated via bas_topic_click).
            if let Ok(bas) = r("BAS") {
                engine.load_bas_menus(&bas, &d);
            }
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
                            .and_then(|h| resolve_scene_hnm(assets, h));
                        (e.text.clone(), scene)
                    })
                    .collect();
                if !lines.is_empty() {
                    engine.set_speech_dialogue(lines);
                }
                // REAL-GAME-VERIFIED (tut_180s..300s captures): the SCRIPT1 console tutorial
                // plays its talk-HNMs over the pyramid-console + eye-orb band.
                engine.set_console_band_dialogue(n == 1);
                // The TOPIC MENU (the game's concept-menu conversation system).
                // The topic LABELS are concept words populated per-context by the
                // script; only SCRIPT2's numerology-consultation labels (TALK /
                // ONE..NINE for its help* topics) are LIVE-VERIFIED (captured from
                // the running game). For the location scripts (SCRIPT3/4/5) the
                // real concept labels are RE-pending (a per-script label table —
                // see re/REVERSE.md), so we do NOT fabricate them: those dialogues
                // keep linear playback until the label source is decoded. Wiring
                // help*→ONE..NINE for locations would be guesswork (SCRIPT3's help1
                // is not the numerology "ONE").
                if n == 1 {
                    // SCRIPT1 = the console TUTORIAL, driven by the FAITHFUL VM:
                    // the tutorial guidance presenter (record 1428) starts as the
                    // scripted opening; console button clicks start their actors'
                    // presentations (HONK=2148, MENU=2220 — from the decompiled
                    // listing decompiled/SCRIPT1.bas); each block plays once and
                    // ends itself (C9), menus rotate via the script's own
                    // self-modifying pokes. No heuristic gating.
                    let cod = std::fs::read(format!("{iso}/SCRIPT1.COD")).unwrap_or_default();
                    let var = std::fs::read(format!("{iso}/SCRIPT1.VAR")).unwrap_or_default();
                    let mut map = std::collections::HashMap::new();
                    for e in bundle.speech_events.iter().filter(|e| !e.text.trim().is_empty()) {
                        let scene = e
                            .background_hnm
                            .as_ref()
                            .and_then(|h| resolve_scene_hnm(assets, h));
                        map.insert(e.offset, (e.text.clone(), scene));
                    }
                    let mut m = commander_blood_tools::vm::VmMachine::new();
                    m.load_cod(&cod);
                    m.load_var(&var);
                    m.start_actor_presentation(1428, 40);
                    let lines: Vec<(String, Option<std::path::PathBuf>)> = m
                        .run_frame()
                        .into_iter()
                        .filter_map(|ev| match ev {
                            commander_blood_tools::vm::VmEvent::Text { offset } => {
                                map.get(&offset).cloned()
                            }
                            _ => None,
                        })
                        .collect();
                    if !lines.is_empty() {
                        engine.set_speech_dialogue(lines);
                    }
                    engine.set_topic_menu(Vec::new());
                    *script_vm.borrow_mut() = Some(m);
                    *vm_lines.borrow_mut() = map;
                    if let Ok(dic_raw) = std::fs::read(format!("{iso}/SCRIPT1.DIC")) {
                        let dict = commander_blood_tools::script::parse_dictionary(&dic_raw);
                        *dic_word_offset.borrow_mut() = dict
                            .into_iter()
                            .map(|(off, w)| (w.to_lowercase(), off))
                            .collect();
                    }
                                } else if n == 2 {
                    // The topic labels are DECODED from the real script, not guessed:
                    // SCRIPT2's help1..help9 are the numerology consultation, whose
                    // concept menu (VM opcode 0xA3 in SCRIPT2.BAS) is [talk, one..nine].
                    // We decode that menu and use its labels; the hard-coded list is only
                    // a fallback if the .BAS is unavailable. See src/concept_menu.rs.
                    let names: Vec<String> = std::fs::read(format!("{iso}/SCRIPT{n}.BAS"))
                        .ok()
                        .map(|bas| concept_menu::decode_menus(&bas, &d, 4))
                        .and_then(|menus| {
                            concept_menu::find_menu_containing(&menus, &["one", "two", "three"]).map(
                                |m| {
                                    m.labels
                                        .iter()
                                        .filter(|l| !l.eq_ignore_ascii_case("talk"))
                                        .map(|l| l.to_uppercase())
                                        .collect()
                                },
                            )
                        })
                        .unwrap_or_else(|| {
                            ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE"]
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        });
                    let mut topics: Vec<(String, usize)> = Vec::new();
                    let mut line_index = 0usize;
                    let mut last_fn = String::new();
                    for e in bundle.speech_events.iter().filter(|e| !e.text.trim().is_empty()) {
                        if e.function_name != last_fn {
                            last_fn = e.function_name.clone();
                            let label = match last_fn.as_str() {
                                f if f.starts_with("help") => f
                                    .strip_prefix("help")
                                    .and_then(|d| d.parse::<usize>().ok())
                                    .and_then(|d| names.get(d - 1).cloned()),
                                f if f.starts_with("honk") || f == "talk" => Some("TALK".to_string()),
                                _ => None,
                            };
                            if let Some(label) = label {
                                topics.push((label, line_index));
                            }
                        }
                        line_index += 1;
                    }
                    topics.sort_by_key(|(l, _)| if l == "TALK" { 0 } else { 1 });
                    let usable = topics.len() >= 3;
                    // Gate auto-play at the first topic-owned line: the scripted OPENING plays
                    // unprompted (the real game's scripted event), then the dialogue holds at
                    // the topic menu — Honk's food menu and the rest play only when clicked.
                    if usable {
                        let first_topic_line = topics.iter().map(|(_, l)| *l).min();
                        engine.set_dialogue_autoplay_end(first_topic_line);
                    }
                    engine.set_topic_menu(if usable { topics } else { Vec::new() });
                } else {
                    // Location scripts (SCRIPT3/4/5): show the decoded concept menu
                    // (bas_vm) — its real topics, clickable via bas_menu_click. The dialogue is
                    // SEGMENTED at the script-function beats (scrujo/bronk1../port1.. — one beat
                    // per character interaction): the host's scripted greeting auto-plays, then
                    // the conversation HOLDS at the concept menu; each topic interaction plays
                    // one beat. (Previously the entire multi-character stream auto-played.)
                    engine.set_topic_menu(Vec::new());
                    engine.sync_topic_menu_from_bas();
                    let mut starts: Vec<usize> = Vec::new();
                    let mut last_fn = String::new();
                    let mut idx = 0usize;
                    for e in bundle.speech_events.iter().filter(|e| !e.text.trim().is_empty()) {
                        if e.function_name != last_fn {
                            last_fn = e.function_name.clone();
                            starts.push(idx);
                        }
                        idx += 1;
                    }
                    if starts.len() > 1 {
                        engine.set_dialogue_segments(starts);
                    }
                }
            }
            if (2..=5).contains(&n) {
                // SCRIPT2-5: the same faithful-VM drive as SCRIPT1. The host's
                // presentation = the first 0xC4 actor in the bytecode (SCRIPT2:
                // rec_0744, the arrival encounter); arrival sets the game flags the
                // opening blocks await (D0/D1 gates). Falls back to the legacy
                // full-stream playback if the first frame yields nothing.
                let cod = std::fs::read(format!("{iso}/SCRIPT{n}.COD")).unwrap_or_default();
                let var = std::fs::read(format!("{iso}/SCRIPT{n}.VAR")).unwrap_or_default();
                let mut map = std::collections::HashMap::new();
                if let Some(bundle) = bundles.iter().find(|b| b.script == format!("SCRIPT{n}")) {
                    for e in bundle.speech_events.iter().filter(|e| !e.text.trim().is_empty()) {
                        let scene = e
                            .background_hnm
                            .as_ref()
                            .and_then(|h| resolve_scene_hnm(assets, h));
                        map.insert(e.offset, (e.text.clone(), scene));
                    }
                }
                let mut m = commander_blood_tools::vm::VmMachine::new();
                m.load_cod(&cod);
                m.load_var(&var);
                m.flag_252a = true;
                m.flag_274f = true;
                let first_actor = commander_blood_tools::vm::walk(&cod, 0, cod.len())
                    .into_iter()
                    .find_map(|t| match t {
                        commander_blood_tools::vm::VmToken::Actor {
                            record_offset,
                            related_record_offset,
                            ..
                        } => Some((record_offset, related_record_offset)),
                        _ => None,
                    });
                if let Some((rec, rel)) = first_actor {
                    m.start_actor_presentation(rec, rel);
                }
                let (lines, _profile) = vm_collect(&mut m, &map);
                if !lines.is_empty() {
                    engine.set_speech_dialogue(lines);
                    if let Ok(dic_raw) = std::fs::read(format!("{iso}/SCRIPT{n}.DIC")) {
                        let dict = commander_blood_tools::script::parse_dictionary(&dic_raw);
                        *dic_word_offset.borrow_mut() = dict
                            .into_iter()
                            .map(|(off, w)| (w.to_lowercase(), off))
                            .collect();
                    }
                    *script_vm.borrow_mut() = Some(m);
                    *vm_lines.borrow_mut() = map;
                } else {
                    // Legacy playback keeps the content reachable until the trigger
                    // for this script's opening is decoded.
                    *script_vm.borrow_mut() = None;
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
    let (mut win_w, mut win_h) = (1920u16, 1200u16); // 6x, aspect-correct
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
    // Mouse capture: clicking into the window LOCKS the mouse to it; Right Shift releases.
    // On Wayland/XWayland an X11 confine-grab is ignored, so we use the pattern XWayland
    // promotes to a REAL Wayland pointer lock (the SDL/game technique): hide the X cursor and
    // continuously warp the pointer back to the window centre, accumulating the relative
    // deltas into a VIRTUAL cursor that we draw ourselves and feed to the engine.
    let mut pointer_locked = false;
    let (mut vcx, mut vcy): (i32, i32) = (win_w as i32 / 2, win_h as i32 / 2);
    // Raw relative motion accumulated this frame while locked — fed to the engine as ring-space
    // deltas (the original's bridge steering tracks the mouse in the 1440-px ring, so rotation
    // continues while the physical mouse moves even with the cursor clamped at the screen edge).
    let (mut raw_dx, mut raw_dy): (i32, i32) = (0, 0);
    // A fully-transparent 1x1 cursor (core protocol, no extension): set as the window cursor
    // while locked so the pinned OS pointer is invisible and only our drawn cursor shows.
    let blank_cursor = {
        let pm = conn.generate_id()?;
        conn.create_pixmap(1, pm, win, 1, 1)?;
        let cur = conn.generate_id()?;
        conn.create_cursor(cur, pm, pm, 0, 0, 0, 0, 0, 0, 0, 0)?;
        conn.free_pixmap(pm)?;
        cur
    };
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
                    if pointer_locked {
                        // Locked: the real pointer is pinned to the window centre; each motion
                        // event's offset from centre is the relative delta. Accumulate it into
                        // the virtual cursor (clamped to the game's letterboxed area) and warp
                        // the real pointer back — XWayland turns this into a Wayland pointer
                        // lock, so the cursor genuinely cannot leave the window.
                        let (cx, cy) = (win_w as i32 / 2, win_h as i32 / 2);
                        let (dx, dy) = (m.event_x as i32 - cx, m.event_y as i32 - cy);
                        raw_dx += dx;
                        raw_dy += dy;
                        if (dx, dy) != (0, 0) {
                            vcx = (vcx + dx)
                                .clamp(off_x as i32, (off_x + dst_w) as i32 - 1);
                            vcy = (vcy + dy)
                                .clamp(off_y as i32, (off_y + dst_h) as i32 - 1);
                            let _ = conn.warp_pointer(
                                x11rb::NONE,
                                win,
                                0,
                                0,
                                0,
                                0,
                                cx as i16,
                                cy as i16,
                            );
                        }
                        mx = (((vcx - off_x as i32) as usize) / scale).min(src_w - 1) as u16;
                        my = (((vcy - off_y as i32) as usize) / scale).min(src_h - 1) as u16;
                    } else {
                        // Map window coords back through the letterbox+scale to source pixels.
                        let ex = (m.event_x as isize - off_x as isize)
                            .clamp(0, dst_w as isize - 1) as usize;
                        let ey = (m.event_y as isize - off_y as isize)
                            .clamp(0, dst_h as isize - 1) as usize;
                        mx = (ex / scale).min(src_w - 1) as u16;
                        my = (ey / scale).min(src_h - 1) as u16;
                    }
                }
                // Clicking into the window LOCKS the mouse to it: hide the X cursor, pin the
                // real pointer to the window centre (XWayland promotes centre-warping with a
                // hidden cursor to a genuine Wayland pointer lock), and track a virtual cursor
                // that we draw ourselves. The locking click is swallowed (it doesn't also click
                // the game). Right Shift releases.
                Event::ButtonPress(b) if !pointer_locked => {
                    pointer_locked = true;
                    // Start the virtual cursor where the user clicked.
                    vcx = (b.event_x as i32).clamp(off_x as i32, (off_x + dst_w) as i32 - 1);
                    vcy = (b.event_y as i32).clamp(off_y as i32, (off_y + dst_h) as i32 - 1);
                    let _ = conn.change_window_attributes(
                        win,
                        &ChangeWindowAttributesAux::new().cursor(blank_cursor),
                    );
                    let _ = conn.grab_pointer(
                        true,
                        win,
                        EventMask::POINTER_MOTION
                            | EventMask::BUTTON_PRESS
                            | EventMask::BUTTON_RELEASE,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                        win, // confined on plain X11; ignored by XWayland (centre-warp covers it)
                        x11rb::NONE,
                        x11rb::CURRENT_TIME,
                    );
                    let _ = conn.warp_pointer(
                        x11rb::NONE,
                        win,
                        0,
                        0,
                        0,
                        0,
                        (win_w / 2) as i16,
                        (win_h / 2) as i16,
                    );
                    conn.change_property8(
                        PropMode::REPLACE,
                        win,
                        u32::from(AtomEnum::WM_NAME),
                        u32::from(AtomEnum::STRING),
                        b"Commander Blood - engine  [mouse locked - Right Shift releases]",
                    )?;
                }
                // Right Shift (keycode 62): release the mouse lock, restoring the real cursor
                // at the virtual cursor's position so the hand-off is seamless.
                Event::KeyPress(k) if k.detail == 62 => {
                    if pointer_locked {
                        pointer_locked = false;
                        let _ = conn.ungrab_pointer(x11rb::CURRENT_TIME);
                        let _ =
                            conn.warp_pointer(x11rb::NONE, win, 0, 0, 0, 0, vcx as i16, vcy as i16);
                        let _ = conn.change_window_attributes(
                            win,
                            &ChangeWindowAttributesAux::new().cursor(x11rb::NONE),
                        );
                        conn.change_property8(
                            PropMode::REPLACE,
                            win,
                            u32::from(AtomEnum::WM_NAME),
                            u32::from(AtomEnum::STRING),
                            b"Commander Blood - engine",
                        )?;
                    }
                }
                // Any click/key dismisses the title screen first (advance to the intro).
                Event::ButtonPress(_) | Event::KeyPress(_) if engine.title_active() => {
                    engine.dismiss_title();
                }
                // During the boot intro, a click/key SKIPS straight to the game (the real
                // game lets you skip the logos/cutscene), rather than sitting through it.
                Event::ButtonPress(_) | Event::KeyPress(_) if engine.intro_active() => {
                    engine.skip_intro();
                }
                // While the MENU submenu ({EXPLANATIONS, GAME}, decoded from the real
                // console) is open, a click resolves it: EXPLANATIONS replays the tutorial
                // (SCRIPT1 — the game's "explanations"), GAME returns to play (the nav).
                // During the VM-driven SCRIPT1 dialogue, the {EXPLANATIONS, GAME}
                // submenu sets the script CONCEPT — the bytecode's own choice logic
                // then runs the explanations or fires RUN PROFILE 1 (-> SCRIPT2).
                Event::ButtonPress(b)
                    if engine.in_dialogue()
                        && engine.menu_submenu_active
                        && script_vm.borrow().is_some()
                        && b.detail == 1 =>
                {
                    let pick = engine.menu_submenu_click(mx, my);
                    engine.menu_submenu_active = false;
                    let word = match pick {
                        Some(0) => Some("explanations"),
                        Some(1) => Some("game"),
                        _ => None,
                    };
                    if let Some(w) = word {
                        let off = dic_word_offset.borrow().get(w).copied();
                        if let Some(off) = off {
                            let mut new_lines: Vec<(String, Option<std::path::PathBuf>)> =
                                Vec::new();
                            let mut profile: Option<i16> = None;
                            if let Some(m) = script_vm.borrow_mut().as_mut() {
                                m.concept = off;
                                m.presentation_busy = true;
                                m.presentation_active = true;
                                let map = vm_lines.borrow();
                                for ev in m.run_frame() {
                                    match ev {
                                        commander_blood_tools::vm::VmEvent::Text { offset } => {
                                            if let Some(l) = map.get(&offset) {
                                                new_lines.push(l.clone());
                                            }
                                        }
                                        commander_blood_tools::vm::VmEvent::ProfileRequest(p) => {
                                            profile = Some(p);
                                        }
                                        _ => {}
                                    }
                                }
                                m.concept = 0;
                            }
                            if let Some(p) = profile {
                                let next = (p.max(0) as u32) + 1;
                                *script_vm.borrow_mut() = None;
                                current_script.set(0);
                                load_script(&mut engine, &mut music, next);
                            } else if !new_lines.is_empty() {
                                engine.set_speech_dialogue(new_lines);
                            }
                        }
                    }
                }
                Event::ButtonPress(b) if engine.bridge_active && engine.menu_submenu_active && b.detail == 1 => {
                    match engine.menu_submenu_click(mx, my) {
                        Some(0) => {
                            engine.menu_submenu_active = false;
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            load_script(&mut engine, &mut music, 1);
                            engine.on_ship = false;
                        }
                        Some(1) => {
                            engine.menu_submenu_active = false;
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            engine.on_ship = true;
                        }
                        _ => {
                            // Click off the submenu closes it and drops the engaged item.
                            engine.menu_submenu_active = false;
                            engine.bridge.release_menu();
                        }
                    }
                }
                // On the bridge, a click runs the decoded hit paths: a golden-menu
                // row selects that console function (HONK (0) = the cook's daily
                // fare (SCRIPT1); TELEPHONE (1) the video-phone; CRYOBOX (2) the
                // cryo-chamber; MENU (3) the {EXPLANATIONS, GAME} submenu; OPTION
                // (4) the 3D pyramid menu), while a hit on the eye-orb arms a
                // station seek — the view auto-rotates there (no screen change).
                Event::ButtonPress(b) if engine.bridge_active && b.detail == 1 => {
                    // In the pyramid nav sector, the destination choice box takes
                    // the click first (choose a location -> its dialogue).
                    if let Some(i) = engine.bridge_nav_destination_click(mx, my) {
                        let dest = 3 + i as u32;
                        engine.progress.visit(&format!("SCRIPT{dest}"));
                        load_script(&mut engine, &mut music, dest);
                        engine.bridge_active = false;
                        engine.on_ship = false;
                        continue;
                    }
                    match engine.bridge_press(mx, my) {
                        Some(0) => {
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            load_script(&mut engine, &mut music, 1);
                            engine.on_ship = false;
                        }
                        Some(1) => {
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            engine.phone_active = true;
                        }
                        Some(2) => {
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            engine.cryobox_active = true;
                        }
                        Some(3) => engine.menu_submenu_active = true, // MENU -> submenu
                        Some(4) => {
                            engine.bridge.release_menu();
                            engine.bridge_active = false;
                            engine.option_active = true; // OPTION -> 3D pyramid menu
                        }
                        _ => {}
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
                // On the OPTION 3D-pyramid menu: a left click selects the item row under
                // the cursor; the right button cycles the selection.
                Event::ButtonPress(b) if engine.option_active && b.detail == 1 => {
                    if let Some(i) = engine.option_item_click(mx, my) {
                        engine.option_cycle(i as i32 - engine.option_item() as i32);
                    }
                }
                Event::ButtonPress(b) if engine.option_active && b.detail == 3 => {
                    engine.option_cycle(1);
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
                        let dest = 3 + i as u32;
                        engine.progress.visit(&format!("SCRIPT{dest}"));
                        load_script(&mut engine, &mut music, dest);
                        engine.on_ship = false;
                    }
                }
                // During a dialogue scene, a left click advances it (snaps the current line
                // fully revealed, then moves to the next) — as the real game does, so the
                // player isn't stuck watching hundreds of lines auto-play.
                Event::ButtonPress(b) if engine.in_dialogue() && b.detail == 1 => {
                    // SCRIPT1 (VM-driven): the golden console menu IS the dispatch —
                    // a button click starts that actor's presentation in the script
                    // VM and the emitted lines play (HONK=2148, MENU=2220; TELEPHONE/
                    // CRYOBOX/OPTION open their screens; CRYOBOX also wakes Cap'n Bob
                    // = the D1 game-flag gate for the BOB block).
                    let mut vm_handled = false;
                    if script_vm.borrow().is_some() {
                        if let Some(row) = engine.console_menu_click(mx, my) {
                            vm_handled = true;
                            let mut new_lines: Vec<(String, Option<std::path::PathBuf>)> =
                                Vec::new();
                            {
                                let mut vm = script_vm.borrow_mut();
                                let m = vm.as_mut().unwrap();
                                let actor = match row {
                                    0 => Some(2148u16), // HONK
                                    _ => None,
                                };
                                if let Some(a) = actor {
                                    m.start_actor_presentation(a, 40);
                                    let map = vm_lines.borrow();
                                    new_lines = m
                                        .run_frame()
                                        .into_iter()
                                        .filter_map(|ev| match ev {
                                            commander_blood_tools::vm::VmEvent::Text {
                                                offset,
                                            } => map.get(&offset).cloned(),
                                            _ => None,
                                        })
                                        .collect();
                                }
                                if row == 2 {
                                    // CRYOBOX: wake Cap'n Bob — the D1 flag opens his block.
                                    m.flag_274f = true;
                                }
                            }
                            if !new_lines.is_empty() {
                                engine.set_speech_dialogue(new_lines);
                            }
                            match row {
                                1 => engine.phone_active = true,
                                2 => engine.cryobox_active = true,
                                3 => engine.menu_submenu_active = true, // {EXPLANATIONS, GAME}
                                4 => engine.option_active = true,
                                _ => {}
                            }
                        }
                    }
                    // VM-driven scripts (2-5): a concept-menu row click sets the
                    // script CONCEPT — the bytecode's A3 guards then run that
                    // topic's own blocks (responses, sub-menus, profile handoffs).
                    if !vm_handled && script_vm.borrow().is_some() && !engine.on_ship {
                        let labels = if engine.topic_menu_is_bas {
                            engine.current_bas_menu_labels()
                        } else {
                            engine.topic_labels()
                        };
                        if let Some(row) = EngineState::list_menu_click(labels.len(), mx, my) {
                            vm_handled = true;
                            let label = labels[row].to_lowercase();
                            if engine.topic_menu_is_bas {
                                engine.bas_topic_click(row); // keep the menu stack behavior
                            }
                            let off = dic_word_offset.borrow().get(&label).copied();
                            if let Some(off) = off {
                                let mut out = (Vec::new(), None);
                                if let Some(m) = script_vm.borrow_mut().as_mut() {
                                    m.concept = off;
                                    let map = vm_lines.borrow();
                                    out = vm_collect(m, &map);
                                    m.concept = 0;
                                }
                                let (new_lines, profile) = out;
                                if let Some(p) = profile {
                                    let next = (p.max(0) as u32) + 1;
                                    *script_vm.borrow_mut() = None;
                                    current_script.set(0);
                                    engine.progress.visit(&format!("SCRIPT{next}"));
                                    load_script(&mut engine, &mut music, next);
                                } else if !new_lines.is_empty() {
                                    engine.set_speech_dialogue(new_lines);
                                }
                            }
                        }
                    }
                    if vm_handled {
                        continue;
                    }
                    // The topic menu takes the click when it is showing (the
                    // concept-menu conversation system); otherwise advance. A BAS
                    // concept menu (topic_menu_is_bas) routes through the decoded
                    // conversation VM (bas_menu_click → sequential responses / pop).
                    let handled = if engine.topic_menu_is_bas {
                        let hit = engine.bas_menu_click(mx, my).is_some();
                        if hit {
                            // A concept-menu interaction advances the conversation one BEAT
                            // (segment) — the menu drives the location dialogue, it doesn't
                            // auto-play through every character's lines.
                            engine.play_next_dialogue_segment();
                        }
                        hit
                    } else {
                        engine.topic_menu_click(mx, my).is_some()
                    };
                    if !handled {
                        engine.skip_dialogue_line();
                    }
                }
                // Left button otherwise drives compass nav selection (via the engine);
                // right button switches between the ship views.
                Event::ButtonPress(b) if b.detail == 1 => {
                    buttons = 1;
                    clicked = true; // latch so a fast press+release still reaches step()
                }
                // Right button switches ship views: console → nav star-map (and back).
                // Without this, landing on the console after the intro would trap the
                // player there with no way to reach navigation/gameplay.
                Event::ButtonPress(b) if b.detail == 3 => {
                    if engine.bridge_active {
                        engine.bridge_active = false;
                        engine.on_ship = true; // console -> nav
                    } else if engine.on_ship {
                        engine.on_ship = false;
                        engine.bridge_active = true; // nav -> console
                    } else {
                        engine.on_ship = true; // from a dialogue scene, back to the nav
                    }
                }
                // Keyboard loop controls: Escape (keycode 9) returns to the nav view.
                Event::KeyPress(k) if k.detail == 9 => {
                    // A visited world-location screen closes back to the nav view first.
                    if engine.world_location_active() {
                        engine.leave_world();
                    } else if engine.menu_submenu_active {
                        // Close the MENU submenu back to the top-level console menu.
                        engine.menu_submenu_active = false;
                    } else if engine.phone_active && engine.phone_connected() {
                        // A connected call hangs up first, back to the phone's dial screen.
                        engine.phone_hangup();
                    } else {
                        engine.alien_view_active = false;
                        engine.tv_active = false;
                        engine.cyber_active = false;
                        engine.cryobox_active = false;
                        engine.phone_active = false;
                        engine.option_active = false;
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
                    if !engine.cyber_active {
                        engine.start_cyberspace();
                    }
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
            dx: raw_dx,
            dy: raw_dy,
        });
        (raw_dx, raw_dy) = (0, 0);
        // SCRIPT1 VM continuation: when the engine finishes the queued lines, run the
        // next script frame — more lines may emit (multi-beat presentations), a D2
        // profile handoff may fire (the tutorial->SCRIPT2 chain), or nothing happens
        // (the console idles awaiting a click, exactly as the game does).
        if !engine.on_ship && !engine.intro_active() && engine.dialogue_finished() {
            let mut new_lines: Vec<(String, Option<std::path::PathBuf>)> = Vec::new();
            let mut profile: Option<i16> = None;
            if let Some(m) = script_vm.borrow_mut().as_mut() {
                let map = vm_lines.borrow();
                for ev in m.run_frame() {
                    match ev {
                        commander_blood_tools::vm::VmEvent::Text { offset } => {
                            if let Some(l) = map.get(&offset) {
                                new_lines.push(l.clone());
                            }
                        }
                        commander_blood_tools::vm::VmEvent::ProfileRequest(p) => {
                            profile = Some(p);
                        }
                        _ => {}
                    }
                }
            }
            if let Some(p) = profile {
                // The script's own D2 handoff: profile p -> SCRIPT{p+1} (profile 1 = SCRIPT2).
                let next = (p.max(0) as u32) + 1;
                *script_vm.borrow_mut() = None;
                current_script.set(0);
                load_script(&mut engine, &mut music, next);
            } else if !new_lines.is_empty() {
                engine.set_speech_dialogue(new_lines);
            }
        }
        // Playable loop: a nav-view click commits a destination → load its dialogue and
        // switch to the scene; when the dialogue finishes, return to the nav view.
        // Suppressed while the startup intro is still playing.
        if engine.ending_active {
            // The finale is playing; when it completes, bookend back to the title screen.
            if engine.ending_finished() {
                engine.ending_active = false;
                music.stop();
                engine.load_title(Path::new(iso));
            }
        } else if engine.cyber_active && engine.cyber_arrived {
            // The cyberspace traversal reached its destination: return to the bridge hub.
            engine.cyber_active = false;
            engine.bridge_active = true;
        } else if engine.intro_active() {
            // Intro playing: start THIS clip's music when the clip begins (the logo reel is
            // silent; the credit cinematic starts blintr.voc). Data-driven from the DESCRIPT
            // `present` record via engine.intro_clip_music(), so timing matches the real game.
            let clip = engine.intro_index();
            if intro_music_clip != Some(clip) {
                intro_music_clip = Some(clip);
                match engine.intro_clip_music() {
                    Some(stem) => music.play(&format!("{assets}/mu/{stem}")),
                    None => music.stop(), // entering a silent clip (the logos): no music
                }
            }
        } else if !tutorial_played {
            // Intro just ended (or was skipped): the real game runs the SCRIPT1 console
            // tutorial AUTOMATICALLY — ORACLE-VERIFIED: a no-input boot of the real
            // BLOODPRG.EXE (runtime_boot INTROTRACE) loads script1.*+chart.fd and starts the
            // tutorial's voices with zero injected input. It then chains to SCRIPT2 via the
            // decoded D2 handoff, after which control returns to the nav for free choice.
            tutorial_played = true;
            if intro_music_clip.is_some() {
                intro_music_clip = None;
                music.stop();
            }
            load_script(&mut engine, &mut music, 1);
            engine.on_ship = false;
        } else if let Some(heading) = engine.take_nav_selection() {
            // SCRIPT1/2 are the forced tutorial + first encounter (played after the intro
            // and chained). The nav offers the free-choice destinations: SCRIPT3/4/5.
            let dest = (heading as u32 * 3 / 180).clamp(0, 2) + 3; // heading → SCRIPT3..5
            engine.progress.visit(&format!("SCRIPT{dest}"));
            load_script(&mut engine, &mut music, dest);
            engine.on_ship = false;
        } else if !engine.on_ship && engine.dialogue_finished() && script_vm.borrow().is_none() {
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
                None if !ending_started && engine.progress.all_visited() => {
                    // Every free-choice location has been visited: play the ending finale,
                    // with the credits music the binary itself names (`mu\credits.voc`,
                    // string at file 0xE16B — the ending IS the credits sequence).
                    ending_started = true;
                    voice = None;
                    voice_line = None;
                    current_script.set(0);
                    engine.start_ending();
                    music.play(&format!("{assets}/mu/credits.voc"));
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
        // Nav target-list music: the DECODED nav-choice handler 4 (file 0x886C) toggles
        // `mu\tablo2.voc` on when the destination list opens and off when it closes (labels
        // DS:0x2578/0x2581 select the list pointer on tablo2 stop/start; DS:0x0BA3 is the active
        // latch). Mirror it: play tablo2 while the nav view is the active screen, stop on leave.
        let nav_music_should_play = engine.nav_view_active()
            && !engine.intro_active()
            && !engine.ending_active
            && !engine.title_active();
        if nav_music_should_play {
            if !nav_music_on {
                nav_music_on = true;
                music.play(&format!("{assets}/mu/tablo2.voc"));
            }
        } else if nav_music_on {
            nav_music_on = false;
            // Stop tablo2 only when the next screen brings no music of its own: the quiet
            // on-ship consoles (bridge/phone/cryobox/option/alien/cyber). Dialogue, the TV,
            // and the ending each start their own music, which already replaced tablo2.
            if engine.on_ship && !engine.tv_active && !engine.ending_active {
                music.stop();
            }
        }
        // TV broadcast music: while a channel is on, play its record's music; stop when the TV
        // closes. Watching per-frame covers every open/close/switch path with one handler.
        if engine.tv_active {
            if let Some(m) = engine.tv_music().map(str::to_string) {
                if tv_music_playing.as_deref() != Some(m.as_str()) {
                    music.play(&format!("{assets}/mu/{m}"));
                    tv_music_playing = Some(m);
                }
            }
        } else if tv_music_playing.take().is_some() {
            music.stop();
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
                        // Voice-paced hold: the real game keeps the line up while its voice
                        // plays (advance gated on SB playback completion) — hold at least the
                        // clip's duration in engine frames (18.2 fps ticks), plus a beat.
                        if clip.sample_rate > 0 {
                            let secs = clip.pcm.len() as f32 / clip.sample_rate as f32;
                            let frames = (secs * 18.2).ceil() as u32 + 4;
                            engine.hold_current_line_at_least(frames);
                        }
                        voice = commander_blood_tools::audio::MusicPlayer::start_once(
                            clip.pcm.clone(),
                            clip.sample_rate,
                        );
                    }
                }
            }
        }
        let _ = &voice; // keep the stream alive while the line plays
        // Subtitle CHATTER (decoded @0xB898): while a line is REVEALING (the chatter flag
        // [0xCFB] is set until the reveal completes @0x94CF), every 4 ticks the game plays a
        // RANDOM burble clip — index 7 + random(0..9) of the talk-burble bank tb.snd (17 clips;
        // 7..16 are ten ~0.12s blips), never repeating the previous pick ([0xC4D]). This is the
        // continuous honk-burble under the text, not a single end-of-line blip.
        if !engine.on_ship {
            if let Some((revealed, total)) = engine.subtitle_reveal_progress() {
                if revealed < total {
                    if chatter_throttle == 0 {
                        chatter_throttle = 4; // [0xB2F] = 4 tick throttle
                        if let Some(bank) = &tb_snd {
                            // random 0..9, != previous (the asm re-rolls until different)
                            chatter_seed = chatter_seed.wrapping_mul(1103515245).wrapping_add(12345);
                            let mut pick = (chatter_seed >> 16) % 10;
                            if Some(pick) == chatter_prev {
                                pick = (pick + 1) % 10;
                            }
                            chatter_prev = Some(pick);
                            if let Some(clip) =
                                bank.clip(7 + pick as usize).filter(|c| !c.pcm.is_empty())
                            {
                                chatter = commander_blood_tools::audio::MusicPlayer::start_once(
                                    clip.pcm.clone(),
                                    clip.sample_rate,
                                );
                            }
                        }
                    } else {
                        chatter_throttle -= 1;
                    }
                } else {
                    chatter_throttle = 0;
                }
            }
        }
        let _ = &chatter;
        let _ = &chatter_done_line;
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
        // While the mouse is locked the OS cursor is hidden and pinned; draw the VIRTUAL cursor
        // (a small white crosshair with a dark outline) at its window position so the player can
        // still point at menus and console buttons.
        if pointer_locked {
            let stride = win_w as usize * 4;
            let mut put = |x: i32, y: i32, c: [u8; 3]| {
                if x >= 0 && y >= 0 && (x as usize) < win_w as usize && (y as usize) < win_h as usize
                {
                    let di = y as usize * stride + x as usize * 4;
                    if di + 2 < image.len() {
                        image[di] = c[2];
                        image[di + 1] = c[1];
                        image[di + 2] = c[0];
                    }
                }
            };
            for d in -5i32..=5 {
                // dark outline first, then the white cross on top
                put(vcx + d, vcy + 1, [20, 20, 20]);
                put(vcx + 1, vcy + d, [20, 20, 20]);
            }
            for d in -5i32..=5 {
                put(vcx + d, vcy, [245, 245, 245]);
                put(vcx, vcy + d, [245, 245, 245]);
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
