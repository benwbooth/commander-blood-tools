//! Checked row-span copies between flat indexed framebuffers.

use std::fmt;

use super::sprite_geometry::BridgeSpriteRect;

/// Width of the original logical display in pixels.
pub const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
/// Height of the original logical display in pixels.
pub const LOGICAL_FRAMEBUFFER_HEIGHT: usize = 200;

const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
const CROPPED_PRESENT_FIRST_ROW: usize = 35;
const CROPPED_PRESENT_LAST_ROW_EXCLUSIVE: usize = 165;
const MAXIMUM_CROPPED_PRESENT_DEPTH: usize =
    (CROPPED_PRESENT_LAST_ROW_EXCLUSIVE - CROPPED_PRESENT_FIRST_ROW) / 2;

/// Which flat framebuffer failed runtime validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferKind {
    /// Display surface receiving complete frames and band fills.
    DisplayBuffer,
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
    /// The requested vertical band is inverted or outside the logical display.
    BandOutsideDisplay {
        /// Inclusive first row of the band.
        top: usize,
        /// Exclusive final row of the band.
        bottom: usize,
    },
    /// A ship-depth crop would invert the authored presentation band.
    CropDepthOutsideDisplay {
        /// Requested inward crop depth in logical rows.
        depth: usize,
    },
    /// A dirty rectangle is inverted, empty vertically, or outside the logical display.
    DirtyRegionOutsideDisplay {
        /// Inclusive left edge.
        left: i32,
        /// Exclusive right edge.
        right: i32,
        /// Inclusive top edge.
        top: i32,
        /// Exclusive bottom edge.
        bottom: i32,
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

/// Fill one half-open row band in the display framebuffer.
///
/// This translates `blit_fill_row_5221` at BLOODPRG offset `0x003D7B`.
/// Logical row indices replace byte-swapped wrapping offsets and the ignored
/// offset half of the native far framebuffer pointer.
pub fn fill_display_band(
    display_buffer: &mut [u8],
    top: usize,
    bottom: usize,
    color: u8,
) -> Result<(), FramebufferCopyError> {
    fill_framebuffer_band(
        display_buffer,
        FramebufferKind::DisplayBuffer,
        top,
        bottom,
        color,
    )
}

/// Fill one half-open row band in the back buffer.
///
/// This translates `back_buffer_fill` at BLOODPRG offset `0x003DBF` over the
/// same checked flat geometry as [`fill_display_band`].
pub fn fill_back_buffer_band(
    back_buffer: &mut [u8],
    top: usize,
    bottom: usize,
    color: u8,
) -> Result<(), FramebufferCopyError> {
    fill_framebuffer_band(back_buffer, FramebufferKind::BackBuffer, top, bottom, color)
}

/// Copy one complete logical frame to the display framebuffer.
///
/// This translates `full_screen_blit` at BLOODPRG offset `0x003E46` without
/// segmented source and destination offset wrapping.
pub fn copy_full_frame_to_display(
    source: &[u8],
    display_buffer: &mut [u8],
) -> Result<(), FramebufferCopyError> {
    copy_full_frame(source, display_buffer, FramebufferKind::DisplayBuffer)
}

/// Copy one complete logical frame to the back buffer.
///
/// This translates `fullscreen_copy_to_backbuffer` at BLOODPRG offset
/// `0x003E5B` without segmented source and destination offset wrapping.
pub fn copy_full_frame_to_back_buffer(
    source: &[u8],
    back_buffer: &mut [u8],
) -> Result<(), FramebufferCopyError> {
    copy_full_frame(source, back_buffer, FramebufferKind::BackBuffer)
}

/// Summary of pixels copied for one dirty-region presentation pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyRegionCopyOutcome {
    /// Number of half-open rectangles processed in their authored order.
    pub copied_region_count: usize,
    /// Number of horizontal rows copied across all rectangles.
    pub copied_row_count: usize,
    /// Total number of indexed pixels copied.
    pub copied_pixel_count: usize,
}

/// Copy dirty rectangles from the secondary surface into the display surface.
///
/// This translates `dirty_rects_copy_secondary_to_primary` at BLOODPRG offset
/// `0x00509D`. It retains the low-bit gate, ordered rectangle traversal, and
/// half-open edges while replacing the signed sentinel, segmented framebuffer
/// aliases, alignment-specific copy loops, and wrapping offsets with a typed
/// rectangle slice and validated flat framebuffers.
pub fn copy_dirty_regions_to_display(
    dirty_copy_requested: bool,
    regions: &[BridgeSpriteRect],
    secondary_buffer: &[u8],
    display_buffer: &mut [u8],
) -> Result<DirtyRegionCopyOutcome, FramebufferCopyError> {
    if !dirty_copy_requested || regions.is_empty() {
        return Ok(DirtyRegionCopyOutcome::default());
    }
    validate_surface(secondary_buffer, FramebufferKind::WorkSurface)?;
    validate_surface(display_buffer, FramebufferKind::DisplayBuffer)?;

    let mut outcome = DirtyRegionCopyOutcome::default();
    let mut validated = Vec::with_capacity(regions.len());
    for region in regions {
        let valid = region.left >= i32::from(u16::MIN)
            && region.left <= region.right
            && region.right <= LOGICAL_FRAMEBUFFER_WIDTH as i32
            && region.top >= i32::from(u16::MIN)
            && region.top < region.bottom
            && region.bottom <= LOGICAL_FRAMEBUFFER_HEIGHT as i32;
        if !valid {
            return Err(FramebufferCopyError::DirtyRegionOutsideDisplay {
                left: region.left,
                right: region.right,
                top: region.top,
                bottom: region.bottom,
            });
        }
        let left = region.left as usize;
        let right = region.right as usize;
        let top = region.top as usize;
        let bottom = region.bottom as usize;
        let width = right - left;
        let row_count = bottom - top;
        outcome.copied_row_count = outcome.copied_row_count.checked_add(row_count).ok_or(
            FramebufferCopyError::DirtyRegionOutsideDisplay {
                left: region.left,
                right: region.right,
                top: region.top,
                bottom: region.bottom,
            },
        )?;
        outcome.copied_pixel_count = outcome
            .copied_pixel_count
            .checked_add(width * row_count)
            .ok_or(FramebufferCopyError::DirtyRegionOutsideDisplay {
                left: region.left,
                right: region.right,
                top: region.top,
                bottom: region.bottom,
            })?;
        validated.push((left, right, top, bottom));
    }

    for (left, right, top, bottom) in validated {
        for row in top..bottom {
            let start = row * LOGICAL_FRAMEBUFFER_WIDTH + left;
            let end = row * LOGICAL_FRAMEBUFFER_WIDTH + right;
            display_buffer[start..end].copy_from_slice(&secondary_buffer[start..end]);
        }
    }
    outcome.copied_region_count = regions.len();
    Ok(outcome)
}

/// Region copied from the logical framebuffer into the presented frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkyFramePresentation {
    /// Every logical row was copied.
    FullFrame,
    /// The symmetric ship-depth crop copied this half-open row range.
    Cropped {
        /// First copied logical row.
        first_row: usize,
        /// Exclusive final copied logical row.
        last_row_exclusive: usize,
    },
    /// Maximum depth intentionally produced an empty crop.
    Empty,
}

/// Copy the visible portion of a chunky indexed frame into presentation state.
///
/// This translates `chunky_to_planar_framebuffer` at BLOODPRG offset
/// `0x003ECE`. In the flat port there are no VGA planes or sequencer map-mask
/// writes: the same row region is copied directly between complete indexed
/// frames. Crop depth is validated before mutation, replacing native wrapping
/// subtraction and offset arithmetic.
pub fn present_chunky_frame(
    source: &[u8],
    presented: &mut [u8],
    crop_enabled: bool,
    depth: usize,
) -> Result<ChunkyFramePresentation, FramebufferCopyError> {
    validate_surface(source, FramebufferKind::WorkSurface)?;
    validate_surface(presented, FramebufferKind::DisplayBuffer)?;

    if !crop_enabled {
        presented[..LOGICAL_FRAMEBUFFER_PIXEL_COUNT]
            .copy_from_slice(&source[..LOGICAL_FRAMEBUFFER_PIXEL_COUNT]);
        return Ok(ChunkyFramePresentation::FullFrame);
    }
    if depth > MAXIMUM_CROPPED_PRESENT_DEPTH {
        return Err(FramebufferCopyError::CropDepthOutsideDisplay { depth });
    }
    if depth == MAXIMUM_CROPPED_PRESENT_DEPTH {
        return Ok(ChunkyFramePresentation::Empty);
    }

    let first_row = CROPPED_PRESENT_FIRST_ROW + depth;
    let last_row_exclusive = CROPPED_PRESENT_LAST_ROW_EXCLUSIVE - depth;
    let start = first_row * LOGICAL_FRAMEBUFFER_WIDTH;
    let end = last_row_exclusive * LOGICAL_FRAMEBUFFER_WIDTH;
    presented[start..end].copy_from_slice(&source[start..end]);
    Ok(ChunkyFramePresentation::Cropped {
        first_row,
        last_row_exclusive,
    })
}

fn fill_framebuffer_band(
    framebuffer: &mut [u8],
    kind: FramebufferKind,
    top: usize,
    bottom: usize,
    color: u8,
) -> Result<(), FramebufferCopyError> {
    validate_surface(framebuffer, kind)?;
    if top > bottom || bottom > LOGICAL_FRAMEBUFFER_HEIGHT {
        return Err(FramebufferCopyError::BandOutsideDisplay { top, bottom });
    }
    let start = top * LOGICAL_FRAMEBUFFER_WIDTH;
    let end = bottom * LOGICAL_FRAMEBUFFER_WIDTH;
    framebuffer[start..end].fill(color);
    Ok(())
}

fn copy_full_frame(
    source: &[u8],
    destination: &mut [u8],
    destination_kind: FramebufferKind,
) -> Result<(), FramebufferCopyError> {
    validate_surface(source, FramebufferKind::WorkSurface)?;
    validate_surface(destination, destination_kind)?;
    destination[..LOGICAL_FRAMEBUFFER_PIXEL_COUNT]
        .copy_from_slice(&source[..LOGICAL_FRAMEBUFFER_PIXEL_COUNT]);
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
    const BAND_FILL_ORACLE_VECTOR_COUNT: usize = 10;
    const FULL_FRAME_COPY_ORACLE_VECTOR_COUNT: usize = 6;
    const CHUNKY_PRESENT_ORACLE_VECTOR_COUNT: usize = 8;
    const DIRTY_REGION_COPY_ORACLE_VECTOR_COUNT: usize = 8;
    const VGA_PLANE_COUNT: usize = 4;
    const DIRTY_COPY_REQUESTED_FLAG: u8 = 1;
    const DISPLAY_INDEX_MULTIPLIER: usize = 13;
    const DISPLAY_CASE_MULTIPLIER: usize = 31;
    const DISPLAY_COLOR_OFFSET: usize = 7;
    const SECONDARY_INDEX_MULTIPLIER: usize = 29;
    const SECONDARY_CASE_MULTIPLIER: usize = 17;
    const SECONDARY_COLOR_OFFSET: usize = 3;

    #[derive(Deserialize)]
    struct CopyOracle {
        name: String,
        x: usize,
        y: usize,
        width: usize,
        copied_sha256: String,
    }

    #[derive(Deserialize)]
    struct BandFillOracle {
        name: String,
        top: usize,
        bottom: usize,
        color: u8,
        destination_offset: usize,
        dword_count: usize,
        written_byte_count: usize,
    }

    #[derive(Deserialize)]
    struct FullFrameCopyOracle {
        name: String,
        source_offset: usize,
        destination_offset: usize,
        copied_byte_count: usize,
    }

    #[derive(Deserialize)]
    struct ChunkyPresentOracle {
        name: String,
        gate: u8,
        depth: usize,
        cropped: bool,
        early_return: bool,
        bytes_per_plane: usize,
    }

    #[derive(Deserialize)]
    struct DirtyRegionCopyOracle {
        name: String,
        dirty_copy_flags: u8,
        rectangles: Vec<[i32; 4]>,
        copied_rows: Vec<[usize; 2]>,
        copied_bytes: usize,
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

    #[test]
    fn display_and_back_buffer_band_fills_match_flat_original_vectors() {
        verify_band_fills(
            include_str!("../../../../../re/tools/oracle_vectors/func_3d7b_natural.json"),
            fill_display_band,
        );
        verify_band_fills(
            include_str!("../../../../../re/tools/oracle_vectors/func_3dbf_natural.json"),
            fill_back_buffer_band,
        );
    }

    #[test]
    fn complete_frame_copies_match_every_original_extent_vector() {
        verify_full_frame_copies(
            include_str!("../../../../../re/tools/oracle_vectors/func_3e46_natural.json"),
            copy_full_frame_to_display,
        );
        verify_full_frame_copies(
            include_str!("../../../../../re/tools/oracle_vectors/func_3e5b_natural.json"),
            copy_full_frame_to_back_buffer,
        );
    }

    #[test]
    fn dirty_region_copies_match_every_original_vector() {
        let vectors: Vec<DirtyRegionCopyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_509d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DIRTY_REGION_COPY_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.iter().enumerate() {
            let secondary: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| {
                    (index * SECONDARY_INDEX_MULTIPLIER
                        + case_index * SECONDARY_CASE_MULTIPLIER
                        + SECONDARY_COLOR_OFFSET) as u8
                })
                .collect();
            let mut display: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| {
                    (index * DISPLAY_INDEX_MULTIPLIER
                        + case_index * DISPLAY_CASE_MULTIPLIER
                        + DISPLAY_COLOR_OFFSET) as u8
                })
                .collect();
            let mut expected = display.clone();
            for [start, width] in &vector.copied_rows {
                let end = start + width;
                expected[*start..end].copy_from_slice(&secondary[*start..end]);
            }
            let regions: Vec<BridgeSpriteRect> = vector
                .rectangles
                .iter()
                .map(|rectangle| BridgeSpriteRect {
                    left: rectangle[0],
                    right: rectangle[1],
                    top: rectangle[2],
                    bottom: rectangle[3],
                })
                .collect();

            let outcome = copy_dirty_regions_to_display(
                vector.dirty_copy_flags & DIRTY_COPY_REQUESTED_FLAG != u8::MIN,
                &regions,
                &secondary,
                &mut display,
            )
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(display, expected, "{}", vector.name);
            assert_eq!(
                outcome.copied_region_count,
                if vector.dirty_copy_flags & DIRTY_COPY_REQUESTED_FLAG == u8::MIN {
                    usize::MIN
                } else {
                    regions.len()
                },
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.copied_row_count,
                vector.copied_rows.len(),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.copied_pixel_count, vector.copied_bytes,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn invalid_dirty_region_is_rejected_before_any_copy() {
        let secondary = vec![1; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        let mut display = vec![2; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        let before = display.clone();
        let regions = [
            BridgeSpriteRect {
                left: 1,
                right: 4,
                top: 2,
                bottom: 3,
            },
            BridgeSpriteRect {
                left: 7,
                right: 3,
                top: 5,
                bottom: 6,
            },
        ];

        assert_eq!(
            copy_dirty_regions_to_display(true, &regions, &secondary, &mut display),
            Err(FramebufferCopyError::DirtyRegionOutsideDisplay {
                left: 7,
                right: 3,
                top: 5,
                bottom: 6,
            })
        );
        assert_eq!(display, before);
    }

    #[test]
    fn new_framebuffer_primitives_reject_incomplete_surfaces() {
        let mut short = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT - 1];
        assert_eq!(
            fill_display_band(&mut short, 0, 1, 7),
            Err(FramebufferCopyError::SurfaceTooShort {
                surface: FramebufferKind::DisplayBuffer,
                actual: LOGICAL_FRAMEBUFFER_PIXEL_COUNT - 1,
            })
        );

        let complete = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        assert_eq!(
            copy_full_frame_to_back_buffer(&short, &mut complete.clone()),
            Err(FramebufferCopyError::SurfaceTooShort {
                surface: FramebufferKind::WorkSurface,
                actual: LOGICAL_FRAMEBUFFER_PIXEL_COUNT - 1,
            })
        );
    }

    #[test]
    fn chunky_presentation_matches_every_flat_original_crop_vector() {
        let vectors: Vec<ChunkyPresentOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3ece_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), CHUNKY_PRESENT_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let source: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 37 + case_index * 29 + 11) as u8)
                .collect();
            let mut presented: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 13 + case_index * 17 + 7) as u8)
                .collect();
            let before = presented.clone();
            let crop_enabled = vector.gate & 1 != u8::MIN;
            assert_eq!(crop_enabled, vector.cropped, "{}", vector.name);
            let result = present_chunky_frame(&source, &mut presented, crop_enabled, vector.depth);

            if crop_enabled && vector.depth > MAXIMUM_CROPPED_PRESENT_DEPTH {
                assert_eq!(
                    result,
                    Err(FramebufferCopyError::CropDepthOutsideDisplay {
                        depth: vector.depth,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(presented, before, "{}", vector.name);
                continue;
            }

            let outcome = result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            let (first_row, last_row_exclusive) = match outcome {
                ChunkyFramePresentation::FullFrame => (0, LOGICAL_FRAMEBUFFER_HEIGHT),
                ChunkyFramePresentation::Cropped {
                    first_row,
                    last_row_exclusive,
                } => (first_row, last_row_exclusive),
                ChunkyFramePresentation::Empty => (
                    CROPPED_PRESENT_FIRST_ROW + MAXIMUM_CROPPED_PRESENT_DEPTH,
                    CROPPED_PRESENT_LAST_ROW_EXCLUSIVE - MAXIMUM_CROPPED_PRESENT_DEPTH,
                ),
            };
            assert_eq!(
                vector.early_return,
                outcome == ChunkyFramePresentation::Empty,
                "{}",
                vector.name
            );
            let start = first_row * LOGICAL_FRAMEBUFFER_WIDTH;
            let end = last_row_exclusive * LOGICAL_FRAMEBUFFER_WIDTH;
            assert_eq!(
                vector.bytes_per_plane * VGA_PLANE_COUNT,
                end - start,
                "{}",
                vector.name
            );
            assert_eq!(&presented[..start], &before[..start], "{}", vector.name);
            assert_eq!(
                &presented[start..end],
                &source[start..end],
                "{}",
                vector.name
            );
            assert_eq!(&presented[end..], &before[end..], "{}", vector.name);
        }
    }

    fn verify_band_fills(
        input: &str,
        fill: fn(&mut [u8], usize, usize, u8) -> Result<(), FramebufferCopyError>,
    ) {
        let vectors: Vec<BandFillOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), BAND_FILL_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mut framebuffer: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 13 + case_index * 29 + 5) as u8)
                .collect();
            let before = framebuffer.clone();
            let result = fill(&mut framebuffer, vector.top, vector.bottom, vector.color);

            let valid = vector.top <= vector.bottom && vector.bottom <= LOGICAL_FRAMEBUFFER_HEIGHT;
            if valid {
                result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
                let start = vector.top * LOGICAL_FRAMEBUFFER_WIDTH;
                let end = vector.bottom * LOGICAL_FRAMEBUFFER_WIDTH;
                assert_eq!(vector.destination_offset, start, "{}", vector.name);
                assert_eq!(vector.dword_count * 4, end - start, "{}", vector.name);
                assert_eq!(vector.written_byte_count, end - start, "{}", vector.name);
                assert_eq!(&framebuffer[..start], &before[..start], "{}", vector.name);
                assert!(
                    framebuffer[start..end]
                        .iter()
                        .all(|pixel| *pixel == vector.color),
                    "{}",
                    vector.name
                );
                assert_eq!(&framebuffer[end..], &before[end..], "{}", vector.name);
            } else {
                assert_eq!(
                    result,
                    Err(FramebufferCopyError::BandOutsideDisplay {
                        top: vector.top,
                        bottom: vector.bottom,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
            }
        }
    }

    fn verify_full_frame_copies(
        input: &str,
        copy: fn(&[u8], &mut [u8]) -> Result<(), FramebufferCopyError>,
    ) {
        let vectors: Vec<FullFrameCopyOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), FULL_FRAME_COPY_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let source: Vec<u8> = (0..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
                .map(|index| (index * 37 + case_index * 11 + 3) as u8)
                .collect();
            let mut destination = vec![u8::MAX; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
            copy(&source, &mut destination)
                .unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            assert_eq!(destination, source, "{}", vector.name);
            assert_eq!(
                vector.copied_byte_count, LOGICAL_FRAMEBUFFER_PIXEL_COUNT,
                "{} source offset {} destination offset {}",
                vector.name, vector.source_offset, vector.destination_offset
            );
        }
    }
}
