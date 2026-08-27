//! Lossless RIFF/WAVE storage for decoded Commander Blood PCM.

use std::error::Error;
use std::fmt;

const RIFF_MAGIC: &[u8; 4] = b"RIFF";
const WAVE_MAGIC: &[u8; 4] = b"WAVE";
const FORMAT_CHUNK_MAGIC: &[u8; 4] = b"fmt ";
const DATA_CHUNK_MAGIC: &[u8; 4] = b"data";
const PCM_FORMAT_TAG: u16 = 1;
const MONO_CHANNEL_COUNT: u16 = 1;
const UNSIGNED_PCM_BITS_PER_SAMPLE: u16 = 8;
const FORMAT_CHUNK_BYTE_COUNT: u32 = 16;
const RIFF_PREFIX_BYTE_COUNT: usize = 12;
const CHUNK_HEADER_BYTE_COUNT: usize = 8;
const WAVE_HEADER_BYTE_COUNT: usize = 44;

/// Validated unsigned 8-bit mono PCM decoded from a standard WAVE file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePcm {
    sample_rate_hz: u32,
    samples: Box<[u8]>,
}

impl WavePcm {
    /// Return the authored source sample rate in hertz.
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Return every unsigned 8-bit mono PCM sample.
    pub fn samples(&self) -> &[u8] {
        &self.samples
    }
}

/// Structural or encoding failure in a normalized PCM WAVE file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaveError {
    /// The fixed RIFF/WAVE prefix is incomplete.
    HeaderTruncated {
        /// Available source bytes.
        actual: usize,
    },
    /// The file does not carry RIFF/WAVE signatures.
    InvalidSignature,
    /// A chunk header or payload extends beyond the file.
    ChunkTruncated {
        /// Byte position of the incomplete chunk.
        position: usize,
    },
    /// No supported PCM format chunk was found.
    FormatUnavailable,
    /// The format is not unsigned 8-bit mono PCM.
    UnsupportedFormat {
        /// WAVE format tag from the `fmt ` chunk.
        format_tag: u16,
        /// Number of interleaved channels.
        channel_count: u16,
        /// Number of bits representing one channel sample.
        bits_per_sample: u16,
    },
    /// The PCM sample rate is zero.
    InvalidSampleRate,
    /// No data chunk was found.
    DataUnavailable,
    /// The WAVE container would exceed its 32-bit RIFF size fields.
    FileTooLarge {
        /// PCM samples that could not fit the RIFF size domain.
        sample_count: usize,
    },
}

impl fmt::Display for WaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid normalized PCM WAVE: {self:?}")
    }
}

impl Error for WaveError {}

/// Encode exact unsigned 8-bit mono PCM into a standard RIFF/WAVE file.
pub fn encode_unsigned_pcm_wave(
    sample_rate_hz: u32,
    samples: &[u8],
) -> Result<Box<[u8]>, WaveError> {
    if sample_rate_hz == u32::MIN {
        return Err(WaveError::InvalidSampleRate);
    }
    let data_byte_count = u32::try_from(samples.len()).map_err(|_| WaveError::FileTooLarge {
        sample_count: samples.len(),
    })?;
    let padding_byte_count = data_byte_count & 1;
    let total_byte_count = u32::try_from(WAVE_HEADER_BYTE_COUNT)
        .expect("fixed WAVE header fits u32")
        .checked_add(data_byte_count)
        .and_then(|count| count.checked_add(padding_byte_count))
        .ok_or(WaveError::FileTooLarge {
            sample_count: samples.len(),
        })?;
    let riff_byte_count = total_byte_count
        .checked_sub(8)
        .expect("complete WAVE file exceeds RIFF prefix");

    let mut encoded = Vec::with_capacity(total_byte_count as usize);
    encoded.extend_from_slice(RIFF_MAGIC);
    encoded.extend_from_slice(&riff_byte_count.to_le_bytes());
    encoded.extend_from_slice(WAVE_MAGIC);
    encoded.extend_from_slice(FORMAT_CHUNK_MAGIC);
    encoded.extend_from_slice(&FORMAT_CHUNK_BYTE_COUNT.to_le_bytes());
    encoded.extend_from_slice(&PCM_FORMAT_TAG.to_le_bytes());
    encoded.extend_from_slice(&MONO_CHANNEL_COUNT.to_le_bytes());
    encoded.extend_from_slice(&sample_rate_hz.to_le_bytes());
    encoded.extend_from_slice(&sample_rate_hz.to_le_bytes());
    encoded.extend_from_slice(&MONO_CHANNEL_COUNT.to_le_bytes());
    encoded.extend_from_slice(&UNSIGNED_PCM_BITS_PER_SAMPLE.to_le_bytes());
    encoded.extend_from_slice(DATA_CHUNK_MAGIC);
    encoded.extend_from_slice(&data_byte_count.to_le_bytes());
    encoded.extend_from_slice(samples);
    if padding_byte_count != u32::MIN {
        encoded.push(u8::MIN);
    }
    Ok(encoded.into_boxed_slice())
}

/// Decode exact unsigned 8-bit mono PCM from a standard RIFF/WAVE file.
pub fn decode_unsigned_pcm_wave(encoded: &[u8]) -> Result<WavePcm, WaveError> {
    if encoded.len() < RIFF_PREFIX_BYTE_COUNT {
        return Err(WaveError::HeaderTruncated {
            actual: encoded.len(),
        });
    }
    if &encoded[..4] != RIFF_MAGIC || &encoded[8..12] != WAVE_MAGIC {
        return Err(WaveError::InvalidSignature);
    }

    let mut cursor = RIFF_PREFIX_BYTE_COUNT;
    let mut sample_rate_hz = None;
    let mut samples = None;
    while cursor < encoded.len() {
        let header_end = cursor
            .checked_add(CHUNK_HEADER_BYTE_COUNT)
            .filter(|end| *end <= encoded.len())
            .ok_or(WaveError::ChunkTruncated { position: cursor })?;
        let chunk_name = &encoded[cursor..cursor + 4];
        let chunk_byte_count = u32::from_le_bytes(
            encoded[cursor + 4..header_end]
                .try_into()
                .expect("validated WAVE chunk length"),
        ) as usize;
        let payload_start = header_end;
        let payload_end = payload_start
            .checked_add(chunk_byte_count)
            .filter(|end| *end <= encoded.len())
            .ok_or(WaveError::ChunkTruncated { position: cursor })?;

        if chunk_name == FORMAT_CHUNK_MAGIC {
            if chunk_byte_count < FORMAT_CHUNK_BYTE_COUNT as usize {
                return Err(WaveError::ChunkTruncated { position: cursor });
            }
            let format_tag = read_u16(encoded, payload_start);
            let channel_count = read_u16(encoded, payload_start + 2);
            let rate = read_u32(encoded, payload_start + 4);
            let bits_per_sample = read_u16(encoded, payload_start + 14);
            if format_tag != PCM_FORMAT_TAG
                || channel_count != MONO_CHANNEL_COUNT
                || bits_per_sample != UNSIGNED_PCM_BITS_PER_SAMPLE
            {
                return Err(WaveError::UnsupportedFormat {
                    format_tag,
                    channel_count,
                    bits_per_sample,
                });
            }
            if rate == u32::MIN {
                return Err(WaveError::InvalidSampleRate);
            }
            sample_rate_hz = Some(rate);
        } else if chunk_name == DATA_CHUNK_MAGIC {
            samples = Some(Box::from(&encoded[payload_start..payload_end]));
        }

        cursor = payload_end
            .checked_add(chunk_byte_count & 1)
            .ok_or(WaveError::ChunkTruncated { position: cursor })?;
        if cursor > encoded.len() {
            return Err(WaveError::ChunkTruncated {
                position: payload_end,
            });
        }
    }

    Ok(WavePcm {
        sample_rate_hz: sample_rate_hz.ok_or(WaveError::FormatUnavailable)?,
        samples: samples.ok_or(WaveError::DataUnavailable)?,
    })
}

fn read_u16(encoded: &[u8], position: usize) -> u16 {
    u16::from_le_bytes(
        encoded[position..position + 2]
            .try_into()
            .expect("validated WAVE word"),
    )
}

fn read_u32(encoded: &[u8], position: usize) -> u32 {
    u32::from_le_bytes(
        encoded[position..position + 4]
            .try_into()
            .expect("validated WAVE double word"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAMPLE_RATE_HZ: u32 = 11_111;

    #[test]
    fn exact_pcm_round_trip_handles_even_and_odd_sample_counts() {
        for samples in [vec![0, 64, 128, 255], vec![255, 128, 0]] {
            let encoded = encode_unsigned_pcm_wave(TEST_SAMPLE_RATE_HZ, &samples).unwrap();
            let decoded = decode_unsigned_pcm_wave(&encoded).unwrap();
            assert_eq!(decoded.sample_rate_hz(), TEST_SAMPLE_RATE_HZ);
            assert_eq!(decoded.samples(), samples);
            assert_eq!(encoded.len() & 1, usize::MIN);
        }
    }

    #[test]
    fn rejects_non_mono_or_non_eight_bit_pcm() {
        let mut encoded = encode_unsigned_pcm_wave(TEST_SAMPLE_RATE_HZ, &[128]).unwrap();
        encoded[22..24].copy_from_slice(&2_u16.to_le_bytes());
        assert!(matches!(
            decode_unsigned_pcm_wave(&encoded),
            Err(WaveError::UnsupportedFormat {
                channel_count: 2,
                ..
            })
        ));
    }
}
