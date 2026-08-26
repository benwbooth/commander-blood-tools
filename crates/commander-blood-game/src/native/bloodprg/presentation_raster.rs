//! Flat indexed-framebuffer primitives for streamed presentation resources.

use std::error::Error;
use std::fmt;

use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH};

const TRANSPARENT_ROW_MODE: u8 = u8::MAX;

/// Invalid source data or flat framebuffer geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationRasterError {
    /// The original direct-call zero-row case underflows to 256 iterations.
    ZeroRows,
    /// The source does not contain every requested row pixel.
    SourceTruncated {
        /// Required source byte count.
        required: usize,
        /// Available source byte count.
        available: usize,
    },
    /// Rectangle geometry lies outside the fixed logical framebuffer.
    RectangleOutOfBounds {
        /// Left pixel coordinate.
        x: usize,
        /// Top pixel coordinate.
        y: usize,
        /// Rectangle width.
        width: usize,
        /// Rectangle height.
        rows: usize,
    },
    /// Advancing one scanline would leave the logical framebuffer.
    ScanlineOutOfBounds {
        /// Current flat framebuffer position.
        row_offset: usize,
    },
}

impl fmt::Display for PresentationRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation raster operation: {self:?}")
    }
}

impl Error for PresentationRasterError {}

/// Observable result of one presentation rectangle blit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRectBlitOutcome {
    /// Source bytes consumed by the rectangle.
    pub consumed_bytes: usize,
    /// Destination pixels whose values changed.
    pub changed_pixels: usize,
}

/// Copy one opaque or zero-transparent rectangle into the indexed framebuffer.
///
/// This translates `resource_rect_blit` at BLOODPRG offset `0x00A4ED`.
/// Checked rows replace wrapping far offsets and the ambient direction flag;
/// the full-width and pitched native paths collapse to the same flat raster.
pub fn blit_presentation_rect(
    source: &[u8],
    framebuffer: &mut [u8],
    x: usize,
    y: usize,
    width: usize,
    row_mode: u16,
) -> Result<PresentationRectBlitOutcome, PresentationRasterError> {
    let rows = usize::from(row_mode as u8);
    if rows == usize::MIN {
        return Err(PresentationRasterError::ZeroRows);
    }
    let right = x.checked_add(width);
    let bottom = y.checked_add(rows);
    if right.is_none_or(|right| right > LOGICAL_FRAMEBUFFER_WIDTH)
        || bottom.is_none_or(|bottom| bottom > LOGICAL_FRAMEBUFFER_HEIGHT)
        || framebuffer.len() < LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT
    {
        return Err(PresentationRasterError::RectangleOutOfBounds { x, y, width, rows });
    }
    let required = width
        .checked_mul(rows)
        .ok_or(PresentationRasterError::SourceTruncated {
            required: usize::MAX,
            available: source.len(),
        })?;
    let pixels = source
        .get(..required)
        .ok_or(PresentationRasterError::SourceTruncated {
            required,
            available: source.len(),
        })?;
    if width == usize::MIN {
        return Ok(PresentationRectBlitOutcome {
            consumed_bytes: usize::MIN,
            changed_pixels: usize::MIN,
        });
    }
    let transparent = (row_mode >> u8::BITS) as u8 == TRANSPARENT_ROW_MODE;
    let mut changed_pixels = usize::MIN;

    for (row_index, source_row) in pixels.chunks_exact(width).enumerate() {
        let row_start = (y + row_index) * LOGICAL_FRAMEBUFFER_WIDTH + x;
        let destination_row = &mut framebuffer[row_start..row_start + width];
        for (destination, source) in destination_row.iter_mut().zip(source_row) {
            if transparent && *source == u8::MIN {
                continue;
            }
            changed_pixels += usize::from(*destination != *source);
            *destination = *source;
        }
    }

    Ok(PresentationRectBlitOutcome {
        consumed_bytes: required,
        changed_pixels,
    })
}

/// Current destination position for a row-oriented presentation decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationScanlineState {
    /// Decoded pixel count in each row.
    pub row_width: usize,
    /// Flat destination position of the current row.
    pub row_offset: usize,
    /// Rows including the current row that remain to be decoded.
    pub rows_remaining: u8,
}

/// Retire one decoded row and select the next framebuffer scanline.
///
/// This translates `gfx_scanline_advance` at `0x00AD96`. Returning `false`
/// replaces the original nonlocal stack unwind. Zero-row underflow and a
/// wrapped 16-bit row position are rejected without changing state.
pub fn advance_presentation_scanline(
    state: &mut PresentationScanlineState,
) -> Result<bool, PresentationRasterError> {
    let next_rows = state
        .rows_remaining
        .checked_sub(1)
        .ok_or(PresentationRasterError::ZeroRows)?;
    if next_rows == u8::MIN {
        state.rows_remaining = next_rows;
        return Ok(false);
    }
    let next_offset = state
        .row_offset
        .checked_add(LOGICAL_FRAMEBUFFER_WIDTH)
        .filter(|offset| *offset < LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT)
        .ok_or(PresentationRasterError::ScanlineOutOfBounds {
            row_offset: state.row_offset,
        })?;
    state.rows_remaining = next_rows;
    state.row_offset = next_offset;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const RECT_BLIT_VECTOR_COUNT: usize = 9;
    const FLAT_RECT_BLIT_VECTOR_COUNT: usize = 6;
    const SCANLINE_VECTOR_COUNT: usize = 5;
    const FLAT_SCANLINE_VECTOR_COUNT: usize = 3;
    const SOURCE_PATTERN_ZERO_DIVISOR: usize = 5;
    const SOURCE_PATTERN_STEP: usize = 17;
    const SOURCE_PATTERN_PAGE_STEP: usize = 7;
    const SOURCE_PATTERN_CASE_STEP: usize = 29;
    const FRAMEBUFFER_PATTERN_STEP: usize = 23;
    const FRAMEBUFFER_PATTERN_PAGE_STEP: usize = 11;
    const FRAMEBUFFER_PATTERN_CASE_STEP: usize = 31;

    #[derive(Deserialize)]
    struct RectBlitOracle {
        name: String,
        width: usize,
        row_mode: u16,
        rows: usize,
        transparent_zero: bool,
        x: usize,
        y: usize,
        direction: String,
        source_offset: usize,
        changed_bytes: usize,
    }

    #[derive(Deserialize)]
    struct ScanlineOracle {
        name: String,
        continues: bool,
        initial_rows_word: u16,
        result_rows_word: u16,
        row_width: usize,
        initial_row_offset: usize,
        result_row_offset: usize,
    }

    fn source_memory(case_index: usize) -> Vec<u8> {
        (usize::MIN..=usize::from(u16::MAX))
            .map(|offset| {
                if (offset + case_index).is_multiple_of(SOURCE_PATTERN_ZERO_DIVISOR) {
                    u8::MIN
                } else {
                    (offset * SOURCE_PATTERN_STEP
                        + (offset >> u8::BITS) * SOURCE_PATTERN_PAGE_STEP
                        + case_index * SOURCE_PATTERN_CASE_STEP) as u8
                }
            })
            .collect()
    }

    fn framebuffer_memory(case_index: usize) -> Vec<u8> {
        (usize::MIN..LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT)
            .map(|offset| {
                (offset * FRAMEBUFFER_PATTERN_STEP
                    + (offset >> u8::BITS) * FRAMEBUFFER_PATTERN_PAGE_STEP
                    + case_index * FRAMEBUFFER_PATTERN_CASE_STEP) as u8
            })
            .collect()
    }

    #[test]
    fn rectangle_blit_matches_every_flat_original_vector() {
        let vectors: Vec<RectBlitOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a4ed_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RECT_BLIT_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for (case_index, vector) in vectors.into_iter().enumerate() {
            assert_eq!(
                usize::from(vector.row_mode as u8),
                vector.rows,
                "{}",
                vector.name
            );
            assert_eq!(
                (vector.row_mode >> u8::BITS) as u8 == TRANSPARENT_ROW_MODE,
                vector.transparent_zero,
                "{}",
                vector.name
            );
            if vector.direction != "forward"
                || vector.rows == usize::MIN
                || vector.x + vector.width > LOGICAL_FRAMEBUFFER_WIDTH
                || vector.y + vector.rows > LOGICAL_FRAMEBUFFER_HEIGHT
            {
                continue;
            }

            let source_memory = source_memory(case_index);
            let source = &source_memory[vector.source_offset..];
            let mut framebuffer = framebuffer_memory(case_index);
            let before = framebuffer.clone();
            let outcome = blit_presentation_rect(
                source,
                &mut framebuffer,
                vector.x,
                vector.y,
                vector.width,
                vector.row_mode,
            )
            .unwrap();
            assert_eq!(
                outcome.consumed_bytes,
                vector.width * vector.rows,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.changed_pixels, vector.changed_bytes,
                "{}",
                vector.name
            );
            assert_eq!(
                framebuffer
                    .iter()
                    .zip(&before)
                    .filter(|(after, before)| *after != *before)
                    .count(),
                vector.changed_bytes,
                "{}",
                vector.name
            );
            matched += 1;
        }
        assert_eq!(matched, FLAT_RECT_BLIT_VECTOR_COUNT);
    }

    #[test]
    fn malformed_native_rectangle_domains_are_rejected_or_canonicalized() {
        let source = source_memory(usize::MIN);
        let mut framebuffer = framebuffer_memory(usize::MIN);
        let before = framebuffer.clone();
        assert_eq!(
            blit_presentation_rect(
                &source,
                &mut framebuffer,
                usize::MIN,
                usize::MIN,
                1,
                u16::MIN,
            ),
            Err(PresentationRasterError::ZeroRows)
        );
        assert_eq!(framebuffer, before);
        assert!(matches!(
            blit_presentation_rect(
                &source,
                &mut framebuffer,
                usize::from(u16::MAX),
                usize::MIN,
                1,
                1,
            ),
            Err(PresentationRasterError::RectangleOutOfBounds { .. })
        ));
        assert_eq!(framebuffer, before);
    }

    #[test]
    fn scanline_advance_matches_flat_vectors_and_rejects_wrapping() {
        let vectors: Vec<ScanlineOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_ad96_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SCANLINE_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for vector in vectors {
            let mut state = PresentationScanlineState {
                row_width: vector.row_width,
                row_offset: vector.initial_row_offset,
                rows_remaining: vector.initial_rows_word as u8,
            };
            let initial = state;
            let result = advance_presentation_scanline(&mut state);
            if vector.initial_rows_word as u8 == u8::MIN
                || (vector.continues
                    && vector
                        .initial_row_offset
                        .checked_add(LOGICAL_FRAMEBUFFER_WIDTH)
                        .is_none_or(|offset| {
                            offset >= LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT
                        }))
            {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(state, initial, "{}", vector.name);
                continue;
            }

            assert_eq!(result.unwrap(), vector.continues, "{}", vector.name);
            assert_eq!(
                state.rows_remaining, vector.result_rows_word as u8,
                "{}",
                vector.name
            );
            assert_eq!(
                state.row_offset, vector.result_row_offset,
                "{}",
                vector.name
            );
            assert_eq!(state.row_width, vector.row_width, "{}", vector.name);
            matched += 1;
        }
        assert_eq!(matched, FLAT_SCANLINE_VECTOR_COUNT);
    }
}
