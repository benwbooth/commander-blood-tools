use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const ISO_URL: &str =
    "https://archive.org/download/Commander_Blood_-_MS-DOS_Game_-_MindscapeEng/CMDR_BLOOD.iso";

const VIEWPORT_W: usize = 320;
const VIEWPORT_H: usize = 200;
const OUTPUT_SCALE: usize = 3;
const OUTPUT_W: usize = VIEWPORT_W * OUTPUT_SCALE;
const OUTPUT_H: usize = VIEWPORT_H * OUTPUT_SCALE;
const HNM_FPS: u32 = 15;
// The exported videos use the default/mid text-speed step observed in the
// binary-derived notes. `subtitle_reveal_chars_per_second` maps this through the
// dialogue updater formula (`4 * frame_rate / gs:0x0ACA`), keeping subtitle
// reveal drawing and line-complete chatter on the same timing source.
const DEFAULT_SUBTITLE_TEXT_SPEED_STEP: u16 = 5;
// A voiceless dialogue line (0xA6 b3==0xFF: radio-receiver / narrator / menu text
// the player still saw on-screen, with no son.snd voice clip — see re/REVERSE.md
// "voice clip-index") is rendered subtitle-only: its text over the scene
// background, with no talking-head HNM and no voice. Its on-screen duration =
// reveal time (the RE-derived rate) + the decoded line-complete hold timer.
// Sample rate used to generate silence for a subtitle-only scene that has no
// voiced clip to inherit a rate from (any rate works for silence; this just
// keeps the concatenated u8 PCM track well-formed).
const SILENT_SUBTITLE_SR: u32 = 11025;
const SCRIPT_OBJECT_TALK_FIELD: u16 = 0x3a;
const SCRIPT_OBJECT_LOCATION_FIELD: usize = 24;

// The fixed leading entries of BLOODPRG.EXE's boot cutscene path table
// (0x10-byte records at file offset ~0x5C90: `sq\mind.HNM`, `sq\the_star.HNM`).
// These play at startup before the main loop; the trailing table slots are
// runtime-filled placeholders (`sq\xxxxxxxx`) and are not part of the fixed intro.
const INTRO_SEQUENCE: &[&str] = &["mind", "the_star"];

fn subtitle_reveal_chars_per_second(text_speed_step: u16) -> f64 {
    if text_speed_step == 0 {
        return HNM_FPS as f64;
    }
    4.0 * HNM_FPS as f64 / text_speed_step as f64
}

fn default_subtitle_reveal_chars_per_second() -> f64 {
    subtitle_reveal_chars_per_second(DEFAULT_SUBTITLE_TEXT_SPEED_STEP)
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: extract-commander-blood-audio [OUTPUT_DIR]");
        eprintln!("       extract-commander-blood-audio --hnm <file.hnm>... [-o DIR]");
        eprintln!("       extract-commander-blood-audio --snd <file.snd>... [-o DIR]");
        eprintln!();
        eprintln!("Downloads Commander Blood ISO from archive.org, extracts blood.dat,");
        eprintln!("decodes HNM(1) video files and converts audio to modern formats.");
        eprintln!("  Video (.hnm) -> MP4 (H.264 CRF 18) via built-in HNM(1) decoder");
        eprintln!("  Audio (.voc/.snd) -> FLAC (lossless) and M4A (AAC 128k)");
        eprintln!("  Direct --snd mode can also build legacy character-inspection MP4s");
        eprintln!();
        eprintln!("With --hnm, decode specific HNM files directly.");
        eprintln!("With --snd, decode SND voice banks; legacy character muxing uses");
        eprintln!("DESCRIPT.DES when it is present beside the extracted data root.");
        eprintln!();
        eprintln!("Requires: curl, 7z/7zz, ffmpeg (--hnm/--snd only need ffmpeg)");
        eprintln!("Default output dir: commander-blood-audio/");
        return Ok(());
    }

    // --hnm / --snd direct mode
    if args.iter().any(|a| a == "--hnm" || a == "--snd") {
        require("ffmpeg");
        let mode_hnm = args.iter().any(|a| a == "--hnm");
        let mut files = Vec::new();
        let mut out_dir = PathBuf::from(".");
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--hnm" | "--snd" => {}
                "-o" => {
                    if let Some(d) = iter.next() {
                        out_dir = PathBuf::from(d);
                    }
                }
                _ => files.push(PathBuf::from(arg)),
            }
        }
        fs::create_dir_all(&out_dir)?;
        if mode_hnm {
            for path in &files {
                let stem = path.file_stem().ok_or("no filename")?.to_string_lossy();
                let mp4_out = out_dir.join(format!("{stem}.mp4"));
                // In direct mode, no music lookup (no dat_dir context)
                match decode_hnm_to_mp4(path, &mp4_out, None) {
                    Ok(n) => eprintln!("{}: {n} frames -> {}", path.display(), mp4_out.display()),
                    Err(e) => {
                        eprintln!("{}: ERROR: {e}", path.display());
                        let _ = fs::remove_file(&mp4_out);
                    }
                }
            }
        } else {
            let flac_dir = out_dir.join("flac");
            let m4a_dir = out_dir.join("m4a");
            let mp4_dir = out_dir.join("mp4");
            fs::create_dir_all(&flac_dir)?;
            fs::create_dir_all(&m4a_dir)?;
            fs::create_dir_all(&mp4_dir)?;
            for path in &files {
                let stem = path.file_stem().ok_or("no filename")?.to_string_lossy();
                match decode_snd_clips(path, &stem, &flac_dir, &m4a_dir) {
                    Ok(n) => eprintln!("{}: {n} clips extracted", path.display()),
                    Err(e) => eprintln!("{}: ERROR: {e}", path.display()),
                }
                let dat_dir = path.parent().and_then(|p| p.parent());
                if let Some(dat_dir) = dat_dir {
                    let descript_db = dat_dir
                        .join("DESCRIPT.DES")
                        .exists()
                        .then(|| parse_descript(&dat_dir.join("DESCRIPT.DES")))
                        .transpose()?;
                    let hnm_music = descript_db
                        .as_ref()
                        .map(|db| db.hnm_music_map())
                        .unwrap_or_default();
                    let subtitle_sfx = dat_dir.join("sn").join("tb.snd");
                    match create_character_videos(
                        path,
                        &stem,
                        dat_dir,
                        &mp4_dir,
                        descript_db.as_ref(),
                        &hnm_music,
                        &[],
                        subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
                    ) {
                        Ok(n) if n > 0 => {
                            eprintln!("{}: {n} character video(s) created", path.display())
                        }
                        Ok(_) => {}
                        Err(e) => eprintln!("{}: video ERROR: {e}", path.display()),
                    }
                }
            }
        }
        return Ok(());
    }

    let out_dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("commander-blood-audio"));

    let sevenz = require_any(&["7z", "7zz"]);
    for cmd in ["curl", "ffmpeg"] {
        require(cmd);
    }

    let iso_path = out_dir.join("CMDR_BLOOD.iso");
    let tmp_iso = out_dir.join("_tmp_iso");
    let tmp_dat = out_dir.join("_tmp_dat");
    let flac_dir = out_dir.join("flac");
    let m4a_dir = out_dir.join("m4a");
    let mp4_dir = out_dir.join("mp4");
    let subtitle_dir = out_dir.join("subtitles");

    fs::create_dir_all(&flac_dir)?;
    fs::create_dir_all(&m4a_dir)?;
    fs::create_dir_all(&mp4_dir)?;

    // --- Download ISO (cached) ---
    if iso_path.exists() {
        eprintln!("Using cached ISO: {}", iso_path.display());
    } else {
        eprintln!("Downloading CMDR_BLOOD.iso (~475 MB)...");
        let ok = Command::new("curl")
            .args(["-L", "--progress-bar", "-o"])
            .arg(&iso_path)
            .arg(ISO_URL)
            .status()?
            .success();
        if !ok {
            return Err("download failed".into());
        }
    }

    // --- Extract blood.dat from ISO ---
    eprintln!("Extracting ISO...");
    let _ = fs::remove_dir_all(&tmp_iso);
    fs::create_dir_all(&tmp_iso)?;

    let ok = Command::new(&sevenz)
        .args(["x", "-y", "-bso0", "-bsp0"])
        .arg(format!("-o{}", tmp_iso.display()))
        .arg(&iso_path)
        .status()?
        .success();
    if !ok {
        return Err("ISO extraction failed".into());
    }

    let blood_dat =
        find_file_recursive(&tmp_iso, "blood.dat").ok_or("blood.dat not found in ISO")?;
    eprintln!("Found: {}", blood_dat.display());

    let script_resource_profiles =
        if let Some(bloodprg_path) = find_file_recursive(&tmp_iso, "BLOODPRG.EXE") {
            eprintln!("Parsing: {}", bloodprg_path.display());
            let bloodprg = BloodPrg::parse_file(&bloodprg_path)?;
            let snd_call_sites = bloodprg.snd_entry_call_sites();
            let render_call_sites = bloodprg.render_call_sites();
            let sprite_blitter_dispatch = bloodprg.sprite_blitter_dispatch_entries()?;
            let script_resource_profiles = bloodprg.script_resource_profiles()?;
            write_bloodprg_snd_call_sites_manifest(
                &snd_call_sites,
                &out_dir.join("bloodprg-snd-call-sites.tsv"),
            )?;
            write_bloodprg_render_call_sites_manifest(
                &render_call_sites,
                &out_dir.join("bloodprg-render-call-sites.tsv"),
            )?;
            write_bloodprg_sprite_blitter_manifest(
                &sprite_blitter_dispatch,
                &out_dir.join("bloodprg-sprite-blitters.tsv"),
            )?;
            eprintln!(
                "Recovered {} BLOODPRG.EXE SND entry call sites",
                snd_call_sites.len()
            );
            eprintln!(
                "Recovered {} BLOODPRG.EXE render call sites",
                render_call_sites.len()
            );
            eprintln!(
                "Recovered {} BLOODPRG.EXE sprite blitter modes",
                sprite_blitter_dispatch.len()
            );
            script_resource_profiles
        } else {
            Vec::new()
        };

    let descript_db = if let Some(descript_path) = find_file_recursive(&tmp_iso, "DESCRIPT.DES") {
        eprintln!("Parsing: {}", descript_path.display());
        let db = parse_descript(&descript_path)?;
        let _ = fs::copy(&descript_path, out_dir.join("DESCRIPT.DES"));
        let subtitle_count = write_descript_subtitles(&db, &subtitle_dir)?;
        write_descript_manifest(&db, &out_dir.join("descript-scenes.tsv"))?;
        eprintln!(
            "Parsed {} DESCRIPT.DES records ({subtitle_count} subtitle sidecars)",
            db.records.len()
        );
        Some(db)
    } else {
        eprintln!("DESCRIPT.DES not found in ISO; falling back to raw media extraction");
        None
    };
    let hnm_music = descript_db
        .as_ref()
        .map(|db| db.hnm_music_map())
        .unwrap_or_default();
    let script_character_contexts = if let Some(db) = &descript_db {
        parse_script_character_contexts(&tmp_iso, db, &hnm_music)?
    } else {
        Vec::new()
    };
    if let Some(db) = &descript_db {
        write_character_manifest(
            db,
            &script_character_contexts,
            &out_dir.join("character-combinations.tsv"),
        )?;
    }

    let script_speech = parse_script_speech(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_speech_manifest(&script_speech, &out_dir.join("script-speech.tsv"))?;
    let script_text_flags = parse_script_text_flags(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_text_flags_manifest(&script_text_flags, &out_dir.join("script-text-flags.tsv"))?;
    let script_executed_speech =
        parse_script_executed_speech(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_executed_speech_manifest(
        &script_executed_speech,
        &out_dir.join("script-executed-dialogue.tsv"),
    )?;
    write_script_executed_dialogue_runs_manifest(
        &script_executed_speech,
        &out_dir.join("script-executed-dialogue-runs.tsv"),
    )?;
    write_script_scene_events_manifest(
        &script_executed_speech,
        &out_dir.join("script-scene-events.tsv"),
    )?;
    let script_profile_sequence = parse_script_profile_sequence(
        &tmp_iso,
        &script_resource_profiles,
        descript_db.as_ref(),
        &hnm_music,
    )?;
    write_script_profile_runs_manifest(
        &script_profile_sequence.runs,
        &out_dir.join("script-profile-runs.tsv"),
    )?;
    write_script_profile_executed_speech_manifest(
        &script_profile_sequence.dialogue,
        &out_dir.join("script-profile-executed-dialogue.tsv"),
    )?;
    write_script_profile_dialogue_runs_manifest(
        &script_profile_sequence.dialogue,
        &out_dir.join("script-profile-dialogue-runs.tsv"),
    )?;
    write_script_profile_scene_events_manifest(
        &script_profile_sequence.dialogue,
        &out_dir.join("script-profile-scene-events.tsv"),
    )?;
    write_script_dialogue_runs_manifest(&script_speech, &out_dir.join("script-dialogue-runs.tsv"))?;
    let script_disassembly = parse_script_disassembly(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_disassembly_manifest(
        &script_disassembly,
        &out_dir.join("script-disassembly.tsv"),
    )?;
    let script_branch_trace = parse_script_branch_trace(&tmp_iso, descript_db.as_ref())?;
    write_script_branch_trace_manifest(
        &script_branch_trace,
        &out_dir.join("script-branch-trace.tsv"),
    )?;
    let script_post_update = parse_script_post_update(&tmp_iso, descript_db.as_ref())?;
    write_script_post_update_manifest(
        &script_post_update,
        &out_dir.join("script-post-update.tsv"),
    )?;
    write_script_branch_decisions_manifest(
        &script_branch_trace,
        &out_dir.join("script-branch-decisions.tsv"),
    )?;
    write_script_branch_coverage_manifest(
        &script_speech,
        &script_executed_speech,
        &script_branch_trace,
        &out_dir.join("script-branch-coverage.tsv"),
    )?;
    let script_branch_scenarios =
        parse_script_branch_scenarios(&tmp_iso, &script_branch_trace, descript_db.as_ref())?;
    write_script_branch_scenarios_manifest(
        &script_branch_scenarios,
        &out_dir.join("script-branch-scenarios.tsv"),
    )?;
    let script_branch_scenario_speech = parse_script_branch_scenario_speech(
        &tmp_iso,
        descript_db.as_ref(),
        &hnm_music,
        &script_branch_scenarios,
    )?;
    write_script_branch_scenario_speech_manifest(
        &script_branch_scenario_speech,
        &out_dir.join("script-branch-scenario-dialogue.tsv"),
    )?;
    write_script_branch_scenario_dialogue_runs_manifest(
        &script_branch_scenario_speech,
        &out_dir.join("script-branch-scenario-dialogue-runs.tsv"),
    )?;
    write_script_scene_events_manifest(
        &script_branch_scenario_speech,
        &out_dir.join("script-branch-scenario-scene-events.tsv"),
    )?;
    if !script_speech.is_empty() {
        eprintln!(
            "Recovered {} script text calls ({} text-flag rows, {} executed lines, {} profile-sequence lines, {} disassembly rows, {} branch events, {} post-update events, {} branch scenarios, {} scenario dialogue lines)",
            script_speech.len(),
            script_text_flags.len(),
            script_executed_speech.len(),
            script_profile_sequence.dialogue.len(),
            script_disassembly.len(),
            script_branch_trace.len(),
            script_post_update.len(),
            script_branch_scenarios.len(),
            script_branch_scenario_speech.len()
        );
    }

    // --- Parse blood.dat and extract all files ---
    eprintln!("Extracting blood.dat contents...");
    let _ = fs::remove_dir_all(&tmp_dat);
    fs::create_dir_all(&tmp_dat)?;
    let count = extract_dat(&blood_dat, &tmp_dat)?;
    eprintln!("Extracted {count} files from blood.dat");
    // The `.spr` sprite banks (ship-3D nav sprites like BORXX/BTV/BHYPER and the
    // character sprites SCRUTER/JERRY/MAXXON/IZWALITO/...) are loose files on the
    // ISO root, not inside blood.dat, and `_tmp_iso` is deleted below. Copy them
    // into `_tmp_dat/spr/` so they are preserved and picked up by the sprite
    // frame-table manifest (they parse with the recovered SpriteSlotFrameTable
    // layout). See re/REVERSE.md "SPRITE PIXEL-DATA SOURCE".
    let spr_dir = tmp_dat.join("spr");
    fs::create_dir_all(&spr_dir)?;
    let mut spr_copied = 0usize;
    for path in walk_files(&tmp_iso) {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("spr"))
            .unwrap_or(false)
        {
            if let Some(name) = path.file_name() {
                if fs::copy(&path, spr_dir.join(name)).is_ok() {
                    spr_copied += 1;
                }
            }
        }
    }
    eprintln!("Copied {spr_copied} .spr sprite banks from the ISO");
    let sprite_frame_rows = parse_sprite_frame_tables_manifest(&tmp_dat)?;
    write_sprite_frame_tables_manifest(
        &sprite_frame_rows,
        &out_dir.join("sprite-frame-tables.tsv"),
    )?;
    if !sprite_frame_rows.is_empty() {
        let sprite_file_count = sprite_frame_rows
            .iter()
            .map(|row| row.path.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        eprintln!(
            "Recovered {} sprite frame-table rows from {} SPR files",
            sprite_frame_rows.len(),
            sprite_file_count
        );
    }

    let _ = fs::remove_dir_all(&tmp_iso);

    // --- Decode verified DESCRIPT video scenes ---
    let mut video_converted = 0u32;
    let mut video_errors = 0u32;
    if let Some(db) = &descript_db {
        let scenes = db.video_scenes(&hnm_music);
        let subtitle_sfx = tmp_dat.join("sn").join("tb.snd");
        write_verified_video_manifest(
            &scenes,
            &out_dir.join("verified-video-scenes.tsv"),
            Some(&tmp_dat),
        )?;
        eprintln!(
            "Found {} DESCRIPT video scenes, decoding verified combinations...",
            scenes.len()
        );

        for scene in &scenes {
            let hnm_paths: Vec<_> = scene
                .hnms
                .iter()
                .map(|hnm| tmp_dat.join(descript_hnm_path(hnm, scene.kind)))
                .collect();
            if hnm_paths.iter().any(|path| !path.exists()) {
                video_errors += 1;
                eprintln!("[video ERROR] {}: missing HNM asset", scene.record_name);
                continue;
            }

            let music_path = scene
                .music
                .as_ref()
                .map(|music| tmp_dat.join("mu").join(format!("{music}.voc")))
                .filter(|path| path.exists());

            let mp4_out = mp4_dir.join(format!(
                "scene - {}.mp4",
                safe_file_stem(&scene.record_name)
            ));
            match decode_hnm_scene_to_mp4(
                &hnm_paths,
                &mp4_out,
                music_path.as_deref(),
                &scene.subtitles,
                subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
            ) {
                Ok(frame_count) => {
                    video_converted += 1;
                    eprintln!(
                        "[video {video_converted}] {} ({} HNM, {frame_count} frames)",
                        scene.record_name,
                        scene.hnms.len()
                    );
                }
                Err(e) => {
                    video_errors += 1;
                    eprintln!("[video ERROR] {}: {e}", scene.record_name);
                    let _ = fs::remove_file(&mp4_out);
                }
            }
        }
    } else {
        let hnm_files: Vec<_> = walk_files(&tmp_dat)
            .into_iter()
            .filter(|p| {
                p.extension()
                    .map(|e| e.to_ascii_lowercase() == "hnm")
                    .unwrap_or(false)
            })
            .collect();
        eprintln!(
            "Found {} raw HNM video files, decoding without DESCRIPT verification...",
            hnm_files.len()
        );

        for path in &hnm_files {
            let rel = path.strip_prefix(&tmp_dat)?;
            let flat_name = rel
                .with_extension("")
                .to_string_lossy()
                .replace(['/', '\\'], " - ");
            let fname = path
                .file_name()
                .ok_or("missing filename")?
                .to_string_lossy();
            let mp4_out = mp4_dir.join(format!("{flat_name}.mp4"));

            match decode_hnm_to_mp4(path, &mp4_out, None) {
                Ok(frame_count) => {
                    video_converted += 1;
                    eprintln!("[video {video_converted}] {fname} ({frame_count} frames)");
                }
                Err(e) => {
                    video_errors += 1;
                    eprintln!("[video ERROR] {fname}: {e}");
                    let _ = fs::remove_file(&mp4_out);
                }
            }
        }
    }

    // --- Decode the boot/intro cutscene sequence ---
    // BLOODPRG.EXE holds a fixed-width path table (0x10-byte records) whose two
    // fixed leading entries are the studio logo and the intro cinematic:
    //   sq\mind.HNM   -> Mindscape logo (verified pixel-accurate against the real
    //                    game: accuracy/oracle-scenarios.tsv `intro-mind-frame01`)
    //   sq\the_star.HNM -> opening space cinematic
    // The remaining table slots are runtime placeholders (`sq\xxxxxxxx`). These
    // intro HNMs are real game presentation but are not DESCRIPT database scenes,
    // so the scene decode above skips them; render them here so the faithful
    // export includes the game's opening and the oracle has a stable path.
    for (index, stem) in INTRO_SEQUENCE.iter().enumerate() {
        let hnm_path = tmp_dat.join("sq").join(format!("{stem}.hnm"));
        if !hnm_path.exists() {
            eprintln!("[intro ERROR] {stem}: missing HNM asset");
            video_errors += 1;
            continue;
        }
        let mp4_out = mp4_dir.join(format!("intro - {:02} - {stem}.mp4", index + 1));
        match decode_hnm_to_mp4(&hnm_path, &mp4_out, None) {
            Ok(frame_count) => {
                video_converted += 1;
                eprintln!("[intro {}] {stem} ({frame_count} frames)", index + 1);
            }
            Err(e) => {
                video_errors += 1;
                eprintln!("[intro ERROR] {stem}: {e}");
                let _ = fs::remove_file(&mp4_out);
            }
        }
    }

    // --- Convert VOC audio files ---
    let voc_files: Vec<_> = walk_files(&tmp_dat)
        .into_iter()
        .filter(|p| {
            p.extension()
                .map(|e| e.to_ascii_lowercase() == "voc")
                .unwrap_or(false)
        })
        .collect();
    eprintln!("Found {} VOC audio files, converting...", voc_files.len());

    let mut audio_converted = 0u32;
    for path in &voc_files {
        let rel = path.strip_prefix(&tmp_dat)?;
        let flat_name = rel
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\'], " - ");
        let fname = path
            .file_name()
            .ok_or("missing filename")?
            .to_string_lossy();

        let flac_out = flac_dir.join(format!("{flat_name}.flac"));
        let flac_ok = run_ffmpeg(path, &flac_out, &[]);

        let m4a_out = m4a_dir.join(format!("{flat_name}.m4a"));
        let m4a_ok = run_ffmpeg(path, &m4a_out, &["-c:a", "aac", "-b:a", "128k"]);

        if flac_ok || m4a_ok {
            audio_converted += 1;
            eprintln!("[audio {audio_converted}] {fname}");
        }
    }

    // --- Decode SND voice banks ---
    let snd_files: Vec<_> = walk_files(&tmp_dat)
        .into_iter()
        .filter(|p| {
            p.extension()
                .map(|e| e.to_ascii_lowercase() == "snd")
                .unwrap_or(false)
        })
        .collect();
    eprintln!(
        "Found {} SND voice banks, extracting clips...",
        snd_files.len()
    );

    let mut snd_clips = 0u32;
    let mut dialogue_run_videos = 0u32;
    let mut profile_dialogue_run_videos = 0u32;
    let mut scenario_dialogue_run_videos = 0u32;
    for path in &snd_files {
        let rel = path.strip_prefix(&tmp_dat)?;
        let flat_name = rel
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\'], " - ");
        let fname = path
            .file_name()
            .ok_or("missing filename")?
            .to_string_lossy();

        match decode_snd_clips(path, &flat_name, &flac_dir, &m4a_dir) {
            Ok(n) => {
                snd_clips += n;
                eprintln!("[voice {fname}] {n} clips");
            }
            Err(e) => {
                eprintln!("[voice ERROR] {fname}: {e}");
            }
        }
    }

    if let Some(db) = descript_db.as_ref() {
        let subtitle_sfx = tmp_dat.join("sn").join("tb.snd");
        match create_executed_dialogue_run_videos(
            &tmp_dat,
            &mp4_dir,
            db,
            &script_executed_speech,
            subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
        ) {
            Ok(n) => {
                dialogue_run_videos = n;
                if n > 0 {
                    eprintln!("[dialogue runs] {n} run-level video(s) created");
                }
            }
            Err(e) => eprintln!("[dialogue runs ERROR] {e}"),
        }
        match create_profile_dialogue_run_videos(
            &tmp_dat,
            &mp4_dir,
            db,
            &script_profile_sequence.dialogue,
            subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
        ) {
            Ok(n) => {
                profile_dialogue_run_videos = n;
                if n > 0 {
                    eprintln!("[profile dialogue runs] {n} profile-sequence video(s) created");
                }
            }
            Err(e) => eprintln!("[profile dialogue runs ERROR] {e}"),
        }
        match create_executed_dialogue_run_videos(
            &tmp_dat,
            &mp4_dir,
            db,
            &script_branch_scenario_speech,
            subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
        ) {
            Ok(n) => {
                scenario_dialogue_run_videos = n;
                if n > 0 {
                    eprintln!("[branch scenario dialogue runs] {n} run-level video(s) created");
                }
            }
            Err(e) => eprintln!("[branch scenario dialogue runs ERROR] {e}"),
        }
    }

    // Cleanup temp extraction
    let _ = fs::remove_dir_all(&tmp_dat);

    if let Err(err) = write_html_index(&out_dir) {
        eprintln!("[index ERROR] {err}");
    }

    eprintln!();
    if video_converted > 0
        || audio_converted > 0
        || snd_clips > 0
        || dialogue_run_videos > 0
        || profile_dialogue_run_videos > 0
        || scenario_dialogue_run_videos > 0
    {
        eprintln!("Done!");
        if video_converted > 0 {
            eprintln!(
                "  {video_converted} video files -> {} ({video_errors} errors)",
                mp4_dir.display()
            );
        }
        if audio_converted > 0 {
            eprintln!(
                "  {audio_converted} music files -> FLAC: {}, M4A: {}",
                flac_dir.display(),
                m4a_dir.display()
            );
        }
        if snd_clips > 0 {
            eprintln!(
                "  {snd_clips} voice clips -> FLAC: {}, M4A: {}",
                flac_dir.display(),
                m4a_dir.display()
            );
        }
        if dialogue_run_videos > 0 {
            eprintln!(
                "  {dialogue_run_videos} executed dialogue run videos -> {}",
                mp4_dir.display()
            );
        }
        if profile_dialogue_run_videos > 0 {
            eprintln!(
                "  {profile_dialogue_run_videos} profile-sequence dialogue run videos -> {}",
                mp4_dir.display()
            );
        }
        if scenario_dialogue_run_videos > 0 {
            eprintln!(
                "  {scenario_dialogue_run_videos} branch scenario dialogue run videos -> {}",
                mp4_dir.display()
            );
        }
    } else {
        eprintln!("No media files found.");
    }
    eprintln!(
        "ISO cached at: {} (delete to save ~475 MB)",
        iso_path.display()
    );
    Ok(())
}

fn write_bloodprg_snd_call_sites_manifest(
    rows: &[SndEntryCallSite],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "file_offset\tsegment\toffset\ttarget_segment\ttarget_offset\tax_value\tax_source_file_offset\tax_source\tintervening_far_calls\tnote"
    )?;
    for row in rows {
        writeln!(
            file,
            "0x{:05x}\t{:04x}\t{:04x}\t{:04x}\t{:04x}\t{}\t{}\t{}\t{}\t{}",
            row.file_offset,
            row.segment,
            row.offset,
            row.target_segment,
            row.target_offset,
            row.ax_value
                .map(|value| format!("{value}"))
                .unwrap_or_default(),
            row.ax_source_file_offset
                .map(|file_offset| format!("0x{file_offset:05x}"))
                .unwrap_or_default(),
            row.ax_source,
            row.intervening_far_calls,
            clean_tsv(row.note),
        )?;
    }
    Ok(())
}

fn write_bloodprg_render_call_sites_manifest(
    rows: &[RenderCallSite],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "file_offset\tsegment\toffset\ttarget_segment\ttarget_offset\ttarget_file_offset\ttarget_name\tax_value\tax_source_file_offset\tax_source\tintervening_far_calls\tnote"
    )?;
    for row in rows {
        writeln!(
            file,
            "0x{:05x}\t{:04x}\t{:04x}\t{:04x}\t{:04x}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.file_offset,
            row.segment,
            row.offset,
            row.target_segment,
            row.target_offset,
            row.target_file_offset,
            row.target_name,
            row.ax_value
                .map(|value| format!("{value}"))
                .unwrap_or_default(),
            row.ax_source_file_offset
                .map(|file_offset| format!("0x{file_offset:05x}"))
                .unwrap_or_default(),
            row.ax_source,
            row.intervening_far_calls,
            clean_tsv(row.note),
        )?;
    }
    Ok(())
}

fn write_bloodprg_sprite_blitter_manifest(
    rows: &[SpriteBlitterDispatchEntry],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "mode\thandler_offset\thandler_file_offset\tname\tnote"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{:04x}\t0x{:05x}\t{}\t{}",
            row.mode,
            row.handler_offset,
            row.handler_file_offset,
            row.name,
            clean_tsv(row.note),
        )?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SpriteFrameTableManifestRow {
    path: String,
    parse_status: String,
    flags: Option<u16>,
    slot_state_flags: Option<u16>,
    dispatch_index: Option<u8>,
    frame_count: Option<usize>,
    frame_index: Option<usize>,
    frame_offset: Option<usize>,
    frame_length: Option<usize>,
    width: Option<u16>,
    height: Option<u16>,
    x_offset: Option<i16>,
    y_offset: Option<i16>,
}

fn parse_sprite_frame_tables_manifest(
    root: &Path,
) -> Result<Vec<SpriteFrameTableManifestRow>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for path in walk_files(root) {
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("spr"))
            .unwrap_or(false)
        {
            continue;
        }

        let rel_path = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        let data = fs::read(&path)?;
        if let Some(table) = SpriteSlotFrameTable::parse(&data) {
            if table.frames.is_empty() {
                rows.push(SpriteFrameTableManifestRow {
                    path: rel_path,
                    parse_status: "ok".to_string(),
                    flags: Some(table.flags),
                    slot_state_flags: Some(table.slot_state_flags()),
                    dispatch_index: Some(table.dispatch_index()),
                    frame_count: Some(0),
                    frame_index: None,
                    frame_offset: None,
                    frame_length: None,
                    width: None,
                    height: None,
                    x_offset: None,
                    y_offset: None,
                });
                continue;
            }

            for (frame_index, frame) in table.frames.iter().copied().enumerate() {
                let (width, height, x_offset, y_offset) = parse_sprite_frame_header(frame);
                rows.push(SpriteFrameTableManifestRow {
                    path: rel_path.clone(),
                    parse_status: "ok".to_string(),
                    flags: Some(table.flags),
                    slot_state_flags: Some(table.slot_state_flags()),
                    dispatch_index: Some(table.dispatch_index()),
                    frame_count: Some(table.frames.len()),
                    frame_index: Some(frame_index),
                    frame_offset: table.frame_offsets.get(frame_index).copied(),
                    frame_length: Some(frame.len()),
                    width,
                    height,
                    x_offset,
                    y_offset,
                });
            }
        } else {
            rows.push(SpriteFrameTableManifestRow {
                path: rel_path,
                parse_status: "invalid".to_string(),
                flags: None,
                slot_state_flags: None,
                dispatch_index: None,
                frame_count: None,
                frame_index: None,
                frame_offset: None,
                frame_length: None,
                width: None,
                height: None,
                x_offset: None,
                y_offset: None,
            });
        }
    }
    Ok(rows)
}

fn parse_sprite_frame_header(frame: &[u8]) -> (Option<u16>, Option<u16>, Option<i16>, Option<i16>) {
    if frame.len() < 8 {
        return (None, None, None, None);
    }

    (
        Some(u16::from_le_bytes([frame[0], frame[1]])),
        Some(u16::from_le_bytes([frame[2], frame[3]])),
        Some(i16::from_le_bytes([frame[4], frame[5]])),
        Some(i16::from_le_bytes([frame[6], frame[7]])),
    )
}

fn write_sprite_frame_tables_manifest(
    rows: &[SpriteFrameTableManifestRow],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "path\tparse_status\tflags\tslot_state_flags\tdispatch_index\tframe_count\tframe_index\tframe_offset\tframe_length\twidth\theight\tx_offset\ty_offset"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            clean_tsv(&row.path),
            row.parse_status,
            format_u16_hex(row.flags),
            format_u16_hex(row.slot_state_flags),
            format_u8_dec(row.dispatch_index),
            format_usize_dec(row.frame_count),
            format_usize_dec(row.frame_index),
            format_usize_hex(row.frame_offset),
            format_usize_dec(row.frame_length),
            format_u16_dec(row.width),
            format_u16_dec(row.height),
            format_i16_dec(row.x_offset),
            format_i16_dec(row.y_offset),
        )?;
    }
    Ok(())
}

fn format_u16_hex(value: Option<u16>) -> String {
    value
        .map(|value| format!("0x{value:04x}"))
        .unwrap_or_default()
}

fn format_usize_hex(value: Option<usize>) -> String {
    value
        .map(|value| format!("0x{value:05x}"))
        .unwrap_or_default()
}

fn format_u16_dec(value: Option<u16>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn format_u8_dec(value: Option<u8>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn format_usize_dec(value: Option<usize>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn format_i16_dec(value: Option<i16>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

mod audio;
mod character;
mod dat;
mod decompress;
mod descript;
mod helpers;
mod hnm;
mod html;
mod lbm;
mod render;
mod script;
mod subtitle_sfx;

use audio::*;
use character::*;
#[cfg(test)]
use commander_blood_tools::bloodprg::ScriptResourceProfileSlot;
use commander_blood_tools::bloodprg::{
    BloodPrg, RenderCallSite, ScriptResourceProfile, SndEntryCallSite, SpriteBlitterDispatchEntry,
};
use commander_blood_tools::snd::{SndBank, SndClip};
use commander_blood_tools::vm;
use dat::*;
use descript::*;
use helpers::*;
use hnm::*;
use html::*;
use render::*;
use script::*;
use subtitle_sfx::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_extract_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        env::temp_dir().join(format!("{name}-{}-{unique}", std::process::id()))
    }

    #[test]
    fn subtitle_reveal_rate_uses_binary_text_speed_step() {
        assert_eq!(subtitle_reveal_chars_per_second(5), 12.0);
        assert_eq!(
            default_subtitle_reveal_chars_per_second(),
            subtitle_reveal_chars_per_second(DEFAULT_SUBTITLE_TEXT_SPEED_STEP)
        );
        assert_eq!(subtitle_reveal_chars_per_second(4), 15.0);
        assert_eq!(subtitle_reveal_chars_per_second(6), 10.0);
    }

    fn sprite_frame(width: u16, height: u16, x_offset: i16, y_offset: i16, body: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&width.to_le_bytes());
        frame.extend_from_slice(&height.to_le_bytes());
        frame.extend_from_slice(&x_offset.to_le_bytes());
        frame.extend_from_slice(&y_offset.to_le_bytes());
        frame.extend_from_slice(body);
        frame
    }

    fn sprite_frame_table(flags: u16, frames: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&flags.to_le_bytes());
        data.extend_from_slice(&(frames.len() as u16).to_le_bytes());

        let mut next_frame_start = 4 + frames.len() * 4;
        for frame in frames {
            data.extend_from_slice(&((next_frame_start - 4) as u32).to_le_bytes());
            next_frame_start += frame.len();
        }

        for frame in frames {
            data.extend_from_slice(frame);
        }
        data
    }

    #[test]
    fn sprite_frame_tables_manifest_reports_parsed_and_invalid_spr_files() {
        let root = temp_extract_dir("commander-blood-sprite-manifest");
        let subdir = root.join("sprites");
        fs::create_dir_all(&subdir).expect("create temp sprite dir");

        let frame = sprite_frame(2, 1, -1, 3, &[1, 0xaa, 0xbb]);
        fs::write(
            subdir.join("BOB.SPR"),
            sprite_frame_table(0x0004, &[&frame]),
        )
        .expect("write valid sprite");

        let mut invalid = Vec::new();
        invalid.extend_from_slice(&0x0004u16.to_le_bytes());
        invalid.extend_from_slice(&1u16.to_le_bytes());
        invalid.extend_from_slice(&0u32.to_le_bytes());
        invalid.extend_from_slice(&frame);
        fs::write(root.join("BROKEN.SPR"), invalid).expect("write invalid sprite");

        let rows = parse_sprite_frame_tables_manifest(&root).expect("parse sprite manifest");
        let ok = rows
            .iter()
            .find(|row| row.path == "sprites/BOB.SPR")
            .expect("valid sprite row");
        assert_eq!(ok.parse_status, "ok");
        assert_eq!(ok.flags, Some(0x0004));
        assert_eq!(ok.slot_state_flags, Some(0x0087));
        assert_eq!(ok.dispatch_index, Some(3));
        assert_eq!(ok.frame_count, Some(1));
        assert_eq!(ok.frame_index, Some(0));
        assert_eq!(ok.frame_offset, Some(8));
        assert_eq!(ok.frame_length, Some(11));
        assert_eq!(ok.width, Some(2));
        assert_eq!(ok.height, Some(1));
        assert_eq!(ok.x_offset, Some(-1));
        assert_eq!(ok.y_offset, Some(3));

        let broken = rows
            .iter()
            .find(|row| row.path == "BROKEN.SPR")
            .expect("invalid sprite row");
        assert_eq!(broken.parse_status, "invalid");
        assert_eq!(broken.flags, None);

        let out_path = root.join("sprite-frame-tables.tsv");
        write_sprite_frame_tables_manifest(&rows, &out_path).expect("write manifest");
        let manifest = fs::read_to_string(&out_path).expect("read manifest");
        assert!(manifest.starts_with("path\tparse_status\tflags\tslot_state_flags"));
        assert!(
            manifest
                .contains("sprites/BOB.SPR\tok\t0x0004\t0x0087\t3\t1\t0\t0x00008\t11\t2\t1\t-1\t3")
        );
        assert!(manifest.contains("BROKEN.SPR\tinvalid\t"));

        fs::remove_dir_all(root).expect("remove temp sprite dir");
    }
}
