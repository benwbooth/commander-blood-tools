//! Concrete original-resource and host-command backend for translated BloodScript.

use std::collections::BTreeMap;
use std::mem::size_of;

use anyhow::{Context, Result, anyhow, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::descript::DescriptRecordKind;
use commander_blood_formats::descript_database::DescriptDatabase;
use commander_blood_formats::instruction::ScriptRecordStateOperand;
use commander_blood_formats::script::{ScriptObjectId, ScriptWordId};

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    DescriptApplicationContext, DescriptBackgroundCache, DescriptBackgroundSource,
    DescriptIdleClipSource, DescriptMusicSelectionOutcome, DescriptPresentationAssets,
    DescriptRecordApplication, DescriptSoundBankLoader, GameLifecycleState, LoadedScriptProfile,
    LoadedSoundBank, OriginalSaveGame, ScriptAboardRecordContext, ScriptActionDescription,
    ScriptActionRecord, ScriptActionRuntimeState, ScriptActionState, ScriptClock,
    ScriptDeferredRecord, ScriptDispatchState, ScriptEnvironmentActivity, ScriptExecutionBackend,
    ScriptExecutionService, ScriptFieldSelector, ScriptFrameOutcome, ScriptPresentationEntity,
    ScriptPresentationScanOutcome, ScriptPresentationScanState, ScriptProfileId,
    ScriptProfileLoadOutcome, ScriptRecordStateNavigationContext, ScriptShipNavigationMode,
    ScriptTransferContext, SequencePresentationState, SequenceRequestContext,
    ShipPresentationState, SoundBankUsage, TextPresentationState, deferred_navigation_record,
    execute_loaded_script_frame, load_sound_bank, lookup_and_apply_descript_record,
    original_save_state_block_byte_count, script_field_offset,
};
use crate::native::random::BloodPrng;

use super::{OriginalGameData, OriginalGameRuntime};

const RADIO_CLIP_INDEX: u8 = 6;
const BACKGROUND_RESOURCE_DIRECTORY: &[u8] = b"FD\\";
const SOUND_BANK_RESOURCE_DIRECTORY: &[u8] = b"SN\\";
const LOCATION_VIDEO_RESOURCE_DIRECTORY: &[u8] = b"PL\\";
const CHARACTER_VIDEO_RESOURCE_DIRECTORY: &[u8] = b"PE\\";
const SEQUENCE_VIDEO_RESOURCE_DIRECTORY: &[u8] = b"SQ\\";
const OBJECT_VIDEO_RESOURCE_DIRECTORY: &[u8] = b"OB\\";

/// One original resource retained by a concrete runtime service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedRuntimeResource {
    name: Box<[u8]>,
    encoded_bytes: Box<[u8]>,
}

impl LoadedRuntimeResource {
    fn new(name: &[u8], encoded_bytes: Box<[u8]>) -> Self {
        Self {
            name: Box::from(name),
            encoded_bytes,
        }
    }

    /// Return the authored case-preserving resource name.
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Return the complete encoded original resource.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded_bytes
    }
}

/// Ordered side effects emitted by translated script logic for runtime consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeScriptCommand {
    /// Reinitialize the character name-area transition with newly applied assets.
    RestartNameAreaEffect,
    /// Advance one fixed presentation entity.
    TransitionPresentationEntity(ScriptPresentationEntity),
    /// Restart the currently selected navigation music resource.
    RestartNavigationMusic,
    /// Start one authored radio clip from the active sound bank.
    PlayRadioClip {
        /// Zero-based clip selected by the native C3 action.
        clip_index: u8,
    },
    /// Start the black-hole navigation-chart transition.
    StartCameraTransition {
        /// Exact shared countdown written by native C6.
        steps: u8,
    },
    /// Rebuild the ship HUD and reset its 3D camera state.
    ResetShipHud,
}

/// One-frame semantic outputs transferred from C1-C6 actions to runtime owners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeScriptActionEffects {
    /// Reinitialize the canonical ship HUD on the next HUD update.
    pub ship_hud_refresh_requested: bool,
    /// Rebuild the ordinary navigation screen after the action transition.
    pub screen_rebuild_requested: bool,
    /// Reload the canonical clip playback countdown after C3 starts radio clip 6.
    pub clip_playback_state_reload: Option<u16>,
    /// Arm the timer-owned no-VOC speaker pulse requested by C3.
    pub speaker_pulse_requested: bool,
}

/// Complete profile-independent state surrounding translated BloodScript execution.
pub struct RuntimeScriptSystem {
    dispatch: ScriptDispatchState,
    service: ScriptExecutionService<RuntimeScriptBackend>,
    presentation_word_buffer_nonempty: bool,
    presentation_inventory_line_pending: bool,
}

impl RuntimeScriptSystem {
    /// Construct the script system from validated original data and a host clock.
    pub fn new(data: &OriginalGameData, clock: ScriptClock) -> Self {
        Self {
            dispatch: ScriptDispatchState::default(),
            service: ScriptExecutionService::new(RuntimeScriptBackend::new(data, clock)),
            presentation_word_buffer_nonempty: false,
            presentation_inventory_line_pending: false,
        }
    }

    /// Load one profile, reset profile-local state, and bind exact DEB names.
    pub fn load_profile(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        profile: ScriptProfileId,
    ) -> Result<ScriptProfileLoadOutcome> {
        let outcome = runtime.load_profile(profile)?;
        self.dispatch.reset_for_profile_change();
        self.service.reset_for_profile_change();
        self.presentation_word_buffer_nonempty = false;
        self.presentation_inventory_line_pending = false;
        self.service.backend_mut().bind_profile(
            runtime
                .current_profile()
                .context("profile loader did not retain the selected profile")?,
        );
        Ok(outcome)
    }

    /// Import the session PRNG shared by every recovered native caller.
    pub fn import_random_state(&mut self, random: BloodPrng) {
        self.dispatch.random = random;
    }

    /// Return the session PRNG after COD, BAS, and text chance handlers run.
    pub const fn random_state(&self) -> BloodPrng {
        self.dispatch.random
    }

    /// Execute one complete translated COD/BAS/presentation frame.
    pub fn execute_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        enabled: bool,
    ) -> Result<ScriptFrameOutcome> {
        let outcome = execute_loaded_script_frame(
            runtime
                .current_profile_mut()
                .context("no BloodScript profile is loaded")?,
            enabled,
            &mut self.dispatch,
            &mut self.service,
        )
        .map_err(|error| anyhow!("executing BloodScript frame: {error:?}"))?;
        let selector = runtime
            .current_profile()
            .context("BloodScript profile disappeared after execution")?
            .selector_state();
        self.presentation_word_buffer_nonempty = selector.has_pending_presentation_choices();
        self.presentation_inventory_line_pending = selector.inventory().saved_line().is_some();
        Ok(outcome)
    }

    /// Import the ship-owned half of globals aliased into BloodScript state.
    pub fn prepare_ship_presentation_state(&mut self, ship: &ShipPresentationState) {
        import_ship_presentation_state(&mut self.dispatch, ship);
    }

    /// Publish BloodScript writes to globals canonically owned by the ship FSM.
    pub fn finish_ship_presentation_state(&mut self, ship: &mut ShipPresentationState) {
        export_ship_presentation_state(&self.dispatch, ship);
        publish_script_active_line_to_ship(&mut self.dispatch, ship);
    }

    /// Refresh lifecycle-owned presentation scan fields before exporting VM state.
    ///
    /// Late subtitle and menu renderers mutate text state outside a VM pass. They
    /// must refresh these fields before using the common exporter so an older VM
    /// snapshot cannot overwrite actor- or scheduler-owned lifecycle state.
    pub fn prepare_lifecycle_scan_state(&mut self, lifecycle: &GameLifecycleState) {
        let source = &lifecycle.presentation;
        {
            let presentation = self.service.presentation_state_mut();
            presentation.active = source.active;
            presentation.c2_gate_active = source.c2_presentation_gate;
            presentation.word_choice_active = source.word_choice_active;
            presentation.start_locked = source.start_locked;
            presentation.hold_ready = source.hold_ready;
            presentation.dialogue_hold_complete = source.dialogue_hold_complete;
            presentation.ui_busy = lifecycle.modal_ui_busy();
            presentation.name_lookup_enabled = lifecycle.presentation_interface_active();
            self.dispatch.import_presentation_scan_state(presentation);
        }
        self.dispatch.record_clear_presentation.sequence_active = source.sequence_active;
    }

    /// Import main-loop writes immediately before a late text renderer runs.
    pub fn prepare_lifecycle_text_frame(&mut self, lifecycle: &GameLifecycleState) {
        self.prepare_lifecycle_frame(lifecycle);
        let source = &lifecycle.presentation;
        let text = &mut self.dispatch.text_presentation;
        text.dialogue_hold_complete = source.dialogue_hold_complete;
    }

    /// Merge late text-renderer writes without replacing them from an older lifecycle snapshot.
    pub fn finish_lifecycle_text_frame(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        let dialogue_hold_complete = self.dispatch.text_presentation.dialogue_hold_complete;
        self.prepare_lifecycle_scan_state(lifecycle);
        self.dispatch.text_presentation.dialogue_hold_complete = dialogue_hold_complete;
        self.service.presentation_state_mut().dialogue_hold_complete = dialogue_hold_complete;
        self.finish_lifecycle_frame(lifecycle)
    }

    /// Import globals owned by the recovered main loop before one VM pass.
    pub fn prepare_lifecycle_frame(&mut self, lifecycle: &GameLifecycleState) {
        self.prepare_lifecycle_scan_state(lifecycle);
        let source = &lifecycle.presentation;
        let text = &mut self.dispatch.text_presentation;
        text.subtitle_display_active = source.subtitle_display_active;
        text.menu_deferred = source.menu_deferred;
        text.request_flags = source.request_flags;
        text.subtitle_word_list_mode = source.subtitle_word_list_mode;
        text.subtitle_voice_trigger = source.subtitle_voice_trigger;
        text.dialogue_chatter_active = source.dialogue_chatter_active;
        text.menu_pending = source.text_menu_pending;
        text.selected_line = source.text_selector;
        text.dialogue_hold_countdown = source.dialogue_hold_countdown;
        self.service
            .backend_mut()
            .set_sequence_context(SequenceRequestContext {
                ship_active: source.ship_active,
                scene_gate_active: source.scene_gate_active,
            });
        self.service
            .backend_mut()
            .set_environment_activity(ScriptEnvironmentActivity {
                bridge_active: lifecycle.presentation_interface_active(),
                travel_active: source.sequence_active,
                contact_active: source.scene_gate_active,
            });
        self.service
            .backend_mut()
            .set_presentation_interface_active(lifecycle.presentation_interface_active());
    }

    /// Publish BloodScript writes to the recovered main-loop globals after a VM pass.
    pub fn finish_lifecycle_frame(&mut self, lifecycle: &mut GameLifecycleState) -> Result<()> {
        let presentation = self.service.presentation_state();
        let target = &mut lifecycle.presentation;
        target.active = presentation.active;
        target.c2_presentation_gate = presentation.c2_gate_active;
        target.word_choice_active = presentation.word_choice_active;
        target.start_locked = presentation.start_locked;
        target.hold_ready = presentation.hold_ready;
        target.dialogue_hold_complete = presentation.dialogue_hold_complete;

        let text = &self.dispatch.text_presentation;
        target.subtitle_display_active = text.subtitle_display_active;
        target.menu_deferred = text.menu_deferred;
        target.request_flags = text.request_flags;
        target.subtitle_word_list_mode = text.subtitle_word_list_mode;
        target.subtitle_voice_trigger = text.subtitle_voice_trigger;
        target.dialogue_chatter_active = text.dialogue_chatter_active;
        target.text_menu_pending = text.menu_pending;
        target.text_selector = text.selected_line;
        target.word_buffer_nonempty = self.presentation_word_buffer_nonempty;
        target.inventory_line_pending = self.presentation_inventory_line_pending;
        target.dialogue_hold_countdown = text.dialogue_hold_countdown;
        target.sequence_active = self.dispatch.record_clear_presentation.sequence_active;
        publish_script_active_line_to_lifecycle(&self.dispatch, target);
        lifecycle.set_modal_ui_busy(presentation.ui_busy);
        lifecycle.pending_profile = self
            .dispatch
            .profile_request
            .pending_profile()
            .context("resolving BloodScript profile request")?;
        if let Some(enabled) = self.dispatch.pending_vm_execution_write.take() {
            lifecycle.vm_execution_enabled = enabled;
        }
        Ok(())
    }

    /// Execute one VM pass with exact main-loop state exchange on both sides.
    pub fn execute_lifecycle_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        lifecycle: &mut GameLifecycleState,
        enabled: bool,
    ) -> Result<ScriptFrameOutcome> {
        self.prepare_lifecycle_frame(lifecycle);
        let outcome = self.execute_frame(runtime, enabled)?;
        self.finish_lifecycle_frame(lifecycle)?;
        Ok(outcome)
    }

    /// Borrow the concrete backend for lifecycle-state synchronization.
    pub const fn backend(&self) -> &RuntimeScriptBackend {
        self.service.backend()
    }

    /// Mutably borrow the concrete backend for lifecycle-state synchronization.
    pub fn backend_mut(&mut self) -> &mut RuntimeScriptBackend {
        self.service.backend_mut()
    }

    /// Drain ordered renderer, audio, camera, and HUD commands.
    pub fn take_commands(&mut self) -> Vec<RuntimeScriptCommand> {
        self.service.backend_mut().take_commands()
    }

    /// Borrow topic and A8 sequence state produced by the translated dispatcher.
    pub const fn sequence_presentation(&self) -> &SequencePresentationState {
        &self.dispatch.sequence_presentation
    }

    /// Shared sequel CC/D7 controls, separate from the inherited A8 finale.
    pub(crate) const fn sequel_presentation_control(
        &self,
    ) -> crate::native::bloodprg::SequelPresentationControl {
        self.dispatch.sequel_presentation
    }

    /// Consume CC before panel callbacks, so later script writes remain pending.
    pub(crate) fn consume_sequel_presentation_choice(&mut self) {
        self.dispatch.sequel_presentation.requested_choice = None;
    }

    /// Drain A8's low-byte clear of the mouse-idle timer alias at native address `0x0B3B`.
    pub fn take_mouse_idle_low_byte_clear_request(&mut self) -> bool {
        self.dispatch
            .sequence_presentation
            .take_mouse_idle_low_byte_clear_request()
    }

    /// Borrow subtitle and inline-menu state produced by translated A6 handlers.
    pub const fn text_presentation(&self) -> &TextPresentationState {
        &self.dispatch.text_presentation
    }

    /// Mutably borrow subtitle and inline-menu state for frame-tail rendering.
    pub fn text_presentation_mut(&mut self) -> &mut TextPresentationState {
        &mut self.dispatch.text_presentation
    }

    /// Borrow the persistent presentation-scan state shared with scene coordinators.
    pub const fn presentation_scan_state(&self) -> &ScriptPresentationScanState {
        self.service.presentation_state()
    }

    /// Mutably borrow presentation-scan state for a frame-tail scene coordinator.
    pub fn presentation_scan_state_mut(&mut self) -> &mut ScriptPresentationScanState {
        self.service.presentation_state_mut()
    }

    /// Drain the palette upload alias written by a fresh presentation scan.
    pub fn take_presentation_palette_dirty(&mut self) -> bool {
        std::mem::take(&mut self.service.presentation_state_mut().palette_dirty)
    }

    /// Publish one completed dialogue choice to the VM and clear its text owner.
    pub fn complete_word_choice(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        concept: ScriptWordId,
    ) -> Result<()> {
        runtime
            .current_profile_mut()
            .context("completing a word choice requires a loaded BloodScript profile")?
            .runtime_mut()
            .set_selected_concept(Some(concept));
        runtime
            .current_profile_mut()
            .expect("loaded profile remains available while completing a choice")
            .selector_state_mut()
            .replace_presentation_words([]);
        self.clear_completed_choice_presentation();
        Ok(())
    }

    fn clear_completed_choice_presentation(&mut self) {
        self.presentation_word_buffer_nonempty = false;
        {
            let presentation = self.service.presentation_state_mut();
            presentation.word_choice_active = false;
            presentation.dialogue_hold_complete = false;
        }
        let text = &mut self.dispatch.text_presentation;
        text.menu_deferred = false;
        text.subtitle_display_active = false;
        text.dialogue_hold_complete = false;
        text.request_flags.clear_text_request();
        text.menu_words = Box::new([]);
        text.menu_word_count = usize::MIN;
    }

    /// Complete an object choice or cancellation without entering the DIC domain.
    pub fn complete_lifecycle_inventory_choice(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        lifecycle: &mut GameLifecycleState,
        object: Option<ScriptObjectId>,
    ) -> Result<()> {
        self.prepare_lifecycle_frame(lifecycle);
        let parts = runtime
            .current_profile_mut()
            .context("completing an inventory choice requires a loaded profile")?
            .execution_parts();
        anyhow::ensure!(
            parts.state.dialect() == commander_blood_formats::code::ScriptDialect::BigBugBang,
            "inventory selection requires the sequel dialect"
        );
        parts
            .selector_state
            .inventory_mut()
            .complete_choice(object, parts.runtime)?;
        parts.selector_state.replace_presentation_words([]);
        self.presentation_inventory_line_pending =
            parts.selector_state.inventory().saved_line().is_some();
        self.clear_completed_choice_presentation();
        self.finish_lifecycle_frame(lifecycle)
    }

    /// Complete a word choice without exporting stale copies of other lifecycle globals.
    pub fn complete_lifecycle_word_choice(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        lifecycle: &mut GameLifecycleState,
        concept: ScriptWordId,
    ) -> Result<()> {
        self.prepare_lifecycle_frame(lifecycle);
        self.complete_word_choice(runtime, concept)?;
        self.finish_lifecycle_frame(lifecycle)
    }

    /// Apply a presentation-panel DESCRIPT record through the live script state.
    pub fn apply_presentation_description(
        &mut self,
        name: &[u8],
    ) -> Result<Option<DescriptRecordApplication>> {
        let Self {
            dispatch, service, ..
        } = self;
        let interface_active = service.backend().presentation_interface_active;
        let object = service
            .backend()
            .object_names
            .iter()
            .find_map(|(object, object_name)| (object_name.as_ref() == name).then_some(*object));
        let application = service.backend_mut().apply_description(
            name,
            interface_active,
            &mut dispatch.text_presentation,
        )?;
        service.backend_mut().active_description_object = application.and(object);
        Ok(application)
    }

    /// Apply a DESCRIPT record by stable profile object identity.
    pub fn apply_object_description(
        &mut self,
        object: ScriptObjectId,
    ) -> Result<Option<DescriptRecordApplication>> {
        let Self {
            dispatch, service, ..
        } = self;
        let name = service.backend().object_name(object)?.to_vec();
        let interface_active = service.backend().presentation_interface_active;
        let application = service.backend_mut().apply_description(
            &name,
            interface_active,
            &mut dispatch.text_presentation,
        )?;
        service.backend_mut().active_description_object = application.map(|_| object);
        Ok(application)
    }

    /// Apply one object's DESCRIPT record to state owned by a frame-tail coordinator.
    pub fn apply_object_description_to_text(
        &mut self,
        object: ScriptObjectId,
        presentation_interface_active: bool,
        text: &mut TextPresentationState,
    ) -> Result<Option<DescriptRecordApplication>> {
        let name = self.service.backend().object_name(object)?.to_vec();
        self.service
            .backend_mut()
            .set_presentation_interface_active(presentation_interface_active);
        let application = self.service.backend_mut().apply_description(
            &name,
            presentation_interface_active,
            text,
        )?;
        self.service.backend_mut().active_description_object = application.map(|_| object);
        Ok(application)
    }

    /// Queue a complete C1 for the deferred presentation path through Arche.
    pub fn defer_navigation_target(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = deferred_navigation_record(target, true);
    }

    /// Queue the actionable C3 record emitted by bridge horn, target, and radio commands.
    pub fn defer_presentation_queue(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = ScriptDeferredRecord::Complete {
            record: ScriptActionRecord::PresentationQueue(target),
            actionable: true,
        };
    }

    /// Write the C1 emitted by `ship_3d_hud_init` directly to `orxx`'s action slot.
    pub fn queue_ship_hud_navigation_target(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        target: ScriptObjectId,
    ) -> Result<()> {
        let profile = runtime
            .current_profile_mut()
            .context("ship HUD navigation requires a loaded BloodScript profile")?;
        let world = profile
            .builtins()
            .world
            .context("loaded BloodScript profile has no orxx object")?;
        profile
            .state()
            .object(target)
            .with_context(|| format!("ship HUD navigation target {target:?} is absent"))?;
        let world_kind = profile
            .state()
            .object(world)
            .context("loaded BloodScript profile has no bound orxx state object")?
            .kind;
        let action_offset = script_field_offset(world_kind, ScriptFieldSelector::ACTION)
            .context("orxx has no C1 action field")?;
        let action_slot = profile
            .state()
            .object_word_triple(world, action_offset / size_of::<u16>())
            .context("orxx has a truncated C1 action field")?;
        let parts = profile.execution_parts();
        parts.record_state.action_records.set_record(
            action_slot,
            ScriptActionRecord::Navigation(ScriptRecordStateOperand::Object(target)),
        );
        parts
            .record_state
            .commit_to_var(parts.state, parts.directory, parts.dictionary)
            .context("committing the ship HUD C1 action to BloodScript VAR state")?;
        Ok(())
    }

    /// Queue the zero-valued, immediately actionable C4 emitted by bridge actors.
    pub fn defer_actor_presentation(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = ScriptDeferredRecord::Complete {
            record: ScriptActionRecord::ActorPresentation(target),
            actionable: true,
        };
    }

    /// Queue the complete typed C6 action emitted by black-hole presentation.
    pub fn defer_travel_target(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = ScriptDeferredRecord::Complete {
            record: ScriptActionRecord::Travel(target),
            actionable: true,
        };
    }

    /// Release the C2 scene gate and its secondary request after actor 3 closes it.
    pub fn finish_actor_scene_presentation(&mut self) {
        self.service.presentation_state_mut().c2_gate_active = false;
        self.dispatch
            .text_presentation
            .request_flags
            .clear_secondary_request();
    }

    /// Borrow persistent semantic state produced by C1 through C8 actions.
    pub const fn action_state(&self) -> &ScriptActionState {
        self.service.action_state()
    }

    /// Mutably borrow action state for synchronization with the outer ship coordinator.
    pub fn action_state_mut(&mut self) -> &mut ScriptActionState {
        self.service.action_state_mut()
    }

    /// Borrow the observable result of the most recent post-frame presentation scan.
    pub const fn last_presentation_outcome(&self) -> Option<&ScriptPresentationScanOutcome> {
        self.service.last_presentation_outcome()
    }

    /// Consume the one-frame C1 target-selection transition while retaining its target.
    pub fn take_selected_ship_target(&mut self) -> Option<ScriptObjectId> {
        let action = self.service.action_state_mut();
        if action.ship_navigation_mode != ScriptShipNavigationMode::TargetSelected {
            return None;
        }
        action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        action.current_ship_target
    }

    /// Consume C1-C6 outputs exactly once when their canonical owners are available.
    pub fn take_action_effects(&mut self, lifecycle_available: bool) -> RuntimeScriptActionEffects {
        let action = self.service.action_state_mut();
        action.active_line = None;
        RuntimeScriptActionEffects {
            ship_hud_refresh_requested: std::mem::take(&mut action.ship_hud_refresh_requested),
            screen_rebuild_requested: lifecycle_available
                && std::mem::take(&mut action.screen_rebuild_requested),
            clip_playback_state_reload: lifecycle_available
                .then(|| action.clip_playback_state_reload.take())
                .flatten(),
            speaker_pulse_requested: lifecycle_available
                && std::mem::take(&mut action.speaker_pulse_requested),
        }
    }
}

fn import_ship_presentation_state(
    dispatch: &mut ScriptDispatchState,
    ship: &ShipPresentationState,
) {
    dispatch.record_clear_presentation.ship_3d_depth_step = ship.depth_step;
}

fn export_ship_presentation_state(
    dispatch: &ScriptDispatchState,
    ship: &mut ShipPresentationState,
) {
    ship.depth_step = dispatch.record_clear_presentation.ship_3d_depth_step;
}

fn publish_script_active_line_to_ship(
    dispatch: &mut ScriptDispatchState,
    ship: &mut ShipPresentationState,
) {
    if let Some(line) = dispatch.take_active_line_write() {
        ship.active_line = line;
    }
}

fn publish_script_active_line_to_lifecycle(
    dispatch: &ScriptDispatchState,
    presentation: &mut crate::native::bloodprg::GamePresentationScheduler,
) {
    if let Some(line) = dispatch.pending_active_line_write() {
        presentation.active_line = Some(line);
    }
}

/// Initialize an already selected profile, then transactionally restore one original save image.
///
/// Profile resource selection remains with the caller because the concrete SDL service also
/// rebuilds profile-owned HUD, navigation, and scene adapters. Keeping the data-only transaction
/// here gives production and headless campaign validation one exact flat-memory restore path.
pub fn initialize_and_restore_original_save_game(
    scripts: &mut RuntimeScriptSystem,
    runtime: &mut OriginalGameRuntime,
    lifecycle: &mut GameLifecycleState,
    data: &[u8],
) -> Result<()> {
    let saved_profile =
        OriginalSaveGame::decode_profile(data).context("decoding the saved BloodScript profile")?;
    let loaded_profile = runtime
        .current_profile()
        .context("save restoration requires a loaded BloodScript profile")?
        .id();
    if loaded_profile != saved_profile {
        bail!(
            "loaded BloodScript profile {} does not match saved profile {}",
            loaded_profile.value(),
            saved_profile.value()
        );
    }

    lifecycle.pending_profile = None;
    lifecycle.vm_execution_enabled = true;
    scripts
        .execute_lifecycle_frame(runtime, lifecycle, true)
        .context("initializing the saved BloodScript profile")?;

    let state_byte_count = original_save_state_block_byte_count(
        runtime
            .current_profile()
            .context("saved profile initialization did not retain a profile")?,
    )
    .context("resolving the saved profile state allocation")?;
    let save = OriginalSaveGame::decode(data, state_byte_count)
        .context("decoding the complete original save image")?;
    save.restore_into(
        runtime
            .current_profile_mut()
            .context("saved profile disappeared before state restoration")?,
    )
    .context("restoring the original save blocks")
}

/// Concrete flat backend state shared by the script service and game lifecycle.
pub struct RuntimeScriptBackend {
    database: DescriptDatabase,
    object_names: BTreeMap<ScriptObjectId, Box<[u8]>>,
    assets: DescriptPresentationAssets,
    backgrounds: DescriptBackgroundCache,
    background_source: RuntimeBackgroundSource,
    sound_loader: RuntimeSoundBankLoader,
    idle_source: RuntimeIdleClipSource,
    environment_activity: ScriptEnvironmentActivity,
    clock: ScriptClock,
    sequence_context: SequenceRequestContext,
    navigation_context: Option<ScriptRecordStateNavigationContext>,
    action_runtime_state: ScriptActionRuntimeState,
    presentation_interface_active: bool,
    active_description_object: Option<ScriptObjectId>,
    last_descript_application: Option<DescriptRecordApplication>,
    commands: Vec<RuntimeScriptCommand>,
}

impl RuntimeScriptBackend {
    /// Clone immutable original-resource services into a persistent script backend.
    pub fn new(data: &OriginalGameData, clock: ScriptClock) -> Self {
        let store = data.resource_store().clone();
        Self {
            database: data.descript_database().clone(),
            object_names: BTreeMap::new(),
            assets: DescriptPresentationAssets::default(),
            backgrounds: DescriptBackgroundCache::default(),
            background_source: RuntimeBackgroundSource::new(store.clone()),
            sound_loader: RuntimeSoundBankLoader::new(store.clone()),
            idle_source: RuntimeIdleClipSource::new(store),
            environment_activity: ScriptEnvironmentActivity::default(),
            clock,
            sequence_context: SequenceRequestContext::default(),
            navigation_context: None,
            action_runtime_state: ScriptActionRuntimeState::default(),
            presentation_interface_active: false,
            active_description_object: None,
            last_descript_application: None,
            commands: Vec::new(),
        }
    }

    /// Bind stable profile object identities to their exact DEB names.
    pub fn bind_profile(&mut self, profile: &LoadedScriptProfile) {
        self.object_names = profile
            .directory()
            .active_objects()
            .map(|(object, entry)| (object, Box::from(entry.name())))
            .collect();
        self.active_description_object = None;
    }

    /// Apply one exact DESCRIPT record through real original-resource loaders.
    pub fn apply_description(
        &mut self,
        name: &[u8],
        presentation_active: bool,
        text: &mut TextPresentationState,
    ) -> Result<Option<DescriptRecordApplication>> {
        self.idle_source.record_kind = self.database.lookup(name).map(|record| record.kind());
        let mut context = DescriptApplicationContext::new(
            presentation_active,
            &mut self.assets,
            text,
            &mut self.backgrounds,
            &mut self.background_source,
            &mut self.sound_loader,
            &mut self.idle_source,
        );
        let application = lookup_and_apply_descript_record(&self.database, name, &mut context)
            .map_err(|error| {
                anyhow!(
                    "applying DESCRIPT record {}: {error:?}",
                    String::from_utf8_lossy(name)
                )
            })?;
        self.last_descript_application = application;
        Ok(application)
    }

    /// Update CE through D1 activity inputs from the current lifecycle state.
    pub fn set_environment_activity(&mut self, activity: ScriptEnvironmentActivity) {
        self.environment_activity = activity;
    }

    /// Update CA and CB inputs from the host clock.
    pub fn set_clock(&mut self, clock: ScriptClock) {
        self.clock = clock;
    }

    /// Update the A8 presentation gates from the current lifecycle state.
    pub fn set_sequence_context(&mut self, context: SequenceRequestContext) {
        self.sequence_context = context;
    }

    /// Bind the dynamic C1 navigation operands for the current bridge frame.
    pub fn set_navigation_context(&mut self, context: Option<ScriptRecordStateNavigationContext>) {
        self.navigation_context = context;
    }

    /// Publish canonical ship and camera state for the next post-frame action scan.
    pub fn set_action_runtime_state(&mut self, state: ScriptActionRuntimeState) {
        self.action_runtime_state = state;
    }

    /// Update the canonical low UI bit shared by DESCRIPT and transfer paths.
    pub fn set_presentation_interface_active(&mut self, active: bool) {
        self.presentation_interface_active = active;
    }

    /// Borrow the current DESCRIPT-selected presentation assets.
    pub const fn assets(&self) -> &DescriptPresentationAssets {
        &self.assets
    }

    /// Borrow the four-slot original background cache.
    pub const fn backgrounds(&self) -> &DescriptBackgroundCache {
        &self.backgrounds
    }

    /// Release every DESCRIPT background retained by the original four-slot cache.
    pub fn clear_background_cache(&mut self) {
        self.backgrounds.clear();
    }

    /// Borrow authored background paths that the original data set cannot resolve.
    pub fn missing_background_resources(&self) -> &[Box<[u8]>] {
        &self.background_source.missing_resources
    }

    /// Borrow the most recently loaded streamed-dialogue SND resource.
    pub fn loaded_streamed_sound_bank_resource(&self) -> Option<&LoadedRuntimeResource> {
        self.sound_loader.loaded.as_ref()
    }

    /// Borrow the validated streamed-dialogue bank selected by DESCRIPT or radio state.
    pub fn streamed_sound_bank(&self) -> Option<&LoadedSoundBank> {
        self.sound_loader.decoded.as_ref()
    }

    /// Borrow every successfully loaded streamed bank in native call order.
    pub fn streamed_sound_bank_load_history(&self) -> &[Box<[u8]>] {
        &self.sound_loader.load_history
    }

    /// Load one authored streamed SND bank through the DESCRIPT resource service.
    pub fn load_streamed_sound_bank(&mut self, bank_name: &[u8]) -> Result<()> {
        self.sound_loader.load_sound_bank(bank_name)
    }

    /// Return the object whose DESCRIPT record currently owns presentation assets.
    pub const fn active_description_object(&self) -> Option<ScriptObjectId> {
        self.active_description_object
    }

    /// Return the application outcome from the most recent DESCRIPT lookup.
    pub const fn last_descript_application(&self) -> Option<DescriptRecordApplication> {
        self.last_descript_application
    }

    /// Borrow ordered side effects not yet consumed by the enclosing lifecycle.
    pub fn pending_commands(&self) -> &[RuntimeScriptCommand] {
        &self.commands
    }

    /// Drain ordered side effects for renderer, audio, camera, and HUD consumers.
    pub fn take_commands(&mut self) -> Vec<RuntimeScriptCommand> {
        std::mem::take(&mut self.commands)
    }

    fn object_name(&self, object: ScriptObjectId) -> Result<&[u8]> {
        self.object_names
            .get(&object)
            .map(Box::as_ref)
            .with_context(|| format!("script object {:?} is not bound to a DEB name", object))
    }

    fn validate_object_name(&self, object: ScriptObjectId, name: &[u8]) -> Result<()> {
        let bound_name = self.object_name(object)?;
        if bound_name != name {
            bail!(
                "script object {:?} name mismatch: bound {}, received {}",
                object,
                String::from_utf8_lossy(bound_name),
                String::from_utf8_lossy(name)
            );
        }
        Ok(())
    }

    fn object_has_description(&self, object: ScriptObjectId) -> Result<bool> {
        let name = self.object_name(object)?;
        Ok(self.database.lookup(name).is_some())
    }
}

impl ScriptExecutionBackend for RuntimeScriptBackend {
    type Error = anyhow::Error;

    fn environment_activity(&self) -> ScriptEnvironmentActivity {
        self.environment_activity
    }

    fn clock(&self) -> ScriptClock {
        self.clock
    }

    fn sequence_context(&self) -> SequenceRequestContext {
        self.sequence_context
    }

    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
        self.navigation_context
    }

    fn action_runtime_state(&self) -> ScriptActionRuntimeState {
        self.action_runtime_state
    }

    fn aboard_context(&mut self, related: ScriptObjectId) -> Result<ScriptAboardRecordContext> {
        Ok(ScriptAboardRecordContext {
            ship_interface_active: self.presentation_interface_active,
            descriptor_available: self.object_has_description(related)?,
        })
    }

    fn transfer_context(&mut self, item: ScriptObjectId) -> Result<ScriptTransferContext> {
        Ok(ScriptTransferContext {
            ship_interface_active: self.presentation_interface_active,
            descriptor_available: self.object_has_description(item)?,
        })
    }

    fn lookup_presentation_description(
        &mut self,
        related: ScriptObjectId,
        name: &[u8],
        text: &mut TextPresentationState,
    ) -> Result<bool> {
        self.validate_object_name(related, name)?;
        let application = self.apply_description(name, self.presentation_interface_active, text)?;
        self.active_description_object = application.map(|_| related);
        Ok(self.assets.character_sprite().is_some())
    }

    fn restart_name_area_effect(&mut self) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::RestartNameAreaEffect);
        Ok(())
    }

    fn transition_presentation_entity(&mut self, entity: ScriptPresentationEntity) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::TransitionPresentationEntity(entity));
        Ok(())
    }

    fn apply_action_description(
        &mut self,
        object: ScriptObjectId,
        name: &[u8],
        text: &mut TextPresentationState,
    ) -> Result<ScriptActionDescription> {
        self.validate_object_name(object, name)?;
        let application = self.apply_description(name, self.presentation_interface_active, text)?;
        self.active_description_object = application.map(|_| object);
        Ok(
            application.map_or_else(ScriptActionDescription::default, |application| {
                ScriptActionDescription {
                    available: true,
                    music_changed: matches!(
                        application.music_selection(),
                        Some(DescriptMusicSelectionOutcome::Changed)
                    ),
                    scene_vertical_offset: self.assets.location_scene_top_row(),
                }
            }),
        )
    }

    fn restart_navigation_music(&mut self) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::RestartNavigationMusic);
        Ok(())
    }

    fn play_radio_clip(&mut self, playback_countdown: u16) -> Result<()> {
        self.action_runtime_state.clip_playback_state = playback_countdown;
        if self.action_runtime_state.voc_playback_enabled {
            self.commands.push(RuntimeScriptCommand::PlayRadioClip {
                clip_index: RADIO_CLIP_INDEX,
            });
        }
        Ok(())
    }

    fn start_camera_transition(&mut self, steps: u8) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::StartCameraTransition { steps });
        Ok(())
    }

    fn reset_ship_hud(&mut self) -> Result<()> {
        self.commands.push(RuntimeScriptCommand::ResetShipHud);
        Ok(())
    }
}

struct RuntimeBackgroundSource {
    store: OriginalResourceStore,
    missing_resources: Vec<Box<[u8]>>,
}

impl RuntimeBackgroundSource {
    fn new(store: OriginalResourceStore) -> Self {
        Self {
            store,
            missing_resources: Vec::new(),
        }
    }
}

impl DescriptBackgroundSource for RuntimeBackgroundSource {
    type Error = anyhow::Error;

    fn load_background(&mut self, source_name: &[u8]) -> Result<Box<[u8]>> {
        let resource_name = prefixed_resource_name(BACKGROUND_RESOURCE_DIRECTORY, source_name)?;
        if !self.store.resource_exists(&resource_name)? {
            self.missing_resources
                .push(Box::from(resource_name.as_bytes()));
            return Ok(Box::new([]));
        }
        self.store.load(&resource_name).with_context(|| {
            format!(
                "loading resource {}",
                String::from_utf8_lossy(resource_name.as_bytes())
            )
        })
    }
}

struct RuntimeSoundBankLoader {
    store: OriginalResourceStore,
    loaded: Option<LoadedRuntimeResource>,
    decoded: Option<LoadedSoundBank>,
    load_history: Vec<Box<[u8]>>,
}

impl RuntimeSoundBankLoader {
    fn new(store: OriginalResourceStore) -> Self {
        Self {
            store,
            loaded: None,
            decoded: None,
            load_history: Vec::new(),
        }
    }
}

impl DescriptSoundBankLoader for RuntimeSoundBankLoader {
    type Error = anyhow::Error;

    fn load_sound_bank(&mut self, bank_name: &[u8]) -> Result<()> {
        let encoded_bytes =
            load_prefixed_resource(&self.store, SOUND_BANK_RESOURCE_DIRECTORY, bank_name)?;
        let decoded = load_sound_bank(true, SoundBankUsage::StreamedDialogue, &encoded_bytes)
            .context("decoding streamed dialogue sound bank")?
            .context("streamed dialogue sound loading was unexpectedly disabled")?;
        self.loaded = Some(LoadedRuntimeResource::new(bank_name, encoded_bytes));
        self.decoded = Some(decoded);
        self.load_history.push(Box::from(bank_name));
        Ok(())
    }
}

struct RuntimeIdleClipSource {
    store: OriginalResourceStore,
    record_kind: Option<DescriptRecordKind>,
}

impl RuntimeIdleClipSource {
    fn new(store: OriginalResourceStore) -> Self {
        Self {
            store,
            record_kind: None,
        }
    }
}

impl DescriptIdleClipSource for RuntimeIdleClipSource {
    type Error = anyhow::Error;

    fn load_idle_clip(&mut self, video_name: &[u8]) -> Result<Box<[u8]>> {
        let record_kind = self
            .record_kind
            .context("idle HNM requested without a matched DESCRIPT record")?;
        load_prefixed_resource(
            &self.store,
            video_resource_directory(record_kind),
            video_name,
        )
    }
}

fn load_prefixed_resource(
    store: &OriginalResourceStore,
    directory: &[u8],
    name: &[u8],
) -> Result<Box<[u8]>> {
    let path = prefixed_resource_name(directory, name)?;
    store.load(&path).with_context(|| {
        format!(
            "loading resource {}",
            String::from_utf8_lossy(path.as_bytes())
        )
    })
}

fn prefixed_resource_name(directory: &[u8], name: &[u8]) -> Result<BloodResourceName> {
    if name.contains(&b'/') || name.contains(&b'\\') {
        return BloodResourceName::new(name).context("validating authored resource path");
    }
    let mut path = Vec::with_capacity(directory.len() + name.len());
    path.extend_from_slice(directory);
    path.extend_from_slice(name);
    BloodResourceName::new(path).context("validating typed DESCRIPT resource path")
}

fn video_resource_directory(kind: DescriptRecordKind) -> &'static [u8] {
    match kind {
        DescriptRecordKind::Location => LOCATION_VIDEO_RESOURCE_DIRECTORY,
        DescriptRecordKind::Character => CHARACTER_VIDEO_RESOURCE_DIRECTORY,
        DescriptRecordKind::Sequence => SEQUENCE_VIDEO_RESOURCE_DIRECTORY,
        DescriptRecordKind::Object => OBJECT_VIDEO_RESOURCE_DIRECTORY,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::{
        ScriptTextWord, ScriptTimerSlot, decode_script_profile_request,
    };

    use crate::native::bloodprg::{
        GameTimerContext, GameTimerState, ORIGINAL_SCRIPT_PROFILE_COUNT, PresentationResourceLine,
        ScriptAboardPresentationLine, ScriptActionPresentationLine, ScriptFrameEnd,
        ScriptProfileId, ScriptTransferPresentationLine, advance_game_timer_tick,
    };

    use super::super::OriginalGameDataPaths;
    use super::*;

    const SHIPPED_DESCRIPT_RECORD_COUNT: usize = 145;
    const SHIPPED_MISSING_BACKGROUND: &[u8] = b"FD\\marais1d.lbm";
    const DEFAULT_BRIDGE_SOUND_BANK: &[u8] = b"tb.snd";
    const TEST_CLOCK: ScriptClock = ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    };
    const UPDATED_TEST_CLOCK: ScriptClock = ScriptClock {
        hour: 23,
        day: 29,
        month: 2,
    };
    const PROFILE_REQUEST_OPCODE: u8 = 0xD2;
    const SCRIPT_END_OPCODE: u8 = 0xFF;
    const REQUESTED_PROFILE_NUMBER: u8 = 3;
    const ALL_PRESENTATION_REQUESTS: u8 = 3;
    const STARTUP_PHONE_TIMER_SLOT: u8 = 22;
    const STARTUP_PHONE_TIMER_VALUE: u16 = 5;
    const TIMER_TICKS_PER_GAME_FRAME: usize = 8;
    const STARTUP_PHONE_FRAME_LIMIT: usize = 160;
    const IZWALITO_NAME: &[u8] = b"Izwalito";
    const IZWALITO_IDLE_VIDEO: &[u8] = b"aaisw.hnm";
    const TEXT_ONLY_PRESENTATION_SELECTOR: i8 = -1;
    const IZWALITO_FIRST_WORD: &[u8] = b"You";
    const IMPORTED_SHIP_DEPTH_STEP: u8 = 41;
    const SCRIPT_UPDATED_SHIP_DEPTH_STEP: u8 = 6;
    const RADIO_CLIP_PLAYBACK_COUNTDOWN: u16 = 2;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-script-backend-test-{}-{sequence}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn script_clock_sample_can_refresh_before_each_vm_pass() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);

        assert_eq!(backend.clock(), TEST_CLOCK);
        backend.set_clock(UPDATED_TEST_CLOCK);
        assert_eq!(backend.clock(), UPDATED_TEST_CLOCK);
    }

    #[test]
    fn script_system_round_trips_the_shared_session_prng_state() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let random = BloodPrng {
            seed: 4_660,
            mix_low: 17,
            mix_high: 29,
            counter: 9,
        };

        scripts.import_random_state(random);

        assert_eq!(scripts.random_state(), random);
    }

    #[test]
    fn ship_depth_step_alias_round_trips_through_script_dispatch_state() {
        let mut dispatch = ScriptDispatchState::default();
        let mut ship = ShipPresentationState {
            depth_step: IMPORTED_SHIP_DEPTH_STEP,
            ..ShipPresentationState::default()
        };

        import_ship_presentation_state(&mut dispatch, &ship);
        assert_eq!(
            dispatch.record_clear_presentation.ship_3d_depth_step,
            IMPORTED_SHIP_DEPTH_STEP
        );

        dispatch.record_clear_presentation.ship_3d_depth_step = SCRIPT_UPDATED_SHIP_DEPTH_STEP;
        export_ship_presentation_state(&dispatch, &mut ship);
        assert_eq!(ship.depth_step, SCRIPT_UPDATED_SHIP_DEPTH_STEP);
    }

    #[test]
    fn every_shipped_descript_record_applies_through_real_resource_services() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let names: Vec<Box<[u8]>> = data
            .descript_database()
            .records()
            .iter()
            .map(|record| Box::from(record.name()))
            .collect();
        assert_eq!(names.len(), SHIPPED_DESCRIPT_RECORD_COUNT);
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);
        let mut text = TextPresentationState::default();
        let mut loaded_sound_bank = false;
        let mut loaded_idle_clip = false;

        for name in names {
            let application = backend
                .apply_description(&name, false, &mut text)
                .unwrap_or_else(|error| {
                    panic!(
                        "DESCRIPT record {} failed: {error:#}",
                        String::from_utf8_lossy(&name)
                    )
                });
            let application =
                application.unwrap_or_else(|| panic!("missing {}", String::from_utf8_lossy(&name)));
            if application.sound_bank_loaded() {
                loaded_sound_bank = true;
                assert!(
                    backend
                        .loaded_streamed_sound_bank_resource()
                        .is_some_and(|resource| !resource.encoded_bytes().is_empty())
                );
                assert!(backend.streamed_sound_bank().is_some());
            }
            if application.idle_clip_loaded() {
                loaded_idle_clip = true;
                assert!(
                    backend
                        .assets()
                        .encoded_idle_video()
                        .is_some_and(|video| !video.is_empty())
                );
            }
        }

        assert!(loaded_sound_bank);
        assert!(loaded_idle_clip);
        assert_eq!(
            backend.missing_background_resources(),
            &[Box::from(SHIPPED_MISSING_BACKGROUND)]
        );
    }

    #[test]
    fn action_description_lookup_applies_location_video_for_shipped_profile_object() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut runtime = super::super::OriginalGameRuntime::new(data);
        let mut text = TextPresentationState::default();

        for profile_id in ScriptProfileId::all() {
            scripts.load_profile(&mut runtime, profile_id).unwrap();
            let objects = runtime
                .current_profile()
                .unwrap()
                .directory()
                .active_objects()
                .map(|(object, entry)| (object, entry.name().to_vec()))
                .collect::<Vec<_>>();

            for (object, name) in objects {
                let applied = scripts
                    .backend_mut()
                    .apply_action_description(object, &name, &mut text)
                    .unwrap();
                if applied.available && scripts.backend().assets().location_scene_video().is_some()
                {
                    assert_eq!(scripts.backend().active_description_object(), Some(object));
                    return;
                }
            }
        }

        panic!("no shipped profile object applied a DESCRIPT location video");
    }

    #[test]
    fn streamed_sound_bank_loads_from_original_resources() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);

        backend
            .load_streamed_sound_bank(DEFAULT_BRIDGE_SOUND_BANK)
            .unwrap();
        let resource = backend.loaded_streamed_sound_bank_resource().unwrap();
        assert_eq!(resource.name(), DEFAULT_BRIDGE_SOUND_BANK);
        assert!(!resource.encoded_bytes().is_empty());
        assert_eq!(
            backend.streamed_sound_bank().unwrap().usage,
            SoundBankUsage::StreamedDialogue
        );
        assert_eq!(
            backend.streamed_sound_bank_load_history(),
            &[Box::from(DEFAULT_BRIDGE_SOUND_BANK)]
        );
    }

    #[test]
    fn lifecycle_exchange_preserves_shared_native_presentation_globals() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.presentation.active = true;
        lifecycle.presentation.ship_active = true;
        lifecycle.presentation.c2_presentation_gate = true;
        lifecycle.presentation.scene_gate_active = true;
        lifecycle.presentation.sequence_active = true;
        lifecycle.presentation.word_choice_active = true;
        lifecycle.presentation.start_locked = true;
        lifecycle.presentation.hold_ready = true;
        lifecycle.presentation.dialogue_hold_complete = true;
        lifecycle.presentation.subtitle_display_active = true;
        lifecycle.presentation.menu_deferred = true;
        lifecycle.presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(3);
        lifecycle.presentation.subtitle_word_list_mode = true;
        lifecycle.presentation.subtitle_voice_trigger = true;
        lifecycle.presentation.dialogue_chatter_active = true;
        lifecycle.presentation.text_menu_pending = true;
        lifecycle.presentation.dialogue_hold_countdown = 12;
        lifecycle.set_presentation_interface_active(true);
        lifecycle.set_modal_ui_busy(true);

        scripts.prepare_lifecycle_frame(&lifecycle);

        let presentation = scripts.service.presentation_state();
        assert!(presentation.active);
        assert!(presentation.c2_gate_active);
        assert!(presentation.word_choice_active);
        assert!(presentation.start_locked);
        assert!(presentation.hold_ready);
        assert!(presentation.dialogue_hold_complete);
        assert!(presentation.ui_busy);
        assert!(scripts.dispatch.sequence_presentation.presentation_active);
        assert!(
            scripts
                .dispatch
                .aboard_presentation
                .presentation_gate_active
        );
        assert!(
            scripts
                .dispatch
                .transfer_presentation
                .presentation_gate_active
        );
        assert_eq!(
            scripts.service.backend().sequence_context(),
            SequenceRequestContext {
                ship_active: true,
                scene_gate_active: true,
            }
        );
        assert!(scripts.service.presentation_state().name_lookup_enabled);
        assert!(scripts.text_presentation().dialogue_chatter_active);
        assert_eq!(
            scripts.backend().environment_activity(),
            ScriptEnvironmentActivity {
                bridge_active: true,
                travel_active: true,
                contact_active: true,
            }
        );

        let presentation = scripts.service.presentation_state_mut();
        presentation.active = false;
        presentation.c2_gate_active = false;
        presentation.word_choice_active = false;
        presentation.start_locked = false;
        presentation.hold_ready = false;
        presentation.dialogue_hold_complete = false;
        presentation.ui_busy = true;
        scripts.dispatch.text_presentation.subtitle_display_active = false;
        scripts.dispatch.text_presentation.menu_deferred = false;
        scripts.dispatch.text_presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::default();
        scripts.dispatch.text_presentation.subtitle_word_list_mode = false;
        scripts.dispatch.text_presentation.subtitle_voice_trigger = false;
        scripts.dispatch.text_presentation.dialogue_chatter_active = false;
        scripts.dispatch.text_presentation.menu_pending = false;
        scripts.dispatch.text_presentation.menu_word_count = 2;
        scripts.presentation_word_buffer_nonempty = true;
        scripts.dispatch.text_presentation.dialogue_hold_countdown = 4;
        scripts.dispatch.record_clear_presentation.sequence_active = false;
        let code = decode_script_code(&[
            PROFILE_REQUEST_OPCODE,
            REQUESTED_PROFILE_NUMBER,
            SCRIPT_END_OPCODE,
        ])
        .unwrap();
        let request = decode_script_profile_request(&code.tokens()[0]).unwrap();
        scripts.dispatch.profile_request.schedule(request);

        lifecycle.vm_execution_enabled = true;
        scripts.dispatch.pending_vm_execution_write = Some(false);
        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();
        assert!(!lifecycle.vm_execution_enabled);
        assert_eq!(scripts.dispatch.pending_vm_execution_write, None);

        assert!(!lifecycle.presentation.active);
        assert!(!lifecycle.presentation.c2_presentation_gate);
        assert!(!lifecycle.presentation.sequence_active);
        assert!(!lifecycle.presentation.word_choice_active);
        assert!(!lifecycle.presentation.start_locked);
        assert!(!lifecycle.presentation.hold_ready);
        assert!(!lifecycle.presentation.dialogue_hold_complete);
        assert!(!lifecycle.presentation.subtitle_display_active);
        assert!(!lifecycle.presentation.menu_deferred);
        assert_eq!(lifecycle.presentation.request_flags.bits(), u8::MIN);
        assert!(!lifecycle.presentation.subtitle_word_list_mode);
        assert!(!lifecycle.presentation.subtitle_voice_trigger);
        assert!(!lifecycle.presentation.dialogue_chatter_active);
        assert!(!lifecycle.presentation.text_menu_pending);
        assert!(lifecycle.presentation.word_buffer_nonempty);
        assert_eq!(lifecycle.presentation.dialogue_hold_countdown, 4);
        assert!(lifecycle.profile_ui_blocked());
        assert_eq!(lifecycle.pending_profile, ScriptProfileId::new(2));
        // A late renderer must not replay a consumed script pause over a UI write.
        lifecycle.vm_execution_enabled = true;
        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();
        assert!(lifecycle.vm_execution_enabled);
    }

    #[test]
    #[ignore = "requires the original sequel database, profile and object clips"]
    fn sequel_inventory_descriptor_runtime_backend_resolves_all_authored_clips() {
        use crate::native::bloodprg::ScriptDispatchHost;
        use commander_blood_formats::script::decode_script_directory;
        use serde::Deserialize;
        use sha2::{Digest, Sha256};

        #[derive(Deserialize)]
        struct Vector {
            #[serde(default)]
            authored: bool,
            name: Vec<u8>,
            video: Vec<u8>,
            database_sha256: Option<String>,
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let bytes = std::fs::read(root.join("DESCRIPT.DES")).unwrap();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let database = DescriptDatabase::parse(&bytes).unwrap();
        let directory =
            decode_script_directory(&std::fs::read(root.join("SCRIPT1.DEB")).unwrap()).unwrap();
        let store = OriginalResourceStore::new(root, None, [], true);
        let backend = RuntimeScriptBackend {
            database,
            object_names: directory
                .active_objects()
                .map(|(id, entry)| (id, Box::from(entry.name())))
                .collect(),
            assets: DescriptPresentationAssets::default(),
            backgrounds: DescriptBackgroundCache::default(),
            background_source: RuntimeBackgroundSource::new(store.clone()),
            sound_loader: RuntimeSoundBankLoader::new(store.clone()),
            idle_source: RuntimeIdleClipSource::new(store.clone()),
            environment_activity: ScriptEnvironmentActivity::default(),
            clock: TEST_CLOCK,
            sequence_context: SequenceRequestContext::default(),
            navigation_context: None,
            action_runtime_state: ScriptActionRuntimeState::default(),
            presentation_interface_active: false,
            active_description_object: None,
            last_descript_application: None,
            commands: Vec::new(),
        };
        let mut service = ScriptExecutionService::new(backend);
        let mut text = TextPresentationState::default();
        let mut count = 0;
        for vector in include_str!(
            "../../../../re/tools/oracle_vectors/big_bug_bang_inventory_descriptor.jsonl"
        )
        .lines()
        .map(|line| serde_json::from_str::<Vector>(line).unwrap())
        .filter(|vector| vector.authored)
        {
            assert_eq!(vector.database_sha256.as_deref(), Some(hash.as_str()));
            let object = directory.find_active_object(&vector.name).unwrap();
            service.presentation_state_mut().start_locked = true;
            assert_eq!(
                service
                    .lookup_inventory_description(object, &vector.name, &mut text)
                    .unwrap(),
                Some(true)
            );
            assert!(!service.presentation_state().start_locked);
            assert_eq!(service.backend().active_description_object(), Some(object));
            assert_eq!(
                service.backend().assets().object_scene_video(),
                Some(vector.video.as_slice())
            );
            let resource =
                prefixed_resource_name(OBJECT_VIDEO_RESOURCE_DIRECTORY, &vector.video).unwrap();
            assert!(!store.load(&resource).unwrap().is_empty(), "{resource:?}");
            assert!(
                !service
                    .backend()
                    .last_descript_application()
                    .unwrap()
                    .sound_bank_loaded()
            );
            assert!(service.backend().pending_commands().is_empty());
            count += 1;
        }
        assert_eq!(count, 25);
    }

    #[test]
    fn dialogue_reveal_words_do_not_alias_the_native_presentation_choice_buffer() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        scripts.dispatch.text_presentation.menu_word_count = 11;

        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();

        assert!(!lifecycle.presentation.word_buffer_nonempty);
        scripts.presentation_word_buffer_nonempty = true;
        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();
        assert!(lifecycle.presentation.word_buffer_nonempty);
    }

    #[test]
    fn canonical_ui_bit_controls_descript_audio_and_idle_loading() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_presentation_interface_active(true);
        scripts.prepare_lifecycle_frame(&lifecycle);

        scripts
            .apply_presentation_description(IZWALITO_NAME)
            .unwrap()
            .expect("Izwalito must have an authored DESCRIPT record");

        assert!(scripts.backend().assets().encoded_idle_video().is_none());
        assert!(
            scripts
                .backend()
                .loaded_streamed_sound_bank_resource()
                .is_none()
        );

        lifecycle.set_presentation_interface_active(false);
        scripts.prepare_lifecycle_frame(&lifecycle);
        scripts
            .apply_presentation_description(IZWALITO_NAME)
            .unwrap()
            .expect("Izwalito must remain available outside the bridge UI");

        assert!(scripts.backend().assets().encoded_idle_video().is_some());
        assert!(
            scripts
                .backend()
                .loaded_streamed_sound_bank_resource()
                .is_some()
        );
    }

    #[test]
    fn startup_phone_timer_queues_and_answers_izwalito_through_authored_script() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut runtime = super::super::OriginalGameRuntime::new(data);
        scripts
            .load_profile(&mut runtime, ScriptProfileId::INITIAL)
            .unwrap();
        let izwalito = runtime
            .current_profile()
            .unwrap()
            .directory()
            .find_active_object(IZWALITO_NAME)
            .unwrap();
        let timer_slot = ScriptTimerSlot::decode(STARTUP_PHONE_TIMER_SLOT).unwrap();
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.vm_execution_enabled = true;
        lifecycle.presentation.scene_gate_active = true;
        lifecycle.set_presentation_interface_active(true);

        scripts
            .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
            .unwrap();
        assert_eq!(
            runtime
                .current_profile()
                .unwrap()
                .runtime()
                .timer(timer_slot),
            STARTUP_PHONE_TIMER_VALUE
        );

        let mut timer = GameTimerState::default();
        timer.start();
        for _ in usize::MIN..STARTUP_PHONE_FRAME_LIMIT {
            for _ in usize::MIN..TIMER_TICKS_PER_GAME_FRAME {
                advance_game_timer_tick(
                    &mut timer,
                    runtime.current_profile_mut().unwrap().runtime_mut(),
                    GameTimerContext::default(),
                );
            }
            scripts
                .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
                .unwrap();
            if scripts.action_state().pending_presentation_owner == Some(izwalito) {
                break;
            }
        }

        assert_eq!(
            scripts.action_state().pending_presentation_owner,
            Some(izwalito),
            "SCRIPT1 never queued the startup Izwalito phone presentation"
        );
        assert!(!lifecycle.presentation.active);

        scripts.action_state_mut().pending_presentation_owner = None;
        scripts.defer_actor_presentation(izwalito);
        scripts
            .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
            .unwrap();

        let dialogue = scripts
            .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
            .unwrap();
        assert!(lifecycle.presentation.active);
        assert_eq!(
            scripts.backend().active_description_object(),
            Some(izwalito)
        );
        assert_eq!(
            scripts
                .backend()
                .assets()
                .idle_clip()
                .unwrap()
                .video()
                .as_bytes(),
            IZWALITO_IDLE_VIDEO
        );
        assert_ne!(dialogue.presentation_yields, usize::MIN);
        let text = scripts.text_presentation();
        assert!(!text.subtitle_display_active);
        assert!(text.menu_deferred);
        assert_eq!(text.selected_line, Some(TEXT_ONLY_PRESENTATION_SELECTOR));
        let first_word = match text.menu_words.first() {
            Some(ScriptTextWord::Dictionary(word)) => *word,
            other => panic!("unexpected first Izwalito word: {other:?}"),
        };
        assert_eq!(
            runtime
                .current_profile()
                .unwrap()
                .dictionary()
                .word(first_word),
            Some(IZWALITO_FIRST_WORD)
        );
        assert_eq!(
            runtime
                .current_profile()
                .unwrap()
                .active_actor_presentation_related(),
            Some(izwalito)
        );

        let unrelated = scripts
            .backend()
            .object_names
            .keys()
            .copied()
            .find(|object| {
                *object != izwalito && scripts.backend().object_has_description(*object).unwrap()
            })
            .expect("SCRIPT1 must contain another DESCRIPT-backed object");
        scripts
            .apply_object_description(unrelated)
            .unwrap()
            .unwrap();
        assert_eq!(
            scripts.backend().active_description_object(),
            Some(unrelated)
        );
        scripts
            .apply_presentation_description(IZWALITO_NAME)
            .unwrap()
            .expect("name-based Izwalito lookup must resolve its DESCRIPT record");
        assert_eq!(
            scripts.backend().active_description_object(),
            Some(izwalito),
            "name-based presentation lookup retained stale DESCRIPT ownership"
        );
        assert!(
            scripts
                .apply_presentation_description(b"missing-descript-record")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            scripts.backend().active_description_object(),
            None,
            "a missing name-based DESCRIPT lookup retained stale ownership"
        );
        assert_eq!(
            runtime
                .current_profile()
                .unwrap()
                .active_actor_presentation_related(),
            Some(izwalito),
            "a later DESCRIPT lookup must not replace blood's live C4 owner"
        );
    }

    #[test]
    fn completed_word_choice_publishes_the_concept_and_releases_text_ownership() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut runtime = super::super::OriginalGameRuntime::new(data);
        scripts
            .load_profile(&mut runtime, ScriptProfileId::INITIAL)
            .unwrap();
        let selected_concept = runtime
            .current_profile()
            .unwrap()
            .dictionary()
            .words()
            .next()
            .unwrap()
            .0;

        scripts.service.presentation_state_mut().word_choice_active = true;
        let text = &mut scripts.dispatch.text_presentation;
        text.menu_deferred = true;
        text.subtitle_display_active = true;
        text.dialogue_hold_complete = true;
        text.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(ALL_PRESENTATION_REQUESTS);
        text.menu_words = Box::new([ScriptTextWord::Dictionary(selected_concept)]);
        text.menu_word_count = text.menu_words.len();

        scripts
            .complete_word_choice(&mut runtime, selected_concept)
            .unwrap();

        assert_eq!(
            runtime
                .current_profile()
                .unwrap()
                .runtime()
                .selected_concept(),
            Some(selected_concept)
        );
        assert!(!scripts.service.presentation_state().word_choice_active);
        let text = &scripts.dispatch.text_presentation;
        assert!(!text.menu_deferred);
        assert!(!text.subtitle_display_active);
        assert!(!text.dialogue_hold_complete);
        assert!(!text.request_flags.text_request_pending());
        assert!(text.request_flags.secondary_request_pending());
        assert!(text.menu_words.is_empty());
        assert_eq!(text.menu_word_count, usize::MIN);
    }

    #[test]
    fn completed_word_choice_preserves_unrelated_main_loop_ownership() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut scripts = RuntimeScriptSystem::new(runtime.data(), TEST_CLOCK);
        runtime.load_profile(ScriptProfileId::INITIAL).unwrap();
        let selected_concept = runtime
            .current_profile()
            .unwrap()
            .dictionary()
            .words()
            .next()
            .unwrap()
            .0;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);
        lifecycle.set_presentation_interface_active(true);
        lifecycle.presentation.active = true;
        lifecycle.presentation.c2_presentation_gate = true;
        lifecycle.presentation.scene_gate_active = true;
        lifecycle.presentation.sequence_active = true;
        lifecycle.presentation.word_choice_active = true;
        lifecycle.presentation.menu_deferred = true;
        lifecycle.presentation.subtitle_display_active = true;
        lifecycle.presentation.dialogue_hold_complete = true;
        lifecycle.presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(3);

        scripts
            .complete_lifecycle_word_choice(&mut runtime, &mut lifecycle, selected_concept)
            .unwrap();

        assert!(lifecycle.modal_ui_busy());
        assert!(lifecycle.presentation_interface_active());
        assert!(lifecycle.presentation.active);
        assert!(lifecycle.presentation.c2_presentation_gate);
        assert!(lifecycle.presentation.scene_gate_active);
        assert!(lifecycle.presentation.sequence_active);
        assert!(!lifecycle.presentation.word_choice_active);
        assert!(!lifecycle.presentation.menu_deferred);
        assert!(!lifecycle.presentation.subtitle_display_active);
        assert!(!lifecycle.presentation.dialogue_hold_complete);
        assert!(!lifecycle.presentation.request_flags.text_request_pending());
        assert!(
            lifecycle
                .presentation
                .request_flags
                .secondary_request_pending()
        );
    }

    #[test]
    fn late_text_completion_is_not_replaced_by_the_pre_renderer_lifecycle_snapshot() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        scripts.dispatch.text_presentation.dialogue_hold_complete = true;
        scripts.dispatch.text_presentation.dialogue_hold_countdown = 17;

        scripts.finish_lifecycle_text_frame(&mut lifecycle).unwrap();

        assert!(lifecycle.presentation.dialogue_hold_complete);
        assert_eq!(lifecycle.presentation.dialogue_hold_countdown, 17);
        assert!(scripts.dispatch.text_presentation.dialogue_hold_complete);
        assert!(scripts.service.presentation_state().dialogue_hold_complete);
    }

    #[test]
    fn main_loop_hold_completion_clear_reaches_the_late_text_renderer() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        scripts.dispatch.text_presentation.dialogue_hold_complete = true;
        scripts.dispatch.text_presentation.menu_deferred = true;
        lifecycle.presentation.dialogue_hold_complete = false;
        lifecycle.presentation.menu_deferred = false;

        scripts.prepare_lifecycle_text_frame(&lifecycle);
        scripts.finish_lifecycle_text_frame(&mut lifecycle).unwrap();

        assert!(!scripts.dispatch.text_presentation.dialogue_hold_complete);
        assert!(!scripts.dispatch.text_presentation.menu_deferred);
        assert!(!scripts.service.presentation_state().dialogue_hold_complete);
        assert!(!lifecycle.presentation.dialogue_hold_complete);
        assert!(!lifecycle.presentation.menu_deferred);
    }

    #[test]
    fn main_loop_presentation_writes_survive_a_scene_state_exchange() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);
        lifecycle.set_presentation_interface_active(true);
        lifecycle.presentation.active = true;
        lifecycle.presentation.c2_presentation_gate = true;
        lifecycle.presentation.word_choice_active = true;
        lifecycle.presentation.start_locked = true;
        lifecycle.presentation.hold_ready = true;
        lifecycle.presentation.dialogue_hold_complete = true;
        lifecycle.presentation.subtitle_display_active = true;
        lifecycle.presentation.menu_deferred = true;
        lifecycle.presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(3);
        lifecycle.presentation.subtitle_word_list_mode = true;
        lifecycle.presentation.subtitle_voice_trigger = true;
        lifecycle.presentation.dialogue_chatter_active = true;
        lifecycle.presentation.text_menu_pending = true;
        lifecycle.presentation.text_selector = Some(20);
        lifecycle.presentation.dialogue_hold_countdown = 17;
        lifecycle.presentation.sequence_active = true;

        scripts.prepare_lifecycle_frame(&lifecycle);
        let presentation = scripts.presentation_scan_state().clone();
        let text = scripts.text_presentation().clone();
        *scripts.presentation_scan_state_mut() = presentation;
        *scripts.text_presentation_mut() = text;
        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();

        assert!(lifecycle.modal_ui_busy());
        assert!(lifecycle.presentation_interface_active());
        assert!(lifecycle.presentation.active);
        assert!(lifecycle.presentation.c2_presentation_gate);
        assert!(lifecycle.presentation.word_choice_active);
        assert!(lifecycle.presentation.start_locked);
        assert!(lifecycle.presentation.hold_ready);
        assert!(lifecycle.presentation.dialogue_hold_complete);
        assert!(lifecycle.presentation.subtitle_display_active);
        assert!(lifecycle.presentation.menu_deferred);
        assert_eq!(lifecycle.presentation.request_flags.bits(), 3);
        assert!(lifecycle.presentation.subtitle_word_list_mode);
        assert!(lifecycle.presentation.subtitle_voice_trigger);
        assert!(lifecycle.presentation.dialogue_chatter_active);
        assert!(lifecycle.presentation.text_menu_pending);
        assert_eq!(lifecycle.presentation.text_selector, Some(20));
        assert_eq!(lifecycle.presentation.dialogue_hold_countdown, 17);
        assert!(lifecycle.presentation.sequence_active);
    }

    #[test]
    fn main_loop_hold_release_replaces_stale_text_owner_state() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        scripts.dispatch.text_presentation.subtitle_display_active = true;
        scripts.dispatch.text_presentation.hold_ready = false;

        let mut lifecycle = GameLifecycleState::default();
        lifecycle.presentation.active = true;
        lifecycle.presentation.subtitle_display_active = false;
        lifecycle.presentation.hold_ready = true;

        scripts.prepare_lifecycle_frame(&lifecycle);

        assert!(!scripts.text_presentation().subtitle_display_active);
        assert!(scripts.text_presentation().hold_ready);
        assert!(scripts.presentation_scan_state().hold_ready);
    }

    #[test]
    fn presentation_palette_dirty_is_a_one_shot_runtime_alias() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        scripts.presentation_scan_state_mut().palette_dirty = true;

        assert!(scripts.take_presentation_palette_dirty());
        assert!(!scripts.take_presentation_palette_dirty());
    }

    #[test]
    fn script_side_effects_remain_ordered_until_the_lifecycle_consumes_them() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);
        backend.action_runtime_state.voc_playback_enabled = true;

        backend.restart_name_area_effect().unwrap();
        backend
            .play_radio_clip(RADIO_CLIP_PLAYBACK_COUNTDOWN)
            .unwrap();
        assert_eq!(
            backend.action_runtime_state.clip_playback_state,
            RADIO_CLIP_PLAYBACK_COUNTDOWN
        );
        backend
            .start_camera_transition(crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS)
            .unwrap();
        backend.reset_ship_hud().unwrap();

        assert_eq!(
            backend.take_commands(),
            vec![
                RuntimeScriptCommand::RestartNameAreaEffect,
                RuntimeScriptCommand::PlayRadioClip {
                    clip_index: RADIO_CLIP_INDEX,
                },
                RuntimeScriptCommand::StartCameraTransition {
                    steps: crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS,
                },
                RuntimeScriptCommand::ResetShipHud,
            ]
        );
        assert!(backend.pending_commands().is_empty());
    }

    #[test]
    fn action_outputs_are_consumed_exactly_once() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let action = scripts.action_state_mut();
        action.ship_hud_refresh_requested = true;
        action.active_line = Some(ScriptActionPresentationLine::NavigationTarget);
        action.screen_rebuild_requested = true;
        action.clip_playback_state_reload = Some(RADIO_CLIP_PLAYBACK_COUNTDOWN);
        action.speaker_pulse_requested = true;

        let effects = scripts.take_action_effects(false);
        assert!(effects.ship_hud_refresh_requested);
        assert_eq!(scripts.action_state().active_line, None);
        assert!(!effects.screen_rebuild_requested);
        assert_eq!(effects.clip_playback_state_reload, None);
        assert!(!effects.speaker_pulse_requested);
        assert!(scripts.action_state().screen_rebuild_requested);
        assert_eq!(
            scripts.action_state().clip_playback_state_reload,
            Some(RADIO_CLIP_PLAYBACK_COUNTDOWN)
        );
        assert!(scripts.action_state().speaker_pulse_requested);

        let effects = scripts.take_action_effects(true);
        assert!(!effects.ship_hud_refresh_requested);
        assert!(effects.screen_rebuild_requested);
        assert_eq!(
            effects.clip_playback_state_reload,
            Some(RADIO_CLIP_PLAYBACK_COUNTDOWN)
        );
        assert!(effects.speaker_pulse_requested);
        assert_eq!(
            scripts.take_action_effects(true),
            RuntimeScriptActionEffects::default()
        );
    }

    #[test]
    fn canonical_vm_active_line_uses_the_last_script_or_scan_write() {
        let mut dispatch = ScriptDispatchState::default();
        dispatch.record_active_line_write(PresentationResourceLine::Sequence.number());
        dispatch.record_active_line_write(ScriptAboardPresentationLine::ActorArrived.number());
        dispatch.record_active_line_write(ScriptTransferPresentationLine::InventoryMoved.number());
        assert_eq!(
            dispatch.pending_active_line_write(),
            Some(ScriptTransferPresentationLine::InventoryMoved.number())
        );
        dispatch.record_active_line_write(ScriptActionPresentationLine::NavigationTarget.number());

        let mut lifecycle = GameLifecycleState::default();
        let mut ship = ShipPresentationState::default();
        publish_script_active_line_to_lifecycle(&dispatch, &mut lifecycle.presentation);
        publish_script_active_line_to_ship(&mut dispatch, &mut ship);

        let expected = ScriptActionPresentationLine::NavigationTarget.number();
        assert_eq!(lifecycle.presentation.active_line, Some(expected));
        assert_eq!(ship.active_line, expected);
        assert_eq!(dispatch.pending_active_line_write(), None);
    }

    #[test]
    fn bridge_c3_queue_promotes_to_an_actionable_c4_and_starts_honk() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        let mut runtime = super::super::OriginalGameRuntime::new(data);
        scripts
            .load_profile(&mut runtime, ScriptProfileId::INITIAL)
            .unwrap();
        let (player, honk, player_slot) = {
            let profile = runtime.current_profile().unwrap();
            let builtins = profile.builtins();
            let player = builtins.player.unwrap();
            let honk = builtins.horn.unwrap();
            let player_kind = profile.state().object(player).unwrap().kind;
            let action_offset =
                script_field_offset(player_kind, ScriptFieldSelector::ACTION).unwrap();
            let player_slot = profile
                .state()
                .object_word_triple(player, action_offset / size_of::<u16>())
                .unwrap();
            (player, honk, player_slot)
        };

        scripts.defer_presentation_queue(honk);
        assert_eq!(
            scripts.presentation_scan_state().deferred,
            ScriptDeferredRecord::Complete {
                record: ScriptActionRecord::PresentationQueue(honk),
                actionable: true,
            }
        );

        scripts.execute_frame(&mut runtime, true).unwrap();
        let first_scan = scripts.last_presentation_outcome().unwrap();
        assert_eq!(first_scan.deferred_destination, Some(player_slot));
        assert!(first_scan.actions.iter().any(|action| {
            action.owner == player && action.record == ScriptActionRecord::PresentationQueue(honk)
        }));
        let first_records = &runtime
            .current_profile()
            .unwrap()
            .record_state()
            .action_records;
        assert_eq!(
            first_records.record(player_slot),
            ScriptActionRecord::ActorPresentation(honk)
        );
        assert!(first_records.is_actionable(player_slot));

        scripts.execute_frame(&mut runtime, true).unwrap();
        let second_scan = scripts.last_presentation_outcome().unwrap();
        assert_eq!(second_scan.presentation_started, Some(honk));
        assert!(second_scan.actions.iter().any(|action| {
            action.owner == player && action.record == ScriptActionRecord::ActorPresentation(honk)
        }));
        assert!(scripts.presentation_scan_state().active);
        assert!(scripts.action_state().post_update_object == Some(honk));
        assert!(
            !runtime
                .current_profile()
                .unwrap()
                .record_state()
                .action_records
                .is_actionable(player_slot)
        );
    }

    #[test]
    fn every_shipped_profile_executes_with_the_concrete_runtime_backend() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, TEST_CLOCK);
        scripts
            .backend_mut()
            .set_environment_activity(ScriptEnvironmentActivity {
                bridge_active: true,
                travel_active: true,
                contact_active: true,
            });
        scripts
            .backend_mut()
            .set_sequence_context(SequenceRequestContext {
                ship_active: true,
                scene_gate_active: true,
            });
        let mut runtime = super::super::OriginalGameRuntime::new(data);
        let mut executed_profile_count = usize::MIN;

        for profile_id in ScriptProfileId::all() {
            scripts.load_profile(&mut runtime, profile_id).unwrap();
            let builtins = runtime.current_profile().unwrap().builtins();
            scripts.backend_mut().set_navigation_context(
                builtins
                    .player
                    .zip(builtins.archetype)
                    .map(|(player, arche)| ScriptRecordStateNavigationContext {
                        primary_object: player,
                        secondary_object: player,
                        arche,
                    }),
            );
            let outcome = scripts
                .execute_frame(&mut runtime, true)
                .unwrap_or_else(|error| {
                    panic!(
                        "profile {} runtime script system failed: {error:?}",
                        profile_id.value() + 1
                    )
                });
            assert_ne!(outcome.end, ScriptFrameEnd::ExecutionDisabled);
            runtime
                .current_profile_mut()
                .unwrap()
                .synchronized_state()
                .unwrap();
            executed_profile_count += 1;
        }

        assert_eq!(executed_profile_count, ORIGINAL_SCRIPT_PROFILE_COUNT);
    }

    fn original_data_paths() -> Option<OriginalGameDataPaths> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root: PathBuf| OriginalGameDataPaths::from_root(root).ok())
    }
}
