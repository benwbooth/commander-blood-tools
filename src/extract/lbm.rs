//! Decoder for the location landscape backgrounds: IFF PBM (`.LBM`) images.
//!
//! These are the static surface views the dialogue plays over (the DESCRIPT
//! Location `Background` commands), as opposed to the planet `FullHnm`. Per the
//! README the `.FD`/`.LBM` assets are IFF **PBM** (chunky 8-bit, one byte per
//! pixel), typically ByteRun1-compressed.

/// A decoded LBM image: dimensions, palette indices, and a 256-entry RGB palette.
pub(super) struct Lbm {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) pixels: Vec<u8>,
    pub(super) palette: [[u8; 3]; 256],
}

fn be_u16(d: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([d[i], d[i + 1]])
}
fn be_u32(d: &[u8], i: usize) -> usize {
    u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as usize
}

/// Decode an IFF PBM `.LBM`. Returns `None` if the data isn't a PBM FORM.
pub(super) fn decode_lbm(data: &[u8]) -> Option<Lbm> {
    if data.len() < 12 || &data[0..4] != b"FORM" || &data[8..12] != b"PBM " {
        return None;
    }
    let mut width = 0usize;
    let mut height = 0usize;
    let mut compression = 0u8;
    let mut palette = [[0u8; 3]; 256];
    let mut body: Option<(usize, usize)> = None; // (start, len)

    let mut pos = 12;
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let len = be_u32(data, pos + 4);
        let chunk_start = pos + 8;
        let chunk_end = (chunk_start + len).min(data.len());
        match id {
            b"BMHD" if len >= 20 => {
                width = be_u16(data, chunk_start) as usize;
                height = be_u16(data, chunk_start + 2) as usize;
                compression = data[chunk_start + 10];
            }
            b"CMAP" => {
                let n = (len / 3).min(256);
                for i in 0..n {
                    let p = chunk_start + i * 3;
                    palette[i] = [data[p], data[p + 1], data[p + 2]];
                }
            }
            b"BODY" => body = Some((chunk_start, chunk_end - chunk_start)),
            _ => {}
        }
        // chunks are padded to an even byte boundary
        pos = chunk_end + (len & 1);
    }

    let (bstart, blen) = body?;
    if width == 0 || height == 0 {
        return None;
    }
    let body = &data[bstart..bstart + blen];
    // PBM rows are width bytes, padded to an even byte count per row.
    let row_bytes = width + (width & 1);
    let mut pixels = vec![0u8; row_bytes * height];

    if compression == 1 {
        // ByteRun1 (PackBits), decoded per row.
        let mut src = 0usize;
        let mut dst = 0usize;
        let total = row_bytes * height;
        while src < body.len() && dst < total {
            let n = body[src] as i8;
            src += 1;
            if n >= 0 {
                let count = n as usize + 1;
                for _ in 0..count {
                    if src < body.len() && dst < total {
                        pixels[dst] = body[src];
                        src += 1;
                        dst += 1;
                    }
                }
            } else if n != -128 {
                let count = (1 - n as isize) as usize;
                if src < body.len() {
                    let v = body[src];
                    src += 1;
                    for _ in 0..count {
                        if dst < total {
                            pixels[dst] = v;
                            dst += 1;
                        }
                    }
                }
            }
        }
    } else {
        let n = body.len().min(pixels.len());
        pixels[..n].copy_from_slice(&body[..n]);
    }

    // Trim row padding so callers index by `width`.
    if row_bytes != width {
        let mut packed = vec![0u8; width * height];
        for y in 0..height {
            let s = y * row_bytes;
            let d = y * width;
            packed[d..d + width].copy_from_slice(&pixels[s..s + width]);
        }
        pixels = packed;
    }

    Some(Lbm { width, height, pixels, palette })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a real extracted landscape LBM if present; check it has sane
    /// dimensions, a populated palette, and non-uniform pixels.
    #[test]
    fn decodes_real_landscape_lbm_if_present() {
        let candidates = [
            "export_check/_tmp_dat/fd/petrol1f.lbm",
            "export_new/_tmp_dat/fd/petrol1f.lbm",
            "../export_check/_tmp_dat/fd/petrol1f.lbm",
        ];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skip: no extracted LBM available");
            return;
        };
        let img = decode_lbm(&data).expect("should decode PBM LBM");
        eprintln!("LBM {}x{}", img.width, img.height);
        assert!(img.width >= 320 && img.height >= 200, "fullscreen-ish");
        assert_eq!(img.pixels.len(), img.width * img.height);
        let nonzero_pal = img.palette.iter().filter(|c| **c != [0, 0, 0]).count();
        assert!(nonzero_pal > 16, "palette populated: {nonzero_pal}");
        let distinct = img.pixels.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(distinct > 8, "image not uniform: {distinct} distinct indices");
    }
}
