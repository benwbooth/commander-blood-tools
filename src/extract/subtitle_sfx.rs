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
    let mut sfx_idx = 0usize;
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
                        let clip = &clips[sfx_idx % clips.len()];
                        sfx_idx += 1;
                        for (idx, &sample) in clip.pcm.iter().enumerate() {
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

    if snd_path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("tb.snd"))
        && clips.len() >= 17
    {
        let sample_rate = clips[7].sample_rate;
        return Ok(clips
            .into_iter()
            .enumerate()
            .filter_map(|(idx, clip)| {
                (idx >= 7 && idx <= 16 && clip.sample_rate == sample_rate).then_some(clip)
            })
            .collect());
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
