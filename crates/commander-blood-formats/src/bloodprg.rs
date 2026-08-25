//! Typed resources embedded in the original `BLOODPRG.EXE` image.

use std::error::Error;
use std::fmt;

/// Number of authored navigation anchors stored before the angle table.
pub const BLOODPRG_BRIDGE_AUTHORED_ANCHOR_COUNT: usize = 10;
/// Number of anchors consumed by the recovered bridge object projector.
pub const BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT: usize = 11;
/// Number of two-degree samples in the bridge trigonometry table.
pub const BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT: usize = 180;

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
const PROJECTION_ANCHOR_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET;
const TRIGONOMETRY_FILE_OFFSET: usize = BLOODPRG_DATA_FILE_OFFSET + BRIDGE_TRIGONOMETRY_DATA_OFFSET;
const REQUIRED_EXECUTABLE_LENGTH: usize = TRIGONOMETRY_FILE_OFFSET
    + BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT * TRIGONOMETRY_SAMPLE_BYTE_COUNT;

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
