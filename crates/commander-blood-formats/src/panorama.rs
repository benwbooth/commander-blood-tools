//! Decoder for the `TB.BIG` bridge panorama archive.

use std::fmt;
use std::ops::Range;

/// Width of one authored panorama frame.
pub const PANORAMA_FRAME_WIDTH: usize = 320;
/// Height of one authored panorama frame.
pub const PANORAMA_FRAME_HEIGHT: usize = 200;
/// Number of indexed pixels in one authored panorama frame.
pub const PANORAMA_FRAME_PIXEL_COUNT: usize = PANORAMA_FRAME_WIDTH * PANORAMA_FRAME_HEIGHT;
/// Number of frames in the shipped full-revolution panorama.
pub const SHIPPED_PANORAMA_FRAME_COUNT: usize = 180;

const DIRECTORY_ENTRY_SIZE: usize = 8;
const FRAME_HEADER_SIZE: usize = 10;
const ORB_BOX_BYTE_COUNT: usize = 8;
const WORD_SIZE: usize = 2;

/// Bridge station associated with one panorama sector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeStation {
    /// Wide helm view and forward window.
    Helm,
    /// Golden command console.
    Console,
    /// Pyramid navigation room.
    Navigation,
    /// Organic Orxx station.
    Orxx,
}

impl BridgeStation {
    /// Number of authored bridge stations.
    pub const COUNT: usize = 4;

    /// Stable zero-based station index used by fixed runtime arrays.
    pub const fn index(self) -> usize {
        match self {
            Self::Helm => 0,
            Self::Console => 1,
            Self::Navigation => 2,
            Self::Orxx => 3,
        }
    }

    fn decode(value: u16) -> Result<Self, BridgePanoramaError> {
        match value {
            0 => Ok(Self::Helm),
            1 => Ok(Self::Console),
            2 => Ok(Self::Navigation),
            3 => Ok(Self::Orxx),
            _ => Err(BridgePanoramaError::InvalidStation(value)),
        }
    }
}

/// Inclusive clickable rectangle for the eye orb in one panorama frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeOrbBox {
    /// Upper-left logical coordinate.
    pub origin: [u16; 2],
    /// Authored inclusive width and height consumed by the native hit test.
    pub size: [u16; 2],
}

/// Typed metadata decoded from one panorama frame header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgePanoramaFrameMetadata {
    /// Bridge station owning this panorama sector.
    pub station: BridgeStation,
    /// Eye-orb hit box, or `None` for the all-ones no-hit marker.
    pub orb_box: Option<BridgeOrbBox>,
}

/// How palette index zero affects the destination framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanoramaDecodeMode {
    /// Write every decoded palette index, including zero.
    Opaque,
    /// Preserve the destination wherever the decoded palette index is zero.
    TransparentZero,
}

/// Malformed panorama archive, frame, or destination data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgePanoramaError {
    /// The archive cannot contain its first directory entry.
    ArchiveTooShort(usize),
    /// The first serialized frame position is not a complete directory size.
    InvalidDirectorySize(usize),
    /// A serialized frame range lies outside the archive or inside its directory.
    InvalidFrameRange {
        /// Zero-based directory entry.
        frame: usize,
        /// Serialized frame start.
        start: usize,
        /// Serialized frame byte count.
        size: usize,
    },
    /// The requested frame is outside the decoded directory.
    FrameOutOfRange {
        /// Requested frame.
        frame: usize,
        /// Number of decoded frames.
        count: usize,
    },
    /// A frame chunk cannot contain its typed header.
    FrameHeaderTruncated {
        /// Requested frame.
        frame: usize,
        /// Available chunk bytes.
        actual: usize,
    },
    /// A frame selected a station outside the four-station runtime array.
    InvalidStation(u16),
    /// The caller did not provide a complete logical framebuffer.
    FramebufferTooShort(usize),
    /// A ByteRun command or payload ends before the frame is complete.
    TruncatedByteRun {
        /// Serialized stream position where decoding stopped.
        stream_position: usize,
    },
    /// A ByteRun command would emit beyond one logical frame.
    ByteRunOutputOverflow {
        /// Pixels emitted before the invalid command.
        pixels_written: usize,
        /// Pixels requested by the invalid command.
        command_pixels: usize,
    },
}

impl fmt::Display for BridgePanoramaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BridgePanoramaError {}

/// Owned bridge panorama archive with serialized positions confined to decoding.
#[derive(Clone, Debug)]
pub struct BridgePanoramaArchive {
    data: Box<[u8]>,
    frame_chunks: Box<[Range<usize>]>,
}

impl BridgePanoramaArchive {
    /// Decode and validate the serialized frame directory.
    ///
    /// File positions are resolved into private checked ranges here. They are
    /// never exposed to game runtime state.
    pub fn decode(data: Box<[u8]>) -> Result<Self, BridgePanoramaError> {
        if data.len() < DIRECTORY_ENTRY_SIZE {
            return Err(BridgePanoramaError::ArchiveTooShort(data.len()));
        }
        let directory_size = usize::try_from(read_u32(&data, usize::MIN))
            .map_err(|_| BridgePanoramaError::InvalidDirectorySize(usize::MAX))?;
        if directory_size == usize::MIN
            || directory_size % DIRECTORY_ENTRY_SIZE != usize::MIN
            || directory_size > data.len()
        {
            return Err(BridgePanoramaError::InvalidDirectorySize(directory_size));
        }

        let frame_count = directory_size / DIRECTORY_ENTRY_SIZE;
        let mut frame_chunks = Vec::with_capacity(frame_count);
        for frame in 0..frame_count {
            let entry = frame * DIRECTORY_ENTRY_SIZE;
            let start = usize::try_from(read_u32(&data, entry)).map_err(|_| {
                BridgePanoramaError::InvalidFrameRange {
                    frame,
                    start: usize::MAX,
                    size: usize::MIN,
                }
            })?;
            let size = usize::try_from(read_u32(&data, entry + WORD_SIZE * 2)).map_err(|_| {
                BridgePanoramaError::InvalidFrameRange {
                    frame,
                    start,
                    size: usize::MAX,
                }
            })?;
            let end = start.checked_add(size);
            if start < directory_size || end.is_none_or(|end| end > data.len()) {
                return Err(BridgePanoramaError::InvalidFrameRange { frame, start, size });
            }
            frame_chunks.push(start..end.expect("validated panorama frame end"));
        }

        Ok(Self {
            data,
            frame_chunks: frame_chunks.into_boxed_slice(),
        })
    }

    /// Number of decoded frame directory entries.
    pub fn frame_count(&self) -> usize {
        self.frame_chunks.len()
    }

    /// Decode one frame's station and optional eye-orb rectangle.
    pub fn frame_metadata(
        &self,
        frame: usize,
    ) -> Result<BridgePanoramaFrameMetadata, BridgePanoramaError> {
        let chunk = self.frame_chunk(frame)?;
        if chunk.len() < FRAME_HEADER_SIZE {
            return Err(BridgePanoramaError::FrameHeaderTruncated {
                frame,
                actual: chunk.len(),
            });
        }
        let raw_box = [
            read_u16(chunk, usize::MIN),
            read_u16(chunk, WORD_SIZE),
            read_u16(chunk, WORD_SIZE * 2),
            read_u16(chunk, WORD_SIZE * 3),
        ];
        let orb_box = if raw_box.iter().all(|value| *value == u16::MAX) {
            None
        } else {
            Some(BridgeOrbBox {
                origin: [raw_box[0], raw_box[1]],
                size: [raw_box[2], raw_box[3]],
            })
        };
        Ok(BridgePanoramaFrameMetadata {
            station: BridgeStation::decode(read_u16(chunk, ORB_BOX_BYTE_COUNT))?,
            orb_box,
        })
    }

    /// Decode one frame over a complete flat indexed framebuffer.
    pub fn decode_frame_over(
        &self,
        frame: usize,
        framebuffer: &mut [u8],
        mode: PanoramaDecodeMode,
    ) -> Result<BridgePanoramaFrameMetadata, BridgePanoramaError> {
        let metadata = self.frame_metadata(frame)?;
        let chunk = self.frame_chunk(frame)?;
        decode_bridge_panorama_pixels(&chunk[FRAME_HEADER_SIZE..], framebuffer, mode)?;
        Ok(metadata)
    }

    /// Decode one opaque frame into a new owned logical framebuffer.
    pub fn decode_frame(
        &self,
        frame: usize,
    ) -> Result<(BridgePanoramaFrameMetadata, Box<[u8]>), BridgePanoramaError> {
        let mut pixels = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT].into_boxed_slice();
        let metadata = self.decode_frame_over(frame, &mut pixels, PanoramaDecodeMode::Opaque)?;
        Ok((metadata, pixels))
    }

    fn frame_chunk(&self, frame: usize) -> Result<&[u8], BridgePanoramaError> {
        let range = self
            .frame_chunks
            .get(frame)
            .ok_or(BridgePanoramaError::FrameOutOfRange {
                frame,
                count: self.frame_chunks.len(),
            })?;
        Ok(&self.data[range.clone()])
    }
}

/// Decode one complete panorama ByteRun stream over a flat framebuffer.
///
/// This translates `bridge_panorama_frame_unpack` at BLOODPRG routine offset
/// `0x002D50`. A temporary owned frame makes malformed input transactional;
/// checked slice indices replace source/output wrapping and inherited direction
/// state. Every valid authored stream retains the original pixel result.
pub fn decode_bridge_panorama_pixels(
    stream: &[u8],
    framebuffer: &mut [u8],
    mode: PanoramaDecodeMode,
) -> Result<(), BridgePanoramaError> {
    if framebuffer.len() < PANORAMA_FRAME_PIXEL_COUNT {
        return Err(BridgePanoramaError::FramebufferTooShort(framebuffer.len()));
    }

    let mut decoded = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT];
    let mut source = usize::MIN;
    let mut destination = usize::MIN;
    while destination < PANORAMA_FRAME_PIXEL_COUNT {
        let control = *stream
            .get(source)
            .ok_or(BridgePanoramaError::TruncatedByteRun {
                stream_position: source,
            })? as i8;
        source += 1;
        let count = if control < 0 {
            usize::try_from(1_i16 - i16::from(control)).expect("signed byte run length")
        } else {
            usize::from(control as u8) + 1
        };
        let end = destination.checked_add(count);
        if end.is_none_or(|end| end > PANORAMA_FRAME_PIXEL_COUNT) {
            return Err(BridgePanoramaError::ByteRunOutputOverflow {
                pixels_written: destination,
                command_pixels: count,
            });
        }
        let end = end.expect("validated panorama output end");

        if control < 0 {
            let value = *stream
                .get(source)
                .ok_or(BridgePanoramaError::TruncatedByteRun {
                    stream_position: source,
                })?;
            source += 1;
            decoded[destination..end].fill(value);
        } else {
            let literal_end = source.checked_add(count);
            let literals = stream
                .get(source..literal_end.unwrap_or(usize::MAX))
                .ok_or(BridgePanoramaError::TruncatedByteRun {
                    stream_position: source,
                })?;
            decoded[destination..end].copy_from_slice(literals);
            source = literal_end.expect("validated panorama literal end");
        }
        destination = end;
    }

    match mode {
        PanoramaDecodeMode::Opaque => {
            framebuffer[..PANORAMA_FRAME_PIXEL_COUNT].copy_from_slice(&decoded)
        }
        PanoramaDecodeMode::TransparentZero => {
            for (destination, source) in framebuffer[..PANORAMA_FRAME_PIXEL_COUNT]
                .iter_mut()
                .zip(decoded)
            {
                if source != u8::MIN {
                    *destination = source;
                }
            }
        }
    }
    Ok(())
}

fn read_u16(data: &[u8], start: usize) -> u16 {
    u16::from_le_bytes(
        data[start..start + WORD_SIZE]
            .try_into()
            .expect("validated panorama word"),
    )
}

fn read_u32(data: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(
        data[start..start + WORD_SIZE * 2]
            .try_into()
            .expect("validated panorama directory field"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde::Deserialize;

    use super::*;

    const UNPACK_ORACLE_VECTOR_COUNT: usize = 4;
    const SYNTHETIC_TRANSPARENT_SEED: u8 = 204;
    const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
    const ORIGINAL_ARCHIVE_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ORIGINAL_ARCHIVE_ROOT";
    const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";

    #[derive(Deserialize)]
    struct UnpackOracle {
        name: String,
        transparent_zero: bool,
        source_bytes: usize,
    }

    #[test]
    fn byte_run_decoder_matches_all_flat_original_semantic_vectors() {
        let vectors: Vec<UnpackOracle> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/func_2d50_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), UNPACK_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let (stream, expected) = oracle_stream(vector.transparent_zero);
            assert_eq!(stream.len(), vector.source_bytes, "{}", vector.name);
            let mut framebuffer = if vector.transparent_zero {
                vec![SYNTHETIC_TRANSPARENT_SEED; PANORAMA_FRAME_PIXEL_COUNT]
            } else {
                vec![u8::MAX; PANORAMA_FRAME_PIXEL_COUNT]
            };
            decode_bridge_panorama_pixels(
                &stream,
                &mut framebuffer,
                if vector.transparent_zero {
                    PanoramaDecodeMode::TransparentZero
                } else {
                    PanoramaDecodeMode::Opaque
                },
            )
            .unwrap();
            assert_eq!(framebuffer, expected, "{}", vector.name);
        }
    }

    #[test]
    fn shipped_archive_decodes_every_frame_and_station_sector() {
        let Some(path) = shipped_archive() else {
            return;
        };
        let archive =
            BridgePanoramaArchive::decode(std::fs::read(path).unwrap().into_boxed_slice()).unwrap();
        assert_eq!(archive.frame_count(), SHIPPED_PANORAMA_FRAME_COUNT);

        let mut stations = Vec::with_capacity(SHIPPED_PANORAMA_FRAME_COUNT);
        for frame in 0..archive.frame_count() {
            let (metadata, pixels) = archive.decode_frame(frame).unwrap();
            assert_eq!(pixels.len(), PANORAMA_FRAME_PIXEL_COUNT);
            stations.push(metadata.station);
        }
        assert!(
            stations[0..=21]
                .iter()
                .all(|station| *station == BridgeStation::Helm)
        );
        assert!(
            stations[22..=71]
                .iter()
                .all(|station| *station == BridgeStation::Console)
        );
        assert!(
            stations[72..=107]
                .iter()
                .all(|station| *station == BridgeStation::Navigation)
        );
        assert!(
            stations[108..=159]
                .iter()
                .all(|station| *station == BridgeStation::Orxx)
        );
        assert!(
            stations[160..]
                .iter()
                .all(|station| *station == BridgeStation::Helm)
        );

        assert_eq!(
            archive.frame_metadata(usize::MIN).unwrap(),
            BridgePanoramaFrameMetadata {
                station: BridgeStation::Helm,
                orb_box: Some(BridgeOrbBox {
                    origin: [133, 130],
                    size: [51, 44],
                }),
            }
        );
    }

    #[test]
    fn malformed_archives_and_runs_fail_without_partial_framebuffer_changes() {
        assert_eq!(
            BridgePanoramaArchive::decode(vec![0; DIRECTORY_ENTRY_SIZE - 1].into_boxed_slice())
                .unwrap_err(),
            BridgePanoramaError::ArchiveTooShort(DIRECTORY_ENTRY_SIZE - 1)
        );

        let mut framebuffer = vec![37; PANORAMA_FRAME_PIXEL_COUNT];
        let before = framebuffer.clone();
        assert!(matches!(
            decode_bridge_panorama_pixels(
                &[u8::MAX, 1],
                &mut framebuffer,
                PanoramaDecodeMode::Opaque
            ),
            Err(BridgePanoramaError::TruncatedByteRun { .. })
        ));
        assert_eq!(framebuffer, before);
    }

    fn shipped_archive() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(root) = std::env::var_os(ASSET_CACHE_ENVIRONMENT_VARIABLE) {
            let root = PathBuf::from(root);
            candidates.push(root.join("companions/TB.BIG"));
            candidates.push(root.join("resources/TB.BIG"));
        }
        if let Some(root) = std::env::var_os(ORIGINAL_ARCHIVE_ROOT_ENVIRONMENT_VARIABLE) {
            candidates.push(PathBuf::from(root).join("TB.BIG"));
        }
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/_tmp_iso/TB.BIG"));
        if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
            return Some(path);
        }
        assert!(
            std::env::var_os(REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE).is_none(),
            "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires a configured TB.BIG"
        );
        None
    }

    fn oracle_stream(transparent: bool) -> (Vec<u8>, Vec<u8>) {
        let mut stream = Vec::new();
        let mut output = vec![
            if transparent {
                SYNTHETIC_TRANSPARENT_SEED
            } else {
                u8::MIN
            };
            PANORAMA_FRAME_PIXEL_COUNT
        ];
        let mut cursor = usize::MIN;

        if transparent {
            append_repeat(
                &mut stream,
                &mut output,
                &mut cursor,
                transparent,
                5,
                u8::MIN,
            );
            append_literal(
                &mut stream,
                &mut output,
                &mut cursor,
                transparent,
                &[1, 0, 2, 0, 3],
            );
            append_repeat(&mut stream, &mut output, &mut cursor, transparent, 4, 7);
            for run in 0..496 {
                let value = if run & 1 == usize::MIN {
                    u8::MIN
                } else {
                    (run * 13 + 17) as u8
                };
                append_repeat(
                    &mut stream,
                    &mut output,
                    &mut cursor,
                    transparent,
                    129,
                    value,
                );
            }
            append_repeat(&mut stream, &mut output, &mut cursor, transparent, 2, 9);
        } else {
            append_literal(
                &mut stream,
                &mut output,
                &mut cursor,
                transparent,
                &[1, 2, 0, 4],
            );
            append_repeat(
                &mut stream,
                &mut output,
                &mut cursor,
                transparent,
                2,
                u8::MIN,
            );
            for run in 0..496 {
                append_repeat(
                    &mut stream,
                    &mut output,
                    &mut cursor,
                    transparent,
                    129,
                    (run * 13 + 17) as u8,
                );
            }
            append_repeat(&mut stream, &mut output, &mut cursor, transparent, 10, 167);
        }
        assert_eq!(cursor, PANORAMA_FRAME_PIXEL_COUNT);
        (stream, output)
    }

    fn append_repeat(
        stream: &mut Vec<u8>,
        output: &mut [u8],
        cursor: &mut usize,
        transparent: bool,
        count: usize,
        value: u8,
    ) {
        stream.push((1_i16 - count as i16) as u8);
        stream.push(value);
        if !transparent || value != u8::MIN {
            output[*cursor..*cursor + count].fill(value);
        }
        *cursor += count;
    }

    fn append_literal(
        stream: &mut Vec<u8>,
        output: &mut [u8],
        cursor: &mut usize,
        transparent: bool,
        values: &[u8],
    ) {
        stream.push((values.len() - 1) as u8);
        stream.extend_from_slice(values);
        for value in values {
            if !transparent || *value != u8::MIN {
                output[*cursor] = *value;
            }
            *cursor += 1;
        }
    }
}
