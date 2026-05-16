use super::decompress::{decompress_lz_171, decompress_rle_173};
use super::*;

// ===========================================================================
// HNM(1) file parser — shared between all decoders
// ===========================================================================

pub(super) struct HnmFile {
    pub(super) data: Vec<u8>,
    pub(super) header_size: usize,
    pub(super) palette: [[u8; 3]; 256],
    pub(super) offsets: Vec<u32>,
}

impl HnmFile {
    pub(super) fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
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

    pub(super) fn frame_count(&self) -> usize {
        if self.offsets.len() > 1 {
            self.offsets.len() - 1
        } else {
            self.offsets.len()
        }
    }

    /// Decode frame `idx` into the framebuffer. Returns (sub_width, sub_height, mode).
    /// Updates palette from any 'pl' chunks in this frame's superchunk.
    pub(super) fn decode_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, false)
    }

    pub(super) fn decode_character_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, true)
    }

    pub(super) fn decode_frame_impl(
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

pub(super) fn clear_outside_character_bounds(fb: &mut [u8], clip_w: usize, clip_h: usize) {
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

        let (frame_w, frame_h, _) = hnm.decode_character_frame(idx, &mut fb, &mut pal);
        let clear_w = if frame_w >= VIEWPORT_W && frame_h >= VIEWPORT_H {
            clip_w
        } else {
            frame_w.min(clip_w)
        };
        let clear_h = if frame_w >= VIEWPORT_W && frame_h >= VIEWPORT_H {
            clip_h
        } else {
            frame_h.min(clip_h)
        };
        clear_outside_character_bounds(&mut fb, clear_w, clear_h);

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
) {
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
