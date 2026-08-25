//! Name-area palette-effect sequences embedded in `BLOODPRG.EXE`.

const DATA_IMAGE_FILE_OFFSET: usize = 0xD420;
const SEQUENCE_POINTER_TABLE_FILE_OFFSET: usize = 0xFC11;
const SEQUENCE_COUNT: usize = 10;
const SEQUENCE_HEADER_SIZE: usize = 2;
const FRAME_SIZE: usize = 8;
const FRAME_FIELD_SIZE: usize = 2;

/// Palette transformation applied by one authored name-area sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameAreaEffectOperation {
    /// Collapse affected pixels to the first reserved palette color.
    CollapseToFirst,
    /// Collapse affected pixels to the last reserved palette color.
    CollapseToLast,
    /// Advance affected pixels through the interior reserved colors.
    CycleForward,
    /// Move affected pixels one step toward the first reserved color.
    FadeBackward,
}

impl NameAreaEffectOperation {
    fn decode(value: u8) -> Self {
        match value {
            0 => Self::CollapseToFirst,
            1 => Self::CollapseToLast,
            2 => Self::CycleForward,
            _ => Self::FadeBackward,
        }
    }
}

/// One rectangular effect frame in the original logical display space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameAreaEffectFrame {
    /// Upper-left logical coordinate.
    pub origin: [u16; 2],
    /// Logical frame dimensions.
    pub size: [u16; 2],
}

/// One decoded effect operation and its authored frame stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameAreaEffectSequence {
    /// Palette operation applied by every frame in this sequence.
    pub operation: NameAreaEffectOperation,
    /// Authored frame stream in execution order.
    pub frames: Box<[NameAreaEffectFrame]>,
}

/// Decode all ten name-area effect sequences from the original executable.
///
/// Serialized near offsets are resolved entirely inside this decoder. Runtime
/// code receives owned frame arrays and never observes executable addresses.
pub fn decode_bloodprg_name_area_effect_sequences(
    executable: &[u8],
) -> Option<Box<[NameAreaEffectSequence]>> {
    let pointer_bytes = executable.get(
        SEQUENCE_POINTER_TABLE_FILE_OFFSET
            ..SEQUENCE_POINTER_TABLE_FILE_OFFSET.checked_add(SEQUENCE_COUNT * FRAME_FIELD_SIZE)?,
    )?;
    let mut sequences = Vec::with_capacity(SEQUENCE_COUNT);
    for pointer in pointer_bytes.chunks_exact(FRAME_FIELD_SIZE) {
        let source_offset = usize::from(u16::from_le_bytes(pointer.try_into().ok()?));
        let sequence_start = DATA_IMAGE_FILE_OFFSET.checked_add(source_offset)?;
        let header = executable.get(sequence_start..sequence_start + SEQUENCE_HEADER_SIZE)?;
        let operation = NameAreaEffectOperation::decode(header[0]);
        let frame_count = usize::from(header[1]);
        if frame_count == usize::MIN {
            return None;
        }
        let frames_start = sequence_start.checked_add(SEQUENCE_HEADER_SIZE)?;
        let frames_end = frames_start.checked_add(frame_count.checked_mul(FRAME_SIZE)?)?;
        let frame_bytes = executable.get(frames_start..frames_end)?;
        let frames = frame_bytes
            .chunks_exact(FRAME_SIZE)
            .map(|frame| NameAreaEffectFrame {
                origin: [read_word(frame, 0), read_word(frame, FRAME_FIELD_SIZE)],
                size: [
                    read_word(frame, FRAME_FIELD_SIZE * 2),
                    read_word(frame, FRAME_FIELD_SIZE * 3),
                ],
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        sequences.push(NameAreaEffectSequence { operation, frames });
    }
    Some(sequences.into_boxed_slice())
}

fn read_word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        data[offset..offset + FRAME_FIELD_SIZE]
            .try_into()
            .expect("fixed name-area frame field"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOTAL_AUTHORED_FRAME_COUNT: usize = 64;

    #[test]
    fn shipped_executable_decodes_all_authored_effect_sequences() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let sequences = decode_bloodprg_name_area_effect_sequences(executable).unwrap();

        assert_eq!(sequences.len(), SEQUENCE_COUNT);
        assert_eq!(
            sequences
                .iter()
                .map(|sequence| sequence.frames.len())
                .sum::<usize>(),
            TOTAL_AUTHORED_FRAME_COUNT
        );
        assert_eq!(
            sequences[0].operation,
            NameAreaEffectOperation::CollapseToFirst
        );
        assert_eq!(sequences[0].frames[0].origin, [16, 79]);
        assert_eq!(sequences[0].frames[0].size, [104, 70]);
        assert_eq!(
            sequences[7].operation,
            NameAreaEffectOperation::CycleForward
        );
        assert_eq!(sequences[7].frames.len(), 15);
        assert_eq!(sequences[9].frames[7].size, [1, 1]);
    }

    #[test]
    fn malformed_sequence_pointers_and_empty_streams_are_rejected() {
        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        executable[SEQUENCE_POINTER_TABLE_FILE_OFFSET..SEQUENCE_POINTER_TABLE_FILE_OFFSET + 2]
            .copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(decode_bloodprg_name_area_effect_sequences(&executable).is_none());

        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let first_pointer = usize::from(u16::from_le_bytes(
            executable[SEQUENCE_POINTER_TABLE_FILE_OFFSET..SEQUENCE_POINTER_TABLE_FILE_OFFSET + 2]
                .try_into()
                .unwrap(),
        ));
        executable[DATA_IMAGE_FILE_OFFSET + first_pointer + 1] = u8::MIN;
        assert!(decode_bloodprg_name_area_effect_sequences(&executable).is_none());
    }
}
