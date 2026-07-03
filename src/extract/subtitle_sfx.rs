use super::*;

pub(super) fn build_subtitle_sfx_track(
    cues: &[SubtitleCue],
    duration: f64,
    snd_path: &Path,
    out_path: &Path,
) -> Result<Option<u32>, Box<dyn Error>> {
    let clips = subtitle_sfx_clips(snd_path)?;
    if clips.is_empty() {
        return Ok(None);
    }
    let sample_rate = clips[0].sample_rate;

    let samples = ((duration + 0.5) * sample_rate as f64).ceil() as usize;
    let mut track = vec![128u8; samples.max(1)];
    let mut used = false;
    // The text/presentation SND entry caller at BLOODPRG.EXE file 0x8534 enters
    // the player with AX=0; the shipped scene manifest also identifies tb.snd#0.
    let chatter_clip = &clips[0];
    for event in subtitle_chatter_events(cues) {
        let start = (event.start_time * sample_rate as f64).round() as usize;
        if start >= track.len() {
            continue;
        }
        used = true;
        for (idx, &sample) in chatter_clip.pcm.iter().enumerate() {
            let pos = start + idx;
            if pos >= track.len() {
                break;
            }
            let mixed = track[pos] as i16 + sample as i16 - 128;
            track[pos] = mixed.clamp(0, 255) as u8;
        }
    }

    if !used {
        return Ok(None);
    }
    fs::write(out_path, track)?;
    Ok(Some(sample_rate))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SubtitleChatterEvent {
    start_time: f64,
}

fn subtitle_chatter_events(cues: &[SubtitleCue]) -> Vec<SubtitleChatterEvent> {
    cues.iter()
        .filter_map(|cue| {
            let text = cue.text.trim();
            let reveal_chars = subtitle_reveal_char_count(text);
            if reveal_chars == 0 {
                return None;
            }

            // BLOODPRG.EXE 0x94A4..0x94DD advances the reveal pointer, then sets
            // gs:0x67bb only after that pointer reaches the terminating NUL.
            // This is a line-complete chatter event, not one SFX per character.
            let cue_start = cue.tick as f64 / 10.0;
            Some(SubtitleChatterEvent {
                start_time: cue_start + reveal_chars as f64 / SUBTITLE_CHARS_PER_SEC,
            })
        })
        .collect()
}

fn subtitle_reveal_char_count(text: &str) -> usize {
    text.chars().filter(|ch| *ch != '\n' && *ch != '\r').count()
}

pub(super) struct SndClip {
    pub(super) pcm: Vec<u8>,
    pub(super) sample_rate: u32,
}

pub(super) fn subtitle_sfx_clips(snd_path: &Path) -> Result<Vec<SndClip>, Box<dyn Error>> {
    let mut clips = read_snd_clips(snd_path)?;
    clips.retain(|clip| !clip.pcm.is_empty());
    if clips.is_empty() {
        return Ok(Vec::new());
    }

    let sample_rate = clips[0].sample_rate;
    Ok(clips
        .into_iter()
        .filter(|clip| clip.sample_rate == sample_rate)
        .take(1)
        .collect())
}

pub(super) fn read_snd_clips(snd_path: &Path) -> Result<Vec<SndClip>, Box<dyn Error>> {
    let data = fs::read(snd_path)?;
    if data.len() < 6 {
        return Ok(Vec::new());
    }
    let num_clips = u16::from_le_bytes([data[0], data[1]]) as usize;
    let header_end = 4 + (num_clips + 1) * 4;
    if header_end > data.len() {
        return Ok(Vec::new());
    }

    let mut clips = Vec::new();
    for clip_idx in 0..num_clips {
        let off_pos = 4 + clip_idx * 4;
        let next_off_pos = off_pos + 4;
        let clip_start =
            header_end + u32::from_le_bytes(data[off_pos..off_pos + 4].try_into()?) as usize;
        let clip_end = header_end
            + u32::from_le_bytes(data[next_off_pos..next_off_pos + 4].try_into()?) as usize;
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
        clips.push(SndClip {
            pcm: data[clip_start + 6..clip_end].to_vec(),
            sample_rate,
        });
    }

    Ok(clips)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(tick: u16, text: &str) -> SubtitleCue {
        SubtitleCue {
            tick,
            text: text.to_string(),
        }
    }

    fn write_test_snd(path: &Path, clips: &[[u8; 2]]) {
        let mut data = Vec::new();
        data.extend_from_slice(&(clips.len() as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        for idx in 0..=clips.len() {
            data.extend_from_slice(&((idx * 8) as u32).to_le_bytes());
        }
        for samples in clips {
            data.extend_from_slice(&[1, 0, 0, 0, 156, 0, samples[0], samples[1]]);
        }
        fs::write(path, data).expect("write test snd");
    }

    #[test]
    fn chatter_events_fire_after_reveal_completes() {
        let events = subtitle_chatter_events(&[cue(10, "abc"), cue(20, "a\nb"), cue(30, "   ")]);
        assert_eq!(events.len(), 2);
        assert!((events[0].start_time - 1.25).abs() < f64::EPSILON);
        assert!((events[1].start_time - (2.0 + 2.0 / SUBTITLE_CHARS_PER_SEC)).abs() < f64::EPSILON);
    }

    #[test]
    fn sfx_track_uses_one_clip_per_subtitle_not_per_character() {
        let root = std::env::temp_dir().join(format!(
            "commander-blood-subtitle-sfx-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&root);
        let snd = root.join("tb.snd");
        let out = root.join("out.raw");
        write_test_snd(&snd, &[[200, 180]]);

        let rate = build_subtitle_sfx_track(&[cue(0, "abcd")], 1.0, &snd, &out)
            .expect("build sfx")
            .expect("sfx rate");
        let track = fs::read(&out).expect("read sfx");
        let _ = fs::remove_dir_all(&root);

        assert_eq!(rate, 10_000);
        let non_silence: Vec<usize> = track
            .iter()
            .enumerate()
            .filter_map(|(idx, sample)| (*sample != 128).then_some(idx))
            .collect();
        let start = (4.0 / SUBTITLE_CHARS_PER_SEC * rate as f64).round() as usize;
        assert_eq!(non_silence, vec![start, start + 1]);
    }

    #[test]
    fn sfx_track_reuses_tb_clip_zero_for_each_subtitle() {
        let root = std::env::temp_dir().join(format!(
            "commander-blood-subtitle-sfx-clip-zero-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&root);
        let snd = root.join("tb.snd");
        let out = root.join("out.raw");
        write_test_snd(&snd, &[[200, 180], [210, 190]]);

        let rate = build_subtitle_sfx_track(&[cue(0, "a"), cue(10, "a")], 2.0, &snd, &out)
            .expect("build sfx")
            .expect("sfx rate");
        let track = fs::read(&out).expect("read sfx");
        let _ = fs::remove_dir_all(&root);

        let first = (1.0 / SUBTITLE_CHARS_PER_SEC * rate as f64).round() as usize;
        let second = ((1.0 + 1.0 / SUBTITLE_CHARS_PER_SEC) * rate as f64).round() as usize;
        assert_eq!(&track[first..first + 2], &[200, 180]);
        assert_eq!(&track[second..second + 2], &[200, 180]);
    }
}
