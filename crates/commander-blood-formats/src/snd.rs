//! Decoders for Commander Blood SND clip banks and Creative Voice music.

use std::fmt;

const SND_HEADER_BYTE_COUNT: usize = 4;
const SND_OFFSET_BYTE_COUNT: usize = 4;
const SND_RATE_NUMERATOR: u32 = 1_000_000;
const SND_RATE_DENOMINATOR_BASE: u32 = 256;
const SND_MAXIMUM_RATE_CODE: u8 = u8::MAX;
const SND_MAXIMUM_RATE_CODE_HZ: u32 = 11_111;
const VOC_MAGIC: &[u8] = b"Creative Voice File\x1a";
const VOC_MINIMUM_HEADER_BYTE_COUNT: usize = 26;
const VOC_HEADER_OFFSET_POSITION: usize = 20;
const VOC_VERSION_POSITION: usize = 22;
const VOC_CHECKSUM_POSITION: usize = 24;
const VOC_VERSION_CHECKSUM_BIAS: u16 = 0x1234;
const VOC_BLOCK_HEADER_BYTE_COUNT: usize = 4;
const VOC_BLOCK_LENGTH_BYTE_COUNT: usize = 3;
const VOC_SOUND_DATA_METADATA_BYTE_COUNT: usize = 2;
const VOC_TERMINATOR_BLOCK: u8 = 0;
const VOC_SOUND_DATA_BLOCK: u8 = 1;
const VOC_SOUND_CONTINUATION_BLOCK: u8 = 2;
const VOC_UNSIGNED_PCM_CODEC: u8 = 0;

/// Number of metadata bytes before PCM data in one encoded SND clip.
pub const SND_CLIP_HEADER_BYTE_COUNT: usize = 6;

/// Validated unsigned 8-bit mono PCM decoded from a Creative Voice file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VocPcm {
    version: u16,
    sample_rate_code: u8,
    samples: Box<[u8]>,
}

impl VocPcm {
    /// Decode the exact VOC block family used by every shipped music resource.
    ///
    /// Commander Blood's corpus contains one type-1 unsigned-PCM block,
    /// optionally followed by one or more type-2 continuation blocks, then a
    /// type-0 terminator. Unsupported codecs and block families are rejected
    /// instead of being silently skipped.
    pub fn decode(encoded: &[u8]) -> Result<Self, VocDecodeError> {
        if encoded.len() < VOC_MINIMUM_HEADER_BYTE_COUNT {
            return Err(VocDecodeError::HeaderTruncated {
                actual: encoded.len(),
            });
        }
        if !encoded.starts_with(VOC_MAGIC) {
            return Err(VocDecodeError::InvalidMagic);
        }

        let data_offset = usize::from(read_u16(encoded, VOC_HEADER_OFFSET_POSITION));
        if !(VOC_MINIMUM_HEADER_BYTE_COUNT..=encoded.len()).contains(&data_offset) {
            return Err(VocDecodeError::InvalidDataOffset {
                offset: data_offset,
                encoded_len: encoded.len(),
            });
        }
        let version = read_u16(encoded, VOC_VERSION_POSITION);
        let checksum = read_u16(encoded, VOC_CHECKSUM_POSITION);
        let expected_checksum = (!version).wrapping_add(VOC_VERSION_CHECKSUM_BIAS);
        if checksum != expected_checksum {
            return Err(VocDecodeError::InvalidVersionChecksum {
                version,
                expected: expected_checksum,
                actual: checksum,
            });
        }

        let mut cursor = data_offset;
        let mut sample_rate_code = None;
        let mut samples = Vec::new();
        loop {
            let block_type = *encoded
                .get(cursor)
                .ok_or(VocDecodeError::MissingTerminator)?;
            if block_type == VOC_TERMINATOR_BLOCK {
                break;
            }

            let block_header_end = cursor
                .checked_add(VOC_BLOCK_HEADER_BYTE_COUNT)
                .ok_or(VocDecodeError::BlockRangeOverflow { offset: cursor })?;
            if block_header_end > encoded.len() {
                return Err(VocDecodeError::BlockHeaderTruncated {
                    offset: cursor,
                    actual: encoded.len().saturating_sub(cursor),
                });
            }
            let length_start = cursor + 1;
            let block_len =
                read_u24(&encoded[length_start..length_start + VOC_BLOCK_LENGTH_BYTE_COUNT]);
            let block_end = block_header_end
                .checked_add(block_len)
                .ok_or(VocDecodeError::BlockRangeOverflow { offset: cursor })?;
            if block_end > encoded.len() {
                return Err(VocDecodeError::BlockPayloadTruncated {
                    block_type,
                    expected_end: block_end,
                    actual: encoded.len(),
                });
            }
            let block = &encoded[block_header_end..block_end];
            match block_type {
                VOC_SOUND_DATA_BLOCK => {
                    if sample_rate_code.is_some() {
                        return Err(VocDecodeError::MultipleSoundDataBlocks);
                    }
                    if block.len() < VOC_SOUND_DATA_METADATA_BYTE_COUNT {
                        return Err(VocDecodeError::SoundDataMetadataTruncated {
                            actual: block.len(),
                        });
                    }
                    let rate_code = block[0];
                    let codec = block[1];
                    if codec != VOC_UNSIGNED_PCM_CODEC {
                        return Err(VocDecodeError::UnsupportedCodec(codec));
                    }
                    sample_rate_code = Some(rate_code);
                    samples.extend_from_slice(&block[VOC_SOUND_DATA_METADATA_BYTE_COUNT..]);
                }
                VOC_SOUND_CONTINUATION_BLOCK => {
                    if sample_rate_code.is_none() {
                        return Err(VocDecodeError::ContinuationBeforeSoundData);
                    }
                    samples.extend_from_slice(block);
                }
                unsupported => return Err(VocDecodeError::UnsupportedBlockType(unsupported)),
            }
            cursor = block_end;
        }

        let sample_rate_code = sample_rate_code.ok_or(VocDecodeError::SoundDataUnavailable)?;
        if samples.is_empty() {
            return Err(VocDecodeError::EmptyPcmPayload);
        }
        Ok(Self {
            version,
            sample_rate_code,
            samples: samples.into_boxed_slice(),
        })
    }

    /// Return the Creative Voice file-format version.
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Return the Creative Voice time constant authored in the type-1 block.
    pub const fn sample_rate_code(&self) -> u8 {
        self.sample_rate_code
    }

    /// Return the decoded unsigned 8-bit mono sample rate in hertz.
    pub fn sample_rate_hz(&self) -> u32 {
        snd_sample_rate_hz(self.sample_rate_code)
    }

    /// Return all decoded unsigned 8-bit mono samples.
    pub fn samples(&self) -> &[u8] {
        &self.samples
    }
}

/// Structural or codec failure while decoding one Creative Voice file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VocDecodeError {
    /// The fixed Creative Voice header is incomplete.
    HeaderTruncated {
        /// Number of available bytes.
        actual: usize,
    },
    /// The file does not begin with the Creative Voice signature.
    InvalidMagic,
    /// The header's first-block offset is outside the encoded file.
    InvalidDataOffset {
        /// Declared absolute block offset.
        offset: usize,
        /// Number of encoded bytes.
        encoded_len: usize,
    },
    /// The header version and checksum do not satisfy the Creative Voice rule.
    InvalidVersionChecksum {
        /// Encoded format version.
        version: u16,
        /// Checksum derived from the version.
        expected: u16,
        /// Encoded checksum.
        actual: u16,
    },
    /// A non-terminator block lacks its three-byte length field.
    BlockHeaderTruncated {
        /// Absolute block offset.
        offset: usize,
        /// Bytes available from that offset.
        actual: usize,
    },
    /// A block's declared payload extends past the encoded file.
    BlockPayloadTruncated {
        /// Encoded block family.
        block_type: u8,
        /// Declared exclusive block end.
        expected_end: usize,
        /// Number of encoded bytes.
        actual: usize,
    },
    /// Checked block-end arithmetic overflowed the host address domain.
    BlockRangeOverflow {
        /// Absolute block offset.
        offset: usize,
    },
    /// The file ended before a type-0 block was encountered.
    MissingTerminator,
    /// The type-1 block lacks its rate-code and codec bytes.
    SoundDataMetadataTruncated {
        /// Number of bytes in the malformed block.
        actual: usize,
    },
    /// More than one type-1 sound-data block was encoded.
    MultipleSoundDataBlocks,
    /// A type-2 continuation appeared before the type-1 sound-data block.
    ContinuationBeforeSoundData,
    /// The type-1 block uses a codec other than unsigned 8-bit PCM.
    UnsupportedCodec(u8),
    /// The file uses a block family absent from Commander Blood's corpus.
    UnsupportedBlockType(u8),
    /// No type-1 sound-data block was present.
    SoundDataUnavailable,
    /// Sound blocks were present but contained no PCM samples.
    EmptyPcmPayload,
}

impl fmt::Display for VocDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for VocDecodeError {}

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

fn read_u16(encoded: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([encoded[offset], encoded[offset + 1]])
}

fn read_u24(encoded: &[u8]) -> usize {
    usize::from(encoded[0])
        | usize::from(encoded[1]) << u8::BITS
        | usize::from(encoded[2]) << (u8::BITS * 2)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const SHIPPED_VOC_RESOURCE_COUNT: usize = 44;
    const SYNTHETIC_RATE_CODE: u8 = 166;
    const SYNTHETIC_VERSION: u16 = 0x010a;

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

    #[test]
    fn voc_sound_and_continuation_blocks_form_one_pcm_stream() {
        let encoded = synthetic_voc(&[
            (VOC_SOUND_DATA_BLOCK, &[SYNTHETIC_RATE_CODE, 0, 1, 2]),
            (VOC_SOUND_CONTINUATION_BLOCK, &[3, 4]),
        ]);
        let decoded = VocPcm::decode(&encoded).unwrap();

        assert_eq!(decoded.version(), SYNTHETIC_VERSION);
        assert_eq!(decoded.sample_rate_code(), SYNTHETIC_RATE_CODE);
        assert_eq!(decoded.sample_rate_hz(), 11_111);
        assert_eq!(decoded.samples(), &[1, 2, 3, 4]);
    }

    #[test]
    fn voc_unknown_blocks_and_codecs_are_not_silently_ignored() {
        let unsupported_block = synthetic_voc(&[(3, &[1])]);
        assert_eq!(
            VocPcm::decode(&unsupported_block),
            Err(VocDecodeError::UnsupportedBlockType(3))
        );

        let unsupported_codec = synthetic_voc(&[(VOC_SOUND_DATA_BLOCK, &[1, 7, 2])]);
        assert_eq!(
            VocPcm::decode(&unsupported_codec),
            Err(VocDecodeError::UnsupportedCodec(7))
        );
    }

    #[test]
    fn every_shipped_music_voc_decodes_without_unsupported_blocks() {
        let Some(directory) = shipped_music_directory() else {
            return;
        };
        let mut paths = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("voc"))
            })
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths.len(), SHIPPED_VOC_RESOURCE_COUNT);

        for path in paths {
            let encoded = std::fs::read(&path).unwrap();
            let decoded = VocPcm::decode(&encoded)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(!decoded.samples().is_empty(), "{}", path.display());
            assert_ne!(decoded.sample_rate_hz(), u32::MIN, "{}", path.display());
        }
    }

    fn synthetic_voc(blocks: &[(u8, &[u8])]) -> Vec<u8> {
        let mut encoded = Vec::from(VOC_MAGIC);
        encoded.extend_from_slice(&(VOC_MINIMUM_HEADER_BYTE_COUNT as u16).to_le_bytes());
        encoded.extend_from_slice(&SYNTHETIC_VERSION.to_le_bytes());
        encoded.extend_from_slice(
            &(!SYNTHETIC_VERSION)
                .wrapping_add(VOC_VERSION_CHECKSUM_BIAS)
                .to_le_bytes(),
        );
        for (block_type, payload) in blocks {
            encoded.push(*block_type);
            let length = payload.len() as u32;
            encoded.extend_from_slice(&length.to_le_bytes()[..VOC_BLOCK_LENGTH_BYTE_COUNT]);
            encoded.extend_from_slice(payload);
        }
        encoded.push(VOC_TERMINATOR_BLOCK);
        encoded
    }

    fn shipped_music_directory() -> Option<PathBuf> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = workspace_root.join("output/_tmp_dat/mu");
        path.is_dir().then_some(path)
    }
}
