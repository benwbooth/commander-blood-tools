//! Object-backed sequel inventory choices, distinct from dictionary concepts.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::code::{ScriptCodeOffset, ScriptDialect};
use commander_blood_formats::instruction::ScriptRecordValue;
use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptState};

use super::{
    AboardObjectRoster, ScriptRecordFields, ScriptRuntime, TextInstructionState,
    TextPresentationState, remove_aboard_object,
};

/// Authored A6 instruction and the recipient encoded before its selector byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequelInventoryLine {
    /// Typed instruction identity, not the native pointer into its selector byte.
    pub instruction: ScriptCodeOffset,
    /// Object receiving the selected inventory item.
    pub recipient: ScriptObjectId,
}

/// Inventory selection state formerly shared through the sequel's raw word list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SequelInventoryState {
    saved_line: Option<SequelInventoryLine>,
    choices: Vec<ScriptObjectId>,
    selected: Option<ScriptObjectId>,
    descriptor_lookup: Option<ScriptObjectId>,
}

/// Typed input that cannot represent a valid native inventory operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelInventoryError {
    /// The object-backed branch is not part of Commander Blood's text dialect.
    WrongDialect,
    /// An object identity does not belong to the active VAR image.
    MissingObject(ScriptObjectId),
    /// Only offered object choices may be selected by the UI.
    NotOffered(ScriptObjectId),
    /// No selector-resume phase is active for the selected object.
    ResumeInactive,
    /// The offered instruction no longer exists in this profile.
    MissingInstruction(ScriptCodeOffset),
    /// A selected record is not an inventory item.
    NotInventory(ScriptObjectId),
    /// A previous transfer still needs its native descriptor continuation.
    DescriptorPending,
    /// The saved recipient cannot be encoded in the original VAR word.
    UnencodableRecipient(ScriptObjectId),
}

impl fmt::Display for SequelInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SequelInventoryError {}

impl SequelInventoryState {
    /// Return offered objects in native roster order, including duplicates.
    pub fn choices(&self) -> &[ScriptObjectId] {
        &self.choices
    }

    /// Return the retained A6 owner; empty condition results do not clear it.
    pub const fn saved_line(&self) -> Option<SequelInventoryLine> {
        self.saved_line
    }

    /// Return the pending object selection, without inventing a DIC identity.
    pub const fn selected(&self) -> Option<ScriptObjectId> {
        self.selected
    }

    /// Return the unresolved native descriptor continuation after a transfer.
    pub const fn descriptor_lookup(&self) -> Option<ScriptObjectId> {
        self.descriptor_lookup
    }

    /// Publish the authored 0x8030 inventory condition after earlier A6 gates.
    ///
    /// This covers 0x6BF0..0x6C44 and helper 0x6C45 in BLOOD2PG. The caller
    /// has already armed resume; this operation does not publish subtitle text.
    pub fn offer(
        &mut self,
        line: SequelInventoryLine,
        roster: &AboardObjectRoster,
        state: &ScriptState,
        runtime: &mut ScriptRuntime,
        presentation: &mut TextPresentationState,
    ) -> Result<bool, SequelInventoryError> {
        if state.dialect() != ScriptDialect::BigBugBang {
            return Err(SequelInventoryError::WrongDialect);
        }
        if self.descriptor_lookup.is_some() {
            return Err(SequelInventoryError::DescriptorPending);
        }
        let mut choices = Vec::new();
        for id in roster.slots().iter().flatten() {
            let object = state
                .object(*id)
                .ok_or(SequelInventoryError::MissingObject(*id))?;
            // Serialized record zero is the roster's empty sentinel.
            if object.source_offset() != 0 && object.kind == ScriptObjectKind::InventoryItem {
                choices.push(*id);
            }
        }
        self.choices = choices;
        presentation.subtitle_word_list_mode = true;
        presentation.condition_presentation_words = Box::new([]);
        if self.choices.is_empty() {
            presentation.yield_signal = 0;
            runtime.set_resume_target(None);
            return Ok(false);
        }
        presentation.yield_signal = 1;
        self.saved_line = Some(line);
        Ok(true)
    }

    /// Select an offered object; dictionary concepts remain a separate domain.
    pub fn select(&mut self, object: ScriptObjectId) -> Result<(), SequelInventoryError> {
        if !self.choices.contains(&object) {
            return Err(SequelInventoryError::NotOffered(object));
        }
        self.selected = Some(object);
        Ok(())
    }

    /// Apply the inventory branch at 0x5C68 through its audio eligibility gates.
    ///
    /// Returns the transferred object, or `None` with no pending selection.
    /// A non-gated transfer retains `descriptor_lookup()` for the native 0x8450
    /// continuation; no descriptor result or presentation success is fabricated.
    #[allow(clippy::too_many_arguments)]
    pub fn commit(
        &mut self,
        state: &mut ScriptState,
        roster: &mut AboardObjectRoster,
        fields: &mut ScriptRecordFields,
        runtime: &mut ScriptRuntime,
        instructions: &mut BTreeMap<ScriptCodeOffset, TextInstructionState>,
        presentation: &TextPresentationState,
        ship_interface_active: bool,
    ) -> Result<Option<ScriptObjectId>, SequelInventoryError> {
        if state.dialect() != ScriptDialect::BigBugBang {
            return Err(SequelInventoryError::WrongDialect);
        }
        if self.descriptor_lookup.is_some() {
            return Err(SequelInventoryError::DescriptorPending);
        }
        let Some(selected) = self.selected else {
            return Ok(None);
        };
        if !runtime.selector_resume_active() {
            return Err(SequelInventoryError::ResumeInactive);
        }
        let line = self
            .saved_line
            .ok_or(SequelInventoryError::NotOffered(selected))?;
        let object = state
            .object(selected)
            .ok_or(SequelInventoryError::MissingObject(selected))?;
        if object.kind != ScriptObjectKind::InventoryItem {
            return Err(SequelInventoryError::NotInventory(selected));
        }
        let recipient = state
            .object(line.recipient)
            .ok_or(SequelInventoryError::MissingObject(line.recipient))?;
        let encoded_recipient = u16::try_from(recipient.source_offset())
            .map_err(|_| SequelInventoryError::UnencodableRecipient(line.recipient))?;
        let instruction = instructions
            .get_mut(&line.instruction)
            .ok_or(SequelInventoryError::MissingInstruction(line.instruction))?;
        // Inventory records have fixed native flags and selector-17 word fields.
        let flags = state
            .object_word(selected, 1)
            .expect("decoded inventory flags");
        let holder = state
            .object_word(selected, 10)
            .expect("decoded inventory holder");
        let flags_value = state.word(flags).expect("resolved inventory flags");
        instruction.activate();
        remove_aboard_object(roster, selected);
        assert!(state.set_word(holder, encoded_recipient));
        fields.set_value(holder, ScriptRecordValue::Object(line.recipient));
        assert!(state.set_word(flags, flags_value | 0x40));
        runtime.clear_alternate_resume_state();
        runtime.take_selected_concept();
        self.saved_line = None;
        self.choices.clear();
        if !ship_interface_active && !presentation.request_flags.secondary_request_pending() {
            self.descriptor_lookup = Some(selected);
        } else {
            self.selected = None;
        }
        Ok(Some(selected))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commander_blood_formats::code::ScriptDialect;
    use commander_blood_formats::instruction::{
        ScriptLineRecordOffset, ScriptText, ScriptTextControl,
    };
    use commander_blood_formats::script::{
        ScriptStateObjectReference, decode_script_dictionary, decode_script_directory,
        decode_script_state_for_dialect,
    };
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Condition {
        name: String,
        slots: Vec<u16>,
        kinds: Vec<(u16, u16)>,
        choices: Vec<u16>,
        accepted: bool,
        resume: u8,
        saved_line: u16,
        spoken: u8,
        #[serde(rename = "yield")]
        yield_signal: u8,
    }

    #[derive(Deserialize)]
    struct Selection {
        condition: String,
        choice: u16,
        audio_gate: String,
        target: u16,
        holder: u16,
        holder_offset: usize,
        object_flags: u8,
        line_flags: u8,
        slots_after: Vec<u16>,
        selected: u16,
        alternate: u16,
        saved_line: u16,
        resume: u8,
        #[serde(rename = "yield")]
        yield_signal: u8,
        spoken: u8,
        pending_head: u16,
    }

    fn conditions() -> Vec<Condition> {
        include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_inventory_condition.jsonl"
        )
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
    }

    fn fixture(
        condition: &Condition,
    ) -> (
        ScriptState,
        BTreeMap<u16, ScriptObjectId>,
        AboardObjectRoster,
    ) {
        let mut records = BTreeMap::from([(0, 2), (0x780, 2)]);
        records.extend(condition.kinds.iter().copied());
        let mut directory_bytes = Vec::new();
        let mut bytes = Vec::new();
        for (index, kind) in records.values().enumerate() {
            let kind = ScriptObjectKind::decode(*kind).unwrap();
            let mut entry = [0; 20];
            let name = format!("O{index}");
            entry[..name.len()].copy_from_slice(name.as_bytes());
            entry[16..18].copy_from_slice(&(bytes.len() as u16).to_le_bytes());
            entry[18..20].copy_from_slice(&1u16.to_le_bytes());
            directory_bytes.extend(entry);
            let mut record = vec![0; kind.record_size_for_dialect(ScriptDialect::BigBugBang)];
            record[..2].copy_from_slice(&kind.mask().to_le_bytes());
            record[2] = 0x12;
            if kind == ScriptObjectKind::InventoryItem {
                record[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
            }
            bytes.extend(record);
        }
        directory_bytes.extend([0; 20]);
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let state =
            decode_script_state_for_dialect(&bytes, &directory, ScriptDialect::BigBugBang).unwrap();
        let ids: BTreeMap<_, _> = records
            .keys()
            .copied()
            .zip(directory.active_objects().map(|(id, _)| id))
            .collect();
        let slots = condition
            .slots
            .iter()
            .map(|offset| (*offset != 0).then(|| ids[offset]))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        (state, ids, AboardObjectRoster::from_test_slots(slots))
    }

    fn runtime() -> ScriptRuntime {
        let mut runtime = ScriptRuntime::default();
        runtime.arm_resume(ScriptCodeOffset::new(100), 0);
        assert!(runtime.activate_selector_resume());
        runtime
    }

    fn instruction() -> TextInstructionState {
        TextInstructionState::new(&ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 0,
            control: ScriptTextControl::decode(0x2130),
            resume_target: Some(ScriptCodeOffset::new(100)),
            record_condition_operand: None,
            words: Box::new([]),
        })
    }

    #[test]
    fn sequel_inventory_conditions_match_all_typed_native_vectors() {
        let vectors = conditions();
        assert_eq!(vectors.len(), 22);
        let mut checked = 0;
        for vector in vectors {
            // These native robustness vectors contain kinds rejected by VAR decoding.
            if ["kind_mask", "no_inventory"].contains(&vector.name.as_str()) {
                continue;
            }
            let (state, ids, roster) = fixture(&vector);
            let mut runtime = runtime();
            let mut presentation = TextPresentationState::default();
            let old_line = SequelInventoryLine {
                instruction: ScriptCodeOffset::new(0x1234),
                recipient: ids[&0x780],
            };
            let line = SequelInventoryLine {
                instruction: ScriptCodeOffset::new(0x3456),
                ..old_line
            };
            let mut inventory = SequelInventoryState {
                saved_line: Some(old_line),
                ..Default::default()
            };
            assert_eq!(
                inventory
                    .offer(line, &roster, &state, &mut runtime, &mut presentation)
                    .unwrap(),
                vector.accepted,
                "{}",
                vector.name
            );
            assert_eq!(
                inventory.choices(),
                vector
                    .choices
                    .iter()
                    .map(|offset| ids[&(offset - 4)])
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                inventory.saved_line().unwrap().instruction.index(),
                usize::from(vector.saved_line)
            );
            assert_eq!(runtime.selector_resume_active(), vector.resume == 2);
            assert_eq!(presentation.yield_signal, vector.yield_signal);
            assert_eq!(presentation.subtitle_word_list_mode, vector.spoken != 0);
            checked += 1;
        }
        assert_eq!(checked, 20);
    }

    #[test]
    fn sequel_inventory_transfers_match_all_typed_native_gated_vectors() {
        let conditions = conditions();
        let selections: Vec<Selection> = include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_inventory_selection.jsonl"
        )
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
        assert_eq!(selections.len(), 82);
        let mut checked = 0;
        for vector in selections {
            // The remaining two captures have invalid combined-kind neighbors,
            // so the complete input VAR cannot be represented by the decoder.
            if vector.condition == "kind_mask" {
                continue;
            }
            let condition = conditions
                .iter()
                .find(|row| row.name == vector.condition)
                .unwrap();
            let (mut state, ids, mut roster) = fixture(condition);
            let mut runtime = runtime();
            let mut presentation = TextPresentationState::default();
            let line = SequelInventoryLine {
                instruction: ScriptCodeOffset::new(0x3456),
                recipient: ids[&vector.target],
            };
            let mut inventory = SequelInventoryState::default();
            assert!(
                inventory
                    .offer(line, &roster, &state, &mut runtime, &mut presentation)
                    .unwrap()
            );
            let selected = ids[&(vector.choice - 4)];
            inventory.select(selected).unwrap();
            let dictionary = decode_script_dictionary(b"BEFORE\0").unwrap();
            runtime.set_alternate_concept(Some(dictionary.words().next().unwrap().0));
            let mut instructions = BTreeMap::from([(line.instruction, instruction())]);
            let mut fields = ScriptRecordFields::default();
            let mut expected_state = state.clone();
            let flags = expected_state.object_word(selected, 1).unwrap();
            let holder = expected_state
                .object_word(selected, vector.holder_offset / 2)
                .unwrap();
            expected_state.set_word(flags, u16::from(vector.object_flags));
            expected_state.set_word(
                holder,
                expected_state
                    .object(ids[&vector.holder])
                    .unwrap()
                    .source_offset() as u16,
            );
            presentation.request_flags = super::super::PresentationRequestFlags::decode(
                if vector.audio_gate == "dialogue" {
                    2
                } else {
                    0
                },
            );
            assert_eq!(
                inventory
                    .commit(
                        &mut state,
                        &mut roster,
                        &mut fields,
                        &mut runtime,
                        &mut instructions,
                        &presentation,
                        vector.audio_gate == "global"
                    )
                    .unwrap(),
                Some(selected)
            );
            assert_eq!(state, expected_state);
            assert_eq!(
                fields.value(holder),
                Some(ScriptRecordValue::Object(ids[&vector.holder]))
            );
            assert_eq!(
                state.object_reference(holder),
                Some(ScriptStateObjectReference::Object(ids[&vector.holder]))
            );
            assert_eq!(
                roster.slots().as_slice(),
                vector
                    .slots_after
                    .iter()
                    .map(|offset| (*offset != 0).then(|| ids[offset]))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                instructions[&line.instruction].is_active(),
                vector.line_flags & 0x80 != 0
            );
            assert_eq!(runtime.resume_state().is_none(), vector.resume == 0);
            assert_eq!(runtime.selected_concept().is_none(), vector.selected == 0);
            assert_eq!(runtime.alternate_concept().is_none(), vector.alternate == 0);
            assert_eq!(inventory.saved_line().is_none(), vector.saved_line == 0);
            assert_eq!(inventory.choices().is_empty(), vector.pending_head == 0);
            assert_eq!(inventory.selected(), None);
            assert_eq!(inventory.descriptor_lookup(), None);
            assert_eq!(presentation.yield_signal, vector.yield_signal);
            assert_eq!(presentation.subtitle_word_list_mode, vector.spoken != 0);
            checked += 1;
        }
        assert_eq!(checked, 80);
    }

    #[test]
    fn sequel_inventory_errors_preserve_state_and_ungated_transfer_retains_continuation() {
        let vectors = conditions();
        let vector = vectors
            .iter()
            .find(|row| row.name == "single_slot_0")
            .unwrap();
        let (mut state, ids, mut roster) = fixture(vector);
        let mut runtime = runtime();
        let mut presentation = TextPresentationState::default();
        let line = SequelInventoryLine {
            instruction: ScriptCodeOffset::new(10),
            recipient: ids[&0x780],
        };
        let mut inventory = SequelInventoryState::default();
        let empty_directory = decode_script_directory(&[]).unwrap();
        let commander =
            decode_script_state_for_dialect(&[], &empty_directory, ScriptDialect::CommanderBlood)
                .unwrap();
        let before = (inventory.clone(), runtime.clone(), presentation.clone());
        assert_eq!(
            inventory.offer(line, &roster, &commander, &mut runtime, &mut presentation),
            Err(SequelInventoryError::WrongDialect)
        );
        assert_eq!(
            (inventory.clone(), runtime.clone(), presentation.clone()),
            before
        );
        assert!(
            inventory
                .offer(line, &roster, &state, &mut runtime, &mut presentation)
                .unwrap()
        );
        let before_selection = inventory.clone();
        assert_eq!(
            inventory.select(line.recipient),
            Err(SequelInventoryError::NotOffered(line.recipient))
        );
        assert_eq!(inventory, before_selection);
        inventory.select(ids[&0x200]).unwrap();
        let before = (
            inventory.clone(),
            state.clone(),
            roster.clone(),
            runtime.clone(),
        );
        let mut instructions = BTreeMap::new();
        let mut fields = ScriptRecordFields::default();
        assert_eq!(
            inventory.commit(
                &mut state,
                &mut roster,
                &mut fields,
                &mut runtime,
                &mut instructions,
                &presentation,
                true
            ),
            Err(SequelInventoryError::MissingInstruction(line.instruction))
        );
        assert_eq!(
            (
                inventory.clone(),
                state.clone(),
                roster.clone(),
                runtime.clone()
            ),
            before
        );
        instructions.insert(line.instruction, instruction());
        assert_eq!(fields, ScriptRecordFields::default());
        runtime.set_resume_target(None);
        let before = (
            inventory.clone(),
            state.clone(),
            roster.clone(),
            runtime.clone(),
            instructions.clone(),
        );
        assert_eq!(
            inventory.commit(
                &mut state,
                &mut roster,
                &mut fields,
                &mut runtime,
                &mut instructions,
                &presentation,
                true
            ),
            Err(SequelInventoryError::ResumeInactive)
        );
        assert_eq!(
            (
                inventory.clone(),
                state.clone(),
                roster.clone(),
                runtime.clone(),
                instructions.clone()
            ),
            before
        );
        runtime.arm_resume(ScriptCodeOffset::new(100), 0);
        assert_eq!(fields, ScriptRecordFields::default());
        assert!(runtime.activate_selector_resume());
        assert_eq!(
            inventory
                .commit(
                    &mut state,
                    &mut roster,
                    &mut fields,
                    &mut runtime,
                    &mut instructions,
                    &presentation,
                    false
                )
                .unwrap(),
            Some(ids[&0x200])
        );
        assert_eq!(inventory.descriptor_lookup(), Some(ids[&0x200]));
        assert_eq!(inventory.selected(), Some(ids[&0x200]));
        let before = (
            inventory.clone(),
            state.clone(),
            roster.clone(),
            runtime.clone(),
            instructions.clone(),
        );
        assert_eq!(
            inventory.commit(
                &mut state,
                &mut roster,
                &mut fields,
                &mut runtime,
                &mut instructions,
                &presentation,
                false
            ),
            Err(SequelInventoryError::DescriptorPending)
        );
        assert_eq!((inventory, state, roster, runtime, instructions), before);
    }
}
