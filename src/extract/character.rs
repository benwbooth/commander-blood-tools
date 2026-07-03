use super::*;

// ===========================================================================
// Character video — voice + animation + background + music
// ===========================================================================

pub(super) struct CharacterContext {
    pub(super) record_name: &'static str,
    pub(super) background_hnm: Option<&'static str>,
}

/// DESCRIPT.DES gives the character foreground and voice bank. Some character
/// records are standalone talking heads/objects; others are composited over the
/// active room. Keep the room part isolated until SCRIPT*.COD can replace it.
pub(super) const CHAR_CONTEXTS: &[CharacterContext] = &[
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

pub(super) fn char_contents() -> &'static [CharacterContext] {
    CHAR_CONTEXTS
}

pub(super) fn lookup_character_context(record_name: &str) -> Option<&'static CharacterContext> {
    char_contents()
        .iter()
        .find(|ctx| ctx.record_name.eq_ignore_ascii_case(record_name))
}

/// Create combined character videos for each DESCRIPT.DES character record that
/// uses this SND bank.
pub(super) fn create_character_videos(
    snd_path: &Path,
    snd_stem: &str,
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: Option<&DescriptDb>,
    hnm_music: &HashMap<String, String>,
    script_speech: &[ScriptExecutedSpeechLine],
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
            db,
            script_speech,
            subtitle_sfx_path,
        )?;
    }
    Ok(created)
}

/// Create a combined character video: all voice clips back-to-back with looping
/// foreground animations from DESCRIPT.DES, composited on the location background.
pub(super) fn create_character_video_from_scene(
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

    // Build ffmpeg command. Frames are authored at 320x200, then encoded 3x.
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
        cmd.arg("-filter_complex")
            .arg(scaled_video_filter(Some(
                "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[voice][music]amix=inputs=2:duration=first[aout]",
            )))
            .args(["-map", "[vout]", "-map", "[aout]"]);
    } else {
        cmd.arg("-filter_complex")
            .arg(scaled_video_filter(None))
            .args(["-map", "[vout]", "-map", "1:a"]);
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
        let frames = decode_character_animation(&hnm);
        if frames.is_empty() {
            continue;
        }

        for out_f in 0..total_frames {
            let frame = &frames[out_f % frames.len()];
            composite_character_frame(&mut rgb, &bg_rgb, frame, false);

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

pub(super) fn create_character_dialogue_videos_from_scene(
    snd_path: &Path,
    scene: &CharacterScene,
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    script_speech: &[ScriptExecutedSpeechLine],
    subtitle_sfx_path: Option<&Path>,
) -> Result<u32, Box<dyn Error>> {
    // Combine the branch-aware executed lines into one longer video per
    // (script, location): all of this character's dialogue at a given location,
    // in game execution order. (Keeping it per-location preserves a single
    // correct background per video; a character at several locations gets one
    // video each.)
    let groups = executed_dialogue_groups_for_scene(scene, script_speech);

    let mut created = 0u32;
    for ((script, location), lines) in groups {
        if create_character_dialogue_video(
            snd_path,
            scene,
            dat_dir,
            mp4_dir,
            descript_db,
            subtitle_sfx_path,
            &script,
            &location,
            &lines,
        )? {
            created += 1;
        }
    }

    Ok(created)
}

fn executed_dialogue_groups_for_scene<'a>(
    scene: &CharacterScene,
    script_speech: &'a [ScriptExecutedSpeechLine],
) -> Vec<((String, String), Vec<&'a ScriptExecutedSpeechLine>)> {
    let mut groups: BTreeMap<(String, String), Vec<&ScriptExecutedSpeechLine>> = BTreeMap::new();
    for line in script_speech {
        if !line
            .actor_record
            .as_deref()
            .is_some_and(|actor| actor.eq_ignore_ascii_case(&scene.record_name))
        {
            continue;
        }
        // Keep the line if it has a resolvable voice clip (voiced, talking-head)
        // OR non-empty subtitle text (voiceless: b3==0xFF radio/narrator text the
        // player still saw, rendered subtitle-only). Drop only lines with neither.
        let has_voice = line
            .clip_index
            .is_some_and(|clip_index| clip_index < scene.talk_hnms.len());
        let has_text = !line.text.trim().is_empty();
        if !has_voice && !has_text {
            continue;
        }
        let location = line
            .background_record
            .clone()
            .unwrap_or_else(|| "nolocation".to_string());
        groups
            .entry((line.script.clone(), location))
            .or_default()
            .push(line);
    }

    let mut ordered: Vec<_> = groups.into_iter().collect();
    for (_, lines) in ordered.iter_mut() {
        lines.sort_by_key(|line| line.sequence_index);
    }
    ordered
}

pub(super) fn create_character_dialogue_video(
    snd_path: &Path,
    scene: &CharacterScene,
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    subtitle_sfx_path: Option<&Path>,
    script: &str,
    function_name: &str,
    lines: &[&ScriptExecutedSpeechLine],
) -> Result<bool, Box<dyn Error>> {
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

    struct VoiceData {
        pcm_start: usize,
        pcm_len: usize,
        sample_rate: u32,
    }
    // One renderable dialogue segment in execution order. A voiced segment plays
    // its son.snd clip over the paired talking-head HNM; a subtitle-only segment
    // (a voiceless b3==0xFF line — radio/narrator text) shows the scene
    // background with no voice and no talking head.
    struct DialogueSegment {
        text: String,
        hnm_path: Option<PathBuf>, // talking-head HNM; None => static background
        voice: Option<VoiceData>,  // None => silent (subtitle-only)
        duration: f64,             // seconds this segment occupies the timeline
    }

    // Render from the VM presentation-event stream rather than scanning the
    // grouped lines directly. `emit_scene_events` turns the decoded per-line
    // fields into the ordered event stream the game's presentation layer
    // effectively produces (SetBackground / PlayMusic / PlayVoice /
    // DrawSubtitle / ...); this is the VM-event-driven render path. Behaviour is
    // preserved: a line contributes a clip+subtitle only when its voice clip
    // resolves, and background/music take the first set value (else context).
    let inputs: Vec<vm::LineInput> = lines
        .iter()
        .map(|line| vm::LineInput {
            actor: line.actor_record.clone(),
            background_hnm: line.background_hnm.clone(),
            background_record: line.background_record.clone(),
            background_music: line.background_music.clone(),
            voice_selector: line.param0,
            flags_b4: line.param1,
            clip_index: line.clip_index,
            text: line.text.clone(),
        })
        .collect();
    let events = vm::emit_scene_events(&inputs);

    let resolve_voiced = |clip_index: usize, text: &str| -> Option<DialogueSegment> {
        if clip_index >= num_clips || clip_index >= scene.talk_hnms.len() {
            return None;
        }
        let hnm_name = &scene.talk_hnms[clip_index].1;
        let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
        if !hnm_path.exists() {
            return None;
        }
        let cs = header_end + clip_offsets[clip_index];
        let ce = header_end + clip_offsets[clip_index + 1];
        if cs + 6 > snd_data.len() || ce > snd_data.len() || ce <= cs {
            return None;
        }
        if snd_data[cs] != 1 {
            return None;
        }
        let sr_code = snd_data[cs + 4];
        let sample_rate = if sr_code < 255 {
            1_000_000 / (256 - sr_code as u32)
        } else {
            11111
        };
        let pcm_len = ce - (cs + 6);
        Some(DialogueSegment {
            text: text.to_string(),
            hnm_path: Some(hnm_path),
            voice: Some(VoiceData {
                pcm_start: cs + 6,
                pcm_len,
                sample_rate,
            }),
            duration: pcm_len as f64 / sample_rate as f64,
        })
    };

    // A voiceless line (no resolvable clip): subtitle-only. Duration = reveal
    // time at the game's char rate + a readable hold (see the SILENT_SUBTITLE_*
    // consts). No talking head, no voice.
    let silent_segment = |text: &str| -> DialogueSegment {
        let chars = text.chars().filter(|c| !c.is_control()).count();
        let reveal = chars as f64 / SUBTITLE_CHARS_PER_SEC;
        let duration = (reveal + SILENT_SUBTITLE_HOLD_SEC).max(SILENT_SUBTITLE_MIN_SEC);
        DialogueSegment {
            text: text.to_string(),
            hnm_path: None,
            voice: None,
            duration,
        }
    };

    let mut segments: Vec<DialogueSegment> = Vec::new();
    let mut ev_background: Option<String> = None;
    let mut ev_background_record: Option<String> = None;
    let mut ev_music: Option<String> = None;
    let mut pending_clip: Option<usize> = None;
    for event in &events {
        match event {
            vm::SceneEvent::SetBackground { hnm, record } => {
                if ev_background.is_none() {
                    ev_background = hnm.clone();
                }
                if ev_background_record.is_none() {
                    ev_background_record = record.clone();
                }
            }
            vm::SceneEvent::PlayMusic { music } => {
                if ev_music.is_none() {
                    ev_music = music.clone();
                }
            }
            vm::SceneEvent::PlayVoice { clip_index } => pending_clip = Some(*clip_index),
            vm::SceneEvent::DrawSubtitle { text, .. } => {
                // Voiced line: play its clip + talking head. If the clip can't be
                // resolved (e.g. missing asset) but the line has text, fall back
                // to a subtitle-only segment instead of dropping the dialogue.
                if let Some(ci) = pending_clip.take() {
                    if let Some(seg) = resolve_voiced(ci, text) {
                        segments.push(seg);
                        continue;
                    }
                }
                if !text.trim().is_empty() {
                    segments.push(silent_segment(text));
                }
            }
            _ => {}
        }
    }

    if segments.is_empty() {
        return Ok(false);
    }

    // The whole timeline's audio is one concatenated u8 PCM track at a single
    // rate: voiced segments share their son.snd rate; silent segments emit
    // silence at that rate (or SILENT_SUBTITLE_SR when the scene is entirely
    // subtitle-only with no voiced segment to inherit a rate from).
    let sr = segments
        .iter()
        .find_map(|seg| seg.voice.as_ref().map(|v| v.sample_rate))
        .unwrap_or(SILENT_SUBTITLE_SR);
    if segments
        .iter()
        .any(|seg| seg.voice.as_ref().is_some_and(|v| v.sample_rate != sr))
    {
        return Err(format!("{} {} uses mixed SND sample rates", script, function_name).into());
    }

    // Background / music are the values computed for this scene from the script
    // VM + DESCRIPT (the actor's location → that location's HNM → its music),
    // surfaced via the event stream's first SetBackground / PlayMusic. No static
    // char-context fallback and no re-guessing: if the data doesn't specify one,
    // there isn't one.
    let background_hnm = ev_background.as_deref();
    let music_name = ev_music.clone();

    // The dialogue plays over the location's LANDSCAPE (a static LBM from the
    // DESCRIPT Location `Background` commands), NOT the planet `FullHnm`. Resolve
    // the landscape LBM from the location record; fall back to the planet HNM
    // only if the location has no landscape (see re/REVERSE.md).
    let landscape_lbm = ev_background_record
        .as_deref()
        .and_then(|loc| descript_db.record(loc))
        .filter(|r| r.kind == 1)
        .and_then(|r| r.backgrounds.first().map(|(_, lbm)| lbm.clone()));

    // Letterbox (scene-band) layout when this is a located/planet dialogue with a
    // landscape; full-screen for no-location close-ups (the ship/intro view).
    let letterbox = landscape_lbm.is_some();
    let bg = landscape_lbm
        .as_deref()
        .and_then(|lbm| load_landscape_lbm(dat_dir, lbm))
        .or_else(|| background_hnm.and_then(|hnm| load_planet_hnm(dat_dir, hnm)))
        .unwrap_or_else(|| (vec![0u8; VIEWPORT_W * VIEWPORT_H], [[0u8; 3]; 256]));
    let (bg_fb, bg_pal) = bg;
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
        for seg in &segments {
            match &seg.voice {
                Some(v) => vf.write_all(&snd_data[v.pcm_start..v.pcm_start + v.pcm_len])?,
                None => {
                    // Subtitle-only segment: unsigned-8-bit PCM silence (0x80) for
                    // the segment's duration, keeping audio aligned with video.
                    let samples = (seg.duration * sr as f64).round() as usize;
                    vf.write_all(&vec![0x80u8; samples])?;
                }
            }
        }
    }

    let mut cues = Vec::new();
    let mut duration = 0.0f64;
    for seg in &segments {
        cues.push(SubtitleCue {
            tick: (duration * 10.0).round() as u16,
            text: seg.text.clone(),
        });
        duration += seg.duration;
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

        let audio_filter = match (has_music, subtitle_sfx_rate.is_some()) {
            (true, true) => Some(
                "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[3:a]volume=0.75[sfx];[voice][music][sfx]amix=inputs=3:duration=first[aout]",
            ),
            (true, false) => Some(
                "[1:a]volume=1.0[voice];[2:a]volume=0.25[music];[voice][music]amix=inputs=2:duration=first[aout]",
            ),
            (false, true) => Some(
                "[1:a]volume=1.0[voice];[2:a]volume=0.75[sfx];[voice][sfx]amix=inputs=2:duration=first[aout]",
            ),
            (false, false) => None,
        };
        cmd.arg("-filter_complex")
            .arg(scaled_video_filter(audio_filter))
            .args(["-map", "[vout]"]);
        if audio_filter.is_some() {
            cmd.args(["-map", "[aout]"]);
        } else {
            cmd.args(["-map", "1:a"]);
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

        for seg in &segments {
            let total_frames = (seg.duration * HNM_FPS as f64).ceil() as usize;
            // Voiced segment: the paired talking-head HNM, looped. Subtitle-only
            // segment (no hnm_path, or an HNM that fails to decode): the static
            // scene background with the subtitle over it.
            let frames = match &seg.hnm_path {
                Some(path) => decode_character_animation(&HnmFile::open(path)?),
                None => Vec::new(),
            };

            for out_f in 0..total_frames {
                if frames.is_empty() {
                    composite_scene_background(&mut rgb, &bg_rgb, letterbox);
                } else {
                    let frame = &frames[out_f % frames.len()];
                    composite_character_frame(&mut rgb, &bg_rgb, frame, letterbox);
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

/// Load a location landscape background from its LBM (`dat_dir/fd/<lbm>`),
/// decoded to a VIEWPORT-sized indexed framebuffer + palette.
fn load_landscape_lbm(dat_dir: &Path, lbm: &str) -> Option<(Vec<u8>, [[u8; 3]; 256])> {
    let path = dat_dir.join("fd").join(lbm.to_ascii_lowercase());
    let data = fs::read(&path).ok()?;
    let img = lbm::decode_lbm(&data)?;
    let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
    for y in 0..VIEWPORT_H.min(img.height) {
        for x in 0..VIEWPORT_W.min(img.width) {
            fb[y * VIEWPORT_W + x] = img.pixels[y * img.width + x];
        }
    }
    Some((fb, img.palette))
}

/// Load frame 0 of a planet/orbital `FullHnm` (the fallback when a location has
/// no landscape LBM).
fn load_planet_hnm(dat_dir: &Path, hnm: &str) -> Option<(Vec<u8>, [[u8; 3]; 256])> {
    let bg_path = character_background_path(dat_dir, hnm);
    if !bg_path.exists() {
        return None;
    }
    let bg_hnm = HnmFile::open(&bg_path).ok()?;
    let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
    let mut pal = bg_hnm.palette;
    bg_hnm.decode_frame(0, &mut fb, &mut pal);
    Some((fb, pal))
}

pub(super) fn character_background_path(dat_dir: &Path, hnm_name: &str) -> PathBuf {
    let lower = hnm_name.to_ascii_lowercase();
    if lower.ends_with(".hnm") || lower.contains('/') || lower.contains('\\') {
        dat_dir.join(descript_hnm_path(&lower, 1))
    } else {
        dat_dir.join("pl").join(format!("{lower}.hnm"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executed_line(
        script: &str,
        sequence_index: usize,
        offset: usize,
        actor: Option<&str>,
        location: Option<&str>,
        clip_index: Option<usize>,
        text: &str,
    ) -> ScriptExecutedSpeechLine {
        ScriptExecutedSpeechLine {
            script: script.to_string(),
            sequence_index,
            function_name: "func".to_string(),
            offset,
            actor_record: actor.map(str::to_string),
            actor_ref: actor.map(|_| 0x003a),
            location_offset: location.map(|_| 0x1000),
            background_record: location.map(str::to_string),
            background_hnm: location.map(|loc| format!("{loc}.hnm")),
            background_music: location.map(|loc| format!("{loc}_music")),
            param0: clip_index.map(|idx| idx as u8 + 1).unwrap_or(0xff),
            param1: 0,
            clip_index,
            text: text.to_string(),
            call_target: 0x1234,
            text_end: offset + 12,
            source: "test".to_string(),
        }
    }

    #[test]
    fn executed_dialogue_groups_filter_actor_and_sort_by_sequence() {
        let scene = CharacterScene {
            record_name: "Actor_A".to_string(),
            talk_hnms: vec![(0, "a.hnm".to_string()), (1, "b.hnm".to_string())],
        };
        let rows = vec![
            executed_line(
                "SCRIPT2",
                1,
                0x10,
                Some("Actor_A"),
                Some("Room1"),
                Some(2),
                "late",
            ),
            executed_line(
                "SCRIPT2",
                0,
                0x50,
                Some("Actor_A"),
                Some("Room1"),
                Some(1),
                "early",
            ),
            executed_line(
                "SCRIPT2",
                2,
                0x30,
                Some("Actor_B"),
                Some("Room1"),
                Some(0),
                "other",
            ),
            executed_line(
                "SCRIPT2",
                3,
                0x40,
                Some("Actor_A"),
                Some("Room2"),
                None,
                "silent",
            ),
        ];

        let groups = executed_dialogue_groups_for_scene(&scene, &rows);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, ("SCRIPT2".to_string(), "Room1".to_string()));
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[0].1[0].sequence_index, 0);
        assert_eq!(groups[0].1[0].clip_index, Some(1));
        assert_eq!(groups[0].1[1].sequence_index, 1);
        assert_eq!(groups[0].1[1].clip_index, Some(2));
        assert_eq!(groups[1].0, ("SCRIPT2".to_string(), "Room2".to_string()));
        assert_eq!(groups[1].1[0].text, "silent");
    }
}
