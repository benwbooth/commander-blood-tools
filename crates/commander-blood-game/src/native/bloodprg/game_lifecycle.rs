//! Flat top-level lifecycle translated from the native game coordinator.

use std::error::Error;
use std::fmt;

use super::{PresentationRequestFlags, ScriptProfileId};

const PRIMARY_TEXT_REQUEST_PENDING: u8 = 1;
const POINTER_PRESS_STATE_MASK: u8 = 3;
const SECONDARY_POINTER_PRESS_STATE: u8 = 2;
const DEFAULT_PRESENTATION_LINE: u16 = 8;
const TEXT_PRESENTATION_LINE_BASE: u16 = 9;
const PRESENTATION_INTERFACE_UI_BIT: u16 = 1;
const RESERVED_PROFILE_BLOCKER_UI_BIT: u16 = 2;
const MODAL_BUSY_UI_BIT: u16 = 4;
const NAVIGATION_BUSY_UI_BIT: u16 = 8;

/// Recovered low UI bits shared by presentation, modal, and navigation systems.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct GameUiState {
    /// Native bit 0 enables presentation-interface and DESCRIPT name handling.
    presentation_interface_active: bool,
    /// Native bit 1 is checked by profile switching but has no recovered writer.
    reserved_profile_blocker: bool,
    /// Native bit 2 is shared by modal presentation, save/load, and camera work.
    modal_busy: bool,
    /// Native bit 3 is owned by navigation-choice transitions.
    navigation_busy: bool,
}

impl GameUiState {
    const fn decode_low(word: u16) -> Self {
        Self {
            presentation_interface_active: word & PRESENTATION_INTERFACE_UI_BIT != u16::MIN,
            reserved_profile_blocker: word & RESERVED_PROFILE_BLOCKER_UI_BIT != u16::MIN,
            modal_busy: word & MODAL_BUSY_UI_BIT != u16::MIN,
            navigation_busy: word & NAVIGATION_BUSY_UI_BIT != u16::MIN,
        }
    }

    const fn encode_low(self) -> u16 {
        (if self.presentation_interface_active {
            PRESENTATION_INTERFACE_UI_BIT
        } else {
            u16::MIN
        }) | (if self.reserved_profile_blocker {
            RESERVED_PROFILE_BLOCKER_UI_BIT
        } else {
            u16::MIN
        }) | (if self.modal_busy {
            MODAL_BUSY_UI_BIT
        } else {
            u16::MIN
        }) | (if self.navigation_busy {
            NAVIGATION_BUSY_UI_BIT
        } else {
            u16::MIN
        })
    }

    const fn profile_change_blocked(self) -> bool {
        self.reserved_profile_blocker || self.modal_busy || self.navigation_busy
    }
}

/// Semantic target forwarded to scene and presentation coordinators.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameSceneLink {
    /// Initial bridge presentation context used at startup.
    #[default]
    Initial,
    /// The active subtitle presentation owns scene completion.
    SubtitlePresentation,
    /// A deferred menu presentation owns scene completion.
    DeferredPresentation,
    /// No owner was installed, so the presentation menu buffer is selected.
    MenuWords,
    /// Bridge steering published a queue-link cursor in the native `BP` register.
    BridgePresentation(u16),
}

/// Presentation field activated when the current scene completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePresentationOwner {
    /// Activate the subtitle-display state.
    Subtitle,
    /// Activate the deferred menu state.
    DeferredMenu,
}

impl GamePresentationOwner {
    const fn scene_link(self) -> GameSceneLink {
        match self {
            Self::Subtitle => GameSceneLink::SubtitlePresentation,
            Self::DeferredMenu => GameSceneLink::DeferredPresentation,
        }
    }
}

/// Source currently supplying words to the progressive menu renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameMenuWordSource {
    /// Preserve the source selected by the active script or presentation.
    #[default]
    Current,
    /// Use the presentation scheduler's assembled menu words.
    PresentationBuffer,
}

/// Result returned by one BloodScript VM pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameVmRunStatus {
    /// VM execution may continue.
    Continue,
    /// The script requested game shutdown.
    ExitRequested,
}

/// Result of loading one requested BloodScript profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameProfileLoadStatus {
    /// The requested profile and its typed state loaded successfully.
    Loaded,
    /// Profile selection failed and the game must shut down.
    Failed,
}

/// Conditions that postpone a pending profile change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameProfileChangeBlockers {
    /// A render update still owns shared scene state.
    pub render_update_active: bool,
    /// Navigation-choice presentation is active.
    pub navigation_choice_active: bool,
    /// A save transaction is active.
    pub save_active: bool,
    /// A load transaction is active.
    pub load_active: bool,
    /// A navigation transition is active.
    pub navigation_transition_active: bool,
    /// A navigation actor is still transitioning.
    pub navigation_actor_transition_active: bool,
}

/// Presentation scheduling state owned by the main lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamePresentationScheduler {
    /// A script presentation currently owns the bridge.
    pub active: bool,
    /// Ship presentation currently owns the bridge.
    pub ship_active: bool,
    /// A menu presentation is deferred.
    pub menu_deferred: bool,
    /// A subtitle is currently visible or being revealed.
    pub subtitle_display_active: bool,
    /// New presentation startup is locked.
    pub start_locked: bool,
    /// The presentation has entered its completion hold.
    pub hold_ready: bool,
    /// The assembled word buffer contains at least one authored word.
    pub word_buffer_nonempty: bool,
    /// Progressive word selection is active.
    pub word_choice_active: bool,
    /// The final dialogue hold was armed.
    pub dialogue_hold_complete: bool,
    /// Remaining frames in the dialogue hold.
    pub dialogue_hold_countdown: u16,
    /// Text and sequence requests published by BloodScript.
    pub request_flags: PresentationRequestFlags,
    /// A scene resource is currently active.
    pub scene_gate_active: bool,
    /// A sequence resource is currently active.
    pub sequence_active: bool,
    /// A menu selection must replace the current presentation line.
    pub text_menu_pending: bool,
    /// Signed A6 presentation selector added to line-table base nine.
    pub text_selector: Option<i8>,
    /// C2 presentation work owns the current line.
    pub c2_presentation_gate: bool,
    /// Current authored presentation line, if any.
    pub active_line: Option<u16>,
    /// Producer metric used to detect an empty presentation list.
    pub list_entry_metric: u16,
    /// Consumer metric used to detect an empty presentation list.
    pub list_read_wrap_index: u16,
    /// Field activated when the current scene completes.
    pub owner: Option<GamePresentationOwner>,
    /// Word source used by the menu reveal pipeline.
    pub menu_word_source: GameMenuWordSource,
    /// Whether subtitle words rather than menu words are being rendered.
    pub subtitle_word_list_mode: bool,
    /// Whether the subtitle voice selector is armed.
    pub subtitle_voice_trigger: bool,
    /// One-shot completion audio request.
    pub completion_audio_pending: bool,
}

impl Default for GamePresentationScheduler {
    fn default() -> Self {
        Self {
            active: false,
            ship_active: false,
            menu_deferred: false,
            subtitle_display_active: false,
            start_locked: false,
            hold_ready: false,
            word_buffer_nonempty: false,
            word_choice_active: false,
            dialogue_hold_complete: false,
            dialogue_hold_countdown: u16::MIN,
            request_flags: PresentationRequestFlags::default(),
            scene_gate_active: false,
            sequence_active: false,
            text_menu_pending: false,
            text_selector: None,
            c2_presentation_gate: false,
            active_line: None,
            list_entry_metric: u16::MIN,
            list_read_wrap_index: u16::MIN,
            owner: None,
            menu_word_source: GameMenuWordSource::Current,
            subtitle_word_list_mode: false,
            subtitle_voice_trigger: false,
            completion_audio_pending: false,
        }
    }
}

/// Mutable state coordinated once per modern SDL frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameLifecycleState {
    /// Input requested game shutdown.
    pub exit_requested: bool,
    /// Pause-HUD refresh owns this frame.
    pub pause_hud_active: bool,
    /// Preserve the last logical pointer position without warping the host cursor.
    pub pointer_position_locked: bool,
    /// Native-compatible short press countdown retained by bridge input.
    pub pointer_press_pending: u8,
    /// Primary press latch consumed by bridge interactions.
    pub primary_pointer_pressed: bool,
    /// Secondary press latch consumed by bridge interactions.
    pub secondary_pointer_pressed: bool,
    /// A navigation target is currently selected.
    pub navigation_target_selected: bool,
    /// Presentation mode suppresses ordinary VM execution.
    pub presentation_mode: bool,
    /// Recovered low UI bits kept private behind semantic subsystem accessors.
    ui_state: GameUiState,
    /// Pending playable profile selected by BloodScript.
    pub pending_profile: Option<ScriptProfileId>,
    /// Other subsystem ownership that postpones profile replacement.
    pub profile_change_blockers: GameProfileChangeBlockers,
    /// Main presentation scheduler state.
    pub presentation: GamePresentationScheduler,
    /// VM execution is enabled after a profile replacement.
    pub vm_execution_enabled: bool,
    /// Navigation artwork must be rebuilt.
    pub navigation_rebuild_pending: bool,
    /// Pending navigation transition state.
    pub navigation_transition_pending: bool,
    /// The current scene produced a frame ready for final presentation.
    pub frame_presented: bool,
    /// Timer-owned radio clip playback countdown used by scripted C3 actions.
    ///
    /// This is the flat owner of `GS:0x0B39`. The separate one-byte
    /// `voc_tablo2_reset_gate` at `DS:0x0D30` belongs to the DOS audio driver
    /// and must never be folded into this script-visible countdown.
    pub clip_playback_state: u16,
    /// One-shot request to arm the no-VOC PC-speaker pulse in the canonical timer.
    pub speaker_pulse_requested: bool,
}

impl GameLifecycleState {
    /// Return bits zero through three of the recovered shared UI word.
    pub(crate) const fn low_ui_state_word(&self) -> u16 {
        self.ui_state.encode_low()
    }

    /// Replace the recovered low UI bits from one canonical native word write.
    ///
    /// The panorama-dependent mode in bits four through seven has a separate
    /// typed owner in the modern bridge services.
    pub(crate) fn set_low_ui_state_word(&mut self, word: u16) {
        self.ui_state = GameUiState::decode_low(word);
    }

    /// Return whether the presentation interface and DESCRIPT name path are active.
    pub const fn presentation_interface_active(&self) -> bool {
        self.ui_state.presentation_interface_active
    }

    /// Update the presentation interface bit without disturbing profile blockers.
    pub fn set_presentation_interface_active(&mut self, active: bool) {
        self.ui_state.presentation_interface_active = active;
    }

    /// Update the shared modal UI blocker without disturbing navigation state.
    pub fn set_modal_ui_busy(&mut self, busy: bool) {
        self.ui_state.modal_busy = busy;
    }

    /// Return whether a confirmation, save/load, or camera modal owns the UI.
    pub const fn modal_ui_busy(&self) -> bool {
        self.ui_state.modal_busy
    }

    /// Update the navigation-choice UI blocker without disturbing modal state.
    pub fn set_navigation_ui_busy(&mut self, busy: bool) {
        self.ui_state.navigation_busy = busy;
    }

    /// Return whether automatic bridge steering owns native UI bit three.
    pub const fn navigation_ui_busy(&self) -> bool {
        self.ui_state.navigation_busy
    }

    /// Return whether any recovered UI bit postpones a pending profile change.
    pub const fn profile_ui_blocked(&self) -> bool {
        self.ui_state.profile_change_blocked()
    }

    fn profile_change_blocked(&self) -> bool {
        let blockers = self.profile_change_blockers;
        self.presentation.active
            || self.presentation.ship_active
            || blockers.render_update_active
            || self.presentation.menu_deferred
            || self.presentation.subtitle_display_active
            || blockers.navigation_choice_active
            || blockers.save_active
            || blockers.load_active
            || blockers.navigation_transition_active
            || blockers.navigation_actor_transition_active
    }
}

/// Why the native game lifetime ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameLifecycleExit {
    /// The required bridge panorama could not be opened.
    BridgePanoramaUnavailable,
    /// SDL input requested shutdown.
    InputRequested,
    /// BloodScript requested shutdown.
    VmRequestedExit,
    /// A requested BloodScript profile could not be loaded.
    ProfileLoadFailed,
}

/// Completed game lifetime and final semantic scene context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameLifecycleOutcome {
    /// Terminal reason selected by the coordinator.
    pub exit: GameLifecycleExit,
    /// Number of complete gameplay frames submitted.
    pub rendered_frames: u64,
    /// Scene ownership forwarded through final presentation cleanup.
    pub final_scene_link: GameSceneLink,
}

/// Host failure while running or shutting down the game.
#[derive(Debug)]
pub enum GameLifecycleError<HostError> {
    /// Active initialization or frame processing failed.
    Runtime(HostError),
    /// Shutdown cleanup failed after an otherwise orderly exit.
    Shutdown(HostError),
    /// Runtime and subsequent best-effort shutdown both failed.
    RuntimeAndShutdown {
        /// Original runtime failure.
        runtime: HostError,
        /// Cleanup failure observed afterward.
        shutdown: HostError,
    },
}

impl<HostError: fmt::Display> fmt::Display for GameLifecycleError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(formatter, "game runtime failed: {error}"),
            Self::Shutdown(error) => write!(formatter, "game shutdown failed: {error}"),
            Self::RuntimeAndShutdown { runtime, shutdown } => write!(
                formatter,
                "game runtime failed ({runtime}) and shutdown also failed ({shutdown})"
            ),
        }
    }
}

impl<HostError: Error + 'static> Error for GameLifecycleError<HostError> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) | Self::Shutdown(error) => Some(error),
            Self::RuntimeAndShutdown { runtime, .. } => Some(runtime),
        }
    }
}

/// SDL, wgpu, resource, audio, and translated subsystem boundaries.
pub trait GameLifecycleHost {
    /// Host operation failure.
    type Error;

    /// Create typed framebuffers, models, resource stores, and audio buffers.
    fn initialize_runtime_storage(&mut self) -> Result<(), Self::Error>;
    /// Draw the loading frame and prepare the explicit writable resource root.
    fn prepare_startup_resources(&mut self) -> Result<(), Self::Error>;
    /// Initialize archive indexing below the explicit source root.
    fn initialize_archive_index(&mut self) -> Result<(), Self::Error>;
    /// Discover and configure optional CD audio.
    fn prepare_cd_audio(&mut self) -> Result<(), Self::Error>;
    /// Load and decode the MANU3 overlay.
    fn load_manu3_overlay(&mut self) -> Result<(), Self::Error>;
    /// Configure the original 320 by 200 logical render surface.
    fn initialize_logical_viewport(&mut self) -> Result<(), Self::Error>;
    /// Open and decode the bridge panorama archive.
    fn open_bridge_panorama(&mut self) -> Result<bool, Self::Error>;
    /// Load the original save-slot directory.
    fn load_save_slots(&mut self) -> Result<(), Self::Error>;
    /// Load the selected startup audio driver resource.
    fn load_startup_audio(&mut self) -> Result<(), Self::Error>;
    /// Resolve startup audio storage and configure the modern audio host.
    fn configure_startup_audio(&mut self) -> Result<(), Self::Error>;
    /// Load the initial resource expected by the audio pipeline.
    fn load_initial_audio_resource(&mut self) -> Result<(), Self::Error>;
    /// Seed and randomize the bridge point cloud.
    fn randomize_ship_point_cloud(&mut self) -> Result<(), Self::Error>;
    /// Run the opening presentation.
    fn run_initial_presentation(&mut self, link: GameSceneLink) -> Result<(), Self::Error>;
    /// Load the default bridge sound bank.
    fn load_default_sound_bank(&mut self) -> Result<(), Self::Error>;
    /// Decode and initialize the bridge back buffer.
    fn initialize_back_buffer(&mut self) -> Result<(), Self::Error>;

    /// Pump SDL events and update lifecycle input state.
    fn dispatch_input(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Sample the logical SDL pointer without changing the host cursor.
    fn poll_pointer(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Refresh the pause HUD.
    fn refresh_pause_hud(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Update typed pointer edge latches.
    fn update_pointer_buttons(&mut self, state: &mut GameLifecycleState)
    -> Result<(), Self::Error>;
    /// Execute one ordinary BloodScript pass.
    fn run_vm(&mut self, state: &mut GameLifecycleState) -> Result<GameVmRunStatus, Self::Error>;
    /// Replace the active typed BloodScript profile.
    fn load_profile(
        &mut self,
        profile: ScriptProfileId,
        state: &mut GameLifecycleState,
    ) -> Result<GameProfileLoadStatus, Self::Error>;
    /// Rebuild record-derived state after a profile replacement.
    fn rebuild_record_state(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Refresh active-object access counters after a profile replacement.
    fn refresh_object_access(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Reset ship HUD palette and camera state after a profile replacement.
    fn reset_ship_hud(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Stop any prior completion sound before loading the authored clip.
    fn stop_completion_audio(&mut self) -> Result<(), Self::Error>;
    /// Load the authored completion voice clip.
    fn load_completion_audio(&mut self) -> Result<(), Self::Error>;
    /// Start completion voice playback.
    fn start_completion_audio(&mut self) -> Result<(), Self::Error>;

    /// Render bridge artwork and actors for the current scene link.
    fn render_bridge_frame(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Advance the navigation confirmation dialog.
    fn update_confirm_dialog(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Refill streamed voice audio.
    fn refill_audio_stream(&mut self) -> Result<(), Self::Error>;
    /// Process queued sound and mixer work against current presentation state.
    fn process_audio(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Advance ship presentation state.
    fn update_ship_presentation(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Advance scene transition state.
    fn update_scene_transition(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Advance save/load UI and transactions.
    fn update_save_load(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Advance presentation-choice transitions.
    fn update_presentation_choice(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Release a resource frame to the presentation scheduler.
    fn mark_presentation_ready(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Submit the complete indexed framebuffer to wgpu.
    fn submit_indexed_frame(&mut self) -> Result<(), Self::Error>;
    /// Reveal progressive inline menu words.
    fn reveal_inline_menu(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Advance subtitle reveal and hold timing.
    fn update_subtitles(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Advance the MANU3 hand overlay.
    fn update_manu3(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error>;
    /// Advance palette interpolation.
    fn update_palette_transition(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error>;
    /// Pace one modern host frame without busy-waiting or masking interrupts.
    fn pace_frame(&mut self) -> Result<(), Self::Error>;
    /// Publish the finished wgpu frame and any dirty palette upload.
    fn present_frame(&mut self) -> Result<(), Self::Error>;

    /// Finish active presentation resources before shutdown.
    fn finish_presentations(&mut self) -> Result<(), Self::Error>;
    /// Stop the modern audio driver.
    fn stop_audio(&mut self) -> Result<(), Self::Error>;
    /// Run the final streamed presentation.
    fn run_final_presentation(&mut self, link: GameSceneLink) -> Result<(), Self::Error>;
    /// Remove a transient voice resource if one exists.
    fn remove_transient_voice(&mut self) -> Result<(), Self::Error>;
    /// Remove a transient music resource if one exists.
    fn remove_transient_music(&mut self) -> Result<(), Self::Error>;
    /// Remove a transient archive-index resource if one exists.
    fn remove_transient_archive_index(&mut self) -> Result<(), Self::Error>;
    /// Delete the authored startup transient paths below the writable root.
    fn delete_startup_transients(&mut self) -> Result<(), Self::Error>;
    /// Close the owned bridge panorama archive.
    fn close_bridge_panorama(&mut self) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug)]
struct GameSession {
    scene_link: GameSceneLink,
    panorama_opened: bool,
    rendered_frames: u64,
}

impl Default for GameSession {
    fn default() -> Self {
        Self {
            scene_link: GameSceneLink::Initial,
            panorama_opened: false,
            rendered_frames: u64::MIN,
        }
    }
}

/// Run the complete translated BLOODPRG game lifetime.
///
/// This is the flat translation of `bloodprg_main` at file offset `0x000EB0`.
/// Typed owned storage replaces six 64 KiB DOS arenas. SDL and wgpu replace
/// mouse interrupts, pointer warping, VGA submission, interrupt masking, and
/// the eight-tick spin wait. Explicit resource roots replace DOS handles and
/// current-directory mutation while preserving initialization, frame, branch,
/// presentation-owner, audio, and shutdown ordering.
pub fn run_game_lifecycle<Host: GameLifecycleHost>(
    state: &mut GameLifecycleState,
    host: &mut Host,
) -> Result<GameLifecycleOutcome, GameLifecycleError<Host::Error>> {
    let mut session = GameSession::default();
    let runtime = run_game_runtime(state, host, &mut session);
    let shutdown = shutdown_game(host, session, runtime.is_ok());

    match (runtime, shutdown) {
        (Ok(exit), Ok(())) => Ok(GameLifecycleOutcome {
            exit,
            rendered_frames: session.rendered_frames,
            final_scene_link: session.scene_link,
        }),
        (Err(runtime), Ok(())) => Err(GameLifecycleError::Runtime(runtime)),
        (Ok(_exit), Err(shutdown)) => Err(GameLifecycleError::Shutdown(shutdown)),
        (Err(runtime), Err(shutdown)) => {
            Err(GameLifecycleError::RuntimeAndShutdown { runtime, shutdown })
        }
    }
}

fn run_game_runtime<Host: GameLifecycleHost>(
    state: &mut GameLifecycleState,
    host: &mut Host,
    session: &mut GameSession,
) -> Result<GameLifecycleExit, Host::Error> {
    host.initialize_runtime_storage()?;
    host.prepare_startup_resources()?;
    host.initialize_archive_index()?;
    host.prepare_cd_audio()?;
    host.load_manu3_overlay()?;
    host.initialize_logical_viewport()?;
    if !host.open_bridge_panorama()? {
        return Ok(GameLifecycleExit::BridgePanoramaUnavailable);
    }
    session.panorama_opened = true;

    host.load_save_slots()?;
    host.load_startup_audio()?;
    host.configure_startup_audio()?;
    host.load_initial_audio_resource()?;

    state.presentation_mode = true;
    state.set_presentation_interface_active(true);
    state.navigation_rebuild_pending = true;
    host.randomize_ship_point_cloud()?;
    host.run_initial_presentation(session.scene_link)?;
    host.load_default_sound_bank()?;
    host.initialize_back_buffer()?;
    state.pending_profile = Some(ScriptProfileId::INITIAL);

    loop {
        host.dispatch_input(state)?;
        if !state.pause_hud_active && !state.pointer_position_locked && !state.navigation_ui_busy()
        {
            host.poll_pointer(state)?;
            consume_pointer_press_state(state);
        }

        if state.exit_requested {
            return Ok(GameLifecycleExit::InputRequested);
        }
        host.refresh_pause_hud(state)?;
        if state.pause_hud_active {
            continue;
        }

        host.update_pointer_buttons(state)?;
        if !state.presentation_mode && host.run_vm(state)? == GameVmRunStatus::ExitRequested {
            return Ok(GameLifecycleExit::VmRequestedExit);
        }

        if let Some(profile) = state.pending_profile
            && !state.profile_ui_blocked()
            && !state.profile_change_blocked()
        {
            if host.load_profile(profile, state)? == GameProfileLoadStatus::Failed {
                return Ok(GameLifecycleExit::ProfileLoadFailed);
            }
            state.pending_profile = None;
            state.vm_execution_enabled = true;
            let _ = host.run_vm(state)?;
            host.rebuild_record_state(state)?;
            host.refresh_object_access(state)?;
            host.reset_ship_hud(state)?;
            state.navigation_rebuild_pending = true;
            state.navigation_transition_pending = false;
        }

        if !state.presentation.c2_presentation_gate {
            state.frame_presented = true;
        }
        update_game_presentation_ownership(state, &mut session.scene_link);
        play_completion_audio_if_pending(state, host)?;
        run_frame_tail(state, session.scene_link, host)?;
        session.rendered_frames = session.rendered_frames.wrapping_add(1);
    }
}

fn consume_pointer_press_state(state: &mut GameLifecycleState) {
    if state.pointer_press_pending & POINTER_PRESS_STATE_MASK == u8::MIN {
        state.primary_pointer_pressed = false;
        state.secondary_pointer_pressed = false;
        state.navigation_target_selected = false;
    } else if state.pointer_press_pending & SECONDARY_POINTER_PRESS_STATE != u8::MIN {
        state.pointer_press_pending = u8::MIN;
    } else {
        state.pointer_press_pending = state.pointer_press_pending.wrapping_sub(1);
    }
}

/// Apply the presentation-ownership section of the recovered main loop.
///
/// This is public so concrete runtime tests can exercise the same ownership
/// transfer between BloodScript and scene dispatch without duplicating it.
pub fn update_game_presentation_ownership(
    state: &mut GameLifecycleState,
    scene_link: &mut GameSceneLink,
) {
    let secondary_pointer_pressed = state.secondary_pointer_pressed;
    let presentation = &mut state.presentation;
    let mut active_hold_path = false;

    if presentation.active {
        if !presentation.menu_deferred && !presentation.subtitle_display_active {
            presentation.start_locked = false;
            presentation.hold_ready = true;
        }
        if presentation.hold_ready {
            if presentation.word_buffer_nonempty {
                presentation.word_choice_active = true;
            } else {
                presentation.menu_deferred = false;
                presentation.subtitle_display_active = false;
            }
            active_hold_path = true;
        } else if presentation.menu_deferred || presentation.subtitle_display_active {
            presentation.owner = Some(if presentation.subtitle_display_active {
                GamePresentationOwner::Subtitle
            } else {
                GamePresentationOwner::DeferredMenu
            });
        }
    }

    if !active_hold_path && presentation.dialogue_hold_complete {
        if presentation.word_buffer_nonempty {
            presentation.word_choice_active = presentation.active;
        }
        if presentation.dialogue_hold_countdown == u16::MIN || secondary_pointer_pressed {
            presentation.dialogue_hold_complete = false;
            presentation.hold_ready = presentation.active;
            if !presentation.hold_ready {
                presentation.subtitle_display_active = false;
                presentation.menu_deferred = false;
            } else {
                presentation.request_flags = PresentationRequestFlags::decode(
                    presentation.request_flags.bits() & !PRIMARY_TEXT_REQUEST_PENDING,
                );
            }
        }
    }

    if !presentation.active && !presentation.hold_ready && !presentation.ship_active {
        presentation.subtitle_display_active = false;
        presentation.menu_deferred = false;
    }

    if presentation.active
        && (presentation.scene_gate_active || presentation.sequence_active)
        && !presentation.request_flags.secondary_request_pending()
    {
        let request_or_countdown = presentation.request_flags.any_request_pending()
            || presentation.dialogue_hold_countdown as u8 != u8::MIN;
        if request_or_countdown {
            if presentation.text_menu_pending {
                presentation.text_menu_pending = false;
                presentation.c2_presentation_gate = false;
                presentation.active_line = Some(presentation.text_selector.map_or(
                    DEFAULT_PRESENTATION_LINE,
                    presentation_line_for_text_selector,
                ));
            } else if !presentation.c2_presentation_gate {
                presentation.subtitle_word_list_mode = false;
                presentation.active_line = Some(DEFAULT_PRESENTATION_LINE);
            }
        } else if presentation.active_line != Some(DEFAULT_PRESENTATION_LINE)
            && presentation
                .list_entry_metric
                .wrapping_sub(presentation.list_read_wrap_index)
                != u16::MIN
        {
            if let Some(owner) = presentation.owner {
                *scene_link = owner.scene_link();
                match owner {
                    GamePresentationOwner::Subtitle => {
                        presentation.subtitle_display_active = true;
                    }
                    GamePresentationOwner::DeferredMenu => {
                        presentation.menu_deferred = true;
                    }
                }
            } else {
                *scene_link = GameSceneLink::MenuWords;
                presentation.menu_deferred = true;
                presentation.menu_word_source = GameMenuWordSource::PresentationBuffer;
            }
            if !presentation.c2_presentation_gate {
                presentation.active_line = Some(DEFAULT_PRESENTATION_LINE);
            }
        }
    }

    if presentation.request_flags.secondary_request_pending()
        || presentation.request_flags.bits() & PRIMARY_TEXT_REQUEST_PENDING == u8::MIN
    {
        presentation.subtitle_word_list_mode = false;
        presentation.subtitle_voice_trigger = false;
    }
}

/// Convert BloodScript's signed voice selector to the shared native presentation line.
///
/// Selector `-1` intentionally wraps back to the default line eight. The recovered
/// main loop performs this conversion before every scene coordinator consumes the
/// single shared `vm_active_line` global.
pub const fn presentation_line_for_text_selector(selector: i8) -> u16 {
    TEXT_PRESENTATION_LINE_BASE.wrapping_add_signed(selector as i16)
}

fn play_completion_audio_if_pending<Host: GameLifecycleHost>(
    state: &mut GameLifecycleState,
    host: &mut Host,
) -> Result<(), Host::Error> {
    if state.presentation.completion_audio_pending {
        state.presentation.completion_audio_pending = false;
        host.stop_completion_audio()?;
        host.load_completion_audio()?;
        host.start_completion_audio()?;
    }
    Ok(())
}

fn run_frame_tail<Host: GameLifecycleHost>(
    state: &mut GameLifecycleState,
    scene_link: GameSceneLink,
    host: &mut Host,
) -> Result<(), Host::Error> {
    host.render_bridge_frame(scene_link, state)?;
    host.update_confirm_dialog(state)?;
    host.refill_audio_stream()?;
    host.process_audio(state)?;
    host.update_ship_presentation(scene_link, state)?;
    host.update_scene_transition(scene_link, state)?;
    host.update_save_load(state)?;
    host.update_presentation_choice(state)?;
    if state.frame_presented {
        host.mark_presentation_ready(state)?;
    }
    host.submit_indexed_frame()?;
    host.reveal_inline_menu(state)?;
    host.update_subtitles(state)?;
    host.update_manu3(state)?;
    host.update_palette_transition(state)?;
    host.pace_frame()?;
    host.present_frame()?;
    Ok(())
}

fn shutdown_game<Host: GameLifecycleHost>(
    host: &mut Host,
    session: GameSession,
    run_final_presentation: bool,
) -> Result<(), Host::Error> {
    host.finish_presentations()?;
    host.stop_audio()?;
    if run_final_presentation {
        host.run_final_presentation(session.scene_link)?;
    }
    host.stop_audio()?;
    if session.panorama_opened {
        host.remove_transient_voice()?;
        host.remove_transient_music()?;
        host.remove_transient_archive_index()?;
    }
    host.delete_startup_transients()?;
    if session.panorama_opened {
        host.close_bridge_panorama()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "game_lifecycle_tests.rs"]
mod tests;
