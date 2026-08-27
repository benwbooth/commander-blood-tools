//! Sprite bank (`.spr`) decoding for the runnable engine — the frame-table parse
//! plus Raw/RLE frame decode to palette-index grids.
//!
//! This is the lib-accessible sprite decoder the engine uses to compose the
//! ship-nav sprite HUD (BCARTE grid + BORXX orb) and other sprite layers. It
//! mirrors the historically-verified decode in `src/extract/render.rs`
//! (`decode_sprite_bank_indices`), which stays in place for the extraction tooling
//! and its regression tests; the two share the same format and should be unified
//! into this module in a later pass.

/// One decoded `.spr` frame as a `width * height` grid of palette indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteFrameImage {
    pub width: usize,
    pub height: usize,
    /// The frame's authored draw offset (header +4/+6). Animation frames vary this to keep the
    /// sprite anchored as its size changes (e.g. the BORXX orb pulses with y-offset 0..49). The
    /// engine captures it so offset-aware draws stay aligned; `blit_sprite_frame_centered`
    /// ignores it (centres by size) for the HUD sprites that are authored symmetric.
    pub x_offset: u16,
    pub y_offset: u16,
    pub indices: Vec<u8>,
}

/// Frame dispatch index from the bank header flags: bit 2 selects RAW (0) vs RLE (3) frame
/// decode for the whole bank (see the body for the cross-bank verification).
fn bank_dispatch_index(flags: u16) -> u8 {
    // Bit 2 of the bank flags selects the frame encoding: clear -> RAW (uncompressed
    // width*height bytes), set -> RLE (row-compressed). Verified across all 44 sprite
    // banks: for flags&4==0 every frame body is exactly width*height (RAW, e.g. BAPPEL.SPR);
    // for flags&4==4 the body is shorter (RLE, e.g. BCARTE.SPR). The earlier formula only
    // ever yielded odd (RLE) codes, so it silently failed RAW banks (decoded to 0 frames).
    if flags & 0x0004 == 0 { 0 } else { 3 }
}

/// Decode every frame of a `.spr` sprite bank to palette-index grids. The bank
/// header is `[0]=flags, [2]=frame_count`, then a `frame_count`-entry table of
/// u32 frame offsets (relative to +4). Each frame header is `[0]=width/stride,
/// [2]=height, [4]=x offset, [6]=y offset`; Raw frames are `width*height` bytes
/// after the 8-byte header, RLE frames decode row-by-row. Returns `None` if the
/// bank header is unparseable; individual undecodable frames are skipped.
pub fn decode_sprite_bank_indices(data: &[u8]) -> Option<Vec<SpriteFrameImage>> {
    if data.len() < 4 {
        return None;
    }
    let flags = u16::from_le_bytes([data[0], data[1]]);
    let frame_count = u16::from_le_bytes([data[2], data[3]]) as usize;
    let table_end = 4usize.checked_add(frame_count.checked_mul(4)?)?;
    if table_end > data.len() {
        return None;
    }

    let mut frame_starts = Vec::with_capacity(frame_count);
    for idx in 0..frame_count {
        let pos = 4 + idx * 4;
        let packed =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let start = 4usize.checked_add(packed)?;
        if start < table_end || start + 8 > data.len() {
            return None;
        }
        frame_starts.push(start);
    }

    let dispatch = bank_dispatch_index(flags);
    let mut out = Vec::with_capacity(frame_count);
    for (i, &start) in frame_starts.iter().enumerate() {
        let end = frame_starts.get(i + 1).copied().unwrap_or(data.len());
        if end < start || end > data.len() {
            return None;
        }
        let frame = &data[start..end];
        if frame.len() < 8 {
            continue;
        }
        let width = u16::from_le_bytes([frame[0], frame[1]]) as usize;
        let height = u16::from_le_bytes([frame[2], frame[3]]) as usize;
        let x_offset = u16::from_le_bytes([frame[4], frame[5]]);
        let y_offset = u16::from_le_bytes([frame[6], frame[7]]);
        if width == 0 || height == 0 {
            continue;
        }
        let indices = match dispatch {
            0 | 2 => {
                let body = &frame[8..];
                if body.len() < width * height {
                    continue;
                }
                body[..width * height].to_vec()
            }
            1 | 3 => match decode_rle_frame(frame, height) {
                Some(px) if px.len() == width * height => px,
                _ => continue,
            },
            _ => continue,
        };
        out.push(SpriteFrameImage {
            width,
            height,
            x_offset,
            y_offset,
            indices,
        });
    }
    Some(out)
}

/// Decode one RLE frame (`[0]=stride, encoded from +8`) to `stride*height` pixels.
/// Control byte: high bit set → run of one value `len = (-control)+1`; else literal
/// run `len = control+1`.
fn decode_rle_frame(frame: &[u8], height: usize) -> Option<Vec<u8>> {
    let stride = u16::from_le_bytes([frame[0], frame[1]]) as usize;
    if stride == 0 {
        return None;
    }
    let encoded = &frame[8..];
    let len = stride.checked_mul(height)?;
    let mut pixels = Vec::with_capacity(len);
    let mut pos = 0usize;
    for _ in 0..height {
        let row_start = pixels.len();
        while pixels.len() - row_start < stride {
            let control = *encoded.get(pos)?;
            pos += 1;
            if control & 0x80 != 0 {
                let run = (0u8.wrapping_sub(control) as usize) + 1;
                if pixels.len() - row_start + run > stride {
                    return None;
                }
                let value = *encoded.get(pos)?;
                pos += 1;
                pixels.extend(std::iter::repeat(value).take(run));
            } else {
                let run = control as usize + 1;
                if pixels.len() - row_start + run > stride {
                    return None;
                }
                let end = pos.checked_add(run)?;
                pixels.extend_from_slice(encoded.get(pos..end)?);
                pos = end;
            }
        }
    }
    Some(pixels)
}

/// The game's TINT-RECT primitive — the inner loop of the blit reached as
/// `0x299:0x40E` (`0x3407..0x341D`):
///
/// ```text
///   0x3407  bx = si                    the 256-byte remap table
///   0x3413  al = es:[di]               read the pixel ALREADY on screen
///   0x3416  xlatb                      al = table[al]
///   0x3417  stosb                      write it back
///   0x340B  bp = 0x140 - dx            row stride 320 minus the rect width
/// ```
///
/// It is not a sprite blit: nothing is copied in. Each pixel of the rect is
/// replaced by `table[pixel]`, which is how the game draws translucent windows —
/// see [`crate::palette::build_palette_blend_remap_table`] for the table.
///
/// Clipping is against the CLIP-WINDOW globals, not the framebuffer — see
/// [`remap_rect_indexed_clipped`], which carries the ladder and its quirk.
pub fn remap_rect_indexed(
    fb: &mut [u8],
    fb_width: usize,
    fb_height: usize,
    table: &[u8; 256],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    // The UI paths that tint boxes run with the clip window open to the whole
    // screen (`DS:0x5235..0x523B` = 0,320,0,200). Passing that explicitly keeps
    // the window a stated input rather than an assumption baked into the loop.
    remap_rect_indexed_clipped(
        fb,
        fb_width,
        fb_height,
        table,
        x,
        y,
        width,
        height,
        ClipWindow::full_screen(fb_width, fb_height),
    );
}

/// The clip window the blit family reads from `DS:0x5235..0x523B`
/// (`re/labels.csv`: `render_yclip_lo`/`render_yclip_hi`; the point plot at
/// `0x9B04` shows the clean four-bound form).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipWindow {
    /// `DS:0x5235`
    pub left: i32,
    /// `DS:0x5237`
    pub right: i32,
    /// `DS:0x5239`
    pub top: i32,
    /// `DS:0x523B` — read by `0x9B04` and the string row draw at `0x3437`, but
    /// NOT by the remap ladder; see [`remap_rect_indexed_clipped`].
    pub bottom: i32,
}

impl ClipWindow {
    pub fn full_screen(fb_width: usize, fb_height: usize) -> Self {
        Self {
            left: 0,
            right: fb_width as i32,
            top: 0,
            bottom: fb_height as i32,
        }
    }
}

/// The remap primitive's clip ladder, `0x33A6..0x33F4`, transcribed:
///
/// ```text
///   0x33A6  or dx,dx / je,js         width  <= 0 -> return
///   0x33AC  or bp,bp / je,js         height <= 0 -> return
///   0x33B4  ax = x - [0x5235]        vs clip LEFT
///   0x33B8  jns                      in range: skip the adjust
///   0x33BA  add dx,ax                off the left: shrink width by the overshoot,
///   0x33C0  mov bx,[0x5235]          then pull x to the left bound
///   0x33C6  ax = x + width - [0x5237]  vs clip RIGHT, shrink width
///   0x33D6  ax = y - [0x5239]        vs clip TOP, shrink height, pull y down
///   0x33E8  ax = y + height - [0x5237]  <-- the RIGHT bound again
/// ```
///
/// That last line is not a transcription slip. The vertical extent is clipped
/// against `DS:0x5237`, the HORIZONTAL right bound, where every other clip ladder
/// in the binary uses `DS:0x523B` for the bottom (`0x9B04` compares y against
/// `0x5239`/`0x523B`; the string row draw at `0x3437` guards with
/// `cmp dx,gs:[0x523B]`). It is a bug in the original, and it survived because
/// `right` (320) exceeds `bottom` (200) on a full-screen window, so the vertical
/// clamp never fires for a rect that fits on screen — which is every rect the game
/// actually asks it to tint.
///
/// Reproducing it matters anyway: under a LETTERBOX window the game still honours
/// the top bound (`0x33E2` pulls y down to `[0x5239]`) while leaving the bottom
/// effectively open, so a tall rect tints past the letterbox floor. A port that
/// "fixed" this by clamping to `bottom` would draw a shorter box than the game.
///
/// The one deliberate divergence is memory safety: the game computes `y*320+x`
/// and writes without bounding against the buffer end, so the quirk lets it run
/// past the visible page. Writes here stop at the framebuffer.
#[allow(clippy::too_many_arguments)]
pub fn remap_rect_indexed_clipped(
    fb: &mut [u8],
    fb_width: usize,
    fb_height: usize,
    table: &[u8; 256],
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    clip: ClipWindow,
) {
    let (mut x, mut y, mut width, mut height) = (x, y, width, height);
    if width <= 0 || height <= 0 {
        return; // 0x33A6..0x33B0: zero/negative extent is rejected outright
    }

    let over = x - clip.left; // 0x33B4
    if over < 0 {
        width += over; // 0x33BA add dx,ax
        if width <= 0 {
            return; // 0x33BC js / 0x33BE je -> 0x341F
        }
        x = clip.left; // 0x33C0
    }
    let over = x + width - clip.right; // 0x33C4..0x33C8
    if over >= 0 {
        width -= over; // 0x33CE sub dx,ax
        if width <= 0 {
            return; // 0x33D0 js / 0x33D2 je
        }
    }
    let over = y - clip.top; // 0x33D4..0x33D6
    if over < 0 {
        height += over; // 0x33DC add bp,ax
        if height <= 0 {
            return; // 0x33DE js / 0x33E0 je
        }
        y = clip.top; // 0x33E2 mov cx,[0x5239]
    }
    // 0x33E6..0x33EA — `sub ax,[0x5237]`, the RIGHT bound. See the note above.
    let over = y + height - clip.right;
    if over >= 0 {
        height -= over; // 0x33F0 sub bp,ax
        if height <= 0 {
            return; // 0x33F2 js / 0x33F4 je
        }
    }

    // 0x33FA..0x3405: di = y*256 + y*64 + x. Bounded here, unbounded in the game.
    let x0 = x.max(0) as usize;
    let x1 = ((x + width).max(0) as usize).min(fb_width);
    let y0 = y.max(0) as usize;
    let y1 = ((y + height).max(0) as usize).min(fb_height);
    for row in y0..y1 {
        let base = row * fb_width;
        for px in fb[base + x0.min(x1)..base + x1].iter_mut() {
            *px = table[*px as usize]; // 0x3413 lodsb-free: read es:[di], xlatb, stosb
        }
    }
}

/// Blit a decoded sprite frame into a `width`-stride indexed framebuffer, centred
/// at `(cx, cy)`, skipping transparent index 0.
/// Blit a frame using its authored draw offset: the frame's top-left is placed at
/// `(base_x + x_offset, base_y + y_offset)`. This reproduces the game's per-frame anchoring -
/// e.g. the BORXX orb, whose `y_offset + height == 82` is constant, so it stays BOTTOM-anchored
/// as it grows (33..82 px tall) rather than growing symmetrically from its centre. Index 0 is
/// transparent; writes are clipped to the framebuffer.
pub fn blit_sprite_frame_at(
    fb: &mut [u8],
    fb_width: usize,
    fb_height: usize,
    frame: &SpriteFrameImage,
    base_x: i32,
    base_y: i32,
) {
    for y in 0..frame.height {
        for x in 0..frame.width {
            let idx = frame.indices[y * frame.width + x];
            if idx == 0 {
                continue;
            }
            let px = base_x + frame.x_offset as i32 + x as i32;
            let py = base_y + frame.y_offset as i32 + y as i32;
            if px >= 0 && (px as usize) < fb_width && py >= 0 && (py as usize) < fb_height {
                fb[py as usize * fb_width + px as usize] = idx;
            }
        }
    }
}

pub fn blit_sprite_frame_centered(
    fb: &mut [u8],
    fb_width: usize,
    fb_height: usize,
    frame: &SpriteFrameImage,
    cx: i32,
    cy: i32,
) {
    for y in 0..frame.height {
        for x in 0..frame.width {
            let idx = frame.indices[y * frame.width + x];
            if idx == 0 {
                continue;
            }
            let px = cx + x as i32 - frame.width as i32 / 2;
            let py = cy + y as i32 - frame.height as i32 / 2;
            if px >= 0 && (px as usize) < fb_width && py >= 0 && (py as usize) < fb_height {
                fb[py as usize * fb_width + px as usize] = idx;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_control_bytes_match_the_decoded_sprite_blit() {
        // Verify decode_rle_frame's control-byte semantics against the disassembled
        // sprite_blit_rle (0x2cd6): negative control -> run of (-control)+1 of the next
        // byte; non-negative control -> literal run of control+1. frame[0]=stride, the
        // encoded stream starts at +8.
        // stride 4, one row: 0xFE (run of 3) 0xAA ; 0x00 (literal 1) 0xBB -> AA AA AA BB
        let frame = [4u8, 0, 0, 0, 0, 0, 0, 0, 0xFE, 0xAA, 0x00, 0xBB];
        let px = decode_rle_frame(&frame, 1).expect("decodes");
        assert_eq!(px, vec![0xAA, 0xAA, 0xAA, 0xBB]);
        // A literal run of 3: control 0x02 -> copy next 3 bytes.
        let frame2 = [3u8, 0, 0, 0, 0, 0, 0, 0, 0x02, 0x11, 0x22, 0x33];
        assert_eq!(
            decode_rle_frame(&frame2, 1).unwrap(),
            vec![0x11, 0x22, 0x33]
        );
        // Index 0 in the decoded output is the transparent value the blit skips
        // (blit_sprite_frame_centered / the decoded di+=cx transparent-skip path).
        let frame3 = [4u8, 0, 0, 0, 0, 0, 0, 0, 0xFE, 0x00, 0x00, 0x77];
        assert_eq!(
            decode_rle_frame(&frame3, 1).unwrap(),
            vec![0x00, 0x00, 0x00, 0x77]
        );
    }

    #[test]
    fn decodes_borxx_orb_bank_matching_the_extract_decoder() {
        let candidates = [
            "output/_tmp_iso/BORXX.SPR",
            "output/_tmp_dat/spr/BORXX.SPR",
            "../output/_tmp_iso/BORXX.SPR",
        ];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BORXX.SPR not available");
            return;
        };
        let frames = decode_sprite_bank_indices(&data).expect("bank decodes");
        // Same invariants the extract-side test asserts (frame 0 = 40x33, 16 frames).
        assert_eq!(frames.len(), 16);
        assert_eq!((frames[0].width, frames[0].height), (40, 33));
        assert!(frames.iter().all(|f| f.indices.len() == f.width * f.height));
    }

    #[test]
    fn decodes_every_sprite_bank_to_valid_frames() {
        // Decode ALL .spr banks in the game data and assert each yields a non-empty frame set
        // whose every frame's index buffer is exactly width*height. Broadens the BORXX-only
        // check to the full sprite set. Skips if the game data isn't in this checkout.
        let dir = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(std::path::PathBuf::from)
            .find(|p| p.exists());
        let Some(dir) = dir else { return };
        let mut standard = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("spr"))
                != Some(true)
            {
                continue;
            }
            let data = std::fs::read(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            // Bank flags word: bit 2 = encoding (0 raw / 4 RLE); other values (e.g. KLAY.SPR
            // flags=6, a non-frame-table asset) are not standard sprite banks.
            let flags = u16::from_le_bytes([data[0], data[1]]);
            if flags != 0 && flags != 4 {
                assert!(
                    decode_sprite_bank_indices(&data).is_none(),
                    "{name}: non-standard flags {flags} should not decode as a sprite bank",
                );
                continue;
            }
            let header_count = u16::from_le_bytes([data[2], data[3]]) as usize;
            let frames = decode_sprite_bank_indices(&data)
                .unwrap_or_else(|| panic!("{name}: standard bank (flags {flags}) must decode"));
            assert!(!frames.is_empty(), "{name}: has frames");
            // No frame is silently dropped: the decoded count equals the header frame count.
            assert_eq!(
                frames.len(),
                header_count,
                "{name}: decoded all header frames"
            );
            for f in &frames {
                assert_eq!(
                    f.indices.len(),
                    f.width * f.height,
                    "{name}: frame index count"
                );
            }
            standard += 1;
        }
        if standard > 0 {
            assert_eq!(
                standard, 43,
                "all 43 standard sprite banks decode (41 RLE + 2 raw)"
            );
        }
    }

    #[test]
    fn offset_blit_bottom_anchors_the_orb_like_the_game() {
        // Two orb frames of different heights, both with yoff+h == 82 (the BORXX invariant).
        // Drawn with blit_sprite_frame_at at the same base, their BOTTOM edges must coincide -
        // the game's bottom-anchored growth, which centre-blitting would not reproduce.
        let small = SpriteFrameImage {
            width: 1,
            height: 4,
            x_offset: 0,
            y_offset: 6,
            indices: vec![5; 4],
        };
        let big = SpriteFrameImage {
            width: 1,
            height: 8,
            x_offset: 0,
            y_offset: 2,
            indices: vec![5; 8],
        };
        assert_eq!(
            small.y_offset as usize + small.height,
            big.y_offset as usize + big.height
        );
        let bottom = |f: &SpriteFrameImage| {
            let mut fb = vec![0u8; 1 * 16];
            blit_sprite_frame_at(&mut fb, 1, 16, f, 0, 0);
            (0..16).rev().find(|&y| fb[y] != 0).unwrap()
        };
        assert_eq!(
            bottom(&small),
            bottom(&big),
            "bottom edges coincide (bottom-anchored)"
        );
    }

    #[test]
    fn captures_frame_draw_offsets_from_the_header() {
        // The decoder must capture each frame's authored x/y draw offset (header +4/+6), not
        // discard it. BORXX.SPR (the nav orb) animates with a varying y-offset. Skips if absent.
        let Some(data) = ["output/_tmp_iso/BORXX.SPR", "../output/_tmp_iso/BORXX.SPR"]
            .iter()
            .find_map(|p| std::fs::read(p).ok())
        else {
            return;
        };
        let frames = decode_sprite_bank_indices(&data).expect("decodes");
        // Cross-check the captured offsets against the raw frame headers in the file.
        let fc = u16::from_le_bytes([data[2], data[3]]) as usize;
        for (i, f) in frames.iter().enumerate().take(fc) {
            let start = 4 + u32::from_le_bytes([
                data[4 + i * 4],
                data[5 + i * 4],
                data[6 + i * 4],
                data[7 + i * 4],
            ]) as usize;
            assert_eq!(
                f.x_offset,
                u16::from_le_bytes([data[start + 4], data[start + 5]])
            );
            assert_eq!(
                f.y_offset,
                u16::from_le_bytes([data[start + 6], data[start + 7]])
            );
        }
        // The orb frames genuinely vary their y-offset (animation anchoring).
        assert!(
            frames
                .iter()
                .map(|f| f.y_offset)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 1
        );
    }

    #[test]
    fn blit_centers_and_skips_transparent() {
        let frame = SpriteFrameImage {
            width: 2,
            height: 2,
            x_offset: 0,
            y_offset: 0,
            indices: vec![0, 5, 5, 0],
        };
        let mut fb = vec![0u8; 4 * 4];
        blit_sprite_frame_centered(&mut fb, 4, 4, &frame, 1, 1);
        // frame centred at (1,1): its (0,0)->(0,0), (1,0)->(1,0), (0,1)->(0,1), (1,1)->(1,1)
        assert_eq!(fb[0 * 4 + 1], 5); // (1,0)
        assert_eq!(fb[1 * 4 + 0], 5); // (0,1)
        assert_eq!(fb[0], 0); // transparent skipped
    }

    /// The remap ladder clips its VERTICAL extent against `DS:0x5237`, the
    /// horizontal right bound (`0x33E6..0x33EA`), where every other clip ladder in
    /// the binary uses `DS:0x523B`. Under a letterbox window the game therefore
    /// honours the TOP bound and leaves the bottom effectively open. This encodes
    /// that asymmetry: a "fixed" implementation clamping to `bottom` would stop at
    /// row 120 and fail the second assertion.
    #[test]
    fn remap_clips_vertically_against_the_right_bound_not_the_bottom() {
        const W: usize = 320;
        const H: usize = 200;
        let table = {
            let mut t = [0u8; 256];
            t[1] = 2; // any pixel of 1 becomes 2, so tinting is observable
            t
        };
        let letterbox = ClipWindow {
            left: 0,
            right: 320,
            top: 40,
            bottom: 120,
        };

        let mut fb = vec![1u8; W * H];
        remap_rect_indexed_clipped(&mut fb, W, H, &table, 0, 0, 8, 190, letterbox);

        // TOP is honoured: 0x33D4 shrinks the height and 0x33E2 pulls y to 40.
        assert_eq!(fb[39 * W], 1, "row above the clip top must be untouched");
        assert_eq!(fb[40 * W], 2, "row at the clip top must be tinted");

        // BOTTOM is not: the rect runs past row 120 because the ladder compared
        // against `right` (320), which y+height never reaches.
        assert_eq!(
            fb[121 * W],
            2,
            "the letterbox floor does NOT clip the remap"
        );
        assert_eq!(
            fb[189 * W],
            2,
            "the rect tints to its full requested height"
        );

        // The bound that DOES fire is `right`. y+height > 320 is clamped, so a rect
        // starting at the clip top can only extend to row 320-40 = 280 -> off-screen
        // here, which is why the bug is invisible at 320x200.
        let mut tall = vec![1u8; W * H];
        remap_rect_indexed_clipped(&mut tall, W, H, &table, 0, 0, 8, 400, letterbox);
        assert_eq!(tall[199 * W], 2, "clamped by `right`=320, not by `bottom`");

        // Horizontal clipping is ordinary: left and right both apply.
        let window = ClipWindow {
            left: 10,
            right: 20,
            top: 0,
            bottom: 200,
        };
        let mut side = vec![1u8; W * H];
        remap_rect_indexed_clipped(&mut side, W, H, &table, 0, 0, 320, 1, window);
        assert_eq!(side[9], 1, "left of the clip window is untouched");
        assert_eq!(side[10], 2, "the left bound is inclusive");
        assert_eq!(side[19], 2, "up to the right bound is tinted");
        assert_eq!(side[20], 1, "the right bound is exclusive");
    }
}
