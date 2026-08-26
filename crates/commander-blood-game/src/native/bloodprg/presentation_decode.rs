//! Flat bounded decoders for compressed presentation payloads.

use std::error::Error;
use std::fmt;

const AB_HEADER_BYTE_COUNT: usize = 6;
const CONTROL_LOW_BIT_MASK: u16 = 1;
const AB_CONTROL_SENTINEL: u16 = 0x8000;
const AB_LONG_DISPLACEMENT_MASK: u16 = 0xE000;
const AB_LONG_DISPLACEMENT_SHIFT: u32 = 3;
const AB_LENGTH_BIAS: usize = 2;
const AB_LONG_LENGTH_MASK: u16 = 7;
const AB_SHORT_LENGTH_SHIFT: usize = 1;
const PAIR_MATCH_CONTROL_FLAG: u8 = 0x80;
const PAIR_CONTROL_DISTANCE_MASK: u8 = 0x7F;
const PAIR_DISTANCE_SHIFT: u32 = 1;
const PAIR_FIRST_DISTANCE_LOW_SHIFT: u32 = 4;
const PAIR_FIRST_LENGTH_SHIFT: u32 = 5;
const PAIR_SECOND_LENGTH_SHIFT: u32 = 1;
const PAIR_PACKED_LOW_BIT_MASK: u8 = 1;
const PAIR_SECOND_LENGTH_MASK: u8 = 7;
const PAIR_DISTANCE_BIAS: usize = 1;
const PAIR_LENGTH_BIAS: usize = 2;
const MAXIMUM_DECODED_BYTE_COUNT: usize = u16::MAX as usize + 1;
const WORD_BYTE_COUNT: usize = size_of::<u16>();

/// Invalid compressed source or destination geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationDecodeError {
    /// A decoder attempted to read beyond the owned compressed bytes.
    SourceTruncated {
        /// Position of the failed read.
        position: usize,
        /// Bytes required by that read.
        required: usize,
        /// Bytes available from the position.
        available: usize,
    },
    /// A match refers before the available decoded history.
    InvalidBackReference {
        /// Current output position.
        output_position: usize,
        /// Requested backward distance.
        distance: usize,
    },
    /// An internal or future AB token requests a nonnegative copy displacement.
    NonBackwardDisplacement {
        /// Signed displacement supplied to the match copier.
        displacement: i16,
    },
    /// Decoding would exceed the original offset-sized output domain.
    OutputTooLarge {
        /// Requested decoded byte count.
        requested: usize,
    },
    /// An in-place destination range is empty, reversed, or outside its buffer.
    InvalidOutputRange {
        /// First output byte.
        start: usize,
        /// Exclusive output boundary.
        end: usize,
        /// Available destination bytes.
        buffer_len: usize,
    },
    /// A complete native match would write beyond its declared output boundary.
    MatchExceedsOutput {
        /// Current output position.
        output_position: usize,
        /// Match length requested by the stream.
        length: usize,
        /// Exclusive declared output boundary.
        output_end: usize,
    },
}

impl fmt::Display for PresentationDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid compressed presentation payload: {self:?}"
        )
    }
}

impl Error for PresentationDecodeError {}

/// Complete decoded bytes and compressed-source progress for the AB codec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbDecodeOutcome {
    /// Independently owned decoded payload.
    pub bytes: Box<[u8]>,
    /// Compressed bytes consumed including the six-byte header.
    pub consumed_bytes: usize,
}

/// Source and destination progress for the pair-LZ staging codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairLzDecodeOutcome {
    /// Compressed source bytes consumed.
    pub consumed_bytes: usize,
    /// Bytes published in the requested destination range.
    pub produced_bytes: usize,
}

fn truncated(source: &[u8], position: usize, required: usize) -> PresentationDecodeError {
    PresentationDecodeError::SourceTruncated {
        position,
        required,
        available: source.len().saturating_sub(position),
    }
}

fn read_byte(source: &[u8], cursor: &mut usize) -> Result<u8, PresentationDecodeError> {
    let value = source
        .get(*cursor)
        .copied()
        .ok_or_else(|| truncated(source, *cursor, size_of::<u8>()))?;
    *cursor += size_of::<u8>();
    Ok(value)
}

fn read_word(source: &[u8], cursor: &mut usize) -> Result<u16, PresentationDecodeError> {
    let end = (*cursor)
        .checked_add(WORD_BYTE_COUNT)
        .ok_or_else(|| truncated(source, *cursor, WORD_BYTE_COUNT))?;
    let bytes = source
        .get(*cursor..end)
        .ok_or_else(|| truncated(source, *cursor, WORD_BYTE_COUNT))?;
    *cursor = end;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("validated two-byte decoder word"),
    ))
}

fn push_output(output: &mut Vec<u8>, value: u8) -> Result<(), PresentationDecodeError> {
    let requested = output
        .len()
        .checked_add(1)
        .ok_or(PresentationDecodeError::OutputTooLarge {
            requested: usize::MAX,
        })?;
    if requested > MAXIMUM_DECODED_BYTE_COUNT {
        return Err(PresentationDecodeError::OutputTooLarge { requested });
    }
    output.push(value);
    Ok(())
}

fn copy_ab_match(
    output: &mut Vec<u8>,
    displacement: i16,
    length: usize,
) -> Result<(), PresentationDecodeError> {
    let distance = displacement
        .checked_neg()
        .filter(|distance| *distance > 0)
        .map(|distance| distance as usize)
        .ok_or(PresentationDecodeError::NonBackwardDisplacement { displacement })?;
    let mut copy_position = output.len().checked_sub(distance).ok_or(
        PresentationDecodeError::InvalidBackReference {
            output_position: output.len(),
            distance,
        },
    )?;
    for _ in 0..length {
        let value = output.get(copy_position).copied().ok_or(
            PresentationDecodeError::InvalidBackReference {
                output_position: output.len(),
                distance,
            },
        )?;
        push_output(output, value)?;
        copy_position += 1;
    }
    Ok(())
}

struct AbBitReader<'a> {
    source: &'a [u8],
    cursor: usize,
    control_bits: u16,
}

impl<'a> AbBitReader<'a> {
    fn new(source: &'a [u8]) -> Result<Self, PresentationDecodeError> {
        if source.len() < AB_HEADER_BYTE_COUNT {
            return Err(truncated(source, usize::MIN, AB_HEADER_BYTE_COUNT));
        }
        Ok(Self {
            source,
            cursor: AB_HEADER_BYTE_COUNT,
            control_bits: u16::MIN,
        })
    }

    fn read_bit(&mut self) -> Result<bool, PresentationDecodeError> {
        let mut value = self.control_bits & CONTROL_LOW_BIT_MASK;
        self.control_bits >>= CONTROL_LOW_BIT_MASK;
        if self.control_bits == u16::MIN {
            let control_word = read_word(self.source, &mut self.cursor)?;
            value = control_word & CONTROL_LOW_BIT_MASK;
            self.control_bits = (control_word >> CONTROL_LOW_BIT_MASK) | AB_CONTROL_SENTINEL;
        }
        Ok(value != u16::MIN)
    }

    fn read_byte(&mut self) -> Result<u8, PresentationDecodeError> {
        read_byte(self.source, &mut self.cursor)
    }

    fn read_word(&mut self) -> Result<u16, PresentationDecodeError> {
        read_word(self.source, &mut self.cursor)
    }
}

/// Decode one terminated AB presentation payload into owned bytes.
///
/// This translates `resource_payload_decode_ab` at BLOODPRG offset `0x00A867`.
/// It retains LSB-first control refills, both displacement forms, extended
/// lengths, and forward overlap while replacing wrapping destination pointers
/// with a bounded output collection.
pub fn decode_presentation_ab(source: &[u8]) -> Result<AbDecodeOutcome, PresentationDecodeError> {
    let mut reader = AbBitReader::new(source)?;
    let mut output = Vec::new();

    loop {
        if reader.read_bit()? {
            let value = reader.read_byte()?;
            push_output(&mut output, value)?;
            continue;
        }

        let (displacement, encoded_length) = if !reader.read_bit()? {
            let high = usize::from(reader.read_bit()?);
            let low = usize::from(reader.read_bit()?);
            let encoded_length = (high << AB_SHORT_LENGTH_SHIFT) | low;
            // The original loads AL and then forces AH to 0xff. This makes
            // every short token a distance in -256..=-1, including bytes below
            // 0x80; it is not an i8 sign extension.
            (i16::from(reader.read_byte()?) - 256, encoded_length)
        } else {
            let packed = reader.read_word()?;
            let mut encoded_length = usize::from(packed & AB_LONG_LENGTH_MASK);
            let displacement =
                ((packed >> AB_LONG_DISPLACEMENT_SHIFT) | AB_LONG_DISPLACEMENT_MASK) as i16;
            if encoded_length == usize::MIN {
                encoded_length = usize::from(reader.read_byte()?);
                if encoded_length == usize::MIN {
                    break;
                }
            }
            (displacement, encoded_length)
        };
        copy_ab_match(&mut output, displacement, encoded_length + AB_LENGTH_BIAS)?;
    }

    Ok(AbDecodeOutcome {
        bytes: output.into_boxed_slice(),
        consumed_bytes: reader.cursor,
    })
}

fn copy_pair_match(
    destination: &mut [u8],
    output: &mut usize,
    output_end: usize,
    distance: usize,
    length: usize,
) -> Result<(), PresentationDecodeError> {
    let mut copy_position =
        output
            .checked_sub(distance)
            .ok_or(PresentationDecodeError::InvalidBackReference {
                output_position: *output,
                distance,
            })?;
    let match_end = output
        .checked_add(length)
        .filter(|end| *end <= output_end)
        .ok_or(PresentationDecodeError::MatchExceedsOutput {
            output_position: *output,
            length,
            output_end,
        })?;
    while *output < match_end {
        destination[*output] = destination[copy_position];
        *output += 1;
        copy_position += 1;
    }
    Ok(())
}

fn write_pair_literal(
    destination: &mut [u8],
    output: &mut usize,
    output_end: usize,
    control: u8,
    literal_bias: u8,
) -> Result<(), PresentationDecodeError> {
    if *output >= output_end {
        return Err(PresentationDecodeError::MatchExceedsOutput {
            output_position: *output,
            length: size_of::<u8>(),
            output_end,
        });
    }
    destination[*output] = if control == u8::MIN {
        u8::MIN
    } else {
        control.wrapping_add(literal_bias)
    };
    *output += 1;
    Ok(())
}

/// Decode pair-LZ bytes into one bounded in-place destination range.
///
/// This translates `resource_pair_lz_decode` at `0x00AABC`. The destination
/// may provide history before `output_start`; sequential match copies retain
/// overlap propagation. Invalid wrapped history and native match overshoot are
/// rejected transactionally.
pub fn decode_presentation_pair_lz(
    source: &[u8],
    destination: &mut [u8],
    output_start: usize,
    output_end: usize,
    literal_bias: u8,
) -> Result<PairLzDecodeOutcome, PresentationDecodeError> {
    if output_start >= output_end || output_end > destination.len() {
        return Err(PresentationDecodeError::InvalidOutputRange {
            start: output_start,
            end: output_end,
            buffer_len: destination.len(),
        });
    }
    let mut staged = destination.to_vec();
    let mut cursor = usize::MIN;
    let mut output = output_start;

    loop {
        let control = read_byte(source, &mut cursor)?;
        if control & PAIR_MATCH_CONTROL_FLAG == u8::MIN {
            write_pair_literal(&mut staged, &mut output, output_end, control, literal_bias)?;
            if output >= output_end {
                break;
            }
            continue;
        }

        let packed = read_byte(source, &mut cursor)?;
        let first_distance = usize::from(
            ((u16::from(control & PAIR_CONTROL_DISTANCE_MASK)) << PAIR_DISTANCE_SHIFT)
                | u16::from((packed >> PAIR_FIRST_DISTANCE_LOW_SHIFT) & PAIR_PACKED_LOW_BIT_MASK),
        ) + PAIR_DISTANCE_BIAS;
        let first_length = usize::from(packed >> PAIR_FIRST_LENGTH_SHIFT) + PAIR_LENGTH_BIAS;
        copy_pair_match(
            &mut staged,
            &mut output,
            output_end,
            first_distance,
            first_length,
        )?;
        if output >= output_end {
            break;
        }

        let second_control = loop {
            let next_control = read_byte(source, &mut cursor)?;
            if next_control & PAIR_MATCH_CONTROL_FLAG != u8::MIN {
                break next_control;
            }
            write_pair_literal(
                &mut staged,
                &mut output,
                output_end,
                next_control,
                literal_bias,
            )?;
            if output >= output_end {
                destination.copy_from_slice(&staged);
                return Ok(PairLzDecodeOutcome {
                    consumed_bytes: cursor,
                    produced_bytes: output - output_start,
                });
            }
        };

        let second_distance = usize::from(
            ((u16::from(second_control & PAIR_CONTROL_DISTANCE_MASK)) << PAIR_DISTANCE_SHIFT)
                | u16::from(packed & PAIR_PACKED_LOW_BIT_MASK),
        ) + PAIR_DISTANCE_BIAS;
        let second_length =
            usize::from((packed >> PAIR_SECOND_LENGTH_SHIFT) & PAIR_SECOND_LENGTH_MASK)
                + PAIR_LENGTH_BIAS;
        copy_pair_match(
            &mut staged,
            &mut output,
            output_end,
            second_distance,
            second_length,
        )?;
        if output >= output_end {
            break;
        }
    }

    destination.copy_from_slice(&staged);
    Ok(PairLzDecodeOutcome {
        consumed_bytes: cursor,
        produced_bytes: output - output_start,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const AB_VECTOR_COUNT: usize = 10;
    const PAIR_VECTOR_COUNT: usize = 13;
    const FLAT_PAIR_VECTOR_COUNT: usize = 11;
    const DESTINATION_PATTERN_STEP: usize = 23;
    const DESTINATION_PATTERN_PAGE_STEP: usize = 11;
    const DESTINATION_PATTERN_CASE_STEP: usize = 31;
    const OVERSHOOT_VECTOR_NAME: &str = "first_match_overshoots_end";
    const WRAPPED_HISTORY_VECTOR_NAME: &str = "copy_source_wrap";

    #[derive(Deserialize)]
    struct AbOracle {
        name: String,
        compressed_stream_hex: String,
        decoded_hex: String,
        decoded_length: usize,
        source_offset: u16,
        source_result_offset: u16,
    }

    #[derive(Deserialize)]
    struct PairOracle {
        name: String,
        compressed_stream_hex: String,
        consumed_bytes: usize,
        decoded_hex: String,
        destination_offset: usize,
        destination_end: usize,
        source_offset: u16,
        source_result_offset: u16,
        literal_bias: u8,
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

    fn destination_memory(case_index: usize) -> Vec<u8> {
        (usize::MIN..MAXIMUM_DECODED_BYTE_COUNT)
            .map(|offset| {
                (offset * DESTINATION_PATTERN_STEP
                    + (offset >> u8::BITS) * DESTINATION_PATTERN_PAGE_STEP
                    + case_index * DESTINATION_PATTERN_CASE_STEP) as u8
            })
            .collect()
    }

    #[test]
    fn ab_decoder_matches_every_original_vector() {
        let vectors: Vec<AbOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a867_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), AB_VECTOR_COUNT);

        for vector in vectors {
            let source = decode_hex(&vector.compressed_stream_hex);
            let expected = decode_hex(&vector.decoded_hex);
            let outcome = decode_presentation_ab(&source).unwrap();
            assert_eq!(&*outcome.bytes, expected, "{}", vector.name);
            assert_eq!(
                outcome.bytes.len(),
                vector.decoded_length,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.consumed_bytes,
                usize::from(
                    vector
                        .source_result_offset
                        .wrapping_sub(vector.source_offset)
                ),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn pair_lz_decoder_matches_flat_vectors_and_rejects_wrapping() {
        let vectors: Vec<PairOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_aabc_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PAIR_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for (case_index, vector) in vectors.into_iter().enumerate() {
            let source = decode_hex(&vector.compressed_stream_hex);
            let expected = decode_hex(&vector.decoded_hex);
            let mut destination = destination_memory(case_index);
            let before = destination.clone();
            let result = decode_presentation_pair_lz(
                &source,
                &mut destination,
                vector.destination_offset,
                vector.destination_end,
                vector.literal_bias,
            );

            if matches!(
                vector.name.as_str(),
                OVERSHOOT_VECTOR_NAME | WRAPPED_HISTORY_VECTOR_NAME
            ) {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(destination, before, "{}", vector.name);
                continue;
            }

            let outcome = result.unwrap();
            assert_eq!(
                outcome.consumed_bytes, vector.consumed_bytes,
                "{}",
                vector.name
            );
            assert_eq!(outcome.produced_bytes, expected.len(), "{}", vector.name);
            assert_eq!(
                &destination[vector.destination_offset..vector.destination_end],
                expected,
                "{}",
                vector.name
            );
            assert_eq!(
                usize::from(
                    vector
                        .source_result_offset
                        .wrapping_sub(vector.source_offset)
                ),
                vector.consumed_bytes,
                "{}",
                vector.name
            );
            matched += 1;
        }
        assert_eq!(matched, FLAT_PAIR_VECTOR_COUNT);
    }
}
