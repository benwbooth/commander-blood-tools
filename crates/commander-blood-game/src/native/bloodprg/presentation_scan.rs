//! Post-frame coordination of BloodScript presentation and action records.

use std::fmt;

use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::ScriptRecordStateOperand;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateWordTriple,
};

use super::record_state::action_slot;
use super::vm::set_object_flag;
use super::{
    ScriptActionRecord, ScriptActionRecords, ScriptFieldSelector, ScriptObjectFlag,
    ScriptProfileRecordState, ScriptRuntime, ScriptSelectorState, object_has_flag,
    script_field_offset,
};

const SELECTOR_YIELD_PREFIX_SIZE: u16 = 1;

const SERIALIZED_WORD_SIZE: usize = std::mem::size_of::<u16>();

/// Record kind retained when save restoration provides no related object yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptDeferredRecordKind {
    /// C1 navigation action.
    Navigation,
    /// C2 aboard-object request.
    AboardRequest,
    /// C3 presentation queue.
    PresentationQueue,
    /// C4 actor-presentation action.
    ActorPresentation,
    /// C5 world-state link.
    WorldStateLink,
    /// C6 travel relation.
    Travel,
    /// C7 active-object link.
    ActiveObjectLink,
    /// C8 opaque marker.
    OpaqueMarker,
    /// Another native record kind outside the translated action family.
    Other,
}

/// Deferred record state drained after player-presentation maintenance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptDeferredRecord {
    /// No deferred update is pending.
    #[default]
    Empty,
    /// A complete typed record is ready to be installed.
    Complete {
        /// Record written to the player or navigation-arche action slot.
        record: ScriptActionRecord,
        /// Whether the post-frame action dispatcher may run it immediately.
        actionable: bool,
    },
    /// A restored related object is waiting for its record kind.
    RelatedOnly(ScriptObjectId),
    /// A restored record kind is waiting for its related object.
    KindOnly(ScriptDeferredRecordKind),
}

/// Fixed renderer entities advanced when a presentation session ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptPresentationEntity {
    /// Original entity 4, which owns the active presentation line overlay.
    DialogueOverlay,
    /// Original entity 2, used by the name-area transition effect.
    NameAreaEffect,
}

/// Mutable global state owned by the post-frame presentation coordinator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptPresentationScanState {
    /// A player presentation currently owns the dialogue UI.
    pub active: bool,
    /// A C2 operation temporarily blocks character handoff.
    pub c2_gate_active: bool,
    /// Progressive word choice currently blocks character handoff.
    pub word_choice_active: bool,
    /// A presentation has started but has not released its handoff lock.
    pub start_locked: bool,
    /// UI mode requests a descriptor lookup when a player presentation starts.
    pub name_lookup_enabled: bool,
    /// The name-area visual effect is enabled for the current UI mode.
    pub name_area_effect_active: bool,
    /// The name-area effect must restart with newly loaded content.
    pub name_area_effect_restart_requested: bool,
    /// The palette must be republished after a presentation starts.
    pub palette_dirty: bool,
    /// Presentation status changed during this scan.
    pub status_changed: bool,
    /// Dialogue UI is occupied by an active presentation.
    pub ui_busy: bool,
    /// Input is currently admitted by the presentation owner.
    pub input_enabled: bool,
    /// Presentation text is waiting on its next scheduler phase.
    pub text_wait_active: bool,
    /// Presentation hold state has reached its ready boundary.
    pub hold_ready: bool,
    /// Dialogue hold completion has been observed.
    pub dialogue_hold_complete: bool,
    /// Related object carried the native presentable flag at presentation start.
    pub related_presentable: bool,
    /// Current player-side presentation owner, cleared on a fresh start.
    pub owner: Option<ScriptObjectId>,
    /// A 5B38 action may disable reciprocal writes for later scan entries.
    pub pair_write_disabled: bool,
    /// Deferred action-record update supplied by an actor state machine.
    pub deferred: ScriptDeferredRecord,
}

/// Stable profile bindings and mutable stores used by one scan.
pub struct ScriptPresentationScanContext<'a, Records = ScriptActionRecords> {
    /// Active profile object state.
    pub state: &'a mut ScriptState,
    /// Profile record store owning the typed C1 through C9 action slots.
    pub records: &'a mut Records,
    /// Main BloodScript control-flow state.
    pub runtime: &'a mut ScriptRuntime,
    /// Dialogue branches and recent concept history.
    pub selector: &'a mut ScriptSelectorState,
    /// Presentation coordinator globals.
    pub presentation: &'a mut ScriptPresentationScanState,
    /// Built-in player object named `blood`.
    pub player: ScriptObjectId,
    /// Built-in navigation entity named `arche`.
    pub arche: ScriptObjectId,
}

/// Mutable profile state supplied to one inline BAS dialogue handoff.
pub struct ScriptDialogueControlDispatchContext<'a, Records = ScriptActionRecords> {
    /// Active VAR image.
    pub state: &'a mut ScriptState,
    /// Shared COD and BAS control-flow state.
    pub runtime: &'a mut ScriptRuntime,
    /// Dialogue branches and recent concept history.
    pub selector: &'a mut ScriptSelectorState,
    /// Complete record store shared with BAS C9 and CD instructions.
    pub records: &'a mut Records,
    /// Presentation globals that inline BAS instructions can change immediately.
    pub presentation: &'a mut ScriptPresentationScanState,
    /// Character entering dialogue control.
    pub actor: ScriptObjectId,
    /// First selector node in the character's authored BAS response list.
    pub selector_root: ScriptCodeOffset,
}

/// Mutable profile state supplied to one inline C1-through-C8 action dispatch.
pub struct ScriptRecordActionDispatchContext<'a, Records = ScriptActionRecords> {
    /// Active VAR image.
    pub state: &'a mut ScriptState,
    /// Complete record store owning the typed action triples.
    pub records: &'a mut Records,
    /// Presentation coordinator state shared with the surrounding scan.
    pub presentation: &'a mut ScriptPresentationScanState,
    /// Object owning the actionable slot.
    pub owner: ScriptObjectId,
    /// Typed action slot.
    pub slot: ScriptStateWordTriple,
    /// Record observed after any deferred overwrite.
    pub record: ScriptActionRecord,
}

/// Action-record disposition returned by the translated 5B38 dispatcher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptActionDisposition {
    /// Leave the record available for a later frame.
    #[default]
    Retain,
    /// Keep the record but suppress further post-frame dispatch.
    Suppress,
    /// Remove the action record from its slot.
    Clear,
}

/// Effects returned after one action-record dispatch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptActionDispatch {
    /// Resulting lifetime of the dispatched record.
    pub disposition: ScriptActionDisposition,
    /// Whether later entries must skip reciprocal actor-pair writes.
    pub disable_pair_writes: bool,
}

/// Semantic callbacks reached by the presentation scan.
pub trait ScriptPresentationScanHost<Records = ScriptActionRecords> {
    /// Typed callback failure.
    type Error;

    /// Enter a character's authored BAS dialogue selector.
    fn dispatch_dialogue_control(
        &mut self,
        context: ScriptDialogueControlDispatchContext<'_, Records>,
    ) -> Result<(), Self::Error>;

    /// Resolve presentation assets for the related object name.
    fn lookup_presentation_description(
        &mut self,
        related: ScriptObjectId,
    ) -> Result<(), Self::Error>;

    /// Load and place the name-area restart effect.
    fn restart_name_area_effect(&mut self) -> Result<(), Self::Error>;

    /// Advance one fixed presentation renderer entity to its next state.
    fn transition_presentation_entity(
        &mut self,
        entity: ScriptPresentationEntity,
    ) -> Result<(), Self::Error>;

    /// Dispatch one actionable C1-through-C8 record through the 5B38 ladder.
    fn dispatch_record_action(
        &mut self,
        context: ScriptRecordActionDispatchContext<'_, Records>,
    ) -> Result<ScriptActionDispatch, Self::Error>;
}

/// Access to the action triples required by the recovered presentation scan.
pub trait ScriptActionRecordStore {
    /// Borrow the typed C1 through C9 action slots.
    fn action_records(&self) -> &ScriptActionRecords;

    /// Mutably borrow the typed C1 through C9 action slots.
    fn action_records_mut(&mut self) -> &mut ScriptActionRecords;
}

impl ScriptActionRecordStore for ScriptActionRecords {
    fn action_records(&self) -> &ScriptActionRecords {
        self
    }

    fn action_records_mut(&mut self) -> &mut ScriptActionRecords {
        self
    }
}

impl ScriptActionRecordStore for ScriptProfileRecordState {
    fn action_records(&self) -> &ScriptActionRecords {
        &self.action_records
    }

    fn action_records_mut(&mut self) -> &mut ScriptActionRecords {
        &mut self.action_records
    }
}

/// One character dialogue handoff emitted during a scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptPresentationHandoff {
    /// Character object entering dialogue control.
    pub actor: ScriptObjectId,
    /// Authored BAS selector-list root.
    pub selector_root: ScriptCodeOffset,
}

/// One action record dispatched in directory order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptPresentationAction {
    /// Object owning the action slot.
    pub owner: ScriptObjectId,
    /// Typed action slot.
    pub slot: ScriptStateWordTriple,
    /// Record observed after any deferred overwrite.
    pub record: ScriptActionRecord,
}

/// Observable result of one post-frame presentation scan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptPresentationScanOutcome {
    /// Number of decoded active-directory objects visited.
    pub processed_objects: usize,
    /// Character handoffs dispatched in directory order.
    pub handoffs: Vec<ScriptPresentationHandoff>,
    /// Action records dispatched in directory order.
    pub actions: Vec<ScriptPresentationAction>,
    /// Related object that started a fresh player presentation.
    pub presentation_started: Option<ScriptObjectId>,
    /// Whether an existing player presentation ended.
    pub presentation_ended: bool,
    /// A fresh presentation cleared the bridge-console selection alias at `0x2A19`.
    pub bridge_console_selection_cleared: bool,
    /// Slot receiving a complete deferred record.
    pub deferred_destination: Option<ScriptStateWordTriple>,
}

/// Invalid typed profile state or host failure during presentation coordination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptPresentationScanError<HostError> {
    /// A supported object kind has no action field in its decoded record.
    MissingActionSlot {
        /// Object lacking its required slot.
        object: ScriptObjectId,
    },
    /// A character lacks its presentation-handoff field.
    MissingHandoffField {
        /// Character lacking the field.
        object: ScriptObjectId,
    },
    /// A typed record references an object absent from this profile.
    MissingObject {
        /// Missing stable object identity.
        object: ScriptObjectId,
    },
    /// A required object-header flag could not be updated.
    ObjectFlagUpdate {
        /// Object whose flag write failed.
        object: ScriptObjectId,
    },
    /// Dialogue, asset, renderer, or action dispatch failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for ScriptPresentationScanError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HostError: fmt::Debug> std::error::Error for ScriptPresentationScanError<HostError> {}

/// Scan all active profile objects after one COD execution pass.
///
/// This translates `presentation_scan` at BLOODPRG file offset `0x005816`.
/// Stable object identities, typed action records, and explicit coordinator
/// state replace the DEB pointer walk, selector arithmetic, and packed globals.
pub fn scan_script_presentations<Records, Host>(
    context: ScriptPresentationScanContext<'_, Records>,
    host: &mut Host,
) -> Result<ScriptPresentationScanOutcome, ScriptPresentationScanError<Host::Error>>
where
    Records: ScriptActionRecordStore,
    Host: ScriptPresentationScanHost<Records>,
{
    let ScriptPresentationScanContext {
        state,
        records,
        runtime,
        selector,
        presentation,
        player,
        arche,
    } = context;
    let mut outcome = ScriptPresentationScanOutcome::default();
    presentation.pair_write_disabled = false;
    let objects: Vec<(ScriptObjectId, ScriptObjectKind)> = state
        .objects()
        .iter()
        .map(|object| (object.id, object.kind))
        .collect();

    for (object, kind) in objects {
        outcome.processed_objects += 1;
        if object_has_flag(state, object, ScriptObjectFlag::Active) != Some(true) {
            continue;
        }
        let run_action = match kind {
            ScriptObjectKind::Actor => {
                scan_character_handoff(
                    CharacterHandoffContext {
                        state,
                        records,
                        runtime,
                        selector,
                        presentation,
                        player,
                    },
                    object,
                    host,
                    &mut outcome,
                )?;
                true
            }
            ScriptObjectKind::NavigationEntity | ScriptObjectKind::WorldState => true,
            ScriptObjectKind::Player => {
                scan_player_presentation(
                    PlayerPresentationContext {
                        state,
                        records,
                        runtime,
                        selector,
                        presentation,
                    },
                    object,
                    host,
                    &mut outcome,
                )?;
                if let Some(destination) =
                    drain_deferred_record(state, records, presentation, object, arche)?
                {
                    outcome.deferred_destination = Some(destination);
                }
                true
            }
            _ => false,
        };

        if run_action {
            dispatch_action(state, records, presentation, object, host, &mut outcome)?;
        }
    }

    Ok(outcome)
}

struct CharacterHandoffContext<'a, Records> {
    state: &'a mut ScriptState,
    records: &'a mut Records,
    runtime: &'a mut ScriptRuntime,
    selector: &'a mut ScriptSelectorState,
    presentation: &'a mut ScriptPresentationScanState,
    player: ScriptObjectId,
}

fn scan_character_handoff<Records, Host>(
    context: CharacterHandoffContext<'_, Records>,
    actor: ScriptObjectId,
    host: &mut Host,
    outcome: &mut ScriptPresentationScanOutcome,
) -> Result<(), ScriptPresentationScanError<Host::Error>>
where
    Records: ScriptActionRecordStore,
    Host: ScriptPresentationScanHost<Records>,
{
    let CharacterHandoffContext {
        state,
        records,
        runtime,
        selector,
        presentation,
        player,
    } = context;
    if !presentation.active
        || presentation.c2_gate_active
        || presentation.word_choice_active
        || presentation.start_locked
        || object_has_flag(state, actor, ScriptObjectFlag::PresentationBlocked) != Some(false)
    {
        return Ok(());
    }
    let primary_slot = action_slot(state, player)
        .ok_or(ScriptPresentationScanError::MissingActionSlot { object: player })?;
    if !matches!(
        records.action_records().record(primary_slot),
        ScriptActionRecord::ActorPresentation(_)
    ) {
        return Ok(());
    }
    let slot = action_slot(state, actor)
        .ok_or(ScriptPresentationScanError::MissingActionSlot { object: actor })?;
    if records.action_records().record(slot) != ScriptActionRecord::ActorPresentation(player) {
        return Ok(());
    }
    let field_offset = script_field_offset(
        ScriptObjectKind::Actor,
        ScriptFieldSelector::PRESENTATION_HANDOFF,
    )
    .ok_or(ScriptPresentationScanError::MissingHandoffField { object: actor })?;
    let field = state
        .object_word(actor, field_offset / SERIALIZED_WORD_SIZE)
        .ok_or(ScriptPresentationScanError::MissingHandoffField { object: actor })?;
    let target = state
        .word(field)
        .ok_or(ScriptPresentationScanError::MissingHandoffField { object: actor })?;
    if target == u16::MIN {
        return Ok(());
    }
    let selector_root =
        ScriptCodeOffset::new(usize::from(target.wrapping_add(SELECTOR_YIELD_PREFIX_SIZE)));
    host.dispatch_dialogue_control(ScriptDialogueControlDispatchContext {
        state,
        runtime,
        selector,
        records,
        presentation,
        actor,
        selector_root,
    })
    .map_err(ScriptPresentationScanError::Host)?;
    outcome.handoffs.push(ScriptPresentationHandoff {
        actor,
        selector_root,
    });
    Ok(())
}

struct PlayerPresentationContext<'a, Records> {
    state: &'a mut ScriptState,
    records: &'a Records,
    runtime: &'a mut ScriptRuntime,
    selector: &'a mut ScriptSelectorState,
    presentation: &'a mut ScriptPresentationScanState,
}

fn scan_player_presentation<Records, Host>(
    context: PlayerPresentationContext<'_, Records>,
    player: ScriptObjectId,
    host: &mut Host,
    outcome: &mut ScriptPresentationScanOutcome,
) -> Result<(), ScriptPresentationScanError<Host::Error>>
where
    Records: ScriptActionRecordStore,
    Host: ScriptPresentationScanHost<Records>,
{
    let PlayerPresentationContext {
        state,
        records,
        runtime,
        selector,
        presentation,
    } = context;
    let slot = action_slot(state, player)
        .ok_or(ScriptPresentationScanError::MissingActionSlot { object: player })?;
    if let ScriptActionRecord::ActorPresentation(related) = records.action_records().record(slot) {
        if state.object(related).is_none() {
            return Err(ScriptPresentationScanError::MissingObject { object: related });
        }
        presentation.related_presentable =
            object_has_flag(state, related, ScriptObjectFlag::Presentable)
                .ok_or(ScriptPresentationScanError::MissingObject { object: related })?;
        if !presentation.active {
            presentation.palette_dirty = true;
            presentation.status_changed = true;
            presentation.active = true;
            presentation.ui_busy = true;
            presentation.input_enabled = false;
            presentation.text_wait_active = false;
            presentation.word_choice_active = false;
            presentation.hold_ready = false;
            presentation.dialogue_hold_complete = false;
            presentation.owner = None;
            presentation.start_locked = true;
            outcome.bridge_console_selection_cleared = true;
            selector.clear_presentation_branches();
            if !set_object_flag(state, related, ScriptObjectFlag::PresentationBlocked, true) {
                return Err(ScriptPresentationScanError::ObjectFlagUpdate { object: related });
            }
            if presentation.name_lookup_enabled {
                host.lookup_presentation_description(related)
                    .map_err(ScriptPresentationScanError::Host)?;
                if presentation.name_area_effect_active {
                    presentation.name_area_effect_restart_requested = true;
                    host.restart_name_area_effect()
                        .map_err(ScriptPresentationScanError::Host)?;
                }
            }
            outcome.presentation_started = Some(related);
        }
    } else if presentation.active {
        presentation.status_changed = true;
        presentation.active = false;
        presentation.ui_busy = false;
        presentation.start_locked = false;
        presentation.word_choice_active = false;
        presentation.name_area_effect_active = false;
        presentation.owner = None;
        runtime.clear_alternate_resume_state();
        selector.clear_presentation_branches();
        selector.clear_concept_history();
        host.transition_presentation_entity(ScriptPresentationEntity::DialogueOverlay)
            .map_err(ScriptPresentationScanError::Host)?;
        host.transition_presentation_entity(ScriptPresentationEntity::NameAreaEffect)
            .map_err(ScriptPresentationScanError::Host)?;
        outcome.presentation_ended = true;
    }
    Ok(())
}

fn drain_deferred_record<Records, HostError>(
    state: &ScriptState,
    records: &mut Records,
    presentation: &mut ScriptPresentationScanState,
    player: ScriptObjectId,
    arche: ScriptObjectId,
) -> Result<Option<ScriptStateWordTriple>, ScriptPresentationScanError<HostError>>
where
    Records: ScriptActionRecordStore,
{
    let ScriptDeferredRecord::Complete { record, actionable } = presentation.deferred else {
        return Ok(None);
    };
    let target = if matches!(
        record,
        ScriptActionRecord::Navigation(_) | ScriptActionRecord::Travel(_)
    ) {
        action_slot(state, arche)
            .ok_or(ScriptPresentationScanError::MissingActionSlot { object: arche })?
    } else {
        action_slot(state, player)
            .ok_or(ScriptPresentationScanError::MissingActionSlot { object: player })?
    };
    records.action_records_mut().set_record(target, record);
    records
        .action_records_mut()
        .set_actionable(target, actionable);
    presentation.deferred = ScriptDeferredRecord::Empty;
    Ok(Some(target))
}

fn dispatch_action<Records, Host>(
    state: &mut ScriptState,
    records: &mut Records,
    presentation: &mut ScriptPresentationScanState,
    owner: ScriptObjectId,
    host: &mut Host,
    outcome: &mut ScriptPresentationScanOutcome,
) -> Result<(), ScriptPresentationScanError<Host::Error>>
where
    Records: ScriptActionRecordStore,
    Host: ScriptPresentationScanHost<Records>,
{
    let slot = action_slot(state, owner)
        .ok_or(ScriptPresentationScanError::MissingActionSlot { object: owner })?;
    if !records.action_records().is_actionable(slot) {
        return Ok(());
    }
    let record = records.action_records().record(slot);
    let dispatch = host
        .dispatch_record_action(ScriptRecordActionDispatchContext {
            state,
            records,
            presentation,
            owner,
            slot,
            record,
        })
        .map_err(ScriptPresentationScanError::Host)?;
    match dispatch.disposition {
        ScriptActionDisposition::Retain => {}
        ScriptActionDisposition::Suppress => {
            records.action_records_mut().set_actionable(slot, false)
        }
        ScriptActionDisposition::Clear => records
            .action_records_mut()
            .set_record(slot, ScriptActionRecord::Empty),
    }
    presentation.pair_write_disabled |= dispatch.disable_pair_writes;
    outcome.actions.push(ScriptPresentationAction {
        owner,
        slot,
        record,
    });
    Ok(())
}

/// Build a complete deferred C1 record for a navigation target.
pub const fn deferred_navigation_record(
    target: ScriptObjectId,
    actionable: bool,
) -> ScriptDeferredRecord {
    ScriptDeferredRecord::Complete {
        record: ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
        actionable,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::script::{
        ScriptDirectory, decode_script_dictionary, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 16;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_ACTIVE_KIND: u16 = 1;
    const TEST_SELECTOR_PREFIX: usize = 51;
    const TEST_SELECTOR_ROOT: usize = 52;
    const PLAYER_INDEX: usize = 0;
    const ACTOR_INDEX: usize = 1;
    const ARCHE_INDEX: usize = 2;
    const WORLD_INDEX: usize = 3;
    const AUXILIARY_INDEX: usize = 4;

    #[derive(Deserialize)]
    struct ScanOracle {
        name: String,
        processed_entries: Vec<usize>,
        calls: Vec<OracleCall>,
        presentation_active_after: u8,
        deferred_after: [u16; 3],
    }

    #[derive(Deserialize)]
    struct OracleCall {
        name: String,
        object_id: Option<u16>,
    }

    #[derive(Default)]
    struct RecordingHost {
        events: Vec<&'static str>,
        selector_roots: Vec<ScriptCodeOffset>,
    }

    impl ScriptPresentationScanHost for RecordingHost {
        type Error = std::convert::Infallible;

        fn dispatch_dialogue_control(
            &mut self,
            context: ScriptDialogueControlDispatchContext<'_>,
        ) -> Result<(), Self::Error> {
            self.events.push("control");
            self.selector_roots.push(context.selector_root);
            Ok(())
        }

        fn lookup_presentation_description(
            &mut self,
            _related: ScriptObjectId,
        ) -> Result<(), Self::Error> {
            self.events.push("lookup");
            Ok(())
        }

        fn restart_name_area_effect(&mut self) -> Result<(), Self::Error> {
            self.events.push("restart");
            Ok(())
        }

        fn transition_presentation_entity(
            &mut self,
            entity: ScriptPresentationEntity,
        ) -> Result<(), Self::Error> {
            self.events.push(match entity {
                ScriptPresentationEntity::DialogueOverlay => "dialogue_transition",
                ScriptPresentationEntity::NameAreaEffect => "effect_transition",
            });
            Ok(())
        }

        fn dispatch_record_action(
            &mut self,
            _context: ScriptRecordActionDispatchContext<'_>,
        ) -> Result<ScriptActionDispatch, Self::Error> {
            self.events.push("action");
            Ok(ScriptActionDispatch::default())
        }
    }

    struct Fixture {
        state: ScriptState,
        records: ScriptActionRecords,
        runtime: ScriptRuntime,
        selector: ScriptSelectorState,
        presentation: ScriptPresentationScanState,
        objects: [ScriptObjectId; 5],
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn fixture() -> Fixture {
        let kinds = [
            ScriptObjectKind::Player,
            ScriptObjectKind::Actor,
            ScriptObjectKind::NavigationEntity,
            ScriptObjectKind::WorldState,
            ScriptObjectKind::Auxiliary,
        ];
        let mut directory_bytes = Vec::new();
        let mut state_bytes = Vec::new();
        for (index, kind) in kinds.into_iter().enumerate() {
            let source_offset = state_bytes.len();
            let mut entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
            let name = format!("object{index}");
            entry[..name.len()].copy_from_slice(name.as_bytes());
            entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE]
                .copy_from_slice(&u16::try_from(source_offset).unwrap().to_le_bytes());
            entry[DIRECTORY_NAME_CAPACITY + SERIALIZED_WORD_SIZE..]
                .copy_from_slice(&DIRECTORY_ACTIVE_KIND.to_le_bytes());
            directory_bytes.extend_from_slice(&entry);

            let mut object = vec![u8::MIN; kind.record_size()];
            object[..SERIALIZED_WORD_SIZE].copy_from_slice(&kind.mask().to_le_bytes());
            state_bytes.extend_from_slice(&object);
        }
        directory_bytes.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
        let directory = decode_script_directory(&directory_bytes).unwrap();
        let state = decode_script_state(&state_bytes, &directory).unwrap();
        let objects: [ScriptObjectId; 5] = state
            .objects()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        Fixture {
            state,
            records: ScriptActionRecords::default(),
            runtime: ScriptRuntime::new(),
            selector: ScriptSelectorState::default(),
            presentation: ScriptPresentationScanState::default(),
            objects,
        }
    }

    fn slot(fixture: &Fixture, index: usize) -> ScriptStateWordTriple {
        action_slot(&fixture.state, fixture.objects[index]).unwrap()
    }

    fn set_active(fixture: &mut Fixture, index: usize, active: bool) {
        assert!(set_object_flag(
            &mut fixture.state,
            fixture.objects[index],
            ScriptObjectFlag::Active,
            active,
        ));
    }

    fn set_handoff_target(fixture: &mut Fixture, target: usize) {
        let actor = fixture.objects[ACTOR_INDEX];
        let offset = script_field_offset(
            ScriptObjectKind::Actor,
            ScriptFieldSelector::PRESENTATION_HANDOFF,
        )
        .unwrap();
        let field = fixture
            .state
            .object_word(actor, offset / SERIALIZED_WORD_SIZE)
            .unwrap();
        assert!(
            fixture
                .state
                .set_word(field, u16::try_from(target).unwrap())
        );
    }

    fn configure_vector(name: &str, fixture: &mut Fixture) {
        let player = fixture.objects[PLAYER_INDEX];
        let actor = fixture.objects[ACTOR_INDEX];
        let arche = fixture.objects[ARCHE_INDEX];
        let world = fixture.objects[WORLD_INDEX];
        match name {
            "inactive_first_entry_only_clears_pair_write_guard" => {
                fixture.presentation.pair_write_disabled = true;
            }
            "kind2_handoff_then_action" => {
                set_active(fixture, ACTOR_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.records.set_record(
                    slot(fixture, ACTOR_INDEX),
                    ScriptActionRecord::ActorPresentation(player),
                );
                set_handoff_target(fixture, TEST_SELECTOR_PREFIX);
            }
            "kind2_blocked_owner_still_runs_action" => {
                set_active(fixture, ACTOR_INDEX, true);
                assert!(set_object_flag(
                    &mut fixture.state,
                    actor,
                    ScriptObjectFlag::PresentationBlocked,
                    true,
                ));
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.records.set_record(
                    slot(fixture, ACTOR_INDEX),
                    ScriptActionRecord::ActorPresentation(player),
                );
                set_handoff_target(fixture, TEST_SELECTOR_PREFIX);
            }
            "kind2_negative_value_suppresses_action" => {
                set_active(fixture, ACTOR_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                let action_slot = slot(fixture, ACTOR_INDEX);
                fixture
                    .records
                    .set_record(action_slot, ScriptActionRecord::ActorPresentation(player));
                fixture.records.set_actionable(action_slot, false);
            }
            "kind1_starts_presentation_without_name_lookup"
            | "kind1_start_runs_descript_effect_chain" => {
                set_active(fixture, PLAYER_INDEX, true);
                set_active(fixture, ACTOR_INDEX, true);
                assert!(set_object_flag(
                    &mut fixture.state,
                    actor,
                    ScriptObjectFlag::Presentable,
                    true,
                ));
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                if name == "kind1_start_runs_descript_effect_chain" {
                    fixture.presentation.name_lookup_enabled = true;
                    fixture.presentation.name_area_effect_active = true;
                }
            }
            "kind1_active_c4_drains_ordinary_deferred_record" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred = ScriptDeferredRecord::Complete {
                    record: ScriptActionRecord::AboardRequest(actor),
                    actionable: true,
                };
            }
            "kind1_teardown_clears_history_before_action" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::AboardRequest(actor),
                );
                let dictionary = decode_script_dictionary(b"history\0").unwrap();
                fixture
                    .selector
                    .history_mut()
                    .push(dictionary.resolve_source_offset(u16::MIN).unwrap());
            }
            "kind1_c1_deferred_targets_arche_ship_field" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred = deferred_navigation_record(actor, true);
            }
            "kind1_c6_deferred_targets_arche_ship_field" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred = ScriptDeferredRecord::Complete {
                    record: ScriptActionRecord::Travel(actor),
                    actionable: true,
                };
            }
            "deferred_negative_value_is_tested_after_overwrite" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred = ScriptDeferredRecord::Complete {
                    record: ScriptActionRecord::AboardRequest(actor),
                    actionable: false,
                };
            }
            "deferred_related_without_type_is_retained" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred = ScriptDeferredRecord::RelatedOnly(actor);
            }
            "deferred_type_without_related_is_retained" => {
                set_active(fixture, PLAYER_INDEX, true);
                fixture.presentation.active = true;
                fixture.records.set_record(
                    slot(fixture, PLAYER_INDEX),
                    ScriptActionRecord::ActorPresentation(actor),
                );
                fixture.presentation.deferred =
                    ScriptDeferredRecord::KindOnly(ScriptDeferredRecordKind::AboardRequest);
            }
            "ship_and_special_entries_both_run_action" => {
                set_active(fixture, ARCHE_INDEX, true);
                set_active(fixture, WORLD_INDEX, true);
                fixture.records.set_record(
                    slot(fixture, ARCHE_INDEX),
                    ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(actor)),
                );
                fixture.records.set_record(
                    slot(fixture, WORLD_INDEX),
                    ScriptActionRecord::Travel(actor),
                );
            }
            "next_directory_kind_must_equal_full_word_one" => {
                set_active(fixture, ARCHE_INDEX, true);
                fixture.records.set_record(
                    slot(fixture, ARCHE_INDEX),
                    ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(actor)),
                );
            }
            "unknown_active_kind_has_no_action" => {
                set_active(fixture, AUXILIARY_INDEX, true);
            }
            unknown => panic!("unaccounted 0x005816 oracle vector {unknown}"),
        }
        assert_eq!(arche, fixture.objects[ARCHE_INDEX]);
        assert_eq!(world, fixture.objects[WORLD_INDEX]);
    }

    fn normalized_original_events(calls: &[OracleCall]) -> Vec<&str> {
        calls
            .iter()
            .filter_map(|call| match call.name.as_str() {
                "field" | "resource" => None,
                "control" => Some("control"),
                "descript" => Some("lookup"),
                "setter" => Some("restart"),
                "action" => Some("action"),
                "transition" if call.object_id == Some(4) => Some("dialogue_transition"),
                "transition" if call.object_id == Some(2) => Some("effect_transition"),
                unknown => panic!("unknown presentation helper {unknown}"),
            })
            .collect()
    }

    fn deferred_matches_native(state: ScriptDeferredRecord, native: [u16; 3]) -> bool {
        match state {
            ScriptDeferredRecord::Empty => native == [u16::MIN; 3],
            ScriptDeferredRecord::RelatedOnly(_) => native[0] == u16::MIN && native[1] != u16::MIN,
            ScriptDeferredRecord::KindOnly(_) => native[0] != u16::MIN && native[1] == u16::MIN,
            ScriptDeferredRecord::Complete { .. } => false,
        }
    }

    #[test]
    fn presentation_scan_accounts_for_every_original_natural_vector() {
        let vectors: Vec<ScanOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5816_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            assert!(!vector.processed_entries.is_empty());
            let mut fixture = fixture();
            configure_vector(&vector.name, &mut fixture);
            let mut host = RecordingHost::default();
            let outcome = scan_script_presentations(
                ScriptPresentationScanContext {
                    state: &mut fixture.state,
                    records: &mut fixture.records,
                    runtime: &mut fixture.runtime,
                    selector: &mut fixture.selector,
                    presentation: &mut fixture.presentation,
                    player: fixture.objects[PLAYER_INDEX],
                    arche: fixture.objects[ARCHE_INDEX],
                },
                &mut host,
            )
            .unwrap();

            assert!(outcome.processed_objects >= vector.processed_entries.len());
            assert_eq!(
                host.events,
                normalized_original_events(&vector.calls),
                "{}",
                vector.name
            );
            assert_eq!(
                fixture.presentation.active,
                vector.presentation_active_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.bridge_console_selection_cleared,
                outcome.presentation_started.is_some(),
                "{}",
                vector.name
            );
            assert!(
                deferred_matches_native(fixture.presentation.deferred, vector.deferred_after),
                "{}",
                vector.name
            );
            assert!(!fixture.presentation.pair_write_disabled);

            if vector.name == "kind2_handoff_then_action" {
                assert_eq!(
                    host.selector_roots,
                    [ScriptCodeOffset::new(TEST_SELECTOR_ROOT)]
                );
            }

            if vector.name == "kind1_teardown_clears_history_before_action" {
                assert!(
                    fixture
                        .selector
                        .history()
                        .entries()
                        .iter()
                        .all(Option::is_none)
                );
            }
            if vector.name == "kind1_c1_deferred_targets_arche_ship_field" {
                assert!(matches!(
                    fixture.records.record(slot(&fixture, ARCHE_INDEX)),
                    ScriptActionRecord::Navigation(_)
                ));
            }
            if vector.name == "kind1_c6_deferred_targets_arche_ship_field" {
                assert_eq!(
                    fixture.records.record(slot(&fixture, ARCHE_INDEX)),
                    ScriptActionRecord::Travel(fixture.objects[ACTOR_INDEX])
                );
            }
        }
    }

    #[test]
    fn every_shipped_profile_object_is_scanned_in_owned_directory_order() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let directory: ScriptDirectory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let mut state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();
            let player = directory.find_active_object(b"blood").unwrap();
            let arche = directory.find_active_object(b"arche").unwrap();
            let object_count = state.objects().len();
            let mut records = ScriptActionRecords::default();
            let mut runtime = ScriptRuntime::new();
            let mut selector = ScriptSelectorState::default();
            let mut presentation = ScriptPresentationScanState::default();
            let mut host = RecordingHost::default();
            let outcome = scan_script_presentations(
                ScriptPresentationScanContext {
                    state: &mut state,
                    records: &mut records,
                    runtime: &mut runtime,
                    selector: &mut selector,
                    presentation: &mut presentation,
                    player,
                    arche,
                },
                &mut host,
            )
            .unwrap();

            assert_eq!(
                outcome.processed_objects, object_count,
                "SCRIPT{profile}.VAR"
            );
            assert!(outcome.handoffs.is_empty());
            assert!(outcome.actions.is_empty());
            assert!(host.events.is_empty());
        }
    }
}
