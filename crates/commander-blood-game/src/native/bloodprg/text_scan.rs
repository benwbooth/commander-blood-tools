//! Typed activation of object-owned BloodScript text instructions.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptState};

use super::TextInstructionState;

/// One top-level text instruction bound to its owning object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundTextInstruction {
    owner: ScriptObjectId,
    state: TextInstructionState,
}

impl BoundTextInstruction {
    /// Return the object resolved from this instruction's line record.
    pub const fn owner(&self) -> ScriptObjectId {
        self.owner
    }

    /// Return the mutable authored activation state.
    pub const fn state(&self) -> TextInstructionState {
        self.state
    }
}

/// Mutable A6 activation state partitioned by decoded script ownership.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptTextActivationRegistry {
    top_level: Vec<BoundTextInstruction>,
    object_blocks: BTreeMap<ScriptObjectId, Vec<TextInstructionState>>,
}

impl ScriptTextActivationRegistry {
    /// Register one top-level A6 instruction after resolving its line owner.
    pub fn push_top_level(&mut self, owner: ScriptObjectId, state: TextInstructionState) {
        self.top_level.push(BoundTextInstruction { owner, state });
    }

    /// Register one A6 instruction from an object's decoded code block.
    pub fn push_object_block(&mut self, object: ScriptObjectId, state: TextInstructionState) {
        self.object_blocks.entry(object).or_default().push(state);
    }

    /// Return top-level text instructions in authored order.
    pub fn top_level(&self) -> &[BoundTextInstruction] {
        &self.top_level
    }

    /// Return one object's block-local text states in authored order.
    pub fn object_block(&self, object: ScriptObjectId) -> &[TextInstructionState] {
        self.object_blocks
            .get(&object)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// Invalid typed state supplied to the object text-activation pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTextActivationError {
    /// The requested object does not exist in this decoded profile.
    MissingObject {
        /// Missing object identity.
        object: ScriptObjectId,
    },
}

impl fmt::Display for ScriptTextActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptTextActivationError {}

/// Activate text bound directly or by code block to one object.
///
/// This translates `vm_cod_scan` at BLOODPRG file offset `0x00739B`.
/// Pre-decoded ownership and block collections replace mutable token scans,
/// parser modes, byte markers, and block terminators.
pub fn activate_object_text(
    profile: &ScriptState,
    object: ScriptObjectId,
    registry: &mut ScriptTextActivationRegistry,
) -> Result<ScriptObjectKind, ScriptTextActivationError> {
    let kind = profile
        .object(object)
        .ok_or(ScriptTextActivationError::MissingObject { object })?
        .kind;

    for text in &mut registry.top_level {
        if text.owner == object {
            text.state.activate();
        }
    }
    if let Some(block) = registry.object_blocks.get_mut(&object) {
        for state in block {
            state.activate();
        }
    }

    Ok(kind)
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::instruction::{
        ScriptLineRecordOffset, ScriptText, ScriptTextControl,
    };
    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 4;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;

    #[derive(Deserialize)]
    struct TextScanOracle {
        name: String,
        script: TextScanScript,
        code: TextScanCode,
        block_scan_flags_after: u8,
    }

    #[derive(Deserialize)]
    struct TextScanScript {
        matching_a6_offsets: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct TextScanCode {
        marked_a6_offsets: Vec<u16>,
    }

    #[test]
    fn activation_matches_every_original_scan_result() {
        let vectors: Vec<TextScanOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_739b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let target_kind = match vector.name.as_str() {
                "zero_code_offset_and_plain_tokens" => ScriptObjectKind::Player,
                "matching_script_and_aa_terminated_code" => ScriptObjectKind::Location,
                "ff_terminated_code_and_wrapped_offsets" => ScriptObjectKind::BlackHole,
                "mode_control_and_variable_token_restore_word" => ScriptObjectKind::Actor,
                name => panic!("unknown text-scan oracle {name}"),
            };
            let profile = state_fixture(&[target_kind, ScriptObjectKind::Actor]);
            let target = profile.objects()[0].id;
            let other = profile.objects()[1].id;
            let mut registry = ScriptTextActivationRegistry::default();
            for _ in &vector.script.matching_a6_offsets {
                registry.push_top_level(target, inactive_text_state());
            }
            registry.push_top_level(other, inactive_text_state());
            for _ in &vector.code.marked_a6_offsets {
                registry.push_object_block(target, inactive_text_state());
            }
            registry.push_object_block(other, inactive_text_state());

            assert_eq!(
                activate_object_text(&profile, target, &mut registry).unwrap(),
                target_kind,
                "{}",
                vector.name
            );
            assert!(
                registry.top_level()[..vector.script.matching_a6_offsets.len()]
                    .iter()
                    .all(|text| text.state().is_active()),
                "{}",
                vector.name
            );
            assert!(
                !registry.top_level().last().unwrap().state().is_active(),
                "{}",
                vector.name
            );
            assert!(
                registry
                    .object_block(target)
                    .iter()
                    .all(|state| state.is_active()),
                "{}",
                vector.name
            );
            assert!(
                registry
                    .object_block(other)
                    .iter()
                    .all(|state| !state.is_active()),
                "{}",
                vector.name
            );
            assert_eq!(vector.block_scan_flags_after, 1, "{}", vector.name);
        }
    }

    #[test]
    fn existing_authored_activation_is_preserved() {
        let profile = state_fixture(&[ScriptObjectKind::Actor]);
        let target = profile.objects()[0].id;
        let mut registry = ScriptTextActivationRegistry::default();
        registry.push_top_level(target, text_state(true));

        activate_object_text(&profile, target, &mut registry).unwrap();

        assert!(registry.top_level()[0].state().is_active());
    }

    fn inactive_text_state() -> TextInstructionState {
        text_state(false)
    }

    fn text_state(active: bool) -> TextInstructionState {
        let text = ScriptText {
            line_record: ScriptLineRecordOffset::decode(u16::MIN),
            presentation_selector: i8::MIN,
            control: ScriptTextControl::decode(if active { 32_768 } else { u16::MIN }),
            resume_target: Some(ScriptCodeOffset::new(usize::MIN)),
            record_condition_operand: None,
            words: Box::new([]),
        };
        TextInstructionState::new(&text)
    }

    fn state_fixture(kinds: &[ScriptObjectKind]) -> ScriptState {
        let mut directory_data = Vec::new();
        let mut state_data = Vec::new();
        for (index, kind) in kinds.iter().copied().enumerate() {
            let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            entry[0] = b'a' + u8::try_from(index).unwrap();
            entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + 2]
                .copy_from_slice(&u16::try_from(state_data.len()).unwrap().to_le_bytes());
            entry[DIRECTORY_NAME_CAPACITY + 2..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..2].copy_from_slice(&kind.mask().to_le_bytes());
            state_data.extend_from_slice(&object);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        decode_script_state(&state_data, &directory).unwrap()
    }
}
