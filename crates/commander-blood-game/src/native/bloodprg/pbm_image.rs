//! Native-compatible PBM presentation decoding over owned byte slices.

use std::error::Error;
use std::fmt;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::panorama::{
    BridgePanoramaError, PANORAMA_FRAME_PIXEL_COUNT, PanoramaDecodeMode,
    decode_bridge_panorama_pixels,
};

use super::IndexedGamePalette;

/// Number of leading colors updated when a scene or ship palette is retained.
pub const PBM_SCENE_PALETTE_COLOR_COUNT: usize = 192;
/// Native resource name selected by `back_buffer_init`.
pub const CHART_BACK_BUFFER_RESOURCE_PATH: &str = "chart.fd";
/// Native resource name selected by `backbuffer_clear_flags`.
pub const ORX_BACK_BUFFER_RESOURCE_PATH: &str = "orx.fd";

const PBM_MARKER: &[u8; 4] = b"PBM ";
const COLOR_MAP_MARKER: &[u8; 4] = b"CMAP";
const BODY_MARKER: &[u8; 4] = b"BODY";
const CHUNK_HEADER_SIZE: usize = 8;
const PALETTE_COMPONENT_SHIFT: u32 = 2;

/// Palette update requested while presenting a PBM image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PbmPaletteUpdate {
    /// Keep the current live palette unchanged.
    #[default]
    Preserve,
    /// Replace all 256 colors from the image color map.
    AllColors,
    /// Replace only the first 192 scene colors.
    SceneColors,
}

/// How decoded palette index zero affects the existing framebuffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PbmTransparency {
    /// Replace every destination pixel.
    #[default]
    Opaque,
    /// Preserve destination pixels where the decoded image is zero.
    TransparentZero,
}

/// Typed presentation controls for one PBM image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PbmDecodeOptions {
    /// Portion of the live palette to refresh.
    pub palette_update: PbmPaletteUpdate,
    /// Indexed-zero composition policy.
    pub transparency: PbmTransparency,
}

/// Observable result of a successful image decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PbmDecodeResult {
    /// Final palette index emitted by the ByteRun stream.
    pub last_palette_index: u8,
    /// Whether the native dirty-palette latch would be set.
    pub palette_changed: bool,
}

/// Marker sought by the recovered PBM scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PbmMarker {
    /// `PBM ` form marker.
    Form,
    /// `CMAP` color-map marker.
    ColorMap,
    /// `BODY` compressed-pixel marker.
    Body,
}

/// Invalid PBM source, destination, or ByteRun stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PbmDecodeError {
    /// A required marker was absent in forward authored order.
    MissingMarker(PbmMarker),
    /// A chunk marker lacks its complete eight-byte header.
    TruncatedChunkHeader(PbmMarker),
    /// The color map cannot provide all 256 RGB entries.
    TruncatedColorMap {
        /// Available component bytes after the `CMAP` header.
        available: usize,
    },
    /// The destination cannot contain one 320 by 200 logical image.
    FramebufferTooShort(usize),
    /// The shared native ByteRun decoder rejected the body stream.
    ByteRun(BridgePanoramaError),
}

impl fmt::Display for PbmDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid PBM presentation: {self:?}")
    }
}

impl Error for PbmDecodeError {}

impl From<BridgePanoramaError> for PbmDecodeError {
    fn from(error: BridgePanoramaError) -> Self {
        Self::ByteRun(error)
    }
}

/// Decode one recovered PBM presentation into typed palette and pixel arrays.
///
/// This translates `pbm_image_load_and_decode` at BLOODPRG file offset
/// `0x002BFD`. It preserves the loose ordered marker search, six-bit VGA color
/// conversion, 192-color limit, zero transparency, and the game's `0x80`
/// 129-pixel repeat. Resource loading and host errors remain outside this pure
/// decoder, while malformed input is transactional.
pub fn decode_pbm_image(
    source: &[u8],
    framebuffer: &mut [u8],
    live_palette: &mut IndexedGamePalette,
    options: PbmDecodeOptions,
) -> Result<PbmDecodeResult, PbmDecodeError> {
    if framebuffer.len() < PANORAMA_FRAME_PIXEL_COUNT {
        return Err(PbmDecodeError::FramebufferTooShort(framebuffer.len()));
    }

    let form = find_marker(source, usize::MIN, PBM_MARKER, PbmMarker::Form)?;
    let color_map = find_marker(source, form + 1, COLOR_MAP_MARKER, PbmMarker::ColorMap)?;
    let palette_start = chunk_payload_start(source, color_map, PbmMarker::ColorMap)?;
    let palette_byte_count = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
    let palette_end =
        palette_start
            .checked_add(palette_byte_count)
            .ok_or(PbmDecodeError::TruncatedColorMap {
                available: source.len().saturating_sub(palette_start),
            })?;
    let palette_source =
        source
            .get(palette_start..palette_end)
            .ok_or(PbmDecodeError::TruncatedColorMap {
                available: source.len().saturating_sub(palette_start),
            })?;
    let body = find_marker(source, palette_end, BODY_MARKER, PbmMarker::Body)?;
    let body_start = chunk_payload_start(source, body, PbmMarker::Body)?;

    let mut decoded = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT];
    decode_bridge_panorama_pixels(
        &source[body_start..],
        &mut decoded,
        PanoramaDecodeMode::Opaque,
    )?;

    let mut updated_palette = *live_palette;
    let updated_color_count = match options.palette_update {
        PbmPaletteUpdate::Preserve => usize::MIN,
        PbmPaletteUpdate::AllColors => PALETTE_ENTRY_COUNT,
        PbmPaletteUpdate::SceneColors => PBM_SCENE_PALETTE_COLOR_COUNT,
    };
    for (destination, source_color) in updated_palette
        .iter_mut()
        .zip(palette_source.chunks_exact(RGB_COMPONENT_COUNT))
        .take(updated_color_count)
    {
        for component in 0..RGB_COMPONENT_COUNT {
            destination[component] = source_color[component] >> PALETTE_COMPONENT_SHIFT;
        }
    }

    match options.transparency {
        PbmTransparency::Opaque => {
            framebuffer[..PANORAMA_FRAME_PIXEL_COUNT].copy_from_slice(&decoded)
        }
        PbmTransparency::TransparentZero => {
            for (destination, source) in framebuffer[..PANORAMA_FRAME_PIXEL_COUNT]
                .iter_mut()
                .zip(&decoded)
            {
                if *source != u8::MIN {
                    *destination = *source;
                }
            }
        }
    }
    *live_palette = updated_palette;

    Ok(PbmDecodeResult {
        last_palette_index: *decoded
            .last()
            .expect("complete PBM frame has a final palette index"),
        palette_changed: updated_color_count != usize::MIN,
    })
}

/// Decode `CHART.FD` into the flat indexed back buffer.
///
/// This translates `back_buffer_init` at BLOODPRG routine offset `0x0017D9`.
/// The selected resource, opaque pixel replacement, palette preservation, and
/// decode result remain. Direct conversion to a temporary Mode-X page is
/// omitted because wgpu consumes the same logical indexed pixels.
pub fn decode_chart_back_buffer(
    source: &[u8],
    framebuffer: &mut [u8],
    live_palette: &mut IndexedGamePalette,
) -> Result<PbmDecodeResult, PbmDecodeError> {
    decode_opaque_back_buffer(source, framebuffer, live_palette)
}

/// Decode `ORX.FD` into the flat indexed back buffer.
///
/// This translates `backbuffer_clear_flags` at BLOODPRG routine offset
/// `0x001817`. The selected resource, opaque pixel replacement, palette
/// preservation, and decode result remain. Direct conversion to a temporary
/// Mode-X page is omitted because wgpu consumes the flat buffer directly.
pub fn decode_orx_back_buffer(
    source: &[u8],
    framebuffer: &mut [u8],
    live_palette: &mut IndexedGamePalette,
) -> Result<PbmDecodeResult, PbmDecodeError> {
    decode_opaque_back_buffer(source, framebuffer, live_palette)
}

fn decode_opaque_back_buffer(
    source: &[u8],
    framebuffer: &mut [u8],
    live_palette: &mut IndexedGamePalette,
) -> Result<PbmDecodeResult, PbmDecodeError> {
    decode_pbm_image(
        source,
        framebuffer,
        live_palette,
        PbmDecodeOptions {
            palette_update: PbmPaletteUpdate::Preserve,
            transparency: PbmTransparency::Opaque,
        },
    )
}

fn find_marker(
    source: &[u8],
    start: usize,
    marker: &[u8; 4],
    marker_kind: PbmMarker,
) -> Result<usize, PbmDecodeError> {
    source
        .get(start..)
        .and_then(|tail| {
            tail.windows(marker.len())
                .position(|window| window == marker)
        })
        .map(|position| start + position)
        .ok_or(PbmDecodeError::MissingMarker(marker_kind))
}

fn chunk_payload_start(
    source: &[u8],
    marker: usize,
    marker_kind: PbmMarker,
) -> Result<usize, PbmDecodeError> {
    marker
        .checked_add(CHUNK_HEADER_SIZE)
        .filter(|start| *start <= source.len())
        .ok_or(PbmDecodeError::TruncatedChunkHeader(marker_kind))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const EXPECTED_VECTOR_COUNT: usize = 12;
    const EXACT_FORWARD_SUCCESS_COUNT: usize = 5;
    const MALFORMED_FORWARD_COUNT: usize = 4;
    const HOST_OPEN_FAILURE_COUNT: usize = 1;
    const FLAT_ALIAS_REPLACEMENT_COUNT: usize = 1;
    const DIRECTION_STATE_REPLACEMENT_COUNT: usize = 1;
    const NORMAL_STREAM_LAST_PALETTE_INDEX: u8 = 167;
    const FULL_PALETTE_BYTE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
    const BLOODPRG_DATA_FILE_OFFSET: usize = 0x0000_D420;
    const ORX_PATH_DATA_OFFSET: usize = 227;
    const CHART_PATH_DATA_OFFSET: usize = 234;

    #[derive(Deserialize)]
    struct PbmOracle {
        name: String,
        payload_hex: String,
        palette_before_hex: String,
        framebuffer_seed_byte: u8,
        succeeded: bool,
        open_success: Option<bool>,
        direction_flag: bool,
        in_place: bool,
        palette_bytes_written: usize,
        transparent_zero: bool,
        return_eax: u32,
        output_sha256: String,
        palette_sha256: String,
    }

    fn decode_hex(encoded: &str) -> Vec<u8> {
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|digits| {
                let digits = std::str::from_utf8(digits).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect()
    }

    fn palette_from_hex(encoded: &str) -> IndexedGamePalette {
        let bytes = decode_hex(encoded);
        assert_eq!(bytes.len(), FULL_PALETTE_BYTE_COUNT);
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        for (destination, source) in palette
            .iter_mut()
            .zip(bytes.chunks_exact(RGB_COMPONENT_COUNT))
        {
            destination.copy_from_slice(source);
        }
        palette
    }

    fn palette_hash(palette: &IndexedGamePalette) -> String {
        let bytes: Vec<_> = palette.iter().flatten().copied().collect();
        format!("{:x}", Sha256::digest(bytes))
    }

    fn framebuffer_hash(framebuffer: &[u8]) -> String {
        format!("{:x}", Sha256::digest(framebuffer))
    }

    #[test]
    fn back_buffer_wrappers_decode_both_authored_resources_without_palette_changes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let asset_root = root.join("accuracy/cblood_install/cblood");
        let mut palette = [[17; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        let expected_palette = palette;
        let mut chart_frame = vec![23; PANORAMA_FRAME_PIXEL_COUNT];
        let mut orx_frame = vec![29; PANORAMA_FRAME_PIXEL_COUNT];

        let chart = std::fs::read(asset_root.join("CHART.FD")).unwrap();
        let chart_result =
            decode_chart_back_buffer(&chart, &mut chart_frame, &mut palette).unwrap();
        assert!(!chart_result.palette_changed);
        assert_eq!(palette, expected_palette);
        assert!(chart_frame.iter().any(|pixel| *pixel != 23));

        let orx = std::fs::read(asset_root.join("ORX.FD")).unwrap();
        let orx_result = decode_orx_back_buffer(&orx, &mut orx_frame, &mut palette).unwrap();
        assert!(!orx_result.palette_changed);
        assert_eq!(palette, expected_palette);
        assert!(orx_frame.iter().any(|pixel| *pixel != 29));
        assert_ne!(chart_frame, orx_frame);
    }

    #[test]
    fn wrapper_resource_names_match_the_original_executable_table() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let executable = std::fs::read(root.join("re/bin/BLOODPRG.EXE")).unwrap();
        assert_eq!(
            c_string_at(
                &executable,
                BLOODPRG_DATA_FILE_OFFSET + ORX_PATH_DATA_OFFSET
            ),
            ORX_BACK_BUFFER_RESOURCE_PATH.as_bytes()
        );
        assert_eq!(
            c_string_at(
                &executable,
                BLOODPRG_DATA_FILE_OFFSET + CHART_PATH_DATA_OFFSET
            ),
            CHART_BACK_BUFFER_RESOURCE_PATH.as_bytes()
        );
    }

    fn c_string_at(bytes: &[u8], start: usize) -> &[u8] {
        let tail = &bytes[start..];
        let end = tail.iter().position(|byte| *byte == u8::MIN).unwrap();
        &tail[..end]
    }

    fn options(vector: &PbmOracle) -> PbmDecodeOptions {
        let palette_update = match vector.palette_bytes_written {
            usize::MIN => PbmPaletteUpdate::Preserve,
            FULL_PALETTE_BYTE_COUNT => PbmPaletteUpdate::AllColors,
            value if value == PBM_SCENE_PALETTE_COLOR_COUNT * RGB_COMPONENT_COUNT => {
                PbmPaletteUpdate::SceneColors
            }
            value => panic!("unexpected oracle palette byte count {value}"),
        };
        PbmDecodeOptions {
            palette_update,
            transparency: if vector.transparent_zero {
                PbmTransparency::TransparentZero
            } else {
                PbmTransparency::Opaque
            },
        }
    }

    #[test]
    fn decoder_matches_every_applicable_native_oracle_vector() {
        let vectors: Vec<PbmOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2bfd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), EXPECTED_VECTOR_COUNT);

        let mut exact_successes = usize::MIN;
        let mut malformed = usize::MIN;
        let mut host_failures = usize::MIN;
        let mut alias_replacements = usize::MIN;
        let mut direction_replacements = usize::MIN;
        for vector in vectors {
            if vector.open_success == Some(false) {
                host_failures += 1;
                continue;
            }
            let source = decode_hex(&vector.payload_hex);
            let mut palette = palette_from_hex(&vector.palette_before_hex);
            let palette_before = palette;
            let mut framebuffer = vec![vector.framebuffer_seed_byte; PANORAMA_FRAME_PIXEL_COUNT];
            let framebuffer_before = framebuffer.clone();
            let result =
                decode_pbm_image(&source, &mut framebuffer, &mut palette, options(&vector));

            if !vector.succeeded {
                if vector.direction_flag {
                    let result = result.unwrap();
                    assert_eq!(result.last_palette_index, NORMAL_STREAM_LAST_PALETTE_INDEX);
                    direction_replacements += 1;
                } else {
                    assert!(result.is_err(), "{}", vector.name);
                    assert_eq!(framebuffer, framebuffer_before);
                    assert_eq!(palette, palette_before);
                    malformed += 1;
                }
                continue;
            }

            let result = result.unwrap();
            assert_eq!(
                result.last_palette_index,
                u8::try_from(vector.return_eax).unwrap(),
                "{}",
                vector.name
            );
            assert_eq!(
                palette_hash(&palette),
                vector.palette_sha256,
                "{}",
                vector.name
            );
            if vector.in_place {
                alias_replacements += 1;
            } else {
                assert_eq!(
                    framebuffer_hash(&framebuffer),
                    vector.output_sha256,
                    "{}",
                    vector.name
                );
                exact_successes += 1;
            }
        }

        assert_eq!(exact_successes, EXACT_FORWARD_SUCCESS_COUNT);
        assert_eq!(malformed, MALFORMED_FORWARD_COUNT);
        assert_eq!(host_failures, HOST_OPEN_FAILURE_COUNT);
        assert_eq!(alias_replacements, FLAT_ALIAS_REPLACEMENT_COUNT);
        assert_eq!(direction_replacements, DIRECTION_STATE_REPLACEMENT_COUNT);
    }

    #[test]
    fn invalid_destinations_fail_before_mutating_palette() {
        let source = b"PBM CMAP";
        let mut framebuffer = [];
        let mut palette = [[91; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        let before = palette;
        assert_eq!(
            decode_pbm_image(
                source,
                &mut framebuffer,
                &mut palette,
                PbmDecodeOptions::default(),
            ),
            Err(PbmDecodeError::FramebufferTooShort(usize::MIN))
        );
        assert_eq!(palette, before);
    }
}
