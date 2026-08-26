//! AD-family presentation payload expansion over flat owned buffers.

use std::error::Error;
use std::fmt;

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, PresentationDecodeError,
    decode_presentation_pair_lz,
};

const PAYLOAD_HEADER_BYTE_COUNT: usize = 6;
const OPTIONAL_PREFIX_BYTE_COUNT: usize = 4;
const FLAG_NO_PREFIX: u8 = 0x04;
const FLAG_HIGH_LITERAL_BIAS: u8 = 0x40;
const FLAG_HIGH_TOKEN_LAYOUT: u8 = 0x80;
const HIGH_LITERAL_BIAS: u8 = 0x80;
const CONTROL_INITIAL_MASK: u16 = 0x8000;
const VARIABLE_NIBBLE_LENGTH_BIAS: usize = 4;
const EXTENDED_LENGTH_BIAS: usize = 20;
const DESCRIPTOR_HIGH_NIBBLE_SHIFT: u32 = 4;
const DESCRIPTOR_LOW_NIBBLE_MASK: u8 = 0x0F;
const RECTANGLE_WIDTH_MASK: usize = 0x01FF;
const MAXIMUM_RECTANGLE_ROWS: usize = 130;
#[cfg(test)]
const MAXIMUM_STAGING_BYTE_COUNT: usize = u16::MAX as usize + 1;
const FRAMEBUFFER_BYTE_COUNT: usize = LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
const WORD_BYTE_COUNT: usize = size_of::<u16>();

/// Invalid AD-family source data or flat destination geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationAdError {
    /// The payload ended before a complete field or control value was available.
    SourceTruncated {
        /// Position of the failed read.
        position: usize,
        /// Bytes required by that read.
        required: usize,
        /// Remaining source bytes.
        available: usize,
    },
    /// The shared pair-LZ staging pass rejected its source or destination.
    StagingDecode(PresentationDecodeError),
    /// A staged tail begins before the owned output buffer.
    StagingExceedsOutput {
        /// Complete declared output extent.
        output_extent: usize,
        /// Tail extent requested for staged values.
        staging_extent: usize,
    },
    /// The caller did not provide the complete reusable staging region.
    StagingBufferTooSmall {
        /// Header-declared staging extent.
        required: usize,
        /// Available owned staging bytes.
        available: usize,
    },
    /// A main-stream token requires another staged value.
    StagedValuesExhausted {
        /// Requested flat staging position.
        position: usize,
        /// Exclusive staging boundary.
        end: usize,
    },
    /// A complete AD run would exceed the declared output extent.
    OutputExtentExceeded {
        /// Current output position.
        position: usize,
        /// Complete run length.
        length: usize,
        /// Exclusive declared boundary.
        end: usize,
    },
    /// A rectangular payload has no drawable columns.
    ZeroWidth,
    /// A rectangular payload has no drawable rows after native low-byte selection.
    ZeroRows,
    /// Rectangle coordinates do not fit the fixed 320 by 200 logical framebuffer.
    RectangleOutOfBounds {
        /// Left pixel coordinate.
        x: usize,
        /// Top pixel coordinate after the vertical presentation offset.
        y: usize,
        /// Masked rectangle width.
        width: usize,
        /// Clamped row count.
        rows: usize,
    },
}

impl fmt::Display for PresentationAdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid AD presentation payload: {self:?}")
    }
}

impl Error for PresentationAdError {}

impl From<PresentationDecodeError> for PresentationAdError {
    fn from(error: PresentationDecodeError) -> Self {
        Self::StagingDecode(error)
    }
}

/// Owned output and cursor accounting from one terminated AD payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationAdOutcome {
    /// Optional four-byte prefix followed by the complete decoded payload.
    pub bytes: Box<[u8]>,
    /// Source bytes consumed by the header, staging pass, and main pass.
    pub consumed_bytes: usize,
    /// Prefix bytes copied before decompression.
    pub prefix_bytes: usize,
    /// Header-declared decoded extent excluding the optional prefix.
    pub decoded_bytes: usize,
    /// Main-pass staged values consumed, including the ignored terminal value.
    pub staged_values_consumed: usize,
}

/// Observable result of one transparent rectangular AD payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRectDecodeOutcome {
    /// Source bytes consumed by the header, staging pass, and main pass.
    pub consumed_bytes: usize,
    /// Main-pass staged values consumed before the final row completed.
    pub staged_values_consumed: usize,
    /// Destination pixels whose indexed value changed.
    pub changed_pixels: usize,
    /// Left pixel coordinate selected by the optional payload coordinates.
    pub x: usize,
    /// Top pixel coordinate after applying the presentation vertical offset.
    pub y: usize,
    /// Nine-bit row width selected by the native format.
    pub width: usize,
    /// Low-byte row count after the native 130-row clamp.
    pub rows: usize,
    /// Start of the final decoded framebuffer row.
    pub final_row_offset: usize,
    /// Exclusive framebuffer position reached in the final row.
    pub final_destination_offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenLayout {
    Low,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PayloadHeader {
    first_extent: usize,
    second_extent: usize,
    flags: u8,
}

impl PayloadHeader {
    fn layout(self) -> TokenLayout {
        if self.flags & FLAG_HIGH_TOKEN_LAYOUT == 0 {
            TokenLayout::Low
        } else {
            TokenLayout::High
        }
    }

    fn literal_bias(self) -> u8 {
        if self.flags & FLAG_HIGH_LITERAL_BIAS == 0 {
            u8::MIN
        } else {
            HIGH_LITERAL_BIAS
        }
    }
}

fn truncated(source: &[u8], position: usize, required: usize) -> PresentationAdError {
    PresentationAdError::SourceTruncated {
        position,
        required,
        available: source.len().saturating_sub(position),
    }
}

fn read_byte(source: &[u8], cursor: &mut usize) -> Result<u8, PresentationAdError> {
    let value = source
        .get(*cursor)
        .copied()
        .ok_or_else(|| truncated(source, *cursor, size_of::<u8>()))?;
    *cursor += size_of::<u8>();
    Ok(value)
}

fn read_word(source: &[u8], cursor: &mut usize) -> Result<u16, PresentationAdError> {
    let end = (*cursor)
        .checked_add(WORD_BYTE_COUNT)
        .ok_or_else(|| truncated(source, *cursor, WORD_BYTE_COUNT))?;
    let bytes = source
        .get(*cursor..end)
        .ok_or_else(|| truncated(source, *cursor, WORD_BYTE_COUNT))?;
    *cursor = end;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("validated two-byte AD field"),
    ))
}

fn read_header(source: &[u8]) -> Result<PayloadHeader, PresentationAdError> {
    if source.len() < PAYLOAD_HEADER_BYTE_COUNT {
        return Err(truncated(source, usize::MIN, PAYLOAD_HEADER_BYTE_COUNT));
    }
    let mut cursor = usize::MIN;
    let first_extent = usize::from(read_word(source, &mut cursor)?);
    let second_extent = usize::from(read_word(source, &mut cursor)?);
    let flags = read_byte(source, &mut cursor)?;
    let _checksum = read_byte(source, &mut cursor)?;
    Ok(PayloadHeader {
        first_extent,
        second_extent,
        flags,
    })
}

struct MsbControlReader<'a> {
    source: &'a [u8],
    cursor: usize,
    control_word: u16,
    control_mask: u16,
}

impl<'a> MsbControlReader<'a> {
    fn new(source: &'a [u8], cursor: usize) -> Self {
        Self {
            source,
            cursor,
            control_word: u16::MIN,
            control_mask: u16::MIN,
        }
    }

    fn read_bit(&mut self) -> Result<bool, PresentationAdError> {
        if self.control_mask == u16::MIN {
            self.control_word = read_word(self.source, &mut self.cursor)?;
            self.control_mask = CONTROL_INITIAL_MASK;
        }
        let value = self.control_word & self.control_mask != u16::MIN;
        self.control_mask >>= 1;
        Ok(value)
    }

    fn read_byte(&mut self) -> Result<u8, PresentationAdError> {
        read_byte(self.source, &mut self.cursor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepeatedRun {
    End,
    Length(usize),
}

fn variable_run_length(
    reader: &mut MsbControlReader<'_>,
    pending_length: &mut usize,
) -> Result<usize, PresentationAdError> {
    if *pending_length > VARIABLE_NIBBLE_LENGTH_BIAS {
        let length = *pending_length;
        *pending_length = usize::MIN;
        return Ok(length);
    }
    if *pending_length == VARIABLE_NIBBLE_LENGTH_BIAS {
        let length = usize::from(reader.read_byte()?) + EXTENDED_LENGTH_BIAS;
        *pending_length = usize::MIN;
        return Ok(length);
    }

    let descriptor = reader.read_byte()?;
    let high_nibble = usize::from(descriptor >> DESCRIPTOR_HIGH_NIBBLE_SHIFT);
    let length = if high_nibble == usize::MIN {
        usize::from(reader.read_byte()?) + EXTENDED_LENGTH_BIAS
    } else {
        high_nibble + VARIABLE_NIBBLE_LENGTH_BIAS
    };
    *pending_length =
        usize::from(descriptor & DESCRIPTOR_LOW_NIBBLE_MASK) + VARIABLE_NIBBLE_LENGTH_BIAS;
    Ok(length)
}

fn repeated_run(
    reader: &mut MsbControlReader<'_>,
    layout: TokenLayout,
    output_at_end: bool,
    pending_length: &mut usize,
) -> Result<RepeatedRun, PresentationAdError> {
    let fixed_length = match layout {
        TokenLayout::High => {
            if !reader.read_bit()? {
                usize::MIN
            } else if !reader.read_bit()? {
                2
            } else if !reader.read_bit()? {
                3
            } else if output_at_end {
                return Ok(RepeatedRun::End);
            } else {
                4
            }
        }
        TokenLayout::Low => {
            if !reader.read_bit()? {
                2
            } else if !reader.read_bit()? {
                3
            } else if !reader.read_bit()? {
                4
            } else if output_at_end {
                return Ok(RepeatedRun::End);
            } else {
                usize::MIN
            }
        }
    };
    if fixed_length == usize::MIN {
        Ok(RepeatedRun::Length(variable_run_length(
            reader,
            pending_length,
        )?))
    } else {
        Ok(RepeatedRun::Length(fixed_length))
    }
}

fn decode_staging_pass(
    source: &[u8],
    cursor: &mut usize,
    destination: &mut [u8],
    output_start: usize,
    output_end: usize,
    literal_bias: u8,
    allow_empty_guard: bool,
) -> Result<(), PresentationAdError> {
    if output_start == output_end && allow_empty_guard {
        let mut guard = [u8::MIN; size_of::<u8>()];
        let guard_end = guard.len();
        let outcome = decode_presentation_pair_lz(
            source
                .get(*cursor..)
                .ok_or_else(|| truncated(source, *cursor, size_of::<u8>()))?,
            &mut guard,
            usize::MIN,
            guard_end,
            literal_bias,
        )?;
        *cursor = cursor.checked_add(outcome.consumed_bytes).ok_or(
            PresentationAdError::SourceTruncated {
                position: *cursor,
                required: outcome.consumed_bytes,
                available: source.len().saturating_sub(*cursor),
            },
        )?;
        return Ok(());
    }

    let compressed = source
        .get(*cursor..)
        .ok_or_else(|| truncated(source, *cursor, size_of::<u8>()))?;
    let outcome = decode_presentation_pair_lz(
        compressed,
        destination,
        output_start,
        output_end,
        literal_bias,
    )?;
    *cursor =
        cursor
            .checked_add(outcome.consumed_bytes)
            .ok_or(PresentationAdError::SourceTruncated {
                position: *cursor,
                required: outcome.consumed_bytes,
                available: source.len().saturating_sub(*cursor),
            })?;
    Ok(())
}

fn checked_run_end(
    position: usize,
    length: usize,
    end: usize,
) -> Result<usize, PresentationAdError> {
    position
        .checked_add(length)
        .filter(|run_end| *run_end <= end)
        .ok_or(PresentationAdError::OutputExtentExceeded {
            position,
            length,
            end,
        })
}

/// Decode one terminated AD presentation resource into an owned byte buffer.
///
/// This translates `resource_payload_decode_ad` at BLOODPRG offset `0x00A914`.
/// It retains the optional prefix, caller-selected staging bias, both run-code
/// layouts, pending nibble lengths, MSB-first controls, and in-buffer staged
/// overlap without far pointers or self-modifying literal-bias instructions.
pub fn decode_presentation_ad(source: &[u8]) -> Result<PresentationAdOutcome, PresentationAdError> {
    let header = read_header(source)?;
    if header.second_extent > header.first_extent {
        return Err(PresentationAdError::StagingExceedsOutput {
            output_extent: header.first_extent,
            staging_extent: header.second_extent,
        });
    }

    let prefix_bytes = if header.flags & FLAG_NO_PREFIX == 0 {
        OPTIONAL_PREFIX_BYTE_COUNT
    } else {
        usize::MIN
    };
    let mut cursor = PAYLOAD_HEADER_BYTE_COUNT;
    let prefix_end = cursor
        .checked_add(prefix_bytes)
        .ok_or_else(|| truncated(source, cursor, prefix_bytes))?;
    let prefix = source
        .get(cursor..prefix_end)
        .ok_or_else(|| truncated(source, cursor, prefix_bytes))?;
    cursor = prefix_end;

    let output_end = prefix_bytes.checked_add(header.first_extent).ok_or(
        PresentationAdError::OutputExtentExceeded {
            position: prefix_bytes,
            length: header.first_extent,
            end: usize::MAX,
        },
    )?;
    let staging_start = output_end - header.second_extent;
    let mut output = vec![u8::MIN; output_end];
    output[..prefix_bytes].copy_from_slice(prefix);
    decode_staging_pass(
        source,
        &mut cursor,
        &mut output,
        staging_start,
        output_end,
        header.literal_bias(),
        true,
    )?;

    let mut reader = MsbControlReader::new(source, cursor);
    let mut output_cursor = prefix_bytes;
    let mut staged_cursor = staging_start;
    let mut pending_length = usize::MIN;
    loop {
        if !reader.read_bit()? {
            let value = output.get(staged_cursor).copied().ok_or(
                PresentationAdError::StagedValuesExhausted {
                    position: staged_cursor,
                    end: output_end,
                },
            )?;
            staged_cursor += size_of::<u8>();
            let next = checked_run_end(output_cursor, size_of::<u8>(), output_end)?;
            output[output_cursor] = value;
            output_cursor = next;
            continue;
        }

        let staged_value = output.get(staged_cursor).copied();
        staged_cursor = staged_cursor.checked_add(size_of::<u8>()).ok_or(
            PresentationAdError::StagedValuesExhausted {
                position: staged_cursor,
                end: output_end,
            },
        )?;
        match repeated_run(
            &mut reader,
            header.layout(),
            output_cursor == output_end,
            &mut pending_length,
        )? {
            RepeatedRun::End => break,
            RepeatedRun::Length(length) => {
                let value = staged_value.ok_or(PresentationAdError::StagedValuesExhausted {
                    position: staged_cursor - size_of::<u8>(),
                    end: output_end,
                })?;
                let next = checked_run_end(output_cursor, length, output_end)?;
                output[output_cursor..next].fill(value);
                output_cursor = next;
            }
        }
    }

    Ok(PresentationAdOutcome {
        bytes: output.into_boxed_slice(),
        consumed_bytes: reader.cursor,
        prefix_bytes,
        decoded_bytes: header.first_extent,
        staged_values_consumed: staged_cursor - staging_start,
    })
}

fn rectangle_geometry(
    source: &[u8],
    cursor: &mut usize,
    header: PayloadHeader,
    vertical_offset: usize,
    raw_width: u16,
    raw_rows: u16,
    framebuffer_len: usize,
) -> Result<(usize, usize, usize, usize), PresentationAdError> {
    let (x, authored_y) = if header.flags & FLAG_NO_PREFIX == 0 {
        (
            usize::from(read_word(source, cursor)?),
            usize::from(read_word(source, cursor)?),
        )
    } else {
        (usize::MIN, usize::MIN)
    };
    let y = authored_y.checked_add(vertical_offset).ok_or(
        PresentationAdError::RectangleOutOfBounds {
            x,
            y: usize::MAX,
            width: usize::from(raw_width) & RECTANGLE_WIDTH_MASK,
            rows: usize::from(raw_rows as u8).min(MAXIMUM_RECTANGLE_ROWS),
        },
    )?;
    let width = usize::from(raw_width) & RECTANGLE_WIDTH_MASK;
    let rows = usize::from(raw_rows as u8).min(MAXIMUM_RECTANGLE_ROWS);
    if width == usize::MIN {
        return Err(PresentationAdError::ZeroWidth);
    }
    if rows == usize::MIN {
        return Err(PresentationAdError::ZeroRows);
    }
    let right = x.checked_add(width);
    let bottom = y.checked_add(rows);
    if right.is_none_or(|right| right > LOGICAL_FRAMEBUFFER_WIDTH)
        || bottom.is_none_or(|bottom| bottom > LOGICAL_FRAMEBUFFER_HEIGHT)
        || framebuffer_len < FRAMEBUFFER_BYTE_COUNT
    {
        return Err(PresentationAdError::RectangleOutOfBounds { x, y, width, rows });
    }
    Ok((x, y, width, rows))
}

/// Decode one transparent rectangular AD payload into the indexed framebuffer.
///
/// This translates `resource_payload_decode_rect` at offset `0x00AB25` and its
/// calls to the shared pair-LZ and scanline helpers. The caller supplies an
/// owned reusable staging buffer so valid back-references can use retained
/// history; all changes are committed only after the complete payload succeeds.
pub fn decode_presentation_rect(
    source: &[u8],
    staging: &mut [u8],
    framebuffer: &mut [u8],
    vertical_offset: usize,
    raw_width: u16,
    raw_rows: u16,
) -> Result<PresentationRectDecodeOutcome, PresentationAdError> {
    let header = read_header(source)?;
    if header.first_extent > staging.len() {
        return Err(PresentationAdError::StagingBufferTooSmall {
            required: header.first_extent,
            available: staging.len(),
        });
    }
    if header.second_extent == usize::MIN || header.second_extent > header.first_extent {
        return Err(PresentationAdError::StagingExceedsOutput {
            output_extent: header.first_extent,
            staging_extent: header.second_extent,
        });
    }

    let mut cursor = PAYLOAD_HEADER_BYTE_COUNT;
    let (x, y, width, rows) = rectangle_geometry(
        source,
        &mut cursor,
        header,
        vertical_offset,
        raw_width,
        raw_rows,
        framebuffer.len(),
    )?;
    let staging_start = header.first_extent - header.second_extent;
    let staging_end = header.first_extent;
    let mut staged_output = staging.to_vec();
    let mut rendered = framebuffer.to_vec();
    decode_staging_pass(
        source,
        &mut cursor,
        &mut staged_output,
        staging_start,
        staging_end,
        header.literal_bias(),
        false,
    )?;

    let mut reader = MsbControlReader::new(source, cursor);
    let mut staged_cursor = staging_start;
    let mut pending_length = usize::MIN;
    let mut rows_remaining = rows;
    let mut row_remaining = width;
    let mut row_offset = y * LOGICAL_FRAMEBUFFER_WIDTH + x;
    let mut destination = row_offset;
    let mut changed_pixels = usize::MIN;

    loop {
        let repeated = reader.read_bit()?;
        let value = staged_output.get(staged_cursor).copied().ok_or(
            PresentationAdError::StagedValuesExhausted {
                position: staged_cursor,
                end: staging_end,
            },
        )?;
        staged_cursor += size_of::<u8>();
        let mut length = if repeated {
            match repeated_run(&mut reader, header.layout(), false, &mut pending_length)? {
                RepeatedRun::Length(length) => length,
                RepeatedRun::End => unreachable!("rectangle streams stop at their final row"),
            }
        } else {
            size_of::<u8>()
        };

        while length != usize::MIN {
            let chunk = length.min(row_remaining);
            let chunk_end = destination + chunk;
            if value != u8::MIN {
                for pixel in &mut rendered[destination..chunk_end] {
                    changed_pixels += usize::from(*pixel != value);
                    *pixel = value;
                }
            }
            destination = chunk_end;
            row_remaining -= chunk;
            length -= chunk;
            if row_remaining != usize::MIN {
                continue;
            }

            rows_remaining -= 1;
            if rows_remaining == usize::MIN {
                staging.copy_from_slice(&staged_output);
                framebuffer.copy_from_slice(&rendered);
                return Ok(PresentationRectDecodeOutcome {
                    consumed_bytes: reader.cursor,
                    staged_values_consumed: staged_cursor - staging_start,
                    changed_pixels,
                    x,
                    y,
                    width,
                    rows,
                    final_row_offset: row_offset,
                    final_destination_offset: destination,
                });
            }
            row_offset += LOGICAL_FRAMEBUFFER_WIDTH;
            destination = row_offset;
            row_remaining = width;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::Value;

    use super::*;

    const AD_VECTOR_COUNT: usize = 9;
    const FLAT_AD_VECTOR_COUNT: usize = 8;
    const RECT_VECTOR_COUNT: usize = 8;
    const AD_CHECKSUM_TARGET: u8 = 0xAD;
    const STAGING_PATTERN_STEP: usize = 23;
    const STAGING_PATTERN_PAGE_STEP: usize = 11;
    const STAGING_PATTERN_CASE_STEP: usize = 31;
    const FRAMEBUFFER_PATTERN_STEP: usize = 37;
    const FRAMEBUFFER_PATTERN_PAGE_STEP: usize = 13;
    const FRAMEBUFFER_PATTERN_CASE_STEP: usize = 19;
    const OVERSHOOT_VECTOR_NAME: &str = "high_fixed_overshoot";

    #[derive(Deserialize)]
    struct AdOracle {
        name: String,
        flags: u8,
        prefix_hex: String,
        staged_values: Vec<u8>,
        staging_stream_hex: String,
        main_stream_hex: String,
        source_offset: u16,
        main_source_result_offset: u16,
        declared_output_extent: usize,
        actual_output_extent: usize,
        decoded_hex: String,
        staged_offset: u16,
        staged_result_offset: u16,
    }

    #[derive(Deserialize)]
    struct RectOracle {
        name: String,
        flags: u8,
        coordinates: Option<[u16; 2]>,
        vertical_offset: usize,
        raw_width: u16,
        row_width: usize,
        raw_rows: u16,
        rows: usize,
        tokens: Vec<Vec<Value>>,
        staged_values: Vec<u8>,
        staging_stream_hex: String,
        main_stream_hex: String,
        source_offset: u16,
        main_source_result_offset: u16,
        staging_offset: usize,
        staged_result_offset: u16,
        first_row_offset: usize,
        final_row_offset: usize,
        destination_result_offset: usize,
        changed_pixels: usize,
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

    fn checksum(first_extent: usize, second_extent: usize, flags: u8) -> u8 {
        let sum = (first_extent as u16)
            .to_le_bytes()
            .into_iter()
            .chain((second_extent as u16).to_le_bytes())
            .chain([flags])
            .fold(u8::MIN, u8::wrapping_add);
        AD_CHECKSUM_TARGET.wrapping_sub(sum)
    }

    fn resource(
        first_extent: usize,
        second_extent: usize,
        flags: u8,
        prefix: &[u8],
        staging_stream: &[u8],
        main_stream: &[u8],
    ) -> Vec<u8> {
        let mut source = Vec::new();
        source.extend_from_slice(&(first_extent as u16).to_le_bytes());
        source.extend_from_slice(&(second_extent as u16).to_le_bytes());
        source.push(flags);
        source.push(checksum(first_extent, second_extent, flags));
        source.extend_from_slice(prefix);
        source.extend_from_slice(staging_stream);
        source.extend_from_slice(main_stream);
        source
    }

    fn patterned_memory(
        length: usize,
        case_index: usize,
        step: usize,
        page_step: usize,
        case_step: usize,
    ) -> Vec<u8> {
        (usize::MIN..length)
            .map(|offset| {
                (offset * step + (offset >> u8::BITS) * page_step + case_index * case_step) as u8
            })
            .collect()
    }

    fn expected_rect_frame(vector: &RectOracle, case_index: usize) -> Vec<u8> {
        let mut expected = patterned_memory(
            FRAMEBUFFER_BYTE_COUNT,
            case_index,
            FRAMEBUFFER_PATTERN_STEP,
            FRAMEBUFFER_PATTERN_PAGE_STEP,
            FRAMEBUFFER_PATTERN_CASE_STEP,
        );
        let mut decoded = Vec::new();
        for (token, value) in vector.tokens.iter().zip(&vector.staged_values) {
            let length = if token.first().and_then(Value::as_str) == Some("literal") {
                size_of::<u8>()
            } else {
                token
                    .get(1)
                    .and_then(Value::as_u64)
                    .expect("oracle repeated token length") as usize
            };
            decoded.extend(std::iter::repeat_n(*value, length));
        }
        decoded.truncate(vector.row_width * vector.rows);
        let x = vector
            .coordinates
            .map_or(usize::MIN, |coordinates| usize::from(coordinates[0]));
        let authored_y = vector
            .coordinates
            .map_or(usize::MIN, |coordinates| usize::from(coordinates[1]));
        let y = authored_y + vector.vertical_offset;
        for (row_index, row) in decoded.chunks_exact(vector.row_width).enumerate() {
            let start = (y + row_index) * LOGICAL_FRAMEBUFFER_WIDTH + x;
            for (pixel, value) in expected[start..start + vector.row_width]
                .iter_mut()
                .zip(row)
            {
                if *value != u8::MIN {
                    *pixel = *value;
                }
            }
        }
        expected
    }

    #[test]
    fn ad_decoder_matches_flat_vectors_and_rejects_overshoot() {
        let vectors: Vec<AdOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a914_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), AD_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for vector in vectors {
            let prefix = decode_hex(&vector.prefix_hex);
            let staging_stream = decode_hex(&vector.staging_stream_hex);
            let main_stream = decode_hex(&vector.main_stream_hex);
            let source = resource(
                vector.declared_output_extent,
                vector.staged_values.len(),
                vector.flags,
                &prefix,
                &staging_stream,
                &main_stream,
            );
            let result = decode_presentation_ad(&source);
            if vector.name == OVERSHOOT_VECTOR_NAME {
                assert!(matches!(
                    result,
                    Err(PresentationAdError::OutputExtentExceeded { .. })
                ));
                continue;
            }

            let outcome = result.unwrap();
            let mut expected = prefix.clone();
            expected.extend_from_slice(&decode_hex(&vector.decoded_hex));
            assert_eq!(&*outcome.bytes, expected, "{}", vector.name);
            assert_eq!(outcome.prefix_bytes, prefix.len(), "{}", vector.name);
            assert_eq!(
                outcome.decoded_bytes, vector.actual_output_extent,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.consumed_bytes,
                usize::from(
                    vector
                        .main_source_result_offset
                        .wrapping_sub(vector.source_offset)
                ),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.staged_values_consumed,
                usize::from(
                    vector
                        .staged_result_offset
                        .wrapping_sub(vector.staged_offset)
                ),
                "{}",
                vector.name
            );
            matched += 1;
        }
        assert_eq!(matched, FLAT_AD_VECTOR_COUNT);
    }

    #[test]
    fn rectangle_decoder_matches_every_original_vector() {
        let vectors: Vec<RectOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_ab25_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RECT_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let coordinate_bytes = vector.coordinates.map_or_else(Vec::new, |coordinates| {
                coordinates.into_iter().flat_map(u16::to_le_bytes).collect()
            });
            let staging_stream = decode_hex(&vector.staging_stream_hex);
            let main_stream = decode_hex(&vector.main_stream_hex);
            let source = resource(
                vector.staged_values.len(),
                vector.staged_values.len(),
                vector.flags,
                &coordinate_bytes,
                &staging_stream,
                &main_stream,
            );
            let mut staging = patterned_memory(
                MAXIMUM_STAGING_BYTE_COUNT,
                case_index,
                STAGING_PATTERN_STEP,
                STAGING_PATTERN_PAGE_STEP,
                STAGING_PATTERN_CASE_STEP,
            );
            let mut framebuffer = patterned_memory(
                FRAMEBUFFER_BYTE_COUNT,
                case_index,
                FRAMEBUFFER_PATTERN_STEP,
                FRAMEBUFFER_PATTERN_PAGE_STEP,
                FRAMEBUFFER_PATTERN_CASE_STEP,
            );
            let expected_frame = expected_rect_frame(&vector, case_index);
            let outcome = decode_presentation_rect(
                &source,
                &mut staging[vector.staging_offset..],
                &mut framebuffer,
                vector.vertical_offset,
                vector.raw_width,
                vector.raw_rows,
            )
            .unwrap();

            assert_eq!(outcome.width, vector.row_width, "{}", vector.name);
            assert_eq!(outcome.rows, vector.rows, "{}", vector.name);
            assert_eq!(
                outcome.consumed_bytes,
                usize::from(
                    vector
                        .main_source_result_offset
                        .wrapping_sub(vector.source_offset)
                ),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.staged_values_consumed,
                usize::from(
                    vector
                        .staged_result_offset
                        .wrapping_sub(vector.staging_offset as u16)
                ),
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.final_row_offset, vector.final_row_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.final_destination_offset, vector.destination_result_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.y * LOGICAL_FRAMEBUFFER_WIDTH + outcome.x,
                vector.first_row_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.changed_pixels, vector.changed_pixels,
                "{}",
                vector.name
            );
            assert_eq!(framebuffer, expected_frame, "{}", vector.name);
            assert_eq!(
                &staging[vector.staging_offset..vector.staging_offset + vector.staged_values.len()],
                vector.staged_values,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_rectangle_is_transactional() {
        let source = [u8::MIN; PAYLOAD_HEADER_BYTE_COUNT];
        let mut staging = vec![0x31; OPTIONAL_PREFIX_BYTE_COUNT];
        let mut framebuffer = vec![0x42; FRAMEBUFFER_BYTE_COUNT];
        let staging_before = staging.clone();
        let framebuffer_before = framebuffer.clone();
        assert!(
            decode_presentation_rect(&source, &mut staging, &mut framebuffer, usize::MIN, 1, 1,)
                .is_err()
        );
        assert_eq!(staging, staging_before);
        assert_eq!(framebuffer, framebuffer_before);
    }
}
