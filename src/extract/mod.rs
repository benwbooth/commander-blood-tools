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
// Subtitle reveal rate. The game reveals one char per `gs:0xACA/4` frames
// (BLOODPRG.EXE dialogue updater @0x94B4), where `gs:0xACA = (textspeed/2)+1`.
// rate = 4 * frame_rate / gs:0xACA chars/sec; ~12/s at ~15fps and a mid text
// speed (gs:0xACA≈5). See re/REVERSE.md "Subtitle REVEAL TIMING".
const SUBTITLE_CHARS_PER_SEC: f64 = 12.0;
// A voiceless dialogue line (0xA6 b3==0xFF: radio-receiver / narrator / menu text
// the player still saw on-screen, with no son.snd voice clip — see re/REVERSE.md
// "voice clip-index") is rendered subtitle-only: its text over the scene
// background, with no talking-head HNM and no voice. Its on-screen duration =
// reveal time (SUBTITLE_CHARS_PER_SEC, the RE-derived rate) + a fixed readable
// hold. The game holds such a line until player input, which is not statically
// knowable, so the hold/min below are a presentation choice (documented), not a
// recovered constant.
const SILENT_SUBTITLE_HOLD_SEC: f64 = 1.5;
const SILENT_SUBTITLE_MIN_SEC: f64 = 2.0;
// Sample rate used to generate silence for a subtitle-only scene that has no
// voiced clip to inherit a rate from (any rate works for silence; this just
// keeps the concatenated u8 PCM track well-formed).
const SILENT_SUBTITLE_SR: u32 = 11025;
const SCRIPT_OBJECT_TALK_FIELD: u16 = 0x3a;
const SCRIPT_OBJECT_LOCATION_FIELD: usize = 24;

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
            let script_resource_profiles = bloodprg.script_resource_profiles()?;
            write_bloodprg_snd_call_sites_manifest(
                &snd_call_sites,
                &out_dir.join("bloodprg-snd-call-sites.tsv"),
            )?;
            write_bloodprg_render_call_sites_manifest(
                &render_call_sites,
                &out_dir.join("bloodprg-render-call-sites.tsv"),
            )?;
            eprintln!(
                "Recovered {} BLOODPRG.EXE SND entry call sites",
                snd_call_sites.len()
            );
            eprintln!(
                "Recovered {} BLOODPRG.EXE render call sites",
                render_call_sites.len()
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
    BloodPrg, RenderCallSite, ScriptResourceProfile, SndEntryCallSite,
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
