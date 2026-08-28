//! Production SDL3 and wgpu host for the recovered top-level game lifecycle.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use sdl3::AudioSubsystem;

use crate::native::bloodprg::{
    BridgeSceneInput, BridgeSteeringInteraction, CdAudioPreparationOutcome, ConfirmDialogOutcome,
    GameLifecycleHost, GameLifecycleState, GameProfileLoadStatus, GameSceneLink, GameTimerContext,
    GameTimerState, GameVmRunStatus, InputAction, InputCancellationOutcome, PresentationResourceId,
    ScriptClock, ScriptProfileId, ScriptRuntime, advance_game_timer_tick,
};

use super::bridge_frame::run_runtime_bridge_frame;
use super::{
    ModernGameServices, RuntimeAssetLoadStatus, RuntimePlatformHost, run_runtime_presentation,
};

const OPENING_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(0);
const CREDITS_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(1);
const INITIAL_SCENE_LINK_TARGET: u16 = 16;
const SUBTITLE_SCENE_LINK_TARGET: u16 = 24_164;
const DEFERRED_MENU_SCENE_LINK_TARGET: u16 = 26_544;
const PRESENTATION_MENU_BUFFER_LINK_TARGET: u16 = u16::MIN;
const GAME_TIMER_TICKS_PER_FRAME: usize = 8;
const COMPLETION_VOICE_RESOURCE: &[u8] = b"mu\\tablo2.voc";

pub(super) fn arm_requested_speaker_pulse(
    state: &mut GameLifecycleState,
    timer: &mut GameTimerState,
) {
    if std::mem::take(&mut state.speaker_pulse_requested) {
        timer.speaker_pulse.request();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RuntimeAudioStartupStage {
    #[default]
    Pending,
    DriverSelected,
    Configured,
}

/// Complete modern host consumed by [`crate::native::bloodprg::run_game_lifecycle`].
pub struct RuntimeGameLifecycleHost<'window, 'audio> {
    services: ModernGameServices<'window>,
    platform: RuntimePlatformHost<'window>,
    audio: &'audio AudioSubsystem,
    packed_clock_seed: u8,
    script_clock_source: fn() -> Result<ScriptClock>,
    frame_limit: Option<u64>,
    main_frames_presented: u64,
    manu3_visible: bool,
    indexed_bridge_ui_active: bool,
    current_scene_link: GameSceneLink,
    timer: GameTimerState,
    startup_timer_runtime: ScriptRuntime,
    runtime_storage_initialized: bool,
    archive_index_initialized: bool,
    audio_startup_stage: RuntimeAudioStartupStage,
    startup_transient_paths: Vec<PathBuf>,
}

impl<'window, 'audio> RuntimeGameLifecycleHost<'window, 'audio> {
    /// Bind initialized SDL services to the complete flat game runtime.
    pub fn new(
        services: ModernGameServices<'window>,
        platform: RuntimePlatformHost<'window>,
        audio: &'audio AudioSubsystem,
        packed_clock_seed: u8,
        script_clock_source: fn() -> Result<ScriptClock>,
        frame_limit: Option<u64>,
    ) -> Self {
        Self {
            services,
            platform,
            audio,
            packed_clock_seed,
            script_clock_source,
            frame_limit,
            main_frames_presented: u64::MIN,
            manu3_visible: false,
            indexed_bridge_ui_active: false,
            current_scene_link: GameSceneLink::Initial,
            timer: GameTimerState::default(),
            startup_timer_runtime: ScriptRuntime::default(),
            runtime_storage_initialized: false,
            archive_index_initialized: false,
            audio_startup_stage: RuntimeAudioStartupStage::Pending,
            startup_transient_paths: Vec::new(),
        }
    }

    /// Borrow the runtime after lifecycle completion for diagnostics.
    pub const fn services(&self) -> &ModernGameServices<'window> {
        &self.services
    }

    fn advance_frame_timers(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        arm_requested_speaker_pulse(state, &mut self.timer);
        let (chatter_cooldown, dialogue_delay) = self.services.audio_event_timer_counters();
        self.timer.chatter_cooldown = chatter_cooldown;
        self.timer.dialogue_delay = dialogue_delay;
        self.timer.dialogue_hold_countdown = state.presentation.dialogue_hold_countdown;
        self.timer.clip_playback_state = state.clip_playback_state;
        self.services.export_game_timer_state(&mut self.timer)?;
        let context = GameTimerContext {
            paused: state.pause_hud_active,
            navigation_link_pending: state.navigation_transition_pending,
        };
        let mut speaker_gate = None;
        if let Some(profile) = self.services.runtime_mut().current_profile_mut() {
            for _ in usize::MIN..GAME_TIMER_TICKS_PER_FRAME {
                speaker_gate =
                    advance_game_timer_tick(&mut self.timer, profile.runtime_mut(), context)
                        .speaker_gate
                        .or(speaker_gate);
            }
        } else {
            for _ in usize::MIN..GAME_TIMER_TICKS_PER_FRAME {
                speaker_gate = advance_game_timer_tick(
                    &mut self.timer,
                    &mut self.startup_timer_runtime,
                    context,
                )
                .speaker_gate
                .or(speaker_gate);
            }
        }
        if let Some(action) = speaker_gate {
            self.services.apply_speaker_gate(action)?;
        }
        state.presentation.dialogue_hold_countdown = self.timer.dialogue_hold_countdown;
        state.clip_playback_state = self.timer.clip_playback_state;
        self.services.import_game_timer_state(&self.timer)?;
        self.services
            .synchronize_audio_event_timers(self.timer.chatter_cooldown, self.timer.dialogue_delay)
    }

    fn frame_limit_reached(&self) -> bool {
        self.frame_limit
            .is_some_and(|limit| self.main_frames_presented >= limit)
    }

    fn scene_link_target(link: GameSceneLink) -> u16 {
        match link {
            GameSceneLink::Initial => INITIAL_SCENE_LINK_TARGET,
            GameSceneLink::SubtitlePresentation => SUBTITLE_SCENE_LINK_TARGET,
            GameSceneLink::DeferredPresentation => DEFERRED_MENU_SCENE_LINK_TARGET,
            GameSceneLink::MenuWords => PRESENTATION_MENU_BUFFER_LINK_TARGET,
        }
    }

    fn bridge_interaction(state: &GameLifecycleState) -> BridgeSteeringInteraction {
        let menu_active = state.modal_ui_busy()
            || state.profile_change_blockers.navigation_choice_active
            || state.presentation.word_choice_active
            || state.presentation.menu_deferred
            || state.presentation.subtitle_display_active;
        if menu_active {
            BridgeSteeringInteraction::MenuEngaged
        } else {
            BridgeSteeringInteraction::Free
        }
    }

    fn indexed_bridge_ui_active(state: &GameLifecycleState) -> bool {
        let blockers = state.profile_change_blockers;
        state.pause_hud_active
            || state.modal_ui_busy()
            || state.navigation_transition_pending
            || state.presentation.active
            || state.presentation.ship_active
            || state.presentation.word_choice_active
            || state.presentation.menu_deferred
            || state.presentation.subtitle_display_active
            || blockers.render_update_active
            || blockers.navigation_choice_active
            || blockers.save_active
            || blockers.load_active
            || blockers.navigation_transition_active
            || blockers.navigation_actor_transition_active
    }
}

impl GameLifecycleHost for RuntimeGameLifecycleHost<'_, '_> {
    type Error = anyhow::Error;

    fn initialize_runtime_storage(&mut self) -> Result<()> {
        if self.runtime_storage_initialized {
            bail!("runtime storage was initialized more than once");
        }
        if self.services.runtime().front_buffer().pixels().is_empty()
            || self.services.runtime().back_buffer().pixels().is_empty()
        {
            bail!("flat runtime framebuffers were not allocated");
        }
        self.timer.start();
        self.runtime_storage_initialized = true;
        Ok(())
    }

    fn prepare_startup_resources(&mut self) -> Result<()> {
        self.services.prepare_startup_resources().map(|_| ())
    }

    fn initialize_archive_index(&mut self) -> Result<()> {
        let data = self.services.runtime().data();
        let indexed_resource_count = data.resource_store().resource_names().len();
        if indexed_resource_count != data.imported_resource_count() {
            bail!(
                "imported loose-resource index has {} entries; expected {}",
                indexed_resource_count,
                data.imported_resource_count()
            );
        }
        self.archive_index_initialized = true;
        Ok(())
    }

    fn prepare_cd_audio(&mut self) -> Result<()> {
        match self.services.prepare_optional_cd_audio() {
            CdAudioPreparationOutcome::Unavailable => Ok(()),
            CdAudioPreparationOutcome::Prepared { .. } => {
                bail!("CD track metadata was prepared without a bound modern track source")
            }
        }
    }

    fn load_manu3_overlay(&mut self) -> Result<()> {
        self.services.load_manu3_overlay().map(|_| ())
    }

    fn initialize_logical_viewport(&mut self) -> Result<()> {
        self.services.initialize_logical_viewport()
    }

    fn open_bridge_panorama(&mut self) -> Result<bool> {
        Ok(matches!(
            self.services.open_bridge_panorama()?,
            RuntimeAssetLoadStatus::LoadedNow | RuntimeAssetLoadStatus::AlreadyLoaded
        ))
    }

    fn load_save_slots(&mut self) -> Result<()> {
        self.services.load_save_slots().map(|_| ())
    }

    fn load_startup_audio(&mut self) -> Result<()> {
        if self.audio_startup_stage != RuntimeAudioStartupStage::Pending {
            bail!("startup audio driver selection is out of order");
        }
        let driver = self.audio.current_audio_driver();
        if driver.is_empty() {
            bail!("SDL selected an empty audio driver name");
        }
        self.audio_startup_stage = RuntimeAudioStartupStage::DriverSelected;
        Ok(())
    }

    fn configure_startup_audio(&mut self) -> Result<()> {
        if self.audio_startup_stage != RuntimeAudioStartupStage::DriverSelected {
            bail!("SDL audio configuration preceded driver selection");
        }
        self.services.initialize_audio(self.audio)?;
        self.audio_startup_stage = RuntimeAudioStartupStage::Configured;
        Ok(())
    }

    fn load_initial_audio_resource(&mut self) -> Result<()> {
        if self.audio_startup_stage != RuntimeAudioStartupStage::Configured {
            bail!("startup CARTE.SPR load preceded SDL audio configuration");
        }
        self.services.load_initial_cartography_resource()
    }

    fn randomize_ship_point_cloud(&mut self) -> Result<()> {
        self.services
            .initialize_bridge_scene(self.packed_clock_seed)
    }

    fn run_initial_presentation(&mut self, link: GameSceneLink) -> Result<()> {
        run_runtime_presentation(
            OPENING_PRESENTATION_LINE,
            Self::scene_link_target(link),
            &mut self.services,
            &mut self.platform,
            &mut self.timer,
            &mut self.startup_timer_runtime,
        )
        .map(|_| ())
    }

    fn load_default_sound_bank(&mut self) -> Result<()> {
        self.services.load_default_sound_bank()
    }

    fn initialize_back_buffer(&mut self) -> Result<()> {
        self.services.initialize_back_buffer().map(|_| ())
    }

    fn dispatch_input(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        if self.frame_limit_reached() {
            state.exit_requested = true;
            return Ok(());
        }
        self.advance_frame_timers(state)?;
        if let Some(action) = self.platform.dispatch_events(&mut self.services, state) {
            let presentation_cancelled = if matches!(action, InputAction::Cancel) {
                self.services.cancel_lifecycle_presentation(state)?
                    == InputCancellationOutcome::CancelledPresentation
            } else {
                false
            };
            if !presentation_cancelled {
                self.services.queue_save_load_input(action)?;
            }
        }
        Ok(())
    }

    fn poll_pointer(&mut self, _state: &mut GameLifecycleState) -> Result<()> {
        let previous = self.services.input().pointer_sample().position;
        self.platform.poll_pointer(&mut self.services);
        if self.services.input().pointer_sample().position != previous {
            self.timer.reset_mouse_idle_counter();
        }
        Ok(())
    }

    fn refresh_pause_hud(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services.refresh_pause_hud(state.pause_hud_active)?;
        Ok(())
    }

    fn update_pointer_buttons(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services.update_lifecycle_pointer_buttons(state);
        Ok(())
    }

    fn run_vm(&mut self, state: &mut GameLifecycleState) -> Result<GameVmRunStatus> {
        let clock =
            (self.script_clock_source)().context("sampling the host clock for BloodScript")?;
        self.services.set_script_clock(clock);
        self.services
            .execute_and_apply_lifecycle_script_frame(state)?;
        if self.services.take_script_mouse_idle_reset_request() {
            self.timer.reset_mouse_idle_counter();
        }
        Ok(GameVmRunStatus::Continue)
    }

    fn load_profile(
        &mut self,
        profile: ScriptProfileId,
        _state: &mut GameLifecycleState,
    ) -> Result<GameProfileLoadStatus> {
        self.services.load_script_profile(profile)?;
        Ok(GameProfileLoadStatus::Loaded)
    }

    fn rebuild_record_state(&mut self, _state: &mut GameLifecycleState) -> Result<()> {
        self.services.rebuild_script_record_state()
    }

    fn refresh_object_access(&mut self, _state: &mut GameLifecycleState) -> Result<()> {
        self.services.refresh_object_access_counters().map(|_| ())
    }

    fn reset_ship_hud(&mut self, _state: &mut GameLifecycleState) -> Result<()> {
        self.services.reset_ship_hud()
    }

    fn stop_completion_audio(&mut self) -> Result<()> {
        self.services.stop_audio()
    }

    fn load_completion_audio(&mut self) -> Result<()> {
        self.services.load_voice_resource(COMPLETION_VOICE_RESOURCE)
    }

    fn start_completion_audio(&mut self) -> Result<()> {
        self.services.start_loaded_voice()
    }

    fn render_bridge_frame(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        self.current_scene_link = link;
        let pointer = self.services.input().pointer_sample();
        run_runtime_bridge_frame(
            &mut self.services,
            state,
            link,
            BridgeSceneInput {
                horizontal_delta: self.platform.take_bridge_horizontal_delta(),
                pointer_buttons: pointer.buttons.bits(),
                interaction: Self::bridge_interaction(state),
            },
            self.timer.navigation_animation_phase,
        )?;
        Ok(())
    }

    fn update_confirm_dialog(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        let outcome = self.services.update_confirm_dialog(state)?;
        if matches!(outcome, ConfirmDialogOutcome::Confirmed(_))
            && self.services.confirm_dialog_state().navigation_choice_gate & 1 != 0
        {
            state.exit_requested = true;
        }
        Ok(())
    }

    fn refill_audio_stream(&mut self) -> Result<()> {
        self.services.refill_navigation_music()
    }

    fn process_audio(&mut self, _state: &mut GameLifecycleState) -> Result<()> {
        self.services.process_runtime_audio_events().map(|_| ())
    }

    fn update_ship_presentation(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        self.services
            .update_runtime_ship_presentation(link, state, &mut self.platform)
            .map(|_| ())
    }

    fn update_scene_transition(
        &mut self,
        link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        self.services
            .update_runtime_scene_transition(link, state, &mut self.platform)
            .map(|_| ())
    }

    fn update_save_load(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services.update_runtime_save_load(state).map(|_| ())
    }

    fn update_presentation_choice(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services.update_runtime_presentation_choice(state)
    }

    fn mark_presentation_ready(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services
            .update_lifecycle_word_choice(state)
            .map(|_| ())
    }

    fn submit_indexed_frame(&mut self) -> Result<()> {
        self.services.submit_indexed_frame()
    }

    fn reveal_inline_menu(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        let word_delay = self.services.dialogue_word_delay()?;
        self.services
            .reveal_lifecycle_inline_menu(state, word_delay)
            .map(|_| ())
    }

    fn update_subtitles(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.services.update_lifecycle_subtitles(state).map(|_| ())
    }

    fn update_manu3(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.manu3_visible = self.services.update_lifecycle_manu3(state)?;
        Ok(())
    }

    fn update_palette_transition(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        self.indexed_bridge_ui_active = Self::indexed_bridge_ui_active(state);
        self.services
            .update_lifecycle_palette_transition(state)
            .map(|_| ())
    }

    fn pace_frame(&mut self) -> Result<()> {
        if self.services.presentation_stream_active() {
            self.platform.pace_presentation_frame()
        } else {
            while self.platform.wait_for_visual_refresh(&mut self.services)? {
                if self.manu3_visible {
                    self.services
                        .reproject_manu3_for_pointer(self.platform.logical_pointer())?;
                }
                self.services
                    .present_current_bridge_frame(self.indexed_bridge_ui_active)?;
            }
            Ok(())
        }
    }

    fn present_frame(&mut self) -> Result<()> {
        self.services
            .present_current_bridge_frame(self.indexed_bridge_ui_active)?;
        self.main_frames_presented = self.main_frames_presented.wrapping_add(1);
        Ok(())
    }

    fn finish_presentations(&mut self) -> Result<()> {
        self.timer.stop();
        self.services.finish_runtime_presentations()
    }

    fn stop_audio(&mut self) -> Result<()> {
        self.services.stop_audio()
    }

    fn run_final_presentation(&mut self, link: GameSceneLink) -> Result<()> {
        run_runtime_presentation(
            CREDITS_PRESENTATION_LINE,
            Self::scene_link_target(link),
            &mut self.services,
            &mut self.platform,
            &mut self.timer,
            &mut self.startup_timer_runtime,
        )
        .map(|_| ())
    }

    fn remove_transient_voice(&mut self) -> Result<()> {
        self.services.discard_loaded_voice();
        Ok(())
    }

    fn remove_transient_music(&mut self) -> Result<()> {
        self.services.discard_loaded_music();
        Ok(())
    }

    fn remove_transient_archive_index(&mut self) -> Result<()> {
        if !self.archive_index_initialized {
            bail!("archive index cleanup ran before initialization");
        }
        self.archive_index_initialized = false;
        Ok(())
    }

    fn delete_startup_transients(&mut self) -> Result<()> {
        for path in self.startup_transient_paths.drain(..) {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("removing runtime transient {}", path.display()));
                }
            }
        }
        Ok(())
    }

    fn close_bridge_panorama(&mut self) -> Result<()> {
        if !self.services.close_bridge_scene() {
            bail!("bridge scene was already closed");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{GameMenuWordSource, SpeakerGateAction};

    #[test]
    fn scene_links_preserve_the_recovered_owner_offsets() {
        assert_eq!(
            RuntimeGameLifecycleHost::scene_link_target(GameSceneLink::Initial),
            INITIAL_SCENE_LINK_TARGET
        );
        assert_eq!(
            RuntimeGameLifecycleHost::scene_link_target(GameSceneLink::SubtitlePresentation),
            SUBTITLE_SCENE_LINK_TARGET
        );
        assert_eq!(
            RuntimeGameLifecycleHost::scene_link_target(GameSceneLink::DeferredPresentation),
            DEFERRED_MENU_SCENE_LINK_TARGET
        );
        assert_eq!(
            RuntimeGameLifecycleHost::scene_link_target(GameSceneLink::MenuWords),
            PRESENTATION_MENU_BUFFER_LINK_TARGET
        );
    }

    #[test]
    fn modal_presentation_owners_constrain_bridge_steering() {
        let mut state = GameLifecycleState::default();
        assert_eq!(
            RuntimeGameLifecycleHost::bridge_interaction(&state),
            BridgeSteeringInteraction::Free
        );
        state.presentation.menu_word_source = GameMenuWordSource::PresentationBuffer;
        state.presentation.menu_deferred = true;
        assert_eq!(
            RuntimeGameLifecycleHost::bridge_interaction(&state),
            BridgeSteeringInteraction::MenuEngaged
        );
        assert!(RuntimeGameLifecycleHost::indexed_bridge_ui_active(&state));
    }

    #[test]
    fn idle_bridge_does_not_claim_an_indexed_overlay() {
        let state = GameLifecycleState::default();
        assert!(!RuntimeGameLifecycleHost::indexed_bridge_ui_active(&state));
    }

    #[test]
    fn lifecycle_speaker_request_reaches_the_native_enable_disable_cadence() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.speaker_pulse_requested = true;
        let mut timer = GameTimerState::default();
        timer.start();
        timer.tick = 31;
        let mut script = ScriptRuntime::default();

        arm_requested_speaker_pulse(&mut lifecycle, &mut timer);
        assert!(!lifecycle.speaker_pulse_requested);
        assert_eq!(
            advance_game_timer_tick(&mut timer, &mut script, GameTimerContext::default())
                .speaker_gate,
            Some(SpeakerGateAction::Enable)
        );
        for _ in 0..31 {
            assert_eq!(
                advance_game_timer_tick(&mut timer, &mut script, GameTimerContext::default())
                    .speaker_gate,
                None
            );
        }
        assert_eq!(
            advance_game_timer_tick(&mut timer, &mut script, GameTimerContext::default())
                .speaker_gate,
            Some(SpeakerGateAction::Disable)
        );
    }
}
