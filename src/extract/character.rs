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

struct DialogueSegment {
    text: String,
    active_line_id: u16,
    hnm_path: Option<PathBuf>, // talking-head HNM; None => static background
    voice: Option<SndClip>,    // None => silent (subtitle-only)
    duration: f64,             // seconds this segment occupies the timeline
}

fn silent_dialogue_segment(text: &str, active_line_id: u16) -> DialogueSegment {
    let chars = text.chars().filter(|c| !c.is_control()).count();
    let reveal = chars as f64 / SUBTITLE_CHARS_PER_SEC;
    let duration = (reveal + SILENT_SUBTITLE_HOLD_SEC).max(SILENT_SUBTITLE_MIN_SEC);
    DialogueSegment {
        text: text.to_string(),
        active_line_id,
        hnm_path: None,
        voice: None,
        duration,
    }
}

fn dialogue_subtitle_cues(segments: &[DialogueSegment]) -> (Vec<SubtitleCue>, f64) {
    let mut cues = Vec::new();
    let mut duration = 0.0f64;
    for seg in segments {
        cues.push(SubtitleCue {
            tick: (duration * 10.0).round() as u16,
            active_line_id: Some(seg.active_line_id),
            text: seg.text.clone(),
        });
        duration += seg.duration;
    }
    (cues, duration)
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

    let snd_bank = SndBank::read(snd_path)?;

    // Collect valid clip+animation pairs
    struct ClipInfo {
        hnm_path: PathBuf,
        voice: SndClip,
    }
    let mut clips: Vec<ClipInfo> = Vec::new();

    for i in 0..snd_bank.clip_count().min(scene.talk_hnms.len()) {
        let hnm_name = &scene.talk_hnms[i].1;
        let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
        if !hnm_path.exists() {
            continue;
        }
        let Some(voice) = snd_bank.clip(i).filter(|clip| !clip.pcm.is_empty()) else {
            continue;
        };

        clips.push(ClipInfo {
            hnm_path,
            voice: voice.clone(),
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
            vf.write_all(&clip.voice.pcm)?;
        }
    }

    // Build ffmpeg command. Frames are authored at 320x200, then encoded 3x.
    let music_path = context
        .background_hnm
        .and_then(|bg_name| hnm_music.get(&media_stem(bg_name)))
        .map(|music| dat_dir.join("mu").join(format!("{music}.voc")));
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let sr = clips[0].voice.sample_rate;

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
        let audio_dur = clip.voice.pcm.len() as f64 / clip.voice.sample_rate as f64;
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

pub(super) fn create_executed_dialogue_run_videos(
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    script_speech: &[ScriptExecutedSpeechLine],
    subtitle_sfx_path: Option<&Path>,
) -> Result<u32, Box<dyn Error>> {
    let runs = script_executed_dialogue_runs(script_speech);
    let mut snd_cache: HashMap<PathBuf, SndBank> = HashMap::new();
    let mut created = 0u32;

    for run in runs {
        if create_executed_dialogue_run_video(
            dat_dir,
            mp4_dir,
            descript_db,
            &run,
            subtitle_sfx_path,
            &mut snd_cache,
        )? {
            created += 1;
        }
    }

    Ok(created)
}

pub(super) fn create_profile_dialogue_run_videos(
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    script_speech: &[ScriptProfileExecutedSpeechLine],
    subtitle_sfx_path: Option<&Path>,
) -> Result<u32, Box<dyn Error>> {
    let runs = script_profile_dialogue_runs(script_speech);
    let mut snd_cache: HashMap<PathBuf, SndBank> = HashMap::new();
    let mut created = 0u32;

    for run in runs {
        if create_profile_dialogue_run_video(
            dat_dir,
            mp4_dir,
            descript_db,
            &run,
            subtitle_sfx_path,
            &mut snd_cache,
        )? {
            created += 1;
        }
    }

    Ok(created)
}

fn create_executed_dialogue_run_video(
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    run: &ScriptExecutedDialogueRun<'_>,
    subtitle_sfx_path: Option<&Path>,
    snd_cache: &mut HashMap<PathBuf, SndBank>,
) -> Result<bool, Box<dyn Error>> {
    let inputs: Vec<vm::LineInput> = run
        .lines
        .iter()
        .map(|line| vm::LineInput {
            actor: line.actor_record.clone(),
            background_hnm: line.background_hnm.clone(),
            background_record: line.background_record.clone(),
            background_music: line.background_music.clone(),
            voice_selector: line.param0,
            active_line_id: line.active_line_id,
            flags_b4: line.param1,
            clip_index: line.clip_index,
            text: line.text.clone(),
        })
        .collect();
    let events = vm::emit_scene_events(&inputs);

    let mut segments: Vec<DialogueSegment> = Vec::new();
    let mut ev_background: Option<String> = None;
    let mut ev_background_record: Option<String> = None;
    let mut ev_music: Option<String> = None;
    let mut current_actor: Option<String> = None;
    let mut pending_clip: Option<(Option<String>, usize)> = None;

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
            vm::SceneEvent::ShowSpeaker { actor } => current_actor = Some(actor.clone()),
            vm::SceneEvent::PlayVoice { clip_index } => {
                pending_clip = Some((current_actor.clone(), *clip_index));
            }
            vm::SceneEvent::DrawSubtitle {
                text,
                active_line_id,
                ..
            } => {
                if let Some((Some(actor), clip_index)) = pending_clip.take() {
                    if let Some(seg) = resolve_actor_voiced_segment(
                        dat_dir,
                        descript_db,
                        snd_cache,
                        &actor,
                        clip_index,
                        text,
                        *active_line_id,
                    ) {
                        segments.push(seg);
                        continue;
                    }
                }
                if !text.trim().is_empty() {
                    segments.push(silent_dialogue_segment(text, *active_line_id));
                }
            }
            _ => {}
        }
    }

    let output_stem = executed_dialogue_run_output_stem(run);
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let label = if let Some(scenario_id) = &run.scenario_id {
        format!("{scenario_id} run {}", run.run_index)
    } else {
        format!("{} run {}", run.script, run.run_index)
    };
    render_dialogue_segments(
        &mp4_out,
        &output_stem,
        &label,
        dat_dir,
        descript_db,
        &segments,
        ev_background_record.as_deref(),
        ev_background.as_deref(),
        ev_music,
        subtitle_sfx_path,
    )
}

fn create_profile_dialogue_run_video(
    dat_dir: &Path,
    mp4_dir: &Path,
    descript_db: &DescriptDb,
    run: &ScriptProfileDialogueRun<'_>,
    subtitle_sfx_path: Option<&Path>,
    snd_cache: &mut HashMap<PathBuf, SndBank>,
) -> Result<bool, Box<dyn Error>> {
    let inputs: Vec<vm::LineInput> = run
        .lines
        .iter()
        .map(|line| vm::LineInput {
            actor: line.row.actor_record.clone(),
            background_hnm: line.row.background_hnm.clone(),
            background_record: line.row.background_record.clone(),
            background_music: line.row.background_music.clone(),
            voice_selector: line.row.param0,
            active_line_id: line.row.active_line_id,
            flags_b4: line.row.param1,
            clip_index: line.row.clip_index,
            text: line.row.text.clone(),
        })
        .collect();
    let events = vm::emit_scene_events(&inputs);

    let mut segments: Vec<DialogueSegment> = Vec::new();
    let mut ev_background: Option<String> = None;
    let mut ev_background_record: Option<String> = None;
    let mut ev_music: Option<String> = None;
    let mut current_actor: Option<String> = None;
    let mut pending_clip: Option<(Option<String>, usize)> = None;

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
            vm::SceneEvent::ShowSpeaker { actor } => current_actor = Some(actor.clone()),
            vm::SceneEvent::PlayVoice { clip_index } => {
                pending_clip = Some((current_actor.clone(), *clip_index));
            }
            vm::SceneEvent::DrawSubtitle {
                text,
                active_line_id,
                ..
            } => {
                if let Some((Some(actor), clip_index)) = pending_clip.take() {
                    if let Some(seg) = resolve_actor_voiced_segment(
                        dat_dir,
                        descript_db,
                        snd_cache,
                        &actor,
                        clip_index,
                        text,
                        *active_line_id,
                    ) {
                        segments.push(seg);
                        continue;
                    }
                }
                if !text.trim().is_empty() {
                    segments.push(silent_dialogue_segment(text, *active_line_id));
                }
            }
            _ => {}
        }
    }

    let output_stem = profile_dialogue_run_output_stem(run);
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let label = format!("{} profile run {}", run.sequence_id, run.run_index);
    render_dialogue_segments(
        &mp4_out,
        &output_stem,
        &label,
        dat_dir,
        descript_db,
        &segments,
        ev_background_record.as_deref(),
        ev_background.as_deref(),
        ev_music,
        subtitle_sfx_path,
    )
}

fn resolve_actor_voiced_segment(
    dat_dir: &Path,
    descript_db: &DescriptDb,
    snd_cache: &mut HashMap<PathBuf, SndBank>,
    actor: &str,
    clip_index: usize,
    text: &str,
    active_line_id: u16,
) -> Option<DialogueSegment> {
    let record = descript_db.record(actor)?;
    if clip_index >= record.talk_hnms.len() {
        return None;
    }
    let hnm_name = &record.talk_hnms[clip_index].1;
    let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
    if !hnm_path.exists() {
        return None;
    }
    let snd_name = record.snd.as_ref()?;
    let snd_path = dat_dir.join("sn").join(snd_name.to_ascii_lowercase());
    if !snd_cache.contains_key(&snd_path) {
        snd_cache.insert(snd_path.clone(), SndBank::read(&snd_path).ok()?);
    }
    let voice = snd_cache
        .get(&snd_path)?
        .clip(clip_index)
        .filter(|clip| !clip.pcm.is_empty())?
        .clone();
    let duration = voice.pcm.len() as f64 / voice.sample_rate as f64;
    Some(DialogueSegment {
        text: text.to_string(),
        active_line_id,
        hnm_path: Some(hnm_path),
        voice: Some(voice),
        duration,
    })
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

    let snd_bank = SndBank::read(snd_path)?;

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
            active_line_id: line.active_line_id,
            flags_b4: line.param1,
            clip_index: line.clip_index,
            text: line.text.clone(),
        })
        .collect();
    let events = vm::emit_scene_events(&inputs);

    let resolve_voiced =
        |clip_index: usize, text: &str, active_line_id: u16| -> Option<DialogueSegment> {
            if clip_index >= scene.talk_hnms.len() {
                return None;
            }
            let hnm_name = &scene.talk_hnms[clip_index].1;
            let hnm_path = dat_dir.join("pe").join(hnm_name.to_ascii_lowercase());
            if !hnm_path.exists() {
                return None;
            }
            let voice = snd_bank
                .clip(clip_index)
                .filter(|clip| !clip.pcm.is_empty())?
                .clone();
            let duration = voice.pcm.len() as f64 / voice.sample_rate as f64;
            Some(DialogueSegment {
                text: text.to_string(),
                active_line_id,
                hnm_path: Some(hnm_path),
                voice: Some(voice),
                duration,
            })
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
            vm::SceneEvent::DrawSubtitle {
                text,
                active_line_id,
                ..
            } => {
                // Voiced line: play its clip + talking head. If the clip can't be
                // resolved (e.g. missing asset) but the line has text, fall back
                // to a subtitle-only segment instead of dropping the dialogue.
                if let Some(ci) = pending_clip.take() {
                    if let Some(seg) = resolve_voiced(ci, text, *active_line_id) {
                        segments.push(seg);
                        continue;
                    }
                }
                if !text.trim().is_empty() {
                    segments.push(silent_dialogue_segment(text, *active_line_id));
                }
            }
            _ => {}
        }
    }

    let output_stem = format!(
        "dialogue - {} - {} - {}",
        safe_file_stem(script),
        safe_file_stem(function_name),
        safe_file_stem(&scene.record_name)
    );
    let mp4_out = mp4_dir.join(format!("{output_stem}.mp4"));
    let label = format!("{script} {function_name}");
    render_dialogue_segments(
        &mp4_out,
        &output_stem,
        &label,
        dat_dir,
        descript_db,
        &segments,
        ev_background_record.as_deref(),
        ev_background.as_deref(),
        ev_music,
        subtitle_sfx_path,
    )
}

fn render_dialogue_segments(
    mp4_out: &Path,
    output_stem: &str,
    label: &str,
    dat_dir: &Path,
    descript_db: &DescriptDb,
    segments: &[DialogueSegment],
    background_record: Option<&str>,
    background_hnm: Option<&str>,
    music_name: Option<String>,
    subtitle_sfx_path: Option<&Path>,
) -> Result<bool, Box<dyn Error>> {
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
        return Err(format!("{label} uses mixed SND sample rates").into());
    }

    // The dialogue plays over the location's LANDSCAPE (a static LBM from the
    // DESCRIPT Location `Background` commands), NOT the planet `FullHnm`. Resolve
    // the landscape LBM from the location record; fall back to the planet HNM
    // only if the location has no landscape (see re/REVERSE.md).
    let landscape_lbm = background_record
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

    let tmp_voice =
        mp4_out.with_file_name(format!("_tmp_{}_voice.raw", safe_file_stem(output_stem)));
    {
        let mut vf = File::create(&tmp_voice)?;
        for seg in segments {
            match &seg.voice {
                Some(v) => vf.write_all(&v.pcm)?,
                None => {
                    // Subtitle-only segment: unsigned-8-bit PCM silence (0x80) for
                    // the segment's duration, keeping audio aligned with video.
                    let samples = (seg.duration * sr as f64).round() as usize;
                    vf.write_all(&vec![0x80u8; samples])?;
                }
            }
        }
    }

    let (cues, duration) = dialogue_subtitle_cues(segments);

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
            "-c:v", "libx264", "-crf", "18", "-preset", "medium", "-pix_fmt", "yuv420p", "-c:a",
            "aac", "-b:a", "128k", "-v", "warning",
        ]);
        cmd.arg(&mp4_out);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut ffmpeg = cmd.spawn()?;
        let mut stdin = ffmpeg.stdin.take().ok_or("no stdin")?;
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];
        let mut global_frame = 0usize;

        for seg in segments {
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
                if let Err(err) = stdin.write_all(&rgb) {
                    drop(stdin);
                    let output = ffmpeg.wait_with_output()?;
                    return Err(format!(
                        "{label}: ffmpeg pipe write failed: {err}; {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
                global_frame += 1;
            }
        }

        drop(stdin);
        let output = ffmpeg.wait_with_output()?;
        if !output.status.success() {
            return Err(format!(
                "{label}: ffmpeg exited unsuccessfully: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
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
        let param0 = clip_index.map(|idx| idx as u8 + 1).unwrap_or(0xff);
        ScriptExecutedSpeechLine {
            scenario_id: None,
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
            param0,
            param1: 0,
            active_line_id: vm::text_selector_active_line_id(param0),
            clip_index,
            text: text.to_string(),
            call_target: 0x1234,
            text_end: offset + 12,
            source: "test".to_string(),
        }
    }

    #[test]
    fn dialogue_subtitle_cues_keep_active_line_ids() {
        let segments = vec![
            DialogueSegment {
                text: "first".to_string(),
                active_line_id: vm::text_selector_active_line_id(0x01),
                hnm_path: None,
                voice: None,
                duration: 1.2,
            },
            DialogueSegment {
                text: "second".to_string(),
                active_line_id: vm::text_selector_active_line_id(0xff),
                hnm_path: None,
                voice: None,
                duration: 0.8,
            },
        ];

        let (cues, duration) = dialogue_subtitle_cues(&segments);
        assert_eq!(duration, 2.0);
        assert_eq!(cues[0].tick, 0);
        assert_eq!(
            cues[0].active_line_id,
            Some(vm::text_selector_active_line_id(0x01))
        );
        assert_eq!(cues[1].tick, 12);
        assert_eq!(
            cues[1].active_line_id,
            Some(vm::text_selector_active_line_id(0xff))
        );
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
