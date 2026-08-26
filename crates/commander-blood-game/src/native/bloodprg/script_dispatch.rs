//! Exhaustive production dispatch for pre-decoded BloodScript COD instructions.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::bas::ScriptBas;
use commander_blood_formats::code::{ScriptCodeOffset, ScriptToken};
use commander_blood_formats::instruction::DecodedScriptInstruction;
use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptState, ScriptWordId,
};

use crate::native::random::BloodPrng;

use super::procedure::{
    ScriptProcedureStateError, apply_procedure_activation, evaluate_procedure_gate,
};
use super::record::{
    ScriptRecordError, ScriptRecordPairReference, ScriptTransferContext,
    ScriptTransferPresentationState, apply_direct_record_operation, apply_record_pair_operation,
    apply_transfer,
};
use super::record_state::{
    ScriptAboardPresentationState, ScriptAboardRecordContext, ScriptRecordClearPresentationState,
    ScriptRecordStateError, ScriptRecordStateNavigationContext, apply_aboard_record_operation,
    apply_active_object_record_operation, apply_actor_record_operation,
    apply_opaque_marker_record_operation, apply_presentation_queue_operation,
    apply_record_clear_operation, apply_record_state_operation, apply_travel_record_operation,
    apply_world_state_record_operation,
};
use super::script_clock::ScriptClock;
use super::script_environment::ScriptEnvironmentActivity;
use super::script_frame::{
    DecodedScriptFrameHost, ScriptFrameError, ScriptFrameFlow, ScriptFrameOutcome, ScriptFrameStep,
    execute_decoded_script_frame,
};
use super::script_profile::{
    LoadedScriptExecutionParts, LoadedScriptProfile, ScriptProfileBuiltins,
    ScriptProfileRecordState, ScriptProfileRecordStateError,
};
use super::script_selector::{ScriptSelectionError, ScriptSelectorState, commit_selected_concept};
use super::sequence::{
    SequencePresentationState, SequenceRequestContext, load_sequence_request,
    offer_topic_if_presentation_active,
};
use super::state::{
    ScriptStateOperationError, apply_bit_flag_operation, apply_shared_bit_operation,
    apply_shared_state_operation,
};
use super::{
    PendingScriptProfileRequest, ScriptControl, ScriptFrameEnd, ScriptProfileRequestSlot,
    ScriptRuntime, ScriptRuntimeError, TextInstructionExecutionError, TextInstructionState,
    TextPresentationState, execute_text_instruction,
};

/// Mutable non-profile state shared by exhaustive COD dispatch and its host services.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptDispatchState {
    /// Original pseudo-random generator shared by control and text handlers.
    pub random: BloodPrng,
    /// Mutable active bits that the DOS executable stored inside A6 COD tokens.
    pub text_instructions: BTreeMap<ScriptCodeOffset, TextInstructionState>,
    /// Subtitle, menu, chatter, and presentation request state.
    pub text_presentation: TextPresentationState,
    /// Topic and sequence request state.
    pub sequence_presentation: SequencePresentationState,
    /// Reference invalidated by B8/B9/BD pair writes.
    pub record_pair_reference: ScriptRecordPairReference,
    /// Presentation state changed by successful C2 operations.
    pub aboard_presentation: ScriptAboardPresentationState,
    /// Presentation state changed by successful CD transfers.
    pub transfer_presentation: ScriptTransferPresentationState,
    /// C9 teardown effects consumed by the post-frame presentation path.
    pub record_clear_presentation: ScriptRecordClearPresentationState,
    /// D2 request retained until the main loop completes profile replacement.
    pub profile_request: ScriptProfileRequestSlot,
}

/// Mutable state exposed to the recovered post-frame presentation scan.
pub struct ScriptPostScanContext<'a> {
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
    /// Complete typed record state synchronized with VAR after the scan.
    pub records: &'a mut ScriptProfileRecordState,
    /// Active selector state.
    pub selector: &'a mut ScriptSelectorState,
    /// Shared COD control flow.
    pub runtime: &'a mut ScriptRuntime,
    /// Shared presentation and action-dispatch state.
    pub dispatch: &'a mut ScriptDispatchState,
    /// Native specially named object bindings.
    pub builtins: ScriptProfileBuiltins,
}

/// Platform and presentation facts required by otherwise fully translated COD handlers.
pub trait ScriptDispatchHost {
    /// Host callback failure.
    type Error;

    /// Apply the recovered pre-frame object-state processor and refresh activity flags.
    fn prepare_script_state(
        &mut self,
        state: &mut ScriptState,
        runtime: &mut ScriptRuntime,
        dispatch: &mut ScriptDispatchState,
    ) -> Result<(), Self::Error>;

    /// Return current bridge, travel, and contact activity for CE-D1.
    fn environment_activity(&self) -> ScriptEnvironmentActivity;

    /// Return current hour, day, and month for CA/CB.
    fn clock(&self) -> ScriptClock;

    /// Return current UI gates for an A8 sequence request.
    fn sequence_context(&self) -> SequenceRequestContext;

    /// Resolve C1's two dynamic navigation operands and archetype owner.
    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext>;

    /// Resolve descriptor and interface gates for one C2 operation.
    fn aboard_context(
        &mut self,
        related: commander_blood_formats::script::ScriptObjectId,
    ) -> Result<ScriptAboardRecordContext, Self::Error>;

    /// Resolve descriptor and interface gates for one CD transfer.
    fn transfer_context(
        &mut self,
        item: commander_blood_formats::script::ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error>;

    /// Return the active BAS selector root used when committing a chosen concept.
    fn selector_root(&self) -> Option<ScriptCodeOffset>;

    /// Run the recovered presentation/action scan after COD traversal.
    fn scan_presentation(&mut self, context: ScriptPostScanContext<'_>) -> Result<(), Self::Error>;
}

/// Failure from a translated leaf handler or an explicit host-dependent boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptDispatchError<HostError> {
    /// A host callback failed.
    Host(HostError),
    /// COD control-flow state was inconsistent.
    Runtime(ScriptRuntimeError),
    /// A shared VAR operation failed.
    State(ScriptStateOperationError),
    /// A procedure identity was invalid.
    Procedure(ScriptProcedureStateError),
    /// A direct record, pair, or transfer operation failed.
    Record(ScriptRecordError),
    /// A C1-C9 action-record operation failed.
    ActionRecord(ScriptRecordStateError),
    /// Typed and serialized VAR record state diverged.
    ProfileRecord(ScriptProfileRecordStateError),
    /// A selected concept could not enter its BAS body.
    Selection(ScriptSelectionError),
    /// A6 could not bind or execute its authored VAR line record.
    Text(TextInstructionExecutionError),
}

impl<HostError: fmt::Debug> fmt::Display for ScriptDispatchError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HostError: fmt::Debug> std::error::Error for ScriptDispatchError<HostError> {}

/// Execute one complete frame of a loaded profile through exhaustive typed dispatch.
pub fn execute_loaded_script_frame<Host: ScriptDispatchHost>(
    profile: &mut LoadedScriptProfile,
    execution_enabled: bool,
    dispatch: &mut ScriptDispatchState,
    host: &mut Host,
) -> Result<ScriptFrameOutcome, ScriptFrameError<ScriptDispatchError<Host::Error>>> {
    let LoadedScriptExecutionParts {
        code,
        instructions,
        dialogue,
        state,
        dictionary,
        directory,
        builtins,
        procedures,
        runtime,
        selector_state,
        sequence_slots,
        record_state,
    } = profile.execution_parts();
    let mut dispatcher = Dispatcher {
        instructions,
        dialogue,
        state,
        dictionary,
        directory,
        builtins,
        procedures,
        selector: selector_state,
        sequence_slots,
        records: record_state,
        dispatch,
        host,
    };
    execute_decoded_script_frame(
        code,
        instructions,
        execution_enabled,
        runtime,
        &mut dispatcher,
    )
}

struct Dispatcher<'a, Host> {
    instructions: &'a [DecodedScriptInstruction],
    dialogue: &'a ScriptBas,
    state: &'a mut ScriptState,
    dictionary: &'a ScriptDictionary,
    directory: &'a ScriptDirectory,
    builtins: ScriptProfileBuiltins,
    procedures: &'a mut super::ScriptProcedureStates,
    selector: &'a mut ScriptSelectorState,
    sequence_slots: &'a mut super::ScriptSequenceSlots,
    records: &'a mut ScriptProfileRecordState,
    dispatch: &'a mut ScriptDispatchState,
    host: &'a mut Host,
}

impl<Host: ScriptDispatchHost> DecodedScriptFrameHost for Dispatcher<'_, Host> {
    type Error = ScriptDispatchError<Host::Error>;

    fn prepare_script_state(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error> {
        self.host
            .prepare_script_state(self.state, runtime, self.dispatch)
            .map_err(ScriptDispatchError::Host)?;
        self.records
            .refresh_from_var(
                self.instructions,
                self.state,
                self.dictionary,
                self.builtins,
            )
            .map_err(ScriptDispatchError::ProfileRecord)
    }

    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        instruction: &DecodedScriptInstruction,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, Self::Error> {
        let mut refresh_from_var = false;
        let mut commit_to_var = false;
        let control = match instruction {
            DecodedScriptInstruction::Control(instruction) => runtime
                .apply_instruction(instruction, &mut self.dispatch.random)
                .map_err(ScriptDispatchError::Runtime)?,
            DecodedScriptInstruction::Text(text) => {
                let instruction_state = self
                    .dispatch
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
                .map_err(ScriptDispatchError::Text)?;
                return Ok(step_with_flow(token.end_offset(), execution.flow));
            }
            DecodedScriptInstruction::TopicOffer(offer) => {
                offer_topic_if_presentation_active(
                    *offer,
                    &mut self.dispatch.sequence_presentation,
                );
                ScriptControl::Continue
            }
            DecodedScriptInstruction::SequenceRequest(request) => {
                load_sequence_request(
                    request,
                    self.host.sequence_context(),
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.sequence_presentation,
                );
                ScriptControl::Continue
            }
            DecodedScriptInstruction::ProcedureGate(gate) => {
                evaluate_procedure_gate(*gate, self.procedures, runtime)
                    .map_err(ScriptDispatchError::Procedure)?
            }
            DecodedScriptInstruction::ProcedureActivation(activation) => {
                apply_procedure_activation(*activation, self.procedures)
                    .map_err(ScriptDispatchError::Procedure)?;
                ScriptControl::Continue
            }
            DecodedScriptInstruction::SharedBit(operation) => {
                refresh_from_var = !runtime.query_mode();
                apply_shared_bit_operation(*operation, self.state, runtime)
                    .map_err(ScriptDispatchError::State)?
            }
            DecodedScriptInstruction::SharedState(operation) => {
                refresh_from_var = !runtime.query_mode();
                apply_shared_state_operation(*operation, self.state, runtime)
                    .map_err(ScriptDispatchError::State)?
            }
            DecodedScriptInstruction::DirectRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_direct_record_operation(
                    *operation,
                    &mut self.records.record_fields,
                    &mut self.records.record_runtime,
                    runtime,
                )
                .map_err(ScriptDispatchError::Record)?
            }
            DecodedScriptInstruction::BitFlag(operation) => {
                refresh_from_var = !runtime.query_mode();
                apply_bit_flag_operation(*operation, self.state, runtime)
                    .map_err(ScriptDispatchError::State)?
            }
            DecodedScriptInstruction::RecordPair(operation) => {
                refresh_from_var = !runtime.query_mode();
                apply_record_pair_operation(
                    *operation,
                    self.state,
                    &mut self.dispatch.record_pair_reference,
                    runtime,
                )
                .map_err(ScriptDispatchError::Record)?
            }
            DecodedScriptInstruction::RecordState(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_record_state_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    self.host.navigation_context(),
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::AboardRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                let context = self
                    .host
                    .aboard_context(operation.related)
                    .map_err(ScriptDispatchError::Host)?;
                apply_aboard_record_operation(
                    *operation,
                    self.state,
                    &self.records.action_records,
                    &mut self.records.record_fields,
                    self.records.record_runtime.aboard_objects_mut(),
                    context,
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.aboard_presentation,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::PresentationQueue(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_presentation_queue_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::ActorRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_actor_record_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::WorldStateRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_world_state_record_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::TravelRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_travel_record_operation(*operation, &mut self.records.action_records, runtime)
                    .map_err(ScriptDispatchError::ActionRecord)?
                    .control
            }
            DecodedScriptInstruction::ActiveObjectRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_active_object_record_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::OpaqueMarkerRecord(operation) => {
                commit_to_var = !runtime.query_mode();
                apply_opaque_marker_record_operation(
                    *operation,
                    &mut self.records.action_records,
                    runtime,
                )
                .map_err(ScriptDispatchError::ActionRecord)?
                .control
            }
            DecodedScriptInstruction::RecordClear(operation) => {
                commit_to_var = true;
                apply_record_clear_operation(
                    *operation,
                    self.state,
                    &mut self.records.action_records,
                    &mut self.dispatch.record_clear_presentation,
                )
                .map_err(ScriptDispatchError::ActionRecord)?;
                ScriptControl::Continue
            }
            DecodedScriptInstruction::HourGuard(guard) => self
                .host
                .clock()
                .evaluate_hour_guard(*guard, runtime)
                .map_err(ScriptDispatchError::Runtime)?,
            DecodedScriptInstruction::DateGuard(guard) => self
                .host
                .clock()
                .evaluate_date_guard(*guard, runtime)
                .map_err(ScriptDispatchError::Runtime)?,
            DecodedScriptInstruction::SequenceSlotAssignment(assignment) => {
                self.sequence_slots.assign(assignment.clone());
                ScriptControl::Continue
            }
            DecodedScriptInstruction::Transfer(transfer) => {
                commit_to_var = !runtime.query_mode();
                let context = self
                    .host
                    .transfer_context(transfer.item)
                    .map_err(ScriptDispatchError::Host)?;
                apply_transfer(
                    *transfer,
                    self.state,
                    &self.records.transfer_records,
                    &mut self.records.record_fields,
                    &mut self.records.record_runtime,
                    context,
                    &mut self.dispatch.text_presentation.request_flags,
                    &mut self.dispatch.transfer_presentation,
                    runtime,
                )
                .map_err(ScriptDispatchError::Record)?
                .control
            }
            DecodedScriptInstruction::Environment(instruction) => self
                .host
                .environment_activity()
                .apply(*instruction, runtime)
                .map_err(ScriptDispatchError::Runtime)?,
            DecodedScriptInstruction::ProfileRequest(request) => {
                self.dispatch.profile_request.schedule(*request);
                ScriptControl::Continue
            }
        };

        if refresh_from_var {
            self.records
                .refresh_from_var(
                    self.instructions,
                    self.state,
                    self.dictionary,
                    self.builtins,
                )
                .map_err(ScriptDispatchError::ProfileRecord)?;
        } else if commit_to_var {
            self.records
                .commit_to_var(self.state, self.directory, self.dictionary)
                .map_err(ScriptDispatchError::ProfileRecord)?;
        }
        Ok(step_from_control(token, control, runtime))
    }

    fn commit_selected_concept(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error> {
        commit_selected_concept(
            runtime,
            self.dictionary,
            self.dialogue,
            self.host.selector_root(),
            self.selector,
        )
        .map_err(ScriptDispatchError::Selection)?;
        Ok(())
    }

    fn scan_presentation(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error> {
        self.host
            .scan_presentation(ScriptPostScanContext {
                instructions: self.instructions,
                dialogue: self.dialogue,
                state: self.state,
                dictionary: self.dictionary,
                directory: self.directory,
                records: self.records,
                selector: self.selector,
                runtime,
                dispatch: self.dispatch,
                builtins: self.builtins,
            })
            .map_err(ScriptDispatchError::Host)?;
        self.records
            .refresh_relationships_from_var(
                self.instructions,
                self.state,
                self.dictionary,
                self.builtins,
            )
            .map_err(ScriptDispatchError::ProfileRecord)?;
        self.records
            .commit_to_var(self.state, self.directory, self.dictionary)
            .map_err(ScriptDispatchError::ProfileRecord)
    }
}

fn step_from_control(
    token: &ScriptToken,
    control: ScriptControl,
    runtime: &mut ScriptRuntime,
) -> ScriptFrameStep {
    let next = match control {
        ScriptControl::Continue => token.end_offset(),
        ScriptControl::Jump(target) => target,
    };
    if runtime.take_yield_request() {
        ScriptFrameStep::continue_after_presentation(next)
    } else {
        ScriptFrameStep::continue_at(next)
    }
}

const fn step_with_flow(next: ScriptCodeOffset, flow: ScriptFrameFlow) -> ScriptFrameStep {
    match flow {
        ScriptFrameFlow::Continue => ScriptFrameStep::continue_at(next),
        ScriptFrameFlow::ContinueAfterPresentation => {
            ScriptFrameStep::continue_after_presentation(next)
        }
        ScriptFrameFlow::SaveResumeCursor => ScriptFrameStep::save_resume_cursor(next),
    }
}

/// Return the pending profile request without consuming its main-loop latch.
pub const fn pending_profile_request(
    dispatch: &ScriptDispatchState,
) -> PendingScriptProfileRequest {
    dispatch.profile_request.pending()
}

/// Return the selected concept when a host needs to prepare BAS presentation state.
pub fn selected_concept(runtime: &ScriptRuntime) -> Option<ScriptWordId> {
    runtime.selected_concept()
}

/// Convenience predicate for disabled-frame outcomes used by main-loop coordinators.
pub const fn frame_execution_was_disabled(outcome: ScriptFrameOutcome) -> bool {
    matches!(outcome.end, ScriptFrameEnd::ExecutionDisabled)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use crate::assets::OriginalResourceStore;

    use super::super::{
        OriginalResourceCache, OriginalResourceCatalog, OriginalScriptProfileCatalog,
        ScriptProfileId, ScriptProfileManager,
    };
    use super::*;

    struct TraversalHost {
        builtins: ScriptProfileBuiltins,
        scans: usize,
    }

    impl ScriptDispatchHost for TraversalHost {
        type Error = Infallible;

        fn prepare_script_state(
            &mut self,
            _state: &mut ScriptState,
            _runtime: &mut ScriptRuntime,
            _dispatch: &mut ScriptDispatchState,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn environment_activity(&self) -> ScriptEnvironmentActivity {
            ScriptEnvironmentActivity {
                bridge_active: true,
                travel_active: true,
                contact_active: true,
            }
        }

        fn clock(&self) -> ScriptClock {
            ScriptClock {
                hour: 12,
                day: 2,
                month: 1,
            }
        }

        fn sequence_context(&self) -> SequenceRequestContext {
            SequenceRequestContext {
                ship_active: true,
                scene_gate_active: true,
            }
        }

        fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
            Some(ScriptRecordStateNavigationContext {
                primary_object: self.builtins.player?,
                secondary_object: self.builtins.player?,
                arche: self.builtins.archetype?,
            })
        }

        fn aboard_context(
            &mut self,
            _related: commander_blood_formats::script::ScriptObjectId,
        ) -> Result<ScriptAboardRecordContext, Self::Error> {
            Ok(ScriptAboardRecordContext::default())
        }

        fn transfer_context(
            &mut self,
            _item: commander_blood_formats::script::ScriptObjectId,
        ) -> Result<ScriptTransferContext, Self::Error> {
            Ok(ScriptTransferContext::default())
        }

        fn selector_root(&self) -> Option<ScriptCodeOffset> {
            None
        }

        fn scan_presentation(
            &mut self,
            _context: ScriptPostScanContext<'_>,
        ) -> Result<(), Self::Error> {
            self.scans += 1;
            Ok(())
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
    fn every_shipped_profile_enters_exhaustive_dispatch_with_coherent_var_state() {
        let Some(root) = original_data_root() else {
            return;
        };
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let profile = manager.current_mut().unwrap();
            let mut dispatch = ScriptDispatchState::default();
            let mut host = TraversalHost {
                builtins: profile.builtins(),
                scans: 0,
            };

            let outcome = execute_loaded_script_frame(profile, true, &mut dispatch, &mut host)
                .unwrap_or_else(|error| {
                    panic!(
                        "profile {} dispatch failed: {error:?}",
                        profile_id.value() + 1
                    )
                });

            assert_ne!(outcome.end, ScriptFrameEnd::ExecutionDisabled);
            assert_eq!(host.scans, 1);
            profile.synchronized_state().unwrap();
        }
    }
}
