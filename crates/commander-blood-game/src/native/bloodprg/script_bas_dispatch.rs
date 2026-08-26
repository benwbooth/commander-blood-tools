//! Complete typed dispatch for executable instructions in BloodScript BAS bodies.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction, ScriptBasToken};
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
    ScriptBlockError, ScriptBlockFlow, ScriptBlockHandler, ScriptBlockOutcome, ScriptBlockStep,
    ScriptControl, ScriptControlFlowContext, ScriptControlFlowError, ScriptDispatchState,
    ScriptFrameFlow, ScriptProfileBuiltins, ScriptProfileRecordState,
    ScriptProfileRecordStateError, ScriptRecordError, ScriptRecordStateError, ScriptRuntime,
    ScriptSelectorBlockContext, ScriptSelectorControlHost, ScriptSelectorState,
    ScriptStateOperationError, ScriptTransferContext, SequenceRequestContext,
    TextInstructionExecutionError, TextInstructionState, apply_record_clear_operation,
    apply_shared_bit_operation, apply_shared_state_operation, apply_transfer, execute_script_block,
    execute_selector_control_with_host, execute_text_instruction, load_sequence_request,
};

/// Mutable state belonging to executable BAS instructions rather than profile VAR bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptBasDispatchState {
    text_instructions: BTreeMap<ScriptCodeOffset, TextInstructionState>,
}

impl ScriptBasDispatchState {
    /// Reset self-modified A6 activity when a script profile is selected again.
    pub fn reset(&mut self) {
        self.text_instructions.clear();
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
    /// Active profile's decoded BAS image.
    pub dialogue: &'a ScriptBas,
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
                load_sequence_request(
                    request,
                    self.host.sequence_context(),
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.sequence_presentation,
                );
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
                apply_transfer(
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
                .map_err(ScriptBasDispatchError::Record)?
                .control
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

    use commander_blood_formats::script::ScriptObjectKind;
    use serde::Deserialize;

    use super::*;
    use crate::assets::OriginalResourceStore;
    use crate::native::bloodprg::{
        OriginalResourceCache, OriginalResourceCatalog, OriginalScriptProfileCatalog,
        ScriptFieldSelector, ScriptProfileId, ScriptProfileManager, script_field_offset,
    };

    const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();
    const EXPECTED_SELECTOR_NODE_COUNTS: [usize; 5] = [1, 122, 98, 43, 57];
    const EXPECTED_TOTAL_SELECTOR_NODE_COUNT: usize = 321;

    #[derive(Deserialize)]
    struct SelectorGraph {
        nodes: Vec<SelectorNode>,
    }

    #[derive(Deserialize)]
    struct SelectorNode {
        offset: usize,
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
        [
            Path::new("output/_tmp_iso"),
            Path::new("commander-blood-audio/_tmp_iso"),
            Path::new("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find(|root| root.join("SCRIPT1.COD").is_file())
        .map(Path::to_owned)
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
                profile.synchronized_state().unwrap();
                executed_lists += 1;
            }
        }

        assert_eq!(executed_lists, EXPECTED_TOTAL_SELECTOR_NODE_COUNT);
    }
}
