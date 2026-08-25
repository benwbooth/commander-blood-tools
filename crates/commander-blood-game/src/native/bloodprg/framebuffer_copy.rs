//! Checked row-span copies between flat indexed framebuffers.

use std::fmt;

/// Width of the original logical display in pixels.
pub const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
/// Height of the original logical display in pixels.
pub const LOGICAL_FRAMEBUFFER_HEIGHT: usize = 200;

const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;

/// Which flat framebuffer failed runtime validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferKind {
    /// Surface from which bridge graphics are copied.
    WorkSurface,
    /// Back buffer receiving the copied pixels.
    BackBuffer,
}

/// Invalid surface or row span supplied to a framebuffer copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferCopyError {
    /// A surface does not contain a complete logical display.
    SurfaceTooShort {
        /// Surface that failed validation.
        surface: FramebufferKind,
        /// Number of available pixels.
        actual: usize,
    },
    /// The requested row span is outside the logical display.
    SpanOutsideDisplay {
        /// First pixel column.
        x: usize,
        /// Pixel row.
        y: usize,
        /// Number of pixels to copy.
        width: usize,
    },
}

impl fmt::Display for FramebufferCopyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FramebufferCopyError {}

/// Copy one horizontal span from the work surface to the back buffer.
///
/// This translates `back_buffer_copy_from` at BLOODPRG routine offset
/// `0x00933A`. Checked indices into flat pixel slices replace far framebuffer
/// pointers and 16-bit offset wrapping. All recovered callers use rows 0
/// through 199; requests outside that domain are rejected.
pub fn copy_work_surface_span(
    work_surface: &[u8],
    back_buffer: &mut [u8],
    x: usize,
    y: usize,
    width: usize,
) -> Result<(), FramebufferCopyError> {
    validate_surface(work_surface, FramebufferKind::WorkSurface)?;
    validate_surface(back_buffer, FramebufferKind::BackBuffer)?;

    let right = x.checked_add(width);
    if y >= LOGICAL_FRAMEBUFFER_HEIGHT
        || x > LOGICAL_FRAMEBUFFER_WIDTH
        || right.is_none_or(|edge| edge > LOGICAL_FRAMEBUFFER_WIDTH)
    {
        return Err(FramebufferCopyError::SpanOutsideDisplay { x, y, width });
    }

    let start = y * LOGICAL_FRAMEBUFFER_WIDTH + x;
    let end = start + width;
    back_buffer[start..end].copy_from_slice(&work_surface[start..end]);
    Ok(())
}

fn validate_surface(surface: &[u8], kind: FramebufferKind) -> Result<(), FramebufferCopyError> {
    if surface.len() < LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        return Err(FramebufferCopyError::SurfaceTooShort {
            surface: kind,
            actual: surface.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 12;

    #[derive(Deserialize)]
    struct CopyOracle {
        name: String,
        x: usize,
        y: usize,
        width: usize,
        copied_sha256: String,
    }

    #[test]
    fn valid_spans_match_original_copied_pixels_and_wrapping_cases_are_rejected() {
        let vectors: Vec<CopyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_933a_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let source: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 37 + case_index * 29 + 11) as u8)
                .collect();
            let mut destination: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 13 + case_index * 17 + 7) as u8)
                .collect();
            let before = destination.clone();
            let result =
                copy_work_surface_span(&source, &mut destination, vector.x, vector.y, vector.width);

            let valid = vector.y < LOGICAL_FRAMEBUFFER_HEIGHT
                && vector.x <= LOGICAL_FRAMEBUFFER_WIDTH
                && vector
                    .x
                    .checked_add(vector.width)
                    .is_some_and(|right| right <= LOGICAL_FRAMEBUFFER_WIDTH);
            if valid {
                result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
                let start = vector.y * LOGICAL_FRAMEBUFFER_WIDTH + vector.x;
                let copied = &destination[start..start + vector.width];
                assert_eq!(
                    format!("{:x}", Sha256::digest(copied)),
                    vector.copied_sha256,
                    "{}",
                    vector.name
                );
                assert_eq!(&destination[..start], &before[..start], "{}", vector.name);
                assert_eq!(
                    &destination[start + vector.width..],
                    &before[start + vector.width..],
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(
                    result,
                    Err(FramebufferCopyError::SpanOutsideDisplay {
                        x: vector.x,
                        y: vector.y,
                        width: vector.width,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(destination, before, "{}", vector.name);
            }
        }
    }

    #[test]
    fn incomplete_surfaces_are_rejected_before_copying() {
        let source = vec![0; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        let mut destination = vec![0; LOGICAL_FRAMEBUFFER_PIXEL_COUNT - 1];
        assert_eq!(
            copy_work_surface_span(&source, &mut destination, 0, 0, 1),
            Err(FramebufferCopyError::SurfaceTooShort {
                surface: FramebufferKind::BackBuffer,
                actual: LOGICAL_FRAMEBUFFER_PIXEL_COUNT - 1,
            })
        );
    }
}
