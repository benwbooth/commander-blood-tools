//! IFF `PBM ` and `ILBM` decoder for original Commander Blood artwork.
//!
//! Commander Blood's location art is `FORM....PBM ` — chunky 8-bit, 320×200, a 768-byte
//! `CMAP` palette, and a `BODY` compressed with ByteRun1. The title image uses planar
//! `ILBM`; both layouts decode to the same indexed representation.

/// Number of entries in an original LBM color map.
pub const PALETTE_ENTRY_COUNT: usize = 256;
/// Number of red, green, and blue components in each color-map entry.
pub const RGB_COMPONENT_COUNT: usize = 3;

const IFF_MAGIC_OFFSET: usize = 0;
const IFF_FORM_TYPE_OFFSET: usize = 8;
const IFF_CHUNK_STREAM_OFFSET: usize = 12;
const FOURCC_BYTE_COUNT: usize = 4;
const CHUNK_HEADER_BYTE_COUNT: usize = 8;
const CHUNK_SIZE_FIELD_OFFSET: usize = 4;
const WORD_ALIGNMENT_MASK: usize = 1;
const BITMAP_HEADER_MINIMUM_SIZE: usize = 11;
const BITMAP_WIDTH_OFFSET: usize = 0;
const BITMAP_HEIGHT_OFFSET: usize = 2;
const BITMAP_PLANE_COUNT_OFFSET: usize = 8;
const BITMAP_COMPRESSION_OFFSET: usize = 10;
const DEFAULT_BITPLANE_COUNT: u8 = 8;
const BYTERUN1_COMPRESSION: u8 = 1;
const BYTE_BIT_COUNT: usize = 8;
const WORD_BIT_COUNT: usize = 16;
const WORD_BYTE_COUNT: usize = 2;
const CONTROL_COUNT_BIAS: usize = 1;
const LITERAL_CONTROL_MINIMUM: i8 = 0;
const BYTERUN1_NO_OPERATION: i8 = -128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PixelLayout {
    Chunky,
    Planar,
}

/// A decoded LBM: indexed pixels + a 256-entry RGB palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LbmImage {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
    /// Row-major palette indices.
    pub pixels: Vec<u8>,
    /// Red, green, and blue palette entries indexed by [`Self::pixels`].
    pub palette: [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
}

fn read_big_endian_u16(data: &[u8], offset: usize) -> Option<usize> {
    let bytes: [u8; size_of::<u16>()] = data
        .get(offset..offset + size_of::<u16>())?
        .try_into()
        .ok()?;
    Some(usize::from(u16::from_be_bytes(bytes)))
}

fn read_big_endian_u32(data: &[u8], offset: usize) -> Option<usize> {
    let bytes: [u8; size_of::<u32>()] = data
        .get(offset..offset + size_of::<u32>())?
        .try_into()
        .ok()?;
    usize::try_from(u32::from_be_bytes(bytes)).ok()
}

/// Decode an IFF `PBM ` (chunky) image. Returns `None` if it isn't a PBM or is malformed.
pub fn decode_pbm(data: &[u8]) -> Option<LbmImage> {
    decode_iff(data, PixelLayout::Chunky)
}

/// Decode an IFF `ILBM` (planar) or `PBM ` (chunky) image — dispatches on the FORM type.
pub fn decode_lbm(data: &[u8]) -> Option<LbmImage> {
    match data.get(IFF_FORM_TYPE_OFFSET..IFF_FORM_TYPE_OFFSET + FOURCC_BYTE_COUNT)? {
        b"PBM " => decode_iff(data, PixelLayout::Chunky),
        b"ILBM" => decode_iff(data, PixelLayout::Planar),
        _ => None,
    }
}

fn decode_iff(data: &[u8], layout: PixelLayout) -> Option<LbmImage> {
    if data.get(IFF_MAGIC_OFFSET..IFF_MAGIC_OFFSET + FOURCC_BYTE_COUNT)? != b"FORM" {
        return None;
    }
    let form_type = data.get(IFF_FORM_TYPE_OFFSET..IFF_FORM_TYPE_OFFSET + FOURCC_BYTE_COUNT)?;
    let expected_form_type = match layout {
        PixelLayout::Chunky => b"PBM ",
        PixelLayout::Planar => b"ILBM",
    };
    if form_type != expected_form_type {
        return None;
    }
    let mut width = usize::MIN;
    let mut height = usize::MIN;
    let mut compression = u8::MIN;
    let mut bitplane_count = DEFAULT_BITPLANE_COUNT;
    let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    let mut body: Option<(usize, usize)> = None;

    let mut chunk_offset = IFF_CHUNK_STREAM_OFFSET;
    while chunk_offset + CHUNK_HEADER_BYTE_COUNT <= data.len() {
        let chunk_id = data.get(chunk_offset..chunk_offset + FOURCC_BYTE_COUNT)?;
        let chunk_size = read_big_endian_u32(data, chunk_offset + CHUNK_SIZE_FIELD_OFFSET)?;
        let chunk_data_start = chunk_offset + CHUNK_HEADER_BYTE_COUNT;
        let chunk_data_end = (chunk_data_start + chunk_size).min(data.len());
        match chunk_id {
            b"BMHD" if chunk_size >= BITMAP_HEADER_MINIMUM_SIZE => {
                width = read_big_endian_u16(data, chunk_data_start + BITMAP_WIDTH_OFFSET)?;
                height = read_big_endian_u16(data, chunk_data_start + BITMAP_HEIGHT_OFFSET)?;
                bitplane_count = data[chunk_data_start + BITMAP_PLANE_COUNT_OFFSET];
                compression = data[chunk_data_start + BITMAP_COMPRESSION_OFFSET];
            }
            b"CMAP" => {
                for (palette_index, rgb) in data[chunk_data_start..chunk_data_end]
                    .chunks_exact(RGB_COMPONENT_COUNT)
                    .take(PALETTE_ENTRY_COUNT)
                    .enumerate()
                {
                    palette[palette_index] = rgb.try_into().ok()?;
                }
            }
            b"BODY" => {
                body = Some((chunk_data_start, chunk_data_end - chunk_data_start));
                break;
            }
            _ => {}
        }
        chunk_offset = chunk_data_start + chunk_size + (chunk_size & WORD_ALIGNMENT_MASK);
    }

    let (body_start, body_length) = body?;
    if width == usize::MIN || height == usize::MIN {
        return None;
    }
    let source = &data[body_start..body_start + body_length];
    let expected_pixel_count = width * height;
    let pixels = match layout {
        PixelLayout::Planar => {
            decode_planar_body(source, width, height, bitplane_count, compression)
        }
        PixelLayout::Chunky if compression == BYTERUN1_COMPRESSION => {
            decode_byterun1(source, expected_pixel_count)
        }
        PixelLayout::Chunky => {
            let mut pixels = source.to_vec();
            pixels.resize(expected_pixel_count, u8::MIN);
            pixels
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
fn decode_planar_body(
    source: &[u8],
    width: usize,
    height: usize,
    bitplane_count: u8,
    compression: u8,
) -> Vec<u8> {
    let row_bytes = width.div_ceil(WORD_BIT_COUNT) * WORD_BYTE_COUNT;
    let mut pixels = vec![u8::MIN; width * height];
    let mut source_offset = usize::MIN;
    for row_index in usize::MIN..height {
        for plane_index in usize::MIN..usize::from(bitplane_count) {
            let plane = if compression == BYTERUN1_COMPRESSION {
                let (row, consumed) =
                    byterun1_row(&source[source_offset.min(source.len())..], row_bytes);
                source_offset += consumed;
                row
            } else {
                let row = source
                    .get(source_offset..source_offset + row_bytes)
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default();
                source_offset += row_bytes;
                row
            };
            for column_index in usize::MIN..width {
                let byte = plane
                    .get(column_index / BYTE_BIT_COUNT)
                    .copied()
                    .unwrap_or(u8::MIN);
                let bit_index =
                    BYTE_BIT_COUNT - CONTROL_COUNT_BIAS - (column_index % BYTE_BIT_COUNT);
                let bit = (byte >> bit_index) & CONTROL_COUNT_BIAS as u8;
                pixels[row_index * width + column_index] |= bit << plane_index;
            }
        }
    }
    pixels
}

/// ByteRun1-decompress exactly `row_bytes` for one scanline, returning `(row, consumed)`
/// where `consumed` is how many source bytes were used.
fn byterun1_row(source: &[u8], row_bytes: usize) -> (Vec<u8>, usize) {
    decode_byterun1_prefix(source, row_bytes)
}

/// ByteRun1 (PackBits) decompression to exactly `expected_length` bytes.
fn decode_byterun1(source: &[u8], expected_length: usize) -> Vec<u8> {
    decode_byterun1_prefix(source, expected_length).0
}

fn decode_byterun1_prefix(source: &[u8], expected_length: usize) -> (Vec<u8>, usize) {
    let mut output = Vec::with_capacity(expected_length);
    let mut source_offset = usize::MIN;
    while source_offset < source.len() && output.len() < expected_length {
        let control = source[source_offset] as i8;
        source_offset += CONTROL_COUNT_BIAS;
        if control >= LITERAL_CONTROL_MINIMUM {
            let count = control as usize + CONTROL_COUNT_BIAS;
            for _ in usize::MIN..count {
                if source_offset < source.len() && output.len() < expected_length {
                    output.push(source[source_offset]);
                    source_offset += CONTROL_COUNT_BIAS;
                }
            }
        } else if control != BYTERUN1_NO_OPERATION {
            let count = (CONTROL_COUNT_BIAS as i32 - i32::from(control)) as usize;
            if source_offset < source.len() {
                let repeated_byte = source[source_offset];
                source_offset += CONTROL_COUNT_BIAS;
                for _ in usize::MIN..count {
                    if output.len() < expected_length {
                        output.push(repeated_byte);
                    }
                }
            }
        }
    }
    output.resize(expected_length, u8::MIN);
    (output, source_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_real_location_background() {
        let path = [
            "output/_tmp_dat/fd/1venus1f.lbm",
            "../output/_tmp_dat/fd/1venus1f.lbm",
        ]
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        let img = decode_pbm(&data).expect("decodes PBM");
        assert_eq!((img.width, img.height), (320, 200));
        assert_eq!(img.pixels.len(), 320 * 200);
        // A real image uses more than one palette index (not a blank fill).
        let distinct = img
            .pixels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert!(
            distinct > 8,
            "background uses a real palette range ({distinct} colours)"
        );
    }

    #[test]
    fn decodes_the_planar_ilbm_title_art() {
        // BLOOD.LBM is a 640x480 planar ILBM (the title/box art) — decode_lbm dispatches
        // to the planar path. Skips if absent.
        let path = ["output/_tmp_iso/BLOOD.LBM", "../output/_tmp_iso/BLOOD.LBM"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        // The PBM-only path rejects it...
        assert!(decode_pbm(&data).is_none(), "title art is ILBM, not PBM");
        // ...the dispatching path decodes it.
        let img = decode_lbm(&data).expect("decodes ILBM");
        assert_eq!((img.width, img.height), (640, 480));
        assert_eq!(img.pixels.len(), 640 * 480);
        // BLOOD.LBM is a 4-plane (16-colour) image; a correct decode uses the full range.
        let distinct = img
            .pixels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert!(
            distinct >= 12,
            "planar decode yields a real image ({distinct} colours)"
        );
        assert!(
            img.pixels.iter().all(|&p| p < 16),
            "4-plane image uses indices 0..16"
        );
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
    fn decodes_every_location_background_in_the_asset_set() {
        // The decoder must handle the whole fd/ location set, not just one file. Skips
        // if assets aren't present.
        let dir = ["output/_tmp_dat/fd", "../output/_tmp_dat/fd"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(dir) = dir else { return };
        let mut count = 0;
        for e in std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) != Some("lbm") {
                continue;
            }
            let data = std::fs::read(&p).unwrap();
            let img =
                decode_lbm(&data).unwrap_or_else(|| panic!("{} failed to decode", p.display()));
            assert_eq!((img.width, img.height), (320, 200), "{}", p.display());
            assert_eq!(img.pixels.len(), img.width * img.height);
            count += 1;
        }
        assert!(
            count >= 160,
            "decoded the full location set ({count} files)"
        );
    }

    #[test]
    fn byterun1_literal_and_replicate() {
        // literal run: control 2 (=n+1=3 bytes) then AA BB CC
        assert_eq!(
            decode_byterun1(&[0x02, 0xAA, 0xBB, 0xCC], 3),
            vec![0xAA, 0xBB, 0xCC]
        );
        // replicate: control 0xFE (-2 -> 1-(-2)=3 copies) of 0x77
        assert_eq!(decode_byterun1(&[0xFE, 0x77], 3), vec![0x77, 0x77, 0x77]);
    }

    #[test]
    fn rejects_non_pbm() {
        assert!(decode_pbm(b"not an iff file at all").is_none());
    }
}
