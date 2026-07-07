//! HNM(1) decompression (LZ block 171 + RLE block 173) — lib-shared.
use std::error::Error;

// ===========================================================================
// HNM(1) decompression — Block 171 (LZ) and Block 173 (RLE)
// ===========================================================================

pub fn decompress_lz_171(data: &[u8], offset: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let unpacked_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    // NOTE: unlike RLE block 173, LZ blocks carry no flags byte / x,y placement pair —
    // byte 4 of their 6-byte header is payload-derived data (verified on logo_bl /
    // inter_sh, whose byte-4 values vary arbitrarily while decoding correctly from +6).
    let mut pos = offset + 6;
    let mut out = Vec::with_capacity(unpacked_len);
    let mut bits_left = 0u32;
    let mut queue = 0u16;

    let get_bit = |pos: &mut usize, bits_left: &mut u32, queue: &mut u16| -> u8 {
        if *bits_left == 0 {
            *queue = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
            *pos += 2;
            *bits_left = 16;
        }
        let b = (*queue & 1) as u8;
        *queue >>= 1;
        *bits_left -= 1;
        b
    };

    while out.len() < unpacked_len {
        if get_bit(&mut pos, &mut bits_left, &mut queue) != 0 {
            out.push(data[pos]);
            pos += 1;
        } else {
            let (count, offset_val);
            if get_bit(&mut pos, &mut bits_left, &mut queue) != 0 {
                let val = u16::from_le_bytes([data[pos], data[pos + 1]]);
                pos += 2;
                let c = (val & 0x07) as usize;
                offset_val = ((val >> 3) as isize) - 8192;
                if c == 0 {
                    let c2 = data[pos] as usize;
                    pos += 1;
                    if c2 == 0 {
                        break;
                    }
                    count = c2;
                } else {
                    count = c;
                }
            } else {
                let b0 = get_bit(&mut pos, &mut bits_left, &mut queue);
                let b1 = get_bit(&mut pos, &mut bits_left, &mut queue);
                count = (b0 as usize) * 2 + (b1 as usize);
                offset_val = (data[pos] as isize) - 256;
                pos += 1;
            }

            let total = count + 2;
            let src = (out.len() as isize + offset_val) as usize;
            for i in 0..total {
                let b = out[src + i];
                out.push(b);
            }
        }
    }

    out.truncate(unpacked_len);
    Ok(out)
}

pub struct BitReaderHigh<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub bits_left: u32,
    pub queue: u16,
}

impl<'a> BitReaderHigh<'a> {
    pub fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            bits_left: 0,
            queue: 0,
        }
    }

    pub fn get_bit(&mut self) -> u8 {
        if self.bits_left == 0 {
            self.queue = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            self.bits_left = 16;
        }
        self.bits_left -= 1;
        ((self.queue >> self.bits_left) & 1) as u8
    }

    pub fn get_byte(&mut self) -> u8 {
        let b = self.data[self.pos];
        self.pos += 1;
        b
    }
}

pub fn decompress_rle_173(data: &[u8], offset: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let frame_size = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let codebook_size = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
    let flags = data[offset + 4];

    let color_base: u8 = if (flags & 0x40) != 0 { 128 } else { 0 };
    let long_runs = (flags & 0x80) != 0;

    let mut pos = offset + 6;
    if (flags & 0x04) == 0 {
        pos += 4;
    }

    // Decompress codebook
    let mut codebook = Vec::with_capacity(codebook_size);
    let mut lc_byte: Option<u8> = None;
    let mut lc_used_top = true;

    while codebook.len() < codebook_size && pos < data.len() {
        let tag = data[pos];
        pos += 1;

        if tag & 0x80 != 0 {
            let temp;
            if lc_byte.is_none() || lc_used_top {
                let b = data[pos];
                pos += 1;
                lc_byte = Some(b);
                temp = (b >> 4) & 0x0F;
                lc_used_top = false;
            } else {
                temp = lc_byte.unwrap() & 0x0F;
                lc_used_top = true;
            }

            let offset_val = (((tag & 0x7F) as usize) << 1) | ((temp & 1) as usize);
            let count = (((temp >> 1) & 0x07) as usize) + 2;
            let src = codebook.len().wrapping_sub(offset_val + 1);
            for i in 0..count {
                let idx = src.wrapping_add(i);
                codebook.push(if idx < codebook.len() {
                    codebook[idx]
                } else {
                    0
                });
            }
        } else {
            codebook.push(if tag != 0 {
                tag.wrapping_add(color_base)
            } else {
                0
            });
        }
    }
    codebook.truncate(codebook_size);

    // Decode RLE raster
    let mut br = BitReaderHigh::new(data, pos);
    let mut cb_pos = 0usize;
    let mut frame = Vec::with_capacity(frame_size);

    let mut rle_lc_byte: Option<u8> = None;
    let mut rle_lc_used_top = true;

    let get_rle_length =
        |br: &mut BitReaderHigh, lc: &mut Option<u8>, used_top: &mut bool| -> usize {
            if lc.is_none() || *used_top {
                let b = br.get_byte();
                *lc = Some(b);
                let length = ((b >> 4) & 0x0F) as usize;
                *used_top = false;
                if length == 0 {
                    return br.get_byte() as usize + 16;
                }
                length
            } else {
                let length = (lc.unwrap() & 0x0F) as usize;
                *used_top = true;
                if length == 0 {
                    return br.get_byte() as usize + 16;
                }
                length
            }
        };

    while frame.len() < frame_size {
        while br.get_bit() == 0 {
            if cb_pos < codebook.len() {
                frame.push(codebook[cb_pos]);
                cb_pos += 1;
            }
            if frame.len() >= frame_size {
                break;
            }
        }
        if frame.len() >= frame_size {
            break;
        }

        let pixel = if cb_pos < codebook.len() {
            let p = codebook[cb_pos];
            cb_pos += 1;
            p
        } else {
            0
        };

        let run = if long_runs {
            if br.get_bit() == 0 {
                get_rle_length(&mut br, &mut rle_lc_byte, &mut rle_lc_used_top) + 4
            } else if br.get_bit() == 0 {
                2
            } else if br.get_bit() == 0 {
                3
            } else {
                4
            }
        } else if br.get_bit() == 0 {
            2
        } else if br.get_bit() == 0 {
            3
        } else if br.get_bit() == 0 {
            4
        } else {
            get_rle_length(&mut br, &mut rle_lc_byte, &mut rle_lc_used_top) + 4
        };

        let actual = run.min(frame_size - frame.len());
        frame.extend(std::iter::repeat(pixel).take(actual));
    }

    frame.truncate(frame_size);
    Ok(frame)
}
