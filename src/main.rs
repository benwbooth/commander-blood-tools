mod extract;

/// The cook's daily-fare lines (the MENU row's presentation — oracle: white
/// subtitle text, e.g. "Jellied URTIKAN with MURFFALO bone marrow"): the SCRIPT1
/// dish block around offset 0x100.
fn menu_dish_lines(iso: &str) -> Vec<String> {
    let Ok(descript) = commander_blood_tools::descript::DescriptDb::parse_file(
        &std::path::Path::new(iso).join("DESCRIPT.DES"),
    ) else {
        return Vec::new();
    };
    let hnm_music = descript.hnm_music_map();
    let Ok(bundles) =
        commander_blood_tools::script::parse_script_dir(iso, &descript, &hnm_music)
    else {
        return Vec::new();
    };
    let Some(b) = bundles.iter().find(|b| b.script == "SCRIPT1") else {
        return Vec::new();
    };
    b.speech_events
        .iter()
        .filter(|e| (0x00F0..0x0150).contains(&e.offset) && !e.text.trim().is_empty())
        .map(|e| e.text.clone())
        .collect()
}

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
    // Per-world CANDIDATE LABELS (the decoded 0x7259 list = the location's flags-
    // filtered entities = its CHARACTERS): the script's distinct DEB-resolved actor
    // names, in speech order — the real names the candidate box offers.
    let world_candidates: Vec<Vec<String>> = (3..=5u32)
        .map(|n| {
            let mut names: Vec<String> = Vec::new();
            if let Some(bundle) = bundles.iter().find(|bu| bu.script == format!("SCRIPT{n}")) {
                for e in &bundle.speech_events {
                    if let Some(a) = &e.actor_record {
                        let a = a.to_uppercase();
                        if !names.contains(&a) {
                            names.push(a);
                        }
                    }
                }
            }
            names.truncate(7);
            names
        })
        .collect();
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
    // SCRIPT1 tutorial auto-chain (ORACLE-observed: the real tutorial plays Izwalito's
    // guidance, then Honk's welcome, then the menu demo WITHOUT clicks — 'hon' at ~52M
    // steps, menus at ~57M in no-dispatch probes). Remaining presenters to auto-start
    // when the current content finishes; clicks then replay from idle.
    let tutorial_chain: std::cell::RefCell<Vec<u16>> = std::cell::RefCell::new(Vec::new());
    let vm_lines: std::cell::RefCell<
        std::collections::HashMap<usize, (String, Option<std::path::PathBuf>, bool)>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
    // Line offset -> the concept-menu labels the LINE RECORD carries after its
    // 0xFFFF separator (the bytecode's own menu source; script.rs menu_labels).
    let vm_menus: std::cell::RefCell<std::collections::HashMap<usize, Vec<String>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    // Concept word -> DIC offset (A3 operands are DIC word offsets).
    let dic_word_offset: std::cell::RefCell<std::collections::HashMap<String, u16>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    // Install a VM-emitted line set (text+scene+style) into the engine.
    let set_vm_dialogue = |engine: &mut EngineState,
                           lines: Vec<(String, Option<std::path::PathBuf>, bool)>| {
        let styles: Vec<bool> = lines.iter().map(|(_, _, sp)| *sp).collect();
        engine.set_speech_dialogue(lines.into_iter().map(|(t, p, _)| (t, p)).collect());
        engine.set_dialogue_styles(styles);
    };
    // Collect a VM frame's output: dialogue lines (mapped through vm_lines) and
    // any D2 profile handoff.
    let vm_collect = |m: &mut commander_blood_tools::vm::VmMachine,
                      map: &std::collections::HashMap<
        usize,
        (String, Option<std::path::PathBuf>, bool),
    >|
     -> (Vec<(String, Option<std::path::PathBuf>, bool)>, Option<i16>, Option<Vec<String>>) {
        let mut lines = Vec::new();
        let mut profile = None;
        let mut menu: Option<Vec<String>> = None;
        // A8 LOADSTR drives the scene backdrop for the FOLLOWING lines (decoded from
        // SCRIPT5's finale: LOADSTR lpm6sc1.hnm / SAY ... reels). "fin.hnm" = the
        // finale film itself — the script's own ENDING trigger (the Bigbang-concert
        // block) — signalled to the driver as profile -100.
        let mut scene_override: Option<std::path::PathBuf> = None;
        for ev in m.run_frame() {
            match ev {
                commander_blood_tools::vm::VmEvent::Text { offset } => {
                    if let Some((text, scene, speech)) = map.get(&offset) {
                        lines.push((
                            text.clone(),
                            scene_override.clone().or_else(|| scene.clone()),
                            *speech,
                        ));
                    }
                    if let Some(labels) = vm_menus.borrow().get(&offset) {
                        menu = Some(labels.clone());
                    }
                }
                commander_blood_tools::vm::VmEvent::LoadString(name) => {
                    let lower = name.to_lowercase();
                    if lower == "fin.hnm" {
                        profile = Some(-100); // ending sentinel
                    } else if lower.ends_with(".hnm") {
                        scene_override = resolve_scene_hnm(assets, &lower);
                    }
                }
                commander_blood_tools::vm::VmEvent::ProfileRequest(p) => profile = Some(p),
                _ => {}
            }
        }
        (lines, profile, menu)
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
                // SCRIPT2's numerology labels (TALK / ONE..NINE) are LIVE-VERIFIED;
                // the location scripts (SCRIPT3/4/5) get their REAL concept labels
                // from the decoded .BAS menu stacks (bas_vm — SCRIPT3 LISTEN/INSIST/
                // TREAT, SCRIPT4 PAINTING/CULTURE/YOLK, SCRIPT5 PEACE/WAR/...),
                // wired below via sync_topic_menu_from_bas + bas_menu_click.
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
                        // Character speech (actor resolves) = green bold reveal; plain
                        // script text (the MENU lists) = white thin static (oracle-verified).
                        let speech = e.actor_record.is_some();
                        map.insert(e.offset, (e.text.clone(), scene, speech));
                        if !e.menu_labels.is_empty() {
                            vm_menus
                                .borrow_mut()
                                .insert(e.offset, e.menu_labels.clone());
                        }
                    }
                    let mut m = commander_blood_tools::vm::VmMachine::new();
                    m.load_cod(&cod);
                    m.load_var(&var);
                    // BOOT PRESENTER = HONK (actor 2148, related 40): oracle-verified
                    // (TUTORIAL4 live sequence == the [061D] Honk.talk block, 9/9 lines
                    // in order: WELCOME ABOARD -> ... -> CLICK QUICK ON CRYOBOX).
                    // Izwalito's guidance (1428) is the MENU>EXPLANATIONS replay block,
                    // NOT the boot. The follow-ups are event-driven (the CRYOBOX click
                    // wakes Bob) — no synthetic chain.
                    m.start_actor_presentation(2148, 40);
                    *tutorial_chain.borrow_mut() = Vec::new();
                    let lines: Vec<(String, Option<std::path::PathBuf>, bool)> = m
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
                        set_vm_dialogue(engine, lines);
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
                        let speech = e.actor_record.is_some();
                        map.insert(e.offset, (e.text.clone(), scene, speech));
                        if !e.menu_labels.is_empty() {
                            vm_menus
                                .borrow_mut()
                                .insert(e.offset, e.menu_labels.clone());
                        }
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
                // Arriving at the scripted location satisfies the opening block's
                // location guards (SCRIPT2: current_location = Pterra).
                m.satisfy_opening_location_guards();
                let (lines, _profile, _menu) = vm_collect(&mut m, &map);
                if !lines.is_empty() {
                    set_vm_dialogue(engine, lines);
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

    let (conn, screen_num) = x11rb::xcb_ffi::XCBConnection::connect(None)
        .map_err(|e| anyhow::anyhow!("X11 connect: {e}"))?;
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
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::VISIBILITY_CHANGE,
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
    // FULLY-OBSCURED windows must not present through the FIFO swapchain: the
    // compositor stops consuming frames, get_current_texture() blocks the whole
    // event loop, and KWin kills the now-unresponsive client. Track X visibility
    // and skip GPU presents while nothing of the window is visible.
    let mut win_visible = true;
    let (mut vcx, mut vcy): (i32, i32) = (win_w as i32 / 2, win_h as i32 / 2);
    // Raw relative motion accumulated this frame while locked — fed to the engine as ring-space
    // deltas (the original's bridge steering tracks the mouse in the 1440-px ring, so rotation
    // continues while the physical mouse moves even with the cursor clamped at the screen edge).
    let (mut raw_dx, mut raw_dy): (i32, i32) = (0, 0);
    let (mut prev_mx, mut prev_my): (i32, i32) = (160, 100);
    // State-countdown beat accumulator (the 0x8AA law): the game divides its
    // 200Hz PIT chain by 25 ([0xB27] reload 0x19) -> ~8.01 beats/s, ticking the
    // state array only while no presentation is active ([0x675A]==0).
    let mut countdown_accum: f32 = 0.0;
    let mut last_tick = std::time::Instant::now();
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
    // GPU presentation (wgpu over the same X11 window): the 320x200 frame as a
    // nearest-scaled quad + the 3D hand as real triangles at window resolution.
    // CB_SOFT=1 forces the software PutImage path.
    let mut gpu = if std::env::var("CB_SOFT").is_err() {
        let raw = conn.get_raw_xcb_connection();
        match unsafe {
            commander_blood_tools::gpu::GpuPresenter::new(
                raw,
                screen_num as i32,
                win,
                win_w as u32,
                win_h as u32,
                commander_blood_tools::manu3_hand::hand_texture().0,
                commander_blood_tools::manu3_hand::hand_texture().1 as u32,
            )
        } {
            Ok(p) => {
                eprintln!("[gpu] wgpu presenter active (hand at window resolution)");
                Some(p)
            }
            Err(e) => {
                eprintln!("[gpu] unavailable ({e}); software presentation");
                None
            }
        }
    } else {
        None
    };
    engine.gpu_hand_enabled = gpu.is_some();
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
                            let mut new_lines: Vec<(
                                String,
                                Option<std::path::PathBuf>,
                                bool,
                            )> = Vec::new();
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
                            if profile == Some(-100) {
                                *script_vm.borrow_mut() = None;
                                engine.start_ending();
                                music.play(&format!("{assets}/mu/credits.voc"));
                            } else if let Some(p) = profile {
                                let next = (p.max(0) as u32) + 1;
                                *script_vm.borrow_mut() = None;
                                current_script.set(0);
                                load_script(&mut engine, &mut music, next);
                            } else if !new_lines.is_empty() {
                                set_vm_dialogue(&mut engine, new_lines);
                            }
                        }
                    }
                }
                // Bob's CONTACT screen: BYE_BYE (topic 0) returns to the bridge;
                // other topics route through the script VM's concept dispatch —
                // set the topic's DIC word as m.concept and play the A3 block's
                // lines (the same decoded mechanism as the conversation menus).
                Event::ButtonPress(b) if engine.bob_contact_active && b.detail == 1 => {
                    match engine.bob_topic_click(mx, my) {
                        Some(0) => {
                            engine.bob_contact_active = false;
                            engine.bridge_active = true;
                        }
                        Some(row) => {
                            engine.console_box_selected = Some(row);
                            let label = if engine.bob_topics.is_empty() {
                                EngineState::BOB_TOPICS[row].to_string()
                            } else {
                                engine.bob_topics[row].clone()
                            }
                            .to_lowercase();
                            let off = dic_word_offset.borrow().get(&label).copied();
                            if let Some(off) = off {
                                let mut out = (Vec::new(), None, None);
                                if let Some(m) = script_vm.borrow_mut().as_mut() {
                                    m.concept = off;
                                    let map = vm_lines.borrow();
                                    out = vm_collect(m, &map);
                                    m.concept = 0;
                                }
                                if !out.0.is_empty() {
                                    engine.set_speech_dialogue(
                                        out.0.into_iter().map(|(t, s, _)| (t, s)).collect(),
                                    );
                                }
                            }
                        }
                        None => {}
                    }
                }
                Event::ButtonPress(b)
                    if engine.bridge_active && !engine.console_box.is_empty() && b.detail == 1 =>
                {
                    if let Some(row) = engine.console_box_click(mx, my) {
                        let last = engine.console_box.len() - 1;
                        let kind = engine.console_box_kind;
                        let box_labels = engine.console_box.clone();
                        engine.console_box.clear();
                        engine.bridge.engaged_row = None;
                        if row < last {
                            engine.hand_pose_event(7); // the decoded SELECTING pose
                            engine.hand_pose_event(0xA); // the decoded transition pose
                            match kind {
                                1 => {
                                    engine.bridge.release_menu();
                                    engine.bridge_active = false;
                                    engine.phone_active = true;
                                    engine.phone_connect(row);
                                }
                                2 => {
                                    // BOB_MORLOCK: the CONTACT screen — Bob's eyes
                                    // (FRIGO.FD) + his concept menu, NOT the cryo
                                    // chamber (ORACLE cryobox_enter vs_005..007).
                                    engine.bridge.release_menu();
                                    engine.bridge_active = false;
                                    engine.bob_contact_active = true;
                                    // THE REAL PRESENTER: SCRIPT2's Bob_Morlock.talk
                                    // (record 132 rel 40 — C4 guard @1C51) plays his
                                    // state-gated greeting; the captured line is the
                                    // no-VM fallback.
                                    let mut lines: Vec<(
                                        String,
                                        Option<std::path::PathBuf>,
                                        bool,
                                    )> = Vec::new();
                                    engine.bob_topics = Vec::new();
                                    if current_script.get() == 2 {
                                        let mut vm = script_vm.borrow_mut();
                                        if let Some(m) = vm.as_mut() {
                                            m.start_actor_presentation(132, 40);
                                            let map = vm_lines.borrow();
                                            let out = vm_collect(m, &map);
                                            lines = out.0;
                                            if let Some(menu) = out.2 {
                                                // The bytecode's own topic list (the
                                                // prompt line's 0xFFFF-carried words).
                                                engine.bob_topics = menu
                                                    .iter()
                                                    .map(|l| l.to_uppercase())
                                                    .collect();
                                            }
                                        }
                                    }
                                    if lines.is_empty() {
                                        lines = vec![(
                                            "HONK! You worthless heap of wires... Are  \nyou working?".into(),
                                            None,
                                            true,
                                        )];
                                    }
                                    engine.set_speech_dialogue(
                                        lines.into_iter().map(|(t, s, _)| (t, s)).collect(),
                                    );
                                    engine.load_bob_contact(Path::new(iso), Path::new(assets));
                                }
                                // The IN-WINDOW concept box (kind 3): fully
                                // BYTECODE-DRIVEN. The clicked topic's DIC word
                                // becomes m.concept; the VM's A3 dispatch plays the
                                // script's own lines, and the NEXT menu is whatever
                                // the emitted line records carry after their 0xFFFF
                                // separator (script.rs menu_labels) — no hardcoded
                                // trees or labels. A poked follow-up presentation
                                // (e.g. talk -> rec_08B8) starts on the extra frame.
                                3 => {
                                    engine.console_box_selected = Some(row);
                                    let label = box_labels
                                        .get(row)
                                        .cloned()
                                        .unwrap_or_default()
                                        .to_lowercase();
                                    let off =
                                        dic_word_offset.borrow().get(&label).copied();
                                    let mut new_lines: Vec<(
                                        String,
                                        Option<std::path::PathBuf>,
                                        bool,
                                    )> = Vec::new();
                                    let mut next_menu: Option<Vec<String>> = None;
                                    if let Some(off) = off {
                                        let mut vm = script_vm.borrow_mut();
                                        if let Some(m) = vm.as_mut() {
                                            m.concept = off;
                                            let map = vm_lines.borrow();
                                            let out = vm_collect(m, &map);
                                            m.concept = 0;
                                            new_lines = out.0;
                                            next_menu = out.2;
                                            if new_lines.is_empty() {
                                                // A poked presentation starts on the
                                                // following frame — collect it.
                                                let out2 = vm_collect(m, &map);
                                                new_lines = out2.0;
                                                next_menu = next_menu.or(out2.2);
                                            }
                                        }
                                    }
                                    if label == "bye_bye" {
                                        // The script's own conversation exit — box
                                        // closed (already cleared above).
                                    } else if let Some(labels) = next_menu {
                                        // The bytecode's next menu, verbatim.
                                        engine.bridge.engaged_row = Some(0);
                                        engine.console_box = labels
                                            .iter()
                                            .map(|l| l.to_uppercase())
                                            .collect();
                                        engine.console_box_kind = 3;
                                        engine.console_box_selected = None;
                                    } else {
                                        // No new menu: the current one persists with
                                        // the engaged topic highlighted (oracle
                                        // honk_blood law).
                                        engine.bridge.engaged_row = Some(0);
                                        engine.console_box = box_labels.clone();
                                        engine.console_box_kind = 3;
                                    }
                                    if !new_lines.is_empty() {
                                        set_vm_dialogue(&mut engine, new_lines);
                                    }
                                }
                                // The OPTION submenu {TEXT, MUSIC_OFF, SAVE, LOAD,
                                // QUIT, CANCEL} — the decoded row surfaces.
                                4 => match row {
                                    0 => {
                                        // TEXT: cycle the decoded speed steps {1,2,3,4,7}.
                                        engine.text_speed_step =
                                            match engine.text_speed_step {
                                                1 => 2,
                                                2 => 3,
                                                3 => 4,
                                                4 => 7,
                                                _ => 1,
                                            };
                                    }
                                    1 => music.stop(), // MUSIC_OFF
                                    2 => {
                                        // SAVE: the oracle-measured slot-name UI.
                                        engine.save_ui_active = true;
                                        engine.save_ui_name.clear();
                                    }
                                    3 => {
                                        // LOAD: read the DOS-format slot 1 save.
                                        if let Ok(bytes) =
                                            std::fs::read(format!("{iso}/game1.sav"))
                                        {
                                            let profile = script_vm
                                                .borrow_mut()
                                                .as_mut()
                                                .and_then(|m| m.apply_dos_save(&bytes));
                                            if let Some(p) = profile {
                                                let n = (p.max(0) as u32).max(1);
                                                load_script(&mut engine, &mut music, n);
                                                engine.on_ship = false;
                                            }
                                        }
                                    }
                                    4 => return Ok(()), // QUIT
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                    } else {
                        engine.console_box.clear(); // click off the box closes it
                        engine.bridge.engaged_row = None;
                        engine.hand_pose_event(0xB);
                    }
                }
                Event::ButtonPress(b) if engine.bridge_active && engine.option_box_active && b.detail == 1 => {
                    // The OPTION choice box: CANCEL (its only hub item) closes it.
                    engine.option_box_active = false;
                    engine.hand_pose_event(0xB); // the decoded UI-close pose
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
                    // The hub presentation's / engaged row's CANCEL label first.
                    if engine.hub_cancel_click(mx, my) {
                        engine.bridge.engaged_row = None;
                        continue;
                    }
                    // In the pyramid nav sector, the destination choice box takes
                    // the click first (choose a location -> its dialogue).
                    // The nav-sector orb with NO granted destinations opens the VIEWSCREEN
                    // console (static — the oracle-verified empty-nav state).
                    if engine.bridge_nav_orb_click(mx, my) && engine.nav_destination_count() == 0 {
                        engine.bridge.release_menu();
                        engine.bridge_active = false;
                        engine.viewscreen_active = true;
                        engine.hand_pose_event(0xA);
                        continue;
                    }
                    if let Some(i) = engine.bridge_nav_destination_click(mx, my) {
                        let dest = 3 + i as u32;
                        engine.progress.visit(&format!("SCRIPT{dest}"));
                        load_script(&mut engine, &mut music, dest);
                        engine.bridge_active = false;
                        engine.on_ship = false;
                        continue;
                    }
                    // ROW SURFACES per the dual-run oracle captures (2026-07-23):
                    // each engaged row turns RED; HONK opens the {TALK, REMEMBER,
                    // BYE_BYE} concept menu with "WHAT DO YOU WANT COMMANDER?";
                    // TELEPHONE arms with just CANCEL; CRYOBOX = {BOB_MORLOCK,
                    // CANCEL}; MENU plays the cook's daily fare; OPTION = {TEXT,
                    // MUSIC_OFF, SAVE, LOAD, QUIT, CANCEL}.
                    match engine.bridge_press(mx, my) {
                        Some(0) => {
                            engine.bridge.engaged_row = Some(0);
                            // THE REAL PRESENTER BLOCK: SCRIPT2's Honk.talk (record
                            // 2220 rel 40 — C4 guards @0B04/0B87/11A8) emits the
                            // state-gated lines the oracle plays ('Commander,
                            // remember ol' Bob snoring in the Cryobox...' -> ... ->
                            // 'What do you want Commander ?'). The hardcoded prompt
                            // remains only as the no-VM fallback.
                            let mut new_lines: Vec<(
                                String,
                                Option<std::path::PathBuf>,
                                bool,
                            )> = Vec::new();
                            let mut honk_menu: Option<Vec<String>> = None;
                            if current_script.get() == 2 {
                                let mut vm = script_vm.borrow_mut();
                                if let Some(m) = vm.as_mut() {
                                    m.start_actor_presentation(2220, 40);
                                    let map = vm_lines.borrow();
                                    let out = vm_collect(m, &map);
                                    new_lines = out.0;
                                    honk_menu = out.2;
                                }
                            }
                            if new_lines.is_empty() {
                                new_lines = vec![(
                                    "What do you want Commander ?".into(),
                                    None,
                                    true,
                                )];
                            }
                            // The BOX comes from the emitted prompt line's own
                            // carried menu (the bytecode's 0xFFFF-separated words);
                            // the captured labels only when no VM is loaded.
                            engine.console_box = honk_menu
                                .unwrap_or_else(|| {
                                    vec![
                                        "TALK".into(),
                                        "REMEMBER".into(),
                                        "BYE_BYE".into(),
                                    ]
                                })
                                .iter()
                                .map(|l| l.to_uppercase())
                                .collect();
                            engine.console_box_kind = 3;
                            engine.console_box_selected = None;
                            set_vm_dialogue(&mut engine, new_lines);
                        }
                        Some(1) => {
                            engine.bridge.engaged_row = Some(1);
                        }
                        Some(2) => {
                            engine.bridge.engaged_row = Some(2);
                            engine.console_box = vec!["BOB_MORLOCK".into(), "CANCEL".into()];
                            engine.console_box_kind = 2;
                        }
                        Some(3) => {
                            engine.bridge.engaged_row = Some(3);
                            let dishes = menu_dish_lines(iso);
                            if !dishes.is_empty() {
                                set_vm_dialogue(
                                    &mut engine,
                                    dishes.into_iter().map(|t| (t, None, false)).collect(),
                                );
                            }
                        }
                        Some(4) => {
                            engine.bridge.engaged_row = Some(4);
                            engine.console_box = vec![
                                "TEXT".into(),
                                "MUSIC_OFF".into(),
                                "SAVE".into(),
                                "LOAD".into(),
                                "QUIT".into(),
                                "CANCEL".into(),
                            ];
                            engine.console_box_kind = 4;
                        }
                        _ => {}
                    }
                }
                // On a visited world-location screen, clicking the world's ENTITY
                // (the decoded .ext object marker — the location's inhabitant)
                // engages that location's dialogue: walk up and talk. The heading
                // that picked the world picks the same destination script.
                Event::ButtonPress(b)
                    if engine.world_location_active() && b.detail == 1 =>
                {
                    // LIST-DRIVEN interaction (decoded 0x7259: the world/entity selection is a
                    // filtered candidate LIST, no free-roam hit-test): clicking an entity
                    // marker opens the candidate box; choosing engages the location's
                    // dialogue; CANCEL/elsewhere steps rooms.
                    if let Some(row) = engine.console_box_click(mx, my) {
                        let last = engine.console_box.len().saturating_sub(1);
                        engine.console_box.clear();
                        if row < last {
                            let heading = engine.compass_angle;
                            let dest = (heading as u32 * 3 / 180).clamp(0, 2) + 3;
                            engine.leave_world();
                            engine.progress.visit(&format!("SCRIPT{dest}"));
                            load_script(&mut engine, &mut music, dest);
                            engine.on_ship = false;
                        }
                    } else if engine.world_object_click(mx, my).is_some() {
                        // The candidate box = the location's CHARACTERS (the decoded
                        // 0x7259 flags-filtered entity list): the script's distinct
                        // DEB-resolved actor names, falling back to the heading's
                        // host label when the bundle carries none.
                        let heading = engine.compass_angle;
                        let dest_idx = (heading as usize * 3 / 180).min(2);
                        let mut labels = world_candidates
                            .get(dest_idx)
                            .cloned()
                            .unwrap_or_default();
                        if labels.is_empty() {
                            labels = vec![engine
                                .nav_destination_label(dest_idx)
                                .unwrap_or_else(|| "TALK".into())];
                        }
                        labels.push("CANCEL".into());
                        engine.console_box = labels;
                        engine.console_box_kind = 10;
                    } else {
                        engine.cycle_world_room(1);
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
                    // ORACLE-verified dispatch rule: while a presentation is PLAYING, a click
                    // ADVANCES it (the option-row probe stepped the menu text); menu rows
                    // DISPATCH only from the idle console (the HONK probe dispatched from
                    // idle). Gate on dialogue_finished.
                    if script_vm.borrow().is_some() && engine.dialogue_finished() {
                        if let Some(row) = engine.console_menu_click(mx, my) {
                            vm_handled = true;
                            let mut new_lines: Vec<(
                                String,
                                Option<std::path::PathBuf>,
                                bool,
                            )> = Vec::new();
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
                                set_vm_dialogue(&mut engine, new_lines);
                            }
                            match row {
                                1 => engine.phone_active = true,
                                2 => engine.cryobox_active = true,
                                3 => engine.menu_submenu_active = true, // {EXPLANATIONS, GAME}
                                4 => engine.option_box_active = true,   // choice box (real)
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
                                let mut out = (Vec::new(), None, None);
                                if let Some(m) = script_vm.borrow_mut().as_mut() {
                                    m.concept = off;
                                    let map = vm_lines.borrow();
                                    out = vm_collect(m, &map);
                                    m.concept = 0;
                                }
                                let (new_lines, profile, _menu) = out;
                                if let Some(p) = profile {
                                    let next = (p.max(0) as u32) + 1;
                                    *script_vm.borrow_mut() = None;
                                    current_script.set(0);
                                    engine.progress.visit(&format!("SCRIPT{next}"));
                                    load_script(&mut engine, &mut music, next);
                                } else if !new_lines.is_empty() {
                                    set_vm_dialogue(&mut engine, new_lines);
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
                    if engine.viewscreen_active {
                        engine.viewscreen_active = false;
                        engine.bridge_active = true;
                    } else if engine.world_location_active() {
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
                    // Also write the DOS-format save (the real game's blood.sav layout,
                    // decoded @0x1C3F): copy blooddos.sav over a DOS install's blood.sav
                    // to carry the VM state into the original game.
                    if let Some(m) = script_vm.borrow().as_ref() {
                        let profile = current_script.get().saturating_sub(1) as u16;
                        let bytes = m.to_dos_save(profile);
                        if std::fs::write("blooddos.sav", &bytes).is_ok() {
                            println!("saved blooddos.sav (DOS format, {} bytes)", bytes.len());
                        }
                    }
                }
                // F9 (keycode 75): load blood.sav and resume that saved state.
                Event::KeyPress(k) if k.detail == 75 => {
                    use commander_blood_tools::save::{SaveScreen, SaveState};
                    // A DOS-format save (from the original game, or blooddos.sav) loads
                    // straight into the script VM: restore the arrays, re-select the
                    // saved profile's script.
                    let dos_bytes = std::fs::read("blooddos.sav")
                        .ok()
                        .filter(|_| SaveState::read(Path::new("blood.sav")).is_none());
                    if let Some(bytes) = dos_bytes {
                        let mut probe = commander_blood_tools::vm::VmMachine::new();
                        if let Some(profile) = probe.apply_dos_save(&bytes) {
                            let script = u32::from(profile) + 1;
                            load_script(&mut engine, &mut music, script.clamp(1, 5));
                            if let Some(m) = script_vm.borrow_mut().as_mut() {
                                m.apply_dos_save(&bytes);
                            }
                            engine.on_ship = false;
                            println!("loaded blooddos.sav (DOS format, script {script})");
                            continue;
                        }
                    }
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
                    } else if { engine.compass_angle = (engine.compass_angle + 45) % 180; true } {
                        let world = match engine.targeted_world_name() {
                            Some(w) => w,
                            None => continue,
                        };
                        if engine.visit_world(world, Path::new(assets)) {
                            // Overlay the world's decoded .ext object positions (from the ISO).
                            if let Ok(ext) = std::fs::read(format!("{iso}/{}.EXT", world.to_uppercase())) {
                                engine.set_world_ext(&ext);
                            }
                        }
                    }
                }
                // SAVE-SLOT NAME ENTRY takes every key while active: X11 keycodes
                // (evdev scancode + 8) -> ASCII, fed through the DOS edit law
                // (digits+lowercase, backspace, Enter commits). On commit, write the
                // DOS-format slot files exactly as the original does: game1.sav (the
                // vm state, bloodsav layout) + blood.sav (the 10x32 slot directory).
                Event::KeyPress(k) if engine.save_ui_active => {
                    let ascii: u8 = match k.detail {
                        10..=18 => b'1' + (k.detail - 10) as u8,
                        19 => b'0',
                        24..=33 => b"qwertyuiop"[(k.detail - 24) as usize],
                        38..=46 => b"asdfghjkl"[(k.detail - 38) as usize],
                        52..=58 => b"zxcvbnm"[(k.detail - 52) as usize],
                        36 => 13, // Enter
                        22 => 8,  // Backspace
                        9 => {
                            engine.save_ui_active = false; // Escape cancels
                            engine.save_ui_name.clear();
                            continue;
                        }
                        _ => 0,
                    };
                    if let Some(name) = engine.save_ui_key(ascii) {
                        let vm_bytes = script_vm
                            .borrow()
                            .as_ref()
                            .map(|m| m.to_dos_save(current_script.get() as u16));
                        if let Some(bytes) = vm_bytes {
                            let _ = std::fs::write(format!("{iso}/game1.sav"), &bytes);
                        }
                        // The slot directory: ten 32-byte {15-char name, NUL,
                        // "game<N>.sav"} records, the typed name in slot 1.
                        let mut dir = Vec::with_capacity(320);
                        for n in 1..=10u32 {
                            let mut rec = [0u8; 32];
                            rec[..15].fill(b' ');
                            if n == 1 {
                                rec[..name.len().min(15)]
                                    .copy_from_slice(&name.as_bytes()[..name.len().min(15)]);
                            }
                            let fname = format!("game{n}.sav");
                            rec[16..16 + fname.len()].copy_from_slice(fname.as_bytes());
                            dir.extend_from_slice(&rec);
                        }
                        let _ = std::fs::write(format!("{iso}/blood.sav"), &dir);
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
                // 'y' (keycode 29): enter CYBERSPACE — the cyber.ext world (decoded:
                // cyberspace is a standard .ext world, level index 36 'cyber', rooms
                // 1cyber*.lbm), visited via the SAME world-visit system as the planets;
                // BIOXX are its entities (touch -> BIONIUM, the decoded goal). Was: the
                // hyper_*.hnm hyperspace-TRAVEL videos (a different sequence). The 'y'
                // hyperspace-flight remains available as the inter-location travel effect
                // via start_cyberspace, but the cyberspace SCREEN is the cyber world.
                Event::KeyPress(k) if k.detail == 29 => {
                    if engine.world_location_active() {
                        engine.leave_world();
                    } else if engine.visit_world("cyber", Path::new(assets)) {
                        if let Ok(ext) = std::fs::read(format!("{iso}/CYBER.EXT")) {
                            engine.set_world_ext(&ext);
                        }
                    }
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
                Event::VisibilityNotify(v) => {
                    win_visible =
                        v.state != x11rb::protocol::xproto::Visibility::FULLY_OBSCURED;
                }
                Event::ConfigureNotify(c) => {
                    if c.width > 0 && c.height > 0 && (c.width != win_w || c.height != win_h) {
                        win_w = c.width;
                        win_h = c.height;
                        image = vec![0u8; win_w as usize * win_h as usize * 4];
                        if let Some(g) = gpu.as_mut() {
                            g.resize(win_w as u32, win_h as u32);
                        }
                    }
                }
                Event::DestroyNotify(_) => return Ok(()),
                _ => {}
            }
        }
        // A click that arrived and released within one frame still presents as pressed
        // GAME TICK at the authentic rate (~15Hz); PRESENT at display refresh.
        // Between ticks the GPU path re-renders with the hand at the live cursor
        // (geometry follows the mouse at 60fps+; sim + tweens stay game-rate).
        // MEASURED game rate: 21.6 fps at the hub (FRAMERATE probe: VGA page flips
        // per PIT-timed second in the interpreter) -> 46 ms per tick.
        let tick_due = last_tick.elapsed() >= Duration::from_millis(46);
        if !tick_due {
            if let (Some(g), true) = (gpu.as_mut(), win_visible) {
                let alpha =
                    (last_tick.elapsed().as_secs_f32() / 0.046).clamp(0.0, 1.0);
                engine.refresh_gpu_hand(mx, my, alpha);
                let tris: Vec<commander_blood_tools::gpu::HandTri> = engine
                    .gpu_hand
                    .take()
                    .unwrap_or_default()
                    .into_iter()
                    .map(commander_blood_tools::gpu::HandTri)
                    .collect();
                let stars = engine.gpu_stars.clone().unwrap_or_default();
                let colorkey = engine.gpu_bg_colorkey;
                if let Err(e) =
                    g.present(&engine.framebuffer, &engine.scene_palette, &tris, &stars, colorkey)
                {
                    eprintln!("[gpu] present failed ({e}); reverting to software");
                    gpu = None;
                    engine.gpu_hand_enabled = false;
                }
            }
            // Flush queued X requests (pointer warps, the Right-Shift unlock's
            // ungrab/cursor restore) — the fast path otherwise never sends them.
            conn.flush()?;
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        last_tick = std::time::Instant::now();
        // for this step, so the engine's edge-triggered nav select fires.
        let step_buttons = if clicked { 1 } else { buttons };
        clicked = false;
        // Steering motion: locked = raw pointer deltas (the DOS mickey model).
        // Unlocked = position deltas, PLUS edge-push — while the cursor rides the
        // horizontal edge of the game area, synthesize continued ring motion (the
        // DOS mouse would keep yielding mickeys there; an unlocked X cursor cannot).
        let (mut step_dx, step_dy) = if pointer_locked {
            (raw_dx, raw_dy)
        } else {
            (mx as i32 - prev_mx, my as i32 - prev_my)
        };
        if !pointer_locked {
            if mx <= 1 {
                step_dx -= 8;
            } else if mx as usize >= src_w - 2 {
                step_dx += 8;
            }
        }
        (prev_mx, prev_my) = (mx as i32, my as i32);
        engine.step(MouseInput {
            x: mx,
            y: my,
            buttons: step_buttons,
            dx: step_dx,
            dy: step_dy,
        });
        (raw_dx, raw_dy) = (0, 0);
        // STATE-COUNTDOWN BEAT (0x8AA): tick state[0..0x1E) at the game's divided
        // rate while idle — expiring countdowns release GUARD state[i]==0 blocks
        // (SCRIPT2 @2744 queues the Scruter interception this way).
        countdown_accum += 8.011 / 70.0;
        if countdown_accum >= 1.0 {
            countdown_accum -= 1.0;
            if engine.dialogue_finished() && !engine.intro_active() {
                if let Some(m) = script_vm.borrow_mut().as_mut() {
                    m.tick_state_countdowns();
                }
            }
        }
        // SCRIPT1 VM continuation: when the engine finishes the queued lines, run the
        // next script frame — more lines may emit (multi-beat presentations), a D2
        // profile handoff may fire (the tutorial->SCRIPT2 chain), or nothing happens
        // (the console idles awaiting a click, exactly as the game does).
        if !engine.on_ship && !engine.intro_active() && engine.dialogue_finished() {
            let mut new_lines: Vec<(String, Option<std::path::PathBuf>, bool)> = Vec::new();
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
                // Tutorial auto-chain: current presenter done, nothing new — start the
                // next one (Honk's welcome, then the menu demo), as the real game does.
                if new_lines.is_empty() && profile.is_none() {
                    let next = tutorial_chain.borrow_mut().pop();
                    if let Some(actor) = next {
                        m.start_actor_presentation(actor, 40);
                        for ev in m.run_frame() {
                            if let commander_blood_tools::vm::VmEvent::Text { offset } = ev {
                                if let Some(l) = map.get(&offset) {
                                    new_lines.push(l.clone());
                                }
                            }
                        }
                    }
                }
            }
            if profile == Some(-100) {
                // The script's own ENDING (SCRIPT5's Bigbang-concert finale: fin.hnm).
                *script_vm.borrow_mut() = None;
                engine.start_ending();
                music.play(&format!("{assets}/mu/credits.voc"));
            } else if let Some(p) = profile {
                // The script's own D2 handoff: profile p -> SCRIPT{p+1} (profile 1 = SCRIPT2).
                let next = (p.max(0) as u32) + 1;
                *script_vm.borrow_mut() = None;
                current_script.set(0);
                load_script(&mut engine, &mut music, next);
            } else if !new_lines.is_empty() {
                set_vm_dialogue(&mut engine, new_lines);
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
            // The cyberspace traversal reached its destination: BIONIUM collected —
            // increment the script's vbio record (the BIOXX->Mantas->BIONIUM loop;
            // Bob's cryobox blocks branch on vbio==0/1/2 and acknowledge vbio>0) —
            // then return to the bridge hub.
            if let Some(m) = script_vm.borrow_mut().as_mut() {
                // vbio = record 0x126C (the C0 guard operands @0570/@0616/@0BD3).
                m.add_record(0x126C, 1);
            }
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
        if let (Some(g), true) = (gpu.as_mut(), win_visible) {
            // GPU path: background quad + the exported hand triangles at window
            // resolution. Falls back to software on surface errors (e.g. lost swapchain).
            let tris: Vec<commander_blood_tools::gpu::HandTri> = engine
                .gpu_hand
                .take()
                .unwrap_or_default()
                .into_iter()
                .map(commander_blood_tools::gpu::HandTri)
                .collect();
            let stars = engine.gpu_stars.clone().unwrap_or_default();
            let colorkey = engine.gpu_bg_colorkey;
            if let Err(e) = g.present(
                &engine.framebuffer,
                &engine.scene_palette,
                &tris,
                &stars,
                colorkey,
            ) {
                eprintln!("[gpu] present failed ({e}); reverting to software");
                gpu = None;
                engine.gpu_hand_enabled = false;
            }
        } else {
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
            // The game draws its OWN cursor (the pointing hand — engine.draw_hand_at_mouse),
            // exactly like the original: no host-drawn cursor overlay at all.
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
        }
        conn.flush()?;
        std::thread::sleep(Duration::from_millis(1));
        // Headless-safety: exit after a bounded run if no display consumer.
        frames_since_input += 1;
        if frames_since_input > 100_000 {
            return Ok(());
        }
    }
}
