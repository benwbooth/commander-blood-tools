//! Checked raw sprite rasterizers over flat indexed framebuffers.

use std::error::Error;
use std::fmt;

use super::framebuffer_copy::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH};
use super::sprite_geometry::{BridgeSpriteBlitterSelection, BridgeSpriteEntity};

const FRAME_STRIDE_OFFSET: usize = 0;
const FRAME_HEIGHT_OFFSET: usize = 2;
const FRAME_X_ORIGIN_OFFSET: usize = 4;
const FRAME_Y_ORIGIN_OFFSET: usize = 6;
const FRAME_HEADER_BYTE_COUNT: usize = 8;
const DESTINATION_REMAP_SHIFT: u32 = 8;
const DESTINATION_REMAP_MASK: u16 = 3;
const DIRECT_COLOR_MODE: u16 = 0;
const FIRST_DESTINATION_REMAP_MODE: u16 = 1;
const PALETTE_ENTRY_COUNT: usize = 256;

/// Destination-color table selected by the high entity flag byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeSpriteRemapSelection {
    /// Write nonzero source pixels directly.
    Direct,
    /// Transform the existing destination through the first authored table.
    FirstDestinationTable,
    /// Transform the existing destination through the second authored table.
    SecondDestinationTable,
}

/// Both authored destination-color transformation tables.
#[derive(Clone, Copy, Debug)]
pub struct BridgeSpriteRemapTables<'a> {
    /// Table selected by high-byte mode one.
    pub first: &'a [u8; PALETTE_ENTRY_COUNT],
    /// Table selected by high-byte modes two and three.
    pub second: &'a [u8; PALETTE_ENTRY_COUNT],
}

/// Geometry selected by one completed raw sprite blit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpriteBlitOutcome {
    /// Number of source columns and rows rasterized after clipping.
    pub clipped_extent: [usize; 2],
    /// Pixel offset from the end of the eight-byte frame header.
    pub source_start_pixel: usize,
    /// First destination pixel visited, before flip stepping.
    pub destination_start: [i32; 2],
    /// Destination-color operation used by a transparent blit.
    pub remap: Option<BridgeSpriteRemapSelection>,
}

/// Geometry and encoded-stream state selected by one RLE sprite blit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpriteRleBlitOutcome {
    /// Number of decoded columns and rows rasterized after clipping.
    pub clipped_extent: [usize; 2],
    /// Encoded rows consumed before rendering began.
    pub skipped_rows: usize,
    /// X origin read through the post-skip encoded cursor.
    pub cursor_x_origin: i16,
    /// Decoded columns clipped from the source left and right edges.
    pub source_clips: [usize; 2],
    /// First destination pixel visited, before flip stepping.
    pub destination_start: [i32; 2],
    /// Destination-color operation used by a transparent blit.
    pub remap: Option<BridgeSpriteRemapSelection>,
}

/// Invalid frame, entity geometry, clipping state, or flat framebuffer access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeSpriteBlitError {
    /// The entity has no selected resource frame.
    MissingFrame,
    /// No dirty region was assigned by the dispatch stage.
    MissingDirtyRegion,
    /// The frame header or declared pixel payload exceeds the resource.
    TruncatedFrame,
    /// Zero-sized geometry would enter the original decrement-first loop.
    EmptyExtent,
    /// Clipping removes more pixels than the entity extent contains.
    ClipOutsideEntity,
    /// The clipped width exceeds the authored source stride.
    WidthExceedsStride {
        /// Clipped destination width.
        width: usize,
        /// Source bytes per row.
        stride: usize,
    },
    /// The destination does not contain a complete logical framebuffer.
    FramebufferTooShort {
        /// Number of available indexed pixels.
        actual: usize,
    },
    /// Checked flat coordinates fall outside the logical display.
    DestinationOutsideFramebuffer,
    /// Checked source rows exceed the declared frame payload.
    SourceOutsideFrame,
    /// An RLE run is truncated, empty, or crosses the decoded row boundary.
    MalformedRleRow,
}

impl fmt::Display for BridgeSpriteBlitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for BridgeSpriteBlitError {}

/// Rasterize a raw sprite while preserving transparent source zeroes.
///
/// This translates `sprite_blit_raw_transparent` at BLOODPRG routine offset
/// `0x004536`. It retains signed clipping, canonical horizontal and vertical
/// flips, destination-color remapping, and the advanced-cursor X-origin reload.
/// Checked slices replace the frame far pointer and wrapping display offset.
pub fn blit_raw_transparent_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
    remap_tables: BridgeSpriteRemapTables<'_>,
) -> Result<BridgeSpriteBlitOutcome, BridgeSpriteBlitError> {
    blit_raw_sprite(
        entity,
        selection,
        resource_bytes,
        framebuffer,
        Some(remap_tables),
    )
}

/// Rasterize every pixel of a raw opaque sprite.
///
/// This translates `sprite_blit_raw_opaque` at BLOODPRG routine offset
/// `0x004BA8`. It shares the original clipping, flip, and advanced-cursor
/// geometry with the transparent path, but always writes the source pixel.
pub fn blit_raw_opaque_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
) -> Result<BridgeSpriteBlitOutcome, BridgeSpriteBlitError> {
    blit_raw_sprite(entity, selection, resource_bytes, framebuffer, None)
}

/// Decode a transparent RLE sprite into the indexed framebuffer.
///
/// This translates `sprite_blit_rle_transparent` at BLOODPRG routine offset
/// `0x0046BC`. It retains run clipping, encoded-row skipping, canonical flips,
/// source-zero transparency, destination remapping, and the cursor-relative
/// X-origin reload without native pointer or counter wrapping.
pub fn blit_rle_transparent_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
    remap_tables: BridgeSpriteRemapTables<'_>,
) -> Result<BridgeSpriteRleBlitOutcome, BridgeSpriteBlitError> {
    blit_rle_sprite(
        entity,
        selection,
        resource_bytes,
        framebuffer,
        Some(remap_tables),
    )
}

/// Decode an opaque RLE sprite into the indexed framebuffer.
///
/// This translates `sprite_blit_rle_opaque` at BLOODPRG routine offset
/// `0x004CD6`. Every selected decoded pixel is written, including zeroes.
pub fn blit_rle_opaque_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
) -> Result<BridgeSpriteRleBlitOutcome, BridgeSpriteBlitError> {
    blit_rle_sprite(entity, selection, resource_bytes, framebuffer, None)
}

fn blit_rle_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
    remap_tables: Option<BridgeSpriteRemapTables<'_>>,
) -> Result<BridgeSpriteRleBlitOutcome, BridgeSpriteBlitError> {
    let frame = entity.frame.ok_or(BridgeSpriteBlitError::MissingFrame)?;
    let dirty_region = entity
        .dirty_region
        .ok_or(BridgeSpriteBlitError::MissingDirtyRegion)?;
    let encoded_start = frame
        .byte_offset
        .checked_add(FRAME_HEADER_BYTE_COUNT)
        .ok_or(BridgeSpriteBlitError::TruncatedFrame)?;
    if encoded_start > resource_bytes.len() {
        return Err(BridgeSpriteBlitError::TruncatedFrame);
    }

    let frame_stride = usize::from(read_u16(
        resource_bytes,
        frame.byte_offset + FRAME_STRIDE_OFFSET,
    ));
    let frame_height = usize::from(read_u16(
        resource_bytes,
        frame.byte_offset + FRAME_HEIGHT_OFFSET,
    ));
    let frame_x_origin = read_i16(resource_bytes, frame.byte_offset + FRAME_X_ORIGIN_OFFSET);
    let frame_y_origin = read_i16(resource_bytes, frame.byte_offset + FRAME_Y_ORIGIN_OFFSET);
    let mut draw_width = usize::from(entity.extent.width);
    let mut draw_height = usize::from(entity.extent.height);
    if frame_stride == usize::MIN
        || frame_height == usize::MIN
        || draw_width == usize::MIN
        || draw_height == usize::MIN
    {
        return Err(BridgeSpriteBlitError::EmptyExtent);
    }

    let sprite_top = signed_wrapping_sum(entity.draw_position.y, frame_y_origin, u16::MIN);
    let sprite_right =
        signed_wrapping_sum(entity.draw_position.x, frame_x_origin, entity.extent.width);
    let sprite_bottom =
        signed_wrapping_sum(entity.draw_position.y, frame_y_origin, entity.extent.height);
    let mut destination_y = sprite_top;
    let mut encoded_position = encoded_start;
    let mut skipped_rows = usize::MIN;

    if sprite_top < dirty_region.top {
        let clipped = usize::try_from(dirty_region.top - sprite_top)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_height = draw_height
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if !selection.flip_vertical {
            for _ in 0..clipped {
                encoded_position = skip_rle_row(resource_bytes, encoded_position, frame_stride)?;
                skipped_rows += 1;
            }
        }
        destination_y = dirty_region.top;
    }
    if sprite_bottom > dirty_region.bottom {
        let clipped = usize::try_from(sprite_bottom - dirty_region.bottom)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_height = draw_height
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if selection.flip_vertical {
            for _ in 0..clipped {
                encoded_position = skip_rle_row(resource_bytes, encoded_position, frame_stride)?;
                skipped_rows += 1;
            }
        }
    }
    if skipped_rows
        .checked_add(draw_height)
        .is_none_or(|rows| rows > frame_height)
    {
        return Err(BridgeSpriteBlitError::SourceOutsideFrame);
    }

    let encoded_cursor = encoded_position - encoded_start;
    let cursor_origin_offset = frame
        .byte_offset
        .checked_add(encoded_cursor)
        .and_then(|offset| offset.checked_add(FRAME_X_ORIGIN_OFFSET))
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    let cursor_origin_end = cursor_origin_offset
        .checked_add(size_of::<u16>())
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    if cursor_origin_end > resource_bytes.len() {
        return Err(BridgeSpriteBlitError::SourceOutsideFrame);
    }
    let cursor_x_origin = read_i16(resource_bytes, cursor_origin_offset);
    let sprite_left = signed_wrapping_sum(entity.draw_position.x, cursor_x_origin, u16::MIN);
    let mut destination_x = sprite_left;
    let mut left_clip = usize::MIN;
    let mut right_clip = usize::MIN;
    if sprite_left < dirty_region.left {
        left_clip = usize::try_from(dirty_region.left - sprite_left)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_width = draw_width
            .checked_sub(left_clip)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        destination_x = dirty_region.left;
    }
    if sprite_right >= dirty_region.right {
        right_clip = usize::try_from(sprite_right - dirty_region.right)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_width = draw_width
            .checked_sub(right_clip)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
    }
    if draw_width == usize::MIN || draw_height == usize::MIN {
        return Err(BridgeSpriteBlitError::ClipOutsideEntity);
    }

    if selection.flip_vertical {
        destination_y += i32::try_from(draw_height - 1)
            .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    }
    if selection.flip_horizontal {
        destination_x += i32::try_from(draw_width - 1)
            .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    }
    validate_destination(
        destination_x,
        destination_y,
        draw_width,
        draw_height,
        selection,
        framebuffer.len(),
    )?;

    let visible_start = if selection.flip_horizontal {
        right_clip
    } else {
        left_clip
    };
    let visible_end = visible_start
        .checked_add(draw_width)
        .ok_or(BridgeSpriteBlitError::MalformedRleRow)?;
    if visible_end > frame_stride {
        return Err(BridgeSpriteBlitError::WidthExceedsStride {
            width: visible_end,
            stride: frame_stride,
        });
    }

    let remap = remap_tables.map(|_| select_remap(entity.flags.bits()));
    for row in 0..draw_height {
        let (decoded_row, next_position) =
            decode_rle_row(resource_bytes, encoded_position, frame_stride)?;
        encoded_position = next_position;
        let destination_row = if selection.flip_vertical {
            destination_y - row as i32
        } else {
            destination_y + row as i32
        };
        for (column, source_pixel) in decoded_row[visible_start..visible_end]
            .iter()
            .copied()
            .enumerate()
        {
            let destination_column = if selection.flip_horizontal {
                destination_x - column as i32
            } else {
                destination_x + column as i32
            };
            let destination_index =
                destination_row as usize * LOGICAL_FRAMEBUFFER_WIDTH + destination_column as usize;
            match remap {
                None => framebuffer[destination_index] = source_pixel,
                Some(BridgeSpriteRemapSelection::Direct) if source_pixel != u8::MIN => {
                    framebuffer[destination_index] = source_pixel;
                }
                Some(BridgeSpriteRemapSelection::FirstDestinationTable)
                    if source_pixel != u8::MIN =>
                {
                    let tables = remap_tables.expect("transparent blit supplied remap tables");
                    framebuffer[destination_index] =
                        tables.first[usize::from(framebuffer[destination_index])];
                }
                Some(BridgeSpriteRemapSelection::SecondDestinationTable)
                    if source_pixel != u8::MIN =>
                {
                    let tables = remap_tables.expect("transparent blit supplied remap tables");
                    framebuffer[destination_index] =
                        tables.second[usize::from(framebuffer[destination_index])];
                }
                Some(_) => {}
            }
        }
    }

    Ok(BridgeSpriteRleBlitOutcome {
        clipped_extent: [draw_width, draw_height],
        skipped_rows,
        cursor_x_origin,
        source_clips: [left_clip, right_clip],
        destination_start: [destination_x, destination_y],
        remap,
    })
}

fn skip_rle_row(
    bytes: &[u8],
    position: usize,
    stride: usize,
) -> Result<usize, BridgeSpriteBlitError> {
    decode_rle_row(bytes, position, stride).map(|(_row, next)| next)
}

fn decode_rle_row(
    bytes: &[u8],
    mut position: usize,
    stride: usize,
) -> Result<(Vec<u8>, usize), BridgeSpriteBlitError> {
    let mut decoded = Vec::with_capacity(stride);
    while decoded.len() < stride {
        let control = *bytes
            .get(position)
            .ok_or(BridgeSpriteBlitError::MalformedRleRow)? as i8;
        position += 1;
        let run_length = if control < 0 {
            usize::from((-i16::from(control) + 1) as u16)
        } else {
            usize::from(control as u8) + 1
        };
        if decoded
            .len()
            .checked_add(run_length)
            .is_none_or(|end| end > stride)
        {
            return Err(BridgeSpriteBlitError::MalformedRleRow);
        }
        if control < 0 {
            let pixel = *bytes
                .get(position)
                .ok_or(BridgeSpriteBlitError::MalformedRleRow)?;
            position += 1;
            decoded.resize(decoded.len() + run_length, pixel);
        } else {
            let literal_end = position
                .checked_add(run_length)
                .ok_or(BridgeSpriteBlitError::MalformedRleRow)?;
            let literal = bytes
                .get(position..literal_end)
                .ok_or(BridgeSpriteBlitError::MalformedRleRow)?;
            decoded.extend_from_slice(literal);
            position = literal_end;
        }
    }
    Ok((decoded, position))
}

fn blit_raw_sprite(
    entity: &BridgeSpriteEntity,
    selection: BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
    remap_tables: Option<BridgeSpriteRemapTables<'_>>,
) -> Result<BridgeSpriteBlitOutcome, BridgeSpriteBlitError> {
    let frame = entity.frame.ok_or(BridgeSpriteBlitError::MissingFrame)?;
    let dirty_region = entity
        .dirty_region
        .ok_or(BridgeSpriteBlitError::MissingDirtyRegion)?;
    let frame_header_end = frame
        .byte_offset
        .checked_add(FRAME_HEADER_BYTE_COUNT)
        .ok_or(BridgeSpriteBlitError::TruncatedFrame)?;
    if frame_header_end > resource_bytes.len() {
        return Err(BridgeSpriteBlitError::TruncatedFrame);
    }

    let frame_stride = usize::from(read_u16(
        resource_bytes,
        frame.byte_offset + FRAME_STRIDE_OFFSET,
    ));
    let frame_height = usize::from(read_u16(
        resource_bytes,
        frame.byte_offset + FRAME_HEIGHT_OFFSET,
    ));
    let frame_x_origin = read_i16(resource_bytes, frame.byte_offset + FRAME_X_ORIGIN_OFFSET);
    let frame_y_origin = read_i16(resource_bytes, frame.byte_offset + FRAME_Y_ORIGIN_OFFSET);
    let frame_pixel_byte_count = frame_stride
        .checked_mul(frame_height)
        .ok_or(BridgeSpriteBlitError::TruncatedFrame)?;
    let frame_pixel_end = frame_header_end
        .checked_add(frame_pixel_byte_count)
        .ok_or(BridgeSpriteBlitError::TruncatedFrame)?;
    if frame_pixel_end > resource_bytes.len() {
        return Err(BridgeSpriteBlitError::TruncatedFrame);
    }

    let mut draw_width = usize::from(entity.extent.width);
    let mut draw_height = usize::from(entity.extent.height);
    if draw_width == usize::MIN || draw_height == usize::MIN {
        return Err(BridgeSpriteBlitError::EmptyExtent);
    }

    let sprite_top = signed_wrapping_sum(entity.draw_position.y, frame_y_origin, u16::MIN);
    let sprite_right =
        signed_wrapping_sum(entity.draw_position.x, frame_x_origin, entity.extent.width);
    let sprite_bottom =
        signed_wrapping_sum(entity.draw_position.y, frame_y_origin, entity.extent.height);
    let mut destination_y = sprite_top;
    let mut source_cursor = usize::MIN;

    if sprite_top < dirty_region.top {
        let clipped = usize::try_from(dirty_region.top - sprite_top)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_height = draw_height
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if !selection.flip_vertical {
            source_cursor = source_cursor
                .checked_add(
                    clipped
                        .checked_mul(frame_stride)
                        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?,
                )
                .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
        }
        destination_y = dirty_region.top;
    }
    if sprite_bottom >= dirty_region.bottom {
        let clipped = usize::try_from(sprite_bottom - dirty_region.bottom)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_height = draw_height
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if selection.flip_vertical {
            source_cursor = source_cursor
                .checked_add(
                    clipped
                        .checked_mul(frame_stride)
                        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?,
                )
                .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
        }
    }

    let cursor_origin_offset = frame
        .byte_offset
        .checked_add(source_cursor)
        .and_then(|offset| offset.checked_add(FRAME_X_ORIGIN_OFFSET))
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    let cursor_origin_end = cursor_origin_offset
        .checked_add(size_of::<u16>())
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    if cursor_origin_end > frame_pixel_end {
        return Err(BridgeSpriteBlitError::SourceOutsideFrame);
    }
    let cursor_x_origin = read_i16(resource_bytes, cursor_origin_offset);
    let sprite_left = signed_wrapping_sum(entity.draw_position.x, cursor_x_origin, u16::MIN);
    let mut destination_x = sprite_left;
    if sprite_left < dirty_region.left {
        let clipped = usize::try_from(dirty_region.left - sprite_left)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_width = draw_width
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if !selection.flip_horizontal {
            source_cursor = source_cursor
                .checked_add(clipped)
                .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
        }
        destination_x = dirty_region.left;
    }
    if sprite_right >= dirty_region.right {
        let clipped = usize::try_from(sprite_right - dirty_region.right)
            .map_err(|_| BridgeSpriteBlitError::ClipOutsideEntity)?;
        draw_width = draw_width
            .checked_sub(clipped)
            .ok_or(BridgeSpriteBlitError::ClipOutsideEntity)?;
        if selection.flip_horizontal {
            source_cursor = source_cursor
                .checked_add(clipped)
                .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
        }
    }
    if draw_width == usize::MIN || draw_height == usize::MIN {
        return Err(BridgeSpriteBlitError::ClipOutsideEntity);
    }
    if draw_width > frame_stride {
        return Err(BridgeSpriteBlitError::WidthExceedsStride {
            width: draw_width,
            stride: frame_stride,
        });
    }

    if selection.flip_vertical {
        destination_y += i32::try_from(draw_height - 1)
            .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    }
    if selection.flip_horizontal {
        destination_x += i32::try_from(draw_width - 1)
            .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    }
    validate_destination(
        destination_x,
        destination_y,
        draw_width,
        draw_height,
        selection,
        framebuffer.len(),
    )?;

    let source_start = frame_header_end
        .checked_add(source_cursor)
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    let final_source_end = source_start
        .checked_add(
            (draw_height - 1)
                .checked_mul(frame_stride)
                .and_then(|offset| offset.checked_add(draw_width))
                .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?,
        )
        .ok_or(BridgeSpriteBlitError::SourceOutsideFrame)?;
    if final_source_end > frame_pixel_end {
        return Err(BridgeSpriteBlitError::SourceOutsideFrame);
    }

    let remap = remap_tables.map(|_| select_remap(entity.flags.bits()));
    for row in 0..draw_height {
        let source_row = source_start + row * frame_stride;
        let destination_row = if selection.flip_vertical {
            destination_y - row as i32
        } else {
            destination_y + row as i32
        };
        for column in 0..draw_width {
            let source_pixel = resource_bytes[source_row + column];
            let destination_column = if selection.flip_horizontal {
                destination_x - column as i32
            } else {
                destination_x + column as i32
            };
            let destination_index =
                destination_row as usize * LOGICAL_FRAMEBUFFER_WIDTH + destination_column as usize;
            match remap {
                None => framebuffer[destination_index] = source_pixel,
                Some(BridgeSpriteRemapSelection::Direct) if source_pixel != u8::MIN => {
                    framebuffer[destination_index] = source_pixel;
                }
                Some(BridgeSpriteRemapSelection::FirstDestinationTable)
                    if source_pixel != u8::MIN =>
                {
                    let tables = remap_tables.expect("transparent blit supplied remap tables");
                    framebuffer[destination_index] =
                        tables.first[usize::from(framebuffer[destination_index])];
                }
                Some(BridgeSpriteRemapSelection::SecondDestinationTable)
                    if source_pixel != u8::MIN =>
                {
                    let tables = remap_tables.expect("transparent blit supplied remap tables");
                    framebuffer[destination_index] =
                        tables.second[usize::from(framebuffer[destination_index])];
                }
                Some(_) => {}
            }
        }
    }

    Ok(BridgeSpriteBlitOutcome {
        clipped_extent: [draw_width, draw_height],
        source_start_pixel: source_cursor,
        destination_start: [destination_x, destination_y],
        remap,
    })
}

fn validate_destination(
    destination_x: i32,
    destination_y: i32,
    draw_width: usize,
    draw_height: usize,
    selection: BridgeSpriteBlitterSelection,
    framebuffer_len: usize,
) -> Result<(), BridgeSpriteBlitError> {
    let required = LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
    if framebuffer_len < required {
        return Err(BridgeSpriteBlitError::FramebufferTooShort {
            actual: framebuffer_len,
        });
    }
    let width = i32::try_from(draw_width)
        .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    let height = i32::try_from(draw_height)
        .map_err(|_| BridgeSpriteBlitError::DestinationOutsideFramebuffer)?;
    let left = if selection.flip_horizontal {
        destination_x - width + 1
    } else {
        destination_x
    };
    let right = if selection.flip_horizontal {
        destination_x + 1
    } else {
        destination_x + width
    };
    let top = if selection.flip_vertical {
        destination_y - height + 1
    } else {
        destination_y
    };
    let bottom = if selection.flip_vertical {
        destination_y + 1
    } else {
        destination_y + height
    };
    if left < 0
        || top < 0
        || right > LOGICAL_FRAMEBUFFER_WIDTH as i32
        || bottom > LOGICAL_FRAMEBUFFER_HEIGHT as i32
    {
        return Err(BridgeSpriteBlitError::DestinationOutsideFramebuffer);
    }
    Ok(())
}

const fn select_remap(flags: u16) -> BridgeSpriteRemapSelection {
    match (flags >> DESTINATION_REMAP_SHIFT) & DESTINATION_REMAP_MASK {
        DIRECT_COLOR_MODE => BridgeSpriteRemapSelection::Direct,
        FIRST_DESTINATION_REMAP_MODE => BridgeSpriteRemapSelection::FirstDestinationTable,
        _ => BridgeSpriteRemapSelection::SecondDestinationTable,
    }
}

fn signed_wrapping_sum(base: u16, signed_offset: i16, extent: u16) -> i32 {
    i32::from(base.wrapping_add(signed_offset as u16).wrapping_add(extent) as i16)
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{
        BridgeSpriteBlitterMode, BridgeSpriteExtent, BridgeSpriteFlags, BridgeSpriteFrameReference,
        BridgeSpritePosition, BridgeSpriteRect, ResourceId,
    };

    const RAW_BLITTER_ORACLE_COUNT: usize = 10;
    const NONCANONICAL_FLIP_ORACLE_COUNT: usize = 2;
    const RLE_BLITTER_ORACLE_COUNT: usize = 10;
    const RLE_NONCANONICAL_FLIP_ORACLE_COUNT: usize = 1;
    const FRAME_HEIGHT_PADDING: usize = 3;
    const MINIMUM_FRAME_HEIGHT: usize = 8;
    const PIXEL_ROW_MULTIPLIER: usize = 29;
    const PIXEL_COLUMN_MULTIPLIER: usize = 17;
    const PIXEL_CASE_MULTIPLIER: usize = 13;
    const TRANSPARENT_PATTERN_DIVISOR: usize = 4;
    const RLE_FRAME_HEIGHT_PADDING: usize = 2;
    const MINIMUM_RLE_FRAME_HEIGHT: usize = 6;
    const RLE_PIXEL_ROW_MULTIPLIER: usize = 37;
    const RLE_PIXEL_COLUMN_MULTIPLIER: usize = 19;
    const RLE_PIXEL_CASE_MULTIPLIER: usize = 11;
    const RLE_TRANSPARENT_PATTERN_DIVISOR: usize = 5;
    const RLE_SPLIT_COLUMN: usize = 4;
    const RLE_REPEAT_LENGTH: usize = 4;
    const FRAMEBUFFER_INDEX_MULTIPLIER: usize = 13;
    const FRAMEBUFFER_CASE_MULTIPLIER: usize = 31;
    const FRAMEBUFFER_BASE: usize = 7;
    const FIRST_REMAP_MAXIMUM: usize = 255;
    const SECOND_REMAP_MULTIPLIER: usize = 3;
    const SECOND_REMAP_COLOR_OFFSET: usize = 11;
    const DIRECT_REMAP_OFFSET: u16 = 0;
    const FIRST_REMAP_OFFSET: u16 = 24_337;
    const SECOND_REMAP_TABLE_NATIVE_OFFSET: u16 = 24_593;
    const CLIP_ALL_EDGES_CASE: &str = "clip_all_edges";
    const TRANSPARENT_BOTH_FLIPS_CASE: &str = "both_flips_and_remap_6011";
    const OPAQUE_BOTH_FLIPS_CASE: &str = "both_flips";
    const ADVANCED_CLIP_X_ORIGIN: i16 = 2;
    const MALFORMED_RLE_STRIDE: u16 = 4;
    const MALFORMED_RLE_HEIGHT: u16 = 1;
    const SYNTHETIC_RESOURCE_ID: ResourceId = ResourceId::new(1);

    #[derive(Deserialize)]
    struct RawBlitterOracle {
        name: String,
        flags: u16,
        flip_bytes: [u8; 2],
        draw: [u16; 2],
        extent: [u16; 2],
        frame_origin_offset: [i16; 2],
        dirty_rect: [u16; 4],
        frame_stride: u16,
        clipped_extent: [usize; 2],
        source_start_pixel: usize,
        destination_start: [i32; 2],
        selected_remap_offset: u16,
        changed_pixels: Vec<[usize; 4]>,
    }

    #[derive(Deserialize)]
    struct RleBlitterOracle {
        name: String,
        flags: u16,
        flip_bytes: [u8; 2],
        draw: [u16; 2],
        extent: [u16; 2],
        frame_origin_offset: [i16; 2],
        dirty_rect: [u16; 4],
        row_kind: String,
        encoded_bytes: usize,
        skipped_rows: usize,
        cursor_x_offset: i16,
        clipped_extent: [usize; 2],
        source_clips: [usize; 2],
        destination_start: [i32; 2],
        changed_pixels: Vec<Vec<usize>>,
        #[serde(default)]
        selected_remap_offset: Option<u16>,
    }

    #[test]
    fn raw_transparent_blitter_matches_every_canonical_original_vector() {
        run_raw_oracles(
            include_str!("../../../../../re/tools/oracle_vectors/func_4536_natural.json"),
            true,
        );
    }

    #[test]
    fn raw_opaque_blitter_matches_every_canonical_original_vector() {
        run_raw_oracles(
            include_str!("../../../../../re/tools/oracle_vectors/func_4ba8_natural.json"),
            false,
        );
    }

    #[test]
    fn rle_transparent_blitter_matches_every_canonical_original_vector() {
        run_rle_oracles(
            include_str!("../../../../../re/tools/oracle_vectors/func_46bc_natural.json"),
            true,
        );
    }

    #[test]
    fn rle_opaque_blitter_matches_every_canonical_original_vector() {
        run_rle_oracles(
            include_str!("../../../../../re/tools/oracle_vectors/func_4cd6_natural.json"),
            false,
        );
    }

    #[test]
    fn malformed_flat_inputs_are_rejected_before_writing_pixels() {
        let remap = [u8::MIN; PALETTE_ENTRY_COUNT];
        let mut framebuffer = vec![7; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
        let before = framebuffer.clone();
        let entity = BridgeSpriteEntity::default();
        assert_eq!(
            blit_raw_transparent_sprite(
                &entity,
                canonical_selection([u8::MIN, u8::MIN]),
                &[],
                &mut framebuffer,
                BridgeSpriteRemapTables {
                    first: &remap,
                    second: &remap,
                },
            ),
            Err(BridgeSpriteBlitError::MissingFrame)
        );
        assert_eq!(framebuffer, before);

        let mut malformed_rle = Vec::new();
        malformed_rle.extend_from_slice(&MALFORMED_RLE_STRIDE.to_le_bytes());
        malformed_rle.extend_from_slice(&MALFORMED_RLE_HEIGHT.to_le_bytes());
        malformed_rle.extend_from_slice(&0_i16.to_le_bytes());
        malformed_rle.extend_from_slice(&0_i16.to_le_bytes());
        malformed_rle.push((MALFORMED_RLE_STRIDE - 1) as u8);
        let entity = BridgeSpriteEntity {
            frame: Some(BridgeSpriteFrameReference {
                resource: SYNTHETIC_RESOURCE_ID,
                frame_index: usize::MIN,
                byte_offset: usize::MIN,
            }),
            extent: BridgeSpriteExtent {
                width: MALFORMED_RLE_STRIDE,
                height: MALFORMED_RLE_HEIGHT,
            },
            dirty_region: Some(BridgeSpriteRect {
                left: 0,
                right: i32::from(MALFORMED_RLE_STRIDE),
                top: 0,
                bottom: i32::from(MALFORMED_RLE_HEIGHT),
            }),
            ..BridgeSpriteEntity::default()
        };
        assert_eq!(
            blit_rle_opaque_sprite(
                &entity,
                canonical_rle_selection([u8::MIN, u8::MIN], false),
                &malformed_rle,
                &mut framebuffer,
            ),
            Err(BridgeSpriteBlitError::MalformedRleRow)
        );
        assert_eq!(framebuffer, before);
    }

    fn run_raw_oracles(input: &str, transparent: bool) {
        let vectors: Vec<RawBlitterOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), RAW_BLITTER_ORACLE_COUNT);
        let mut noncanonical_flip_count = 0;

        for (case_index, vector) in vectors.iter().enumerate() {
            if vector.flip_bytes.iter().any(|value| *value > 1) {
                noncanonical_flip_count += 1;
                continue;
            }
            let resource = synthetic_frame(vector, case_index);
            let entity = synthetic_entity(vector);
            let first_remap = std::array::from_fn(|index| (FIRST_REMAP_MAXIMUM - index) as u8);
            let second_remap = std::array::from_fn(|index| {
                (index * SECOND_REMAP_MULTIPLIER + SECOND_REMAP_COLOR_OFFSET) as u8
            });
            let mut framebuffer = synthetic_framebuffer(case_index);
            let mut expected = framebuffer.clone();
            for change in &vector.changed_pixels {
                expected[change[0]] = change[3] as u8;
            }

            let outcome = if transparent {
                blit_raw_transparent_sprite(
                    &entity,
                    canonical_selection(vector.flip_bytes),
                    &resource,
                    &mut framebuffer,
                    BridgeSpriteRemapTables {
                        first: &first_remap,
                        second: &second_remap,
                    },
                )
            } else {
                blit_raw_opaque_sprite(
                    &entity,
                    canonical_selection(vector.flip_bytes),
                    &resource,
                    &mut framebuffer,
                )
            }
            .unwrap();

            assert_eq!(framebuffer, expected, "{}", vector.name);
            assert_eq!(
                outcome.clipped_extent, vector.clipped_extent,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.source_start_pixel, vector.source_start_pixel,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.destination_start, vector.destination_start,
                "{}",
                vector.name
            );
            let expected_remap = transparent.then(|| match vector.selected_remap_offset {
                DIRECT_REMAP_OFFSET => BridgeSpriteRemapSelection::Direct,
                FIRST_REMAP_OFFSET => BridgeSpriteRemapSelection::FirstDestinationTable,
                SECOND_REMAP_TABLE_NATIVE_OFFSET => {
                    BridgeSpriteRemapSelection::SecondDestinationTable
                }
                _ => panic!("unknown remap offset in {}", vector.name),
            });
            assert_eq!(outcome.remap, expected_remap, "{}", vector.name);
        }
        assert_eq!(
            noncanonical_flip_count, NONCANONICAL_FLIP_ORACLE_COUNT,
            "oracle suite changed its noncanonical ABI-only domain"
        );
    }

    fn run_rle_oracles(input: &str, transparent: bool) {
        let vectors: Vec<RleBlitterOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), RLE_BLITTER_ORACLE_COUNT);
        let mut noncanonical_flip_count = 0;

        for (case_index, vector) in vectors.iter().enumerate() {
            if vector.flip_bytes.iter().any(|value| *value > 1) {
                noncanonical_flip_count += 1;
                continue;
            }
            let resource = synthetic_rle_frame(vector, case_index);
            let entity = synthetic_rle_entity(vector);
            let first_remap = std::array::from_fn(|index| (FIRST_REMAP_MAXIMUM - index) as u8);
            let second_remap = std::array::from_fn(|index| {
                (index * SECOND_REMAP_MULTIPLIER + SECOND_REMAP_COLOR_OFFSET) as u8
            });
            let mut framebuffer = synthetic_framebuffer(case_index);
            let mut expected = framebuffer.clone();
            for change in &vector.changed_pixels {
                let after_index = if transparent { 3 } else { 2 };
                expected[change[0]] = change[after_index] as u8;
            }

            let outcome = if transparent {
                blit_rle_transparent_sprite(
                    &entity,
                    canonical_rle_selection(vector.flip_bytes, true),
                    &resource,
                    &mut framebuffer,
                    BridgeSpriteRemapTables {
                        first: &first_remap,
                        second: &second_remap,
                    },
                )
            } else {
                blit_rle_opaque_sprite(
                    &entity,
                    canonical_rle_selection(vector.flip_bytes, false),
                    &resource,
                    &mut framebuffer,
                )
            }
            .unwrap();

            assert_eq!(framebuffer, expected, "{}", vector.name);
            assert_eq!(
                outcome.clipped_extent, vector.clipped_extent,
                "{}",
                vector.name
            );
            assert_eq!(outcome.skipped_rows, vector.skipped_rows, "{}", vector.name);
            assert_eq!(
                outcome.cursor_x_origin, vector.cursor_x_offset,
                "{}",
                vector.name
            );
            assert_eq!(outcome.source_clips, vector.source_clips, "{}", vector.name);
            assert_eq!(
                outcome.destination_start, vector.destination_start,
                "{}",
                vector.name
            );
            let expected_remap = vector.selected_remap_offset.map(|offset| match offset {
                DIRECT_REMAP_OFFSET => BridgeSpriteRemapSelection::Direct,
                FIRST_REMAP_OFFSET => BridgeSpriteRemapSelection::FirstDestinationTable,
                SECOND_REMAP_TABLE_NATIVE_OFFSET => {
                    BridgeSpriteRemapSelection::SecondDestinationTable
                }
                _ => panic!("unknown remap offset in {}", vector.name),
            });
            assert_eq!(outcome.remap, expected_remap, "{}", vector.name);
        }
        assert_eq!(
            noncanonical_flip_count, RLE_NONCANONICAL_FLIP_ORACLE_COUNT,
            "RLE oracle suite changed its noncanonical ABI-only domain"
        );
    }

    fn synthetic_frame(vector: &RawBlitterOracle, case_index: usize) -> Vec<u8> {
        let frame_height =
            (usize::from(vector.extent[1]) + FRAME_HEIGHT_PADDING).max(MINIMUM_FRAME_HEIGHT);
        let stride = usize::from(vector.frame_stride);
        let mut pixels = Vec::with_capacity(frame_height * stride);
        for row in 0..frame_height {
            for column in 0..stride {
                let mut value = (row * PIXEL_ROW_MULTIPLIER
                    + column * PIXEL_COLUMN_MULTIPLIER
                    + case_index * PIXEL_CASE_MULTIPLIER
                    + 1) as u8;
                if (row + column + case_index).is_multiple_of(TRANSPARENT_PATTERN_DIVISOR) {
                    value = u8::MIN;
                }
                pixels.push(value);
            }
        }
        if vector.name == CLIP_ALL_EDGES_CASE {
            pixels[FRAME_X_ORIGIN_OFFSET..FRAME_X_ORIGIN_OFFSET + size_of::<u16>()]
                .copy_from_slice(&ADVANCED_CLIP_X_ORIGIN.to_le_bytes());
        } else if vector.name == TRANSPARENT_BOTH_FLIPS_CASE
            || vector.name == OPAQUE_BOTH_FLIPS_CASE
        {
            let advanced_x_origin_index = FRAME_X_ORIGIN_OFFSET - 1;
            pixels[advanced_x_origin_index..advanced_x_origin_index + size_of::<u16>()]
                .copy_from_slice(&0_i16.to_le_bytes());
        }

        let mut frame = Vec::with_capacity(FRAME_HEADER_BYTE_COUNT + pixels.len());
        frame.extend_from_slice(&vector.frame_stride.to_le_bytes());
        frame.extend_from_slice(&(frame_height as u16).to_le_bytes());
        frame.extend_from_slice(&vector.frame_origin_offset[0].to_le_bytes());
        frame.extend_from_slice(&vector.frame_origin_offset[1].to_le_bytes());
        frame.extend_from_slice(&pixels);
        frame
    }

    fn synthetic_rle_frame(vector: &RleBlitterOracle, case_index: usize) -> Vec<u8> {
        let stride = usize::from(vector.extent[0]);
        let frame_height = (usize::from(vector.extent[1]) + RLE_FRAME_HEIGHT_PADDING)
            .max(MINIMUM_RLE_FRAME_HEIGHT);
        let mut encoded = Vec::new();
        for row in 0..frame_height {
            let mut values: Vec<u8> = (0..stride)
                .map(|column| {
                    (row * RLE_PIXEL_ROW_MULTIPLIER
                        + column * RLE_PIXEL_COLUMN_MULTIPLIER
                        + case_index * RLE_PIXEL_CASE_MULTIPLIER
                        + 1) as u8
                })
                .collect();
            for (column, value) in values.iter_mut().enumerate() {
                if (row + column + case_index).is_multiple_of(RLE_TRANSPARENT_PATTERN_DIVISOR) {
                    *value = u8::MIN;
                }
            }
            if vector.skipped_rows != usize::MIN {
                values[stride - FRAME_X_ORIGIN_OFFSET..stride - FRAME_HEIGHT_OFFSET]
                    .copy_from_slice(&vector.cursor_x_offset.to_le_bytes());
            }
            encode_rle_test_row(&mut encoded, &mut values, &vector.row_kind);
        }
        assert_eq!(encoded.len(), vector.encoded_bytes, "{}", vector.name);

        let mut frame = Vec::with_capacity(FRAME_HEADER_BYTE_COUNT + encoded.len());
        frame.extend_from_slice(&(stride as u16).to_le_bytes());
        frame.extend_from_slice(&(frame_height as u16).to_le_bytes());
        frame.extend_from_slice(&vector.frame_origin_offset[0].to_le_bytes());
        frame.extend_from_slice(&vector.frame_origin_offset[1].to_le_bytes());
        frame.extend_from_slice(&encoded);
        frame
    }

    fn encode_rle_test_row(encoded: &mut Vec<u8>, values: &mut [u8], kind: &str) {
        match kind {
            "literal" => encode_literal(encoded, values),
            "split_literal" => {
                encode_literal(encoded, &values[..RLE_SPLIT_COLUMN]);
                encode_literal(encoded, &values[RLE_SPLIT_COLUMN..]);
            }
            "mixed" => {
                values[1] = values[0];
                values[2] = values[0];
                encode_repeat(encoded, 3, values[0]);
                encode_literal(encoded, &values[3..]);
            }
            "repeat_prefix" => {
                let repeated = values[0];
                values[1..RLE_REPEAT_LENGTH].fill(repeated);
                encode_repeat(encoded, RLE_REPEAT_LENGTH, repeated);
                encode_literal(encoded, &values[RLE_REPEAT_LENGTH..]);
            }
            "literal_prefix" | "repeat_suffix" => {
                let repeated = values[RLE_SPLIT_COLUMN];
                values[RLE_SPLIT_COLUMN + 1..].fill(repeated);
                encode_literal(encoded, &values[..RLE_SPLIT_COLUMN]);
                encode_repeat(encoded, RLE_REPEAT_LENGTH, repeated);
            }
            "literal_suffix" => {
                let repeated = values[0];
                values[1..RLE_REPEAT_LENGTH].fill(repeated);
                encode_repeat(encoded, RLE_REPEAT_LENGTH, repeated);
                encode_literal(encoded, &values[RLE_REPEAT_LENGTH..]);
            }
            _ => panic!("unknown RLE test row kind {kind}"),
        }
    }

    fn encode_literal(encoded: &mut Vec<u8>, values: &[u8]) {
        encoded.push((values.len() - 1) as u8);
        encoded.extend_from_slice(values);
    }

    fn encode_repeat(encoded: &mut Vec<u8>, length: usize, value: u8) {
        encoded.push((1_i16 - length as i16) as i8 as u8);
        encoded.push(value);
    }

    fn synthetic_entity(vector: &RawBlitterOracle) -> BridgeSpriteEntity {
        BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(vector.flags),
            frame: Some(BridgeSpriteFrameReference {
                resource: SYNTHETIC_RESOURCE_ID,
                frame_index: usize::MIN,
                byte_offset: usize::MIN,
            }),
            draw_position: BridgeSpritePosition {
                x: vector.draw[0],
                y: vector.draw[1],
            },
            extent: BridgeSpriteExtent {
                width: vector.extent[0],
                height: vector.extent[1],
            },
            dirty_region: Some(rect(vector.dirty_rect)),
            ..BridgeSpriteEntity::default()
        }
    }

    fn synthetic_rle_entity(vector: &RleBlitterOracle) -> BridgeSpriteEntity {
        BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(vector.flags),
            frame: Some(BridgeSpriteFrameReference {
                resource: SYNTHETIC_RESOURCE_ID,
                frame_index: usize::MIN,
                byte_offset: usize::MIN,
            }),
            draw_position: BridgeSpritePosition {
                x: vector.draw[0],
                y: vector.draw[1],
            },
            extent: BridgeSpriteExtent {
                width: vector.extent[0],
                height: vector.extent[1],
            },
            dirty_region: Some(rect(vector.dirty_rect)),
            ..BridgeSpriteEntity::default()
        }
    }

    fn synthetic_framebuffer(case_index: usize) -> Vec<u8> {
        (0..LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT)
            .map(|index| {
                (index * FRAMEBUFFER_INDEX_MULTIPLIER
                    + case_index * FRAMEBUFFER_CASE_MULTIPLIER
                    + FRAMEBUFFER_BASE) as u8
            })
            .collect()
    }

    fn canonical_selection(flip_bytes: [u8; 2]) -> BridgeSpriteBlitterSelection {
        BridgeSpriteBlitterSelection {
            mode: BridgeSpriteBlitterMode::RawTransparent,
            flip_horizontal: flip_bytes[0] != u8::MIN,
            flip_vertical: flip_bytes[1] != u8::MIN,
        }
    }

    fn canonical_rle_selection(
        flip_bytes: [u8; 2],
        transparent: bool,
    ) -> BridgeSpriteBlitterSelection {
        BridgeSpriteBlitterSelection {
            mode: if transparent {
                BridgeSpriteBlitterMode::RleTransparent
            } else {
                BridgeSpriteBlitterMode::RleOpaque
            },
            flip_horizontal: flip_bytes[0] != u8::MIN,
            flip_vertical: flip_bytes[1] != u8::MIN,
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
}
