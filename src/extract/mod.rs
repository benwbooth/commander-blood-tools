use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::TryInto;
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
        eprintln!("  Character videos: voice + animation + background + music -> MP4");
        eprintln!();
        eprintln!("With --hnm, decode specific HNM files directly.");
        eprintln!("With --snd, decode SND voice banks; character muxing uses DESCRIPT.DES");
        eprintln!("when it is present beside the extracted data root.");
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
            &hnm_music,
            &script_character_contexts,
            &out_dir.join("character-combinations.tsv"),
        )?;
    }

    let script_speech = parse_script_speech(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_speech_manifest(&script_speech, &out_dir.join("script-speech.tsv"))?;
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
    write_script_dialogue_manifest(
        &script_executed_speech,
        &out_dir.join("script-dialogue-videos.tsv"),
    )?;
    write_script_dialogue_runs_manifest(&script_speech, &out_dir.join("script-dialogue-runs.tsv"))?;
    let script_disassembly = parse_script_disassembly(&tmp_iso, descript_db.as_ref(), &hnm_music)?;
    write_script_disassembly_manifest(
        &script_disassembly,
        &out_dir.join("script-disassembly.tsv"),
    )?;
    let script_branch_trace = parse_script_branch_trace(&tmp_iso)?;
    write_script_branch_trace_manifest(
        &script_branch_trace,
        &out_dir.join("script-branch-trace.tsv"),
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
    if !script_speech.is_empty() {
        eprintln!(
            "Recovered {} script text calls ({} executed lines, {} disassembly rows, {} branch events)",
            script_speech.len(),
            script_executed_speech.len(),
            script_disassembly.len(),
            script_branch_trace.len()
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

    // --- Decode SND voice banks and create character videos ---
    let snd_files: Vec<_> = walk_files(&tmp_dat)
        .into_iter()
        .filter(|p| {
            p.extension()
                .map(|e| e.to_ascii_lowercase() == "snd")
                .unwrap_or(false)
        })
        .collect();
    eprintln!(
        "Found {} SND voice banks, extracting and creating character videos...",
        snd_files.len()
    );

    let mut snd_clips = 0u32;
    let mut char_videos = 0u32;
    let mut dialogue_run_videos = 0u32;
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
        let char_name = path
            .file_stem()
            .ok_or("no stem")?
            .to_string_lossy()
            .to_string();

        match decode_snd_clips(path, &flat_name, &flac_dir, &m4a_dir) {
            Ok(n) => {
                snd_clips += n;
                eprintln!("[voice {fname}] {n} clips");
            }
            Err(e) => {
                eprintln!("[voice ERROR] {fname}: {e}");
            }
        }

        let subtitle_sfx = tmp_dat.join("sn").join("tb.snd");
        match create_character_videos(
            path,
            &char_name,
            &tmp_dat,
            &mp4_dir,
            descript_db.as_ref(),
            &hnm_music,
            &script_executed_speech,
            subtitle_sfx.exists().then_some(subtitle_sfx.as_path()),
        ) {
            Ok(n) => {
                char_videos += n;
                if n > 0 {
                    eprintln!("[character {char_name}] {n} video(s) created");
                }
            }
            Err(e) => eprintln!("[character ERROR] {char_name}: {e}"),
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
    }

    // Cleanup temp extraction
    let _ = fs::remove_dir_all(&tmp_dat);

    if let Err(err) = write_html_index(&out_dir) {
        eprintln!("[index ERROR] {err}");
    }

    eprintln!();
    if video_converted > 0 || audio_converted > 0 || snd_clips > 0 || dialogue_run_videos > 0 {
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
        if char_videos > 0 {
            eprintln!(
                "  {char_videos} character videos (voice+anim+bg+music) -> {}",
                mp4_dir.display()
            );
        }
        if dialogue_run_videos > 0 {
            eprintln!(
                "  {dialogue_run_videos} executed dialogue run videos -> {}",
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
use commander_blood_tools::vm;
use dat::*;
use descript::*;
use helpers::*;
use hnm::*;
use html::*;
use render::*;
use script::*;
use subtitle_sfx::*;
