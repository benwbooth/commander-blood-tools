use super::*;

// ===========================================================================
// SND voice bank decoder
// ===========================================================================

pub(super) fn decode_snd_clips(
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

pub(super) fn run_raw_pcm_to_ffmpeg(
    pcm: &[u8],
    sample_rate: u32,
    output: &Path,
    extra: &[&str],
) -> bool {
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
