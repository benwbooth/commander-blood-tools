use super::*;

// ===========================================================================
// DESCRIPT.DES parser
// ===========================================================================

#[derive(Clone, Debug)]
pub(super) struct SubtitleCue {
    pub(super) tick: u16,
    pub(super) text: String,
}

#[derive(Clone, Debug)]
pub(super) struct DescriptRecord {
    pub(super) name: String,
    pub(super) kind: u8,
    pub(super) music: Vec<String>,
    pub(super) full_hnms: Vec<String>,
    /// Location landscape backgrounds (slot, LBM): the static surface images the
    /// dialogue plays over (the `0x03` Background commands), distinct from the
    /// `full_hnms` planet/orbital view. See re/REVERSE.md.
    pub(super) backgrounds: Vec<(u8, String)>,
    pub(super) sequence_hnms: Vec<String>,
    pub(super) idle_hnms: Vec<(u8, String)>,
    pub(super) talk_hnms: Vec<(u8, String)>,
    pub(super) snd: Option<String>,
    pub(super) sprite: Option<String>,
    pub(super) labels: Vec<String>,
    pub(super) subtitles: Vec<SubtitleCue>,
}

#[derive(Clone, Debug)]
pub(super) struct DescriptDb {
    pub(super) records: Vec<DescriptRecord>,
}

#[derive(Clone, Debug)]
pub(super) struct CharacterScene {
    pub(super) record_name: String,
    pub(super) talk_hnms: Vec<(u8, String)>,
}

#[derive(Clone, Debug)]
pub(super) struct DescriptVideoScene {
    pub(super) record_name: String,
    pub(super) kind: u8,
    pub(super) music: Option<String>,
    pub(super) hnms: Vec<String>,
    pub(super) subtitles: Vec<SubtitleCue>,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptSpeechLine {
    pub(super) script: String,
    pub(super) function_name: String,
    pub(super) offset: usize,
    pub(super) actor_record: Option<String>,
    pub(super) param0: Option<u8>,
    pub(super) param1: Option<u8>,
    pub(super) clip_index: Option<usize>,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) source: String,
    pub(super) text: String,
    pub(super) call_target: u16,
    pub(super) params_hex: String,
    pub(super) text_end: usize,
    pub(super) actor_ref: Option<u16>,
    pub(super) actor_proof: String,
    pub(super) word_count: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptExecutedSpeechLine {
    pub(super) script: String,
    pub(super) sequence_index: usize,
    pub(super) function_name: String,
    pub(super) offset: usize,
    pub(super) actor_record: Option<String>,
    pub(super) actor_ref: Option<u16>,
    pub(super) location_offset: Option<u16>,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) param0: u8,
    pub(super) param1: u8,
    pub(super) clip_index: Option<usize>,
    pub(super) text: String,
    pub(super) call_target: u16,
    pub(super) text_end: usize,
    pub(super) source: String,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptActorRef {
    pub(super) talk_ref: u16,
    pub(super) record_name: String,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) talk_count: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptDisassemblyLine {
    pub(super) script: String,
    pub(super) function_name: String,
    pub(super) offset: usize,
    pub(super) len: usize,
    pub(super) opcode: String,
    pub(super) mnemonic: String,
    pub(super) operands: String,
    pub(super) actor_record: Option<String>,
    pub(super) text: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptBranchTraceLine {
    pub(super) script: String,
    pub(super) event_index: usize,
    pub(super) offset: usize,
    pub(super) opcode: u8,
    pub(super) target: Option<u16>,
    pub(super) branch_taken: bool,
    pub(super) condition_passed: Option<bool>,
    pub(super) stack_depth: usize,
    pub(super) detail: String,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptBranchScenarioLine {
    pub(super) script: String,
    pub(super) scenario_id: String,
    pub(super) decision_index: usize,
    pub(super) forced_offset: usize,
    pub(super) opcode: u8,
    pub(super) default_condition_passed: bool,
    pub(super) forced_condition_passed: bool,
    pub(super) default_text_calls: usize,
    pub(super) scenario_text_calls: usize,
    pub(super) new_text_calls: usize,
    pub(super) lost_text_calls: usize,
    pub(super) first_new_offsets: String,
    pub(super) halted: String,
    pub(super) steps: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ScriptCharacterContextLine {
    pub(super) script: String,
    pub(super) actor_record: String,
    pub(super) actor_object_offset: u16,
    pub(super) actor_talk_ref: u16,
    pub(super) background_record: Option<String>,
    pub(super) background_hnm: Option<String>,
    pub(super) background_music: Option<String>,
    pub(super) source: String,
}

impl DescriptDb {
    pub(super) fn record(&self, name: &str) -> Option<&DescriptRecord> {
        self.records
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn character_names(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| record.kind == 2)
            .map(|record| record.name.to_ascii_lowercase())
            .collect()
    }

    pub(super) fn hnm_music_map(&self) -> HashMap<String, String> {
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

    pub(super) fn character_scenes_for_snd(&self, snd_stem: &str) -> Vec<CharacterScene> {
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

    pub(super) fn video_scenes(
        &self,
        hnm_music: &HashMap<String, String>,
    ) -> Vec<DescriptVideoScene> {
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

pub(super) fn parse_descript(path: &Path) -> Result<DescriptDb, Box<dyn Error>> {
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
            backgrounds: Vec::new(),
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
                    let slot = data.get(pos).copied().unwrap_or(0);
                    pos += 1;
                    let lbm = read_des_media(&data, &mut pos, end, ".lbm");
                    record.backgrounds.push((slot, lbm));
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

pub(super) fn read_des_cstr(data: &[u8], pos: &mut usize, end: usize) -> String {
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

pub(super) fn read_des_media(data: &[u8], pos: &mut usize, end: usize, ext: &str) -> String {
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

pub(super) fn is_des_opcode(byte: u8) -> bool {
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

pub(super) fn media_stem(name: &str) -> String {
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

pub(super) fn safe_file_stem(name: &str) -> String {
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

pub(super) fn write_descript_subtitles(
    db: &DescriptDb,
    out_dir: &Path,
) -> Result<u32, Box<dyn Error>> {
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

pub(super) fn write_descript_manifest(
    db: &DescriptDb,
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
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

pub(super) fn write_verified_video_manifest(
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

pub(super) fn write_character_manifest(
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

pub(super) fn format_srt_time(seconds: f64) -> String {
    let millis = (seconds * 1000.0).round().max(0.0) as u64;
    let h = millis / 3_600_000;
    let m = (millis / 60_000) % 60;
    let s = (millis / 1_000) % 60;
    let ms = millis % 1_000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

pub(super) fn descript_hnm_path(hnm_name: &str, kind: u8) -> PathBuf {
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
