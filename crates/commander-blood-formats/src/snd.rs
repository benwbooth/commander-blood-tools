//! Decoder for Commander Blood SND clip banks.

use std::fmt;

const SND_HEADER_BYTE_COUNT: usize = 4;
const SND_OFFSET_BYTE_COUNT: usize = 4;
const SND_RATE_NUMERATOR: u32 = 1_000_000;
const SND_RATE_DENOMINATOR_BASE: u32 = 256;
const SND_MAXIMUM_RATE_CODE: u8 = u8::MAX;
const SND_MAXIMUM_RATE_CODE_HZ: u32 = 11_111;

/// Number of metadata bytes before PCM data in one encoded SND clip.
pub const SND_CLIP_HEADER_BYTE_COUNT: usize = 6;

/// Four-byte bank header shared by resident and streamed SND files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SndBankHeader {
    /// Number of clips described by the following offset table.
    pub clip_count: u16,
    /// Base deterministic-dialogue delay.
    pub dialogue_delay_base: u8,
    /// Inclusive deterministic-dialogue delay limit.
    pub dialogue_delay_limit: u8,
}

/// Validated owned SND bank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SndBank {
    header: SndBankHeader,
    offsets: Box<[usize]>,
    payload: Box<[u8]>,
}

impl SndBank {
    /// Decode a complete SND file into an owned clip table and payload.
    pub fn decode(encoded: &[u8]) -> Result<Self, SndBankDecodeError> {
        if encoded.len() < SND_HEADER_BYTE_COUNT {
            return Err(SndBankDecodeError::HeaderTruncated {
                actual: encoded.len(),
            });
        }

        let header = SndBankHeader {
            clip_count: u16::from_le_bytes([encoded[0], encoded[1]]),
            dialogue_delay_base: encoded[2],
            dialogue_delay_limit: encoded[3],
        };
        let table_count = usize::from(header.clip_count)
            .checked_add(1)
            .ok_or(SndBankDecodeError::OffsetTableSizeOverflow)?;
        let table_bytes = table_count
            .checked_mul(SND_OFFSET_BYTE_COUNT)
            .ok_or(SndBankDecodeError::OffsetTableSizeOverflow)?;
        let payload_start = SND_HEADER_BYTE_COUNT
            .checked_add(table_bytes)
            .ok_or(SndBankDecodeError::OffsetTableSizeOverflow)?;
        if payload_start > encoded.len() {
            return Err(SndBankDecodeError::OffsetTableTruncated {
                expected_end: payload_start,
                actual: encoded.len(),
            });
        }

        let mut offsets = Vec::with_capacity(table_count);
        for index in 0..table_count {
            let start = SND_HEADER_BYTE_COUNT + index * SND_OFFSET_BYTE_COUNT;
            let offset = u32::from_le_bytes([
                encoded[start],
                encoded[start + 1],
                encoded[start + 2],
                encoded[start + 3],
            ]) as usize;
            if let Some(previous) = offsets.last().copied()
                && offset < previous
            {
                return Err(SndBankDecodeError::OffsetsNotMonotonic {
                    index,
                    previous,
                    current: offset,
                });
            }
            offsets.push(offset);
        }

        let payload = Box::<[u8]>::from(&encoded[payload_start..]);
        if let Some(&final_offset) = offsets.last()
            && final_offset > payload.len()
        {
            return Err(SndBankDecodeError::ClipOutsidePayload {
                index: usize::from(header.clip_count),
                offset: final_offset,
                payload_len: payload.len(),
            });
        }

        Ok(Self {
            header,
            offsets: offsets.into_boxed_slice(),
            payload,
        })
    }

    /// Return the decoded bank header.
    pub const fn header(&self) -> SndBankHeader {
        self.header
    }

    /// Return all clip boundaries relative to the payload.
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// Return the complete owned payload after the offset table.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Return one validated encoded clip by its authored index.
    pub fn clip(&self, index: usize) -> Option<SndClip<'_>> {
        if index >= usize::from(self.header.clip_count) {
            return None;
        }
        let start = self.offsets[index];
        let end = self.offsets[index + 1];
        Some(SndClip {
            index,
            encoded: &self.payload[start..end],
        })
    }
}

/// One encoded clip borrowed from a validated SND bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SndClip<'a> {
    index: usize,
    encoded: &'a [u8],
}

impl<'a> SndClip<'a> {
    /// Return the clip's authored zero-based index.
    pub const fn index(self) -> usize {
        self.index
    }

    /// Return the complete encoded clip including its six-byte metadata header.
    pub const fn encoded(self) -> &'a [u8] {
        self.encoded
    }

    /// Return the unsigned 8-bit PCM payload after the clip metadata.
    pub fn pcm(self) -> Option<&'a [u8]> {
        self.encoded.get(SND_CLIP_HEADER_BYTE_COUNT..)
    }

    /// Return the Creative Voice time constant stored in the clip header.
    pub fn sample_rate_code(self) -> Option<u8> {
        self.encoded.get(4).copied()
    }

    /// Return the decoded sample rate in hertz.
    pub fn sample_rate_hz(self) -> Option<u32> {
        self.sample_rate_code().map(snd_sample_rate_hz)
    }
}

/// Structural failure while decoding an SND bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndBankDecodeError {
    /// The fixed four-byte header is incomplete.
    HeaderTruncated {
        /// Number of available bytes.
        actual: usize,
    },
    /// The clip count cannot be represented as an offset-table size.
    OffsetTableSizeOverflow,
    /// The declared offset table extends past the input.
    OffsetTableTruncated {
        /// Required exclusive table end.
        expected_end: usize,
        /// Number of available bytes.
        actual: usize,
    },
    /// One clip boundary precedes the prior boundary.
    OffsetsNotMonotonic {
        /// Boundary index that failed.
        index: usize,
        /// Prior boundary.
        previous: usize,
        /// Invalid current boundary.
        current: usize,
    },
    /// A clip boundary extends past the owned payload.
    ClipOutsidePayload {
        /// Boundary index that failed.
        index: usize,
        /// Invalid payload-relative boundary.
        offset: usize,
        /// Number of available payload bytes.
        payload_len: usize,
    },
}

impl fmt::Display for SndBankDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SndBankDecodeError {}

/// Convert a Creative Voice time constant to its unsigned 8-bit PCM rate.
pub fn snd_sample_rate_hz(rate_code: u8) -> u32 {
    if rate_code == SND_MAXIMUM_RATE_CODE {
        SND_MAXIMUM_RATE_CODE_HZ
    } else {
        SND_RATE_NUMERATOR / (SND_RATE_DENOMINATOR_BASE - u32::from(rate_code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_offsets_are_rejected_before_slicing() {
        let encoded = [1, 0, 4, 20, 3, 0, 0, 0, 2, 0, 0, 0, 1, 2, 3];
        assert_eq!(
            SndBank::decode(&encoded),
            Err(SndBankDecodeError::OffsetsNotMonotonic {
                index: 1,
                previous: 3,
                current: 2,
            })
        );
    }
}
