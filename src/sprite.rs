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
    pub indices: Vec<u8>,
}

/// Frame dispatch index from the bank header flags: `((flags & 4) | 0x83) >> 1 & 7`
/// selects Raw (0/2), RLE (1/3), or Scaled (4) frame decode for the whole bank.
fn bank_dispatch_index(flags: u16) -> u8 {
    ((((flags & 0x0004) | 0x0083) >> 1) & 0x07) as u8
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

/// Blit a decoded sprite frame into a `width`-stride indexed framebuffer, centred
/// at `(cx, cy)`, skipping transparent index 0.
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
    fn blit_centers_and_skips_transparent() {
        let frame = SpriteFrameImage {
            width: 2,
            height: 2,
            indices: vec![0, 5, 5, 0],
        };
        let mut fb = vec![0u8; 4 * 4];
        blit_sprite_frame_centered(&mut fb, 4, 4, &frame, 1, 1);
        // frame centred at (1,1): its (0,0)->(0,0), (1,0)->(1,0), (0,1)->(0,1), (1,1)->(1,1)
        assert_eq!(fb[0 * 4 + 1], 5); // (1,0)
        assert_eq!(fb[1 * 4 + 0], 5); // (0,1)
        assert_eq!(fb[0], 0); // transparent skipped
    }
}
