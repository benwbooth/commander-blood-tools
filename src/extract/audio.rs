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
    let bank = SndBank::read(snd_path)?;

    let mut converted = 0u32;
    for clip in bank.clips() {
        if clip.pcm.is_empty() {
            continue;
        }

        let clip_name = format!("{base_name} - {:03}", clip.original_index);

        let flac_out = flac_dir.join(format!("{clip_name}.flac"));
        let flac_ok = run_raw_pcm_to_ffmpeg(&clip.pcm, clip.sample_rate, &flac_out, &[]);

        let m4a_out = m4a_dir.join(format!("{clip_name}.m4a"));
        let m4a_ok = run_raw_pcm_to_ffmpeg(
            &clip.pcm,
            clip.sample_rate,
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
