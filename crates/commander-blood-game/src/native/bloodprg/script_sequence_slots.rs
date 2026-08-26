//! BloodScript-selected DESCRIPT sequence names stored as ordinary owned values.

use commander_blood_formats::instruction::{
    ScriptSequenceSlot, ScriptSequenceSlotAssignment, ScriptSequenceSlotName,
};

/// Six presentation sequence names retained across BloodScript execution.
///
/// The original implementation used adjacent fixed-width fields. The modern
/// port preserves their stable order and replacement behavior without exposing
/// byte offsets or allowing one assignment to overwrite another slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSequenceSlots {
    slots: [Option<ScriptSequenceSlotName>; ScriptSequenceSlot::COUNT],
}

impl ScriptSequenceSlots {
    /// Return the name currently assigned to one slot.
    pub fn name(&self, slot: ScriptSequenceSlot) -> Option<&ScriptSequenceSlotName> {
        self.slots[slot.index()].as_ref()
    }

    /// Apply `vm_op_cc_set_record_byte` as one bounded owned-name replacement.
    pub fn assign(&mut self, assignment: ScriptSequenceSlotAssignment) {
        let (slot, name) = assignment.into_parts();
        self.slots[slot.index()] = Some(name);
    }

    /// Clear all six names when a newly loaded script profile initializes its state.
    pub fn clear(&mut self) {
        self.slots.fill(None);
    }
}

impl Default for ScriptSequenceSlots {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
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
}
