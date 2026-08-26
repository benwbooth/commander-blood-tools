//! Checked 2D drawing primitives over the flat indexed framebuffer.

use std::error::Error;
use std::fmt;

use super::framebuffer_copy::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH};
use super::sprite_geometry::BridgeSpriteRect;

const PALETTE_ENTRY_COUNT: usize = 256;
const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;

/// Logical signed pixel coordinate in the original 320 by 200 display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RasterPoint {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}

/// Pixel operation applied by a horizontal or vertical span.
#[derive(Clone, Copy, Debug)]
pub enum RasterSpanPaint<'a> {
    /// Replace every selected destination pixel with one palette index.
    Solid(u8),
    /// Transform each existing destination index through an authored table.
    Remap(&'a [u8; PALETTE_ENTRY_COUNT]),
}

/// Observable geometry selected by a span primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterSpanOutcome {
    /// Signed extent or orthogonal clipping rejected the span without mutation.
    Rejected,
    /// One clipped span was drawn.
    Drawn {
        /// First logical pixel modified.
        start: RasterPoint,
        /// Number of modified pixels.
        pixel_count: usize,
    },
}

/// Invalid flat framebuffer or clipping geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterPrimitiveError {
    /// The destination does not contain a complete logical framebuffer.
    FramebufferTooShort {
        /// Number of available indexed pixels.
        actual: usize,
    },
    /// The clip rectangle is inverted or outside the logical display.
    ClipOutsideDisplay(BridgeSpriteRect),
}

impl fmt::Display for RasterPrimitiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Commander Blood raster primitive: {self:?}"
        )
    }
}

impl Error for RasterPrimitiveError {}

/// Draw a clipped horizontal span.
///
/// This translates `gfx_horizontal_span` at BLOODPRG offset `0x0032AC`.
/// It retains signed nonpositive-width rejection, half-open clipping, and the
/// destination-color remap mode. A flat logical origin replaces far-pointer
/// offsets, 16-bit row wrapping, and inherited CPU direction state.
pub fn draw_horizontal_span(
    framebuffer: &mut [u8],
    clip: BridgeSpriteRect,
    start: RasterPoint,
    width_word: u16,
    paint: RasterSpanPaint<'_>,
) -> Result<RasterSpanOutcome, RasterPrimitiveError> {
    validate_flat_raster(framebuffer, clip)?;
    let signed_width = i32::from(width_word as i16);
    if signed_width <= i32::from(u16::MIN) || start.y < clip.top || start.y >= clip.bottom {
        return Ok(RasterSpanOutcome::Rejected);
    }

    let clipped_left = start.x.max(clip.left);
    let clipped_right = start.x.saturating_add(signed_width).min(clip.right);
    if clipped_left >= clipped_right {
        return Ok(RasterSpanOutcome::Rejected);
    }
    let pixel_count =
        usize::try_from(clipped_right - clipped_left).expect("validated positive horizontal span");
    let row = usize::try_from(start.y).expect("validated clip row");
    let first_column = usize::try_from(clipped_left).expect("validated clip column");
    let start_index = row * LOGICAL_FRAMEBUFFER_WIDTH + first_column;
    paint_pixels(
        &mut framebuffer[start_index..start_index + pixel_count],
        paint,
    );
    Ok(RasterSpanOutcome::Drawn {
        start: RasterPoint {
            x: clipped_left,
            y: start.y,
        },
        pixel_count,
    })
}

/// Draw a clipped vertical span.
///
/// This translates `gfx_vertical_span` at BLOODPRG offset `0x003321`.
/// It retains signed nonpositive-height rejection, half-open clipping, and the
/// destination-color remap mode while replacing segment offsets and wrapping
/// 320-byte pointer steps with checked logical rows.
pub fn draw_vertical_span(
    framebuffer: &mut [u8],
    clip: BridgeSpriteRect,
    start: RasterPoint,
    height_word: u16,
    paint: RasterSpanPaint<'_>,
) -> Result<RasterSpanOutcome, RasterPrimitiveError> {
    validate_flat_raster(framebuffer, clip)?;
    let signed_height = i32::from(height_word as i16);
    if signed_height <= i32::from(u16::MIN) || start.x < clip.left || start.x >= clip.right {
        return Ok(RasterSpanOutcome::Rejected);
    }

    let clipped_top = start.y.max(clip.top);
    let clipped_bottom = start.y.saturating_add(signed_height).min(clip.bottom);
    if clipped_top >= clipped_bottom {
        return Ok(RasterSpanOutcome::Rejected);
    }
    let pixel_count =
        usize::try_from(clipped_bottom - clipped_top).expect("validated positive vertical span");
    let column = usize::try_from(start.x).expect("validated clip column");
    let first_row = usize::try_from(clipped_top).expect("validated clip row");
    for row in first_row..first_row + pixel_count {
        let pixel = &mut framebuffer[row * LOGICAL_FRAMEBUFFER_WIDTH + column];
        paint_pixel(pixel, paint);
    }
    Ok(RasterSpanOutcome::Drawn {
        start: RasterPoint {
            x: start.x,
            y: clipped_top,
        },
        pixel_count,
    })
}

fn validate_flat_raster(
    framebuffer: &[u8],
    clip: BridgeSpriteRect,
) -> Result<(), RasterPrimitiveError> {
    if framebuffer.len() < LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        return Err(RasterPrimitiveError::FramebufferTooShort {
            actual: framebuffer.len(),
        });
    }
    let valid = clip.left >= i32::from(u16::MIN)
        && clip.left <= clip.right
        && clip.right <= LOGICAL_FRAMEBUFFER_WIDTH as i32
        && clip.top >= i32::from(u16::MIN)
        && clip.top <= clip.bottom
        && clip.bottom <= LOGICAL_FRAMEBUFFER_HEIGHT as i32;
    if !valid {
        return Err(RasterPrimitiveError::ClipOutsideDisplay(clip));
    }
    Ok(())
}

fn paint_pixels(pixels: &mut [u8], paint: RasterSpanPaint<'_>) {
    match paint {
        RasterSpanPaint::Solid(color) => pixels.fill(color),
        RasterSpanPaint::Remap(table) => {
            for pixel in pixels {
                *pixel = table[usize::from(*pixel)];
            }
        }
    }
}

fn paint_pixel(pixel: &mut u8, paint: RasterSpanPaint<'_>) {
    match paint {
        RasterSpanPaint::Solid(color) => *pixel = color,
        RasterSpanPaint::Remap(table) => *pixel = table[usize::from(*pixel)],
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const HORIZONTAL_SPAN_ORACLE_COUNT: usize = 14;
    const VERTICAL_SPAN_ORACLE_COUNT: usize = 14;
    const REMAP_ENABLED_FLAG: u8 = 1;
    const FRAMEBUFFER_INDEX_MULTIPLIER: usize = 37;
    const FRAMEBUFFER_CASE_MULTIPLIER: usize = 41;
    const FRAMEBUFFER_COLOR_OFFSET: usize = 43;
    const REMAP_COLOR_MULTIPLIER: usize = 73;
    const REMAP_CASE_MULTIPLIER: usize = 11;
    const REMAP_XOR_MASK: usize = 165;

    #[derive(Deserialize)]
    struct HorizontalSpanOracle {
        name: String,
        x: u16,
        y: u16,
        input_width: u16,
        color: u8,
        clip: [u16; 4],
        rejected: bool,
        clipped_x: u16,
        clipped_width: u16,
        remap_flag: u8,
    }

    #[derive(Deserialize)]
    struct VerticalSpanOracle {
        name: String,
        x: u16,
        input_y: u16,
        input_height: u16,
        color: u8,
        clip: [u16; 4],
        rejected: bool,
        clipped_y: u16,
        clipped_height: u16,
        remap_flag: u8,
    }

    #[test]
    fn horizontal_spans_match_every_flat_original_vector() {
        let vectors: Vec<HorizontalSpanOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_32ac_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), HORIZONTAL_SPAN_ORACLE_COUNT);

        for (case_index, vector) in vectors.iter().enumerate() {
            let clip = rect(vector.clip);
            let mut framebuffer = framebuffer(case_index);
            let before = framebuffer.clone();
            let remap = remap_table(case_index);
            let result = draw_horizontal_span(
                &mut framebuffer,
                clip,
                RasterPoint {
                    x: i32::from(vector.x as i16),
                    y: i32::from(vector.y as i16),
                },
                vector.input_width,
                paint(vector.color, vector.remap_flag, &remap),
            );

            if !valid_clip(clip) {
                assert_eq!(
                    result,
                    Err(RasterPrimitiveError::ClipOutsideDisplay(clip)),
                    "{}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
                continue;
            }
            let outcome = result.unwrap();
            if vector.rejected {
                assert_eq!(outcome, RasterSpanOutcome::Rejected, "{}", vector.name);
                assert_eq!(framebuffer, before, "{}", vector.name);
                continue;
            }
            let expected_start = RasterPoint {
                x: i32::from(vector.clipped_x as i16),
                y: i32::from(vector.y as i16),
            };
            let expected_count = usize::from(vector.clipped_width);
            assert_eq!(
                outcome,
                RasterSpanOutcome::Drawn {
                    start: expected_start,
                    pixel_count: expected_count,
                },
                "{}",
                vector.name
            );
            apply_expected_horizontal(
                &mut framebuffer,
                &before,
                expected_start,
                expected_count,
                paint(vector.color, vector.remap_flag, &remap),
            );
        }
    }

    #[test]
    fn vertical_spans_match_every_flat_original_vector() {
        let vectors: Vec<VerticalSpanOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3321_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), VERTICAL_SPAN_ORACLE_COUNT);

        for (case_index, vector) in vectors.iter().enumerate() {
            let clip = rect(vector.clip);
            let mut framebuffer = framebuffer(case_index);
            let before = framebuffer.clone();
            let remap = remap_table(case_index);
            let result = draw_vertical_span(
                &mut framebuffer,
                clip,
                RasterPoint {
                    x: i32::from(vector.x as i16),
                    y: i32::from(vector.input_y as i16),
                },
                vector.input_height,
                paint(vector.color, vector.remap_flag, &remap),
            );

            if !valid_clip(clip) {
                assert_eq!(
                    result,
                    Err(RasterPrimitiveError::ClipOutsideDisplay(clip)),
                    "{}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
                continue;
            }
            let outcome = result.unwrap();
            if vector.rejected {
                assert_eq!(outcome, RasterSpanOutcome::Rejected, "{}", vector.name);
                assert_eq!(framebuffer, before, "{}", vector.name);
                continue;
            }
            let expected_start = RasterPoint {
                x: i32::from(vector.x as i16),
                y: i32::from(vector.clipped_y as i16),
            };
            let expected_count = usize::from(vector.clipped_height);
            assert_eq!(
                outcome,
                RasterSpanOutcome::Drawn {
                    start: expected_start,
                    pixel_count: expected_count,
                },
                "{}",
                vector.name
            );
            apply_expected_vertical(
                &mut framebuffer,
                &before,
                expected_start,
                expected_count,
                paint(vector.color, vector.remap_flag, &remap),
            );
        }
    }

    fn apply_expected_horizontal(
        actual: &mut [u8],
        before: &[u8],
        start: RasterPoint,
        pixel_count: usize,
        paint: RasterSpanPaint<'_>,
    ) {
        let first = start.y as usize * LOGICAL_FRAMEBUFFER_WIDTH + start.x as usize;
        let mut expected = before.to_vec();
        paint_pixels(&mut expected[first..first + pixel_count], paint);
        assert_eq!(actual, expected);
    }

    fn apply_expected_vertical(
        actual: &mut [u8],
        before: &[u8],
        start: RasterPoint,
        pixel_count: usize,
        paint: RasterSpanPaint<'_>,
    ) {
        let mut expected = before.to_vec();
        for row in start.y as usize..start.y as usize + pixel_count {
            paint_pixel(
                &mut expected[row * LOGICAL_FRAMEBUFFER_WIDTH + start.x as usize],
                paint,
            );
        }
        assert_eq!(actual, expected);
    }

    fn framebuffer(case_index: usize) -> Vec<u8> {
        (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
            .map(|index| {
                (index * FRAMEBUFFER_INDEX_MULTIPLIER
                    + case_index * FRAMEBUFFER_CASE_MULTIPLIER
                    + FRAMEBUFFER_COLOR_OFFSET) as u8
            })
            .collect()
    }

    fn remap_table(case_index: usize) -> [u8; PALETTE_ENTRY_COUNT] {
        std::array::from_fn(|value| {
            ((value * REMAP_COLOR_MULTIPLIER + case_index * REMAP_CASE_MULTIPLIER) ^ REMAP_XOR_MASK)
                as u8
        })
    }

    fn paint<'a>(color: u8, remap_flag: u8, remap: &'a [u8; 256]) -> RasterSpanPaint<'a> {
        if remap_flag & REMAP_ENABLED_FLAG == u8::MIN {
            RasterSpanPaint::Solid(color)
        } else {
            RasterSpanPaint::Remap(remap)
        }
    }

    fn rect(words: [u16; 4]) -> BridgeSpriteRect {
        BridgeSpriteRect {
            left: i32::from(words[0] as i16),
            right: i32::from(words[1] as i16),
            top: i32::from(words[2] as i16),
            bottom: i32::from(words[3] as i16),
        }
    }

    fn valid_clip(clip: BridgeSpriteRect) -> bool {
        clip.left >= i32::from(u16::MIN)
            && clip.left <= clip.right
            && clip.right <= LOGICAL_FRAMEBUFFER_WIDTH as i32
            && clip.top >= i32::from(u16::MIN)
            && clip.top <= clip.bottom
            && clip.bottom <= LOGICAL_FRAMEBUFFER_HEIGHT as i32
    }
}
