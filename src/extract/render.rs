use super::*;

// ===========================================================================
// Palette and framebuffer helpers
// ===========================================================================

pub(super) fn parse_palette_block(
    data: &[u8],
    mut pos: usize,
    palette: &mut [[u8; 3]; 256],
) -> usize {
    while pos + 1 < data.len() {
        let start = data[pos];
        let count = data[pos + 1];
        pos += 2;

        if start == 0xFF && count == 0xFF {
            break;
        }

        let n = if count == 0 { 256 } else { count as usize };
        for i in 0..n {
            if pos + 2 >= data.len() {
                return pos;
            }
            let idx = start as usize + i;
            if idx < 256 {
                palette[idx] = [
                    (data[pos] << 2) | (data[pos] >> 4),
                    (data[pos + 1] << 2) | (data[pos + 1] >> 4),
                    (data[pos + 2] << 2) | (data[pos + 2] >> 4),
                ];
            }
            pos += 3;
        }
    }
    pos
}

pub(super) fn fb_to_rgb(fb: &[u8], palette: &[[u8; 3]; 256], rgb: &mut [u8]) {
    for (i, &px) in fb.iter().enumerate() {
        let c = palette[px as usize];
        rgb[i * 3] = c[0];
        rgb[i * 3 + 1] = c[1];
        rgb[i * 3 + 2] = c[2];
    }
}

pub(super) fn fill_rect_indexed_clipped(
    fb: &mut [u8],
    color: u8,
    x: isize,
    y: isize,
    width: isize,
    height: isize,
    clip: (usize, usize, usize, usize),
) {
    if width <= 0 || height <= 0 || fb.len() < VIEWPORT_W * VIEWPORT_H {
        return;
    }

    let (clip_left, clip_right, clip_top, clip_bottom) = clip;
    let x0 = x.max(clip_left as isize).max(0) as usize;
    let y0 = y.max(clip_top as isize).max(0) as usize;
    let x1 = (x + width)
        .min(clip_right as isize)
        .min(VIEWPORT_W as isize)
        .max(x0 as isize) as usize;
    let y1 = (y + height)
        .min(clip_bottom as isize)
        .min(VIEWPORT_H as isize)
        .max(y0 as isize) as usize;

    for row in y0..y1 {
        fb[row * VIEWPORT_W + x0..row * VIEWPORT_W + x1].fill(color);
    }
}

pub(super) fn fill_band_indexed(fb: &mut [u8], color: u8, clip_top: usize, clip_bottom: usize) {
    if clip_bottom <= clip_top {
        return;
    }
    fill_rect_indexed_clipped(
        fb,
        color,
        0,
        clip_top as isize,
        VIEWPORT_W as isize,
        (clip_bottom - clip_top) as isize,
        (0, VIEWPORT_W, clip_top, clip_bottom),
    );
}

pub(super) fn fill_scene_band_indexed(fb: &mut [u8], color: u8) {
    fill_band_indexed(fb, color, SCENE_TOP, SCENE_BOTTOM);
}

pub(super) fn copy_framebuffer_full_indexed(dst: &mut [u8], src: &[u8]) {
    let len = VIEWPORT_W * VIEWPORT_H;
    if dst.len() < len || src.len() < len {
        return;
    }
    dst[..len].copy_from_slice(&src[..len]);
}

pub(super) fn copy_vga_planar_to_linear_indexed(dst: &mut [u8], planes: &[u8]) {
    let len = VIEWPORT_W * VIEWPORT_H;
    let plane_len = len / 4;
    if dst.len() < len || planes.len() < len {
        return;
    }

    for plane in 0..4 {
        let src = &planes[plane * plane_len..(plane + 1) * plane_len];
        for (idx, value) in src.iter().copied().enumerate() {
            dst[idx * 4 + plane] = value;
        }
    }
}

pub(super) fn render_subtitles_indexed(fb: &mut [u8], cues: &[SubtitleCue], time: f64) {
    let Some((cue, visible_lines)) = active_subtitle_lines(cues, time) else {
        return;
    };

    let (clip_top, clip_bottom) = subtitle_clip_bounds(cue.active_line_id);
    for (line_idx, line) in visible_lines.iter().enumerate() {
        let y = SUBTITLE_Y + line_idx * GAME_FONT_LINE_HEIGHT;
        draw_game_text_indexed_clipped(fb, line, SUBTITLE_X, y, clip_top, clip_bottom);
    }
}

pub(super) fn render_subtitles_rgb(
    rgb: &mut [u8],
    palette: &[[u8; 3]; 256],
    cues: &[SubtitleCue],
    time: f64,
) {
    let Some((cue, visible_lines)) = active_subtitle_lines(cues, time) else {
        return;
    };

    let (clip_top, clip_bottom) = subtitle_clip_bounds(cue.active_line_id);
    for (line_idx, line) in visible_lines.iter().enumerate() {
        let y = SUBTITLE_Y + line_idx * GAME_FONT_LINE_HEIGHT;
        draw_game_text_rgb_clipped(rgb, palette, line, SUBTITLE_X, y, clip_top, clip_bottom);
    }
}

fn active_subtitle_lines(cues: &[SubtitleCue], time: f64) -> Option<(&SubtitleCue, Vec<String>)> {
    let Some((_, cue)) = cues.iter().enumerate().find(|(idx, cue)| {
        let start = cue.tick as f64 / 10.0;
        let end = cue_end_time(cues, *idx);
        time >= start && time < end
    }) else {
        return None;
    };

    let full_text = cue.text.trim();
    if full_text.is_empty() {
        return None;
    }

    let start = cue.tick as f64 / 10.0;
    let visible_chars = ((time - start).max(0.0) * SUBTITLE_CHARS_PER_SEC).ceil() as usize;
    if visible_chars == 0 {
        return None;
    }

    // The line breaks are already game-exact: assemble_dialogue inserts them at
    // the 35-char boundary like the game's 0xA6 handler (see re/REVERSE.md). Use
    // those breaks directly rather than re-wrapping by pixel width.
    let lines: Vec<String> = full_text
        .replace('\r', "\n")
        .lines()
        .map(|l| l.to_string())
        .collect();
    let visible_lines = visible_subtitle_lines(&lines, visible_chars);
    Some((cue, visible_lines))
}

pub(super) fn subtitle_clip_bounds(active_line_id: Option<u16>) -> (usize, usize) {
    match active_line_id {
        // BLOODPRG.EXE's per-frame dialogue updater sets render_string clipping
        // to gs:0x5239..0x523B for these active line ids, then restores it.
        Some(5 | 0x27) => (SCENE_TOP, SCENE_BOTTOM),
        _ => (0, VIEWPORT_H),
    }
}

pub(super) fn cue_end_time(cues: &[SubtitleCue], idx: usize) -> f64 {
    let start = cues[idx].tick as f64 / 10.0;
    cues.get(idx + 1)
        .map(|next| next.tick as f64 / 10.0)
        .filter(|end| *end > start + 0.25)
        .unwrap_or(start + 4.0)
}

pub(super) fn visible_subtitle_lines(lines: &[String], visible_chars: usize) -> Vec<String> {
    let mut remaining = visible_chars;
    let mut out = Vec::new();
    for line in lines {
        if remaining == 0 {
            break;
        }
        let line_len = line.chars().count();
        let take = remaining.min(line_len);
        out.push(line.chars().take(take).collect());
        remaining = remaining.saturating_sub(line_len);
    }
    out
}

fn draw_game_text_indexed_clipped(
    fb: &mut [u8],
    text: &str,
    x: usize,
    y: usize,
    clip_top: usize,
    clip_bottom: usize,
) {
    let visible_chars = text.chars().count();
    let mut cx = x;
    for (char_index, ch) in text.chars().enumerate() {
        if let Some(glyph) = game_font_glyph(ch).or_else(|| game_font_glyph('?')) {
            draw_game_glyph_indexed_clipped(
                fb,
                glyph.rows,
                cx,
                y,
                subtitle_glyph_color_index(char_index, visible_chars),
                clip_top,
                clip_bottom,
            );
        }
        cx += game_font_advance(ch);
        if cx >= VIEWPORT_W {
            break;
        }
    }
}

fn draw_game_text_rgb_clipped(
    rgb: &mut [u8],
    palette: &[[u8; 3]; 256],
    text: &str,
    x: usize,
    y: usize,
    clip_top: usize,
    clip_bottom: usize,
) {
    let visible_chars = text.chars().count();
    let mut cx = x;
    for (char_index, ch) in text.chars().enumerate() {
        if let Some(glyph) = game_font_glyph(ch).or_else(|| game_font_glyph('?')) {
            draw_game_glyph_rgb_clipped(
                rgb,
                glyph.rows,
                cx,
                y,
                palette[subtitle_glyph_color_index(char_index, visible_chars) as usize],
                clip_top,
                clip_bottom,
            );
        }
        cx += game_font_advance(ch);
        if cx >= VIEWPORT_W {
            break;
        }
    }
}

fn subtitle_glyph_color_index(char_index: usize, visible_chars: usize) -> u8 {
    if char_index + 1 == visible_chars {
        SUBTITLE_COLOR_REVEAL_EDGE
    } else {
        SUBTITLE_COLOR_REVEALED
    }
}

fn draw_game_glyph_indexed_clipped(
    fb: &mut [u8],
    rows: [u8; GAME_FONT_HEIGHT],
    x: usize,
    y: usize,
    color_index: u8,
    clip_top: usize,
    clip_bottom: usize,
) {
    for (gy, row) in rows.iter().copied().enumerate() {
        for gx in 0..GAME_FONT_WIDTH {
            if (row & (0x80 >> gx)) == 0 {
                continue;
            }
            let px = x + gx;
            let py = y + gy;
            if px < VIEWPORT_W && py >= clip_top && py < clip_bottom && py < VIEWPORT_H {
                fb[py * VIEWPORT_W + px] = color_index;
            }
        }
    }
}

fn draw_game_glyph_rgb_clipped(
    rgb: &mut [u8],
    rows: [u8; GAME_FONT_HEIGHT],
    x: usize,
    y: usize,
    color: [u8; 3],
    clip_top: usize,
    clip_bottom: usize,
) {
    for (gy, row) in rows.iter().copied().enumerate() {
        for gx in 0..GAME_FONT_WIDTH {
            if (row & (0x80 >> gx)) == 0 {
                continue;
            }
            let px = x + gx;
            let py = y + gy;
            if px < VIEWPORT_W && py >= clip_top && py < clip_bottom && py < VIEWPORT_H {
                let idx = (py * VIEWPORT_W + px) * 3;
                rgb[idx] = color[0];
                rgb[idx + 1] = color[1];
                rgb[idx + 2] = color[2];
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GameFontGlyph {
    pub(super) rows: [u8; GAME_FONT_HEIGHT],
    pub(super) advance: usize,
}

pub(super) fn game_font_glyph(ch: char) -> Option<GameFontGlyph> {
    let code = ch as usize;
    let idx = *GAME_FONT_CHAR_MAP.get(code)?;
    if idx == 0xff {
        return None;
    }
    let idx = idx as usize;
    Some(GameFontGlyph {
        rows: GAME_FONT_GLYPHS[idx],
        advance: GAME_FONT_WIDTHS[idx] as usize,
    })
}

pub(super) fn game_font_advance(ch: char) -> usize {
    if ch == ' ' {
        return GAME_FONT_SPACE_ADVANCE;
    }
    game_font_glyph(ch)
        .or_else(|| game_font_glyph('?'))
        .map(|glyph| glyph.advance)
        .unwrap_or(GAME_FONT_SPACE_ADVANCE)
}

// Dialogue scene layout, RE'd from BLOODPRG.EXE. The scene is composited in the
// framebuffer band `gs:0x5239..0x523B` with black bars / HUD outside. The
// letterbox mode (gs:0x2793 & 8) uses rows 0x23..0xA5 (35..165), a 130px band
// that the 320x130 talk-HNM frames fill exactly. See re/REVERSE.md.
pub(super) const SCENE_TOP: usize = 0x23; // 35
pub(super) const SCENE_BOTTOM: usize = 0xA5; // 165

// BLOODPRG.EXE's reveal renderer is called from 0x94EE with BX=[0x5E5C] and
// DX=[0x5E5E]. The initialized words at those DS offsets are 10 and 8; each
// CR-delimited subtitle line advances DX by 8.
pub(super) const SUBTITLE_X: usize = 10;
pub(super) const SUBTITLE_Y: usize = 8;
pub(super) const GAME_FONT_WIDTH: usize = 8;
pub(super) const GAME_FONT_HEIGHT: usize = 8;
pub(super) const GAME_FONT_LINE_HEIGHT: usize = 8;
pub(super) const SUBTITLE_COLOR_REVEALED: u8 = 0xFD;
pub(super) const SUBTITLE_COLOR_REVEAL_EDGE: u8 = 0xFE;
// Space advance: the game's glyph blitter (BLOODPRG.EXE render_string @0x31D7)
// advances a 0x20 space by 6 pixels (`add di, 6`), not a full glyph cell.
pub(super) const GAME_FONT_SPACE_ADVANCE: usize = 6;

// Extracted from BLOODPRG.EXE:
// - ASCII to glyph index map: file offset 0x14c22
// - glyph advances: file offsets 0x14cd2..0x14d27
// - 8-byte glyph rows: file offset 0x14d28
#[rustfmt::skip]
pub(super) const GAME_FONT_CHAR_MAP: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x1c, 0x24, 0xff, 0xff, 0xff, 0xff, 0x26, 0xff, 0xff, 0xff, 0x23, 0x25, 0x22, 0x1e, 0xff,
    0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x1f, 0x20, 0xff, 0xff, 0xff, 0x1a,
    0xff, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0xff, 0xff, 0xff, 0xff, 0x21,
    0xff, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35,
    0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[rustfmt::skip]
pub(super) const GAME_FONT_WIDTHS: [u8; 86] = [
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x03, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x09, 0x09, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x05, 0x07, 0x07, 0x07, 0x03, 0x03, 0x08, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x08, 0x03,
    0x06, 0x08, 0x03, 0x09, 0x08, 0x08, 0x08, 0x08, 0x06, 0x08, 0x06, 0x08, 0x09, 0x08, 0x08, 0x08,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x05, 0x05, 0x08, 0x08, 0x08, 0x08, 0x09, 0x04, 0x09, 0x09,
    0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
];

#[rustfmt::skip]
pub(super) const GAME_FONT_GLYPHS: [[u8; GAME_FONT_HEIGHT]; 86] = [
    [0x00, 0x7e, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x00],
    [0x00, 0xfc, 0x84, 0xfe, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x86, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0xfc, 0x80, 0x80, 0x86, 0x82, 0xfe, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0xfe, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x02, 0x02, 0x02, 0x02, 0x82, 0xfe, 0x00],
    [0x00, 0x84, 0x84, 0x84, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00],
    [0x00, 0xc2, 0xa2, 0x92, 0x8a, 0x86, 0x82, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0x82, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfe, 0x80, 0x80, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0x82, 0xfe, 0x02, 0x00],
    [0x00, 0xfe, 0x82, 0x82, 0xfc, 0x82, 0x82, 0x00],
    [0x00, 0xfe, 0x80, 0xfe, 0x02, 0x02, 0xfe, 0x00],
    [0x00, 0xfe, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x44, 0x28, 0x10, 0x00],
    [0x00, 0x81, 0x81, 0x81, 0x81, 0x99, 0x66, 0x00],
    [0x00, 0x82, 0x44, 0x38, 0x44, 0x82, 0x82, 0x00],
    [0x00, 0x82, 0x82, 0x82, 0x7e, 0x04, 0x78, 0x00],
    [0x00, 0xfe, 0x02, 0x7c, 0x80, 0x80, 0xfe, 0x00],
    [0x00, 0xfe, 0x82, 0x1e, 0x10, 0x00, 0x10, 0x00],
    [0x00, 0x10, 0x00, 0x10, 0xf0, 0x82, 0xfe, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x00, 0x80, 0x00],
    [0x00, 0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00],
    [0x00, 0x00, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x80, 0x80],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0xf8, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x20, 0x20, 0xf8, 0x20, 0x20, 0x00],
    [0x00, 0xa0, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x80],
    [0x00, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x00],
    [0x00, 0x04, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x00, 0xf0, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x04, 0x7c],
    [0x00, 0x80, 0xfc, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x80, 0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x10, 0x00, 0x10, 0x10, 0x10, 0x10, 0x90, 0xf0],
    [0x00, 0x80, 0x88, 0x88, 0xf8, 0x84, 0x84, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xec, 0x92, 0x92, 0x92, 0x92, 0x00],
    [0x00, 0x00, 0xf8, 0x84, 0x84, 0x84, 0x84, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0xfc, 0x80, 0x80],
    [0x00, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x04],
    [0x00, 0x00, 0xf0, 0x90, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0xfc, 0x80, 0xfc, 0x04, 0xfc, 0x00],
    [0x00, 0x80, 0xf0, 0x80, 0x80, 0x80, 0x80, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0x48, 0x30, 0x00],
    [0x00, 0x00, 0x82, 0x82, 0x82, 0x92, 0x6c, 0x00],
    [0x00, 0x00, 0x84, 0x48, 0x30, 0x48, 0x84, 0x00],
    [0x00, 0x00, 0x84, 0x84, 0x84, 0xfc, 0x10, 0x10],
    [0x00, 0x00, 0xfc, 0x04, 0x78, 0x80, 0xfc, 0x00],
    [0x48, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x7c, 0x04, 0xfc, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00],
    [0xa0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0xe0, 0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x48, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0xfc, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x48, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x78, 0x00, 0x84, 0x84, 0x84, 0x84, 0xfc, 0x00],
    [0x00, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xfc, 0x20],
    [0x00, 0x7c, 0x82, 0x82, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x40, 0xc0, 0x40, 0x40, 0x40, 0x40, 0x00],
    [0x00, 0x7c, 0x82, 0x02, 0x7c, 0x80, 0xfe, 0x00],
    [0x00, 0xfc, 0x02, 0x3c, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x80, 0x80, 0x80, 0x88, 0xfe, 0x08, 0x00],
    [0x00, 0xfe, 0x80, 0xfc, 0x02, 0x02, 0xfc, 0x00],
    [0x00, 0x7c, 0x80, 0xfc, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0xfe, 0x82, 0x04, 0x04, 0x04, 0x04, 0x00],
    [0x00, 0x7c, 0x82, 0x7c, 0x82, 0x82, 0x7c, 0x00],
    [0x00, 0x7c, 0x82, 0x82, 0x7e, 0x02, 0x7c, 0x00],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub(super) fn recovered_game_font_matches_executable_rows() {
        let m = game_font_glyph('M').expect("M glyph");
        assert_eq!(m.advance, 10);
        assert_eq!(m.rows, [0x00, 0xe7, 0x99, 0x81, 0x81, 0x81, 0x81, 0x00]);

        let e = game_font_glyph('e').expect("e glyph");
        assert_eq!(e.advance, 8);
        assert_eq!(e.rows, [0x00, 0x00, 0xfc, 0x84, 0xfc, 0x80, 0xfc, 0x00]);
    }

    #[test]
    fn active_line_clip_bounds_match_dialogue_updater_special_cases() {
        assert_eq!(subtitle_clip_bounds(Some(5)), (SCENE_TOP, SCENE_BOTTOM));
        assert_eq!(subtitle_clip_bounds(Some(0x27)), (SCENE_TOP, SCENE_BOTTOM));
        assert_eq!(subtitle_clip_bounds(Some(0x2b)), (0, VIEWPORT_H));
        assert_eq!(subtitle_clip_bounds(None), (0, VIEWPORT_H));
    }

    #[test]
    fn clipped_text_does_not_draw_outside_active_line_window() {
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];
        draw_game_text_indexed_clipped(&mut fb, "M", SUBTITLE_X, 0, SCENE_TOP, SCENE_BOTTOM);
        assert!(fb.iter().all(|sample| *sample == 0));

        draw_game_text_indexed_clipped(
            &mut fb,
            "M",
            SUBTITLE_X,
            SCENE_TOP,
            SCENE_TOP,
            SCENE_BOTTOM,
        );
        assert!(fb.iter().any(|sample| *sample != 0));
    }

    #[test]
    fn recovered_rect_fill_clips_to_viewport_and_active_band() {
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        fill_rect_indexed_clipped(
            &mut fb,
            7,
            318,
            (SCENE_TOP as isize) - 5,
            8,
            8,
            (0, VIEWPORT_W, SCENE_TOP, SCENE_TOP + 3),
        );

        let filled = fb.iter().filter(|sample| **sample == 7).count();
        assert_eq!(filled, 2 * 3);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 317], 0);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 318], 7);
        assert_eq!(fb[SCENE_TOP * VIEWPORT_W + 319], 7);
        assert_eq!(fb[(SCENE_TOP + 3) * VIEWPORT_W + 318], 0);
    }

    #[test]
    fn recovered_scene_band_fill_only_touches_scene_rows() {
        let mut fb = vec![1u8; VIEWPORT_W * VIEWPORT_H];

        fill_scene_band_indexed(&mut fb, 9);

        assert!(
            fb[..SCENE_TOP * VIEWPORT_W]
                .iter()
                .all(|sample| *sample == 1)
        );
        assert!(
            fb[SCENE_TOP * VIEWPORT_W..SCENE_BOTTOM * VIEWPORT_W]
                .iter()
                .all(|sample| *sample == 9)
        );
        assert!(
            fb[SCENE_BOTTOM * VIEWPORT_W..]
                .iter()
                .all(|sample| *sample == 1)
        );
    }

    #[test]
    fn recovered_framebuffer_copy_uses_one_full_viewport() {
        let len = VIEWPORT_W * VIEWPORT_H;
        let mut src = vec![0u8; len + 1];
        let mut dst = vec![3u8; len + 1];
        src[0] = 11;
        src[len - 1] = 22;
        src[len] = 33;

        copy_framebuffer_full_indexed(&mut dst, &src);

        assert_eq!(dst[0], 11);
        assert_eq!(dst[len - 1], 22);
        assert_eq!(dst[len], 3);
    }

    #[test]
    fn recovered_vga_planar_capture_interleaves_four_read_map_planes() {
        let len = VIEWPORT_W * VIEWPORT_H;
        let plane_len = len / 4;
        let mut planes = vec![0u8; len + 1];
        let mut dst = vec![0u8; len + 1];

        planes[0] = 10;
        planes[plane_len] = 20;
        planes[plane_len * 2] = 30;
        planes[plane_len * 3] = 40;
        planes[1] = 11;
        planes[plane_len + 1] = 21;
        planes[plane_len * 2 + 1] = 31;
        planes[plane_len * 3 + 1] = 41;
        planes[len] = 99;
        dst[len] = 7;

        copy_vga_planar_to_linear_indexed(&mut dst, &planes);

        assert_eq!(&dst[..8], &[10, 20, 30, 40, 11, 21, 31, 41]);
        assert_eq!(dst[len], 7);
    }

    #[test]
    fn subtitles_use_binary_reveal_palette_indices() {
        let cues = [SubtitleCue {
            tick: 0,
            text: "ME".to_string(),
            active_line_id: None,
        }];
        let mut fb = vec![0u8; VIEWPORT_W * VIEWPORT_H];

        render_subtitles_indexed(&mut fb, &cues, 2.0 / SUBTITLE_CHARS_PER_SEC);

        assert!(fb.iter().any(|sample| *sample == SUBTITLE_COLOR_REVEALED));
        assert!(
            fb.iter()
                .any(|sample| *sample == SUBTITLE_COLOR_REVEAL_EDGE)
        );
    }

    #[test]
    fn rgb_subtitles_map_binary_indices_through_palette() {
        let cues = [SubtitleCue {
            tick: 0,
            text: "ME".to_string(),
            active_line_id: None,
        }];
        let mut palette = [[0u8; 3]; 256];
        palette[SUBTITLE_COLOR_REVEALED as usize] = [1, 2, 3];
        palette[SUBTITLE_COLOR_REVEAL_EDGE as usize] = [4, 5, 6];
        let mut rgb = vec![0u8; VIEWPORT_W * VIEWPORT_H * 3];

        render_subtitles_rgb(&mut rgb, &palette, &cues, 2.0 / SUBTITLE_CHARS_PER_SEC);

        assert!(rgb.chunks_exact(3).any(|pixel| pixel == [1, 2, 3]));
        assert!(rgb.chunks_exact(3).any(|pixel| pixel == [4, 5, 6]));
    }
}
