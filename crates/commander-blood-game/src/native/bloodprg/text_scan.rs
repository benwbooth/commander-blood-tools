//! Typed activation of object-owned BloodScript text instructions.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction};
use commander_blood_formats::code::{ScriptCode, ScriptCodeOffset};
use commander_blood_formats::instruction::DecodedScriptInstruction;
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState,
};

use super::{
    ScriptBasDispatchState, ScriptFieldSelector, TextInstructionState, script_field_offset,
};

const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();

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
    /// The object has no matching active DEB directory entry.
    MissingDirectoryObject {
        /// Missing object identity.
        object: ScriptObjectId,
    },
    /// COD tokens and their typed instruction stream were not parallel.
    MismatchedInstructionCount {
        /// Number of losslessly framed COD tokens.
        tokens: usize,
        /// Number of typed COD instructions.
        instructions: usize,
    },
    /// The object's recovered kind has no selector-2 BAS block field.
    MissingObjectBlockField {
        /// Object lacking the field.
        object: ScriptObjectId,
    },
    /// A nonzero selector-2 value did not resolve to a BAS token boundary.
    InvalidObjectBlockOffset {
        /// Object owning the invalid offset.
        object: ScriptObjectId,
        /// Authored BAS position.
        source_offset: ScriptCodeOffset,
    },
    /// A nonzero object block reached the end of BAS without AA or FF.
    UnterminatedObjectBlock {
        /// Object owning the block.
        object: ScriptObjectId,
        /// Authored BAS start position.
        source_offset: ScriptCodeOffset,
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

/// Observable activation counts from one complete native `vm_cod_scan` pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptObjectTextActivationOutcome {
    /// Decoded kind returned by the native routine.
    pub kind: ScriptObjectKind,
    /// Matching top-level COD A6 instructions activated.
    pub top_level_text_count: usize,
    /// A6 instructions activated in the object's selector-2 BAS block.
    pub object_block_text_count: usize,
}

/// Activate every top-level and object-block A6 instruction owned by an object.
///
/// This is the production flat-state translation of `vm_cod_scan` at BLOODPRG
/// file offset `0x00739B`. COD line ownership comes from the exact VAR byte
/// offset stored in the object's DEB entry. The BAS block starts at field
/// selector 2 and follows decoded physical token order through AA or FF.
#[allow(clippy::too_many_arguments)]
pub fn activate_profile_object_text(
    code: &ScriptCode,
    instructions: &[DecodedScriptInstruction],
    dialogue: &ScriptBas,
    state: &ScriptState,
    directory: &ScriptDirectory,
    object: ScriptObjectId,
    cod_text_states: &mut BTreeMap<ScriptCodeOffset, TextInstructionState>,
    bas: &mut ScriptBasDispatchState,
) -> Result<ScriptObjectTextActivationOutcome, ScriptTextActivationError> {
    let kind = state
        .object(object)
        .ok_or(ScriptTextActivationError::MissingObject { object })?
        .kind;
    let object_offset = directory
        .object(object)
        .ok_or(ScriptTextActivationError::MissingDirectoryObject { object })?
        .value;
    if code.tokens().len() != instructions.len() {
        return Err(ScriptTextActivationError::MismatchedInstructionCount {
            tokens: code.tokens().len(),
            instructions: instructions.len(),
        });
    }

    let mut top_level_text_count = usize::MIN;
    for (token, instruction) in code.tokens().iter().zip(instructions) {
        let DecodedScriptInstruction::Text(text) = instruction else {
            continue;
        };
        if text.line_record.byte_offset() != usize::from(object_offset) {
            continue;
        }
        cod_text_states
            .entry(token.source_offset())
            .or_insert_with(|| TextInstructionState::new(text))
            .activate();
        top_level_text_count += 1;
    }

    let block_field_offset =
        script_field_offset(kind, ScriptFieldSelector::PRESENTATION_HANDOFF)
            .ok_or(ScriptTextActivationError::MissingObjectBlockField { object })?;
    let block_field = state
        .object_word(object, block_field_offset / SERIALIZED_WORD_SIZE)
        .ok_or(ScriptTextActivationError::MissingObjectBlockField { object })?;
    let encoded_block_offset = state
        .word(block_field)
        .ok_or(ScriptTextActivationError::MissingObjectBlockField { object })?;
    let mut object_block_text_count = usize::MIN;
    if encoded_block_offset != u16::MIN {
        let source_offset = ScriptCodeOffset::new(usize::from(encoded_block_offset));
        let start = dialogue
            .tokens()
            .binary_search_by_key(&source_offset, |token| token.source_offset())
            .map_err(|_| ScriptTextActivationError::InvalidObjectBlockOffset {
                object,
                source_offset,
            })?;
        let mut terminated = false;
        for token in &dialogue.tokens()[start..] {
            match token.instruction() {
                ScriptBasInstruction::Text(text) => {
                    bas.activate_text(token.source_offset(), text);
                    object_block_text_count += 1;
                }
                ScriptBasInstruction::Yield | ScriptBasInstruction::End => {
                    terminated = true;
                    break;
                }
                _ => {}
            }
        }
        if !terminated {
            return Err(ScriptTextActivationError::UnterminatedObjectBlock {
                object,
                source_offset,
            });
        }
    }

    Ok(ScriptObjectTextActivationOutcome {
        kind,
        top_level_text_count,
        object_block_text_count,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::instruction::{
        ScriptLineRecordOffset, ScriptText, ScriptTextControl,
    };
    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use crate::assets::OriginalResourceStore;

    use super::super::{
        OriginalResourceCache, OriginalResourceCatalog, OriginalScriptProfileCatalog,
        ScriptProfileId, ScriptProfileManager,
    };
    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 4;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const SHIPPED_ACTOR_COUNT: usize = 166;
    const SHIPPED_ACTOR_COD_TEXT_COUNT: usize = 3_687;
    const SHIPPED_ACTOR_BAS_TEXT_COUNT: usize = 1_849;

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

    #[test]
    fn every_shipped_actor_resolves_its_native_cod_and_bas_text_scan() {
        let Some(root) = original_data_root() else {
            return;
        };
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);
        let mut actor_count = usize::MIN;
        let mut top_level_text_count = usize::MIN;
        let mut object_block_text_count = usize::MIN;

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let profile = manager.current().unwrap();

            for actor in profile
                .state()
                .objects()
                .iter()
                .filter(|object| object.kind == ScriptObjectKind::Actor)
            {
                let mut cod_text_states = BTreeMap::new();
                let mut bas = ScriptBasDispatchState::default();
                let outcome = activate_profile_object_text(
                    profile.code(),
                    profile.instructions(),
                    profile.dialogue(),
                    profile.state(),
                    profile.directory(),
                    actor.id,
                    &mut cod_text_states,
                    &mut bas,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "profile {} actor {:?} failed: {error:?}",
                        profile_id.value() + 1,
                        actor.id
                    )
                });

                assert_eq!(outcome.kind, ScriptObjectKind::Actor);
                assert_eq!(cod_text_states.len(), outcome.top_level_text_count);
                assert!(cod_text_states.values().all(|state| state.is_active()));
                let active_bas_text_count = profile
                    .dialogue()
                    .tokens()
                    .iter()
                    .filter(|token| {
                        bas.text_state(token.source_offset())
                            .is_some_and(TextInstructionState::is_active)
                    })
                    .count();
                assert_eq!(active_bas_text_count, outcome.object_block_text_count);

                actor_count += 1;
                top_level_text_count += outcome.top_level_text_count;
                object_block_text_count += outcome.object_block_text_count;
            }
        }

        assert_eq!(actor_count, SHIPPED_ACTOR_COUNT);
        assert_eq!(top_level_text_count, SHIPPED_ACTOR_COD_TEXT_COUNT);
        assert_eq!(object_block_text_count, SHIPPED_ACTOR_BAS_TEXT_COUNT);
    }

    fn original_data_root() -> Option<PathBuf> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find(|root| root.join("SCRIPT1.COD").is_file())
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
