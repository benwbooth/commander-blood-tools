//! Concrete original-resource and host-command backend for translated BloodScript.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::descript::DescriptRecordKind;
use commander_blood_formats::descript_database::DescriptDatabase;
use commander_blood_formats::script::{ScriptObjectId, ScriptWordId};

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    DescriptApplicationContext, DescriptBackgroundCache, DescriptBackgroundSource,
    DescriptIdleClipSource, DescriptPresentationAssets, DescriptRecordApplication,
    DescriptSoundBankLoader, GameLifecycleState, LoadedScriptProfile, ScriptAboardRecordContext,
    ScriptActionRecord, ScriptActionState, ScriptClock, ScriptDeferredRecord, ScriptDispatchState,
    ScriptEnvironmentActivity, ScriptExecutionBackend, ScriptExecutionService, ScriptFrameOutcome,
    ScriptPresentationEntity, ScriptPresentationScanOutcome, ScriptPresentationScanState,
    ScriptProfileId, ScriptProfileLoadOutcome, ScriptRecordStateNavigationContext,
    ScriptShipNavigationMode, ScriptTransferContext, SequencePresentationState,
    SequenceRequestContext, TextPresentationState, deferred_navigation_record,
    execute_loaded_script_frame, lookup_and_apply_descript_record,
};

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
    /// Start the black-hole camera transition.
    StartCameraTransition,
    /// Rebuild the ship HUD and reset its 3D camera state.
    ResetShipHud,
}

/// Complete profile-independent state surrounding translated BloodScript execution.
pub struct RuntimeScriptSystem {
    dispatch: ScriptDispatchState,
    service: ScriptExecutionService<RuntimeScriptBackend>,
}

impl RuntimeScriptSystem {
    /// Construct the script system from validated original data and a host clock.
    pub fn new(data: &OriginalGameData, clock: ScriptClock) -> Self {
        Self {
            dispatch: ScriptDispatchState::default(),
            service: ScriptExecutionService::new(RuntimeScriptBackend::new(data, clock)),
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
        self.service.backend_mut().bind_profile(
            runtime
                .current_profile()
                .context("profile loader did not retain the selected profile")?,
        );
        Ok(outcome)
    }

    /// Execute one complete translated COD/BAS/presentation frame.
    pub fn execute_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        enabled: bool,
    ) -> Result<ScriptFrameOutcome> {
        execute_loaded_script_frame(
            runtime
                .current_profile_mut()
                .context("no BloodScript profile is loaded")?,
            enabled,
            &mut self.dispatch,
            &mut self.service,
        )
        .map_err(|error| anyhow!("executing BloodScript frame: {error:?}"))
    }

    /// Import globals owned by the recovered main loop before one VM pass.
    pub fn prepare_lifecycle_frame(&mut self, lifecycle: &GameLifecycleState) {
        let source = &lifecycle.presentation;
        {
            let presentation = self.service.presentation_state_mut();
            presentation.active = source.active;
            presentation.c2_gate_active = source.c2_presentation_gate;
            presentation.word_choice_active = source.word_choice_active;
            presentation.start_locked = source.start_locked;
            presentation.hold_ready = source.hold_ready;
            presentation.dialogue_hold_complete = source.dialogue_hold_complete;
            presentation.name_lookup_enabled = lifecycle.presentation_interface_active();
            self.dispatch.import_presentation_scan_state(presentation);
        }
        let text = &mut self.dispatch.text_presentation;
        text.subtitle_display_active = source.subtitle_display_active;
        text.menu_deferred = source.menu_deferred;
        text.request_flags = source.request_flags;
        text.subtitle_word_list_mode = source.subtitle_word_list_mode;
        text.subtitle_voice_trigger = source.subtitle_voice_trigger;
        text.menu_pending = source.text_menu_pending;
        text.dialogue_hold_countdown = source.dialogue_hold_countdown;
        self.dispatch.record_clear_presentation.sequence_active = source.sequence_active;

        self.service
            .backend_mut()
            .set_sequence_context(SequenceRequestContext {
                ship_active: source.ship_active,
                scene_gate_active: source.scene_gate_active,
            });
        self.service
            .backend_mut()
            .set_ship_interface_active(lifecycle.presentation_interface_active());
    }

    /// Publish BloodScript writes to the recovered main-loop globals after a VM pass.
    pub fn finish_lifecycle_frame(&self, lifecycle: &mut GameLifecycleState) -> Result<()> {
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
        target.text_menu_pending = text.menu_pending;
        target.word_buffer_nonempty = text.menu_word_count != usize::MIN;
        target.dialogue_hold_countdown = text.dialogue_hold_countdown;
        target.sequence_active = self.dispatch.record_clear_presentation.sequence_active;
        lifecycle.set_modal_ui_busy(presentation.ui_busy);
        lifecycle.pending_profile = self
            .dispatch
            .profile_request
            .pending_profile()
            .context("resolving BloodScript profile request")?;
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
        self.service.presentation_state_mut().word_choice_active = false;
        let text = &mut self.dispatch.text_presentation;
        text.menu_deferred = false;
        text.subtitle_display_active = false;
        text.dialogue_hold_complete = false;
        text.request_flags.clear_pending_requests();
        text.menu_words = Box::new([]);
        text.menu_word_count = usize::MIN;
        Ok(())
    }

    /// Apply a presentation-panel DESCRIPT record through the live script state.
    pub fn apply_presentation_description(
        &mut self,
        name: &[u8],
    ) -> Result<Option<DescriptRecordApplication>> {
        let Self { dispatch, service } = self;
        service
            .backend_mut()
            .apply_description(name, true, &mut dispatch.text_presentation)
    }

    /// Apply a DESCRIPT record by stable profile object identity.
    pub fn apply_object_description(
        &mut self,
        object: ScriptObjectId,
    ) -> Result<Option<DescriptRecordApplication>> {
        let Self { dispatch, service } = self;
        let name = service.backend().object_name(object)?.to_vec();
        let application = service.backend_mut().apply_description(
            &name,
            true,
            &mut dispatch.text_presentation,
        )?;
        service.backend_mut().active_description_object = application.map(|_| object);
        Ok(application)
    }

    /// Apply one object's DESCRIPT record to state owned by a frame-tail coordinator.
    pub fn apply_object_description_to_text(
        &mut self,
        object: ScriptObjectId,
        text: &mut TextPresentationState,
    ) -> Result<Option<DescriptRecordApplication>> {
        let name = self.service.backend().object_name(object)?.to_vec();
        let application = self
            .service
            .backend_mut()
            .apply_description(&name, true, text)?;
        self.service.backend_mut().active_description_object = application.map(|_| object);
        Ok(application)
    }

    /// Queue the complete typed C1 action emitted by a ship-HUD target choice.
    pub fn defer_navigation_target(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = deferred_navigation_record(target, true);
    }

    /// Queue the non-actionable C4 record emitted by the ship navigation coordinator.
    pub fn defer_actor_presentation(&mut self, target: ScriptObjectId) {
        self.service.presentation_state_mut().deferred = ScriptDeferredRecord::Complete {
            record: ScriptActionRecord::ActorPresentation(target),
            actionable: false,
        };
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
    selector_root: Option<commander_blood_formats::code::ScriptCodeOffset>,
    ship_interface_active: bool,
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
            selector_root: None,
            ship_interface_active: false,
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
        self.selector_root = None;
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

    /// Bind the active BAS selector root used by concept commits.
    pub fn set_selector_root(
        &mut self,
        root: Option<commander_blood_formats::code::ScriptCodeOffset>,
    ) {
        self.selector_root = root;
    }

    /// Update whether the ship interface suppresses new transfer presentations.
    pub fn set_ship_interface_active(&mut self, active: bool) {
        self.ship_interface_active = active;
    }

    /// Borrow the current DESCRIPT-selected presentation assets.
    pub const fn assets(&self) -> &DescriptPresentationAssets {
        &self.assets
    }

    /// Borrow the four-slot original background cache.
    pub const fn backgrounds(&self) -> &DescriptBackgroundCache {
        &self.backgrounds
    }

    /// Borrow authored background paths that the original data set cannot resolve.
    pub fn missing_background_resources(&self) -> &[Box<[u8]>] {
        &self.background_source.missing_resources
    }

    /// Borrow the most recently loaded SND bank and its encoded bytes.
    pub fn loaded_sound_bank(&self) -> Option<&LoadedRuntimeResource> {
        self.sound_loader.loaded.as_ref()
    }

    /// Load one authored resident SND bank through the same resource service as DESCRIPT.
    pub fn load_resident_sound_bank(&mut self, bank_name: &[u8]) -> Result<()> {
        self.sound_loader.load_sound_bank(bank_name)
    }

    /// Restore the complete resident bank captured before a temporary overlay swap.
    pub fn restore_resident_sound_bank(&mut self, bank: LoadedRuntimeResource) {
        self.sound_loader.loaded = Some(bank);
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

    fn aboard_context(&mut self, related: ScriptObjectId) -> Result<ScriptAboardRecordContext> {
        Ok(ScriptAboardRecordContext {
            ship_interface_active: self.ship_interface_active,
            descriptor_available: self.object_has_description(related)?,
        })
    }

    fn transfer_context(&mut self, item: ScriptObjectId) -> Result<ScriptTransferContext> {
        Ok(ScriptTransferContext {
            ship_interface_active: self.ship_interface_active,
            descriptor_available: self.object_has_description(item)?,
        })
    }

    fn selector_root(&self) -> Option<commander_blood_formats::code::ScriptCodeOffset> {
        self.selector_root
    }

    fn lookup_presentation_description(
        &mut self,
        related: ScriptObjectId,
        name: &[u8],
        text: &mut TextPresentationState,
    ) -> Result<()> {
        self.validate_object_name(related, name)?;
        let application = self.apply_description(name, true, text)?;
        self.active_description_object = application.map(|_| related);
        Ok(())
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

    fn description_available(&mut self, object: ScriptObjectId, name: &[u8]) -> Result<bool> {
        self.validate_object_name(object, name)?;
        Ok(self.database.lookup(name).is_some())
    }

    fn restart_navigation_music(&mut self) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::RestartNavigationMusic);
        Ok(())
    }

    fn play_radio_clip(&mut self) -> Result<()> {
        self.commands.push(RuntimeScriptCommand::PlayRadioClip {
            clip_index: RADIO_CLIP_INDEX,
        });
        Ok(())
    }

    fn start_camera_transition(&mut self) -> Result<()> {
        self.commands
            .push(RuntimeScriptCommand::StartCameraTransition);
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
}

impl RuntimeSoundBankLoader {
    fn new(store: OriginalResourceStore) -> Self {
        Self {
            store,
            loaded: None,
        }
    }
}

impl DescriptSoundBankLoader for RuntimeSoundBankLoader {
    type Error = anyhow::Error;

    fn load_sound_bank(&mut self, bank_name: &[u8]) -> Result<()> {
        let encoded_bytes =
            load_prefixed_resource(&self.store, SOUND_BANK_RESOURCE_DIRECTORY, bank_name)?;
        self.loaded = Some(LoadedRuntimeResource::new(bank_name, encoded_bytes));
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
    use commander_blood_formats::instruction::{ScriptTextWord, decode_script_profile_request};

    use crate::native::bloodprg::{ORIGINAL_SCRIPT_PROFILE_COUNT, ScriptFrameEnd, ScriptProfileId};

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
    const PROFILE_REQUEST_OPCODE: u8 = 0xD2;
    const SCRIPT_END_OPCODE: u8 = 0xFF;
    const REQUESTED_PROFILE_NUMBER: u8 = 3;
    const ALL_PRESENTATION_REQUESTS: u8 = 3;
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
                        .loaded_sound_bank()
                        .is_some_and(|resource| !resource.encoded_bytes().is_empty())
                );
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
    fn default_bridge_sound_bank_loads_from_original_resources() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);

        backend
            .load_resident_sound_bank(DEFAULT_BRIDGE_SOUND_BANK)
            .unwrap();
        let resource = backend.loaded_sound_bank().unwrap();
        assert_eq!(resource.name(), DEFAULT_BRIDGE_SOUND_BANK);
        assert!(!resource.encoded_bytes().is_empty());
        commander_blood_formats::snd::SndBank::decode(resource.encoded_bytes()).unwrap();
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
        lifecycle.presentation.text_menu_pending = true;
        lifecycle.presentation.dialogue_hold_countdown = 12;
        lifecycle.set_presentation_interface_active(true);

        scripts.prepare_lifecycle_frame(&lifecycle);

        let presentation = scripts.service.presentation_state();
        assert!(presentation.active);
        assert!(presentation.c2_gate_active);
        assert!(presentation.word_choice_active);
        assert!(presentation.start_locked);
        assert!(presentation.hold_ready);
        assert!(presentation.dialogue_hold_complete);
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
        scripts.dispatch.text_presentation.menu_pending = false;
        scripts.dispatch.text_presentation.menu_word_count = 2;
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

        scripts.finish_lifecycle_frame(&mut lifecycle).unwrap();

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
        assert!(!lifecycle.presentation.text_menu_pending);
        assert!(lifecycle.presentation.word_buffer_nonempty);
        assert_eq!(lifecycle.presentation.dialogue_hold_countdown, 4);
        assert!(lifecycle.profile_ui_blocked());
        assert_eq!(lifecycle.pending_profile, ScriptProfileId::new(2));
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
        assert_eq!(text.request_flags.bits(), u8::MIN);
        assert!(text.menu_words.is_empty());
        assert_eq!(text.menu_word_count, usize::MIN);
    }

    #[test]
    fn script_side_effects_remain_ordered_until_the_lifecycle_consumes_them() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut backend = RuntimeScriptBackend::new(&data, TEST_CLOCK);

        backend.restart_name_area_effect().unwrap();
        backend.play_radio_clip().unwrap();
        backend.start_camera_transition().unwrap();
        backend.reset_ship_hud().unwrap();

        assert_eq!(
            backend.take_commands(),
            vec![
                RuntimeScriptCommand::RestartNameAreaEffect,
                RuntimeScriptCommand::PlayRadioClip {
                    clip_index: RADIO_CLIP_INDEX,
                },
                RuntimeScriptCommand::StartCameraTransition,
                RuntimeScriptCommand::ResetShipHud,
            ]
        );
        assert!(backend.pending_commands().is_empty());
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
