//! Dialogue selector control over decoded BAS branches and typed actor state.

use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::script::{
    ScriptDictionary, ScriptObjectId, ScriptState, ScriptWordId,
};

use super::{
    ScriptBlockError, ScriptBlockHandler, ScriptBlockOutcome, ScriptFieldSelector, ScriptRuntime,
    ScriptSelectionError, ScriptSelectorError, ScriptSelectorState, collect_selector_menu,
    execute_script_block, find_selector_body, script_field_offset,
};

const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();

/// Immutable profile inputs for one actor dialogue dispatch.
#[derive(Clone, Copy, Debug)]
pub struct ScriptControlFlowContext<'a> {
    /// Actor whose dialogue-control field selects the response.
    pub actor: ScriptObjectId,
    /// Active profile dictionary owning every selector concept.
    pub dictionary: &'a ScriptDictionary,
    /// Active profile's fully decoded BAS dialogue image.
    pub dialogue: &'a ScriptBas,
    /// First selector node in the actor's authored response list.
    pub selector_root: ScriptCodeOffset,
}

/// Result of selecting and executing current and parent dialogue bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptControlFlowOutcome {
    /// Interned concept chosen from branch state, the actor field, or list root.
    pub selected_control: ScriptWordId,
    /// Current response body selected from the linked BAS list.
    pub current_body: Option<ScriptCodeOffset>,
    /// Suspended parent response body selected independently.
    pub parent_body: Option<ScriptCodeOffset>,
    /// Execution result for the current response body.
    pub current_execution: Option<ScriptBlockOutcome>,
    /// Execution result for the parent response body.
    pub parent_execution: Option<ScriptBlockOutcome>,
    /// Whether the current response published a decoded menu.
    pub menu_collected: bool,
}

/// Invalid typed selector state or nested instruction failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptControlFlowError<HandlerError> {
    /// The requested actor object is absent from owned profile state.
    MissingObject {
        /// Missing stable object identity.
        object: ScriptObjectId,
    },
    /// The object's decoded kind has no dialogue-control field.
    MissingControlField {
        /// Object lacking the field.
        object: ScriptObjectId,
    },
    /// A nonzero actor field does not begin an interned dictionary word.
    UnknownControlWord {
        /// Unresolved serialized dictionary position.
        encoded: u16,
    },
    /// An interned selected concept has no serialized dictionary encoding.
    MissingControlEncoding {
        /// Concept from incompatible or malformed state.
        concept: ScriptWordId,
    },
    /// Linked selector traversal failed typed validation.
    Selector(ScriptSelectorError),
    /// Nested BAS execution failed.
    Block(ScriptBlockError<HandlerError>),
    /// Menu collection failed after current-body execution.
    Selection(ScriptSelectionError),
}

impl<HandlerError: fmt::Debug> fmt::Display for ScriptControlFlowError<HandlerError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HandlerError: fmt::Debug> std::error::Error for ScriptControlFlowError<HandlerError> {}

/// Select and execute dialogue response bodies for one actor.
///
/// This translates `vm_control_flow` at BLOODPRG file offset `0x0056FE`.
/// Stable object identities, interned words, and decoded BAS positions replace
/// record arithmetic, dictionary offsets used as values, and code pointers.
pub fn execute_selector_control<Handler: ScriptBlockHandler>(
    state: &mut ScriptState,
    context: ScriptControlFlowContext<'_>,
    runtime: &mut ScriptRuntime,
    selector_state: &mut ScriptSelectorState,
    offered_topic: &mut Option<ScriptWordId>,
    handler: &mut Handler,
) -> Result<ScriptControlFlowOutcome, ScriptControlFlowError<Handler::Error>> {
    let actor_kind = state
        .object(context.actor)
        .ok_or(ScriptControlFlowError::MissingObject {
            object: context.actor,
        })?
        .kind;
    let field_offset = script_field_offset(actor_kind, ScriptFieldSelector::DIALOGUE_CONTROL)
        .ok_or(ScriptControlFlowError::MissingControlField {
            object: context.actor,
        })?;
    let control_field = state
        .object_word(context.actor, field_offset / SERIALIZED_WORD_SIZE)
        .ok_or(ScriptControlFlowError::MissingControlField {
            object: context.actor,
        })?;
    let encoded_field =
        state
            .word(control_field)
            .ok_or(ScriptControlFlowError::MissingControlField {
                object: context.actor,
            })?;
    let field_control = if encoded_field == u16::MIN {
        None
    } else {
        Some(
            context
                .dictionary
                .resolve_source_offset(encoded_field)
                .ok_or(ScriptControlFlowError::UnknownControlWord {
                    encoded: encoded_field,
                })?,
        )
    };
    let root_control = selector_at(context.dialogue, context.selector_root)?;
    let selected_control = selector_state
        .current_control()
        .or(selector_state.current_branch().map(|branch| branch.concept))
        .or(field_control)
        .unwrap_or(root_control);
    let parent_control = selector_state
        .parent_control()
        .or(selector_state.parent_branch().map(|branch| branch.concept));
    let current_body =
        find_selector_body(context.dialogue, context.selector_root, selected_control)?;
    let parent_body = parent_control
        .map(|control| find_selector_body(context.dialogue, context.selector_root, control))
        .transpose()?
        .flatten();
    let encoded_control = context.dictionary.source_offset(selected_control).ok_or(
        ScriptControlFlowError::MissingControlEncoding {
            concept: selected_control,
        },
    )?;

    let assigned = state.set_word(control_field, encoded_control);
    debug_assert!(assigned, "validated actor control field remains writable");
    selector_state.select_control_branch(selected_control, current_body);

    let current_execution = current_body
        .map(|body| execute_script_block(context.dialogue, body, runtime, handler))
        .transpose()
        .map_err(ScriptControlFlowError::Block)?;
    let menu_collected = if current_body.is_some() {
        collect_selector_menu(context.dialogue, selector_state, offered_topic)
            .map_err(ScriptControlFlowError::Selection)?
    } else {
        false
    };
    let parent_execution = parent_body
        .map(|body| execute_script_block(context.dialogue, body, runtime, handler))
        .transpose()
        .map_err(ScriptControlFlowError::Block)?;

    Ok(ScriptControlFlowOutcome {
        selected_control,
        current_body,
        parent_body,
        current_execution,
        parent_execution,
        menu_collected,
    })
}

fn selector_at(
    dialogue: &ScriptBas,
    source_offset: ScriptCodeOffset,
) -> Result<ScriptWordId, ScriptSelectorError> {
    let token = dialogue
        .tokens()
        .iter()
        .find(|token| token.source_offset() == source_offset)
        .ok_or(ScriptSelectorError::MissingNode { source_offset })?;
    let ScriptBasInstruction::SelectorNode { selector, .. } = token.instruction() else {
        return Err(ScriptSelectorError::MissingNode { source_offset });
    };
    Ok(*selector)
}

impl<HandlerError> From<ScriptSelectorError> for ScriptControlFlowError<HandlerError> {
    fn from(source: ScriptSelectorError) -> Self {
        Self::Selector(source)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bas::decode_script_bas;
    use commander_blood_formats::script::{
        ScriptObjectKind, decode_script_dictionary, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::ScriptBlockStep;

    const ORACLE_VECTOR_COUNT: usize = 14;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_ACTIVE_KIND: u16 = 1;
    const MENU_OPCODE: u8 = 0xA3;
    const SELECTOR_YIELD_OPCODE: u8 = 0xAC;
    const END_OPCODE: u8 = 0xFF;
    const FIRST_CONTROL_ENCODING: u16 = 0x1111;
    const SECOND_CONTROL_ENCODING: u16 = 0x2222;
    const UNMATCHED_CONTROL_ENCODING: u16 = 0x3333;
    const MENU_WORD_ENCODING: u16 = FIRST_CONTROL_ENCODING;

    #[derive(Deserialize)]
    struct ControlFlowOracle {
        name: String,
        field_before: u16,
        branch_a_before: u16,
        selected_control: u16,
        first_match: u16,
        branch_b: u16,
        parent_match: u16,
        block_calls: Vec<u16>,
        collector_called: bool,
    }

    #[derive(Deserialize)]
    struct SelectorGraph {
        lists: Vec<SelectorList>,
    }

    #[derive(Deserialize)]
    struct SelectorList {
        node_offsets: Vec<usize>,
    }

    #[derive(Default)]
    struct RecordingHandler {
        calls: Vec<ScriptCodeOffset>,
    }

    impl ScriptBlockHandler for RecordingHandler {
        type Error = Infallible;

        fn execute_instruction(
            &mut self,
            token: &commander_blood_formats::bas::ScriptBasToken,
            _runtime: &mut ScriptRuntime,
        ) -> Result<ScriptBlockStep, Self::Error> {
            self.calls.push(token.source_offset());
            Ok(ScriptBlockStep::continue_at(token.end_offset()))
        }
    }

    struct Fixture {
        dictionary: ScriptDictionary,
        dialogue: ScriptBas,
        state: ScriptState,
        actor: ScriptObjectId,
        root: ScriptCodeOffset,
        first_body: ScriptCodeOffset,
        second_body: ScriptCodeOffset,
        first_control: ScriptWordId,
        second_control: ScriptWordId,
        unmatched_control: ScriptWordId,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn fixture(field: u16) -> Fixture {
        let mut dictionary_bytes = vec![u8::MIN; usize::from(UNMATCHED_CONTROL_ENCODING) + 8];
        dictionary_bytes[..3].copy_from_slice(b"ok\0");
        dictionary_bytes
            [usize::from(FIRST_CONTROL_ENCODING)..usize::from(FIRST_CONTROL_ENCODING) + 6]
            .copy_from_slice(b"first\0");
        dictionary_bytes
            [usize::from(SECOND_CONTROL_ENCODING)..usize::from(SECOND_CONTROL_ENCODING) + 7]
            .copy_from_slice(b"second\0");
        dictionary_bytes
            [usize::from(UNMATCHED_CONTROL_ENCODING)..usize::from(UNMATCHED_CONTROL_ENCODING) + 8]
            .copy_from_slice(b"missing\0");
        let dictionary = decode_script_dictionary(&dictionary_bytes).unwrap();
        let first_control = dictionary
            .resolve_source_offset(FIRST_CONTROL_ENCODING)
            .unwrap();
        let second_control = dictionary
            .resolve_source_offset(SECOND_CONTROL_ENCODING)
            .unwrap();
        let unmatched_control = dictionary
            .resolve_source_offset(UNMATCHED_CONTROL_ENCODING)
            .unwrap();

        let mut bas_bytes = vec![SELECTOR_YIELD_OPCODE];
        let root = ScriptCodeOffset::new(bas_bytes.len());
        bas_bytes.extend_from_slice(&FIRST_CONTROL_ENCODING.to_le_bytes());
        let first_next = bas_bytes.len();
        bas_bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let first_body = ScriptCodeOffset::new(bas_bytes.len());
        bas_bytes.push(MENU_OPCODE);
        bas_bytes.extend_from_slice(&MENU_WORD_ENCODING.to_le_bytes());
        bas_bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        bas_bytes.push(END_OPCODE);
        bas_bytes.push(SELECTOR_YIELD_OPCODE);
        let second_node = u16::try_from(bas_bytes.len()).unwrap();
        bas_bytes[first_next..first_next + SERIALIZED_WORD_SIZE]
            .copy_from_slice(&second_node.to_le_bytes());
        bas_bytes.extend_from_slice(&SECOND_CONTROL_ENCODING.to_le_bytes());
        bas_bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let second_body = ScriptCodeOffset::new(bas_bytes.len());
        bas_bytes.push(MENU_OPCODE);
        bas_bytes.extend_from_slice(&MENU_WORD_ENCODING.to_le_bytes());
        bas_bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        bas_bytes.push(END_OPCODE);
        let dialogue = decode_script_bas(&bas_bytes, &dictionary).unwrap();

        let kind = ScriptObjectKind::Actor;
        let mut directory_entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
        directory_entry[..5].copy_from_slice(b"actor");
        directory_entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE]
            .copy_from_slice(&u16::MIN.to_le_bytes());
        directory_entry[DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE..]
            .copy_from_slice(&DIRECTORY_ACTIVE_KIND.to_le_bytes());
        let mut directory_bytes = directory_entry.to_vec();
        directory_bytes.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let mut state_bytes = vec![u8::MIN; kind.record_size()];
        state_bytes[..SERIALIZED_WORD_SIZE].copy_from_slice(&kind.mask().to_le_bytes());
        let field_offset =
            script_field_offset(kind, ScriptFieldSelector::DIALOGUE_CONTROL).unwrap();
        state_bytes[field_offset..field_offset + SERIALIZED_WORD_SIZE]
            .copy_from_slice(&field.to_le_bytes());
        let state = decode_script_state(&state_bytes, &directory).unwrap();
        let actor = state.objects()[usize::MIN].id;

        Fixture {
            dictionary,
            dialogue,
            state,
            actor,
            root,
            first_body,
            second_body,
            first_control,
            second_control,
            unmatched_control,
        }
    }

    fn control_for_encoding(fixture: &Fixture, encoded: u16) -> Option<ScriptWordId> {
        match encoded {
            u16::MIN => None,
            FIRST_CONTROL_ENCODING => Some(fixture.first_control),
            SECOND_CONTROL_ENCODING => Some(fixture.second_control),
            UNMATCHED_CONTROL_ENCODING => Some(fixture.unmatched_control),
            _ => panic!("unexpected oracle control {encoded:#06x}"),
        }
    }

    fn actor_control(fixture: &Fixture) -> u16 {
        let field_offset = script_field_offset(
            ScriptObjectKind::Actor,
            ScriptFieldSelector::DIALOGUE_CONTROL,
        )
        .unwrap();
        fixture
            .state
            .word(
                fixture
                    .state
                    .object_word(fixture.actor, field_offset / SERIALIZED_WORD_SIZE)
                    .unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn selector_control_accounts_for_every_original_natural_vector() {
        let vectors: Vec<ControlFlowOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_56fe_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut fixture = fixture(vector.field_before);
            let mut selector_state = ScriptSelectorState::default();
            selector_state.set_control_selections(
                control_for_encoding(&fixture, vector.branch_a_before),
                control_for_encoding(&fixture, vector.branch_b),
            );
            let mut runtime = ScriptRuntime::new();
            let mut offered_topic = None;
            let mut handler = RecordingHandler::default();
            let outcome = execute_selector_control(
                &mut fixture.state,
                ScriptControlFlowContext {
                    actor: fixture.actor,
                    dictionary: &fixture.dictionary,
                    dialogue: &fixture.dialogue,
                    selector_root: fixture.root,
                },
                &mut runtime,
                &mut selector_state,
                &mut offered_topic,
                &mut handler,
            )
            .unwrap();

            assert_eq!(
                outcome.selected_control,
                control_for_encoding(&fixture, vector.selected_control).unwrap(),
                "{}",
                vector.name
            );
            assert_eq!(
                actor_control(&fixture),
                vector.selected_control,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.current_body.is_some(),
                vector.first_match != u16::MIN
            );
            assert_eq!(
                outcome.parent_body.is_some(),
                vector.parent_match != u16::MIN
            );
            assert_eq!(
                outcome.current_execution.is_some(),
                vector.first_match != u16::MIN
            );
            assert_eq!(
                outcome.parent_execution.is_some(),
                vector.parent_match != u16::MIN
            );
            assert_eq!(outcome.menu_collected, vector.collector_called);
            assert_eq!(
                handler.calls.len(),
                vector.block_calls.len(),
                "{}",
                vector.name
            );

            if matches!(
                vector.name.as_str(),
                "negative_field_offset" | "wrapped_field_offset" | "lowest_kind_bit_selects_column"
            ) {
                assert_eq!(
                    script_field_offset(
                        ScriptObjectKind::Actor,
                        ScriptFieldSelector::DIALOGUE_CONTROL,
                    ),
                    Some(70)
                );
            }
        }
    }

    #[test]
    fn current_and_parent_bodies_execute_in_order() {
        let mut fixture = fixture(u16::MIN);
        let mut selector_state = ScriptSelectorState::default();
        selector_state
            .set_control_selections(Some(fixture.first_control), Some(fixture.second_control));
        let mut handler = RecordingHandler::default();
        let outcome = execute_selector_control(
            &mut fixture.state,
            ScriptControlFlowContext {
                actor: fixture.actor,
                dictionary: &fixture.dictionary,
                dialogue: &fixture.dialogue,
                selector_root: fixture.root,
            },
            &mut ScriptRuntime::new(),
            &mut selector_state,
            &mut None,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.current_body, Some(fixture.first_body));
        assert_eq!(outcome.parent_body, Some(fixture.second_body));
        assert_eq!(handler.calls, [fixture.first_body, fixture.second_body]);
        assert_eq!(
            selector_state.pending_presentation_words().len(),
            1,
            "only the current body publishes its menu"
        );
    }

    #[test]
    fn malformed_control_and_selector_fail_before_actor_mutation() {
        let mut bad_control_fixture = fixture(1);
        let before = bad_control_fixture.state.clone();
        assert!(matches!(
            execute_selector_control(
                &mut bad_control_fixture.state,
                ScriptControlFlowContext {
                    actor: bad_control_fixture.actor,
                    dictionary: &bad_control_fixture.dictionary,
                    dialogue: &bad_control_fixture.dialogue,
                    selector_root: bad_control_fixture.root,
                },
                &mut ScriptRuntime::new(),
                &mut ScriptSelectorState::default(),
                &mut None,
                &mut RecordingHandler::default(),
            ),
            Err(ScriptControlFlowError::UnknownControlWord { .. })
        ));
        assert_eq!(bad_control_fixture.state, before);

        let mut fixture = fixture(u16::MIN);
        let before = fixture.state.clone();
        assert!(matches!(
            execute_selector_control(
                &mut fixture.state,
                ScriptControlFlowContext {
                    actor: fixture.actor,
                    dictionary: &fixture.dictionary,
                    dialogue: &fixture.dialogue,
                    selector_root: fixture.first_body,
                },
                &mut ScriptRuntime::new(),
                &mut ScriptSelectorState::default(),
                &mut None,
                &mut RecordingHandler::default(),
            ),
            Err(ScriptControlFlowError::Selector(_))
        ));
        assert_eq!(fixture.state, before);
    }

    #[test]
    fn every_shipped_selector_list_dispatches_from_typed_profile_state() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut dispatched_lists = usize::MIN;

        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let dictionary = decode_script_dictionary(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap(),
            )
            .unwrap();
            let dialogue = decode_script_bas(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.BAS"))).unwrap(),
                &dictionary,
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let mut state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();
            let actor = state
                .objects()
                .iter()
                .find(|object| object.kind == ScriptObjectKind::Actor)
                .unwrap()
                .id;
            let graph: SelectorGraph = serde_json::from_slice(
                &std::fs::read(workspace_root.join(format!(
                    "re/vm/bas-control-flow/script{profile}.bas.cfg.json"
                )))
                .unwrap(),
            )
            .unwrap();
            let field_offset = script_field_offset(
                ScriptObjectKind::Actor,
                ScriptFieldSelector::DIALOGUE_CONTROL,
            )
            .unwrap();
            let control_field = state
                .object_word(actor, field_offset / SERIALIZED_WORD_SIZE)
                .unwrap();

            for list in graph.lists {
                assert!(state.set_word(control_field, u16::MIN));
                let outcome = execute_selector_control(
                    &mut state,
                    ScriptControlFlowContext {
                        actor,
                        dictionary: &dictionary,
                        dialogue: &dialogue,
                        selector_root: ScriptCodeOffset::new(list.node_offsets[usize::MIN]),
                    },
                    &mut ScriptRuntime::new(),
                    &mut ScriptSelectorState::default(),
                    &mut None,
                    &mut RecordingHandler::default(),
                )
                .unwrap();
                assert!(outcome.current_body.is_some(), "SCRIPT{profile}.BAS");
                dispatched_lists += 1;
            }
        }
        assert!(dispatched_lists > usize::MIN);
    }
}
