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
use super::sequel_growth::{SequelGrowthError, SequelSimulationContext, apply_sequel_growth};
use super::sequence::{
    PresentationResourceLine, SequencePresentationState, SequenceRequestContext,
    load_sequence_request, offer_topic_if_presentation_active,
};
use super::state::{
    ScriptStateOperationError, apply_bit_flag_operation, apply_multiply_divide_operation,
    apply_shared_bit_operation, apply_shared_state_operation,
};
use super::{
    PendingScriptProfileRequest, ScriptControl, ScriptFrameEnd, ScriptPresentationScanState,
    ScriptProfileRequestSlot, ScriptRuntime, ScriptRuntimeError, TextInstructionExecutionError,
    TextInstructionState, TextPresentationState, execute_text_instruction,
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
    /// Last write to the single shared `vm_active_line` during this VM frame.
    pending_active_line_write: Option<u16>,
}

impl ScriptDispatchState {
    fn begin_frame(&mut self) {
        self.pending_active_line_write = None;
    }

    pub(crate) fn record_active_line_write(&mut self, line: u16) {
        self.pending_active_line_write = Some(line);
    }

    pub(crate) const fn pending_active_line_write(&self) -> Option<u16> {
        self.pending_active_line_write
    }

    pub(crate) fn take_active_line_write(&mut self) -> Option<u16> {
        self.pending_active_line_write.take()
    }

    /// Reset state owned by one loaded profile while preserving the session PRNG.
    pub fn reset_for_profile_change(&mut self) {
        let random = self.random;
        *self = Self::default();
        self.random = random;
    }

    /// Publish the canonical post-scan globals to every translated opcode owner.
    ///
    /// The DOS executable stored these values once. The typed translation keeps
    /// the specialized A6, A8, C2, and CD state separate, so each frame begins by
    /// broadcasting the latest post-scan values before any handler can read them.
    pub(crate) fn import_presentation_scan_state(
        &mut self,
        presentation: &ScriptPresentationScanState,
    ) {
        self.sequence_presentation.presentation_active = presentation.active;
        self.sequence_presentation.presentation_gate_active = presentation.c2_gate_active;
        self.aboard_presentation.presentation_gate_active = presentation.c2_gate_active;
        self.transfer_presentation.presentation_gate_active = presentation.c2_gate_active;
        self.text_presentation.hold_ready = presentation.hold_ready;
        self.text_presentation.dialogue_hold_complete = presentation.dialogue_hold_complete;
    }

    /// Merge opcode writes back into the canonical state before presentation scan.
    ///
    /// A8, C2, and CD only release the shared C2 gate. Since every typed owner is
    /// initialized from the same value at frame start, conjunction reproduces a
    /// write of zero by any handler without inventing a last-writer heuristic.
    pub(crate) fn export_presentation_scan_state(
        &self,
        presentation: &mut ScriptPresentationScanState,
    ) {
        presentation.active = self.sequence_presentation.presentation_active;
        presentation.c2_gate_active = self.sequence_presentation.presentation_gate_active
            && self.aboard_presentation.presentation_gate_active
            && self.transfer_presentation.presentation_gate_active;
        presentation.hold_ready = self.text_presentation.hold_ready;
        presentation.dialogue_hold_complete = self.text_presentation.dialogue_hold_complete;
    }
}

/// Mutable state exposed to the recovered post-frame presentation scan.
pub struct ScriptPostScanContext<'a> {
    /// Losslessly framed COD image used for object-owned text activation.
    pub code: &'a commander_blood_formats::code::ScriptCode,
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

/// Complete flat profile state exposed to the recovered pre-frame processor.
pub struct ScriptPreFrameContext<'a> {
    /// Active VAR image.
    pub state: &'a mut ScriptState,
    /// Shared COD control-flow state.
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
        context: ScriptPreFrameContext<'_>,
    ) -> Result<(), Self::Error>;

    /// Return current bridge, travel, and contact activity for CE-D1.
    fn environment_activity(&self) -> ScriptEnvironmentActivity;

    /// Return current hour, day, and month for CA/CB.
    fn clock(&self) -> ScriptClock;

    /// Supply the sequel's main-loop countdown and bound Trashlando identity.
    ///
    /// Commander hosts have no sequel simulation clock. A sequel instruction
    /// without this context errors rather than running at presentation speed.
    fn sequel_simulation_context(&self) -> Option<SequelSimulationContext> {
        None
    }

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
    /// COD opcode AA emitted signal one, which the native outer wrapper rejects.
    InvalidLegacyYieldSignal,
    /// A shared VAR operation failed.
    State(ScriptStateOperationError),
    /// A sequel instruction was dispatched without its native clock/object bindings.
    MissingSequelSimulationContext,
    /// Sequel actor-state arithmetic or relationship binding failed.
    SequelGrowth(SequelGrowthError),
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
    dispatch.begin_frame();
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
        code,
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
    code: &'a commander_blood_formats::code::ScriptCode,
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
            .prepare_script_state(ScriptPreFrameContext {
                state: self.state,
                runtime,
                dispatch: self.dispatch,
                builtins: self.builtins,
            })
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
            DecodedScriptInstruction::MultiplyDivide(operation) => {
                refresh_from_var = true;
                apply_multiply_divide_operation(*operation, self.state)
                    .map_err(ScriptDispatchError::State)?
            }
            DecodedScriptInstruction::SequelGrowth(operation) => {
                let context = self
                    .host
                    .sequel_simulation_context()
                    .ok_or(ScriptDispatchError::MissingSequelSimulationContext)?;
                refresh_from_var = true;
                apply_sequel_growth(*operation, context, self.state)
                    .map_err(ScriptDispatchError::SequelGrowth)?
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
                let outcome = apply_aboard_record_operation(
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
                .map_err(ScriptDispatchError::ActionRecord)?;
                if outcome.presentation_requested {
                    let line = self
                        .dispatch
                        .aboard_presentation
                        .active_line
                        .expect("C2 reports a request only after selecting an active line");
                    self.dispatch.record_active_line_write(line.number());
                }
                outcome.control
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
                let outcome = apply_transfer(
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
                .map_err(ScriptDispatchError::Record)?;
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
        step_from_control(token.end_offset(), control, runtime)
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
                code: self.code,
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

fn step_from_control<HostError>(
    next: ScriptCodeOffset,
    control: ScriptControl,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptFrameStep, ScriptDispatchError<HostError>> {
    let next = match control {
        ScriptControl::Continue => next,
        ScriptControl::Jump(target) => target,
    };
    if runtime.take_yield_request() {
        Err(ScriptDispatchError::InvalidLegacyYieldSignal)
    } else {
        Ok(ScriptFrameStep::continue_at(next))
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
        simulation: Option<SequelSimulationContext>,
    }

    impl ScriptDispatchHost for TraversalHost {
        type Error = Infallible;

        fn prepare_script_state(
            &mut self,
            _context: ScriptPreFrameContext<'_>,
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

        fn sequel_simulation_context(&self) -> Option<SequelSimulationContext> {
            self.simulation
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
    fn profile_reset_preserves_session_random_state_only() {
        let mut dispatch = ScriptDispatchState::default();
        dispatch.random.seed = 4_660;
        dispatch.random.counter = 9;
        dispatch.text_presentation.subtitle_display_active = true;
        dispatch.sequence_presentation.finale_requested = true;

        dispatch.reset_for_profile_change();

        assert_eq!(dispatch.random.seed, 4_660);
        assert_eq!(dispatch.random.counter, 9);
        assert_eq!(dispatch.text_presentation, TextPresentationState::default());
        assert_eq!(
            dispatch.sequence_presentation,
            SequencePresentationState::default()
        );
    }

    #[test]
    fn presentation_scan_globals_are_broadcast_to_every_opcode_owner() {
        let mut dispatch = ScriptDispatchState::default();
        let presentation = ScriptPresentationScanState {
            active: true,
            c2_gate_active: true,
            hold_ready: true,
            dialogue_hold_complete: true,
            ..ScriptPresentationScanState::default()
        };

        dispatch.import_presentation_scan_state(&presentation);

        assert!(dispatch.sequence_presentation.presentation_active);
        assert!(dispatch.sequence_presentation.presentation_gate_active);
        assert!(dispatch.aboard_presentation.presentation_gate_active);
        assert!(dispatch.transfer_presentation.presentation_gate_active);
        assert!(dispatch.text_presentation.hold_ready);
        assert!(dispatch.text_presentation.dialogue_hold_complete);
    }

    #[test]
    fn any_direct_presentation_handler_can_release_the_shared_c2_gate() {
        let mut dispatch = ScriptDispatchState::default();
        let mut presentation = ScriptPresentationScanState {
            active: true,
            c2_gate_active: true,
            hold_ready: true,
            dialogue_hold_complete: true,
            ..ScriptPresentationScanState::default()
        };
        dispatch.import_presentation_scan_state(&presentation);
        dispatch.transfer_presentation.presentation_gate_active = false;
        dispatch.text_presentation.hold_ready = false;
        dispatch.text_presentation.dialogue_hold_complete = false;

        dispatch.export_presentation_scan_state(&mut presentation);

        assert!(presentation.active);
        assert!(!presentation.c2_gate_active);
        assert!(!presentation.hold_ready);
        assert!(!presentation.dialogue_hold_complete);
    }

    #[test]
    fn legacy_aa_signal_is_rejected_by_the_outer_cod_dispatch() {
        const NEXT_INSTRUCTION_OFFSET: usize = 12;
        let mut runtime = ScriptRuntime::default();
        runtime.request_yield();

        let outcome = step_from_control::<Infallible>(
            ScriptCodeOffset::new(NEXT_INSTRUCTION_OFFSET),
            ScriptControl::Continue,
            &mut runtime,
        );

        assert_eq!(outcome, Err(ScriptDispatchError::InvalidLegacyYieldSignal));
        assert!(!runtime.take_yield_request());
    }

    #[test]
    fn sequel_growth_dispatch_requires_clock_context_and_updates_in_query_mode() {
        use commander_blood_formats::bas::decode_script_bas;
        use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
        use commander_blood_formats::instruction::decode_complete_script_instruction;
        use commander_blood_formats::script::{
            ScriptObjectKind, decode_script_dictionary, decode_script_directory,
            decode_script_state_for_dialect,
        };
        const DIRECTORY_ENTRY_SIZE: usize = 20;
        const DIRECTORY_VALUE_OFFSET: usize = 16;
        const DIRECTORY_KIND_OFFSET: usize = 18;
        const FLAGS_OFFSET: usize = 2;
        const GROUP_OFFSET: usize = 20;
        const QUANTITY_OFFSET: usize = 22;
        const BALANCE_OFFSET: usize = 52;
        const RELIEF_OFFSET: usize = 56;
        const INITIAL_QUANTITY: u16 = 500;
        const UPDATED_QUANTITY: u16 = 524;
        let actor_offset = ScriptObjectKind::Player.record_size();
        let excluded_offset = actor_offset
            + ScriptObjectKind::Actor.record_size_for_dialect(ScriptDialect::BigBugBang);
        let mut bytes = vec![
            0;
            excluded_offset
                + ScriptObjectKind::Location
                    .record_size_for_dialect(ScriptDialect::BigBugBang)
        ];
        let mut directory_bytes = vec![0; DIRECTORY_ENTRY_SIZE * 4];
        for (index, (name, offset, kind)) in [
            ("blood", 0, ScriptObjectKind::Player),
            ("actor", actor_offset, ScriptObjectKind::Actor),
            ("Trashlando", excluded_offset, ScriptObjectKind::Location),
        ]
        .into_iter()
        .enumerate()
        {
            bytes[offset..offset + 2].copy_from_slice(&kind.mask().to_le_bytes());
            bytes[offset + FLAGS_OFFSET] = 5;
            let entry = &mut directory_bytes
                [index * DIRECTORY_ENTRY_SIZE..(index + 1) * DIRECTORY_ENTRY_SIZE];
            entry[..name.len()].copy_from_slice(name.as_bytes());
            entry[DIRECTORY_VALUE_OFFSET..DIRECTORY_KIND_OFFSET]
                .copy_from_slice(&(offset as u16).to_le_bytes());
            entry[DIRECTORY_KIND_OFFSET] = 1;
        }
        for (offset, value) in [
            (GROUP_OFFSET, 1u16),
            (QUANTITY_OFFSET, INITIAL_QUANTITY),
            (BALANCE_OFFSET, 500),
            (RELIEF_OFFSET, 500),
        ] {
            bytes[actor_offset + offset..actor_offset + offset + 2]
                .copy_from_slice(&value.to_le_bytes());
        }
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let dialogue = decode_script_bas(&[u8::MAX], &dictionary).unwrap();
        let code = decode_script_code_for_dialect(
            &[0xD6, 1, 0, 10, 0, u8::MAX],
            ScriptDialect::BigBugBang,
        )
        .unwrap();
        let token = &code.tokens()[0];
        let builtins = ScriptProfileBuiltins {
            player: directory.find_active_object(b"blood"),
            ..ScriptProfileBuiltins::default()
        };
        let excluded_location = directory.find_active_object(b"Trashlando").unwrap();
        for query in [false, true] {
            for countdown in [None, Some(0), Some(1)] {
                let mut state =
                    decode_script_state_for_dialect(&bytes, &directory, ScriptDialect::BigBugBang)
                        .unwrap();
                let instruction =
                    decode_complete_script_instruction(token, &state, &directory, &dictionary)
                        .unwrap();
                let instructions = [instruction];
                let mut records =
                    ScriptProfileRecordState::recover(&instructions, &state, &dictionary, builtins)
                        .unwrap();
                let mut runtime = ScriptRuntime::new();
                if query {
                    runtime.begin_root_guard(token.end_offset());
                }
                let mut procedures = super::super::ScriptProcedureStates::default();
                let mut selector = ScriptSelectorState::default();
                let mut slots = super::super::ScriptSequenceSlots::default();
                let mut dispatch = ScriptDispatchState::default();
                let mut host = TraversalHost {
                    builtins,
                    scans: 0,
                    simulation: countdown.map(|countdown| SequelSimulationContext {
                        countdown,
                        excluded_location,
                    }),
                };
                let result = Dispatcher {
                    code: &code,
                    instructions: &instructions,
                    dialogue: &dialogue,
                    state: &mut state,
                    dictionary: &dictionary,
                    directory: &directory,
                    builtins,
                    procedures: &mut procedures,
                    selector: &mut selector,
                    sequence_slots: &mut slots,
                    records: &mut records,
                    dispatch: &mut dispatch,
                    host: &mut host,
                }
                .execute_instruction(token, &instructions[0], &mut runtime);
                if countdown.is_none() {
                    assert_eq!(
                        result,
                        Err(ScriptDispatchError::MissingSequelSimulationContext)
                    );
                } else {
                    assert!(result.is_ok(), "{result:?}");
                }
                let quantity = state
                    .resolve_word_source_offset((actor_offset + QUANTITY_OFFSET) as u16)
                    .unwrap();
                assert_eq!(
                    state.word(quantity),
                    Some(if countdown == Some(0) {
                        UPDATED_QUANTITY
                    } else {
                        INITIAL_QUANTITY
                    })
                );
                assert_eq!(runtime.query_mode(), query);
            }
        }
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
                simulation: None,
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
