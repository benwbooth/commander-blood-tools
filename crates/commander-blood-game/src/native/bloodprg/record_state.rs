//! Typed C1 action records and navigation-state dispatch.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use commander_blood_formats::instruction::{
    ScriptAboardRecordOperation, ScriptActiveObjectRecordOperation, ScriptActorRecordOperation,
};
use commander_blood_formats::instruction::{
    ScriptOpaqueMarkerRecordOperation, ScriptPresentationQueueOperation,
    ScriptRecordClearOperation, ScriptRecordStateOperand, ScriptRecordStateOperation,
    ScriptRecordValue, ScriptTravelRecordOperation, ScriptWorldStateRecordOperation,
};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference,
    ScriptStateWordTriple,
};

use super::{
    AboardObjectRoster, PresentationRequestFlags, ScriptControl, ScriptFieldSelector,
    ScriptNavigationError, ScriptObjectFlag, ScriptRecordFields, ScriptRuntime, ScriptRuntimeError,
    insert_aboard_object, navigation_distance, navigation_source_objects, object_has_flag,
    object_links_to, script_field_offset,
};

const POST_ACTOR_CLEAR_DEPTH_STEP: u8 = 6;

/// Typed contents of one three-word action slot relevant to the C1 handler.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptActionRecord {
    /// All three words are available for a new action.
    #[default]
    Empty,
    /// C1 navigation action carrying its typed operand.
    Navigation(ScriptRecordStateOperand),
    /// C2 aboard-object request produced by the wider record processor.
    AboardRequest(ScriptObjectId),
    /// C3 presentation queued for later promotion to an active actor record.
    PresentationQueue(ScriptObjectId),
    /// C4 actor-presentation action carrying its related object.
    ActorPresentation(ScriptObjectId),
    /// C5 link to an active world-state object.
    WorldStateLink(ScriptObjectId),
    /// C6 travel relation to a destination object.
    Travel(ScriptObjectId),
    /// C7 relation to an active object.
    ActiveObjectLink(ScriptObjectId),
    /// C8 marker carrying its opaque query word; assignments always store zero.
    OpaqueMarker(u16),
    /// Another native record kind currently owns the slot.
    Occupied,
}

impl ScriptActionRecord {
    /// Serialized record kind written by the C4 actor-presentation operation.
    pub const ACTOR_PRESENTATION_KIND: u16 = 0x00C4;
}

/// Sparse typed action slots; absent entries are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptActionRecords {
    records: BTreeMap<ScriptStateWordTriple, ScriptActionRecord>,
    suppressed_actions: BTreeSet<ScriptStateWordTriple>,
}

impl ScriptActionRecords {
    /// Read one action slot, treating an unmaterialized slot as empty.
    pub fn record(&self, slot: ScriptStateWordTriple) -> ScriptActionRecord {
        self.records.get(&slot).copied().unwrap_or_default()
    }

    /// Initialize or replace one typed action slot.
    pub fn set_record(&mut self, slot: ScriptStateWordTriple, record: ScriptActionRecord) {
        if record == ScriptActionRecord::Empty {
            self.records.remove(&slot);
            self.suppressed_actions.remove(&slot);
        } else {
            self.records.insert(slot, record);
            self.suppressed_actions.remove(&slot);
        }
    }

    /// Return whether the slot contains unprocessed action work.
    pub fn is_actionable(&self, slot: ScriptStateWordTriple) -> bool {
        self.record(slot) != ScriptActionRecord::Empty && !self.suppressed_actions.contains(&slot)
    }

    /// Mark whether a retained record should be dispatched by the post-frame scan.
    pub fn set_actionable(&mut self, slot: ScriptStateWordTriple, actionable: bool) {
        if actionable || self.record(slot) == ScriptActionRecord::Empty {
            self.suppressed_actions.remove(&slot);
        } else {
            self.suppressed_actions.insert(slot);
        }
    }
}

/// Explicit object identities replacing C1's special raw operands 1 and 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordStateNavigationContext {
    /// Object selected by the original primary special operand.
    pub primary_object: ScriptObjectId,
    /// Object selected by the original secondary special operand.
    pub secondary_object: ScriptObjectId,
    /// Navigation arche used when a parent relation contains the sentinel.
    pub arche: ScriptObjectId,
}

/// Observable result of one C1 query or assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordStateOutcome {
    /// Resulting BloodScript control flow.
    pub control: ScriptControl,
    /// Slot written by a successful assignment, including redirected writes.
    pub written_slot: Option<ScriptStateWordTriple>,
}

/// Presentation line selected after a successful C2 aboard transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptAboardPresentationLine {
    /// Native line 39 for a character moved aboard.
    ActorArrived,
    /// Native line 43 for descriptor-backed inventory moved aboard.
    InventoryArrived,
}

/// Presentation state changed by a successful C2 aboard transition.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptAboardPresentationState {
    /// Existing presentation work still owns the C2 gate.
    pub presentation_gate_active: bool,
    /// Line selected for the new aboard presentation.
    pub active_line: Option<ScriptAboardPresentationLine>,
}

/// Already-resolved host inputs used by C2 presentation gates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptAboardRecordContext {
    /// The ship interface currently suppresses new presentation work.
    pub ship_interface_active: bool,
    /// The related inventory object's name has a descriptor entry.
    pub descriptor_available: bool,
}

/// Observable effects of one C2 aboard-object operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptAboardRecordOutcome {
    /// Script control flow after a query or assignment.
    pub control: ScriptControl,
    /// Whether the related object's holder was changed to aboard.
    pub holder_changed: bool,
    /// Whether the descriptor catalog was consulted.
    pub descriptor_checked: bool,
    /// Whether a new presentation line was selected.
    pub presentation_requested: bool,
}

/// Presentation globals changed only when C9 tears down a C4 actor record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptRecordClearPresentationState {
    /// Whether the native sequence gate at this record layer is active.
    pub sequence_active: bool,
    /// Ship-view depth transition step consumed after actor teardown.
    pub ship_3d_depth_step: u8,
}

/// Observable result of one C9 action-record teardown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordClearOutcome {
    /// Reciprocal actor slot cleared for an old C4 record, if any.
    pub reciprocal_slot: Option<ScriptStateWordTriple>,
}

/// Invalid typed state encountered by the C1 handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRecordStateError {
    /// The authored record slot has no owning profile object.
    MissingOwner {
        /// Ownerless slot.
        slot: ScriptStateWordTriple,
    },
    /// An object identity does not exist in this profile state.
    MissingObject {
        /// Missing object.
        object: ScriptObjectId,
    },
    /// A required action field is absent from an object's proven layout.
    MissingActionField {
        /// Object lacking the field.
        object: ScriptObjectId,
    },
    /// A C2 related object has no proven holder field.
    MissingHolderField {
        /// Object lacking the field.
        object: ScriptObjectId,
    },
    /// A special operand was applied without its explicit object mapping.
    MissingNavigationContext,
    /// An untyped native operand cannot participate in object navigation.
    MissingOperandObject,
    /// A typed navigation helper rejected malformed state.
    Navigation(ScriptNavigationError),
    /// A failed operation had no branch target to consume.
    Control(ScriptRuntimeError),
}

impl fmt::Display for ScriptRecordStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptRecordStateError {}

/// Apply `vm_op_c1_record_state` to typed action slots and object relations.
pub fn apply_record_state_operation(
    operation: ScriptRecordStateOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    navigation: Option<ScriptRecordStateNavigationContext>,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    if runtime.query_mode() {
        return query_record_state(operation, state, records, runtime);
    }

    let owner = operation
        .target
        .object()
        .ok_or(ScriptRecordStateError::MissingOwner {
            slot: operation.target,
        })?;
    if object_has_flag(state, owner, ScriptObjectFlag::Active) != Some(true) {
        return failed_outcome(runtime);
    }

    let mut target = owner;
    if is_special_operand(operation.operand) {
        let context = navigation.ok_or(ScriptRecordStateError::MissingNavigationContext)?;
        let operand_object = operand_object(operation.operand, Some(context))?;
        let distance = navigation_distance(
            state,
            operand_object,
            owner,
            context.arche,
            u16::from(operation.inverted),
        )
        .map_err(ScriptRecordStateError::Navigation)?;
        if distance != u16::MIN {
            let Some(parent) = parent_object(state, owner) else {
                return failed_outcome(runtime);
            };
            if state.object(parent).map(|object| object.kind)
                != Some(ScriptObjectKind::NavigationEntity)
            {
                return failed_outcome(runtime);
            }
            target = parent;
        }
    }

    let destination = if state.object(target).map(|object| object.kind)
        == Some(ScriptObjectKind::NavigationEntity)
    {
        let operand_object = operand_object(operation.operand, navigation)?;
        let sources =
            navigation_source_objects(state, target).map_err(ScriptRecordStateError::Navigation)?;
        let operand_in_play =
            object_has_flag(state, operand_object, ScriptObjectFlag::InPlay) == Some(true);
        let accepted = sources
            .iter()
            .any(|source| match state.object(*source).map(|o| o.kind) {
                Some(ScriptObjectKind::Actor) => {
                    object_links_to(state, *source, operand_object) == Some(true)
                }
                Some(ScriptObjectKind::Player) => operand_in_play,
                _ => false,
            });
        if !accepted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        action_slot(state, target)
            .ok_or(ScriptRecordStateError::MissingActionField { object: target })?
    } else {
        operation.target
    };

    if records.record(destination) != ScriptActionRecord::Empty {
        return failed_outcome(runtime);
    }
    records.set_record(
        destination,
        ScriptActionRecord::Navigation(operation.operand),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(destination),
    })
}

/// Apply `vm_op_c2_record_full` as a typed aboard-object transition or query.
#[allow(clippy::too_many_arguments)]
pub fn apply_aboard_record_operation(
    operation: ScriptAboardRecordOperation,
    state: &ScriptState,
    records: &ScriptActionRecords,
    fields: &mut ScriptRecordFields,
    roster: &mut AboardObjectRoster,
    context: ScriptAboardRecordContext,
    request_flags: &mut PresentationRequestFlags,
    presentation: &mut ScriptAboardPresentationState,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptAboardRecordOutcome, ScriptRecordStateError> {
    let owner = operation
        .target
        .object()
        .ok_or(ScriptRecordStateError::MissingOwner {
            slot: operation.target,
        })?;
    let owner_active = object_has_flag(state, owner, ScriptObjectFlag::Active) == Some(true);
    if runtime.query_mode() {
        let matches = owner_active
            && records.record(operation.target)
                == ScriptActionRecord::AboardRequest(operation.related);
        let control = if matches != operation.inverted {
            ScriptControl::Continue
        } else {
            runtime
                .fail_guard()
                .map_err(ScriptRecordStateError::Control)?
        };
        return Ok(ScriptAboardRecordOutcome {
            control,
            holder_changed: false,
            descriptor_checked: false,
            presentation_requested: false,
        });
    }

    let related_presentable =
        object_has_flag(state, operation.related, ScriptObjectFlag::Presentable) == Some(true);
    if !owner_active || !related_presentable || !insert_aboard_object(roster, operation.related) {
        return Ok(ScriptAboardRecordOutcome {
            control: ScriptControl::Continue,
            holder_changed: false,
            descriptor_checked: false,
            presentation_requested: false,
        });
    }

    let related = state
        .object(operation.related)
        .ok_or(ScriptRecordStateError::MissingObject {
            object: operation.related,
        })?;
    let holder_offset = script_field_offset(related.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
        .ok_or(ScriptRecordStateError::MissingHolderField {
            object: operation.related,
        })?;
    let holder = state
        .object_word(
            operation.related,
            holder_offset / std::mem::size_of::<u16>(),
        )
        .ok_or(ScriptRecordStateError::MissingHolderField {
            object: operation.related,
        })?;
    fields.set_value(holder, ScriptRecordValue::Aboard);

    let can_request = !context.ship_interface_active && !request_flags.secondary_request_pending();
    let descriptor_checked = can_request && related.kind == ScriptObjectKind::InventoryItem;
    let active_line = if can_request && related.kind == ScriptObjectKind::Actor {
        Some(ScriptAboardPresentationLine::ActorArrived)
    } else if descriptor_checked && context.descriptor_available {
        request_flags.request_secondary();
        Some(ScriptAboardPresentationLine::InventoryArrived)
    } else {
        None
    };
    if let Some(active_line) = active_line {
        presentation.presentation_gate_active = false;
        presentation.active_line = Some(active_line);
    }

    Ok(ScriptAboardRecordOutcome {
        control: ScriptControl::Continue,
        holder_changed: true,
        descriptor_checked,
        presentation_requested: active_line.is_some(),
    })
}

/// Apply `vm_op_c3_state_record` to one typed presentation-queue slot.
pub fn apply_presentation_queue_operation(
    operation: ScriptPresentationQueueOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let owner = operation
        .target
        .object()
        .ok_or(ScriptRecordStateError::MissingOwner {
            slot: operation.target,
        })?;
    let owner_active = object_has_flag(state, owner, ScriptObjectFlag::Active) == Some(true);
    let matches = owner_active
        && records.record(operation.target)
            == ScriptActionRecord::PresentationQueue(operation.related);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    let related_active =
        object_has_flag(state, operation.related, ScriptObjectFlag::Active) == Some(true);
    if !owner_active || !related_active || record_is_actor(records.record(operation.target)) {
        return failed_outcome(runtime);
    }

    records.set_record(
        operation.target,
        ScriptActionRecord::PresentationQueue(operation.related),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply `vm_op_c4_actor` to typed actor-presentation action slots.
pub fn apply_actor_record_operation(
    operation: ScriptActorRecordOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let owner = operation
        .target
        .object()
        .ok_or(ScriptRecordStateError::MissingOwner {
            slot: operation.target,
        })?;
    let matches = object_has_flag(state, owner, ScriptObjectFlag::Active) == Some(true)
        && records.record(operation.target)
            == ScriptActionRecord::ActorPresentation(operation.related);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    if object_has_flag(state, owner, ScriptObjectFlag::Active) != Some(true)
        || object_has_flag(state, operation.related, ScriptObjectFlag::Active) != Some(true)
    {
        return failed_outcome(runtime);
    }
    let owner_kind = state
        .object(owner)
        .ok_or(ScriptRecordStateError::MissingObject { object: owner })?
        .kind;
    let related_kind = state
        .object(operation.related)
        .ok_or(ScriptRecordStateError::MissingObject {
            object: operation.related,
        })?
        .kind;
    if owner_kind != ScriptObjectKind::Player && related_kind != ScriptObjectKind::Player {
        if record_is_actor(records.record(operation.target)) {
            return failed_outcome(runtime);
        }
        let reciprocal_is_actor = action_slot(state, operation.related)
            .is_some_and(|slot| record_is_actor(records.record(slot)));
        if reciprocal_is_actor {
            return failed_outcome(runtime);
        }
    }

    records.set_record(
        operation.target,
        ScriptActionRecord::ActorPresentation(operation.related),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply `vm_op_c5_record_match` to one typed world-state link slot.
pub fn apply_world_state_record_operation(
    operation: ScriptWorldStateRecordOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let matches =
        records.record(operation.target) == ScriptActionRecord::WorldStateLink(operation.related);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    let related_is_active_world_state =
        object_has_flag(state, operation.related, ScriptObjectFlag::Active) == Some(true)
            && state.object(operation.related).map(|object| object.kind)
                == Some(ScriptObjectKind::WorldState);
    if !related_is_active_world_state
        || records.record(operation.target) != ScriptActionRecord::Empty
    {
        return failed_outcome(runtime);
    }

    records.set_record(
        operation.target,
        ScriptActionRecord::WorldStateLink(operation.related),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply `vm_op_c6_record_match` to one typed travel-action slot.
pub fn apply_travel_record_operation(
    operation: ScriptTravelRecordOperation,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let matches =
        records.record(operation.target) == ScriptActionRecord::Travel(operation.destination);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    records.set_record(
        operation.target,
        ScriptActionRecord::Travel(operation.destination),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply `vm_op_c7_record_match` to one typed active-object link slot.
pub fn apply_active_object_record_operation(
    operation: ScriptActiveObjectRecordOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let matches =
        records.record(operation.target) == ScriptActionRecord::ActiveObjectLink(operation.related);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    let current = records.record(operation.target);
    let destination_available = current == ScriptActionRecord::Empty || record_is_actor(current);
    if object_has_flag(state, operation.related, ScriptObjectFlag::Active) != Some(true)
        || !destination_available
    {
        return failed_outcome(runtime);
    }

    records.set_record(
        operation.target,
        ScriptActionRecord::ActiveObjectLink(operation.related),
    );
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply dormant `vm_op_c8_record_match` behavior to one bounded marker slot.
///
/// The marker has no evidenced gameplay-specific meaning: it is absent from all
/// shipped scripts and no native routine has a C8-specific consumer.
pub fn apply_opaque_marker_record_operation(
    operation: ScriptOpaqueMarkerRecordOperation,
    records: &mut ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let matches = records.record(operation.target)
        == ScriptActionRecord::OpaqueMarker(operation.comparison_word);
    if runtime.query_mode() {
        if matches != operation.inverted {
            return Ok(ScriptRecordStateOutcome {
                control: ScriptControl::Continue,
                written_slot: None,
            });
        }
        return failed_outcome(runtime);
    }

    if records.record(operation.target) != ScriptActionRecord::Empty {
        return failed_outcome(runtime);
    }
    records.set_record(operation.target, ScriptActionRecord::OpaqueMarker(u16::MIN));
    Ok(ScriptRecordStateOutcome {
        control: ScriptControl::Continue,
        written_slot: Some(operation.target),
    })
}

/// Apply `vm_op_c9_clear_record_full` through typed action ownership.
pub fn apply_record_clear_operation(
    operation: ScriptRecordClearOperation,
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    presentation: &mut ScriptRecordClearPresentationState,
) -> Result<ScriptRecordClearOutcome, ScriptRecordStateError> {
    let old_record = records.record(operation.target);
    records.set_record(operation.target, ScriptActionRecord::Empty);

    let reciprocal_slot = if let ScriptActionRecord::ActorPresentation(related) = old_record {
        let reciprocal = action_slot(state, related)
            .ok_or(ScriptRecordStateError::MissingActionField { object: related })?;
        presentation.sequence_active = false;
        presentation.ship_3d_depth_step = POST_ACTOR_CLEAR_DEPTH_STEP;
        records.set_record(reciprocal, ScriptActionRecord::Empty);
        Some(reciprocal)
    } else {
        None
    };

    Ok(ScriptRecordClearOutcome { reciprocal_slot })
}

fn query_record_state(
    operation: ScriptRecordStateOperation,
    state: &ScriptState,
    records: &ScriptActionRecords,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    let direct = records.record(operation.target);
    let comparison_slot = if is_special_operand(operation.operand)
        && !matches!(direct, ScriptActionRecord::Navigation(_))
    {
        resolved_query_slot(state, operation.target, operation.operand)
    } else {
        Some(operation.target)
    };
    let matches = comparison_slot.is_some_and(|slot| {
        records.record(slot) == ScriptActionRecord::Navigation(operation.operand)
    });
    if matches != operation.inverted {
        Ok(ScriptRecordStateOutcome {
            control: ScriptControl::Continue,
            written_slot: None,
        })
    } else {
        failed_outcome(runtime)
    }
}

fn resolved_query_slot(
    state: &ScriptState,
    direct_slot: ScriptStateWordTriple,
    operand: ScriptRecordStateOperand,
) -> Option<ScriptStateWordTriple> {
    let owner = direct_slot.object()?;
    let layout_kind = match operand {
        ScriptRecordStateOperand::PrimaryNavigationObject => ScriptObjectKind::Player,
        ScriptRecordStateOperand::SecondaryNavigationObject => ScriptObjectKind::Actor,
        _ => return None,
    };
    let field_offset = script_field_offset(layout_kind, ScriptFieldSelector::HOLDER_OR_LOCATION)?;
    let field = state.object_word(owner, field_offset / std::mem::size_of::<u16>())?;
    let ScriptStateObjectReference::Object(target) = state.object_reference(field)? else {
        return None;
    };
    action_slot(state, target)
}

fn parent_object(state: &ScriptState, object: ScriptObjectId) -> Option<ScriptObjectId> {
    let state_object = state.object(object)?;
    let field_offset =
        script_field_offset(state_object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)?;
    let field = state.object_word(object, field_offset / std::mem::size_of::<u16>())?;
    match state.object_reference(field)? {
        ScriptStateObjectReference::Object(parent) => Some(parent),
        ScriptStateObjectReference::Sentinel => None,
    }
}

pub(super) fn action_slot(
    state: &ScriptState,
    object: ScriptObjectId,
) -> Option<ScriptStateWordTriple> {
    let state_object = state.object(object)?;
    let field_offset = script_field_offset(state_object.kind, ScriptFieldSelector::ACTION)?;
    state.object_word_triple(object, field_offset / std::mem::size_of::<u16>())
}

const fn record_is_actor(record: ScriptActionRecord) -> bool {
    matches!(record, ScriptActionRecord::ActorPresentation(_))
}

fn operand_object(
    operand: ScriptRecordStateOperand,
    context: Option<ScriptRecordStateNavigationContext>,
) -> Result<ScriptObjectId, ScriptRecordStateError> {
    match operand {
        ScriptRecordStateOperand::PrimaryNavigationObject => context
            .map(|context| context.primary_object)
            .ok_or(ScriptRecordStateError::MissingNavigationContext),
        ScriptRecordStateOperand::SecondaryNavigationObject => context
            .map(|context| context.secondary_object)
            .ok_or(ScriptRecordStateError::MissingNavigationContext),
        ScriptRecordStateOperand::Object(object) => Ok(object),
        ScriptRecordStateOperand::NativeWord(_) => {
            Err(ScriptRecordStateError::MissingOperandObject)
        }
    }
}

const fn is_special_operand(operand: ScriptRecordStateOperand) -> bool {
    matches!(
        operand,
        ScriptRecordStateOperand::PrimaryNavigationObject
            | ScriptRecordStateOperand::SecondaryNavigationObject
    )
}

fn failed_outcome(
    runtime: &mut ScriptRuntime,
) -> Result<ScriptRecordStateOutcome, ScriptRecordStateError> {
    Ok(ScriptRecordStateOutcome {
        control: runtime
            .fail_guard()
            .map_err(ScriptRecordStateError::Control)?,
        written_slot: None,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::{ScriptCodeOffset, ScriptDecodingMode, decode_script_code};
    use commander_blood_formats::instruction::{
        decode_script_actor_record_operation, decode_script_record_state_operation,
    };
    use commander_blood_formats::script::{
        ScriptDirectory, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const RECORD_STATE_OPCODE: u8 = 0xC1;
    const ACTOR_RECORD_OPCODE: u8 = 0xC4;
    const SHIPPED_RECORD_STATE_COUNT: usize = 20;
    const SHIPPED_ACTOR_RECORD_COUNTS: [usize; PROFILE_COUNT] = [9, 95, 138, 66, 81];
    const ACTOR_HANDLER_VECTOR_COUNT: usize = 20;
    const PRESENTATION_QUEUE_HANDLER_VECTOR_COUNT: usize = 16;
    const WORLD_STATE_HANDLER_VECTOR_COUNT: usize = 14;
    const TRAVEL_HANDLER_VECTOR_COUNT: usize = 11;
    const ACTIVE_OBJECT_HANDLER_VECTOR_COUNT: usize = 15;
    const OPAQUE_MARKER_HANDLER_VECTOR_COUNT: usize = 13;
    const RECORD_CLEAR_HANDLER_VECTOR_COUNT: usize = 8;
    const FAILURE_TARGET: usize = 9_320;
    const HANDLER_VECTOR_COUNT: usize = 21;
    const ABOARD_HANDLER_VECTOR_COUNT: usize = 23;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const ACTIVE_FLAG: u16 = 1;
    const IN_PLAY_FLAG: u16 = 2;
    const QUERY_MODE_FLAG: u8 = 1;
    const PRIMARY_NAVIGATION_OPERAND: u16 = 1;
    const SECONDARY_NAVIGATION_OPERAND: u16 = 2;
    const OBJECT_FLAGS_WORD_INDEX: usize = 1;
    const EMPTY_ACTION_RECORD: [u16; 3] = [u16::MIN; 3];

    const PRIMARY_OBJECT: usize = 0;
    const SECONDARY_OBJECT: usize = 1;
    const DIRECT_OWNER: usize = 2;
    const DESTINATION_OBJECT: usize = 3;
    const NAVIGATION_TARGET: usize = 4;
    const ACTOR_SOURCE: usize = 5;
    const PLAYER_SOURCE: usize = 6;
    const WRONG_PARENT: usize = 7;
    const ARCHE_OBJECT: usize = 8;
    const AUXILIARY_OWNER: usize = 9;

    const ACTOR_OWNER: usize = 0;
    const ACTOR_RELATED: usize = 1;
    const ACTOR_ALTERNATE_RELATED: usize = 2;
    const PLAYER_KIND_MASK: u16 = 1;
    const ACTOR_KIND_MASK: u16 = 2;
    const CELESTIAL_BODY_KIND_MASK: u16 = 8;
    const BLACK_HOLE_KIND_MASK: u16 = 256;
    const WORLD_STATE_KIND_MASK: u16 = 512;
    const INVENTORY_ITEM_KIND_MASK: u16 = 1_024;
    const MIXED_CELESTIAL_WORLD_KIND_MASK: u16 = CELESTIAL_BODY_KIND_MASK | WORLD_STATE_KIND_MASK;
    const PRESENTATION_QUEUE_RECORD_KIND: u16 = 195;
    const ACTOR_RECORD_KIND: u16 = 196;
    const WORLD_STATE_RECORD_KIND: u16 = 197;
    const TRAVEL_RECORD_KIND: u16 = 198;
    const ACTIVE_OBJECT_RECORD_KIND: u16 = 199;
    const OPAQUE_MARKER_RECORD_KIND: u16 = 200;
    const ABOARD_RECORD_KIND: u16 = 194;
    const ACTOR_ABOARD_LINE: u16 = 39;
    const INVENTORY_ABOARD_LINE: u16 = 43;
    const UNCHANGED_ACTIVE_LINE: u16 = 4_951;
    const SECONDARY_REQUEST_FLAG: u8 = 2;
    const UNRELATED_REQUEST_FLAG: u8 = 64;
    const INITIAL_RECORD_CLEAR_DEPTH_STEP: u8 = 90;

    #[derive(Deserialize)]
    struct HandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        operand: u16,
        branch_failed: bool,
        destination_offset: Option<u16>,
    }

    #[derive(Deserialize)]
    struct ActorHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        owner_kind: u16,
        owner_flags: u16,
        related_offset: u16,
        related_kind: u16,
        related_flags: u16,
        record_before: [u16; 3],
        reciprocal_field: ActorReciprocalOracle,
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct ActorReciprocalOracle {
        value: u16,
    }

    #[derive(Deserialize)]
    struct PresentationQueueHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        owner_flags: u16,
        related_offset: u16,
        related_flags: u16,
        record_before: [u16; 3],
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct WorldStateHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        operand: u16,
        record_before: [u16; 3],
        related_kind: u16,
        related_active_byte: u16,
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct TravelHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        operand: u16,
        record_before: [u16; 3],
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct ActiveObjectHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        operand: u16,
        record_before: [u16; 3],
        related_active_byte: u16,
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct OpaqueMarkerHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        operand: u16,
        record_before: [u16; 3],
        branch_failed: bool,
    }

    #[derive(Deserialize)]
    struct RecordClearHandlerOracle {
        name: String,
        old_record: [u16; 3],
        sequence_active_after: u8,
        depth_step_after: u8,
    }

    #[derive(Deserialize)]
    struct AboardHandlerOracle {
        name: String,
        query_mode_before: u8,
        inverted: bool,
        owner_flags: u16,
        related_offset: u16,
        related_kind: u16,
        related_flags: u16,
        record: [u16; 3],
        slot_insert: AboardSlotOracle,
        field_store: AboardFieldStoreOracle,
        descript_called: bool,
        descript_result: u16,
        branch_failed: bool,
        request_flags_after: u8,
        active_line_after: u16,
    }

    #[derive(Deserialize)]
    struct AboardSlotOracle {
        called: bool,
        succeeded: bool,
    }

    #[derive(Deserialize)]
    struct AboardFieldStoreOracle {
        written: bool,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn every_shipped_c1_navigation_request_writes_its_typed_action() {
        let mut count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == RECORD_STATE_OPCODE)
            {
                let operation =
                    decode_script_record_state_operation(token, &state, &directory).unwrap();
                assert_eq!(
                    state.word_triple(operation.target),
                    Some(EMPTY_ACTION_RECORD),
                    "shipped C1 destination starts occupied"
                );
                let mut records = ScriptActionRecords::default();
                let mut runtime = ScriptRuntime::new();
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));

                let outcome = apply_record_state_operation(
                    operation,
                    &state,
                    &mut records,
                    None,
                    &mut runtime,
                )
                .unwrap();

                assert_eq!(outcome.control, ScriptControl::Continue);
                assert_eq!(outcome.written_slot, Some(operation.target));
                assert_eq!(
                    records.record(operation.target),
                    ScriptActionRecord::Navigation(operation.operand)
                );
                count += 1;
            }
        }

        assert_eq!(count, SHIPPED_RECORD_STATE_COUNT);
    }

    #[test]
    fn every_shipped_c4_actor_request_has_typed_runtime_inputs() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == ACTOR_RECORD_OPCODE)
            {
                let operation =
                    decode_script_actor_record_operation(token, &state, &directory).unwrap();
                let mut records = ScriptActionRecords::default();
                let mut runtime = ScriptRuntime::new();
                let query_mode = token.mode_before() == ScriptDecodingMode::Query;
                if query_mode {
                    runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
                } else {
                    runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
                }

                let outcome =
                    apply_actor_record_operation(operation, &state, &mut records, &mut runtime)
                        .unwrap();

                let owner = operation.target.object().unwrap();
                let endpoints_active = object_has_flag(&state, owner, ScriptObjectFlag::Active)
                    == Some(true)
                    && object_has_flag(&state, operation.related, ScriptObjectFlag::Active)
                        == Some(true);
                let branch_failed = query_mode || !endpoints_active;
                assert_eq!(
                    outcome.control,
                    if branch_failed {
                        ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                    } else {
                        ScriptControl::Continue
                    }
                );
                assert_eq!(outcome.written_slot.is_some(), !branch_failed);
                if !branch_failed {
                    assert_eq!(
                        records.record(operation.target),
                        ScriptActionRecord::ActorPresentation(operation.related)
                    );
                }
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, SHIPPED_ACTOR_RECORD_COUNTS);
    }

    #[test]
    fn aboard_record_handler_matches_every_original_decision_vector() {
        let vectors: Vec<AboardHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6e34_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ABOARD_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let related_kind = aboard_oracle_kind(vector.related_kind);
            let (mut state, ids) = aboard_handler_fixture(related_kind);
            set_flags(&mut state, ids[ACTOR_OWNER], vector.owner_flags);
            set_flags(&mut state, ids[ACTOR_RELATED], vector.related_flags);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptAboardRecordOperation {
                target,
                related: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            if vector.record[0] == ABOARD_RECORD_KIND {
                let related = if vector.record[1] == vector.related_offset {
                    ids[ACTOR_RELATED]
                } else {
                    ids[ACTOR_ALTERNATE_RELATED]
                };
                records.set_record(target, ScriptActionRecord::AboardRequest(related));
            } else if vector.record[0] != u16::MIN {
                records.set_record(target, ScriptActionRecord::Occupied);
            }

            let mut roster = AboardObjectRoster::default();
            if vector.name == "set_slot_existing_succeeds" {
                assert!(insert_aboard_object(&mut roster, ids[ACTOR_RELATED]));
            } else if vector.name == "set_slot_full_returns_without_branch" {
                for object in ids
                    .iter()
                    .copied()
                    .skip(ACTOR_ALTERNATE_RELATED)
                    .take(super::super::ABOARD_OBJECT_CAPACITY)
                {
                    assert!(insert_aboard_object(&mut roster, object));
                }
            }
            let slots_before = roster.clone();
            let mut fields = ScriptRecordFields::default();
            let initial_request_flags = if vector.name == "set_request_gate_blocks_request" {
                SECONDARY_REQUEST_FLAG
            } else if vector.name == "set_descript_success_preserves_other_request_bits" {
                UNRELATED_REQUEST_FLAG
            } else {
                u8::MIN
            };
            let mut request_flags = PresentationRequestFlags::decode(initial_request_flags);
            let mut presentation = ScriptAboardPresentationState {
                presentation_gate_active: true,
                active_line: None,
            };
            let context = ScriptAboardRecordContext {
                ship_interface_active: vector.name == "set_ui_gate_blocks_request",
                descriptor_available: vector.descript_result != u16::MIN,
            };
            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }

            let outcome = apply_aboard_record_operation(
                operation,
                &state,
                &records,
                &mut fields,
                &mut roster,
                context,
                &mut request_flags,
                &mut presentation,
                &mut runtime,
            )
            .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.holder_changed, vector.field_store.written,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.descriptor_checked, vector.descript_called,
                "{}",
                vector.name
            );
            assert_eq!(
                request_flags.bits(),
                vector.request_flags_after,
                "{}",
                vector.name
            );
            let expected_line = match vector.active_line_after {
                ACTOR_ABOARD_LINE => Some(ScriptAboardPresentationLine::ActorArrived),
                INVENTORY_ABOARD_LINE => Some(ScriptAboardPresentationLine::InventoryArrived),
                UNCHANGED_ACTIVE_LINE => None,
                unknown => panic!("unknown C2 oracle presentation line {unknown}"),
            };
            assert_eq!(presentation.active_line, expected_line, "{}", vector.name);
            assert_eq!(
                outcome.presentation_requested,
                expected_line.is_some(),
                "{}",
                vector.name
            );
            assert_eq!(
                vector.slot_insert.called,
                vector.field_store.written || vector.name == "set_slot_full_returns_without_branch"
            );
            if vector.slot_insert.succeeded {
                assert!(roster.slots().contains(&Some(ids[ACTOR_RELATED])));
            } else if !vector.slot_insert.called {
                assert_eq!(roster, slots_before);
            }
            if vector.field_store.written {
                let holder = resolved_object_field(
                    &state,
                    ids[ACTOR_RELATED],
                    ScriptFieldSelector::HOLDER_OR_LOCATION,
                )
                .unwrap();
                assert_eq!(fields.value(holder), Some(ScriptRecordValue::Aboard));
            }
        }
    }

    #[test]
    fn actor_record_handler_matches_every_original_decision_vector() {
        let vectors: Vec<ActorHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6c7e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ACTOR_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let owner_kind = oracle_object_kind(vector.owner_kind);
            let related_kind = oracle_object_kind(vector.related_kind);
            let (mut state, ids) = actor_handler_fixture(owner_kind, related_kind);
            set_flags(&mut state, ids[ACTOR_OWNER], vector.owner_flags);
            set_flags(&mut state, ids[ACTOR_RELATED], vector.related_flags);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptActorRecordOperation {
                target,
                related: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            if vector.record_before[0] == ACTOR_RECORD_KIND {
                let related = if vector.record_before[1] == vector.related_offset {
                    ids[ACTOR_RELATED]
                } else {
                    ids[ACTOR_ALTERNATE_RELATED]
                };
                records.set_record(target, ScriptActionRecord::ActorPresentation(related));
            } else if vector.record_before[0] != u16::MIN {
                records.set_record(target, ScriptActionRecord::Occupied);
            }
            if vector.reciprocal_field.value == ACTOR_RECORD_KIND {
                let reciprocal = action_slot(&state, ids[ACTOR_RELATED]).unwrap();
                records.set_record(
                    reciprocal,
                    ScriptActionRecord::ActorPresentation(ids[ACTOR_OWNER]),
                );
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_actor_record_operation(operation, &state, &mut records, &mut runtime)
                    .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write =
                vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN && !vector.branch_failed;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::ActorPresentation(ids[ACTOR_RELATED]),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn presentation_queue_handler_matches_every_original_decision_vector() {
        let vectors: Vec<PresentationQueueHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6eee_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PRESENTATION_QUEUE_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (mut state, ids) =
                actor_handler_fixture(ScriptObjectKind::Actor, ScriptObjectKind::Player);
            set_flags(&mut state, ids[ACTOR_OWNER], vector.owner_flags);
            set_flags(&mut state, ids[ACTOR_RELATED], vector.related_flags);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptPresentationQueueOperation {
                target,
                related: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            match vector.record_before[0] {
                PRESENTATION_QUEUE_RECORD_KIND => {
                    let related = if vector.record_before[1] == vector.related_offset {
                        ids[ACTOR_RELATED]
                    } else {
                        ids[ACTOR_ALTERNATE_RELATED]
                    };
                    records.set_record(target, ScriptActionRecord::PresentationQueue(related));
                }
                ACTOR_RECORD_KIND => records.set_record(
                    target,
                    ScriptActionRecord::ActorPresentation(ids[ACTOR_ALTERNATE_RELATED]),
                ),
                0 => {}
                _ => records.set_record(target, ScriptActionRecord::Occupied),
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_presentation_queue_operation(operation, &state, &mut records, &mut runtime)
                    .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write =
                vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN && !vector.branch_failed;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::PresentationQueue(ids[ACTOR_RELATED]),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn world_state_record_handler_matches_every_original_decision_vector() {
        let vectors: Vec<WorldStateHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6d18_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), WORLD_STATE_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let related_kind = oracle_object_kind(vector.related_kind);
            let (mut state, ids) = actor_handler_fixture(ScriptObjectKind::Actor, related_kind);
            set_flags(&mut state, ids[ACTOR_RELATED], vector.related_active_byte);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptWorldStateRecordOperation {
                target,
                related: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            if vector.record_before[0] == WORLD_STATE_RECORD_KIND {
                let related = if vector.record_before[1] == vector.operand {
                    ids[ACTOR_RELATED]
                } else {
                    ids[ACTOR_ALTERNATE_RELATED]
                };
                records.set_record(target, ScriptActionRecord::WorldStateLink(related));
            } else if vector.record_before[0] != u16::MIN {
                records.set_record(target, ScriptActionRecord::Occupied);
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_world_state_record_operation(operation, &state, &mut records, &mut runtime)
                    .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write =
                vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN && !vector.branch_failed;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::WorldStateLink(ids[ACTOR_RELATED]),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn travel_record_handler_matches_every_original_decision_vector() {
        let vectors: Vec<TravelHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6d80_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TRAVEL_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (state, ids) = actor_handler_fixture(
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::BlackHole,
            );
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptTravelRecordOperation {
                target,
                destination: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            if vector.record_before[0] == TRAVEL_RECORD_KIND {
                let destination = if vector.record_before[1] == vector.operand {
                    ids[ACTOR_RELATED]
                } else {
                    ids[ACTOR_ALTERNATE_RELATED]
                };
                records.set_record(target, ScriptActionRecord::Travel(destination));
            } else if vector.record_before[0] != u16::MIN {
                records.set_record(target, ScriptActionRecord::Occupied);
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_travel_record_operation(operation, &mut records, &mut runtime).unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write = vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::Travel(ids[ACTOR_RELATED]),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn active_object_record_handler_matches_every_original_decision_vector() {
        let vectors: Vec<ActiveObjectHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6dcf_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ACTIVE_OBJECT_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (mut state, ids) = actor_handler_fixture(
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::WorldState,
            );
            set_flags(&mut state, ids[ACTOR_RELATED], vector.related_active_byte);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptActiveObjectRecordOperation {
                target,
                related: ids[ACTOR_RELATED],
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            match vector.record_before[0] {
                ACTIVE_OBJECT_RECORD_KIND => {
                    let related = if vector.record_before[1] == vector.operand {
                        ids[ACTOR_RELATED]
                    } else {
                        ids[ACTOR_ALTERNATE_RELATED]
                    };
                    records.set_record(target, ScriptActionRecord::ActiveObjectLink(related));
                }
                ACTOR_RECORD_KIND => {
                    records.set_record(
                        target,
                        ScriptActionRecord::ActorPresentation(ids[ACTOR_ALTERNATE_RELATED]),
                    );
                }
                u16::MIN => {}
                _ => records.set_record(target, ScriptActionRecord::Occupied),
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_active_object_record_operation(operation, &state, &mut records, &mut runtime)
                    .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write =
                vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN && !vector.branch_failed;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::ActiveObjectLink(ids[ACTOR_RELATED]),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn opaque_marker_handler_matches_every_original_decision_vector() {
        let vectors: Vec<OpaqueMarkerHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6f62_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OPAQUE_MARKER_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (state, ids) =
                actor_handler_fixture(ScriptObjectKind::Actor, ScriptObjectKind::Player);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let operation = ScriptOpaqueMarkerRecordOperation {
                target,
                comparison_word: vector.operand,
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            match vector.record_before[0] {
                OPAQUE_MARKER_RECORD_KIND => records.set_record(
                    target,
                    ScriptActionRecord::OpaqueMarker(vector.record_before[1]),
                ),
                0 => {}
                _ => records.set_record(target, ScriptActionRecord::Occupied),
            }

            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }
            let outcome =
                apply_opaque_marker_record_operation(operation, &mut records, &mut runtime)
                    .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write =
                vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN && !vector.branch_failed;
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
            if expected_write {
                assert_eq!(
                    records.record(target),
                    ScriptActionRecord::OpaqueMarker(u16::MIN),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn record_clear_handler_matches_every_original_decision_vector() {
        let vectors: Vec<RecordClearHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6fb9_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RECORD_CLEAR_HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (state, ids) =
                actor_handler_fixture(ScriptObjectKind::Actor, ScriptObjectKind::Player);
            let target = state
                .object_word_triple(ids[ACTOR_OWNER], OBJECT_FLAGS_WORD_INDEX)
                .unwrap();
            let reciprocal = action_slot(&state, ids[ACTOR_RELATED]).unwrap();
            let mut records = ScriptActionRecords::default();
            match vector.old_record[0] {
                ACTOR_RECORD_KIND => records.set_record(
                    target,
                    ScriptActionRecord::ActorPresentation(ids[ACTOR_RELATED]),
                ),
                OPAQUE_MARKER_RECORD_KIND => records.set_record(
                    target,
                    ScriptActionRecord::OpaqueMarker(vector.old_record[1]),
                ),
                0 => {}
                _ => records.set_record(target, ScriptActionRecord::Occupied),
            }
            records.set_record(reciprocal, ScriptActionRecord::Occupied);
            let mut presentation = ScriptRecordClearPresentationState {
                sequence_active: true,
                ship_3d_depth_step: INITIAL_RECORD_CLEAR_DEPTH_STEP,
            };

            let outcome = apply_record_clear_operation(
                ScriptRecordClearOperation { target },
                &state,
                &mut records,
                &mut presentation,
            )
            .unwrap();

            let cleared_actor = vector.old_record[0] == ACTOR_RECORD_KIND;
            assert_eq!(
                records.record(target),
                ScriptActionRecord::Empty,
                "{}",
                vector.name
            );
            assert_eq!(
                records.record(reciprocal),
                if cleared_actor {
                    ScriptActionRecord::Empty
                } else {
                    ScriptActionRecord::Occupied
                },
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.reciprocal_slot,
                cleared_actor.then_some(reciprocal),
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.sequence_active,
                vector.sequence_active_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.ship_3d_depth_step, vector.depth_step_after,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn record_state_handler_matches_every_original_decision_vector() {
        let vectors: Vec<HandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6b4c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), HANDLER_VECTOR_COUNT);

        for vector in vectors {
            let (directory, mut state) = handler_fixture();
            let ids = state
                .objects()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>();
            let context = ScriptRecordStateNavigationContext {
                primary_object: ids[PRIMARY_OBJECT],
                secondary_object: ids[SECONDARY_OBJECT],
                arche: ids[ARCHE_OBJECT],
            };
            let operand = match vector.operand {
                PRIMARY_NAVIGATION_OPERAND => ScriptRecordStateOperand::PrimaryNavigationObject,
                SECONDARY_NAVIGATION_OPERAND => ScriptRecordStateOperand::SecondaryNavigationObject,
                _ => ScriptRecordStateOperand::Object(ids[DESTINATION_OBJECT]),
            };
            let uses_navigation_slot = vector.name.starts_with("set_nav_");
            let uses_auxiliary_owner =
                vector.name == "set_zero_parent_field_follows_kind_word_as_pointer";
            let owner = if uses_navigation_slot {
                ids[NAVIGATION_TARGET]
            } else if uses_auxiliary_owner {
                ids[AUXILIARY_OWNER]
            } else {
                ids[DIRECT_OWNER]
            };
            let target = if uses_auxiliary_owner {
                state
                    .object_word_triple(owner, OBJECT_FLAGS_WORD_INDEX)
                    .unwrap()
            } else {
                action_slot(&state, owner).unwrap()
            };
            let operation = ScriptRecordStateOperation {
                target,
                operand,
                inverted: vector.inverted,
            };
            let mut records = ScriptActionRecords::default();
            configure_handler_case(
                &vector.name,
                &directory,
                &mut state,
                &mut records,
                operation,
                &ids,
            );
            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_FLAG != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(FAILURE_TARGET));
            } else {
                runtime.arm_root_failure_target(ScriptCodeOffset::new(FAILURE_TARGET));
            }

            let outcome = apply_record_state_operation(
                operation,
                &state,
                &mut records,
                Some(context),
                &mut runtime,
            )
            .unwrap();

            assert_eq!(
                outcome.control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(FAILURE_TARGET))
                } else {
                    ScriptControl::Continue
                },
                "{}",
                vector.name
            );
            let expected_write = vector.query_mode_before & QUERY_MODE_FLAG == u8::MIN
                && !vector.branch_failed
                && vector.destination_offset.is_some();
            assert_eq!(
                outcome.written_slot.is_some(),
                expected_write,
                "{}",
                vector.name
            );
        }
    }

    fn configure_handler_case(
        name: &str,
        directory: &ScriptDirectory,
        state: &mut ScriptState,
        records: &mut ScriptActionRecords,
        operation: ScriptRecordStateOperation,
        ids: &[ScriptObjectId],
    ) {
        let direct_slot = action_slot(state, ids[DIRECT_OWNER]).unwrap();
        let navigation_slot = action_slot(state, ids[NAVIGATION_TARGET]).unwrap();
        match name {
            "query_direct_exact_pass_owner_activity_irrelevant"
            | "query_direct_exact_inverted_branches"
            | "query_operand_one_direct_c1_skips_resolution" => {
                records.set_record(
                    operation.target,
                    ScriptActionRecord::Navigation(operation.operand),
                );
            }
            "query_direct_kind_mismatch_branches" => {
                records.set_record(operation.target, ScriptActionRecord::Occupied);
            }
            "query_direct_related_mismatch_inverted_passes" => {
                records.set_record(
                    operation.target,
                    ScriptActionRecord::Navigation(
                        ScriptRecordStateOperand::PrimaryNavigationObject,
                    ),
                );
            }
            "query_operand_two_resolves_exact_slot" => {
                set_layout_relation(
                    directory,
                    state,
                    ids[DIRECT_OWNER],
                    ScriptObjectKind::Actor,
                    ids[NAVIGATION_TARGET],
                );
                records.set_record(
                    navigation_slot,
                    ScriptActionRecord::Navigation(operation.operand),
                );
            }
            "query_resolved_zero_destination_field_branches" => {
                set_layout_relation(
                    directory,
                    state,
                    ids[DIRECT_OWNER],
                    ScriptObjectKind::Player,
                    ids[AUXILIARY_OWNER],
                );
            }
            "query_resolved_mismatch_inverted_passes_with_negative_field" => {
                set_layout_relation(
                    directory,
                    state,
                    ids[DIRECT_OWNER],
                    ScriptObjectKind::Actor,
                    ids[NAVIGATION_TARGET],
                );
                records.set_record(navigation_slot, ScriptActionRecord::Occupied);
            }
            "set_inactive_owner_branches" => {
                set_flags(state, ids[DIRECT_OWNER], u16::MIN);
            }
            "set_direct_occupied_record_branches" => {
                records.set_record(direct_slot, ScriptActionRecord::Occupied);
            }
            "set_operand_one_zero_distance_uses_requested_record" => {
                set_position(state, ids[DIRECT_OWNER], [10, 10]);
                set_position(state, ids[ARCHE_OBJECT], [10, 10]);
            }
            "set_distance_redirect_wrong_kind_branches" => {
                set_position(state, ids[DIRECT_OWNER], [10, 10]);
                set_position(state, ids[ARCHE_OBJECT], [0, 0]);
                set_parent(directory, state, ids[DIRECT_OWNER], ids[WRONG_PARENT]);
            }
            "set_nav_skips_unknown_then_accepts_kind_one_flag" => {
                set_parent(directory, state, ids[PLAYER_SOURCE], ids[NAVIGATION_TARGET]);
                set_flags(state, ids[DESTINATION_OBJECT], ACTIVE_FLAG | IN_PLAY_FLAG);
                records.set_record(navigation_slot, ScriptActionRecord::Occupied);
            }
            "set_nav_kind_one_flag_missing_exhausts_list" => {
                set_parent(directory, state, ids[PLAYER_SOURCE], ids[NAVIGATION_TARGET]);
                set_flags(state, ids[DESTINATION_OBJECT], ACTIVE_FLAG);
            }
            "set_nav_kind_two_reject_then_accept" => {
                set_parent(directory, state, ids[ACTOR_SOURCE], ids[NAVIGATION_TARGET]);
                set_parent(
                    directory,
                    state,
                    ids[SECONDARY_OBJECT],
                    ids[NAVIGATION_TARGET],
                );
                set_link(state, ids[SECONDARY_OBJECT], ids[DESTINATION_OBJECT]);
                records.set_record(navigation_slot, ScriptActionRecord::Occupied);
            }
            "set_nav_accepted_destination_occupied_branches" => {
                set_parent(directory, state, ids[ACTOR_SOURCE], ids[NAVIGATION_TARGET]);
                set_link(state, ids[ACTOR_SOURCE], ids[DESTINATION_OBJECT]);
                records.set_record(navigation_slot, ScriptActionRecord::Occupied);
            }
            "set_a1_distance_inherits_dh_and_redirects" => {
                set_position(state, ids[DIRECT_OWNER], [10, 10]);
                set_position(state, ids[ARCHE_OBJECT], [0, 0]);
                set_parent(directory, state, ids[DIRECT_OWNER], ids[NAVIGATION_TARGET]);
                set_parent(directory, state, ids[ACTOR_SOURCE], ids[NAVIGATION_TARGET]);
                set_link(state, ids[ACTOR_SOURCE], ids[SECONDARY_OBJECT]);
                records.set_record(navigation_slot, ScriptActionRecord::Occupied);
            }
            "set_direct_empty_record_writes_triple"
            | "set_zero_parent_field_follows_kind_word_as_pointer"
            | "set_nav_empty_list_reaches_shipped_epilogue_defect"
            | "set_direct_script_cursor_wraps" => {}
            unknown => panic!("unknown C1 oracle case {unknown}"),
        }
    }

    fn handler_fixture() -> (ScriptDirectory, ScriptState) {
        let kinds = [
            ScriptObjectKind::Player,
            ScriptObjectKind::Actor,
            ScriptObjectKind::WorldState,
            ScriptObjectKind::Location,
            ScriptObjectKind::NavigationEntity,
            ScriptObjectKind::Actor,
            ScriptObjectKind::Player,
            ScriptObjectKind::Location,
            ScriptObjectKind::CelestialBody,
            ScriptObjectKind::Auxiliary,
        ];
        state_fixture(&kinds)
    }

    fn actor_handler_fixture(
        owner_kind: ScriptObjectKind,
        related_kind: ScriptObjectKind,
    ) -> (ScriptState, Vec<ScriptObjectId>) {
        let (_directory, state) =
            state_fixture(&[owner_kind, related_kind, ScriptObjectKind::Location]);
        let ids = state.objects().iter().map(|object| object.id).collect();
        (state, ids)
    }

    fn aboard_handler_fixture(
        related_kind: ScriptObjectKind,
    ) -> (ScriptState, Vec<ScriptObjectId>) {
        let mut kinds = vec![ScriptObjectKind::Actor, related_kind];
        kinds.extend(std::iter::repeat_n(
            ScriptObjectKind::Location,
            super::super::ABOARD_OBJECT_CAPACITY + 1,
        ));
        let (_directory, state) = state_fixture(&kinds);
        let ids = state.objects().iter().map(|object| object.id).collect();
        (state, ids)
    }

    fn aboard_oracle_kind(mask: u16) -> ScriptObjectKind {
        match mask {
            ACTOR_KIND_MASK => ScriptObjectKind::Actor,
            INVENTORY_ITEM_KIND_MASK => ScriptObjectKind::InventoryItem,
            // These vectors patch the native field helper for a black-hole
            // record. Location preserves the no-presentation path while
            // providing the bounded holder field required by flat Rust state.
            BLACK_HOLE_KIND_MASK => ScriptObjectKind::Location,
            unknown => panic!("unknown C2 oracle object-kind mask {unknown}"),
        }
    }

    fn oracle_object_kind(mask: u16) -> ScriptObjectKind {
        match mask {
            PLAYER_KIND_MASK => ScriptObjectKind::Player,
            BLACK_HOLE_KIND_MASK => ScriptObjectKind::BlackHole,
            WORLD_STATE_KIND_MASK => ScriptObjectKind::WorldState,
            INVENTORY_ITEM_KIND_MASK => ScriptObjectKind::InventoryItem,
            MIXED_CELESTIAL_WORLD_KIND_MASK => ScriptObjectKind::CelestialBody,
            unknown => panic!("unknown oracle object-kind mask {unknown}"),
        }
    }

    fn state_fixture(kinds: &[ScriptObjectKind]) -> (ScriptDirectory, ScriptState) {
        let mut offsets = Vec::with_capacity(kinds.len());
        let mut cursor = usize::MIN;
        for kind in kinds.iter().copied() {
            offsets.push(cursor);
            cursor += kind.record_size();
        }

        let mut directory_data = Vec::new();
        let mut state_data = Vec::with_capacity(cursor);
        for (index, kind) in kinds.iter().copied().enumerate() {
            let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            entry[0] = b'a' + u8::try_from(index).unwrap();
            entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + 2]
                .copy_from_slice(&u16::try_from(offsets[index]).unwrap().to_le_bytes());
            entry[DIRECTORY_NAME_CAPACITY + 2..]
                .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
            directory_data.extend_from_slice(&entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..2].copy_from_slice(&kind.mask().to_le_bytes());
            object[2..4].copy_from_slice(&ACTIVE_FLAG.to_le_bytes());
            if let Some(parent_offset) =
                script_field_offset(kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
            {
                object[parent_offset..parent_offset + 2].copy_from_slice(&u16::MAX.to_le_bytes());
            }
            state_data.extend_from_slice(&object);
        }
        directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_data).unwrap();
        let state = decode_script_state(&state_data, &directory).unwrap();
        (directory, state)
    }

    fn set_flags(state: &mut ScriptState, object: ScriptObjectId, flags: u16) {
        assert!(state.set_word(
            state.object_word(object, OBJECT_FLAGS_WORD_INDEX).unwrap(),
            flags
        ));
    }

    fn resolved_object_field(
        state: &ScriptState,
        object: ScriptObjectId,
        selector: ScriptFieldSelector,
    ) -> Option<commander_blood_formats::script::ScriptStateWord> {
        let kind = state.object(object)?.kind;
        let byte_offset = script_field_offset(kind, selector)?;
        state.object_word(object, byte_offset / std::mem::size_of::<u16>())
    }

    fn set_parent(
        directory: &ScriptDirectory,
        state: &mut ScriptState,
        object: ScriptObjectId,
        parent: ScriptObjectId,
    ) {
        let kind = state.object(object).unwrap().kind;
        set_layout_relation(directory, state, object, kind, parent);
    }

    fn set_layout_relation(
        directory: &ScriptDirectory,
        state: &mut ScriptState,
        object: ScriptObjectId,
        layout_kind: ScriptObjectKind,
        target: ScriptObjectId,
    ) {
        let byte_offset =
            script_field_offset(layout_kind, ScriptFieldSelector::HOLDER_OR_LOCATION).unwrap();
        let field = state
            .object_word(object, byte_offset / std::mem::size_of::<u16>())
            .unwrap();
        assert!(state.set_word(field, directory.object(target).unwrap().value));
    }

    fn set_position(state: &mut ScriptState, object: ScriptObjectId, position: [u16; 2]) {
        let kind = state.object(object).unwrap().kind;
        let byte_offset =
            script_field_offset(kind, ScriptFieldSelector::NAVIGATION_POSITION).unwrap();
        let field = state
            .object_word_pair(object, byte_offset / std::mem::size_of::<u16>())
            .unwrap();
        assert!(state.set_word_pair(field, position));
    }

    fn set_link(state: &mut ScriptState, source: ScriptObjectId, target: ScriptObjectId) {
        let source_kind = state.object(source).unwrap().kind;
        let field_offset =
            script_field_offset(source_kind, ScriptFieldSelector::OBJECT_LINKS).unwrap();
        let byte_index = field_offset + target.index() / u8::BITS as usize;
        let field = state.object_byte(source, byte_index).unwrap();
        let mask = 1_u8 << (u8::BITS as usize - 1 - target.index() % u8::BITS as usize);
        let value = state.byte(field).unwrap() | mask;
        assert!(state.set_byte(field, value));
    }
}
