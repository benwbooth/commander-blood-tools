//! BloodScript-selected DESCRIPT sequence names stored as ordinary owned values.

use std::fmt;

use commander_blood_formats::instruction::{
    ScriptSequenceSlot, ScriptSequenceSlotAssignment, ScriptSequenceSlotName,
};

/// Byte count of the six fixed sequence-name fields in an original save.
pub const SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT: usize = 96;

const SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT: usize = 16;
const INITIAL_SEQUENCE_NAME: &[u8] = b"present";

/// Invalid fixed sequence-name data in an original save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptSequenceSaveError {
    /// One fixed field has no NUL terminator.
    UnterminatedSlot {
        /// One-based slot number used by BloodScript.
        slot: u8,
    },
}

impl fmt::Display for ScriptSequenceSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid saved sequence-name block: {self:?}")
    }
}

impl std::error::Error for ScriptSequenceSaveError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ScriptSequenceSlotValue {
    name: Option<ScriptSequenceSlotName>,
    serialized_field: [u8; SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT],
}

/// Six presentation sequence names retained across BloodScript execution.
///
/// The original implementation used adjacent fixed-width fields. The modern
/// port preserves their stable order and replacement behavior without exposing
/// byte offsets or allowing one assignment to overwrite another slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSequenceSlots {
    slots: [ScriptSequenceSlotValue; ScriptSequenceSlot::COUNT],
}

impl ScriptSequenceSlots {
    /// Return the name currently assigned to one slot.
    pub fn name(&self, slot: ScriptSequenceSlot) -> Option<&ScriptSequenceSlotName> {
        self.slots[slot.index()].name.as_ref()
    }

    /// Apply `vm_op_cc_set_record_byte` as one bounded owned-name replacement.
    pub fn assign(&mut self, assignment: ScriptSequenceSlotAssignment) {
        let (slot, name) = assignment.into_parts();
        let value = &mut self.slots[slot.index()];
        let name_bytes = name.as_bytes();
        value.serialized_field[..name_bytes.len()].copy_from_slice(name_bytes);
        value.serialized_field[name_bytes.len()] = u8::MIN;
        value.name = Some(name);
    }

    /// Explicitly clear all six names and their retained fixed-field bytes.
    ///
    /// Script-profile selection does not call this operation: the native fields
    /// are global and survive profile changes.
    pub fn clear(&mut self) {
        *self = Self::empty();
    }

    /// Encode all six fields exactly as the original save path writes them.
    pub fn encode_save_block(&self) -> [u8; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT] {
        let mut block = [u8::MIN; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT];
        for (value, destination) in self
            .slots
            .iter()
            .zip(block.chunks_exact_mut(SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT))
        {
            destination.copy_from_slice(&value.serialized_field);
        }
        block
    }

    /// Restore all six fixed fields transactionally from an original save.
    pub fn restore_save_block(
        &mut self,
        block: &[u8; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT],
    ) -> Result<(), ScriptSequenceSaveError> {
        let mut restored = Self::empty();
        for (index, source) in block
            .chunks_exact(SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT)
            .enumerate()
        {
            let terminator = source.iter().position(|byte| *byte == u8::MIN).ok_or(
                ScriptSequenceSaveError::UnterminatedSlot {
                    slot: (index + 1) as u8,
                },
            )?;
            let value = &mut restored.slots[index];
            value.serialized_field.copy_from_slice(source);
            if terminator != usize::MIN {
                value.name = Some(
                    ScriptSequenceSlotName::new(Box::<[u8]>::from(&source[..terminator]))
                        .expect("NUL-terminated 16-byte field has a bounded name"),
                );
            }
        }
        *self = restored;
        Ok(())
    }

    fn empty() -> Self {
        Self {
            slots: std::array::from_fn(|_| ScriptSequenceSlotValue::default()),
        }
    }
}

impl Default for ScriptSequenceSlots {
    fn default() -> Self {
        let mut slots = Self::empty();
        let first = &mut slots.slots[usize::MIN];
        first.serialized_field[..INITIAL_SEQUENCE_NAME.len()]
            .copy_from_slice(INITIAL_SEQUENCE_NAME);
        first.name = Some(
            ScriptSequenceSlotName::new(Box::<[u8]>::from(INITIAL_SEQUENCE_NAME))
                .expect("initial sequence name fits its native field"),
        );
        slots
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::{
        ScriptSequenceSlotName, decode_script_sequence_slot_assignment,
    };
    use serde::Deserialize;

    use super::*;

    const SEQUENCE_SLOT_ASSIGNMENT_OPCODE: u8 = 0xCC;
    const CODE_END_MARKER: u8 = 0xFF;
    const ORACLE_VECTOR_COUNT: usize = 9;

    #[derive(Deserialize)]
    struct SequenceSlotOracle {
        name: String,
        slot_byte: u8,
        copied_byte_count: usize,
    }

    fn encoded_assignment(slot: u8, copied_byte_count: usize) -> Vec<u8> {
        let mut bytes = vec![SEQUENCE_SLOT_ASSIGNMENT_OPCODE, slot];
        bytes.extend(std::iter::repeat_n(
            b'x',
            copied_byte_count.saturating_sub(1),
        ));
        bytes.extend_from_slice(&[u8::MIN, u8::MIN, CODE_END_MARKER]);
        bytes
    }

    #[test]
    fn bounded_assignments_match_every_semantically_valid_original_vector() {
        let vectors: Vec<SequenceSlotOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64ce_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let code = decode_script_code(&encoded_assignment(
                vector.slot_byte,
                vector.copied_byte_count,
            ))
            .unwrap();
            let decoded = decode_script_sequence_slot_assignment(&code.tokens()[0]);
            let valid = ScriptSequenceSlot::decode(vector.slot_byte).is_some()
                && vector.copied_byte_count <= ScriptSequenceSlotName::MAXIMUM_BYTE_LENGTH + 1;

            if valid {
                let assignment = decoded.unwrap_or_else(|error| {
                    panic!("{} should be a bounded assignment: {error}", vector.name)
                });
                let slot = assignment.slot();
                let expected_length = vector.copied_byte_count - 1;
                let mut slots = ScriptSequenceSlots::default();
                slots.assign(assignment);
                assert_eq!(
                    slots.name(slot).unwrap().as_bytes(),
                    vec![b'x'; expected_length],
                    "{}",
                    vector.name
                );
            } else {
                assert!(decoded.is_err(), "{} must be rejected", vector.name);
            }
        }
    }

    #[test]
    fn assignments_replace_only_the_selected_owned_name() {
        let decode = |slot, name: &[u8]| {
            let mut bytes = vec![SEQUENCE_SLOT_ASSIGNMENT_OPCODE, slot];
            bytes.extend_from_slice(name);
            bytes.extend_from_slice(&[u8::MIN, u8::MIN, CODE_END_MARKER]);
            let code = decode_script_code(&bytes).unwrap();
            decode_script_sequence_slot_assignment(&code.tokens()[0]).unwrap()
        };
        let first = ScriptSequenceSlot::decode(1).unwrap();
        let second = ScriptSequenceSlot::decode(2).unwrap();
        let mut slots = ScriptSequenceSlots::default();

        slots.assign(decode(1, b"match"));
        slots.assign(decode(2, b"ppit"));
        slots.assign(decode(1, b"present"));

        assert_eq!(slots.name(first).unwrap().as_bytes(), b"present");
        assert_eq!(slots.name(second).unwrap().as_bytes(), b"ppit");
        slots.clear();
        assert_eq!(slots.name(first), None);
        assert_eq!(slots.name(second), None);
    }

    #[test]
    fn initial_slots_match_the_executable_static_fields() {
        const BLOODPRG_INITIAL_SEQUENCE_SLOTS_FILE_OFFSET: usize = 0x0140FE;

        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let expected = &executable[BLOODPRG_INITIAL_SEQUENCE_SLOTS_FILE_OFFSET
            ..BLOODPRG_INITIAL_SEQUENCE_SLOTS_FILE_OFFSET + SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT];
        let slots = ScriptSequenceSlots::default();
        let first = ScriptSequenceSlot::decode(1).unwrap();

        assert_eq!(slots.name(first).unwrap().as_bytes(), INITIAL_SEQUENCE_NAME);
        assert_eq!(slots.encode_save_block(), expected);
    }

    #[test]
    fn shorter_assignments_preserve_bytes_after_the_new_terminator() {
        let mut source = [u8::MIN; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT];
        source[..SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT].copy_from_slice(b"longer\0tail-data");
        let mut slots = ScriptSequenceSlots::empty();
        slots.restore_save_block(&source).unwrap();

        let code = decode_script_code(&[
            SEQUENCE_SLOT_ASSIGNMENT_OPCODE,
            1,
            b'x',
            u8::MIN,
            u8::MIN,
            CODE_END_MARKER,
        ])
        .unwrap();
        slots.assign(decode_script_sequence_slot_assignment(&code.tokens()[0]).unwrap());

        let encoded = slots.encode_save_block();
        assert_eq!(&encoded[..2], b"x\0");
        assert_eq!(
            &encoded[2..SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT],
            &source[2..SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT]
        );
    }

    #[test]
    fn restoring_an_unterminated_field_is_transactional() {
        let mut slots = ScriptSequenceSlots::default();
        let before = slots.clone();
        let mut malformed = slots.encode_save_block();
        malformed[SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT..SCRIPT_SEQUENCE_SLOT_FIELD_BYTE_COUNT * 2]
            .fill(b'x');

        assert_eq!(
            slots.restore_save_block(&malformed).unwrap_err(),
            ScriptSequenceSaveError::UnterminatedSlot { slot: 2 }
        );
        assert_eq!(slots, before);
    }
}
