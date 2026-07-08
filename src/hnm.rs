//! HNM(1) video frame decoder core (`HnmFile`: open + decode_frame) — lib-shared
//! so the runnable engine can decode talk-HNM / scene backgrounds. The higher-level
//! compositing / MP4 helpers stay in the extract tooling and build on this.
use crate::decompress::{decompress_lz_171, decompress_rle_173};
use crate::{VIEWPORT_H, VIEWPORT_W};
use std::error::Error;
use std::fs;
use std::path::Path;

pub fn parse_palette_block(data: &[u8], mut pos: usize, palette: &mut [[u8; 3]; 256]) -> usize {
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

pub struct HnmFile {
    pub data: Vec<u8>,
    pub header_size: usize,
    pub palette: [[u8; 3]; 256],
    pub offsets: Vec<u32>,
}

impl HnmFile {
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
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

    pub fn frame_count(&self) -> usize {
        if self.offsets.len() > 1 {
            self.offsets.len() - 1
        } else {
            self.offsets.len()
        }
    }

    /// Decode frame `idx` into the framebuffer. Returns (sub_width, sub_height, mode).
    /// Updates palette from any 'pl' chunks in this frame's superchunk.
    pub fn decode_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, false)
    }

    pub fn decode_character_frame(
        &self,
        idx: usize,
        fb: &mut [u8],
        pal: &mut [[u8; 3]; 256],
    ) -> (usize, usize, u8) {
        self.decode_frame_impl(idx, fb, pal, true)
    }

    pub fn decode_frame_impl(
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

        // Destination placement (RLE blocks only): when the RLE block header's flags
        // bit 0x04 is clear, the 4 bytes after the 6-byte header carry the sub-frame's
        // x,y words; the game's blit routine (0xAB34..0xAB7E) reads them (`al=[si+4];
        // add si,6; test al,4; jne` then `mov dx,[si]; mov cx,[si+2]; add si,4`) and
        // writes at `di = y*320 + x`. LZ blocks have no flags/x,y (byte 4 is payload
        // data) and always draw at the band origin. Ignoring the pair drew every RLE
        // delta sub-frame at (0,0), smearing them across the screen (the intro
        // trail/speckle artifact).
        let (dst_x, dst_y) = if checksum == 0xAD
            && self.data[fds + 4] & 0x04 == 0
            && fds + 10 <= self.data.len()
        {
            (
                u16::from_le_bytes([self.data[fds + 6], self.data[fds + 7]]) as usize,
                u16::from_le_bytes([self.data[fds + 8], self.data[fds + 9]]) as usize,
            )
        } else {
            (0, 0)
        };

        if let Some(pixels) = pixels {
            let cw = fw.min(VIEWPORT_W.saturating_sub(dst_x));
            let ch = fh.min(VIEWPORT_H.saturating_sub(dst_y));
            match mode {
                0xFF => {
                    for y in 0..ch {
                        for x in 0..cw {
                            let si = y * fw + x;
                            if si < pixels.len() && (clear_zeroes || pixels[si] != 0) {
                                fb[(dst_y + y) * VIEWPORT_W + dst_x + x] = pixels[si];
                            }
                        }
                    }
                }
                _ => {
                    for y in 0..ch {
                        let so = y * fw;
                        let d = (dst_y + y) * VIEWPORT_W + dst_x;
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

impl HnmFile {
    pub fn raw(&self) -> &[u8] {
        &self.data
    }
    pub fn header_size(&self) -> usize {
        self.header_size
    }
    pub fn offset(&self, i: usize) -> u32 {
        self.offsets[i]
    }

    /// Parse frame `idx`'s video header (sub-width, sub-height, mode) without
    /// decompressing any pixel data — walks the typed chunks to the vhdr only.
    pub fn frame_dims(&self, idx: usize) -> Option<(usize, usize, u8)> {
        let abs_off = self.header_size + self.offsets.get(idx).copied()? as usize;
        if abs_off + 2 > self.data.len() {
            return None;
        }
        let sc_size = u16::from_le_bytes([self.data[abs_off], self.data[abs_off + 1]]) as usize;
        let mut cpos = abs_off + 2;
        let sc_end = abs_off + sc_size;
        while cpos < sc_end && cpos + 4 <= self.data.len() {
            let (t0, t1) = (self.data[cpos], self.data[cpos + 1]);
            let csz = u16::from_le_bytes([self.data[cpos + 2], self.data[cpos + 3]]) as usize;
            if (0x20..0x7f).contains(&t0) && (0x20..0x7f).contains(&t1) && csz >= 4 {
                cpos += csz;
            } else {
                break;
            }
        }
        if cpos + 4 > self.data.len() {
            return None;
        }
        let vhdr = u32::from_le_bytes([
            self.data[cpos],
            self.data[cpos + 1],
            self.data[cpos + 2],
            self.data[cpos + 3],
        ]);
        Some((
            (vhdr & 0x1FF) as usize,
            ((vhdr >> 16) & 0xFF) as usize,
            ((vhdr >> 24) & 0xFF) as u8,
        ))
    }

    /// The letterbox-band screen origin for this clip: the game blits every frame at
    /// `stream_y + gs:[0x1fa7]`, where the base is the letterbox band top (row 0x23)
    /// in band mode and 0 for full-screen clips. Band clips have a band-height
    /// (<= 0x82 = 130, the blit routine's height cap) keyframe; full-screen clips are
    /// 200 tall. Returns 0x23 for band clips, 0 for full-screen.
    pub fn band_y_origin(&self) -> usize {
        for idx in 0..self.frame_count().min(4) {
            if let Some((fw, fh, _)) = self.frame_dims(idx) {
                if fw > 1 && fh > 1 {
                    return if fh <= 0x82 { 0x23 } else { 0 };
                }
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every HNM(1) file in the game data must OPEN, report a positive frame count, and expose
    /// valid frame dimensions for frame 0 (width 1..=511, height 1..=255) - a broad parse-
    /// robustness check across the whole HNM asset set. Skips if the data isn't in this checkout.
    #[test]
    fn opens_and_parses_every_hnm_asset() {
        let roots = ["output/_tmp_dat", "../output/_tmp_dat"];
        let Some(root) = roots.iter().map(std::path::Path::new).find(|p| p.exists()) else {
            return;
        };
        // Walk the asset tree collecting .hnm files.
        let mut stack = vec![root.to_path_buf()];
        let mut files = Vec::new();
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("hnm") {
                    files.push(p);
                }
            }
        }
        let mut checked = 0;
        for p in &files {
            let hnm = HnmFile::open(p).unwrap_or_else(|e| panic!("{}: open failed: {e}", p.display()));
            assert!(hnm.frame_count() > 0, "{}: zero frames", p.display());
            let (w, h, _m) = hnm
                .frame_dims(0)
                .unwrap_or_else(|| panic!("{}: no frame-0 dims", p.display()));
            assert!((1..=511).contains(&w) && (1..=255).contains(&h), "{}: dims {w}x{h}", p.display());
            // Actually DECODE frame 0 (exercises the palette 'pl'-chunk parse + RLE body decode,
            // which frame_dims skips). A generously-sized fb (max mode-X) absorbs the sub-frame.
            let mut fb = vec![0u8; 512 * 256];
            let mut pal = [[0u8; 3]; 256];
            let (sw, sh, _m) = hnm.decode_frame(0, &mut fb, &mut pal);
            assert!(sw <= 512 && sh <= 256, "{}: decoded sub-frame {sw}x{sh} too large", p.display());
            checked += 1;
        }
        if checked > 0 {
            assert!(checked >= 600, "parsed the full HNM set ({checked} files)");
        }
    }
}
