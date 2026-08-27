//! Typed post-frame dispatch for BloodScript action records.

use std::fmt;
use std::mem::size_of;

use commander_blood_formats::instruction::ScriptRecordStateOperand;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference, ScriptStateWordPair,
    ScriptStateWordTriple,
};

use super::record_state::action_slot;
use super::{
    AboardObjectRoster, PresentationRequestFlags, ScriptActionDispatch, ScriptActionDisposition,
    ScriptActionRecord, ScriptActionRecords, ScriptFieldSelector, ScriptNavigationError,
    ScriptObjectFlag, ScriptPresentationScanState, ScriptRecordStateNavigationContext,
    insert_aboard_object, resolve_navigation_position, script_field_offset, set_object_flag,
};

const SERIALIZED_WORD_SIZE: usize = size_of::<u16>();

/// Ship-navigation mode observed by the C1 action path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptShipNavigationMode {
    /// The bridge is not currently presenting the 3D navigation interface.
    #[default]
    Inactive,
    /// The 3D navigation interface is active with its existing target.
    Active,
    /// C1 selected a target and reset the interface for its next frame.
    TargetSelected,
}

impl ScriptShipNavigationMode {
    const fn is_active(self) -> bool {
        !matches!(self, Self::Inactive)
    }
}

/// Multi-frame phase of a C6 black-hole travel action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptTravelActionPhase {
    /// Wait for the presentation actor to finish its entry sequence.
    #[default]
    WaitingForActor,
    /// Wait for the camera transition requested by the first phase.
    WaitingForCamera,
    /// Wait for presentation work before committing the destination relation.
    WaitingForPresentation,
}

/// Authored presentation line selected by an action transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptActionPresentationLine {
    /// Native line 3 after selecting a 3D navigation target.
    NavigationTarget,
    /// Native line 39 after bringing a character aboard.
    CharacterAboard,
    /// Native line 43 after moving a descriptor-backed object aboard.
    InventoryAboard,
    /// Native line 44 between the camera and relation phases of black-hole travel.
    TravelReady,
}

/// Mutable semantic state shared by the five modeled action-record arms.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptActionState {
    /// Whether Arche has completed the four-stage camera approach.
    pub navigation_approach_complete: bool,
    /// Current 3D ship-navigation mode.
    pub ship_navigation_mode: ScriptShipNavigationMode,
    /// Object currently selected by the 3D navigation interface.
    pub current_ship_target: Option<ScriptObjectId>,
    /// A changed descriptor selected a new music path.
    pub navigation_music_changed: bool,
    /// First pending presentation word has not yet been cleared.
    pub presentation_words_pending: bool,
    /// The ship HUD must be initialized on the next frame.
    pub ship_hud_refresh_requested: bool,
    /// The ordinary bridge frame still needs rebuilding.
    pub bridge_redraw_pending: bool,
    /// Vertical scene offset loaded by the latest resource parse.
    pub loaded_scene_vertical_offset: u16,
    /// Vertical scene offset committed for rendering.
    pub scene_vertical_offset: u16,
    /// Most recent presentation line selected by this dispatcher.
    pub active_line: Option<ScriptActionPresentationLine>,
    /// Navigation owner claimed by a wildcard C3 record.
    pub pending_presentation_owner: Option<ScriptObjectId>,
    /// Whether VOC playback is already enabled.
    pub voc_playback_enabled: bool,
    /// Whether C3 requested the disabled VOC path to be enabled.
    pub radio_clip_enable_requested: bool,
    /// Whether the fixed radio clip is already playing.
    pub radio_clip_playing: bool,
    /// Current C6 travel phase.
    pub travel_phase: ScriptTravelActionPhase,
    /// Whether the travel presentation actor is still busy.
    pub travel_actor_busy: bool,
    /// C6 cleared the aliased bridge actor-slot flag after camera completion.
    pub travel_actor_clear_requested: bool,
    /// Whether a camera transition still has frames to execute.
    pub camera_transition_in_progress: bool,
    /// Whether the camera view currently owns presentation input.
    pub camera_view_active: bool,
    /// Whether the bridge scene must be rebuilt after travel.
    pub screen_rebuild_requested: bool,
    /// Object whose encounter counter requires a nested COD scan.
    pub post_update_object: Option<ScriptObjectId>,
    /// Runtime-only kind-`0x20` navigation link currently carrying the active bit.
    ///
    /// No shipped VAR object has this kind. The native arena can still expose one
    /// as a transient parent, so its identity is tracked separately from the
    /// fixed decoded object kind.
    pub active_navigation_link: Option<ScriptObjectId>,
}

/// Profile bindings and owned stores required by one action dispatch.
pub struct ScriptActionContext<'a> {
    /// Active decoded VAR state.
    pub state: &'a mut ScriptState,
    /// Typed action slots associated with the decoded state.
    pub records: &'a mut ScriptActionRecords,
    /// Fixed-capacity roster replacing the native aboard-object offset array.
    pub aboard_objects: &'a mut AboardObjectRoster,
    /// Presentation request flags shared with text and sequence scheduling.
    pub request_flags: &'a mut PresentationRequestFlags,
    /// Post-frame presentation state shared with the surrounding scan.
    pub presentation: &'a mut ScriptPresentationScanState,
    /// Action-specific ship, radio, and travel state.
    pub action: &'a mut ScriptActionState,
    /// Object owning the dispatched action slot.
    pub owner: ScriptObjectId,
    /// Typed slot supplied by the presentation scan.
    pub slot: ScriptStateWordTriple,
    /// Built-in player object named `blood`.
    pub player: ScriptObjectId,
    /// Dynamic C1 navigation bindings, unnecessary for all other record kinds.
    pub navigation: Option<ScriptRecordStateNavigationContext>,
}

/// Platform and nested-script work reached by action transitions.
pub trait ScriptActionHost {
    /// Callback failure.
    type Error;

    /// Return whether an object name resolves to a DESCRIPT record.
    fn description_available(&mut self, object: ScriptObjectId) -> Result<bool, Self::Error>;

    /// Copy the ship band and restart streaming from the selected music path.
    fn restart_navigation_music(&mut self) -> Result<(), Self::Error>;

    /// Execute the related object's COD state after an encounter-counter update.
    fn execute_object_code(
        &mut self,
        state: &ScriptState,
        object: ScriptObjectId,
    ) -> Result<(), Self::Error>;

    /// Start native radio clip 6.
    fn play_radio_clip(&mut self) -> Result<(), Self::Error>;

    /// Start the fixed camera entity transition used by C6.
    fn start_camera_transition(&mut self) -> Result<(), Self::Error>;

    /// Draw and capture the ship HUD, then reset the 3D camera.
    fn reset_ship_hud(&mut self) -> Result<(), Self::Error>;
}

/// Invalid typed profile state or host failure during action dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptActionError<HostError> {
    /// A referenced object does not exist in the active profile.
    MissingObject {
        /// Missing stable object identity.
        object: ScriptObjectId,
    },
    /// A required field is absent from an object's proven fixed record shape.
    MissingField {
        /// Object lacking the field.
        object: ScriptObjectId,
        /// Field selected by the original routine.
        selector: ScriptFieldSelector,
    },
    /// An existing holder word contains neither an object nor the aboard sentinel.
    InvalidObjectReference {
        /// Object containing the malformed relation.
        object: ScriptObjectId,
    },
    /// A native-only C1 operand could not be mapped to a profile object.
    UnsupportedNavigationOperand {
        /// Preserved word from an unshipped action record.
        value: u16,
    },
    /// A C1 record reached post-frame dispatch without live navigation bindings.
    MissingNavigationContext,
    /// A decoded object's source position does not fit the original VAR word domain.
    ObjectOffsetOutOfRange {
        /// Object whose source position cannot be encoded.
        object: ScriptObjectId,
    },
    /// A required header flag could not be updated.
    ObjectFlagUpdate {
        /// Object whose flag update failed.
        object: ScriptObjectId,
    },
    /// Typed navigation-position traversal failed.
    Navigation(ScriptNavigationError),
    /// Audio, renderer, or nested-script work failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for ScriptActionError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HostError: fmt::Debug> std::error::Error for ScriptActionError<HostError> {}

impl<HostError> From<ScriptNavigationError> for ScriptActionError<HostError> {
    fn from(error: ScriptNavigationError) -> Self {
        Self::Navigation(error)
    }
}

/// Dispatch one post-frame action record through the native C1/C2/C3/C4/C6 ladder.
///
/// This translates `record_c1_ship3d_action` at BLOODPRG file offset `0x005B38`.
/// Stable object identities, owned state fields, and typed phase enums replace
/// arena offsets and packed globals. C2 is dormant in shipped play but retains
/// the intended state transitions preceding its native unmatched stack pop.
/// The C9 arm is unreachable because `vm_op_c9_clear_record_full` clears
/// synchronously; the CD replacement arm is also unreachable because its
/// replacement globals have no writers. Unknown, C5, C7, and C8 records retain
/// the native no-op behavior.
pub fn dispatch_script_action<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    record: ScriptActionRecord,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    match record {
        ScriptActionRecord::Navigation(operand) => dispatch_navigation(context, operand, host),
        ScriptActionRecord::AboardRequest(related) => {
            dispatch_aboard_request(context, related, host)
        }
        ScriptActionRecord::PresentationQueue(related) => {
            dispatch_presentation_queue(context, related, host)
        }
        ScriptActionRecord::ActorPresentation(related) => {
            dispatch_actor_presentation(context, related, host)
        }
        ScriptActionRecord::Travel(related) => dispatch_travel(context, related, host),
        ScriptActionRecord::Empty
        | ScriptActionRecord::WorldStateLink(_)
        | ScriptActionRecord::ActiveObjectLink(_)
        | ScriptActionRecord::OpaqueMarker(_)
        | ScriptActionRecord::Occupied => Ok(ScriptActionDispatch::default()),
    }
}

fn dispatch_navigation<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    operand: ScriptRecordStateOperand,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    let ScriptActionContext {
        state,
        records,
        request_flags,
        presentation,
        action,
        owner,
        player,
        navigation,
        ..
    } = context;
    let navigation = navigation.ok_or(ScriptActionError::MissingNavigationContext)?;
    let arche = navigation.arche;
    if owner == arche && !action.navigation_approach_complete {
        return Ok(ScriptActionDispatch::default());
    }

    let related = resolve_navigation_operand(
        operand,
        navigation.primary_object,
        navigation.secondary_object,
    )?;
    let owner_kind = object_kind(state, owner)?;
    object_kind(state, related)?;
    let holder = object_word(state, owner, ScriptFieldSelector::HOLDER_OR_LOCATION)?;
    if let Some(ScriptStateObjectReference::Object(prior)) = state.object_reference(holder) {
        if action.active_navigation_link == Some(prior) {
            if !set_object_flag(state, prior, ScriptObjectFlag::Active, false) {
                return Err(ScriptActionError::ObjectFlagUpdate { object: prior });
            }
            action.active_navigation_link = None;
        }
    } else if state.object_reference(holder).is_none() {
        return Err(ScriptActionError::InvalidObjectReference { object: owner });
    }
    write_object_reference(state, holder, related)?;

    let mut copy_position = matches!(
        owner_kind,
        ScriptObjectKind::NavigationEntity | ScriptObjectKind::WorldState
    );
    let mut preserve_native_music_restart_bug = false;
    if owner_kind == ScriptObjectKind::WorldState && action.ship_navigation_mode.is_active() {
        let reset_interface = if action.current_ship_target == Some(related) {
            true
        } else {
            let available = host
                .description_available(related)
                .map_err(ScriptActionError::Host)?;
            if available && action.navigation_music_changed {
                host.restart_navigation_music()
                    .map_err(ScriptActionError::Host)?;
                preserve_native_music_restart_bug = true;
            }
            available
        };
        if reset_interface {
            clear_primary_actor_pair(state, records, player)?;
            action.current_ship_target = Some(related);
            action.ship_navigation_mode = ScriptShipNavigationMode::TargetSelected;
            action.presentation_words_pending = false;
            presentation.word_choice_active = false;
            *request_flags = PresentationRequestFlags::default();
            presentation.c2_gate_active = false;
            action.ship_hud_refresh_requested = true;
            action.bridge_redraw_pending = false;
            action.scene_vertical_offset = action.loaded_scene_vertical_offset;
            action.active_line = Some(ScriptActionPresentationLine::NavigationTarget);
        }
    } else if owner_kind != ScriptObjectKind::NavigationEntity
        && owner_kind != ScriptObjectKind::WorldState
    {
        copy_position = false;
    }

    if copy_position {
        let holder_byte_offset =
            field_byte_offset(owner_kind, ScriptFieldSelector::HOLDER_OR_LOCATION).ok_or(
                ScriptActionError::MissingField {
                    object: owner,
                    selector: ScriptFieldSelector::HOLDER_OR_LOCATION,
                },
            )?;
        let source = resolve_navigation_position(
            state,
            related,
            arche,
            u16::try_from(holder_byte_offset)
                .map_err(|_| ScriptActionError::ObjectOffsetOutOfRange { object: owner })?,
        )?;
        let position = state
            .word_pair(source)
            .ok_or(ScriptActionError::MissingField {
                object: related,
                selector: ScriptFieldSelector::NAVIGATION_POSITION,
            })?;
        if !preserve_native_music_restart_bug {
            let destination =
                object_word_pair(state, owner, ScriptFieldSelector::NAVIGATION_POSITION)?;
            if !state.set_word_pair(destination, position) {
                return Err(ScriptActionError::MissingField {
                    object: owner,
                    selector: ScriptFieldSelector::NAVIGATION_POSITION,
                });
            }
        }
    }

    Ok(clear_dispatch())
}

fn dispatch_aboard_request<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    related: ScriptObjectId,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    let ScriptActionContext {
        state,
        aboard_objects,
        request_flags,
        presentation,
        action,
        ..
    } = context;
    if !insert_aboard_object(aboard_objects, related) {
        return Ok(ScriptActionDispatch::default());
    }
    let related_kind = object_kind(state, related)?;
    let holder = object_word(state, related, ScriptFieldSelector::HOLDER_OR_LOCATION)?;
    if !state.set_word(holder, u16::MAX) {
        return Err(ScriptActionError::MissingField {
            object: related,
            selector: ScriptFieldSelector::HOLDER_OR_LOCATION,
        });
    }

    if !presentation.name_lookup_enabled && !request_flags.secondary_request_pending() {
        if related_kind == ScriptObjectKind::Actor {
            presentation.c2_gate_active = false;
            action.active_line = Some(ScriptActionPresentationLine::CharacterAboard);
        } else if related_kind == ScriptObjectKind::InventoryItem
            && host
                .description_available(related)
                .map_err(ScriptActionError::Host)?
        {
            presentation.c2_gate_active = false;
            request_flags.request_secondary();
            action.active_line = Some(ScriptActionPresentationLine::InventoryAboard);
        }
    }
    Ok(clear_dispatch())
}

fn dispatch_presentation_queue<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    related: ScriptObjectId,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    let ScriptActionContext {
        records,
        presentation,
        action,
        owner,
        slot,
        player,
        ..
    } = context;
    if related != player {
        records.set_record(slot, ScriptActionRecord::ActorPresentation(related));
        return Ok(ScriptActionDispatch::default());
    }

    action.pending_presentation_owner = Some(owner);
    if presentation.name_lookup_enabled {
        if !action.voc_playback_enabled {
            action.radio_clip_enable_requested = true;
        }
        if !action.radio_clip_playing {
            host.play_radio_clip().map_err(ScriptActionError::Host)?;
            action.radio_clip_playing = true;
        }
    }
    Ok(ScriptActionDispatch::default())
}

fn dispatch_actor_presentation<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    related: ScriptObjectId,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    let ScriptActionContext {
        state,
        records,
        presentation,
        action,
        owner,
        ..
    } = context;
    if presentation.pair_write_disabled {
        return Ok(ScriptActionDispatch::default());
    }
    let owner_kind = object_kind(state, owner)?;
    let related_kind = object_kind(state, related)?;
    let update = if owner_kind == ScriptObjectKind::Player {
        action.pending_presentation_owner = None;
        encounter_counter(state, related)?.map(|field| (related, field))
    } else if related_kind == ScriptObjectKind::Player {
        encounter_counter(state, owner)?.map(|field| (owner, field))
    } else {
        None
    };

    if let Some((updated_object, counter)) = update {
        let value = state.word(counter).ok_or(ScriptActionError::MissingField {
            object: updated_object,
            selector: ScriptFieldSelector::ENCOUNTER_COUNT,
        })?;
        if !state.set_word(counter, value.wrapping_add(1)) {
            return Err(ScriptActionError::MissingField {
                object: updated_object,
                selector: ScriptFieldSelector::ENCOUNTER_COUNT,
            });
        }
        if !set_object_flag(state, owner, ScriptObjectFlag::PresentationBlocked, true) {
            return Err(ScriptActionError::ObjectFlagUpdate { object: owner });
        }
        action.post_update_object = Some(updated_object);
        host.execute_object_code(state, updated_object)
            .map_err(ScriptActionError::Host)?;
    }

    let reciprocal = action_slot(state, related).ok_or(ScriptActionError::MissingField {
        object: related,
        selector: ScriptFieldSelector::ACTION,
    })?;
    records.set_record(reciprocal, ScriptActionRecord::ActorPresentation(owner));
    records.set_actionable(reciprocal, false);
    Ok(ScriptActionDispatch {
        disposition: ScriptActionDisposition::Suppress,
        disable_pair_writes: false,
    })
}

fn dispatch_travel<Host: ScriptActionHost>(
    context: ScriptActionContext<'_>,
    related: ScriptObjectId,
    host: &mut Host,
) -> Result<ScriptActionDispatch, ScriptActionError<Host::Error>> {
    let ScriptActionContext {
        state,
        presentation,
        action,
        owner,
        ..
    } = context;
    match action.travel_phase {
        ScriptTravelActionPhase::WaitingForActor => {
            if !action.travel_actor_busy {
                return Ok(ScriptActionDispatch::default());
            }
            action.travel_phase = ScriptTravelActionPhase::WaitingForCamera;
            action.camera_transition_in_progress = true;
            host.start_camera_transition()
                .map_err(ScriptActionError::Host)?;
            return Ok(ScriptActionDispatch::default());
        }
        ScriptTravelActionPhase::WaitingForCamera => {
            if action.camera_transition_in_progress {
                return Ok(ScriptActionDispatch::default());
            }
            action.travel_phase = ScriptTravelActionPhase::WaitingForPresentation;
            action.travel_actor_busy = false;
            action.travel_actor_clear_requested = true;
            action.camera_view_active = false;
            action.active_line = Some(ScriptActionPresentationLine::TravelReady);
            return Ok(ScriptActionDispatch::default());
        }
        ScriptTravelActionPhase::WaitingForPresentation => {
            if presentation.c2_gate_active {
                return Ok(ScriptActionDispatch::default());
            }
        }
    }

    action.travel_phase = ScriptTravelActionPhase::WaitingForActor;
    action.screen_rebuild_requested = true;
    host.reset_ship_hud().map_err(ScriptActionError::Host)?;
    presentation.ui_busy = false;

    let relation_field = object_word(state, owner, ScriptFieldSelector::BLACK_HOLE_RELATION)?;
    let relation = state
        .word(relation_field)
        .ok_or(ScriptActionError::MissingField {
            object: owner,
            selector: ScriptFieldSelector::BLACK_HOLE_RELATION,
        })?;
    let comparison_field = object_word(state, related, ScriptFieldSelector::BLACK_HOLE_COMPARISON)?;
    let comparison = state
        .word(comparison_field)
        .ok_or(ScriptActionError::MissingField {
            object: related,
            selector: ScriptFieldSelector::BLACK_HOLE_COMPARISON,
        })?;
    let (next_relation, position_selector) = if relation == comparison {
        let field = object_word(
            state,
            related,
            ScriptFieldSelector::BLACK_HOLE_MATCH_RELATION,
        )?;
        (
            state.word(field).ok_or(ScriptActionError::MissingField {
                object: related,
                selector: ScriptFieldSelector::BLACK_HOLE_MATCH_RELATION,
            })?,
            ScriptFieldSelector::BLACK_HOLE_POSITION_B,
        )
    } else {
        (comparison, ScriptFieldSelector::BLACK_HOLE_POSITION_A)
    };
    let source = object_word_pair(state, related, position_selector)?;
    let position = state
        .word_pair(source)
        .ok_or(ScriptActionError::MissingField {
            object: related,
            selector: position_selector,
        })?;
    let destination = object_word_pair(state, owner, ScriptFieldSelector::NAVIGATION_POSITION)?;
    if !state.set_word_pair(destination, position) || !state.set_word(relation_field, next_relation)
    {
        return Err(ScriptActionError::MissingField {
            object: owner,
            selector: ScriptFieldSelector::NAVIGATION_POSITION,
        });
    }
    Ok(clear_dispatch())
}

fn resolve_navigation_operand<HostError>(
    operand: ScriptRecordStateOperand,
    primary: ScriptObjectId,
    secondary: ScriptObjectId,
) -> Result<ScriptObjectId, ScriptActionError<HostError>> {
    match operand {
        ScriptRecordStateOperand::PrimaryNavigationObject => Ok(primary),
        ScriptRecordStateOperand::SecondaryNavigationObject => Ok(secondary),
        ScriptRecordStateOperand::Object(object) => Ok(object),
        ScriptRecordStateOperand::NativeWord(value) => {
            Err(ScriptActionError::UnsupportedNavigationOperand { value })
        }
    }
}

fn object_kind<HostError>(
    state: &ScriptState,
    object: ScriptObjectId,
) -> Result<ScriptObjectKind, ScriptActionError<HostError>> {
    state
        .object(object)
        .map(|record| record.kind)
        .ok_or(ScriptActionError::MissingObject { object })
}

fn field_byte_offset(kind: ScriptObjectKind, selector: ScriptFieldSelector) -> Option<usize> {
    script_field_offset(kind, selector)
}

fn object_word<HostError>(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<commander_blood_formats::script::ScriptStateWord, ScriptActionError<HostError>> {
    let kind = object_kind(state, object)?;
    let offset = field_byte_offset(kind, selector)
        .ok_or(ScriptActionError::MissingField { object, selector })?;
    state
        .object_word(object, offset / SERIALIZED_WORD_SIZE)
        .ok_or(ScriptActionError::MissingField { object, selector })
}

fn object_word_pair<HostError>(
    state: &ScriptState,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<ScriptStateWordPair, ScriptActionError<HostError>> {
    let kind = object_kind(state, object)?;
    let offset = field_byte_offset(kind, selector)
        .ok_or(ScriptActionError::MissingField { object, selector })?;
    state
        .object_word_pair(object, offset / SERIALIZED_WORD_SIZE)
        .ok_or(ScriptActionError::MissingField { object, selector })
}

fn encounter_counter<HostError>(
    state: &ScriptState,
    object: ScriptObjectId,
) -> Result<Option<commander_blood_formats::script::ScriptStateWord>, ScriptActionError<HostError>>
{
    let kind = object_kind(state, object)?;
    let Some(offset) = field_byte_offset(kind, ScriptFieldSelector::ENCOUNTER_COUNT) else {
        return Ok(None);
    };
    Ok(state.object_word(object, offset / SERIALIZED_WORD_SIZE))
}

fn write_object_reference<HostError>(
    state: &mut ScriptState,
    field: commander_blood_formats::script::ScriptStateWord,
    object: ScriptObjectId,
) -> Result<(), ScriptActionError<HostError>> {
    let source_offset = state
        .object(object)
        .ok_or(ScriptActionError::MissingObject { object })?
        .source_offset();
    let encoded = u16::try_from(source_offset)
        .map_err(|_| ScriptActionError::ObjectOffsetOutOfRange { object })?;
    if state.set_word(field, encoded) {
        Ok(())
    } else {
        Err(ScriptActionError::MissingObject { object })
    }
}

fn clear_primary_actor_pair<HostError>(
    state: &ScriptState,
    records: &mut ScriptActionRecords,
    player: ScriptObjectId,
) -> Result<(), ScriptActionError<HostError>> {
    let primary = action_slot(state, player).ok_or(ScriptActionError::MissingField {
        object: player,
        selector: ScriptFieldSelector::ACTION,
    })?;
    let ScriptActionRecord::ActorPresentation(related) = records.record(primary) else {
        return Ok(());
    };
    records.set_record(primary, ScriptActionRecord::Empty);
    let reciprocal = action_slot(state, related).ok_or(ScriptActionError::MissingField {
        object: related,
        selector: ScriptFieldSelector::ACTION,
    })?;
    records.set_record(reciprocal, ScriptActionRecord::Empty);
    Ok(())
}

const fn clear_dispatch() -> ScriptActionDispatch {
    ScriptActionDispatch {
        disposition: ScriptActionDisposition::Clear,
        disable_pair_writes: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{object_has_flag, remove_aboard_object};

    const ORACLE_VECTOR_COUNT: usize = 32;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_OBJECT_KIND: u16 = 1;
    const ACTIVE_OBJECT_FLAGS: u16 = 1;
    const EXTRA_ROSTER_OBJECT_COUNT: usize = 20;
    const PLAYER_INDEX: usize = 0;
    const ACTOR_INDEX: usize = 1;
    const ARCHE_INDEX: usize = 2;
    const WORLD_INDEX: usize = 3;
    const LOCATION_INDEX: usize = 4;
    const ANCHOR_INDEX: usize = 5;
    const BLACK_HOLE_INDEX: usize = 6;
    const INVENTORY_INDEX: usize = 7;
    const AUXILIARY_INDEX: usize = 8;

    #[derive(Deserialize)]
    struct ActionOracle {
        name: String,
        record_kind: u16,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum HostCall {
        Description(ScriptObjectId),
        RestartMusic,
        ExecuteCode(ScriptObjectId),
        PlayRadio,
        StartCamera,
        ResetHud,
    }

    #[derive(Default)]
    struct MockHost {
        description_available: bool,
        calls: Vec<HostCall>,
    }

    impl ScriptActionHost for MockHost {
        type Error = &'static str;

        fn description_available(&mut self, object: ScriptObjectId) -> Result<bool, Self::Error> {
            self.calls.push(HostCall::Description(object));
            Ok(self.description_available)
        }

        fn restart_navigation_music(&mut self) -> Result<(), Self::Error> {
            self.calls.push(HostCall::RestartMusic);
            Ok(())
        }

        fn execute_object_code(
            &mut self,
            _state: &ScriptState,
            object: ScriptObjectId,
        ) -> Result<(), Self::Error> {
            self.calls.push(HostCall::ExecuteCode(object));
            Ok(())
        }

        fn play_radio_clip(&mut self) -> Result<(), Self::Error> {
            self.calls.push(HostCall::PlayRadio);
            Ok(())
        }

        fn start_camera_transition(&mut self) -> Result<(), Self::Error> {
            self.calls.push(HostCall::StartCamera);
            Ok(())
        }

        fn reset_ship_hud(&mut self) -> Result<(), Self::Error> {
            self.calls.push(HostCall::ResetHud);
            Ok(())
        }
    }

    struct Fixture {
        state: ScriptState,
        records: ScriptActionRecords,
        aboard: AboardObjectRoster,
        requests: PresentationRequestFlags,
        presentation: ScriptPresentationScanState,
        action: ScriptActionState,
        objects: Vec<ScriptObjectId>,
    }

    impl Fixture {
        fn new() -> Self {
            let mut kinds = vec![
                ScriptObjectKind::Player,
                ScriptObjectKind::Actor,
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::WorldState,
                ScriptObjectKind::Location,
                ScriptObjectKind::NavigationEntity,
                ScriptObjectKind::BlackHole,
                ScriptObjectKind::InventoryItem,
                ScriptObjectKind::Auxiliary,
            ];
            kinds.extend(std::iter::repeat_n(
                ScriptObjectKind::Auxiliary,
                EXTRA_ROSTER_OBJECT_COUNT,
            ));

            let mut offsets = Vec::with_capacity(kinds.len());
            let mut cursor = usize::MIN;
            for kind in &kinds {
                offsets.push(cursor);
                cursor += kind.record_size();
            }
            let mut directory_data = Vec::new();
            let mut state_data = Vec::with_capacity(cursor);
            for (index, kind) in kinds.iter().copied().enumerate() {
                let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
                let name = format!("object{index}");
                entry[..name.len()].copy_from_slice(name.as_bytes());
                entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE]
                    .copy_from_slice(&u16::try_from(offsets[index]).unwrap().to_le_bytes());
                entry[DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE..]
                    .copy_from_slice(&DIRECTORY_OBJECT_KIND.to_le_bytes());
                directory_data.extend_from_slice(&entry);

                let mut object = vec![u8::MIN; kind.record_size()];
                object[..SERIALIZED_WORD_SIZE].copy_from_slice(&kind.mask().to_le_bytes());
                object[SERIALIZED_WORD_SIZE..SERIALIZED_WORD_SIZE * 2]
                    .copy_from_slice(&ACTIVE_OBJECT_FLAGS.to_le_bytes());
                state_data.extend_from_slice(&object);
            }
            directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
            let directory = decode_script_directory(&directory_data).unwrap();
            let mut state = decode_script_state(&state_data, &directory).unwrap();
            let objects: Vec<_> = state.objects().iter().map(|object| object.id).collect();

            set_reference(&mut state, objects[ACTOR_INDEX], objects[ANCHOR_INDEX]);
            set_reference(&mut state, objects[ARCHE_INDEX], objects[LOCATION_INDEX]);
            set_reference(&mut state, objects[WORLD_INDEX], objects[LOCATION_INDEX]);
            set_reference(&mut state, objects[LOCATION_INDEX], objects[ANCHOR_INDEX]);
            set_reference(
                &mut state,
                objects[INVENTORY_INDEX],
                objects[LOCATION_INDEX],
            );
            set_pair(
                &mut state,
                objects[ARCHE_INDEX],
                ScriptFieldSelector::NAVIGATION_POSITION,
                [100, 200],
            );
            set_pair(
                &mut state,
                objects[WORLD_INDEX],
                ScriptFieldSelector::NAVIGATION_POSITION,
                [10, 20],
            );
            set_pair(
                &mut state,
                objects[ANCHOR_INDEX],
                ScriptFieldSelector::NAVIGATION_POSITION,
                [300, 400],
            );
            set_word(
                &mut state,
                objects[ACTOR_INDEX],
                ScriptFieldSelector::ENCOUNTER_COUNT,
                7,
            );
            set_word(
                &mut state,
                objects[ARCHE_INDEX],
                ScriptFieldSelector::BLACK_HOLE_RELATION,
                100,
            );
            set_word(
                &mut state,
                objects[BLACK_HOLE_INDEX],
                ScriptFieldSelector::BLACK_HOLE_COMPARISON,
                100,
            );
            set_word(
                &mut state,
                objects[BLACK_HOLE_INDEX],
                ScriptFieldSelector::BLACK_HOLE_MATCH_RELATION,
                200,
            );
            set_pair(
                &mut state,
                objects[BLACK_HOLE_INDEX],
                ScriptFieldSelector::BLACK_HOLE_POSITION_A,
                [500, 600],
            );
            set_pair(
                &mut state,
                objects[BLACK_HOLE_INDEX],
                ScriptFieldSelector::BLACK_HOLE_POSITION_B,
                [700, 800],
            );

            Self {
                state,
                records: ScriptActionRecords::default(),
                aboard: AboardObjectRoster::default(),
                requests: PresentationRequestFlags::default(),
                presentation: ScriptPresentationScanState::default(),
                action: ScriptActionState::default(),
                objects,
            }
        }

        fn dispatch(
            &mut self,
            owner_index: usize,
            record: ScriptActionRecord,
            host: &mut MockHost,
        ) -> Result<ScriptActionDispatch, ScriptActionError<&'static str>> {
            let owner = self.objects[owner_index];
            let slot = action_slot(&self.state, owner).unwrap();
            dispatch_script_action(
                ScriptActionContext {
                    state: &mut self.state,
                    records: &mut self.records,
                    aboard_objects: &mut self.aboard,
                    request_flags: &mut self.requests,
                    presentation: &mut self.presentation,
                    action: &mut self.action,
                    owner,
                    slot,
                    player: self.objects[PLAYER_INDEX],
                    navigation: Some(ScriptRecordStateNavigationContext {
                        primary_object: self.objects[LOCATION_INDEX],
                        secondary_object: self.objects[ANCHOR_INDEX],
                        arche: self.objects[ARCHE_INDEX],
                    }),
                },
                record,
                host,
            )
        }
    }

    #[test]
    fn original_oracle_separates_reachable_and_dead_record_arms() {
        let vectors: Vec<ActionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5b38_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let dead_names = vectors
            .iter()
            .filter(|vector| matches!(vector.record_kind, 201 | 205 | 215))
            .map(|vector| vector.name.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            dead_names,
            BTreeSet::from([
                "cd_descript_queues_line_43",
                "cd_restores_related_link_and_replaces_record",
                "c9_clears_matching_reciprocal",
                "c9_preserves_nonmatching_reciprocal",
                "unknown_record_is_ignored",
            ])
        );
        assert_eq!(vectors.len() - dead_names.len(), 27);
    }

    #[test]
    fn navigation_waits_for_arche_and_relinks_before_copying_position() {
        let mut fixture = Fixture::new();
        let mut host = MockHost::default();
        let target = fixture.objects[ANCHOR_INDEX];
        let waiting = fixture
            .dispatch(
                ARCHE_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();
        assert_eq!(waiting, ScriptActionDispatch::default());
        assert_eq!(
            position(&fixture.state, fixture.objects[ARCHE_INDEX]),
            [100, 200]
        );

        fixture.action.navigation_approach_complete = true;
        fixture.action.active_navigation_link = Some(fixture.objects[LOCATION_INDEX]);
        let completed = fixture
            .dispatch(
                ARCHE_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();
        assert_eq!(completed, clear_dispatch());
        assert_eq!(
            position(&fixture.state, fixture.objects[ARCHE_INDEX]),
            [300, 400]
        );
        assert_eq!(
            object_has_flag(
                &fixture.state,
                fixture.objects[LOCATION_INDEX],
                ScriptObjectFlag::Active
            ),
            Some(false)
        );
    }

    #[test]
    fn world_navigation_gates_interface_reset_on_description_lookup() {
        let mut fixture = Fixture::new();
        fixture.action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        fixture.action.current_ship_target = Some(fixture.objects[LOCATION_INDEX]);
        fixture.action.loaded_scene_vertical_offset = 55;
        fixture.action.presentation_words_pending = true;
        fixture.presentation.word_choice_active = true;
        fixture.presentation.c2_gate_active = true;
        fixture.requests = PresentationRequestFlags::decode(3);
        let mut host = MockHost::default();
        let target = fixture.objects[ANCHOR_INDEX];

        fixture
            .dispatch(
                WORLD_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();
        assert_eq!(host.calls, [HostCall::Description(target)]);
        assert_eq!(
            fixture.action.ship_navigation_mode,
            ScriptShipNavigationMode::Active
        );
        assert_eq!(
            position(&fixture.state, fixture.objects[WORLD_INDEX]),
            [300, 400]
        );

        let mut fixture = Fixture::new();
        fixture.action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        fixture.action.current_ship_target = Some(fixture.objects[LOCATION_INDEX]);
        fixture.action.loaded_scene_vertical_offset = 55;
        fixture.action.presentation_words_pending = true;
        fixture.presentation.word_choice_active = true;
        fixture.presentation.c2_gate_active = true;
        fixture.requests = PresentationRequestFlags::decode(3);
        let mut host = MockHost {
            description_available: true,
            ..MockHost::default()
        };
        fixture
            .dispatch(
                WORLD_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();
        assert_eq!(fixture.action.current_ship_target, Some(target));
        assert_eq!(
            fixture.action.ship_navigation_mode,
            ScriptShipNavigationMode::TargetSelected
        );
        assert!(!fixture.action.presentation_words_pending);
        assert!(!fixture.presentation.word_choice_active);
        assert!(!fixture.presentation.c2_gate_active);
        assert_eq!(fixture.requests.bits(), u8::MIN);
        assert!(fixture.action.ship_hud_refresh_requested);
        assert_eq!(fixture.action.scene_vertical_offset, 55);
        assert_eq!(
            fixture.action.active_line,
            Some(ScriptActionPresentationLine::NavigationTarget)
        );
    }

    #[test]
    fn navigation_music_restart_preserves_the_original_position_copy_quirk() {
        let mut fixture = Fixture::new();
        fixture.action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        fixture.action.navigation_music_changed = true;
        let mut host = MockHost {
            description_available: true,
            ..MockHost::default()
        };
        let target = fixture.objects[ANCHOR_INDEX];
        fixture
            .dispatch(
                WORLD_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();
        assert_eq!(
            host.calls,
            [HostCall::Description(target), HostCall::RestartMusic]
        );
        assert_eq!(
            position(&fixture.state, fixture.objects[WORLD_INDEX]),
            [10, 20]
        );
    }

    #[test]
    fn unchanged_world_target_resets_interface_and_clears_primary_pair_without_lookup() {
        let mut fixture = Fixture::new();
        let target = fixture.objects[ANCHOR_INDEX];
        let player = fixture.objects[PLAYER_INDEX];
        let actor = fixture.objects[ACTOR_INDEX];
        fixture.action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        fixture.action.current_ship_target = Some(target);
        let player_slot = action_slot(&fixture.state, player).unwrap();
        let actor_slot = action_slot(&fixture.state, actor).unwrap();
        fixture
            .records
            .set_record(player_slot, ScriptActionRecord::ActorPresentation(actor));
        fixture
            .records
            .set_record(actor_slot, ScriptActionRecord::ActorPresentation(player));
        let mut host = MockHost::default();

        fixture
            .dispatch(
                WORLD_INDEX,
                ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
                &mut host,
            )
            .unwrap();

        assert!(host.calls.is_empty());
        assert_eq!(
            fixture.records.record(player_slot),
            ScriptActionRecord::Empty
        );
        assert_eq!(
            fixture.records.record(actor_slot),
            ScriptActionRecord::Empty
        );
        assert_eq!(
            fixture.action.ship_navigation_mode,
            ScriptShipNavigationMode::TargetSelected
        );
    }

    #[test]
    fn aboard_requests_retain_when_full_and_select_typed_lines_on_success() {
        let mut fixture = Fixture::new();
        for object in fixture.objects.iter().copied().skip(2).take(16) {
            assert!(insert_aboard_object(&mut fixture.aboard, object));
        }
        let actor = fixture.objects[ACTOR_INDEX];
        let mut host = MockHost::default();
        assert_eq!(
            fixture
                .dispatch(
                    PLAYER_INDEX,
                    ScriptActionRecord::AboardRequest(actor),
                    &mut host
                )
                .unwrap(),
            ScriptActionDispatch::default()
        );
        assert_ne!(
            holder(&fixture.state, actor),
            Some(ScriptStateObjectReference::Sentinel)
        );

        for object in fixture.objects.iter().copied().skip(2).take(16) {
            remove_aboard_object(&mut fixture.aboard, object);
        }
        assert_eq!(
            fixture
                .dispatch(
                    PLAYER_INDEX,
                    ScriptActionRecord::AboardRequest(actor),
                    &mut host
                )
                .unwrap(),
            clear_dispatch()
        );
        assert_eq!(
            holder(&fixture.state, actor),
            Some(ScriptStateObjectReference::Sentinel)
        );
        assert_eq!(
            fixture.action.active_line,
            Some(ScriptActionPresentationLine::CharacterAboard)
        );

        let mut fixture = Fixture::new();
        let inventory = fixture.objects[INVENTORY_INDEX];
        let mut host = MockHost {
            description_available: true,
            ..MockHost::default()
        };
        fixture
            .dispatch(
                PLAYER_INDEX,
                ScriptActionRecord::AboardRequest(inventory),
                &mut host,
            )
            .unwrap();
        assert_eq!(host.calls, [HostCall::Description(inventory)]);
        assert!(fixture.requests.secondary_request_pending());
        assert_eq!(
            fixture.action.active_line,
            Some(ScriptActionPresentationLine::InventoryAboard)
        );
    }

    #[test]
    fn presentation_queue_promotes_objects_and_drives_wildcard_radio_state() {
        let mut fixture = Fixture::new();
        let actor = fixture.objects[ACTOR_INDEX];
        let owner = fixture.objects[PLAYER_INDEX];
        let owner_slot = action_slot(&fixture.state, owner).unwrap();
        let mut host = MockHost::default();
        fixture
            .dispatch(
                PLAYER_INDEX,
                ScriptActionRecord::PresentationQueue(actor),
                &mut host,
            )
            .unwrap();
        assert_eq!(
            fixture.records.record(owner_slot),
            ScriptActionRecord::ActorPresentation(actor)
        );

        let player = fixture.objects[PLAYER_INDEX];
        fixture.presentation.name_lookup_enabled = true;
        fixture.action.voc_playback_enabled = false;
        fixture
            .dispatch(
                ACTOR_INDEX,
                ScriptActionRecord::PresentationQueue(player),
                &mut host,
            )
            .unwrap();
        assert_eq!(fixture.action.pending_presentation_owner, Some(actor));
        assert!(fixture.action.radio_clip_enable_requested);
        assert!(fixture.action.radio_clip_playing);
        assert_eq!(host.calls, [HostCall::PlayRadio]);
        fixture
            .dispatch(
                ACTOR_INDEX,
                ScriptActionRecord::PresentationQueue(player),
                &mut host,
            )
            .unwrap();
        assert_eq!(host.calls, [HostCall::PlayRadio]);
    }

    #[test]
    fn actor_presentation_updates_counter_runs_cod_and_writes_reciprocal() {
        let mut fixture = Fixture::new();
        let player = fixture.objects[PLAYER_INDEX];
        let actor = fixture.objects[ACTOR_INDEX];
        fixture.action.pending_presentation_owner = Some(actor);
        let mut host = MockHost::default();
        let dispatch = fixture
            .dispatch(
                PLAYER_INDEX,
                ScriptActionRecord::ActorPresentation(actor),
                &mut host,
            )
            .unwrap();
        assert_eq!(dispatch.disposition, ScriptActionDisposition::Suppress);
        assert_eq!(encounter_count(&fixture.state, actor), 8);
        assert_eq!(fixture.action.post_update_object, Some(actor));
        assert_eq!(host.calls, [HostCall::ExecuteCode(actor)]);
        let reciprocal = action_slot(&fixture.state, actor).unwrap();
        assert_eq!(
            fixture.records.record(reciprocal),
            ScriptActionRecord::ActorPresentation(player)
        );
        assert!(!fixture.records.is_actionable(reciprocal));
        assert_eq!(fixture.action.pending_presentation_owner, None);

        fixture.presentation.pair_write_disabled = true;
        let previous_counter = encounter_count(&fixture.state, actor);
        let guarded = fixture
            .dispatch(
                ACTOR_INDEX,
                ScriptActionRecord::ActorPresentation(player),
                &mut host,
            )
            .unwrap();
        assert_eq!(guarded, ScriptActionDispatch::default());
        assert_eq!(encounter_count(&fixture.state, actor), previous_counter);
    }

    #[test]
    fn actor_related_and_nonactor_c4_paths_preserve_native_update_order() {
        let mut fixture = Fixture::new();
        let player = fixture.objects[PLAYER_INDEX];
        let actor = fixture.objects[ACTOR_INDEX];
        let mut host = MockHost::default();
        let dispatch = fixture
            .dispatch(
                ACTOR_INDEX,
                ScriptActionRecord::ActorPresentation(player),
                &mut host,
            )
            .unwrap();
        assert_eq!(dispatch.disposition, ScriptActionDisposition::Suppress);
        assert_eq!(encounter_count(&fixture.state, actor), 8);
        assert_eq!(host.calls, [HostCall::ExecuteCode(actor)]);
        let player_slot = action_slot(&fixture.state, player).unwrap();
        assert_eq!(
            fixture.records.record(player_slot),
            ScriptActionRecord::ActorPresentation(actor)
        );

        let mut fixture = Fixture::new();
        let world = fixture.objects[WORLD_INDEX];
        let navigation_entity = fixture.objects[ARCHE_INDEX];
        let mut host = MockHost::default();
        fixture
            .dispatch(
                WORLD_INDEX,
                ScriptActionRecord::ActorPresentation(navigation_entity),
                &mut host,
            )
            .unwrap();
        assert!(host.calls.is_empty());
        let navigation_slot = action_slot(&fixture.state, navigation_entity).unwrap();
        assert_eq!(
            fixture.records.record(navigation_slot),
            ScriptActionRecord::ActorPresentation(world)
        );
        assert!(!fixture.records.is_actionable(navigation_slot));
    }

    #[test]
    fn travel_advances_three_phases_and_commits_matching_position_pair() {
        let mut fixture = Fixture::new();
        fixture.action.travel_actor_busy = true;
        fixture.action.camera_view_active = true;
        fixture.presentation.ui_busy = true;
        let black_hole = fixture.objects[BLACK_HOLE_INDEX];
        let mut host = MockHost::default();
        let record = ScriptActionRecord::Travel(black_hole);

        assert_eq!(
            fixture.dispatch(ARCHE_INDEX, record, &mut host).unwrap(),
            ScriptActionDispatch::default()
        );
        assert_eq!(
            fixture.action.travel_phase,
            ScriptTravelActionPhase::WaitingForCamera
        );
        assert!(fixture.action.camera_transition_in_progress);
        assert_eq!(host.calls, [HostCall::StartCamera]);

        fixture.action.camera_transition_in_progress = false;
        fixture.dispatch(ARCHE_INDEX, record, &mut host).unwrap();
        assert_eq!(
            fixture.action.travel_phase,
            ScriptTravelActionPhase::WaitingForPresentation
        );
        assert!(!fixture.action.travel_actor_busy);
        assert!(fixture.action.travel_actor_clear_requested);
        assert!(!fixture.action.camera_view_active);
        assert_eq!(
            fixture.action.active_line,
            Some(ScriptActionPresentationLine::TravelReady)
        );

        fixture.presentation.c2_gate_active = true;
        assert_eq!(
            fixture.dispatch(ARCHE_INDEX, record, &mut host).unwrap(),
            ScriptActionDispatch::default()
        );
        fixture.presentation.c2_gate_active = false;
        assert_eq!(
            fixture.dispatch(ARCHE_INDEX, record, &mut host).unwrap(),
            clear_dispatch()
        );
        assert_eq!(host.calls, [HostCall::StartCamera, HostCall::ResetHud]);
        assert_eq!(
            fixture.action.travel_phase,
            ScriptTravelActionPhase::WaitingForActor
        );
        assert!(fixture.action.screen_rebuild_requested);
        assert!(!fixture.presentation.ui_busy);
        assert_eq!(
            position(&fixture.state, fixture.objects[ARCHE_INDEX]),
            [700, 800]
        );
        assert_eq!(relation(&fixture.state, fixture.objects[ARCHE_INDEX]), 200);
    }

    #[test]
    fn travel_uses_first_position_pair_when_relation_does_not_match() {
        let mut fixture = Fixture::new();
        fixture.action.travel_phase = ScriptTravelActionPhase::WaitingForPresentation;
        set_word(
            &mut fixture.state,
            fixture.objects[ARCHE_INDEX],
            ScriptFieldSelector::BLACK_HOLE_RELATION,
            99,
        );
        let black_hole = fixture.objects[BLACK_HOLE_INDEX];
        let mut host = MockHost::default();
        fixture
            .dispatch(
                ARCHE_INDEX,
                ScriptActionRecord::Travel(black_hole),
                &mut host,
            )
            .unwrap();
        assert_eq!(
            position(&fixture.state, fixture.objects[ARCHE_INDEX]),
            [500, 600]
        );
        assert_eq!(relation(&fixture.state, fixture.objects[ARCHE_INDEX]), 100);
    }

    #[test]
    fn non_dispatch_record_families_retain_without_host_work() {
        let mut fixture = Fixture::new();
        let related = fixture.objects[AUXILIARY_INDEX];
        let mut host = MockHost::default();
        for record in [
            ScriptActionRecord::Empty,
            ScriptActionRecord::WorldStateLink(related),
            ScriptActionRecord::ActiveObjectLink(related),
            ScriptActionRecord::OpaqueMarker(17),
            ScriptActionRecord::Occupied,
        ] {
            assert_eq!(
                fixture.dispatch(WORLD_INDEX, record, &mut host).unwrap(),
                ScriptActionDispatch::default()
            );
        }
        assert!(host.calls.is_empty());
    }

    fn field(
        state: &ScriptState,
        object: ScriptObjectId,
        selector: ScriptFieldSelector,
    ) -> commander_blood_formats::script::ScriptStateWord {
        object_word::<&'static str>(state, object, selector).unwrap()
    }

    fn pair(
        state: &ScriptState,
        object: ScriptObjectId,
        selector: ScriptFieldSelector,
    ) -> ScriptStateWordPair {
        object_word_pair::<&'static str>(state, object, selector).unwrap()
    }

    fn set_reference(state: &mut ScriptState, object: ScriptObjectId, related: ScriptObjectId) {
        let target = field(state, object, ScriptFieldSelector::HOLDER_OR_LOCATION);
        write_object_reference::<&'static str>(state, target, related).unwrap();
    }

    fn set_word(
        state: &mut ScriptState,
        object: ScriptObjectId,
        selector: ScriptFieldSelector,
        value: u16,
    ) {
        let target = field(state, object, selector);
        assert!(state.set_word(target, value));
    }

    fn set_pair(
        state: &mut ScriptState,
        object: ScriptObjectId,
        selector: ScriptFieldSelector,
        value: [u16; 2],
    ) {
        let target = pair(state, object, selector);
        assert!(state.set_word_pair(target, value));
    }

    fn holder(state: &ScriptState, object: ScriptObjectId) -> Option<ScriptStateObjectReference> {
        state.object_reference(field(
            state,
            object,
            ScriptFieldSelector::HOLDER_OR_LOCATION,
        ))
    }

    fn position(state: &ScriptState, object: ScriptObjectId) -> [u16; 2] {
        state
            .word_pair(pair(
                state,
                object,
                ScriptFieldSelector::NAVIGATION_POSITION,
            ))
            .unwrap()
    }

    fn encounter_count(state: &ScriptState, object: ScriptObjectId) -> u16 {
        state
            .word(field(state, object, ScriptFieldSelector::ENCOUNTER_COUNT))
            .unwrap()
    }

    fn relation(state: &ScriptState, object: ScriptObjectId) -> u16 {
        state
            .word(field(
                state,
                object,
                ScriptFieldSelector::BLACK_HOLE_RELATION,
            ))
            .unwrap()
    }
}
