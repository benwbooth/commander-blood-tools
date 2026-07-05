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
