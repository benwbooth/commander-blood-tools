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
            composite_character_frame(&mut rgb, &bg_rgb, frame);

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

pub(super) fn create_character_dialogue_video(
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
                composite_character_frame(&mut rgb, &bg_rgb, frame);

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

pub(super) fn character_background_path(dat_dir: &Path, hnm_name: &str) -> PathBuf {
    let lower = hnm_name.to_ascii_lowercase();
    if lower.ends_with(".hnm") || lower.contains('/') || lower.contains('\\') {
        dat_dir.join(descript_hnm_path(&lower, 1))
    } else {
        dat_dir.join("pl").join(format!("{lower}.hnm"))
    }
}
