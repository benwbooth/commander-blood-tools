use std::collections::{BTreeMap, HashMap};
use std::convert::TryInto;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const ISO_URL: &str =
    "https://archive.org/download/Commander_Blood_-_MS-DOS_Game_-_MindscapeEng/CMDR_BLOOD.iso";

const VIEWPORT_W: usize = 320;
const VIEWPORT_H: usize = 200;
const HNM_FPS: u32 = 15;
const SUBTITLE_CHARS_PER_SEC: f64 = 36.0;
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
    write_script_dialogue_manifest(&script_speech, &out_dir.join("script-dialogue-videos.tsv"))?;
    if !script_speech.is_empty() {
        eprintln!(
            "Recovered {} script dialogue/subtitle lines",
            script_speech.len()
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
            &script_speech,
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

    // Cleanup temp extraction
    let _ = fs::remove_dir_all(&tmp_dat);

    eprintln!();
    if video_converted > 0 || audio_converted > 0 || snd_clips > 0 {
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
    } else {
        eprintln!("No media files found.");
    }
    eprintln!(
        "ISO cached at: {} (delete to save ~475 MB)",
        iso_path.display()
    );
    Ok(())
}

// ===========================================================================
// DESCRIPT.DES parser
// ===========================================================================

#[derive(Clone, Debug)]
struct SubtitleCue {
    tick: u16,
    text: String,
}

#[derive(Clone, Debug)]
struct DescriptRecord {
    name: String,
    kind: u8,
    music: Vec<String>,
    full_hnms: Vec<String>,
    sequence_hnms: Vec<String>,
    idle_hnms: Vec<(u8, String)>,
    talk_hnms: Vec<(u8, String)>,
    snd: Option<String>,
    sprite: Option<String>,
    labels: Vec<String>,
    subtitles: Vec<SubtitleCue>,
}

#[derive(Clone, Debug)]
struct DescriptDb {
    records: Vec<DescriptRecord>,
}

#[derive(Clone, Debug)]
struct CharacterScene {
    record_name: String,
    talk_hnms: Vec<(u8, String)>,
}

#[derive(Clone, Debug)]
struct DescriptVideoScene {
    record_name: String,
    kind: u8,
    music: Option<String>,
    hnms: Vec<String>,
    subtitles: Vec<SubtitleCue>,
}

#[derive(Clone, Debug)]
struct ScriptSpeechLine {
    script: String,
    function_name: String,
    offset: usize,
    actor_record: Option<String>,
    param0: Option<u8>,
    param1: Option<u8>,
    clip_index: Option<usize>,
    background_record: Option<String>,
    background_hnm: Option<String>,
    background_music: Option<String>,
    source: String,
    text: String,
}

#[derive(Clone, Debug)]
struct ScriptActorRef {
    record_name: String,
    background_record: Option<String>,
    background_hnm: Option<String>,
    background_music: Option<String>,
    talk_count: usize,
}

#[derive(Clone, Debug)]
struct ScriptCharacterContextLine {
    script: String,
    actor_record: String,
    actor_object_offset: u16,
    actor_talk_ref: u16,
    background_record: Option<String>,
    background_hnm: Option<String>,
    background_music: Option<String>,
    source: String,
}

impl DescriptDb {
    fn record(&self, name: &str) -> Option<&DescriptRecord> {
        self.records
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(name))
    }

    fn character_names(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| record.kind == 2)
            .map(|record| record.name.to_ascii_lowercase())
            .collect()
    }

    fn hnm_music_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let mut inherited_location_music: Option<String> = None;

        for record in &self.records {
            if record.kind == 1 {
                let explicit_music = record.music.first().map(|music| media_stem(music));
                if let Some(music) = explicit_music {
                    inherited_location_music = Some(music.clone());
                    for hnm in &record.full_hnms {
                        map.insert(media_stem(hnm), music.clone());
                    }
                } else if let Some(music) = &inherited_location_music {
                    for hnm in &record.full_hnms {
                        map.entry(media_stem(hnm)).or_insert_with(|| music.clone());
                    }
                }
                continue;
            }

            if let Some(music) = record.music.first().map(|m| media_stem(m)) {
                for hnm in record.full_hnms.iter().chain(record.sequence_hnms.iter()) {
                    map.insert(media_stem(hnm), music.clone());
                }
            }
        }

        map
    }

    fn character_scenes_for_snd(&self, snd_stem: &str) -> Vec<CharacterScene> {
        let snd_stem = snd_stem.to_ascii_lowercase();
        self.records
            .iter()
            .filter(|record| record.kind == 2)
            .filter_map(|record| {
                let snd_name = record.snd.as_ref()?;
                if media_stem(snd_name) != snd_stem {
                    return None;
                }
                Some(CharacterScene {
                    record_name: record.name.clone(),
                    talk_hnms: record.talk_hnms.clone(),
                })
            })
            .collect()
    }

    fn video_scenes(&self, hnm_music: &HashMap<String, String>) -> Vec<DescriptVideoScene> {
        self.records
            .iter()
            .filter(|record| record.kind != 2)
            .filter_map(|record| {
                let hnms = if record.sequence_hnms.is_empty() {
                    record.full_hnms.clone()
                } else {
                    record.sequence_hnms.clone()
                };
                if hnms.is_empty() {
                    return None;
                }

                let music = record
                    .music
                    .first()
                    .map(|music| media_stem(music))
                    .or_else(|| {
                        hnms.first()
                            .and_then(|hnm| hnm_music.get(&media_stem(hnm)))
                            .cloned()
                    });

                Some(DescriptVideoScene {
                    record_name: record.name.clone(),
                    kind: record.kind,
                    music,
                    hnms,
                    subtitles: record.subtitles.clone(),
                })
            })
            .collect()
    }
}

fn parse_descript(path: &Path) -> Result<DescriptDb, Box<dyn Error>> {
    let data = fs::read(path)?;
    if data.len() < 2 {
        return Err("DESCRIPT.DES too small".into());
    }

    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let table_end = 2 + count * 18;
    if table_end > data.len() {
        return Err("DESCRIPT.DES index exceeds file size".into());
    }

    let mut records = Vec::with_capacity(count);
    for i in 0..count {
        let table_pos = 2 + i * 18;
        let name_len = data[table_pos..table_pos + 16]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(16);
        let name = String::from_utf8_lossy(&data[table_pos..table_pos + name_len]).to_string();
        let ptr = u16::from_le_bytes([data[table_pos + 16], data[table_pos + 17]]) as usize;
        if ptr == 0 || ptr + 2 > data.len() {
            continue;
        }

        let kind = data[ptr - 1];
        let len = u16::from_le_bytes([data[ptr], data[ptr + 1]]) as usize;
        let end = (ptr + len).min(data.len());
        let mut pos = ptr + 2;

        let mut record = DescriptRecord {
            name,
            kind,
            music: Vec::new(),
            full_hnms: Vec::new(),
            sequence_hnms: Vec::new(),
            idle_hnms: Vec::new(),
            talk_hnms: Vec::new(),
            snd: None,
            sprite: None,
            labels: Vec::new(),
            subtitles: Vec::new(),
        };

        while pos < end {
            let op = data[pos];
            pos += 1;
            match op {
                0x03 => {
                    pos += 1;
                    let _ = read_des_media(&data, &mut pos, end, ".lbm");
                }
                0x05 => record.labels.push(read_des_cstr(&data, &mut pos, end)),
                0x06 => record
                    .full_hnms
                    .push(read_des_media(&data, &mut pos, end, ".hnm")),
                0x07 => {
                    if pos >= end {
                        break;
                    }
                    let slot = data[pos];
                    pos += 1;
                    let hnm = read_des_media(&data, &mut pos, end, ".hnm");
                    record.talk_hnms.push((slot, hnm));
                }
                0x08 => pos = (pos + 2).min(end),
                0x09 | 0x0a => {
                    let _ = read_des_media(&data, &mut pos, end, ".hnm");
                }
                0x0b => {
                    if pos >= end {
                        break;
                    }
                    let slot = data[pos];
                    pos += 1;
                    let hnm = read_des_media(&data, &mut pos, end, ".hnm");
                    record.idle_hnms.push((slot, hnm));
                }
                0x0c => record
                    .sequence_hnms
                    .push(read_des_media(&data, &mut pos, end, ".hnm")),
                0x0d => {
                    // Location records use 0d 00 before the final HNM command. Cutscene
                    // subtitle cues use 0d + little-endian decisecond tick + text.
                    if pos + 1 < end && data[pos] == 0 && is_des_opcode(data[pos + 1]) {
                        pos += 1;
                    } else if pos + 2 <= end {
                        let tick = u16::from_le_bytes([data[pos], data[pos + 1]]);
                        pos += 2;
                        let text = read_des_cstr(&data, &mut pos, end);
                        record.subtitles.push(SubtitleCue { tick, text });
                    } else {
                        break;
                    }
                }
                0x0e => record.sprite = Some(read_des_media(&data, &mut pos, end, ".spr")),
                0x10 => record
                    .full_hnms
                    .push(read_des_media(&data, &mut pos, end, ".hnm")),
                0x11 => record.snd = Some(read_des_media(&data, &mut pos, end, ".snd")),
                0x12 => record
                    .music
                    .push(read_des_media(&data, &mut pos, end, ".voc")),
                0x04 => pos = (pos + 2).min(end),
                0x00 | 0x02 | 0xff => break,
                _ => break,
            }
        }

        records.push(record);
    }

    Ok(DescriptDb { records })
}

fn read_des_cstr(data: &[u8], pos: &mut usize, end: usize) -> String {
    let start = *pos;
    while *pos < end && data[*pos] != 0 {
        *pos += 1;
    }
    let text = String::from_utf8_lossy(&data[start..*pos])
        .replace('\r', "\n")
        .trim_end()
        .to_string();
    if *pos < end {
        *pos += 1;
    }
    text
}

fn read_des_media(data: &[u8], pos: &mut usize, end: usize, ext: &str) -> String {
    let start = *pos;
    let ext_bytes = ext.as_bytes();
    let mut media_end = end;
    let mut i = *pos;
    while i + ext_bytes.len() <= end {
        if data[i..i + ext_bytes.len()].eq_ignore_ascii_case(ext_bytes) {
            media_end = i + ext_bytes.len();
            break;
        }
        i += 1;
    }
    *pos = media_end;
    String::from_utf8_lossy(&data[start..media_end]).to_string()
}

fn is_des_opcode(byte: u8) -> bool {
    matches!(
        byte,
        0x00 | 0x02
            | 0x03
            | 0x04
            | 0x05
            | 0x06
            | 0x07
            | 0x08
            | 0x09
            | 0x0a
            | 0x0b
            | 0x0c
            | 0x0d
            | 0x0e
            | 0x10
            | 0x11
            | 0x12
            | 0xff
    )
}

fn media_stem(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_lowercase();
    match base.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => base,
    }
}

fn safe_file_stem(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' || ch == '-' {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn write_descript_subtitles(db: &DescriptDb, out_dir: &Path) -> Result<u32, Box<dyn Error>> {
    fs::create_dir_all(out_dir)?;
    let mut written = 0u32;

    for record in &db.records {
        let cues: Vec<_> = record
            .subtitles
            .iter()
            .filter(|cue| !cue.text.trim().is_empty())
            .collect();
        if cues.is_empty() {
            continue;
        }

        let path = out_dir.join(format!("{}.srt", safe_file_stem(&record.name)));
        let mut file = File::create(path)?;
        for (idx, cue) in cues.iter().enumerate() {
            let start = cue.tick as f64 / 10.0;
            let end = cues
                .get(idx + 1)
                .map(|next| next.tick as f64 / 10.0)
                .filter(|next| *next > start + 0.25)
                .unwrap_or(start + 4.0);
            writeln!(file, "{}", idx + 1)?;
            writeln!(
                file,
                "{} --> {}",
                format_srt_time(start),
                format_srt_time(end)
            )?;
            writeln!(file, "{}", cue.text.trim())?;
            writeln!(file)?;
        }
        written += 1;
    }

    Ok(written)
}

fn write_descript_manifest(db: &DescriptDb, out_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "record\tkind\tmusic\tfull_hnm\tsequence_hnm\tsnd\tidle_hnm\tchar_background_hnm\ttalk_hnm_count\tsubtitle_count"
    )?;
    for record in &db.records {
        let idle = record
            .idle_hnms
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let char_background = lookup_character_context(&record.name)
            .and_then(|ctx| ctx.background_hnm)
            .unwrap_or("");
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            record.name,
            record.kind,
            record.music.join(","),
            record.full_hnms.join(","),
            record.sequence_hnms.join(","),
            record.snd.as_deref().unwrap_or(""),
            idle,
            char_background,
            record.talk_hnms.len(),
            record.subtitles.len()
        )?;
    }
    Ok(())
}

fn write_verified_video_manifest(
    scenes: &[DescriptVideoScene],
    out_path: &Path,
    dat_dir: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "record\tkind\tmusic\thnm_count\thnms\tasset_status\tmissing_hnms\tsubtitle_count\tsubtitle_sfx"
    )?;
    for scene in scenes {
        let missing_hnms = dat_dir
            .map(|dat_dir| {
                scene
                    .hnms
                    .iter()
                    .filter(|hnm| !dat_dir.join(descript_hnm_path(hnm, scene.kind)).exists())
                    .map(|hnm| hnm.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            scene.record_name,
            scene.kind,
            scene.music.as_deref().unwrap_or(""),
            scene.hnms.len(),
            scene.hnms.join(","),
            if missing_hnms.is_empty() {
                "ok"
            } else {
                "missing_assets"
            },
            missing_hnms.join(","),
            scene.subtitles.len(),
            if scene.subtitles.is_empty() {
                ""
            } else {
                "sn/tb.snd#0"
            }
        )?;
    }
    Ok(())
}

fn write_character_manifest(
    db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
    script_contexts: &[ScriptCharacterContextLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "record\tsnd\tidle_hnm\tscript\tactor_object_offset\tactor_talk_ref\tbackground_record\tbackground_hnm\tbackground_music\tbackground_source\ttalk_hnm_count\ttalk_hnms"
    )?;
    for record in db.records.iter().filter(|record| record.kind == 2) {
        let idle = record
            .idle_hnms
            .iter()
            .map(|(_, hnm)| hnm.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let talk = record
            .talk_hnms
            .iter()
            .map(|(_, hnm)| hnm.as_str())
            .collect::<Vec<_>>()
            .join(",");

        let mut contexts = script_contexts
            .iter()
            .filter(|context| context.actor_record.eq_ignore_ascii_case(&record.name))
            .peekable();
        if contexts.peek().is_some() {
            for context in contexts {
                writeln!(
                    file,
                    "{}\t{}\t{}\t{}\t0x{:04x}\t0x{:04x}\t{}\t{}\t{}\t{}\t{}\t{}",
                    record.name,
                    record.snd.as_deref().unwrap_or(""),
                    idle,
                    context.script,
                    context.actor_object_offset,
                    context.actor_talk_ref,
                    context.background_record.as_deref().unwrap_or(""),
                    context.background_hnm.as_deref().unwrap_or(""),
                    context.background_music.as_deref().unwrap_or(""),
                    context.source,
                    record.talk_hnms.len(),
                    talk
                )?;
            }
            continue;
        }

        let context = lookup_character_context(&record.name);
        let background_hnm = context.and_then(|ctx| ctx.background_hnm).unwrap_or("");
        let background_music = context
            .and_then(|ctx| ctx.background_hnm)
            .and_then(|hnm| hnm_music.get(&media_stem(hnm)))
            .map(|music| music.as_str())
            .unwrap_or("");
        let source = match context {
            Some(ctx) if ctx.background_hnm.is_some() => {
                "legacy static fallback background; no SCRIPT object context recovered"
            }
            Some(_) => "legacy static fallback standalone; no SCRIPT object context recovered",
            None => "DESCRIPT foreground+voice only; no SCRIPT object context recovered",
        };

        writeln!(
            file,
            "{}\t{}\t{}\t\t\t\t\t{}\t{}\t{}\t{}\t{}",
            record.name,
            record.snd.as_deref().unwrap_or(""),
            idle,
            background_hnm,
            background_music,
            source,
            record.talk_hnms.len(),
            talk
        )?;
    }
    Ok(())
}

fn format_srt_time(seconds: f64) -> String {
    let millis = (seconds * 1000.0).round().max(0.0) as u64;
    let h = millis / 3_600_000;
    let m = (millis / 60_000) % 60;
    let s = (millis / 1_000) % 60;
    let ms = millis % 1_000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

fn descript_hnm_path(hnm_name: &str, kind: u8) -> PathBuf {
    let lower = match hnm_name.to_ascii_lowercase().as_str() {
        // DESCRIPT.DES' New Year advert drops the "b"; BLOOD.DAT only has the
        // matching Venusia advert asset as sq/pubven1.hnm.
        "puven1.hnm" => "pubven1.hnm".to_string(),
        other => other.to_string(),
    };
    if lower.contains('/') || lower.contains('\\') {
        return PathBuf::from(lower.replace('\\', "/"));
    }

    let dir = match kind {
        1 => "pl",
        2 => "pe",
        4 => "sq",
        15 => "ob",
        _ => "sq",
    };
    PathBuf::from(dir).join(lower)
}

fn parse_script_speech(
    iso_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptSpeechLine>, Box<dyn Error>> {
    let mut rows = Vec::new();
    let character_names = descript_db
        .map(|db| db.character_names())
        .unwrap_or_default();

    for script_idx in 1..=5 {
        let cod_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.COD"));
        let dic_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DIC"));
        let deb_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("SCRIPT{script_idx}.VAR"));
        let (Some(cod_path), Some(dic_path)) = (cod_path, dic_path) else {
            continue;
        };

        let cod = fs::read(cod_path)?;
        let words = parse_script_dictionary(&dic_path)?;
        let script = format!("SCRIPT{script_idx}");
        let (mut functions, actor_refs, _) =
            if let (Some(deb_path), Some(var_path), Some(db)) = (deb_path, var_path, descript_db) {
                parse_script_symbols(
                    &script,
                    &deb_path,
                    &var_path,
                    db,
                    hnm_music,
                    &character_names,
                )?
            } else {
                (Vec::new(), HashMap::new(), Vec::new())
            };
        if functions.is_empty() {
            functions.push((0, script.as_str().to_string()));
        }
        functions.sort_by_key(|(offset, _)| *offset);
        functions.push((cod.len(), "END".to_string()));

        for pair in functions.windows(2) {
            let (function_start, function_name) = (&pair[0].0, &pair[0].1);
            let function_end = pair[1].0.min(cod.len());
            if *function_start >= function_end {
                continue;
            }
            let mut current_actor: Option<ScriptActorRef> = None;
            let mut rel = 0usize;
            while *function_start + rel < function_end {
                let pos = *function_start + rel;
                if pos + 2 < function_end && cod[pos] == 0xc4 {
                    let addr = u16::from_le_bytes([cod[pos + 1], cod[pos + 2]]);
                    if let Some(actor) = actor_refs.get(&addr) {
                        current_actor = Some(actor.clone());
                    }
                }

                if !cod[pos..function_end].starts_with(b"\xa6\x0a\x07") {
                    rel += 1;
                    continue;
                }

                let Some(marker_rel) = cod[pos + 3..function_end.min(pos + 12)]
                    .iter()
                    .position(|&b| b == 0x80)
                else {
                    rel += 1;
                    continue;
                };
                let marker = pos + 3 + marker_rel;
                if marker < pos + 5 {
                    rel += 1;
                    continue;
                }

                let mut text_pos = marker + 1;
                let mut words_out = Vec::new();
                while text_pos + 1 < function_end {
                    let word_off = u16::from_le_bytes([cod[text_pos], cod[text_pos + 1]]);
                    text_pos += 2;
                    if word_off == 0 {
                        break;
                    }
                    let Some(word) = words.get(&word_off) else {
                        words_out.clear();
                        break;
                    };
                    words_out.push(word.as_str());
                }

                if words_out.is_empty() {
                    rel += 1;
                    continue;
                }

                let params = &cod[pos + 3..marker];
                let param0 = params.first().copied();
                let param1 = params.get(1).copied();
                let actor = current_actor.clone();
                let actor_speaks = actor.is_some() && param1.is_some_and(|style| style < 0x10);
                let clip_index = actor.as_ref().and_then(|actor| {
                    if !actor_speaks {
                        return None;
                    }
                    match (param0, param1) {
                        (Some(0xff), Some(idx)) if (idx as usize) < actor.talk_count => {
                            Some(idx as usize)
                        }
                        (Some(idx), _) if idx > 0 && (idx as usize) <= actor.talk_count => {
                            Some(idx as usize - 1)
                        }
                        _ => None,
                    }
                });
                let source = match (&actor, actor_speaks, clip_index) {
                    (Some(_), true, Some(_)) => {
                        "SCRIPT bytecode actor ref + DESCRIPT talk clip".to_string()
                    }
                    (Some(_), true, None) => {
                        "SCRIPT bytecode actor ref; subtitle has no mapped talk clip".to_string()
                    }
                    (Some(_), false, _) => {
                        "SCRIPT bytecode actor ref; non-character subtitle channel".to_string()
                    }
                    (None, _, _) => "SCRIPT subtitle text only".to_string(),
                };

                rows.push(ScriptSpeechLine {
                    script: script.clone(),
                    function_name: function_name.clone(),
                    offset: pos,
                    actor_record: actor.as_ref().map(|actor| actor.record_name.clone()),
                    param0,
                    param1,
                    clip_index,
                    background_record: actor
                        .as_ref()
                        .and_then(|actor| actor.background_record.clone()),
                    background_hnm: actor
                        .as_ref()
                        .and_then(|actor| actor.background_hnm.clone()),
                    background_music: actor
                        .as_ref()
                        .and_then(|actor| actor.background_music.clone()),
                    source,
                    text: words_out.join(" "),
                });

                rel += 1;
            }
        }
    }

    Ok(rows)
}

fn parse_script_character_contexts(
    iso_dir: &Path,
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
) -> Result<Vec<ScriptCharacterContextLine>, Box<dyn Error>> {
    let character_names = descript_db.character_names();
    let mut rows = Vec::new();

    for script_idx in 1..=5 {
        let script = format!("SCRIPT{script_idx}");
        let deb_path = find_file_recursive(iso_dir, &format!("{script}.DEB"));
        let var_path = find_file_recursive(iso_dir, &format!("{script}.VAR"));
        let (Some(deb_path), Some(var_path)) = (deb_path, var_path) else {
            continue;
        };
        let (_, _, contexts) = parse_script_symbols(
            &script,
            &deb_path,
            &var_path,
            descript_db,
            hnm_music,
            &character_names,
        )?;
        rows.extend(contexts);
    }

    rows.sort_by(|a, b| {
        a.actor_record
            .to_ascii_lowercase()
            .cmp(&b.actor_record.to_ascii_lowercase())
            .then(a.script.cmp(&b.script))
            .then(a.actor_object_offset.cmp(&b.actor_object_offset))
    });
    Ok(rows)
}

fn parse_script_symbols(
    script: &str,
    deb_path: &Path,
    var_path: &Path,
    descript_db: &DescriptDb,
    hnm_music: &HashMap<String, String>,
    character_names: &[String],
) -> Result<
    (
        Vec<(usize, String)>,
        HashMap<u16, ScriptActorRef>,
        Vec<ScriptCharacterContextLine>,
    ),
    Box<dyn Error>,
> {
    let deb = fs::read(deb_path)?;
    let var = fs::read(var_path)?;
    let mut object_names: HashMap<u16, String> = HashMap::new();
    let mut functions = Vec::new();

    for record in deb.chunks_exact(20) {
        let name_len = record[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&record[..name_len]).to_string();
        let offset = u16::from_le_bytes([record[16], record[17]]);
        let kind = u16::from_le_bytes([record[18], record[19]]);
        match kind {
            1 => {
                object_names.insert(offset, name);
            }
            2 if offset != 0xffff => functions.push((offset as usize, name)),
            _ => {}
        }
    }

    let mut actor_refs = HashMap::new();
    let mut contexts = Vec::new();
    for (&offset, name) in &object_names {
        if !character_names
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(name))
        {
            continue;
        }

        let Some(record) = descript_db.record(name) else {
            continue;
        };
        let var_offset = offset as usize;
        let location_offset = var
            .get(
                var_offset + SCRIPT_OBJECT_LOCATION_FIELD
                    ..var_offset + SCRIPT_OBJECT_LOCATION_FIELD + 2,
            )
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]));
        let background_record = location_offset.and_then(|loc| object_names.get(&loc).cloned());
        let background = background_record
            .as_ref()
            .and_then(|loc_name| descript_db.record(loc_name))
            .filter(|record| record.kind == 1);
        let background_hnm = background.and_then(|record| record.full_hnms.first().cloned());
        let background_music = background_hnm
            .as_ref()
            .and_then(|hnm| hnm_music.get(&media_stem(hnm)).cloned())
            .or_else(|| {
                background
                    .and_then(|record| record.music.first())
                    .map(|music| media_stem(music))
            });

        let actor_talk_ref = offset.saturating_add(SCRIPT_OBJECT_TALK_FIELD);
        contexts.push(ScriptCharacterContextLine {
            script: script.to_string(),
            actor_record: record.name.clone(),
            actor_object_offset: offset,
            actor_talk_ref,
            background_record: background_record.clone(),
            background_hnm: background_hnm.clone(),
            background_music: background_music.clone(),
            source: format!("{script}.DEB object + {script}.VAR object location field"),
        });
        actor_refs.insert(
            actor_talk_ref,
            ScriptActorRef {
                record_name: record.name.clone(),
                background_record,
                background_hnm,
                background_music,
                talk_count: record.talk_hnms.len(),
            },
        );
    }

    contexts.sort_by(|a, b| {
        a.actor_record
            .to_ascii_lowercase()
            .cmp(&b.actor_record.to_ascii_lowercase())
            .then(a.actor_object_offset.cmp(&b.actor_object_offset))
    });

    Ok((functions, actor_refs, contexts))
}

fn write_script_speech_manifest(
    rows: &[ScriptSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "script\tfunction\toffset\tactor\tparam0\tparam1\tclip_index\tbackground_record\tbackground_hnm\tbackground_music\tsource\ttext"
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\t{}\t0x{:05x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.script,
            row.function_name,
            row.offset,
            row.actor_record.as_deref().unwrap_or(""),
            row.param0
                .map(|param| format!("{param:02x}"))
                .unwrap_or_default(),
            row.param1
                .map(|param| format!("{param:02x}"))
                .unwrap_or_default(),
            row.clip_index
                .map(|idx| idx.to_string())
                .unwrap_or_default(),
            row.background_record.as_deref().unwrap_or(""),
            row.background_hnm.as_deref().unwrap_or(""),
            row.background_music.as_deref().unwrap_or(""),
            row.source,
            row.text.replace('\t', " ")
        )?;
    }
    Ok(())
}

fn write_script_dialogue_manifest(
    rows: &[ScriptSpeechLine],
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut groups: BTreeMap<(String, String, String), Vec<&ScriptSpeechLine>> = BTreeMap::new();
    for row in rows {
        let Some(actor) = row.actor_record.as_ref() else {
            continue;
        };
        if row.clip_index.is_none() {
            continue;
        }
        groups
            .entry((row.script.clone(), row.function_name.clone(), actor.clone()))
            .or_default()
            .push(row);
    }

    let mut file = File::create(out_path)?;
    writeln!(
        file,
        "mp4\tscript\tfunction\tactor\tbackground_record\tbackground_hnm\tbackground_music\tline_count\tclip_indices"
    )?;
    for ((script, function_name, actor), mut lines) in groups {
        lines.sort_by_key(|line| line.offset);
        let output_stem = format!(
            "dialogue - {} - {} - {}",
            safe_file_stem(&script),
            safe_file_stem(&function_name),
            safe_file_stem(&actor)
        );
        let clip_indices = lines
            .iter()
            .filter_map(|line| line.clip_index.map(|idx| idx.to_string()))
            .collect::<Vec<_>>()
            .join(",");
        let first = lines[0];
        writeln!(
            file,
            "{}.mp4\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            output_stem,
            script,
            function_name,
            actor,
            first.background_record.as_deref().unwrap_or(""),
            first.background_hnm.as_deref().unwrap_or(""),
            first.background_music.as_deref().unwrap_or(""),
            lines.len(),
            clip_indices
        )?;
    }

    Ok(())
}

fn parse_script_dictionary(path: &Path) -> Result<HashMap<u16, String>, Box<dyn Error>> {
    let data = fs::read(path)?;
    let mut words = HashMap::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let start = pos;
        while pos < data.len() && data[pos] != 0 {
            pos += 1;
        }
        if pos > start {
            words.insert(
                start as u16,
                String::from_utf8_lossy(&data[start..pos]).to_string(),
            );
        }
        pos += 1;
    }
    Ok(words)
}

// ===========================================================================
// HNM(1) file parser — shared between all decoders
// ===========================================================================

struct HnmFile {
    data: Vec<u8>,
    header_size: usize,
    palette: [[u8; 3]; 256],
    offsets: Vec<u32>,
}

impl HnmFile {
    fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let data = fs::read(path)?;
        if data.len() < 4 {
            return Err("file too small".into());
        }

        let header_size = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut palette = [[0u8; 3]; 256];
        let mut pos = 2usize;
        pos = parse_palette_block(&data, pos, &mut palette);
        while pos < data.len() && data[pos] == 0xFF {
            pos += 1;
        }
        let mut offsets = Vec::new();
        while pos + 3 < header_size && pos + 3 < data.len() {
            offsets.push(u32::from_le_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]));
            pos += 4;
        }

        Ok(Self {
            data,
            header_size,
            palette,
            offsets,
        })
    }

    fn frame_count(&self) -> usize {
        if self.offsets.len() > 1 {
            self.offsets.len() - 1
        } else {
            self.offsets.len()
        }
    }

    /// Decode frame `idx` into the framebuffer. Returns (sub_width, sub_height, mode).
    /// Updates palette from any 'pl' chunks in this frame's superchunk.
    fn decode_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, false)
    }

    fn decode_character_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, true)
    }

    fn decode_frame_impl(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
        clear_zeroes: bool,
    ) -> (usize, usize, u8) {
        let abs_off = self.header_size + self.offsets[idx] as usize;
        if abs_off + 2 > self.data.len() {
            return (0, 0, 0);
        }

        let sc_size = u16::from_le_bytes([self.data[abs_off], self.data[abs_off + 1]]) as usize;
        let mut cpos = abs_off + 2;
        let sc_end = abs_off + sc_size;

        // Process typed chunks
        while cpos < sc_end && cpos + 4 <= self.data.len() {
            let t0 = self.data[cpos];
            let t1 = self.data[cpos + 1];
            let csz = u16::from_le_bytes([self.data[cpos + 2], self.data[cpos + 3]]) as usize;
            if t0 >= 0x20 && t0 < 0x7f && t1 >= 0x20 && t1 < 0x7f && csz >= 4 {
                if t0 == b'p' && t1 == b'l' {
                    parse_palette_block(&self.data, cpos + 4, pal);
                }
                cpos += csz;
            } else {
                break;
            }
        }

        if cpos + 4 > self.data.len() {
            return (0, 0, 0);
        }

        let vhdr = u32::from_le_bytes([
            self.data[cpos],
            self.data[cpos + 1],
            self.data[cpos + 2],
            self.data[cpos + 3],
        ]);
        let fw = (vhdr & 0x1FF) as usize;
        let fh = ((vhdr >> 16) & 0xFF) as usize;
        let mode = ((vhdr >> 24) & 0xFF) as u8;

        if fw == 0 || fh == 0 {
            return (0, 0, mode);
        }

        let fds = cpos + 4;
        if fds + 6 > self.data.len() {
            return (fw, fh, mode);
        }

        let checksum = self.data[fds..fds + 6]
            .iter()
            .map(|&b| b as u32)
            .sum::<u32>()
            & 0xFF;

        let pixels = if checksum == 0xAB {
            decompress_lz_171(&self.data, fds).ok()
        } else if checksum == 0xAD {
            decompress_rle_173(&self.data, fds).ok()
        } else {
            None
        };

        if let Some(pixels) = pixels {
            let cw = fw.min(VIEWPORT_W);
            let ch = fh.min(VIEWPORT_H);
            match mode {
                0xFF => {
                    for y in 0..ch {
                        for x in 0..cw {
                            let si = y * fw + x;
                            if si < pixels.len() && (clear_zeroes || pixels[si] != 0) {
                                fb[y * VIEWPORT_W + x] = pixels[si];
                            }
                        }
                    }
                }
                _ => {
                    for y in 0..ch {
                        let so = y * fw;
                        let d = y * VIEWPORT_W;
                        let rl = cw.min(pixels.len().saturating_sub(so));
                        if rl > 0 {
                            fb[d..d + rl].copy_from_slice(&pixels[so..so + rl]);
                        }
                    }
                }
            }
        }

        (fw, fh, mode)
    }
}

fn character_foreground_bounds(hnm: &HnmFile) -> (usize, usize) {
    let mut max_w = 0usize;
    let mut max_h = 0usize;
    let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
    let mut pal = hnm.palette;

    for idx in 0..hnm.frame_count() {
        let (w, h, _) = hnm.decode_frame(idx, &mut fb, &mut pal);
        if w == 0 || h == 0 {
            continue;
        }
        if w < VIEWPORT_W || h < VIEWPORT_H {
            max_w = max_w.max(w.min(VIEWPORT_W));
            max_h = max_h.max(h.min(VIEWPORT_H));
        }
    }

    if max_w == 0 || max_h == 0 {
        (VIEWPORT_W, VIEWPORT_H)
    } else {
        (max_w, max_h)
    }
}

fn clear_outside_character_bounds(fb: &mut [u8], clip_w: usize, clip_h: usize) {
    if clip_w >= VIEWPORT_W && clip_h >= VIEWPORT_H {
        return;
    }

    for y in 0..VIEWPORT_H {
        for x in 0..VIEWPORT_W {
            if x >= clip_w || y >= clip_h {
                fb[y * VIEWPORT_W + x] = 0;
            }
        }
    }
}

fn clear_character_bounds(fb: &mut [u8], clip_w: usize, clip_h: usize) {
    let clip_w = clip_w.min(VIEWPORT_W);
    let clip_h = clip_h.min(VIEWPORT_H);
    for y in 0..clip_h {
        let row = y * VIEWPORT_W;
        fb[row..row + clip_w].fill(0);
    }
}

// ===========================================================================
// Standalone HNM to MP4 decoder (for all HNM files)
// ===========================================================================

fn decode_hnm_to_mp4(
    hnm_path: &Path,
    mp4_path: &Path,
    music_path: Option<&Path>,
) -> Result<usize, Box<dyn Error>> {
    decode_hnm_scene_to_mp4(&[hnm_path.to_path_buf()], mp4_path, music_path, &[], None)
}

fn decode_hnm_scene_to_mp4(
    hnm_paths: &[PathBuf],
    mp4_path: &Path,
    music_path: Option<&Path>,
    subtitles: &[SubtitleCue],
    subtitle_sfx_path: Option<&Path>,
) -> Result<usize, Box<dyn Error>> {
    if hnm_paths.is_empty() {
        return Err("empty HNM scene".into());
    }

    let mut hnms = Vec::with_capacity(hnm_paths.len());
    let mut frame_count = 0usize;
    for path in hnm_paths {
        let hnm = HnmFile::open(path)?;
        let count = hnm.frame_count();
        if count == 0 {
            return Err(format!("{} has no frames", path.display()).into());
        }
        frame_count += count;
        hnms.push(hnm);
    }
    let duration = frame_count as f64 / HNM_FPS as f64;

    let tmp_sfx = mp4_path.with_extension("subtitle_sfx.raw");
    let subtitle_sfx_rate = if subtitles.is_empty() {
        None
    } else if let Some(path) = subtitle_sfx_path {
        build_subtitle_sfx_track(subtitles, duration, path, &tmp_sfx)?
    } else {
        None
    };

    let result = (|| -> Result<usize, Box<dyn Error>> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &format!("{VIEWPORT_W}x{VIEWPORT_H}"),
            "-framerate",
            &HNM_FPS.to_string(),
            "-i",
            "pipe:0",
        ]);

        let has_music = music_path.is_some();
        if let Some(mp) = music_path {
            cmd.args(["-stream_loop", "-1", "-i"]);
            cmd.arg(mp);
        }

        if let Some(sample_rate) = subtitle_sfx_rate {
            cmd.args([
                "-f",
                "u8",
                "-ar",
                &sample_rate.to_string(),
                "-ac",
                "1",
                "-i",
            ]);
            cmd.arg(&tmp_sfx);
        }

        match (has_music, subtitle_sfx_rate.is_some()) {
            (true, true) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=0.5[music];[2:a]volume=0.75[sfx];[music][sfx]amix=inputs=2:duration=shortest[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (true, false) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=0.5[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (false, true) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=0.75[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (false, false) => {
                cmd.args(["-map", "0:v"]);
            }
        }

        cmd.args([
            "-c:v", "libx264", "-crf", "18", "-preset", "medium", "-pix_fmt", "yuv420p", "-c:a",
            "aac", "-b:a", "128k",
        ]);
        if has_music || subtitle_sfx_rate.is_some() {
            cmd.arg("-shortest");
        }
        cmd.args(["-v", "warning"]);

        cmd.arg(mp4_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut ffmpeg = cmd.spawn()?;
        let mut stdin = ffmpeg.stdin.take().ok_or("no stdin")?;
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
        let mut global_frame = 0usize;

        for hnm in &hnms {
            let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
            let mut pal = hnm.palette;
            for frame_idx in 0..hnm.frame_count() {
                hnm.decode_frame(frame_idx, &mut fb, &mut pal);
                fb_to_rgb(&fb, &pal, &mut rgb);
                let time = global_frame as f64 / HNM_FPS as f64;
                render_subtitles(&mut rgb, subtitles, time);
                stdin.write_all(&rgb)?;
                global_frame += 1;
            }
        }

        drop(stdin);
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err(format!("ffmpeg: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        Ok(frame_count)
    })();

    let _ = fs::remove_file(&tmp_sfx);
    result
}

// ===========================================================================
// Character video — voice + animation + background + music
// ===========================================================================

struct CharacterContext {
    record_name: &'static str,
    background_hnm: Option<&'static str>,
}

/// DESCRIPT.DES gives the character foreground and voice bank. Some character
/// records are standalone talking heads/objects; others are composited over the
/// active room. Keep the room part isolated until SCRIPT*.COD can replace it.
const CHAR_CONTEXTS: &[CharacterContext] = &[
    CharacterContext {
        record_name: "receiver",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Rotator",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Maziok",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Outrageor",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Super_Tromp",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Betakam",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Bratakas",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Anna_Haf",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Hom",
        background_hnm: Some("satell10"),
    },
    CharacterContext {
        record_name: "Kran_Dobu",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Yoko",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Eviscerator",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Emasculator",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Cyberquizz",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Jerry_Khan",
        background_hnm: Some("petrol10"),
    },
    CharacterContext {
        record_name: "Morning_Oil",
        background_hnm: Some("concert"),
    },
    CharacterContext {
        record_name: "Super_Zen",
        background_hnm: Some("1masta20"),
    },
    CharacterContext {
        record_name: "Amigo",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Migrator",
        background_hnm: Some("tumull40"),
    },
    CharacterContext {
        record_name: "Tina_Burner",
        background_hnm: Some("1rondo20"),
    },
    CharacterContext {
        record_name: "Tromp_la_Mort",
        background_hnm: Some("larve"),
    },
    CharacterContext {
        record_name: "Scruter_Mac",
        background_hnm: Some("2vista20"),
    },
    CharacterContext {
        record_name: "Scruter_Jo",
        background_hnm: Some("2vista20"),
    },
    CharacterContext {
        record_name: "Scruter_K",
        background_hnm: Some("2vista20"),
    },
    CharacterContext {
        record_name: "Daddy_Gluxx",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Otto_Von_Smile",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Maxxon",
        background_hnm: Some("petrol10"),
    },
    CharacterContext {
        record_name: "Bronko",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Izwalito",
        background_hnm: Some("kort_1b"),
    },
    CharacterContext {
        record_name: "Fifi",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Beauregard",
        background_hnm: Some("gobar2"),
    },
    CharacterContext {
        record_name: "Bob_Morlock",
        background_hnm: Some("gobar1"),
    },
    CharacterContext {
        record_name: "Bug_Deluxe",
        background_hnm: None,
    },
    CharacterContext {
        record_name: "Sinox",
        background_hnm: Some("glacia10"),
    },
    CharacterContext {
        record_name: "ondoyant",
        background_hnm: Some("ondoya"),
    },
];

fn char_contents() -> &'static [CharacterContext] {
    CHAR_CONTEXTS
}

fn lookup_character_context(record_name: &str) -> Option<&'static CharacterContext> {
    char_contents()
        .iter()
        .find(|ctx| ctx.record_name.eq_ignore_ascii_case(record_name))
}

/// Create combined character videos for each DESCRIPT.DES character record that
/// uses this SND bank.
fn create_character_videos(
    snd_path: &Path,
    snd_stem: &str,
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
    script_speech: &[ScriptSpeechLine],
    subtitle_sfx_path: Option<&Path>,
) -> Result<u32, Box<dyn Error>> {
    let Some(db) = descript_db else {
        return Ok(0);
    };

    let scenes = db.character_scenes_for_snd(snd_stem);
    let mut created = 0u32;
    for scene in scenes {
        if script_speech.is_empty()
            && create_character_video_from_scene(snd_path, &scene, dat_dir, mp4_dir, hnm_music)?
        {
            created += 1;
        }
        created += create_character_dialogue_videos_from_scene(
            snd_path,
            &scene,
            dat_dir,
            mp4_dir,
            hnm_music,
            script_speech,
            subtitle_sfx_path,
        )?;
    }
    Ok(created)
}

/// Create a combined character video: all voice clips back-to-back with looping
/// foreground animations from DESCRIPT.DES, composited on the location background.
fn create_character_video_from_scene(
    snd_path: &Path,
    scene: &CharacterScene,
    dat_dir: &Path,
    mp4_dir: &Path,
    hnm_music: &HashMap<String, String>,
) -> Result<bool, Box<dyn Error>> {
    let Some(context) = lookup_character_context(&scene.record_name) else {
        return Ok(false);
    };

    // Parse SND file
    let snd_data = fs::read(snd_path)?;
    if snd_data.len() < 6 {
        return Ok(false);
    }
    let num_clips = u16::from_le_bytes([snd_data[0], snd_data[1]]) as usize;
    let header_end = 4 + (num_clips + 1) * 4;
    if header_end > snd_data.len() {
        return Ok(false);
    }
    let mut clip_offsets = Vec::with_capacity(num_clips + 1);
    for i in 0..=num_clips {
        let pos = 4 + i * 4;
        clip_offsets.push(u32::from_le_bytes([
            snd_data[pos],
            snd_data[pos + 1],
            snd_data[pos + 2],
            snd_data[pos + 3],
        ]) as usize);
    }

    // Collect valid clip+animation pairs
    struct ClipInfo {
        hnm_path: PathBuf,
        pcm_start: usize,
        pcm_len: usize,
        sample_rate: u32,
    }
    let mut clips: Vec<ClipInfo> = Vec::new();

    for i in 0..num_clips.min(scene.talk_hnms.len()) {
        let hnm_name = &scene.talk_hnms[i].1;
        let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
        if !hnm_path.exists() {
            continue;
        }

        let cs = header_end + clip_offsets[i];
        let ce = header_end + clip_offsets[i + 1];
        if cs + 6 > snd_data.len() || ce > snd_data.len() || ce <= cs {
            continue;
        }
        if snd_data[cs] != 1 {
            continue;
        }

        let sr_code = snd_data[cs + 4];
        let sample_rate = if sr_code < 255 {
            1_000_000 / (256 - sr_code as u32)
        } else {
            11111
        };
        let pcm_start = cs + 6;
        let pcm_len = ce - pcm_start;

        clips.push(ClipInfo {
            hnm_path,
            pcm_start,
            pcm_len,
            sample_rate,
        });
    }

    if clips.is_empty() {
        return Ok(false);
    }

    // Load background as indexed framebuffer + palette from pl/ location
    let (bg_fb, bg_pal) = if let Some(bg_name) = context.background_hnm {
        let bg_path = dat_dir
            .join("pl")
            .join(format!("{}.hnm", bg_name.to_ascii_lowercase()));
        if bg_path.exists() {
            if let Ok(bg_hnm) = HnmFile::open(&bg_path) {
                let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
                let mut pal = bg_hnm.palette;
                bg_hnm.decode_frame(0, &mut fb, &mut pal);
                (fb, pal)
            } else {
                (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
            }
        } else {
            (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
        }
    } else {
        (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
    };

    // Pre-render background to RGB (used for transparent pixels)
    let mut bg_rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
    fb_to_rgb(&bg_fb, &bg_pal, &mut bg_rgb);

    // Write concatenated voice PCM to temp file
    let output_stem = safe_file_stem(&scene.record_name);
    let tmp_voice = mp4_dir.join(format!("_tmp_{output_stem}_voice.raw"));
    {
        let mut vf = File::create(&tmp_voice)?;
        for clip in &clips {
            vf.write_all(&snd_data[clip.pcm_start..clip.pcm_start + clip.pcm_len])?;
        }
    }

    // Build ffmpeg command — full 320x200 output
    let music_path = context
        .background_hnm
        .and_then(|bg_name| hnm_music.get(&media_stem(bg_name)))
        .map(|music| dat_dir.join("mu").join(format!("{music}.voc")));
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let sr = clips[0].sample_rate;

    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgb24",
        "-video_size",
        &format!("{VIEWPORT_W}x{VIEWPORT_H}"),
        "-framerate",
        &HNM_FPS.to_string(),
        "-i",
        "pipe:0",
        "-f",
        "u8",
        "-ar",
        &sr.to_string(),
        "-ac",
        "1",
        "-i",
    ]);
    cmd.arg(&tmp_voice);

    if music_path.as_ref().is_some_and(|p| p.exists()) {
        cmd.args(["-stream_loop", "-1", "-i"]);
        cmd.arg(music_path.as_ref().unwrap());
        cmd.args([
            "-filter_complex",
            "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[voice][music]amix=inputs=2:duration=first[aout]",
            "-map", "0:v", "-map", "[aout]",
        ]);
    } else {
        cmd.args(["-map", "0:v", "-map", "1:a"]);
    }

    cmd.args([
        "-c:v",
        "libx264",
        "-crf",
        "18",
        "-preset",
        "medium",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-shortest",
        "-v",
        "warning",
    ]);
    cmd.arg(&mp4_out);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut ffmpeg = cmd.spawn()?;
    let mut stdin = ffmpeg.stdin.take().ok_or("no stdin")?;

    let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];

    for clip in &clips {
        let audio_dur = clip.pcm_len as f64 / clip.sample_rate as f64;
        let total_frames = (audio_dur * HNM_FPS as f64).ceil() as usize;

        let hnm = HnmFile::open(&clip.hnm_path)?;
        let anim_frames = hnm.frame_count();
        if anim_frames == 0 {
            continue;
        }

        let mut char_fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        let mut pal = hnm.palette;
        hnm.decode_character_frame(0, &mut char_fb, &mut pal);
        let (clip_w, clip_h) = character_foreground_bounds(&hnm);
        clear_outside_character_bounds(&mut char_fb, clip_w, clip_h);
        let base_fb = char_fb.clone();
        let base_pal = pal;

        for out_f in 0..total_frames {
            let anim_idx = out_f % anim_frames;
            if anim_idx == 0 {
                char_fb.copy_from_slice(&base_fb);
                pal = base_pal;
            } else {
                clear_character_bounds(&mut char_fb, clip_w, clip_h);
                hnm.decode_character_frame(anim_idx, &mut char_fb, &mut pal);
                clear_outside_character_bounds(&mut char_fb, clip_w, clip_h);
            }

            // Composite: character on top of background
            // Character pixel index 0 = transparent -> show background
            for i in 0..(VIEWPORT_W * VIEWPORT_H) {
                let ci = i * 3;
                if char_fb[i] != 0 {
                    let c = pal[char_fb[i] as usize];
                    rgb[ci] = c[0];
                    rgb[ci + 1] = c[1];
                    rgb[ci + 2] = c[2];
                } else {
                    rgb[ci] = bg_rgb[ci];
                    rgb[ci + 1] = bg_rgb[ci + 1];
                    rgb[ci + 2] = bg_rgb[ci + 2];
                }
            }

            if stdin.write_all(&rgb).is_err() {
                break;
            }
        }
    }

    drop(stdin);
    let result = ffmpeg.wait_with_output()?;
    let _ = fs::remove_file(&tmp_voice);

    if !result.status.success() {
        let _ = fs::remove_file(&mp4_out);
        return Err(format!("ffmpeg: {}", String::from_utf8_lossy(&result.stderr)).into());
    }

    Ok(true)
}

fn create_character_dialogue_videos_from_scene(
    snd_path: &Path,
    scene: &CharacterScene,
    dat_dir: &Path,
    mp4_dir: &Path,
    hnm_music: &HashMap<String, String>,
    script_speech: &[ScriptSpeechLine],
    subtitle_sfx_path: Option<&Path>,
) -> Result<u32, Box<dyn Error>> {
    let mut groups: BTreeMap<(String, String), Vec<&ScriptSpeechLine>> = BTreeMap::new();
    for line in script_speech {
        if !line
            .actor_record
            .as_deref()
            .is_some_and(|actor| actor.eq_ignore_ascii_case(&scene.record_name))
        {
            continue;
        }
        let Some(clip_index) = line.clip_index else {
            continue;
        };
        if clip_index >= scene.talk_hnms.len() {
            continue;
        }
        groups
            .entry((line.script.clone(), line.function_name.clone()))
            .or_default()
            .push(line);
    }

    let mut created = 0u32;
    for ((script, function_name), mut lines) in groups {
        lines.sort_by_key(|line| line.offset);
        if create_character_dialogue_video(
            snd_path,
            scene,
            dat_dir,
            mp4_dir,
            hnm_music,
            subtitle_sfx_path,
            &script,
            &function_name,
            &lines,
        )? {
            created += 1;
        }
    }

    Ok(created)
}

fn create_character_dialogue_video(
    snd_path: &Path,
    scene: &CharacterScene,
    dat_dir: &Path,
    mp4_dir: &Path,
    hnm_music: &HashMap<String, String>,
    subtitle_sfx_path: Option<&Path>,
    script: &str,
    function_name: &str,
    lines: &[&ScriptSpeechLine],
) -> Result<bool, Box<dyn Error>> {
    let Some(context) = lookup_character_context(&scene.record_name) else {
        return Ok(false);
    };
    if lines.is_empty() {
        return Ok(false);
    }

    let snd_data = fs::read(snd_path)?;
    if snd_data.len() < 6 {
        return Ok(false);
    }
    let num_clips = u16::from_le_bytes([snd_data[0], snd_data[1]]) as usize;
    let header_end = 4 + (num_clips + 1) * 4;
    if header_end > snd_data.len() {
        return Ok(false);
    }
    let mut clip_offsets = Vec::with_capacity(num_clips + 1);
    for i in 0..=num_clips {
        let pos = 4 + i * 4;
        clip_offsets.push(u32::from_le_bytes([
            snd_data[pos],
            snd_data[pos + 1],
            snd_data[pos + 2],
            snd_data[pos + 3],
        ]) as usize);
    }

    struct DialogueClip {
        hnm_path: PathBuf,
        pcm_start: usize,
        pcm_len: usize,
        sample_rate: u32,
        text: String,
    }

    let mut clips = Vec::new();
    for line in lines {
        let Some(clip_index) = line.clip_index else {
            continue;
        };
        if clip_index >= num_clips || clip_index >= scene.talk_hnms.len() {
            continue;
        }

        let hnm_name = &scene.talk_hnms[clip_index].1;
        let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
        if !hnm_path.exists() {
            continue;
        }

        let cs = header_end + clip_offsets[clip_index];
        let ce = header_end + clip_offsets[clip_index + 1];
        if cs + 6 > snd_data.len() || ce > snd_data.len() || ce <= cs {
            continue;
        }
        if snd_data[cs] != 1 {
            continue;
        }

        let sr_code = snd_data[cs + 4];
        let sample_rate = if sr_code < 255 {
            1_000_000 / (256 - sr_code as u32)
        } else {
            11111
        };
        clips.push(DialogueClip {
            hnm_path,
            pcm_start: cs + 6,
            pcm_len: ce - (cs + 6),
            sample_rate,
            text: line.text.clone(),
        });
    }

    if clips.is_empty() {
        return Ok(false);
    }

    let sr = clips[0].sample_rate;
    if clips.iter().any(|clip| clip.sample_rate != sr) {
        return Err(format!("{} {} uses mixed SND sample rates", script, function_name).into());
    }

    let background_hnm = lines
        .iter()
        .find_map(|line| line.background_hnm.as_deref())
        .or(context.background_hnm);
    let music_name = lines
        .iter()
        .find_map(|line| line.background_music.as_deref().map(str::to_string))
        .or_else(|| background_hnm.and_then(|hnm| hnm_music.get(&media_stem(hnm)).cloned()));

    let (bg_fb, bg_pal) = if let Some(bg_name) = background_hnm {
        let bg_path = character_background_path(dat_dir, bg_name);
        if bg_path.exists() {
            if let Ok(bg_hnm) = HnmFile::open(&bg_path) {
                let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
                let mut pal = bg_hnm.palette;
                bg_hnm.decode_frame(0, &mut fb, &mut pal);
                (fb, pal)
            } else {
                (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
            }
        } else {
            (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
        }
    } else {
        (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256])
    };
    let mut bg_rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
    fb_to_rgb(&bg_fb, &bg_pal, &mut bg_rgb);

    let output_stem = format!(
        "dialogue - {} - {} - {}",
        safe_file_stem(script),
        safe_file_stem(function_name),
        safe_file_stem(&scene.record_name)
    );
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let tmp_voice = mp4_dir.join(format!("_tmp_{}_voice.raw", safe_file_stem(&output_stem)));
    {
        let mut vf = File::create(&tmp_voice)?;
        for clip in &clips {
            vf.write_all(&snd_data[clip.pcm_start..clip.pcm_start + clip.pcm_len])?;
        }
    }

    let mut cues = Vec::new();
    let mut duration = 0.0f64;
    for clip in &clips {
        cues.push(SubtitleCue {
            tick: (duration * 10.0).round() as u16,
            text: clip.text.clone(),
        });
        duration += clip.pcm_len as f64 / clip.sample_rate as f64;
    }

    let tmp_sfx = mp4_out.with_extension("subtitle_sfx.raw");
    let subtitle_sfx_rate = if let Some(path) = subtitle_sfx_path {
        build_subtitle_sfx_track(&cues, duration, path, &tmp_sfx)?
    } else {
        None
    };

    let music_path = music_name.map(|music| dat_dir.join("mu").join(format!("{music}.voc")));
    let has_music = music_path.as_ref().is_some_and(|path| path.exists());

    let result = (|| -> Result<bool, Box<dyn Error>> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &format!("{VIEWPORT_W}x{VIEWPORT_H}"),
            "-framerate",
            &HNM_FPS.to_string(),
            "-i",
            "pipe:0",
            "-f",
            "u8",
            "-ar",
            &sr.to_string(),
            "-ac",
            "1",
            "-i",
        ]);
        cmd.arg(&tmp_voice);

        if has_music {
            cmd.args(["-stream_loop", "-1", "-i"]);
            cmd.arg(music_path.as_ref().unwrap());
        }
        if let Some(sample_rate) = subtitle_sfx_rate {
            cmd.args([
                "-f",
                "u8",
                "-ar",
                &sample_rate.to_string(),
                "-ac",
                "1",
                "-i",
            ]);
            cmd.arg(&tmp_sfx);
        }

        match (has_music, subtitle_sfx_rate.is_some()) {
            (true, true) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[3:a]volume=0.75[sfx];[voice][music][sfx]amix=inputs=3:duration=first[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (true, false) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[voice][music]amix=inputs=2:duration=first[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (false, true) => {
                cmd.args([
                    "-filter_complex",
                    "[1:a]volume=1.0[voice];[2:a]volume=0.75[sfx];[voice][sfx]amix=inputs=2:duration=first[aout]",
                    "-map",
                    "0:v",
                    "-map",
                    "[aout]",
                ]);
            }
            (false, false) => {
                cmd.args(["-map", "0:v", "-map", "1:a"]);
            }
        }

        cmd.args([
            "-c:v",
            "libx264",
            "-crf",
            "18",
            "-preset",
            "medium",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-shortest",
            "-v",
            "warning",
        ]);
        cmd.arg(&mp4_out);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut ffmpeg = cmd.spawn()?;
        let mut stdin = ffmpeg.stdin.take().ok_or("no stdin")?;
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
        let mut global_frame = 0usize;

        for clip in &clips {
            let audio_dur = clip.pcm_len as f64 / clip.sample_rate as f64;
            let total_frames = (audio_dur * HNM_FPS as f64).ceil() as usize;
            let hnm = HnmFile::open(&clip.hnm_path)?;
            let anim_frames = hnm.frame_count();
            if anim_frames == 0 {
                continue;
            }

            let mut char_fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
            let mut pal = hnm.palette;
            hnm.decode_character_frame(0, &mut char_fb, &mut pal);
            let (clip_w, clip_h) = character_foreground_bounds(&hnm);
            clear_outside_character_bounds(&mut char_fb, clip_w, clip_h);
            let base_fb = char_fb.clone();
            let base_pal = pal;

            for out_f in 0..total_frames {
                let anim_idx = out_f % anim_frames;
                if anim_idx == 0 {
                    char_fb.copy_from_slice(&base_fb);
                    pal = base_pal;
                } else {
                    clear_character_bounds(&mut char_fb, clip_w, clip_h);
                    hnm.decode_character_frame(anim_idx, &mut char_fb, &mut pal);
                    clear_outside_character_bounds(&mut char_fb, clip_w, clip_h);
                }

                for i in 0..(VIEWPORT_W * VIEWPORT_H) {
                    let ci = i * 3;
                    if char_fb[i] != 0 {
                        let c = pal[char_fb[i] as usize];
                        rgb[ci] = c[0];
                        rgb[ci + 1] = c[1];
                        rgb[ci + 2] = c[2];
                    } else {
                        rgb[ci] = bg_rgb[ci];
                        rgb[ci + 1] = bg_rgb[ci + 1];
                        rgb[ci + 2] = bg_rgb[ci + 2];
                    }
                }

                let time = global_frame as f64 / HNM_FPS as f64;
                render_subtitles(&mut rgb, &cues, time);
                stdin.write_all(&rgb)?;
                global_frame += 1;
            }
        }

        drop(stdin);
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err(format!("ffmpeg: {}", String::from_utf8_lossy(&output.stderr)).into());
        }
        Ok(true)
    })();

    let _ = fs::remove_file(&tmp_voice);
    let _ = fs::remove_file(&tmp_sfx);
    if result.is_err() {
        let _ = fs::remove_file(&mp4_out);
    }
    result
}

fn character_background_path(dat_dir: &Path, hnm_name: &str) -> PathBuf {
    let lower = hnm_name.to_ascii_lowercase();
    if lower.ends_with(".hnm") || lower.contains('/') || lower.contains('\\') {
        dat_dir.join(descript_hnm_path(&lower, 1))
    } else {
        dat_dir.join("pl").join(format!("{lower}.hnm"))
    }
}

// ===========================================================================
// SND voice bank decoder
// ===========================================================================

fn decode_snd_clips(
    snd_path: &Path,
    base_name: &str,
    flac_dir: &Path,
    m4a_dir: &Path,
) -> Result<u32, Box<dyn Error>> {
    let data = fs::read(snd_path)?;
    if data.len() < 6 {
        return Err("file too small".into());
    }

    let num_clips = u16::from_le_bytes([data[0], data[1]]) as usize;
    let header_end = 4 + (num_clips + 1) * 4;
    if header_end > data.len() {
        return Err("header exceeds file size".into());
    }

    let mut offsets = Vec::with_capacity(num_clips + 1);
    for i in 0..=num_clips {
        let pos = 4 + i * 4;
        offsets.push(
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize,
        );
    }

    let mut converted = 0u32;
    for i in 0..num_clips {
        let clip_start = header_end + offsets[i];
        let clip_end = header_end + offsets[i + 1];
        if clip_start + 6 > data.len() || clip_end > data.len() || clip_end <= clip_start {
            continue;
        }
        if data[clip_start] != 1 {
            continue;
        }

        let sr_code = data[clip_start + 4];
        let sample_rate = if sr_code < 255 {
            1_000_000 / (256 - sr_code as u32)
        } else {
            11111
        };

        let pcm_data = &data[clip_start + 6..clip_end];
        if pcm_data.is_empty() {
            continue;
        }

        let clip_name = format!("{base_name} - {i:03}");

        let flac_out = flac_dir.join(format!("{clip_name}.flac"));
        let flac_ok = run_raw_pcm_to_ffmpeg(pcm_data, sample_rate, &flac_out, &[]);

        let m4a_out = m4a_dir.join(format!("{clip_name}.m4a"));
        let m4a_ok = run_raw_pcm_to_ffmpeg(
            pcm_data,
            sample_rate,
            &m4a_out,
            &["-c:a", "aac", "-b:a", "128k"],
        );

        if flac_ok || m4a_ok {
            converted += 1;
        }
    }

    Ok(converted)
}

fn run_raw_pcm_to_ffmpeg(pcm: &[u8], sample_rate: u32, output: &Path, extra: &[&str]) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-y",
        "-f",
        "u8",
        "-ar",
        &sample_rate.to_string(),
        "-ac",
        "1",
        "-i",
        "pipe:0",
    ]);
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.args(["-v", "warning"]).arg(output);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let Ok(mut child) = cmd.spawn() else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(pcm);
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

// ===========================================================================
// Palette and framebuffer helpers
// ===========================================================================

fn parse_palette_block(data: &[u8], mut pos: usize, palette: &mut [[u8; 3]; 256]) -> usize {
    while pos + 1 < data.len() {
        let start = data[pos];
        let count = data[pos + 1];
        pos += 2;

        if start == 0xFF && count == 0xFF {
            break;
        }

        let n = if count == 0 { 256 } else { count as usize };
        for i in 0..n {
            if pos + 2 >= data.len() {
                return pos;
            }
            let idx = start as usize + i;
            if idx < 256 {
                palette[idx] = [
                    (data[pos] << 2) | (data[pos] >> 4),
                    (data[pos + 1] << 2) | (data[pos + 1] >> 4),
                    (data[pos + 2] << 2) | (data[pos + 2] >> 4),
                ];
            }
            pos += 3;
        }
    }
    pos
}

fn fb_to_rgb(fb: &[u8], palette: &[[u8; 3]; 256], rgb: &mut [u8]) {
    for (i, &px) in fb.iter().enumerate() {
        let c = palette[px as usize];
        rgb[i * 3] = c[0];
        rgb[i * 3 + 1] = c[1];
        rgb[i * 3 + 2] = c[2];
    }
}

fn render_subtitles(rgb: &mut [u8], cues: &[SubtitleCue], time: f64) {
    let Some((_, cue)) = cues.iter().enumerate().find(|(idx, cue)| {
        let start = cue.tick as f64 / 10.0;
        let end = cue_end_time(cues, *idx);
        time >= start && time < end
    }) else {
        return;
    };

    let full_text = cue.text.trim();
    if full_text.is_empty() {
        return;
    }

    let start = cue.tick as f64 / 10.0;
    let visible_chars = ((time - start).max(0.0) * SUBTITLE_CHARS_PER_SEC).ceil() as usize;
    if visible_chars == 0 {
        return;
    }

    let lines = wrap_subtitle_text_pixels(
        full_text,
        VIEWPORT_W.saturating_sub(SUBTITLE_X + SUBTITLE_RIGHT_MARGIN),
    );
    let visible_lines = visible_subtitle_lines(&lines, visible_chars);

    for (line_idx, line) in visible_lines.iter().enumerate() {
        let y = SUBTITLE_Y + line_idx * GAME_FONT_LINE_HEIGHT;
        draw_game_text(rgb, line, SUBTITLE_X, y, [245, 245, 245]);
    }
}

fn cue_end_time(cues: &[SubtitleCue], idx: usize) -> f64 {
    let start = cues[idx].tick as f64 / 10.0;
    cues.get(idx + 1)
        .map(|next| next.tick as f64 / 10.0)
        .filter(|end| *end > start + 0.25)
        .unwrap_or(start + 4.0)
}

fn visible_subtitle_lines(lines: &[String], visible_chars: usize) -> Vec<String> {
    let mut remaining = visible_chars;
    let mut out = Vec::new();
    for line in lines {
        if remaining == 0 {
            break;
        }
        let line_len = line.chars().count();
        let take = remaining.min(line_len);
        out.push(line.chars().take(take).collect());
        remaining = remaining.saturating_sub(line_len);
    }
    out
}

fn wrap_subtitle_text_pixels(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in text.replace('\r', "\n").lines() {
        let mut current = String::new();
        for word in raw_line.split_whitespace() {
            let word_width = game_text_width(word);
            let cur_width = game_text_width(&current);
            let sep_width = if current.is_empty() {
                0
            } else {
                game_font_advance(' ')
            };
            if cur_width > 0 && cur_width + sep_width + word_width > max_width {
                lines.push(current);
                current = String::new();
            }

            if word_width > max_width {
                let mut part = String::new();
                for ch in word.chars() {
                    if !part.is_empty()
                        && game_text_width(&part) + game_font_advance(ch) > max_width
                    {
                        lines.push(part);
                        part = String::new();
                    }
                    part.push(ch);
                }
                current = part;
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn game_text_width(text: &str) -> usize {
    text.chars().map(game_font_advance).sum()
}

fn draw_game_text(rgb: &mut [u8], text: &str, x: usize, y: usize, color: [u8; 3]) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some(glyph) = game_font_glyph(ch).or_else(|| game_font_glyph('?')) {
            draw_game_glyph(rgb, glyph.rows, cx, y, color);
        }
        cx += game_font_advance(ch);
        if cx >= VIEWPORT_W {
            break;
        }
    }
}

fn draw_game_glyph(
    rgb: &mut [u8],
    rows: [u8; GAME_FONT_HEIGHT],
    x: usize,
    y: usize,
    color: [u8; 3],
) {
    for (gy, row) in rows.iter().copied().enumerate() {
        for gx in 0..GAME_FONT_WIDTH {
            if (row & (0x80 >> gx)) == 0 {
                continue;
            }
            let px = x + gx;
            let py = y + gy;
            if px < VIEWPORT_W && py < VIEWPORT_H {
                let idx = (py * VIEWPORT_W + px) * 3;
                rgb[idx] = color[0];
                rgb[idx + 1] = color[1];
                rgb[idx + 2] = color[2];
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GameFontGlyph {
    rows: [u8; GAME_FONT_HEIGHT],
    advance: usize,
}

fn game_font_glyph(ch: char) -> Option<GameFontGlyph> {
    let code = ch as usize;
    let idx = *GAME_FONT_CHAR_MAP.get(code)?;
    if idx == 0xff {
        return None;
    }
    let idx = idx as usize;
    Some(GameFontGlyph {
        rows: GAME_FONT_GLYPHS[idx],
        advance: GAME_FONT_WIDTHS[idx] as usize,
    })
}

fn game_font_advance(ch: char) -> usize {
    if ch == ' ' {
        return GAME_FONT_SPACE_ADVANCE;
    }
    game_font_glyph(ch)
        .or_else(|| game_font_glyph('?'))
        .map(|glyph| glyph.advance)
        .unwrap_or(GAME_FONT_SPACE_ADVANCE)
}

const SUBTITLE_X: usize = 9;
const SUBTITLE_Y: usize = 7;
const SUBTITLE_RIGHT_MARGIN: usize = 8;
const GAME_FONT_WIDTH: usize = 8;
const GAME_FONT_HEIGHT: usize = 8;
const GAME_FONT_LINE_HEIGHT: usize = 8;
const GAME_FONT_SPACE_ADVANCE: usize = 8;

// Extracted from BLOODPRG.EXE:
// - ASCII to glyph index map: file offset 0x14c22
// - glyph advances: file offsets 0x14cd2..0x14d27
// - 8-byte glyph rows: file offset 0x14d28
#[rustfmt::skip]
const GAME_FONT_CHAR_MAP: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x1c, 0x24, 0xff, 0xff, 0xff, 0xff, 0x26, 0xff, 0xff, 0xff, 0x23, 0x25, 0x22, 0x1e, 0xff,
    0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x1f, 0x20, 0xff, 0xff, 0xff, 0x1a,
    0xff, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0xff, 0xff, 0xff, 0xff, 0x21,
    0xff, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
    0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[rustfmt::skip]
const GAME_FONT_WIDTHS: [u8; 86] = [
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x03, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x05, 0x07, 0x07, 0x07, 0x03, 0x03, 0x08, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x08, 0x03,
    0x06, 0x08, 0x03, 0x09, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x06, 0x08, 0x09, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x05, 0x05, 0x08, 0x08, 0x08, 0x08, 0x09, 0x04, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
];

#[rustfmt::skip]
const GAME_FONT_GLYPHS: [[u8; GAME_FONT_HEIGHT]; 86] = [
    [0x00, 0x7e, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x00],
    [0x00, 0xfc, 0x84, 0xfe, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x86, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x86, 0x82, 0xfe, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x02, 0x02, 0x02, 0x02, 0x82, 0xfe, 0x00],
    [0x00, 0x84, 0x84, 0x84, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00],
    [0x00, 0xc2, 0xa2, 0x92, 0x8a, 0x86, 0x82, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfe, 0x80, 0x80, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0xfe, 0x02, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x02, 0x02, 0xfe, 0x00],
    [0x00, 0xfe, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x44, 0x28, 0x10, 0x00],
    [0x00, 0x81, 0x81, 0x81, 0x81, 0x99, 0x66, 0x00],
    [0x00, 0x82, 0x44, 0x38, 0x44, 0x82, 0x82, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x7e, 0x04, 0x78, 0x00],
    [0x00, 0xfe, 0x02, 0x7c, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x1e, 0x10, 0x00, 0x10, 0x00],
    [0x00, 0x10, 0x00, 0x10, 0xf0, 0x82, 0xfe, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x00, 0x80, 0x00],
    [0x00, 0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00],
    [0x00, 0x00, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x80, 0x80],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x20, 0x20, 0xf8, 0x20, 0x20, 0x00],
    [0x00, 0xa0, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x80],
    [0x00, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x00],
    [0x00, 0x04, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x00, 0xf0, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x04, 0x7c],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x10, 0x00, 0x10, 0x10, 0x10, 0x10, 0x90, 0xf0],
    [0x00, 0x80, 0x88, 0x88, 0xf8, 0x84, 0x84, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xec, 0x92, 0x92, 0x92, 0x92, 0x00],
    [0x00, 0x00, 0xf8, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x80, 0x80],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x04],
    [0x00, 0x00, 0xf0, 0x90, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x80, 0xfc, 0x04, 0xfc, 0x00],
    [0x00, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x48, 0x30, 0x00],
    [0x00, 0x00, 0x82, 0x82, 0x82, 0x92, 0x6c, 0x00],
    [0x00, 0x00, 0x84, 0x48, 0x30, 0x48, 0x84, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0xfc, 0x10, 0x10],
    [0x00, 0x00, 0xfc, 0x04, 0x78, 0x80, 0xfc, 0x00],
    [0x48, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0xa0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0xe0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x20],
    [0x00, 0x7c, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x40, 0xc0, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x00, 0x7c, 0x82, 0x02, 0x7c, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x02, 0x3c, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x88, 0xfe, 0x08, 0x00],
    [0x00, 0xfe, 0x80, 0xfc, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x7c, 0x80, 0xfc, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0xfe, 0x82, 0x04, 0x04, 0x04, 0x04, 0x00],
    [0x00, 0x7c, 0x82, 0x7c, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x7c, 0x82, 0x82, 0x7e, 0x02, 0x7c, 0x00],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovered_game_font_matches_executable_rows() {
        let m = game_font_glyph('M').expect("M glyph");
        assert_eq!(m.advance, 10);
        assert_eq!(m.rows, [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00]);

        let e = game_font_glyph('e').expect("e glyph");
        assert_eq!(e.advance, 8);
        assert_eq!(e.rows, [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00]);
    }

    #[test]
    fn wraps_reference_subtitle_by_game_font_pixels() {
        let lines = wrap_subtitle_text_pixels(
            "Me see keyboard... And oh! CDROM double speed. Nice hard dish... you be lucky, friend...",
            VIEWPORT_W - SUBTITLE_X - SUBTITLE_RIGHT_MARGIN,
        );

        assert_eq!(
            lines,
            [
                "Me see keyboard... And oh! CDROM double",
                "speed. Nice hard dish... you be lucky,",
                "friend..."
            ]
        );
    }
}

fn build_subtitle_sfx_track(
    cues: &[SubtitleCue],
    duration: f64,
    snd_path: &Path,
    out_path: &Path,
) -> Result<Option<u32>, Box<dyn Error>> {
    let Some((pcm, sample_rate)) = read_snd_clip(snd_path, 0)? else {
        return Ok(None);
    };
    if pcm.is_empty() {
        return Ok(None);
    }

    let samples = ((duration + 0.5) * sample_rate as f64).ceil() as usize;
    let mut track = vec![128u8; samples.max(1)];
    let mut used = false;
    for cue in cues {
        let text = cue.text.trim();
        if text.is_empty() {
            continue;
        }

        let cue_start = cue.tick as f64 / 10.0;
        let mut visible_idx = 0usize;
        for ch in text.chars() {
            if ch != '\n' && ch != '\r' {
                if !ch.is_whitespace() {
                    let start_time = cue_start + visible_idx as f64 / SUBTITLE_CHARS_PER_SEC;
                    let start = (start_time * sample_rate as f64).round() as usize;
                    if start < track.len() {
                        used = true;
                        for (idx, &sample) in pcm.iter().enumerate() {
                            let pos = start + idx;
                            if pos >= track.len() {
                                break;
                            }
                            let mixed = track[pos] as i16 + sample as i16 - 128;
                            track[pos] = mixed.clamp(0, 255) as u8;
                        }
                    }
                }
                visible_idx += 1;
            }
        }
    }

    if !used {
        return Ok(None);
    }
    fs::write(out_path, track)?;
    Ok(Some(sample_rate))
}

fn read_snd_clip(
    snd_path: &Path,
    clip_idx: usize,
) -> Result<Option<(Vec<u8>, u32)>, Box<dyn Error>> {
    let data = fs::read(snd_path)?;
    if data.len() < 6 {
        return Ok(None);
    }
    let num_clips = u16::from_le_bytes([data[0], data[1]]) as usize;
    if clip_idx >= num_clips {
        return Ok(None);
    }
    let header_end = 4 + (num_clips + 1) * 4;
    if header_end > data.len() {
        return Ok(None);
    }

    let off_pos = 4 + clip_idx * 4;
    let next_off_pos = off_pos + 4;
    let clip_start =
        header_end + u32::from_le_bytes(data[off_pos..off_pos + 4].try_into()?) as usize;
    let clip_end =
        header_end + u32::from_le_bytes(data[next_off_pos..next_off_pos + 4].try_into()?) as usize;
    if clip_start + 6 > data.len() || clip_end > data.len() || clip_end <= clip_start {
        return Ok(None);
    }
    if data[clip_start] != 1 {
        return Ok(None);
    }

    let sr_code = data[clip_start + 4];
    let sample_rate = if sr_code < 255 {
        1_000_000 / (256 - sr_code as u32)
    } else {
        11111
    };
    Ok(Some((data[clip_start + 6..clip_end].to_vec(), sample_rate)))
}

// ===========================================================================
// HNM(1) decompression — Block 171 (LZ) and Block 173 (RLE)
// ===========================================================================

fn decompress_lz_171(data: &[u8], offset: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let unpacked_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let mut pos = offset + 6;
    let mut out = Vec::with_capacity(unpacked_len);
    let mut bits_left = 0u32;
    let mut queue = 0u16;

    let get_bit = |pos: &mut usize, bits_left: &mut u32, queue: &mut u16| -> u8 {
        if *bits_left == 0 {
            *queue = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
            *pos += 2;
            *bits_left = 16;
        }
        let b = (*queue & 1) as u8;
        *queue >>= 1;
        *bits_left -= 1;
        b
    };

    while out.len() < unpacked_len {
        if get_bit(&mut pos, &mut bits_left, &mut queue) != 0 {
            out.push(data[pos]);
            pos += 1;
        } else {
            let (count, offset_val);
            if get_bit(&mut pos, &mut bits_left, &mut queue) != 0 {
                let val = u16::from_le_bytes([data[pos], data[pos + 1]]);
                pos += 2;
                let c = (val & 0x07) as usize;
                offset_val = ((val >> 3) as isize) - 8192;
                if c == 0 {
                    let c2 = data[pos] as usize;
                    pos += 1;
                    if c2 == 0 {
                        break;
                    }
                    count = c2;
                } else {
                    count = c;
                }
            } else {
                let b0 = get_bit(&mut pos, &mut bits_left, &mut queue);
                let b1 = get_bit(&mut pos, &mut bits_left, &mut queue);
                count = (b0 as usize) * 2 + (b1 as usize);
                offset_val = (data[pos] as isize) - 256;
                pos += 1;
            }

            let total = count + 2;
            let src = (out.len() as isize + offset_val) as usize;
            for i in 0..total {
                let b = out[src + i];
                out.push(b);
            }
        }
    }

    out.truncate(unpacked_len);
    Ok(out)
}

struct BitReaderHigh<'a> {
    data: &'a [u8],
    pos: usize,
    bits_left: u32,
    queue: u16,
}

impl<'a> BitReaderHigh<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            bits_left: 0,
            queue: 0,
        }
    }

    fn get_bit(&mut self) -> u8 {
        if self.bits_left == 0 {
            self.queue = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            self.bits_left = 16;
        }
        self.bits_left -= 1;
        ((self.queue >> self.bits_left) & 1) as u8
    }

    fn get_byte(&mut self) -> u8 {
        let b = self.data[self.pos];
        self.pos += 1;
        b
    }
}

fn decompress_rle_173(data: &[u8], offset: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let frame_size = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let codebook_size = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
    let flags = data[offset + 4];

    let color_base: u8 = if (flags & 0x40) != 0 { 128 } else { 0 };
    let long_runs = (flags & 0x80) != 0;

    let mut pos = offset + 6;
    if (flags & 0x04) == 0 {
        pos += 4;
    }

    // Decompress codebook
    let mut codebook = Vec::with_capacity(codebook_size);
    let mut lc_byte: Option<u8> = None;
    let mut lc_used_top = true;

    while codebook.len() < codebook_size && pos < data.len() {
        let tag = data[pos];
        pos += 1;

        if tag & 0x80 != 0 {
            let temp;
            if lc_byte.is_none() || lc_used_top {
                let b = data[pos];
                pos += 1;
                lc_byte = Some(b);
                temp = (b >> 4) & 0x0F;
                lc_used_top = false;
            } else {
                temp = lc_byte.unwrap() & 0x0F;
                lc_used_top = true;
            }

            let offset_val = (((tag & 0x7F) as usize) << 1) | ((temp & 1) as usize);
            let count = (((temp >> 1) & 0x07) as usize) + 2;
            let src = codebook.len().wrapping_sub(offset_val + 1);
            for i in 0..count {
                let idx = src.wrapping_add(i);
                codebook.push(if idx < codebook.len() {
                    codebook[idx]
                } else {
                    0
                });
            }
        } else {
            codebook.push(if tag != 0 {
                tag.wrapping_add(color_base)
            } else {
                0
            });
        }
    }
    codebook.truncate(codebook_size);

    // Decode RLE raster
    let mut br = BitReaderHigh::new(data, pos);
    let mut cb_pos = 0usize;
    let mut frame = Vec::with_capacity(frame_size);

    let mut rle_lc_byte: Option<u8> = None;
    let mut rle_lc_used_top = true;

    let get_rle_length =
        |br: &mut BitReaderHigh, lc: &mut Option<u8>, used_top: &mut bool| -> usize {
            if lc.is_none() || *used_top {
                let b = br.get_byte();
                *lc = Some(b);
                let length = ((b >> 4) & 0x0F) as usize;
                *used_top = false;
                if length == 0 {
                    return br.get_byte() as usize + 16;
                }
                length
            } else {
                let length = (lc.unwrap() & 0x0F) as usize;
                *used_top = true;
                if length == 0 {
                    return br.get_byte() as usize + 16;
                }
                length
            }
        };

    while frame.len() < frame_size {
        while br.get_bit() == 0 {
            if cb_pos < codebook.len() {
                frame.push(codebook[cb_pos]);
                cb_pos += 1;
            }
            if frame.len() >= frame_size {
                break;
            }
        }
        if frame.len() >= frame_size {
            break;
        }

        let pixel = if cb_pos < codebook.len() {
            let p = codebook[cb_pos];
            cb_pos += 1;
            p
        } else {
            0
        };

        let run = if long_runs {
            if br.get_bit() == 0 {
                get_rle_length(&mut br, &mut rle_lc_byte, &mut rle_lc_used_top) + 4
            } else if br.get_bit() == 0 {
                2
            } else if br.get_bit() == 0 {
                3
            } else {
                4
            }
        } else if br.get_bit() == 0 {
            2
        } else if br.get_bit() == 0 {
            3
        } else if br.get_bit() == 0 {
            4
        } else {
            get_rle_length(&mut br, &mut rle_lc_byte, &mut rle_lc_used_top) + 4
        };

        let actual = run.min(frame_size - frame.len());
        frame.extend(std::iter::repeat(pixel).take(actual));
    }

    frame.truncate(frame_size);
    Ok(frame)
}

// ===========================================================================
// blood.dat parser
// ===========================================================================

fn extract_dat(dat: &Path, out_dir: &Path) -> Result<u32, Box<dyn Error>> {
    let mut f = File::open(dat)?;
    let mut count = 0u32;

    f.seek(SeekFrom::Start(2))?;

    loop {
        if f.stream_position()? >= 65536 {
            break;
        }

        let mut name_buf = [0u8; 16];
        if f.read_exact(&mut name_buf).is_err() {
            break;
        }
        let name_len = name_buf.iter().position(|&b| b == 0).unwrap_or(16);
        if name_len == 0 {
            break;
        }
        let name = String::from_utf8_lossy(&name_buf[..name_len])
            .to_lowercase()
            .replace('\\', "/");

        let mut buf4 = [0u8; 4];
        f.read_exact(&mut buf4)?;
        let size = i32::from_le_bytes(buf4);
        f.read_exact(&mut buf4)?;
        let offset = i32::from_le_bytes(buf4);

        f.seek(SeekFrom::Current(1))?;

        if size <= 0 || offset < 0 {
            continue;
        }

        let resume = f.stream_position()?;

        let out_path = out_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        f.seek(SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        if f.read_exact(&mut data).is_ok() {
            if let Ok(mut out) = File::create(&out_path) {
                let _ = out.write_all(&data);
                count += 1;
            }
        }

        f.seek(SeekFrom::Start(resume))?;
    }

    Ok(count)
}

// ===========================================================================
// Helpers
// ===========================================================================

fn run_ffmpeg(input: &Path, output: &Path, extra: &[&str]) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-i"]).arg(input);
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.args(["-v", "warning"]).arg(output);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

fn find_file_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let target_lower = target.to_lowercase();
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, target) {
                return Some(found);
            }
        } else if path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase() == target_lower)
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn require(cmd: &str) {
    if !cmd_exists(cmd) {
        eprintln!("Required tool not found: {cmd}");
        std::process::exit(1);
    }
}

fn require_any(cmds: &[&str]) -> String {
    for cmd in cmds {
        if cmd_exists(cmd) {
            return cmd.to_string();
        }
    }
    eprintln!("Required tool not found (need one of: {})", cmds.join(", "));
    std::process::exit(1)
}

fn cmd_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
