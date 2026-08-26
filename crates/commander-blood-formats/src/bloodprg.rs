//! Typed resources embedded in the original `BLOODPRG.EXE` image.

use std::error::Error;
use std::fmt;

/// Number of authored navigation anchors stored before the angle table.
pub const BLOODPRG_BRIDGE_AUTHORED_ANCHOR_COUNT: usize = 10;
/// Number of anchors consumed by the recovered bridge object projector.
pub const BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT: usize = 11;
/// Number of two-degree samples in the bridge trigonometry table.
pub const BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT: usize = 180;
/// Number of byte values represented by the proportional-font character maps.
pub const BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT: usize = 176;
/// Number of byte values represented by the compact small-font character map.
pub const BLOODPRG_SMALL_FONT_CHARACTER_COUNT: usize = 128;
/// Byte count consumed by the dual-font measurement routine from each advance base.
pub const BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT: usize = 256;
/// Number of square-cap glyphs embedded in the executable.
pub const BLOODPRG_SQUARE_CAPS_GLYPH_COUNT: usize = 48;
/// Number of main dialogue glyphs embedded in the executable.
pub const BLOODPRG_MAIN_FONT_GLYPH_COUNT: usize = 86;
/// Number of subtitle-console glyphs embedded in the executable.
pub const BLOODPRG_SUBTITLE_FONT_GLYPH_COUNT: usize = 55;
/// Number of compact small-font glyphs embedded in the executable.
pub const BLOODPRG_SMALL_FONT_GLYPH_COUNT: usize = 42;

const MZ_SIGNATURE: [u8; 2] = [b'M', b'Z'];
const MZ_SIGNATURE_FILE_OFFSET: usize = 0;
const BLOODPRG_DATA_FILE_OFFSET: usize = 0xD420;
const BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET: usize = 0x4F09;
const BRIDGE_TRIGONOMETRY_DATA_OFFSET: usize = 0x4F45;
const POSITION_COMPONENT_COUNT: usize = 3;
const TRIGONOMETRY_COMPONENT_COUNT: usize = 2;
const WORD_BYTE_COUNT: usize = 2;
const PROJECTION_ANCHOR_BYTE_COUNT: usize = POSITION_COMPONENT_COUNT * WORD_BYTE_COUNT;
const TRIGONOMETRY_SAMPLE_BYTE_COUNT: usize = TRIGONOMETRY_COMPONENT_COUNT * WORD_BYTE_COUNT;
const SQUARE_CAPS_GLYPH_HEIGHT: usize = 10;
const SQUARE_CAPS_ROW_BYTE_COUNT: usize = 2;
const MAIN_FONT_GLYPH_HEIGHT: usize = 8;
const SUBTITLE_FONT_GLYPH_HEIGHT: usize = 8;
const SMALL_FONT_GLYPH_HEIGHT: usize = 5;
const SMALL_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x6FA8;
const SMALL_FONT_GLYPH_DATA_OFFSET: usize = 0x7028;
const SUBTITLE_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x70FA;
const SUBTITLE_FONT_GLYPH_DATA_OFFSET: usize = 0x71AA;
const SQUARE_CAPS_CHARACTER_MAP_DATA_OFFSET: usize = 0x7362;
const SQUARE_CAPS_ADVANCE_DATA_OFFSET: usize = 0x7412;
const SQUARE_CAPS_GLYPH_DATA_OFFSET: usize = 0x7442;
const MAIN_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x7802;
const MAIN_FONT_ADVANCE_DATA_OFFSET: usize = 0x78B2;
const MAIN_FONT_GLYPH_DATA_OFFSET: usize = 0x7908;
const SQUARE_CAPS_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SQUARE_CAPS_GLYPH_COUNT * SQUARE_CAPS_GLYPH_HEIGHT * SQUARE_CAPS_ROW_BYTE_COUNT;
const MAIN_FONT_GLYPH_BYTE_COUNT: usize = BLOODPRG_MAIN_FONT_GLYPH_COUNT * MAIN_FONT_GLYPH_HEIGHT;
const SUBTITLE_FONT_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SUBTITLE_FONT_GLYPH_COUNT * SUBTITLE_FONT_GLYPH_HEIGHT;
const SMALL_FONT_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SMALL_FONT_GLYPH_COUNT * SMALL_FONT_GLYPH_HEIGHT;
const PROJECTION_ANCHOR_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET;
const TRIGONOMETRY_FILE_OFFSET: usize = BLOODPRG_DATA_FILE_OFFSET + BRIDGE_TRIGONOMETRY_DATA_OFFSET;
const SMALL_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SMALL_FONT_CHARACTER_MAP_DATA_OFFSET;
const SMALL_FONT_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SMALL_FONT_GLYPH_DATA_OFFSET;
const SUBTITLE_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_FONT_CHARACTER_MAP_DATA_OFFSET;
const SUBTITLE_FONT_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_FONT_GLYPH_DATA_OFFSET;
const SQUARE_CAPS_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_CHARACTER_MAP_DATA_OFFSET;
const SQUARE_CAPS_ADVANCE_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_ADVANCE_DATA_OFFSET;
const SQUARE_CAPS_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_GLYPH_DATA_OFFSET;
const MAIN_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_CHARACTER_MAP_DATA_OFFSET;
const MAIN_FONT_ADVANCE_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_ADVANCE_DATA_OFFSET;
const MAIN_FONT_GLYPH_FILE_OFFSET: usize = BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_GLYPH_DATA_OFFSET;
const REQUIRED_EXECUTABLE_LENGTH: usize = TRIGONOMETRY_FILE_OFFSET
    + BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
const FONT_REQUIRED_EXECUTABLE_LENGTH: usize =
    MAIN_FONT_GLYPH_FILE_OFFSET + MAIN_FONT_GLYPH_BYTE_COUNT;

/// One world-space navigation anchor decoded from the executable image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgBridgeAnchor {
    /// Three wrapping source-coordinate components.
    pub position: [u16; POSITION_COMPONENT_COUNT],
}

/// One signed Q14 cosine and sine pair from the executable image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgBridgeTrigonometrySample {
    /// Cosine at this two-degree step.
    pub cosine: i16,
    /// Sine at this two-degree step.
    pub sine: i16,
}

/// Complete bridge projection resources decoded from `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgBridgeResources {
    /// Eleven projector inputs, including the recovered final overlapping read.
    pub projection_anchors: [BloodprgBridgeAnchor; BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT],
    /// Complete authored two-degree angle table.
    pub trigonometry: [BloodprgBridgeTrigonometrySample; BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT],
}

/// Complete font maps, advances, and glyph bitmaps embedded in `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgFontResources {
    /// Byte-to-glyph map used by the compact five-row font.
    pub small_character_map: [u8; BLOODPRG_SMALL_FONT_CHARACTER_COUNT],
    /// Five one-byte rows for every compact glyph.
    pub small_glyphs: [u8; SMALL_FONT_GLYPH_BYTE_COUNT],
    /// Byte-to-glyph map used by the fixed-width subtitle console font.
    pub subtitle_character_map: [u8; BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT],
    /// Eight one-byte rows for every subtitle glyph.
    pub subtitle_glyphs: [u8; SUBTITLE_FONT_GLYPH_BYTE_COUNT],
    /// Byte-to-glyph map used by the square-cap UI font.
    pub square_caps_character_map: [u8; BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT],
    /// Signed-byte pen advances indexed by square-cap glyph number.
    pub square_caps_advances: [u8; BLOODPRG_SQUARE_CAPS_GLYPH_COUNT],
    /// Complete unsigned lookup region consumed by dual-font measurement.
    pub square_caps_measurement_advances: [u8; BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT],
    /// Ten big-endian two-byte rows for every square-cap glyph.
    pub square_caps_glyphs: [u8; SQUARE_CAPS_GLYPH_BYTE_COUNT],
    /// Byte-to-glyph map used by the main dialogue font.
    pub main_character_map: [u8; BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT],
    /// Signed-byte pen advances indexed by main-font glyph number.
    pub main_advances: [u8; BLOODPRG_MAIN_FONT_GLYPH_COUNT],
    /// Complete unsigned lookup region consumed by dual-font measurement.
    pub main_measurement_advances: [u8; BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT],
    /// Eight one-byte rows for every main-font glyph.
    pub main_glyphs: [u8; MAIN_FONT_GLYPH_BYTE_COUNT],
}

/// Malformed or truncated executable font resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgFontResourceError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// A recovered font range extends beyond the supplied executable image.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by every font table.
        required: usize,
    },
}

impl fmt::Display for BloodprgFontResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG font resources: {self:?}")
    }
}

impl Error for BloodprgFontResourceError {}

/// Malformed or truncated `BLOODPRG.EXE` bridge resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgBridgeResourceError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// A fixed bridge resource range extends beyond the supplied image.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by all decoded ranges.
        required: usize,
    },
}

impl fmt::Display for BloodprgBridgeResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG bridge resources: {self:?}")
    }
}

impl Error for BloodprgBridgeResourceError {}

/// Decode bridge projection anchors and trigonometry into owned arrays.
///
/// The ten authored anchors end at the angle table. The original projector
/// consumes eleven records, so its final six-byte input is decoded from the
/// beginning of that adjacent table. This overlap is resolved here once; game
/// code receives independent typed arrays and never handles executable offsets.
pub fn decode_bloodprg_bridge_resources(
    executable: &[u8],
) -> Result<BloodprgBridgeResources, BloodprgBridgeResourceError> {
    if executable.len() < REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgBridgeResourceError::TruncatedExecutable {
            actual: executable.len(),
            required: REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgBridgeResourceError::InvalidExecutableSignature);
    }

    let mut projection_anchors =
        [BloodprgBridgeAnchor::default(); BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT];
    for (index, anchor) in projection_anchors.iter_mut().enumerate() {
        let position = PROJECTION_ANCHOR_FILE_OFFSET + index * PROJECTION_ANCHOR_BYTE_COUNT;
        anchor.position = std::array::from_fn(|component| {
            read_unsigned_word(executable, position + component * WORD_BYTE_COUNT)
        });
    }

    let mut trigonometry =
        [BloodprgBridgeTrigonometrySample::default(); BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT];
    for (index, sample) in trigonometry.iter_mut().enumerate() {
        let position = TRIGONOMETRY_FILE_OFFSET + index * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
        sample.cosine = read_signed_word(executable, position);
        sample.sine = read_signed_word(executable, position + WORD_BYTE_COUNT);
    }

    Ok(BloodprgBridgeResources {
        projection_anchors,
        trigonometry,
    })
}

/// Decode every recovered font table into owned flat arrays.
///
/// Executable positions are resolved once at load time. Render and measurement
/// code receives ordinary arrays and does not retain DOS data-segment offsets
/// or rely on adjacency between unrelated native tables.
pub fn decode_bloodprg_font_resources(
    executable: &[u8],
) -> Result<BloodprgFontResources, BloodprgFontResourceError> {
    if executable.len() < FONT_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgFontResourceError::TruncatedExecutable {
            actual: executable.len(),
            required: FONT_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgFontResourceError::InvalidExecutableSignature);
    }

    Ok(BloodprgFontResources {
        small_character_map: read_byte_array(executable, SMALL_FONT_CHARACTER_MAP_FILE_OFFSET),
        small_glyphs: read_byte_array(executable, SMALL_FONT_GLYPH_FILE_OFFSET),
        subtitle_character_map: read_byte_array(
            executable,
            SUBTITLE_FONT_CHARACTER_MAP_FILE_OFFSET,
        ),
        subtitle_glyphs: read_byte_array(executable, SUBTITLE_FONT_GLYPH_FILE_OFFSET),
        square_caps_character_map: read_byte_array(
            executable,
            SQUARE_CAPS_CHARACTER_MAP_FILE_OFFSET,
        ),
        square_caps_advances: read_byte_array(executable, SQUARE_CAPS_ADVANCE_FILE_OFFSET),
        square_caps_measurement_advances: read_byte_array(
            executable,
            SQUARE_CAPS_ADVANCE_FILE_OFFSET,
        ),
        square_caps_glyphs: read_byte_array(executable, SQUARE_CAPS_GLYPH_FILE_OFFSET),
        main_character_map: read_byte_array(executable, MAIN_FONT_CHARACTER_MAP_FILE_OFFSET),
        main_advances: read_byte_array(executable, MAIN_FONT_ADVANCE_FILE_OFFSET),
        main_measurement_advances: read_byte_array(executable, MAIN_FONT_ADVANCE_FILE_OFFSET),
        main_glyphs: read_byte_array(executable, MAIN_FONT_GLYPH_FILE_OFFSET),
    })
}

fn read_byte_array<const BYTE_COUNT: usize>(data: &[u8], position: usize) -> [u8; BYTE_COUNT] {
    data[position..position + BYTE_COUNT]
        .try_into()
        .expect("validated BLOODPRG font range")
}

fn read_unsigned_word(data: &[u8], position: usize) -> u16 {
    u16::from_le_bytes([data[position], data[position + 1]])
}

fn read_signed_word(data: &[u8], position: usize) -> i16 {
    i16::from_le_bytes([data[position], data[position + 1]])
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const RESOURCE_ORACLE_COUNT: usize = 1;

    #[derive(Deserialize)]
    struct BridgeResourceOracle {
        data_file_offset: usize,
        projection_anchor_offset: usize,
        authored_anchor_count: usize,
        projection_anchor_count: usize,
        anchors: Vec<[u16; POSITION_COMPONENT_COUNT]>,
        trigonometry_offset: usize,
        trigonometry_count: usize,
        trigonometry: Vec<[i16; TRIGONOMETRY_COMPONENT_COUNT]>,
    }

    #[test]
    fn bridge_resources_match_every_original_executable_value() {
        let vectors: Vec<BridgeResourceOracle> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/bloodprg_bridge_resources.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RESOURCE_ORACLE_COUNT);

        for vector in vectors {
            assert_eq!(vector.data_file_offset, BLOODPRG_DATA_FILE_OFFSET);
            assert_eq!(
                vector.projection_anchor_offset,
                BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET
            );
            assert_eq!(
                vector.authored_anchor_count,
                BLOODPRG_BRIDGE_AUTHORED_ANCHOR_COUNT
            );
            assert_eq!(
                vector.projection_anchor_count,
                BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT
            );
            assert_eq!(vector.trigonometry_offset, BRIDGE_TRIGONOMETRY_DATA_OFFSET);
            assert_eq!(
                vector.trigonometry_count,
                BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT
            );

            let executable = executable_fixture(&vector);
            let resources = decode_bloodprg_bridge_resources(&executable).unwrap();
            assert_eq!(
                resources
                    .projection_anchors
                    .map(|anchor| anchor.position)
                    .as_slice(),
                vector.anchors
            );
            assert_eq!(
                resources
                    .trigonometry
                    .map(|sample| [sample.cosine, sample.sine])
                    .as_slice(),
                vector.trigonometry
            );
        }
    }

    #[test]
    fn malformed_executables_are_rejected_before_decoding() {
        assert_eq!(
            decode_bloodprg_bridge_resources(&[]),
            Err(BloodprgBridgeResourceError::TruncatedExecutable {
                actual: usize::MIN,
                required: REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let truncated = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH - 1];
        assert_eq!(
            decode_bloodprg_bridge_resources(&truncated),
            Err(BloodprgBridgeResourceError::TruncatedExecutable {
                actual: REQUIRED_EXECUTABLE_LENGTH - 1,
                required: REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let invalid_signature = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_bridge_resources(&invalid_signature),
            Err(BloodprgBridgeResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    fn executable_font_tables_decode_into_independent_owned_arrays() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let resources = decode_bloodprg_font_resources(executable).unwrap();

        assert_eq!(
            resources.square_caps_advances,
            resources.square_caps_measurement_advances[..BLOODPRG_SQUARE_CAPS_GLYPH_COUNT]
        );
        assert_eq!(
            resources.main_advances,
            resources.main_measurement_advances[..BLOODPRG_MAIN_FONT_GLYPH_COUNT]
        );
        assert_ne!(
            resources.small_glyphs,
            [u8::MIN; SMALL_FONT_GLYPH_BYTE_COUNT]
        );
        assert_ne!(
            resources.subtitle_glyphs,
            [u8::MIN; SUBTITLE_FONT_GLYPH_BYTE_COUNT]
        );
        assert_ne!(
            resources.square_caps_glyphs,
            [u8::MIN; SQUARE_CAPS_GLYPH_BYTE_COUNT]
        );
        assert_ne!(resources.main_glyphs, [u8::MIN; MAIN_FONT_GLYPH_BYTE_COUNT]);
    }

    #[test]
    fn malformed_font_executables_are_rejected_before_decoding() {
        assert_eq!(
            decode_bloodprg_font_resources(&[]),
            Err(BloodprgFontResourceError::TruncatedExecutable {
                actual: usize::MIN,
                required: FONT_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let truncated = vec![u8::MIN; FONT_REQUIRED_EXECUTABLE_LENGTH - 1];
        assert_eq!(
            decode_bloodprg_font_resources(&truncated),
            Err(BloodprgFontResourceError::TruncatedExecutable {
                actual: FONT_REQUIRED_EXECUTABLE_LENGTH - 1,
                required: FONT_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let invalid_signature = vec![u8::MIN; FONT_REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_font_resources(&invalid_signature),
            Err(BloodprgFontResourceError::InvalidExecutableSignature)
        );
    }

    fn executable_fixture(vector: &BridgeResourceOracle) -> Vec<u8> {
        let mut executable = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH];
        executable[MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()].copy_from_slice(&MZ_SIGNATURE);
        for (index, anchor) in vector.anchors.iter().copied().enumerate() {
            let position = PROJECTION_ANCHOR_FILE_OFFSET + index * PROJECTION_ANCHOR_BYTE_COUNT;
            for (component, value) in anchor.into_iter().enumerate() {
                let component_position = position + component * WORD_BYTE_COUNT;
                executable[component_position..component_position + WORD_BYTE_COUNT]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        for (index, sample) in vector.trigonometry.iter().copied().enumerate() {
            let position = TRIGONOMETRY_FILE_OFFSET + index * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
            for (component, value) in sample.into_iter().enumerate() {
                let component_position = position + component * WORD_BYTE_COUNT;
                executable[component_position..component_position + WORD_BYTE_COUNT]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        executable
    }
}
