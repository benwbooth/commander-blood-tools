//! Exhaustive production dispatch for pre-decoded BloodScript COD instructions.

use std::collections::BTreeMap;
use std::fmt;

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
use super::sequel_growth::{
    SequelConflictState, SequelGrowthError, SequelSettlementContext, SequelSettlementState,
    SequelSimulationContext, apply_sequel_conflict, apply_sequel_growth, apply_sequel_settlement,
};
use super::sequel_presentation::{SequelPresentationControl, assign_presentation_sequence};
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
    /// Shared range-search override used by sequel settlement and conflict.
    pub sequel_settlement: SequelSettlementState,
    /// Last attack rate published by D4, including query/suppressed updates.
    pub sequel_conflict: SequelConflictState,
    /// Sequel CC/D7 controls shared with the panel and retained across profiles.
    pub sequel_presentation: SequelPresentationControl,
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

    /// Reset profile state, preserving the session PRNG and sequel panel controls.
    pub fn reset_for_profile_change(&mut self) {
        let random = self.random;
        let sequel_presentation = self.sequel_presentation;
        *self = Self::default();
        self.random = random;
        self.sequel_presentation = sequel_presentation;
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
    /// Dialogue resource, decoded only when a scan enters its structures.
    pub dialogue: &'a dyn super::ScriptDialogueSource,
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

    /// Supply the sequel's additional named-object bindings for settlement/conflict.
    fn sequel_settlement_context(&self) -> Option<SequelSettlementContext> {
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
    /// An object-backed inventory choice could not bind its typed transfer state.
    SequelInventory(super::SequelInventoryError),
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
    dialogue: &'a dyn super::ScriptDialogueSource,
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
                    Some(super::TextInventoryContext {
                        instruction: token.source_offset(),
                        roster: self.records.record_runtime.aboard_objects(),
                    }),
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
            DecodedScriptInstruction::SequelSettlement(operation) => {
                let context = self
                    .host
                    .sequel_settlement_context()
                    .ok_or(ScriptDispatchError::MissingSequelSimulationContext)?;
                refresh_from_var = true;
                apply_sequel_settlement(
                    *operation,
                    context,
                    &mut self.dispatch.sequel_settlement,
                    self.state,
                )
                .map_err(ScriptDispatchError::SequelGrowth)?
            }
            DecodedScriptInstruction::SequelConflict(operation) => {
                let context = self
                    .host
                    .sequel_settlement_context()
                    .ok_or(ScriptDispatchError::MissingSequelSimulationContext)?;
                refresh_from_var = true;
                apply_sequel_conflict(
                    *operation,
                    context,
                    &mut self.dispatch.sequel_conflict,
                    &mut self.dispatch.sequel_settlement,
                    self.state,
                    runtime,
                )
                .map_err(ScriptDispatchError::SequelGrowth)?
            }
            DecodedScriptInstruction::SequelEnding => {
                self.dispatch.sequel_presentation.begin_ending();
                ScriptControl::Continue
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
                assign_presentation_sequence(
                    assignment.clone(),
                    self.state.dialect(),
                    self.sequence_slots,
                    &mut self.dispatch.sequel_presentation,
                );
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
        if self.selector.inventory().descriptor_lookup().is_some() {
            return Err(ScriptDispatchError::SequelInventory(
                super::SequelInventoryError::DescriptorPending,
            ));
        }
        if self.selector.inventory().selected().is_some() {
            self.selector
                .inventory_mut()
                .commit(
                    self.state,
                    self.records.record_runtime.aboard_objects_mut(),
                    &mut self.records.record_fields,
                    runtime,
                    &mut self.dispatch.text_instructions,
                    &self.dispatch.text_presentation,
                    self.host.environment_activity().bridge_active,
                )
                .map_err(ScriptDispatchError::SequelInventory)?;
            self.selector.replace_presentation_words([]);
            if self.selector.inventory().descriptor_lookup().is_some() {
                return Err(ScriptDispatchError::SequelInventory(
                    super::SequelInventoryError::DescriptorPending,
                ));
            }
            return Ok(());
        }
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
        settlement: Option<SequelSettlementContext>,
    }

    #[test]
    fn sequel_inventory_dispatch_bypasses_dictionary_history_and_preserves_roster_holes() {
        use commander_blood_formats::bas::decode_script_bas;
        use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
        use commander_blood_formats::instruction::{
            ScriptRecordValue, decode_complete_script_instruction,
        };
        use commander_blood_formats::script::{
            ScriptObjectKind, decode_script_dictionary, decode_script_directory,
            decode_script_state_for_dialect,
        };

        let mut directory_bytes = Vec::new();
        let mut state_bytes = Vec::new();
        for (name, kind) in [
            (b"blood".as_slice(), ScriptObjectKind::Player),
            (b"item", ScriptObjectKind::InventoryItem),
            (b"actor", ScriptObjectKind::Actor),
        ] {
            let mut entry = [0; 20];
            entry[..name.len()].copy_from_slice(name);
            entry[16..18].copy_from_slice(&(state_bytes.len() as u16).to_le_bytes());
            entry[18..20].copy_from_slice(&1u16.to_le_bytes());
            directory_bytes.extend(entry);
            let mut record = vec![0; kind.record_size_for_dialect(ScriptDialect::BigBugBang)];
            record[..2].copy_from_slice(&kind.mask().to_le_bytes());
            if kind == ScriptObjectKind::InventoryItem {
                record[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
            }
            state_bytes.extend(record);
        }
        directory_bytes.extend([0; 20]);
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let mut state =
            decode_script_state_for_dialect(&state_bytes, &directory, ScriptDialect::BigBugBang)
                .unwrap();
        let player = directory.find_active_object(b"blood").unwrap();
        let item = directory.find_active_object(b"item").unwrap();
        let actor = directory.find_active_object(b"actor").unwrap();
        let action_offset = super::super::script_field_offset(
            ScriptObjectKind::Actor,
            super::super::ScriptFieldSelector::ACTION,
        )
        .unwrap();
        let action = state.object_word(actor, action_offset / 2).unwrap();
        state.set_word(action, 196);
        let dictionary = decode_script_dictionary(b"PREVIOUS\0").unwrap();
        let previous = dictionary.words().next().unwrap().0;
        let dialogue = decode_script_bas(&[0xff], &dictionary).unwrap();
        let code = decode_script_code_for_dialect(
            &[
                0xa6, 58, 0, 0, 0x30, 0x80, 14, 0, 0xff, 0xff, 0xfe, 0xff, 0, 0, 0xff,
            ],
            ScriptDialect::BigBugBang,
        )
        .unwrap();
        let instructions = code
            .tokens()
            .iter()
            .map(|token| {
                decode_complete_script_instruction(token, &state, &directory, &dictionary).unwrap()
            })
            .collect::<Vec<_>>();
        let builtins = ScriptProfileBuiltins {
            player: Some(player),
            ..Default::default()
        };
        let mut records =
            ScriptProfileRecordState::recover(&[], &state, &dictionary, builtins).unwrap();
        let mut slots = [None; 16];
        slots[3] = Some(item);
        slots[12] = Some(item);
        *records.record_runtime.aboard_objects_mut() =
            super::super::AboardObjectRoster::from_test_slots(slots);
        let mut runtime = ScriptRuntime::default();
        let mut selector = ScriptSelectorState::default();
        selector.history_mut().push(previous);
        selector.replace_presentation_words([previous]);
        let history = selector.history().clone();
        let mut dispatch = ScriptDispatchState::default();
        let line = super::super::SequelInventoryLine {
            instruction: ScriptCodeOffset::new(0),
            recipient: actor,
        };
        let mut host = TraversalHost {
            builtins,
            scans: 0,
            simulation: None,
            settlement: None,
        };
        let mut procedures = super::super::ScriptProcedureStates::default();
        let mut sequence_slots = super::super::ScriptSequenceSlots::default();
        let mut dispatcher = Dispatcher {
            code: &code,
            instructions: &instructions,
            dialogue: &dialogue,
            state: &mut state,
            dictionary: &dictionary,
            directory: &directory,
            builtins,
            procedures: &mut procedures,
            selector: &mut selector,
            sequence_slots: &mut sequence_slots,
            records: &mut records,
            dispatch: &mut dispatch,
            host: &mut host,
        };
        let frame =
            execute_decoded_script_frame(&code, &instructions, true, &mut runtime, &mut dispatcher)
                .unwrap();
        assert_eq!(frame.executed_instructions, 1);
        assert_eq!(dispatcher.selector.inventory().choices(), &[item, item]);
        assert_eq!(
            dispatcher.records.record_runtime.aboard_objects().slots(),
            &slots
        );
        assert!(runtime.selector_resume_active());
        dispatcher.selector.inventory_mut().select(item).unwrap();
        dispatcher.commit_selected_concept(&mut runtime).unwrap();
        dispatcher.prepare_script_state(&mut runtime).unwrap();
        assert_eq!(selector.history(), &history);
        assert!(selector.pending_presentation_words().is_empty());
        assert_eq!(records.record_runtime.aboard_objects().slots()[3], None);
        assert_eq!(
            records.record_runtime.aboard_objects().slots()[12],
            Some(item)
        );
        let holder = state.object_word(item, 10).unwrap();
        assert_eq!(
            records.record_fields.value(holder),
            Some(ScriptRecordValue::Object(actor))
        );
        let mut synchronized = state.clone();
        records
            .commit_to_var(&mut synchronized, &directory, &dictionary)
            .unwrap();
        assert_eq!(synchronized, state);
        assert!(dispatch.text_instructions[&line.instruction].is_active());
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

        fn sequel_settlement_context(&self) -> Option<SequelSettlementContext> {
            self.settlement
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
                    settlement: None,
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
    fn sequel_panel_dispatch_preserves_query_and_session_controls() {
        use commander_blood_formats::bas::decode_script_bas;
        use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
        use commander_blood_formats::instruction::{
            ScriptSequenceSlot, decode_complete_script_instruction,
        };
        use commander_blood_formats::script::{
            ScriptObjectKind, decode_script_dictionary, decode_script_directory,
            decode_script_state_for_dialect,
        };
        const DIRECTORY_ENTRY_BYTES: usize = 20;
        const DIRECTORY_KIND: usize = 18;
        const GUARD_TARGET: usize = 30;
        let mut directory_bytes = [0; DIRECTORY_ENTRY_BYTES * 2];
        directory_bytes[..5].copy_from_slice(b"blood");
        directory_bytes[DIRECTORY_KIND] = 1;
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let dialogue = decode_script_bas(&[u8::MAX], &dictionary).unwrap();
        let mut var = vec![0; ScriptObjectKind::Player.record_size()];
        var[..2].copy_from_slice(&ScriptObjectKind::Player.mask().to_le_bytes());
        let builtins = ScriptProfileBuiltins {
            player: directory.find_active_object(b"blood"),
            ..ScriptProfileBuiltins::default()
        };
        for query in [false, true] {
            let mut state =
                decode_script_state_for_dialect(&var, &directory, ScriptDialect::BigBugBang)
                    .unwrap();
            let code = decode_script_code_for_dialect(
                b"\xcc\x03end\0\0\xd7\xff",
                ScriptDialect::BigBugBang,
            )
            .unwrap();
            let instructions: Vec<_> = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() != u8::MAX)
                .map(|token| {
                    decode_complete_script_instruction(token, &state, &directory, &dictionary)
                        .unwrap()
                })
                .collect();
            assert_eq!(instructions.len(), 2);
            let mut records =
                ScriptProfileRecordState::recover(&instructions, &state, &dictionary, builtins)
                    .unwrap();
            let mut runtime = ScriptRuntime::new();
            if query {
                runtime.begin_root_guard(ScriptCodeOffset::new(GUARD_TARGET));
            }
            let mut procedures = super::super::ScriptProcedureStates::default();
            let mut selector = ScriptSelectorState::default();
            let mut slots = super::super::ScriptSequenceSlots::default();
            let mut dispatch = ScriptDispatchState::default();
            let mut host = TraversalHost {
                builtins,
                scans: 0,
                simulation: None,
                settlement: None,
            };
            for (token, instruction) in code.tokens().iter().zip(&instructions) {
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
                .execute_instruction(token, instruction, &mut runtime);
                assert_eq!(result, Ok(ScriptFrameStep::continue_at(token.end_offset())));
            }
            let slot = ScriptSequenceSlot::decode(3).unwrap();
            assert_eq!(slots.name(slot).unwrap().as_bytes(), b"end");
            assert_eq!(
                dispatch.sequel_presentation,
                SequelPresentationControl {
                    ending_active: true,
                    requested_choice: Some(slot)
                }
            );
            assert!(
                !dispatch.sequence_presentation.finale_requested,
                "D7 is not A8 fin.*"
            );
            assert_eq!(state.encode(), var);
            assert_eq!(runtime.query_mode(), query);
            assert_eq!(
                runtime.current_guard_target(),
                query.then_some(ScriptCodeOffset::new(GUARD_TARGET))
            );
            let expected = dispatch.sequel_presentation;
            dispatch.reset_for_profile_change();
            assert_eq!(dispatch.sequel_presentation, expected);
        }
    }

    #[test]
    fn sequel_simulation_dispatch_matches_native_cases_and_requires_bindings() {
        const FAILURE_TARGET: usize = 384;
        use commander_blood_formats::bas::decode_script_bas;
        use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
        use commander_blood_formats::instruction::decode_complete_script_instruction;
        use commander_blood_formats::script::{
            decode_script_dictionary, decode_script_directory, decode_script_state_for_dialect,
        };
        #[derive(serde::Deserialize)]
        struct Oracle {
            name: String,
            token: Vec<u8>,
            directory: Vec<u8>,
            state_before: Vec<u8>,
            state_after: Vec<u8>,
            countdown: u16,
            query_mode: u8,
            range_override_before: u8,
            range_override_after: u8,
            attack_rate_before: Option<u16>,
            attack_rate_after: Option<u16>,
            query_mode_after: Option<u8>,
            #[serde(default)]
            branch_taken: bool,
            #[serde(default)]
            divide_error: bool,
        }
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let dialogue = decode_script_bas(&[u8::MAX], &dictionary).unwrap();
        let mut count = 0;
        for line in [
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_settlement.jsonl"),
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_conflict.jsonl"),
        ]
        .into_iter()
        .flat_map(str::lines)
        {
            let vector: Oracle = serde_json::from_str(line).unwrap();
            let directory = decode_script_directory(&vector.directory).unwrap();
            let builtins = ScriptProfileBuiltins {
                player: directory.find_active_object(b"blood"),
                ..ScriptProfileBuiltins::default()
            };
            let context = SequelSettlementContext {
                simulation: SequelSimulationContext {
                    countdown: vector.countdown,
                    excluded_location: directory.find_active_object(b"Trashlando").unwrap(),
                },
                arche: directory.find_active_object(b"arche").unwrap(),
                honk: directory.find_active_object(b"Honk").unwrap(),
                excluded_destination: directory.find_active_object(b"Arche").unwrap(),
            };
            let mut code_bytes = vector.token.clone();
            code_bytes.push(u8::MAX);
            let code =
                decode_script_code_for_dialect(&code_bytes, ScriptDialect::BigBugBang).unwrap();
            let token = &code.tokens()[0];
            for bound in [false, true] {
                let mut state = decode_script_state_for_dialect(
                    &vector.state_before,
                    &directory,
                    ScriptDialect::BigBugBang,
                )
                .unwrap();
                let instructions =
                    [
                        decode_complete_script_instruction(token, &state, &directory, &dictionary)
                            .unwrap(),
                    ];
                let mut records =
                    ScriptProfileRecordState::recover(&instructions, &state, &dictionary, builtins)
                        .unwrap();
                let mut runtime = ScriptRuntime::new();
                if vector.query_mode != 0 {
                    runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
                }
                let mut procedures = super::super::ScriptProcedureStates::default();
                let mut selector = ScriptSelectorState::default();
                let mut slots = super::super::ScriptSequenceSlots::default();
                let mut dispatch = ScriptDispatchState::default();
                dispatch.sequel_settlement.range_override_active =
                    vector.range_override_before != 0;
                dispatch.sequel_conflict.attack_rate =
                    vector.attack_rate_before.unwrap_or_default();
                let mut host = TraversalHost {
                    builtins,
                    scans: 0,
                    simulation: Some(context.simulation),
                    settlement: bound.then_some(context),
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
                if bound {
                    if vector.divide_error {
                        assert!(
                            matches!(
                                result,
                                Err(ScriptDispatchError::SequelGrowth(
                                    SequelGrowthError::ConflictOverflow { .. }
                                ))
                            ),
                            "{}: {result:?}",
                            vector.name
                        );
                    } else {
                        assert_eq!(
                            result,
                            Ok(ScriptFrameStep::continue_at(if vector.branch_taken {
                                ScriptCodeOffset::new(FAILURE_TARGET)
                            } else {
                                token.end_offset()
                            })),
                            "{}",
                            vector.name
                        );
                    }
                    assert_eq!(state.encode(), vector.state_after, "{}", vector.name);
                    assert_eq!(
                        u8::from(dispatch.sequel_settlement.range_override_active),
                        vector.range_override_after,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        dispatch.sequel_conflict.attack_rate,
                        vector.attack_rate_after.unwrap_or_default(),
                        "{}",
                        vector.name
                    );
                } else {
                    assert_eq!(
                        result,
                        Err(ScriptDispatchError::MissingSequelSimulationContext),
                        "{}",
                        vector.name
                    );
                    assert_eq!(state.encode(), vector.state_before, "{}", vector.name);
                    assert_eq!(
                        u8::from(dispatch.sequel_settlement.range_override_active),
                        vector.range_override_before
                    );
                    assert_eq!(
                        dispatch.sequel_conflict.attack_rate,
                        vector.attack_rate_before.unwrap_or_default()
                    );
                }
                assert_eq!(
                    runtime.query_mode(),
                    if bound {
                        vector.query_mode_after.unwrap_or(vector.query_mode) != 0
                    } else {
                        vector.query_mode != 0
                    }
                );
            }
            count += 1;
        }
        assert_eq!(count, 224);
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
                settlement: None,
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
