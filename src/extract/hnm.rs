use super::*;
pub(super) use commander_blood_tools::hnm::HnmFile;

pub(super) fn character_foreground_bounds(hnm: &HnmFile) -> (usize, usize) {
    let mut max_w = 0usize;
    let mut max_h = 0usize;
    let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
    let mut pal = hnm.palette;

    for idx in 0..hnm.frame_count() {
        let (w, h, _) = hnm.decode_frame(idx, &mut fb, &mut pal);
        if w == 0 || h == 0 {
            continue;
        }
        // Frame 0 is the full character KEYFRAME (e.g. boba.hnm: 320x200);
        // later frames are smaller talk UPDATES (e.g. 320x130) that only animate
        // the top. The keyframe defines the character's full vertical extent, so
        // include it — otherwise the bounds come from the 320x130 updates and the
        // keyframe's lower portion gets wrongly cleared (character "half cut off").
        if idx == 0 || w < VIEWPORT_W || h < VIEWPORT_H {
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

pub(super) fn clear_outside_character_bounds(fb: &mut [u8], clip_w: usize, clip_h: usize) {
    if clip_w >= VIEWPORT_W && clip_h >= VIEWPORT_H {
        return;
    }

    let clear_w = clip_w.min(VIEWPORT_W);
    if clear_w < VIEWPORT_W {
        fill_rect_indexed_clipped(
            fb,
            0,
            clear_w as isize,
            0,
            (VIEWPORT_W - clear_w) as isize,
            VIEWPORT_H as isize,
            (0, VIEWPORT_W, 0, VIEWPORT_H),
        );
    }
    let clear_h = clip_h.min(VIEWPORT_H);
    if clear_h < VIEWPORT_H {
        fill_rect_indexed_clipped(
            fb,
            0,
            0,
            clear_h as isize,
            clear_w as isize,
            (VIEWPORT_H - clear_h) as isize,
            (0, VIEWPORT_W, 0, VIEWPORT_H),
        );
    }
}

pub(super) struct DecodedCharacterFrame {
    pub(super) fb: Vec<u8>,
    pub(super) palette: [[u8; 3]; 256],
}

pub(super) fn decode_character_animation(hnm: &HnmFile) -> Vec<DecodedCharacterFrame> {
    let frame_count = hnm.frame_count();
    let mut frames = Vec::with_capacity(frame_count);
    let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
    let mut pal = hnm.palette;
    let (clip_w, clip_h) = character_foreground_bounds(hnm);

    for idx in 0..frame_count {
        if idx == 0 {
            fb.fill(0);
            pal = hnm.palette;
        }

        let (_frame_w, _frame_h, _) = hnm.decode_character_frame(idx, &mut fb, &mut pal);
        // Clear outside the character's FULL extent (from the keyframe), not the
        // per-frame update size — otherwise a 320x130 talk update would wipe the
        // keyframe's lower 70px every frame (character "half cut off").
        clear_outside_character_bounds(&mut fb, clip_w, clip_h);

        frames.push(DecodedCharacterFrame {
            fb: fb.clone(),
            palette: pal,
        });
    }

    frames
}

pub(super) fn composite_character_frame(
    rgb: &mut [u8],
    bg_rgb: &[u8],
    frame: &DecodedCharacterFrame,
    letterbox: bool,
) {
    if !letterbox {
        // Full-screen layout (e.g. a character close-up with no location, like
        // the ship/intro view): character over background, no scene band.
        for i in 0..(VIEWPORT_W * VIEWPORT_H) {
            let ci = i * 3;
            if frame.fb[i] != 0 {
                let c = frame.palette[frame.fb[i] as usize];
                rgb[ci] = c[0];
                rgb[ci + 1] = c[1];
                rgb[ci + 2] = c[2];
            } else {
                rgb[ci] = bg_rgb[ci];
                rgb[ci + 1] = bg_rgb[ci + 1];
                rgb[ci + 2] = bg_rgb[ci + 2];
            }
        }
        return;
    }
    // Letterbox layout (planet dialogue): the scene is composited in the band
    // SCENE_TOP..SCENE_BOTTOM (gs:0x5239..0x523B), with black bars / HUD outside.
    // The character talk HNM is drawn starting at the band top, over the location
    // background.
    for y in 0..VIEWPORT_H {
        for x in 0..VIEWPORT_W {
            let oi = (y * VIEWPORT_W + x) * 3;
            if y < SCENE_TOP || y >= SCENE_BOTTOM {
                rgb[oi] = 0;
                rgb[oi + 1] = 0;
                rgb[oi + 2] = 0;
                continue;
            }
            let cy = y - SCENE_TOP;
            let ci = cy * VIEWPORT_W + x;
            let char_px = frame.fb.get(ci).copied().unwrap_or(0);
            if char_px != 0 {
                let c = frame.palette[char_px as usize];
                rgb[oi] = c[0];
                rgb[oi + 1] = c[1];
                rgb[oi + 2] = c[2];
            } else {
                rgb[oi] = bg_rgb[oi];
                rgb[oi + 1] = bg_rgb[oi + 1];
                rgb[oi + 2] = bg_rgb[oi + 2];
            }
        }
    }
}

/// Composite a scene frame with NO character — just the background, with the
/// same letterbox treatment as `composite_character_frame`. Used for
/// subtitle-only (voiceless) dialogue lines, which show the scene background and
/// the subtitle with no talking-head HNM.
pub(super) fn composite_scene_background(rgb: &mut [u8], bg_rgb: &[u8], letterbox: bool) {
    for y in 0..VIEWPORT_H {
        for x in 0..VIEWPORT_W {
            let oi = (y * VIEWPORT_W + x) * 3;
            if letterbox && (y < SCENE_TOP || y >= SCENE_BOTTOM) {
                rgb[oi] = 0;
                rgb[oi + 1] = 0;
                rgb[oi + 2] = 0;
            } else {
                rgb[oi] = bg_rgb[oi];
                rgb[oi + 1] = bg_rgb[oi + 1];
                rgb[oi + 2] = bg_rgb[oi + 2];
            }
        }
    }
}

// ===========================================================================
// Standalone HNM to MP4 decoder (for all HNM files)
// ===========================================================================

pub(super) fn decode_hnm_to_mp4(
    hnm_path: &Path,
    mp4_path: &Path,
    music_path: Option<&Path>,
) -> Result<usize, Box<dyn Error>> {
    decode_hnm_scene_to_mp4(&[hnm_path.to_path_buf()], mp4_path, music_path, &[], None)
}

pub(super) fn decode_hnm_scene_to_mp4(
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

        let audio_filter = match (has_music, subtitle_sfx_rate.is_some()) {
            (true, true) => Some(
                "[1:a]volume=0.5[music];[2:a]volume=0.75[sfx];[music][sfx]amix=inputs=2:duration=shortest[aout]",
            ),
            (true, false) => Some("[1:a]volume=0.5[aout]"),
            (false, true) => Some("[1:a]volume=0.75[aout]"),
            (false, false) => None,
        };
        cmd.arg("-filter_complex")
            .arg(scaled_video_filter(audio_filter))
            .args(["-map", "[vout]"]);
        if audio_filter.is_some() {
            cmd.args(["-map", "[aout]"]);
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
                let time = global_frame as f64 / HNM_FPS as f64;
                if subtitles.is_empty() {
                    fb_to_rgb(&fb, &pal, &mut rgb);
                } else {
                    let mut subtitle_fb = fb.clone();
                    render_subtitles_indexed(&mut subtitle_fb, subtitles, time);
                    // The HNM palette leaves the reserved subtitle indices
                    // 0xFD/0xFE at [0,0,0]; set them so the reveal is visible.
                    let mut sub_pal = pal;
                    apply_reserved_subtitle_palette(&mut sub_pal);
                    fb_to_rgb(&subtitle_fb, &sub_pal, &mut rgb);
                }
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
