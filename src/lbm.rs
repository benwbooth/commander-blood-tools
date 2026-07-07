//! Minimal IFF `PBM ` (DPaint chunky) decoder for the game's `fd/` location backgrounds.
//!
//! Commander Blood's location art is `FORM....PBM ` — chunky 8-bit, 320×200, a 768-byte
//! `CMAP` palette, and a `BODY` compressed with ByteRun1 (compression=1). This is the
//! lib-accessible decoder the engine uses to show a world's room background (the private
//! `extract` LBM tooling isn't reachable from the lib).

/// A decoded LBM: indexed pixels + a 256-entry RGB palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LbmImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    pub palette: [[u8; 3]; 256],
}

fn be_u32(d: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_be_bytes([*d.get(i)?, *d.get(i + 1)?, *d.get(i + 2)?, *d.get(i + 3)?]) as usize)
}

/// Decode an IFF `PBM ` (chunky) image. Returns `None` if it isn't a PBM or is malformed.
pub fn decode_pbm(data: &[u8]) -> Option<LbmImage> {
    decode_iff(data, false)
}

/// Decode an IFF `ILBM` (planar) or `PBM ` (chunky) image — dispatches on the FORM type.
pub fn decode_lbm(data: &[u8]) -> Option<LbmImage> {
    match data.get(8..12)? {
        b"PBM " => decode_iff(data, false),
        b"ILBM" => decode_iff(data, true),
        _ => None,
    }
}

fn decode_iff(data: &[u8], planar: bool) -> Option<LbmImage> {
    if data.get(0..4)? != b"FORM" {
        return None;
    }
    let form = data.get(8..12)?;
    if (planar && form != b"ILBM") || (!planar && form != b"PBM ") {
        return None;
    }
    let mut width = 0usize;
    let mut height = 0usize;
    let mut compression = 0u8;
    let mut n_planes = 8u8;
    let mut palette = [[0u8; 3]; 256];
    let mut body: Option<(usize, usize)> = None;

    let mut o = 12usize;
    while o + 8 <= data.len() {
        let cid = data.get(o..o + 4)?;
        let sz = be_u32(data, o + 4)?;
        let start = o + 8;
        let end = (start + sz).min(data.len());
        match cid {
            b"BMHD" if sz >= 11 => {
                // BMHD: w(2) h(2) x(2) y(2) nPlanes(1) masking(1) compression(1) ...
                width = u16::from_be_bytes([data[start], data[start + 1]]) as usize;
                height = u16::from_be_bytes([data[start + 2], data[start + 3]]) as usize;
                n_planes = data[start + 8];
                compression = data[start + 10];
            }
            b"CMAP" => {
                for (i, rgb) in data[start..end].chunks_exact(3).take(256).enumerate() {
                    palette[i] = [rgb[0], rgb[1], rgb[2]];
                }
            }
            b"BODY" => {
                body = Some((start, end - start));
                break;
            }
            _ => {}
        }
        // chunks are word-aligned (pad to even).
        o = start + sz + (sz & 1);
    }

    let (bstart, blen) = body?;
    if width == 0 || height == 0 {
        return None;
    }
    let src = &data[bstart..bstart + blen];
    let pixels = if planar {
        decode_planar_body(src, width, height, n_planes, compression)
    } else {
        let want = width * height;
        if compression == 1 {
            decode_byterun1(src, want)
        } else {
            let mut p = src.to_vec();
            p.resize(want, 0);
            p
        }
    };
    Some(LbmImage {
        width,
        height,
        pixels,
        palette,
    })
}

/// Decode an ILBM planar BODY into chunky indexed pixels. Each row stores `n_planes`
/// bitplanes of `row_bytes = ((width+15)/16)*2` each (optionally ByteRun1-compressed);
/// bit `p` of a pixel comes from plane `p`.
fn decode_planar_body(src: &[u8], width: usize, height: usize, n_planes: u8, compression: u8) -> Vec<u8> {
    let row_bytes = ((width + 15) / 16) * 2;
    let mut pixels = vec![0u8; width * height];
    let mut i = 0usize; // position in src
    for y in 0..height {
        for p in 0..n_planes as usize {
            // Get this plane's scanline (row_bytes), decompressing if needed.
            let plane: Vec<u8> = if compression == 1 {
                let (row, consumed) = byterun1_row(&src[i.min(src.len())..], row_bytes);
                i += consumed;
                row
            } else {
                let row = src.get(i..i + row_bytes).map(|s| s.to_vec()).unwrap_or_default();
                i += row_bytes;
                row
            };
            // Scatter plane bits into the chunky pixels of this row.
            for x in 0..width {
                let byte = plane.get(x / 8).copied().unwrap_or(0);
                let bit = (byte >> (7 - (x % 8))) & 1;
                pixels[y * width + x] |= bit << p;
            }
        }
    }
    pixels
}

/// ByteRun1-decompress exactly `row_bytes` for one scanline, returning `(row, consumed)`
/// where `consumed` is how many source bytes were used.
fn byterun1_row(src: &[u8], row_bytes: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(row_bytes);
    let mut i = 0usize;
    while out.len() < row_bytes && i < src.len() {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            let count = n as usize + 1;
            for _ in 0..count {
                if out.len() < row_bytes && i < src.len() {
                    out.push(src[i]);
                    i += 1;
                }
            }
        } else if n != -128 {
            let count = (1 - n as i32) as usize;
            if i < src.len() {
                let b = src[i];
                i += 1;
                for _ in 0..count {
                    if out.len() < row_bytes {
                        out.push(b);
                    }
                }
            }
        }
    }
    out.resize(row_bytes, 0);
    (out, i)
}

/// ByteRun1 (PackBits) decompression to exactly `want` bytes.
fn decode_byterun1(src: &[u8], want: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(want);
    let mut i = 0usize;
    while i < src.len() && out.len() < want {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            // copy the next n+1 bytes literally
            let count = n as usize + 1;
            for _ in 0..count {
                if i < src.len() && out.len() < want {
                    out.push(src[i]);
                    i += 1;
                }
            }
        } else if n != -128 {
            // repeat the next byte (1 - n) times
            let count = (1 - n as i32) as usize;
            if i < src.len() {
                let b = src[i];
                i += 1;
                for _ in 0..count {
                    if out.len() < want {
                        out.push(b);
                    }
                }
            }
        }
        // n == -128 is a no-op
    }
    out.resize(want, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_real_location_background() {
        let path = ["output/_tmp_dat/fd/1venus1f.lbm", "../output/_tmp_dat/fd/1venus1f.lbm"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        let img = decode_pbm(&data).expect("decodes PBM");
        assert_eq!((img.width, img.height), (320, 200));
        assert_eq!(img.pixels.len(), 320 * 200);
        // A real image uses more than one palette index (not a blank fill).
        let distinct = img.pixels.iter().collect::<std::collections::BTreeSet<_>>().len();
        assert!(distinct > 8, "background uses a real palette range ({distinct} colours)");
    }

    #[test]
    fn decodes_the_planar_ilbm_title_art() {
        // BLOOD.LBM is a 640x480 planar ILBM (the title/box art) — decode_lbm dispatches
        // to the planar path. Skips if absent.
        let path = ["output/_tmp_iso/BLOOD.LBM", "../output/_tmp_iso/BLOOD.LBM"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        // The PBM-only path rejects it...
        assert!(decode_pbm(&data).is_none(), "title art is ILBM, not PBM");
        // ...the dispatching path decodes it.
        let img = decode_lbm(&data).expect("decodes ILBM");
        assert_eq!((img.width, img.height), (640, 480));
        assert_eq!(img.pixels.len(), 640 * 480);
        let distinct = img.pixels.iter().collect::<std::collections::BTreeSet<_>>().len();
        assert!(distinct > 16, "planar decode yields a real image ({distinct} colours)");
    }

    #[test]
    fn planar_body_scatters_bits_across_planes() {
        // 8x1, 2 planes, uncompressed. Plane0 = 0b10100000, plane1 = 0b11000000.
        // pixel0 = bit0(1)|bit1(1)<<1 = 3; pixel1 = 0|1<<1 = 2; pixel2 = 1|0 = 1.
        let src = [0b1010_0000u8, 0, 0b1100_0000u8, 0]; // 2 bytes/plane row (row_bytes=2)
        let px = decode_planar_body(&src, 8, 1, 2, 0);
        assert_eq!(px[0], 3);
        assert_eq!(px[1], 2);
        assert_eq!(px[2], 1);
        assert_eq!(px[3], 0);
    }

    #[test]
    fn byterun1_literal_and_replicate() {
        // literal run: control 2 (=n+1=3 bytes) then AA BB CC
        assert_eq!(decode_byterun1(&[0x02, 0xAA, 0xBB, 0xCC], 3), vec![0xAA, 0xBB, 0xCC]);
        // replicate: control 0xFE (-2 -> 1-(-2)=3 copies) of 0x77
        assert_eq!(decode_byterun1(&[0xFE, 0x77], 3), vec![0x77, 0x77, 0x77]);
    }

    #[test]
    fn rejects_non_pbm() {
        assert!(decode_pbm(b"not an iff file at all").is_none());
    }
}
