//! Complete typed dispatch for executable instructions in BloodScript BAS bodies.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::bas::{ScriptBasInstruction, ScriptBasToken};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstructionError, decode_script_record_clear_operation,
    decode_script_shared_bit_operation, decode_script_shared_state_operation,
    decode_script_transfer,
};
use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptState, ScriptWordId,
};

use super::{
    PresentationResourceLine, ScriptBlockError, ScriptBlockFlow, ScriptBlockHandler,
    ScriptBlockOutcome, ScriptBlockStep, ScriptControl, ScriptControlFlowContext,
    ScriptControlFlowError, ScriptDispatchState, ScriptFrameFlow, ScriptProfileBuiltins,
    ScriptProfileRecordState, ScriptProfileRecordStateError, ScriptRecordError,
    ScriptRecordStateError, ScriptRuntime, ScriptSelectorBlockContext, ScriptSelectorControlHost,
    ScriptSelectorState, ScriptStateOperationError, ScriptTransferContext, SequenceRequestContext,
    TextInstructionExecutionError, TextInstructionState, apply_record_clear_operation,
    apply_shared_bit_operation, apply_shared_state_operation, apply_transfer, execute_script_block,
    execute_selector_control_with_host, execute_text_instruction, load_sequence_request,
};

/// Mutable state belonging to executable BAS instructions rather than profile VAR bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptBasDispatchState {
    text_instructions: BTreeMap<ScriptCodeOffset, TextInstructionState>,
    #[cfg(test)]
    last_published_text: Option<ScriptCodeOffset>,
}

impl ScriptBasDispatchState {
    /// Reset self-modified A6 activity when a script profile is selected again.
    pub fn reset(&mut self) {
        self.text_instructions.clear();
    }

    pub(super) fn activate_text(
        &mut self,
        source_offset: ScriptCodeOffset,
        text: &commander_blood_formats::instruction::ScriptText,
    ) {
        self.text_instructions
            .entry(source_offset)
            .or_insert_with(|| TextInstructionState::new(text))
            .activate();
    }

    #[cfg(test)]
    pub(super) fn text_state(
        &self,
        source_offset: ScriptCodeOffset,
    ) -> Option<TextInstructionState> {
        self.text_instructions.get(&source_offset).copied()
    }

    #[cfg(test)]
    fn take_last_published_text(&mut self) -> Option<ScriptCodeOffset> {
        self.last_published_text.take()
    }
}

/// Complete flat profile bindings used by one character dialogue handoff.
pub struct ScriptDialogueExecutionContext<'a> {
    /// Character entering dialogue control.
    pub actor: ScriptObjectId,
    /// First selector node in the character's authored BAS response list.
    pub selector_root: ScriptCodeOffset,
    /// Pre-bound COD instructions used to keep derived record stores coherent.
    pub instructions: &'a [DecodedScriptInstruction],
    /// Dialogue resource to validate before traversing this response list.
    pub dialogue: &'a dyn super::ScriptDialogueSource,
    /// Active VAR image.
    pub state: &'a mut ScriptState,
    /// Interned profile words.
    pub dictionary: &'a ScriptDictionary,
    /// Object and state-label directory.
    pub directory: &'a ScriptDirectory,
    /// Native specially named object bindings.
    pub builtins: ScriptProfileBuiltins,
    /// Shared COD and BAS control-flow state.
    pub runtime: &'a mut ScriptRuntime,
    /// Dialogue branches and recent concept history.
    pub selector: &'a mut ScriptSelectorState,
    /// Typed stores synchronized with the active VAR image.
    pub records: &'a mut ScriptProfileRecordState,
    /// Shared text, sequence, transfer, and random state.
    pub dispatch: &'a mut ScriptDispatchState,
    /// BAS-specific mutable A6 activity.
    pub bas: &'a mut ScriptBasDispatchState,
}

/// Dynamic presentation facts required by BAS A8 and CD instructions.
pub trait ScriptBasDispatchHost {
    /// Typed platform or descriptor failure.
    type Error;

    /// Return current UI gates for an A8 sequence request.
    fn sequence_context(&self) -> SequenceRequestContext;

    /// Resolve descriptor and interface gates for one CD transfer.
    fn transfer_context(
        &mut self,
        item: ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error>;
}

/// Invalid BAS instruction state or dynamic host failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptBasDispatchError<HostError> {
    /// A framed BAS token could not bind to the active flat profile.
    Instruction(ScriptInstructionError),
    /// A shared VAR instruction failed.
    State(ScriptStateOperationError),
    /// A CD transfer failed.
    Record(ScriptRecordError),
    /// A C9 action-record clear failed.
    ActionRecord(ScriptRecordStateError),
    /// Typed record stores could not be synchronized with VAR.
    ProfileRecord(ScriptProfileRecordStateError),
    /// A6 could not bind or execute its authored VAR line record.
    Text(TextInstructionExecutionError),
    /// Selector-node data was reached as executable body code.
    UnexpectedSelectorNode {
        /// Source position of the non-executable node.
        source_offset: ScriptCodeOffset,
    },
    /// A dynamic descriptor or presentation callback failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for ScriptBasDispatchError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HostError: fmt::Debug> std::error::Error for ScriptBasDispatchError<HostError> {}

/// Select and execute a character's current and parent BAS response bodies.
pub fn execute_script_dialogue_control<Host: ScriptBasDispatchHost>(
    context: ScriptDialogueExecutionContext<'_>,
    host: &mut Host,
) -> Result<
    super::ScriptControlFlowOutcome,
    ScriptControlFlowError<ScriptBasDispatchError<Host::Error>>,
> {
    let ScriptDialogueExecutionContext {
        actor,
        selector_root,
        instructions,
        dialogue,
        state,
        dictionary,
        directory,
        builtins,
        runtime,
        selector,
        records,
        dispatch,
        bas,
    } = context;
    #[cfg(test)]
    {
        bas.last_published_text = None;
    }
    let mut offered_topic = dispatch.sequence_presentation.offered_topic.take();
    let result = {
        let mut control_host = BasControlHost {
            instructions,
            dictionary,
            directory,
            builtins,
            records,
            dispatch,
            bas,
            host,
        };
        execute_selector_control_with_host(
            state,
            ScriptControlFlowContext {
                actor,
                dictionary,
                dialogue,
                selector_root,
            },
            runtime,
            selector,
            &mut offered_topic,
            &mut control_host,
        )
    };
    dispatch.sequence_presentation.offered_topic = offered_topic;
    result
}

struct BasControlHost<'a, Host> {
    instructions: &'a [DecodedScriptInstruction],
    dictionary: &'a ScriptDictionary,
    directory: &'a ScriptDirectory,
    builtins: ScriptProfileBuiltins,
    records: &'a mut ScriptProfileRecordState,
    dispatch: &'a mut ScriptDispatchState,
    bas: &'a mut ScriptBasDispatchState,
    host: &'a mut Host,
}

impl<Host: ScriptBasDispatchHost> ScriptSelectorControlHost for BasControlHost<'_, Host> {
    type Error = ScriptBasDispatchError<Host::Error>;

    fn execute_block(
        &mut self,
        context: ScriptSelectorBlockContext<'_>,
    ) -> Result<ScriptBlockOutcome, ScriptBlockError<Self::Error>> {
        let mut dispatcher = BasInstructionDispatcher {
            instructions: self.instructions,
            dictionary: self.dictionary,
            directory: self.directory,
            builtins: self.builtins,
            state: context.state,
            selector: context.selector,
            records: self.records,
            dispatch: self.dispatch,
            bas: self.bas,
            offered_topic: context.offered_topic,
            host: self.host,
        };
        execute_script_block(
            context.dialogue,
            context.start,
            context.runtime,
            &mut dispatcher,
        )
    }
}

struct BasInstructionDispatcher<'a, Host> {
    instructions: &'a [DecodedScriptInstruction],
    dictionary: &'a ScriptDictionary,
    directory: &'a ScriptDirectory,
    builtins: ScriptProfileBuiltins,
    state: &'a mut ScriptState,
    selector: &'a mut ScriptSelectorState,
    records: &'a mut ScriptProfileRecordState,
    dispatch: &'a mut ScriptDispatchState,
    bas: &'a mut ScriptBasDispatchState,
    offered_topic: &'a mut Option<ScriptWordId>,
    host: &'a mut Host,
}

impl<Host: ScriptBasDispatchHost> ScriptBlockHandler for BasInstructionDispatcher<'_, Host> {
    type Error = ScriptBasDispatchError<Host::Error>;

    fn execute_instruction(
        &mut self,
        token: &ScriptBasToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptBlockStep, Self::Error> {
        let mut refresh_from_var = false;
        let mut commit_to_var = false;
        let control = match token.instruction() {
            ScriptBasInstruction::Menu(_) => ScriptControl::Continue,
            ScriptBasInstruction::Text(text) => {
                let instruction_state = self
                    .bas
                    .text_instructions
                    .entry(token.source_offset())
                    .or_insert_with(|| TextInstructionState::new(text));
                let execution = execute_text_instruction(
                    text,
                    instruction_state,
                    self.dictionary,
                    self.state,
                    self.selector,
                    runtime,
                    &mut self.dispatch.random,
                    &mut self.dispatch.text_presentation,
                )
                .map_err(ScriptBasDispatchError::Text)?;
                #[cfg(test)]
                if execution.flow != ScriptFrameFlow::Continue {
                    self.bas.last_published_text = Some(token.source_offset());
                }
                return Ok(block_step_from_text_flow(
                    token.end_offset(),
                    execution.flow,
                ));
            }
            ScriptBasInstruction::Yield => {
                runtime.request_yield();
                debug_assert!(runtime.take_yield_request());
                return Ok(ScriptBlockStep::stop_at(token.end_offset()));
            }
            ScriptBasInstruction::SelectorYield => {
                runtime.request_selector_yield();
                debug_assert!(runtime.take_yield_request());
                return Ok(ScriptBlockStep::stop_at(token.end_offset()));
            }
            ScriptBasInstruction::SelectorNode { .. } => {
                return Err(ScriptBasDispatchError::UnexpectedSelectorNode {
                    source_offset: token.source_offset(),
                });
            }
            ScriptBasInstruction::TopicOffer(offer) => {
                if self.dispatch.sequence_presentation.presentation_active {
                    *self.offered_topic = offer.topic;
                }
                ScriptControl::Continue
            }
            ScriptBasInstruction::SequenceRequest(request) => {
                if load_sequence_request(
                    request,
                    self.host.sequence_context(),
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.sequence_presentation,
                ) {
                    self.dispatch
                        .record_active_line_write(PresentationResourceLine::Sequence.number());
                }
                ScriptControl::Continue
            }
            ScriptBasInstruction::SharedBitState(framed) => {
                refresh_from_var = !runtime.query_mode();
                let operation = decode_script_shared_bit_operation(framed, self.state)
                    .map_err(ScriptBasDispatchError::Instruction)?;
                apply_shared_bit_operation(operation, self.state, runtime)
                    .map_err(ScriptBasDispatchError::State)?
            }
            ScriptBasInstruction::SharedState(framed) => {
                refresh_from_var = !runtime.query_mode();
                let operation = decode_script_shared_state_operation(framed, self.state)
                    .map_err(ScriptBasDispatchError::Instruction)?;
                apply_shared_state_operation(operation, self.state, runtime)
                    .map_err(ScriptBasDispatchError::State)?
            }
            ScriptBasInstruction::RecordClear(framed) => {
                let operation = decode_script_record_clear_operation(framed, self.state)
                    .map_err(ScriptBasDispatchError::Instruction)?;
                apply_record_clear_operation(
                    operation,
                    self.state,
                    &mut self.records.action_records,
                    &mut self.dispatch.record_clear_presentation,
                )
                .map_err(ScriptBasDispatchError::ActionRecord)?;
                commit_to_var = true;
                ScriptControl::Continue
            }
            ScriptBasInstruction::RecordTriple(framed) => {
                commit_to_var = !runtime.query_mode();
                let transfer = decode_script_transfer(framed, self.state, self.directory)
                    .map_err(ScriptBasDispatchError::Instruction)?;
                let context = self
                    .host
                    .transfer_context(transfer.item)
                    .map_err(ScriptBasDispatchError::Host)?;
                let outcome = apply_transfer(
                    transfer,
                    self.state,
                    &self.records.transfer_records,
                    &mut self.records.record_fields,
                    &mut self.records.record_runtime,
                    context,
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.transfer_presentation,
                    runtime,
                )
                .map_err(ScriptBasDispatchError::Record)?;
                if outcome.presentation_requested {
                    let line = self
                        .dispatch
                        .transfer_presentation
                        .active_line
                        .expect("CD reports a request only after selecting an active line");
                    self.dispatch.record_active_line_write(line.number());
                }
                outcome.control
            }
            ScriptBasInstruction::End => unreachable!("block traversal handles BAS end markers"),
        };

        if refresh_from_var {
            self.records
                .refresh_from_var(
                    self.instructions,
                    self.state,
                    self.dictionary,
                    self.builtins,
                )
                .map_err(ScriptBasDispatchError::ProfileRecord)?;
        } else if commit_to_var {
            self.records
                .commit_to_var(self.state, self.directory, self.dictionary)
                .map_err(ScriptBasDispatchError::ProfileRecord)?;
        }
        Ok(block_step_from_control(token.end_offset(), control))
    }
}

const fn block_step_from_text_flow(
    next: ScriptCodeOffset,
    flow: ScriptFrameFlow,
) -> ScriptBlockStep {
    match flow {
        ScriptFrameFlow::Continue => ScriptBlockStep::continue_at(next),
        ScriptFrameFlow::ContinueAfterPresentation | ScriptFrameFlow::SaveResumeCursor => {
            ScriptBlockStep::continue_after_presentation(next)
        }
    }
}

const fn block_step_from_control(
    next: ScriptCodeOffset,
    control: ScriptControl,
) -> ScriptBlockStep {
    ScriptBlockStep {
        next_instruction: match control {
            ScriptControl::Continue => next,
            ScriptControl::Jump(target) => target,
        },
        flow: ScriptBlockFlow::Continue,
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::instruction::{ScriptText, ScriptTextWord};
    use commander_blood_formats::script::ScriptObjectKind;
    use serde::Deserialize;

    use super::*;
    use crate::assets::OriginalResourceStore;
    use crate::native::bloodprg::{
        LoadedScriptProfile, OriginalResourceCache, OriginalResourceCatalog,
        OriginalScriptProfileCatalog, ScriptActionRecord, ScriptFieldSelector, ScriptProfileId,
        ScriptProfileManager, ScriptSelectionOutcome, TextPresentationState,
        commit_selected_concept, script_field_offset,
    };
    use crate::native::random::BloodPrng;

    const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();
    const FIRST_CONCEPT_HISTORY_SLOT: usize = usize::MIN;
    const EXPECTED_SELECTOR_NODE_COUNTS: [usize; 5] = [1, 122, 98, 43, 57];
    const EXPECTED_TOTAL_SELECTOR_NODE_COUNT: usize = 321;
    const EXPECTED_TOTAL_MENU_CHOICE_COUNT: usize = 1_396;
    const EXPECTED_TOTAL_DIALOGUE_EVENT_COUNT: usize = 1_849;
    const EXPECTED_REACHABLE_DIALOGUE_EVENT_COUNT: usize = 1_847;
    const EXPECTED_UNREACHABLE_DIALOGUE_EVENT_COUNT: usize = 2;
    const MAXIMUM_DIALOGUE_TARGET_PASSES: usize = 96;
    const LINE_FLAGS_BYTE_OFFSET: usize = std::mem::size_of::<u16>();
    const LINE_ALREADY_SHOWN_FLAG: u16 = 0x8000;
    const RECORD_SELECTOR_SHIFT: u32 = 1;
    const RECORD_SELECTOR_MASK: u8 = 0x07;
    const FIRST_CONDITIONAL_RECORD_SELECTOR: u8 = 1;
    const HISTORY_REQUIRED_MASK: u8 = 0x07;
    const RANDOM_WARMUP_LIMIT: u8 = 128;
    const CLOCK_SECONDS_PER_MINUTE: u8 = 60;

    #[derive(Deserialize)]
    struct SelectorGraph {
        lists: Vec<SelectorList>,
        nodes: Vec<SelectorNode>,
    }

    #[derive(Deserialize)]
    struct SelectorNode {
        offset: usize,
        selector: u16,
        body_start: usize,
        list_index: usize,
        menu_choices: Vec<SelectorMenuChoice>,
        dialogue_events: Vec<SelectorDialogueEvent>,
    }

    #[derive(Deserialize)]
    struct SelectorList {
        entrypoint: SelectorEntrypoint,
        node_offsets: Vec<usize>,
    }

    #[derive(Deserialize)]
    struct SelectorEntrypoint {
        object_name: String,
        root_node: usize,
    }

    #[derive(Deserialize)]
    struct SelectorMenuChoice {
        offset: u16,
    }

    #[derive(Deserialize)]
    struct SelectorDialogueEvent {
        offset: usize,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct UnreachableDialogueEvent {
        profile: u8,
        node: usize,
        blocker: usize,
        target: usize,
    }

    struct TestHost;

    impl ScriptBasDispatchHost for TestHost {
        type Error = Infallible;

        fn sequence_context(&self) -> SequenceRequestContext {
            SequenceRequestContext {
                ship_active: true,
                scene_gate_active: true,
            }
        }

        fn transfer_context(
            &mut self,
            _item: ScriptObjectId,
        ) -> Result<ScriptTransferContext, Self::Error> {
            Ok(ScriptTransferContext::default())
        }
    }

    fn original_data_root() -> Option<PathBuf> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            Path::new("output/_tmp_iso"),
            Path::new("commander-blood-audio/_tmp_iso"),
            Path::new("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .map(|root| workspace_root.join(root))
        .find(|root| root.join("SCRIPT1.COD").is_file())
    }

    #[test]
    fn every_shipped_selector_body_executes_through_complete_typed_dispatch() {
        let Some(root) = original_data_root() else {
            return;
        };
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);
        let mut executed_lists = usize::MIN;

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let template = manager.current().unwrap().clone();
            let graph: SelectorGraph = serde_json::from_slice(
                &std::fs::read(workspace_root.join(format!(
                    "re/vm/bas-control-flow/script{}.bas.cfg.json",
                    profile_id.value() + 1
                )))
                .unwrap(),
            )
            .unwrap();
            assert_eq!(
                graph.nodes.len(),
                EXPECTED_SELECTOR_NODE_COUNTS[usize::from(profile_id.value())]
            );

            for node in graph.nodes {
                let mut profile = template.clone();
                let actor = profile
                    .state()
                    .objects()
                    .iter()
                    .find(|object| object.kind == ScriptObjectKind::Actor)
                    .unwrap()
                    .id;
                let control_offset = script_field_offset(
                    ScriptObjectKind::Actor,
                    ScriptFieldSelector::DIALOGUE_CONTROL,
                )
                .unwrap();
                let control = profile
                    .state()
                    .object_word(actor, control_offset / SERIALIZED_WORD_SIZE)
                    .unwrap();
                assert!(profile.state_mut().set_word(control, u16::MIN));
                let parts = profile.execution_parts();
                let mut dispatch = ScriptDispatchState::default();
                dispatch.sequence_presentation.presentation_active = true;
                let outcome = execute_script_dialogue_control(
                    ScriptDialogueExecutionContext {
                        actor,
                        selector_root: ScriptCodeOffset::new(node.offset),
                        instructions: parts.instructions,
                        dialogue: parts.dialogue,
                        state: parts.state,
                        dictionary: parts.dictionary,
                        directory: parts.directory,
                        builtins: parts.builtins,
                        runtime: parts.runtime,
                        selector: parts.selector_state,
                        records: parts.record_state,
                        dispatch: &mut dispatch,
                        bas: &mut ScriptBasDispatchState::default(),
                    },
                    &mut TestHost,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "SCRIPT{} BAS list {:#06x} failed: {error:?}",
                        profile_id.value() + 1,
                        node.offset
                    )
                });
                if outcome.menu_collected {
                    assert!(
                        !profile
                            .selector_state()
                            .pending_presentation_words()
                            .is_empty(),
                        "SCRIPT{} BAS list {:#06x} collected an empty selector menu",
                        profile_id.value() + 1,
                        node.offset
                    );
                    if dispatch
                        .text_presentation
                        .request_flags
                        .text_request_pending()
                    {
                        assert!(
                            !dispatch.text_presentation.menu_words.is_empty()
                                || !dispatch.text_presentation.subtitle_text.is_empty()
                        );
                    } else {
                        assert!(dispatch.text_presentation.menu_words.is_empty());
                    }
                }
                profile.synchronized_state().unwrap();
                executed_lists += 1;
            }
        }

        assert_eq!(executed_lists, EXPECTED_TOTAL_SELECTOR_NODE_COUNT);
    }

    #[test]
    fn every_recovered_menu_choice_commits_and_dispatches_through_typed_runtime() {
        let Some(root) = original_data_root() else {
            return;
        };
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);
        let mut dispatched_choices = usize::MIN;

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let template = manager.current().unwrap().clone();
            let graph: SelectorGraph = serde_json::from_slice(
                &std::fs::read(workspace_root.join(format!(
                    "re/vm/bas-control-flow/script{}.bas.cfg.json",
                    profile_id.value() + 1
                )))
                .unwrap(),
            )
            .unwrap();

            for node in &graph.nodes {
                let list = &graph.lists[node.list_index];
                for choice in &node.menu_choices {
                    let mut profile = template.clone();
                    let actor = profile
                        .directory()
                        .find_active_object(list.entrypoint.object_name.as_bytes())
                        .unwrap_or_else(|| {
                            panic!(
                                "SCRIPT{} has no active object named {}",
                                profile_id.value() + 1,
                                list.entrypoint.object_name
                            )
                        });
                    let control = profile
                        .dictionary()
                        .resolve_source_offset(node.selector)
                        .unwrap();
                    let selected = profile
                        .dictionary()
                        .resolve_source_offset(choice.offset)
                        .unwrap();
                    let expected_menu = node
                        .menu_choices
                        .iter()
                        .map(|menu_choice| {
                            profile
                                .dictionary()
                                .resolve_source_offset(menu_choice.offset)
                                .unwrap()
                        })
                        .collect::<Vec<_>>();
                    profile
                        .selector_state_mut()
                        .set_control_selections(Some(control), None);

                    let mut dispatch = ScriptDispatchState::default();
                    dispatch.sequence_presentation.presentation_active = true;
                    let mut bas = ScriptBasDispatchState::default();
                    let initial_outcome = {
                        let parts = profile.execution_parts();
                        execute_script_dialogue_control(
                            ScriptDialogueExecutionContext {
                                actor,
                                selector_root: ScriptCodeOffset::new(node.offset),
                                instructions: parts.instructions,
                                dialogue: parts.dialogue,
                                state: parts.state,
                                dictionary: parts.dictionary,
                                directory: parts.directory,
                                builtins: parts.builtins,
                                runtime: parts.runtime,
                                selector: parts.selector_state,
                                records: parts.record_state,
                                dispatch: &mut dispatch,
                                bas: &mut bas,
                            },
                            &mut TestHost,
                        )
                    }
                    .unwrap_or_else(|error| {
                        panic!(
                            "SCRIPT{} node {:#06x} failed before selecting {:#06x}: {error:?}",
                            profile_id.value() + 1,
                            node.offset,
                            choice.offset
                        )
                    });
                    assert_eq!(
                        initial_outcome.current_body,
                        Some(ScriptCodeOffset::new(node.body_start))
                    );
                    assert!(initial_outcome.menu_collected);
                    assert_eq!(
                        profile.selector_state().pending_presentation_words(),
                        expected_menu
                    );
                    assert!(expected_menu.contains(&selected));

                    profile.runtime_mut().set_selected_concept(Some(selected));
                    dispatch.text_presentation = TextPresentationState::default();
                    let expected_matched_node = list.node_offsets.iter().find_map(|node_offset| {
                        let candidate = graph
                            .nodes
                            .iter()
                            .find(|candidate| candidate.offset == *node_offset)
                            .unwrap();
                        (candidate.selector == choice.offset).then_some(candidate)
                    });
                    let expected_matched_body = expected_matched_node
                        .map(|candidate| ScriptCodeOffset::new(candidate.body_start));
                    let expected_response_menu = expected_matched_node.map_or_else(
                        || expected_menu.clone(),
                        |candidate| {
                            candidate
                                .menu_choices
                                .iter()
                                .map(|menu_choice| {
                                    profile
                                        .dictionary()
                                        .resolve_source_offset(menu_choice.offset)
                                        .unwrap()
                                })
                                .collect::<Vec<_>>()
                        },
                    );
                    let selection_outcome = {
                        let parts = profile.execution_parts();
                        commit_selected_concept(
                            parts.runtime,
                            parts.dictionary,
                            parts.dialogue,
                            Some(ScriptCodeOffset::new(list.entrypoint.root_node)),
                            parts.selector_state,
                        )
                    }
                    .unwrap();
                    assert_eq!(
                        selection_outcome,
                        ScriptSelectionOutcome::Committed {
                            concept: selected,
                            matched_body: expected_matched_body,
                            menu_activated: expected_matched_body.is_some(),
                        }
                    );

                    let response_root = expected_matched_body
                        .map(|_| list.entrypoint.root_node)
                        .unwrap_or(node.offset);
                    let expected_response_body = expected_matched_body
                        .unwrap_or_else(|| ScriptCodeOffset::new(node.body_start));
                    let response_outcome = {
                        let parts = profile.execution_parts();
                        execute_script_dialogue_control(
                            ScriptDialogueExecutionContext {
                                actor,
                                selector_root: ScriptCodeOffset::new(response_root),
                                instructions: parts.instructions,
                                dialogue: parts.dialogue,
                                state: parts.state,
                                dictionary: parts.dictionary,
                                directory: parts.directory,
                                builtins: parts.builtins,
                                runtime: parts.runtime,
                                selector: parts.selector_state,
                                records: parts.record_state,
                                dispatch: &mut dispatch,
                                bas: &mut bas,
                            },
                            &mut TestHost,
                        )
                    }
                    .unwrap_or_else(|error| {
                        panic!(
                            "SCRIPT{} node {:#06x} failed after selecting {:#06x}: {error:?}",
                            profile_id.value() + 1,
                            node.offset,
                            choice.offset
                        )
                    });
                    assert_eq!(response_outcome.current_body, Some(expected_response_body));
                    let published_words = dispatch
                        .text_presentation
                        .menu_words
                        .iter()
                        .filter_map(|word| match word {
                            ScriptTextWord::Dictionary(word) => Some(*word),
                            ScriptTextWord::SectionSeparator => None,
                            ScriptTextWord::StateNumber(_) => {
                                panic!("Commander fixture cannot contain a sequel number")
                            }
                        })
                        .collect::<Vec<_>>();
                    if dispatch
                        .text_presentation
                        .request_flags
                        .text_request_pending()
                    {
                        assert_ne!(
                            published_words,
                            expected_menu,
                            "SCRIPT{} node {:#06x} overwrote the selected response with its returning menu",
                            profile_id.value() + 1,
                            node.offset
                        );
                        assert!(
                            !published_words.is_empty()
                                || !dispatch.text_presentation.subtitle_text.is_empty(),
                            "SCRIPT{} node {:#06x} requested text without publishing response words",
                            profile_id.value() + 1,
                            node.offset
                        );
                    } else {
                        assert!(
                            published_words.is_empty(),
                            "SCRIPT{} node {:#06x} published selector choices as dialogue text",
                            profile_id.value() + 1,
                            node.offset
                        );
                    }
                    if response_outcome.menu_collected {
                        assert_eq!(
                            profile.selector_state().pending_presentation_words(),
                            expected_response_menu,
                            "SCRIPT{} node {:#06x} lost its independently collected selector menu",
                            profile_id.value() + 1,
                            node.offset
                        );
                    }
                    assert_eq!(
                        profile.selector_state().history().entries()[FIRST_CONCEPT_HISTORY_SLOT],
                        Some(selected)
                    );
                    profile.synchronized_state().unwrap();
                    dispatched_choices += 1;
                }
            }
        }

        assert_eq!(dispatched_choices, EXPECTED_TOTAL_MENU_CHOICE_COUNT);
    }

    #[test]
    fn every_recovered_dialogue_event_publishes_through_typed_dispatch() {
        let Some(root) = original_data_root() else {
            return;
        };
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);
        let random_states = reachable_random_states(EXPECTED_TOTAL_DIALOGUE_EVENT_COUNT);
        let mut published_events = usize::MIN;
        let mut failures = Vec::new();
        let unreachable = unreachable_dialogue_events();
        assert_eq!(unreachable.len(), EXPECTED_UNREACHABLE_DIALOGUE_EVENT_COUNT);

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let template = manager.current().unwrap().clone();
            let graph: SelectorGraph = serde_json::from_slice(
                &std::fs::read(workspace_root.join(format!(
                    "re/vm/bas-control-flow/script{}.bas.cfg.json",
                    profile_id.value() + 1
                )))
                .unwrap(),
            )
            .unwrap();

            for node in &graph.nodes {
                let list = &graph.lists[node.list_index];
                for event in &node.dialogue_events {
                    if unreachable.iter().any(|entry| {
                        entry.profile == profile_id.value() + 1
                            && entry.node == node.offset
                            && entry.target == event.offset
                    }) {
                        continue;
                    }
                    let mut profile = template.clone();
                    let actor = profile
                        .directory()
                        .find_active_object(list.entrypoint.object_name.as_bytes())
                        .unwrap_or_else(|| {
                            panic!(
                                "SCRIPT{} has no active object named {}",
                                profile_id.value() + 1,
                                list.entrypoint.object_name
                            )
                        });
                    let control = profile
                        .dictionary()
                        .resolve_source_offset(node.selector)
                        .unwrap();
                    profile
                        .selector_state_mut()
                        .set_control_selections(Some(control), None);
                    configure_dialogue_target(&mut profile, event.offset);

                    let mut dispatch = ScriptDispatchState::default();
                    dispatch.sequence_presentation.presentation_active = true;
                    let mut bas = ScriptBasDispatchState::default();
                    let mut observed = Vec::new();
                    let mut reached = false;

                    for pass in usize::MIN..MAXIMUM_DIALOGUE_TARGET_PASSES {
                        dispatch.random = random_states[pass % random_states.len()];
                        let outcome = {
                            let parts = profile.execution_parts();
                            execute_script_dialogue_control(
                                ScriptDialogueExecutionContext {
                                    actor,
                                    selector_root: ScriptCodeOffset::new(node.offset),
                                    instructions: parts.instructions,
                                    dialogue: parts.dialogue,
                                    state: parts.state,
                                    dictionary: parts.dictionary,
                                    directory: parts.directory,
                                    builtins: parts.builtins,
                                    runtime: parts.runtime,
                                    selector: parts.selector_state,
                                    records: parts.record_state,
                                    dispatch: &mut dispatch,
                                    bas: &mut bas,
                                },
                                &mut TestHost,
                            )
                        }
                        .unwrap_or_else(|error| {
                            panic!(
                                "SCRIPT{} node {:#06x} failed while targeting {:#06x}: {error:?}",
                                profile_id.value() + 1,
                                node.offset,
                                event.offset
                            )
                        });
                        assert_eq!(
                            outcome.current_body,
                            Some(ScriptCodeOffset::new(node.body_start))
                        );

                        let Some(published) = bas.take_last_published_text() else {
                            continue;
                        };
                        observed.push(published.index());
                        if published.index() == event.offset {
                            reached = true;
                            break;
                        }
                        clear_published_dialogue(&mut profile, published);
                        dispatch.text_presentation = TextPresentationState::default();
                    }

                    if reached {
                        profile.synchronized_state().unwrap_or_else(|error| {
                            panic!(
                                "SCRIPT{} BAS target {:#06x} left incoherent state: {error:?}",
                                profile_id.value() + 1,
                                event.offset
                            )
                        });
                        published_events += 1;
                    } else {
                        failures.push((
                            profile_id.value() + 1,
                            list.entrypoint.object_name.clone(),
                            node.offset,
                            event.offset,
                            observed,
                        ));
                    }
                }
            }
        }

        assert!(
            failures.is_empty(),
            "recovered BAS dialogue events did not publish: {failures:#?}"
        );
        assert_eq!(published_events, EXPECTED_REACHABLE_DIALOGUE_EVENT_COUNT);
    }

    #[test]
    fn declared_unreachable_dialogue_events_remain_dominated_by_repeatable_text() {
        let Some(root) = original_data_root() else {
            return;
        };
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);
        let unreachable = unreachable_dialogue_events();

        for entry in unreachable {
            let profile_id = ScriptProfileId::new(entry.profile - 1).unwrap();
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let profile = manager.current().unwrap();
            let blocker = dialogue_text_at(profile, entry.blocker);
            let target = dialogue_text_at(profile, entry.target);

            assert!(entry.node < entry.blocker && entry.blocker < entry.target);
            assert!(blocker.control.is_active());
            assert!(blocker.control.preserves_active());
            assert!(blocker.control.uses_history_condition());
            assert!(!blocker.control.uses_random_gate());
            assert!(!blocker.control.uses_record_condition());
            assert_eq!(blocker.control.rejection_skip_count(), None);
            assert!(target.control.is_active());
            assert!(target.control.uses_history_condition());
            assert_eq!(
                history_condition_words(blocker),
                history_condition_words(target)
            );
            assert_eq!(blocker.line_record, target.line_record);
        }
    }

    fn configure_dialogue_target(profile: &mut LoadedScriptProfile, target_offset: usize) {
        let target = profile
            .dialogue()
            .decoded()
            .unwrap()
            .tokens()
            .iter()
            .find(|token| token.source_offset().index() == target_offset)
            .unwrap_or_else(|| panic!("BAS target {target_offset:#06x} is not decoded"));
        let ScriptBasInstruction::Text(text) = target.instruction() else {
            panic!("BAS target {target_offset:#06x} is not a text instruction");
        };
        let text = text.clone();

        let line_offset = text.line_record.byte_offset();
        let line_kind = profile
            .state()
            .objects()
            .iter()
            .find(|object| object.source_offset() == line_offset)
            .unwrap_or_else(|| {
                panic!("BAS text {target_offset:#06x} has no line record {line_offset:#06x}")
            })
            .kind;
        let action_offset = script_field_offset(line_kind, ScriptFieldSelector::ACTION).unwrap();
        let action = u16::try_from(line_offset + action_offset).unwrap();
        let action = profile.state().resolve_word_source_offset(action).unwrap();
        assert!(
            profile
                .state_mut()
                .set_word(action, ScriptActionRecord::ACTOR_PRESENTATION_KIND)
        );
        let flags_offset = u16::try_from(line_offset + LINE_FLAGS_BYTE_OFFSET).unwrap();
        let flags = profile
            .state()
            .resolve_word_source_offset(flags_offset)
            .unwrap();
        let value = profile.state().word(flags).unwrap() & !LINE_ALREADY_SHOWN_FLAG;
        assert!(profile.state_mut().set_word(flags, value));

        if text.control.uses_history_condition() {
            let candidates = text
                .words
                .split(|word| matches!(word, ScriptTextWord::SectionSeparator))
                .nth(1)
                .expect("history-gated BAS text retains its condition section")
                .iter()
                .filter_map(|word| match word {
                    ScriptTextWord::Dictionary(word) => Some(*word),
                    ScriptTextWord::SectionSeparator => None,
                    ScriptTextWord::StateNumber(_) => {
                        panic!("Commander fixture cannot contain a sequel number")
                    }
                })
                .collect::<Vec<_>>();
            assert!(
                !candidates.is_empty(),
                "history-gated BAS text {target_offset:#06x} has no candidates"
            );
            let required = usize::from(text.control.detail() & HISTORY_REQUIRED_MASK);
            let insertions = if required == usize::MIN {
                super::super::SCRIPT_CONCEPT_HISTORY_LENGTH
            } else {
                required
            };
            for index in usize::MIN..insertions {
                profile
                    .selector_state_mut()
                    .history_mut()
                    .push(candidates[index % candidates.len()]);
            }
        }

        if text.control.uses_record_condition() {
            let selector_index = ((text.control.detail() >> RECORD_SELECTOR_SHIFT)
                & RECORD_SELECTOR_MASK)
                .wrapping_add(FIRST_CONDITIONAL_RECORD_SELECTOR);
            let selector = ScriptFieldSelector::new(selector_index).unwrap();
            let line = profile
                .state()
                .objects()
                .iter()
                .find(|object| object.source_offset() == line_offset)
                .unwrap_or_else(|| {
                    panic!("BAS text {target_offset:#06x} has no line record {line_offset:#06x}")
                });
            let field_offset = script_field_offset(line.kind, selector).unwrap();
            let target = u16::try_from(line_offset + field_offset).unwrap();
            let target = profile.state().resolve_word_source_offset(target).unwrap();
            let operand = text.record_condition_operand.unwrap();
            let accepted = if text.control.detail() & 1 != u8::MIN {
                operand
            } else {
                (operand as i16).checked_add(1).unwrap() as u16
            };
            assert!(profile.state_mut().set_word(target, accepted));
        }
    }

    fn clear_published_dialogue(profile: &mut LoadedScriptProfile, published: ScriptCodeOffset) {
        let token = profile
            .dialogue()
            .decoded()
            .unwrap()
            .tokens()
            .iter()
            .find(|token| token.source_offset() == published)
            .unwrap();
        let ScriptBasInstruction::Text(text) = token.instruction() else {
            unreachable!("published BAS source is always text");
        };
        let flags_offset =
            u16::try_from(text.line_record.byte_offset() + LINE_FLAGS_BYTE_OFFSET).unwrap();
        let flags = profile
            .state()
            .resolve_word_source_offset(flags_offset)
            .unwrap();
        let value = profile.state().word(flags).unwrap() & !LINE_ALREADY_SHOWN_FLAG;
        assert!(profile.state_mut().set_word(flags, value));
    }

    fn reachable_random_states(call_count: usize) -> Vec<BloodPrng> {
        let mut uncovered = (usize::MIN..call_count).collect::<std::collections::BTreeSet<_>>();
        let mut selected = Vec::new();
        for seconds in u8::MIN..CLOCK_SECONDS_PER_MINUTE {
            for warmup in u8::MIN..RANDOM_WARMUP_LIMIT {
                let mut candidate = BloodPrng::default();
                candidate.seed_from_clock_register(seconds);
                for _ in u8::MIN..warmup {
                    candidate.next(u16::MIN);
                }
                let mut probe = candidate;
                let covered = (usize::MIN..call_count)
                    .filter(|_index| probe.next(5) == u16::MIN)
                    .filter(|index| uncovered.contains(index))
                    .collect::<Vec<_>>();
                if covered.is_empty() {
                    continue;
                }
                selected.push(candidate);
                for index in covered {
                    uncovered.remove(&index);
                }
                if uncovered.is_empty() {
                    return selected;
                }
            }
        }
        assert!(uncovered.is_empty(), "uncovered PRNG calls: {uncovered:?}");
        selected
    }

    fn unreachable_dialogue_events() -> Vec<UnreachableDialogueEvent> {
        let rows = include_str!("../../../../../re/vm/bas-control-flow/unreachable-dialogue.tsv");
        rows.lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let columns = line.split('\t').collect::<Vec<_>>();
                assert_eq!(columns.len(), 6);
                assert!(!columns[5].trim().is_empty());
                UnreachableDialogueEvent {
                    profile: columns[0].strip_prefix("SCRIPT").unwrap().parse().unwrap(),
                    node: parse_hex_offset(columns[2]),
                    blocker: parse_hex_offset(columns[3]),
                    target: parse_hex_offset(columns[4]),
                }
            })
            .collect()
    }

    fn parse_hex_offset(value: &str) -> usize {
        usize::from_str_radix(value.strip_prefix("0x").unwrap(), 16).unwrap()
    }

    fn dialogue_text_at(profile: &LoadedScriptProfile, offset: usize) -> &ScriptText {
        let token = profile
            .dialogue()
            .decoded()
            .unwrap()
            .tokens()
            .iter()
            .find(|token| token.source_offset().index() == offset)
            .unwrap_or_else(|| panic!("missing BAS text at {offset:#06x}"));
        let ScriptBasInstruction::Text(text) = token.instruction() else {
            panic!("BAS offset {offset:#06x} is not text");
        };
        text
    }

    fn history_condition_words(text: &ScriptText) -> Vec<ScriptWordId> {
        text.words
            .split(|word| matches!(word, ScriptTextWord::SectionSeparator))
            .nth(1)
            .expect("history-gated text retains its condition section")
            .iter()
            .filter_map(|word| match word {
                ScriptTextWord::Dictionary(word) => Some(*word),
                ScriptTextWord::SectionSeparator => None,
                ScriptTextWord::StateNumber(_) => {
                    panic!("Commander fixture cannot contain a sequel number")
                }
            })
            .collect()
    }
}
