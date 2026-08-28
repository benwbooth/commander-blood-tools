//! Concrete flat-memory owner for the six recovered bridge actor handlers.

use anyhow::{Result, anyhow};
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    BlackHoleNavigationTarget, BlackHolePresentationActorBackend,
    BlackHolePresentationActorContext, BlackHolePresentationActorState,
    BlackHolePresentationBlockers, CameraPageFlipOutcome, CameraPresentationActorBackend,
    CameraPresentationActorOutcome, CameraPresentationActorState, CameraPresentationBlockers,
    CameraViewAnimation, GameLifecycleState, HyperjumpLocationPanelState,
    HyperjumpPresentationActorBackend, HyperjumpPresentationActorOutcome,
    HyperjumpPresentationActorState, Manu3AnimationSelector, NAV_ACTOR_SLOT_COUNT,
    NavActorBusyState, NavActorHandler, NavActorMouseState, NavActorSeekState, NavActorSlot,
    NavActorSlotBackend, NavActorSlotUpdateOutcome, PanelCloseActorBackend, PanelCloseActorState,
    PresentationBridgeMode, PresentationLine, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper, PrimaryPointerSample, RadioActorBackend, RadioActorOutcome,
    RadioActorState, ShipPaletteActorBackend, ShipPaletteActorOutcome, ShipPaletteActorState,
    ShipViewEntityId, latch_primary_pointer_hit, update_black_hole_presentation_actor,
    update_camera_presentation_actor, update_hyperjump_presentation_actor, update_nav_actor_slots,
    update_panel_close_actor, update_presentation_line, update_radio_actor,
    update_ship_palette_actor,
};

use super::ModernGameServices;

const CAMERA_ACTOR_SLOT: usize = 0;
const RADIO_ACTOR_SLOT: usize = 1;
const PANEL_ACTOR_SLOT: usize = 2;
const PALETTE_ACTOR_SLOT: usize = 3;
const BLACK_HOLE_ACTOR_SLOT: usize = 4;
const HYPERJUMP_ACTOR_SLOT: usize = 5;

const LOCATION_PANEL_ENTITY: ShipViewEntityId = ShipViewEntityId::new(0);
const SHARED_PRESENTATION_ENTITY: ShipViewEntityId = ShipViewEntityId::new(4);
const CAMERA_TRANSITION_CLIP: u8 = 3;
const RADIO_COMPLETION_CLIP: u8 = 2;
const SHIP_ACTIVATION_CLIP: u8 = 5;
const BLACK_HOLE_TRANSITION_CLIP: u8 = 5;
const HYPERJUMP_TRANSITION_CLIP: u8 = 5;
const SHIP_ACTIVE_FLAGS: u16 = 1;
const ACTIVE_ONLY_SLOT_FLAGS: u8 = 1;

/// Persistent state that was shared through native globals and actor records.
#[derive(Default)]
pub(super) struct RuntimeBridgeActors {
    playback: PresentationLinePlayback,
    camera: CameraPresentationActorState<ScriptObjectId>,
    radio: RadioActorState<ScriptObjectId>,
    panel: PanelCloseActorState,
    palette: ShipPaletteActorState,
    black_hole: BlackHolePresentationActorState<ScriptObjectId>,
    hyperjump: HyperjumpPresentationActorState<ScriptObjectId>,
    location_panel: HyperjumpLocationPanelState,
}

impl RuntimeBridgeActors {
    /// Clear frame latches reset by the recovered bridge-screen initializer.
    pub(super) fn reset_bridge_screen_latches(&mut self) {
        self.panel.completion_latched = false;
    }

    /// Return whether the panel actor latched completion for final remapping.
    pub(super) const fn completion_latched(&self) -> bool {
        self.panel.completion_latched
    }

    /// Return the camera actor's sole eight-step chart-transition countdown.
    pub(super) const fn camera_transition_step(&self) -> u8 {
        match self.camera.camera_animation {
            CameraViewAnimation::Unchanged => u8::MIN,
            CameraViewAnimation::Transitioning { steps_remaining } => steps_remaining,
        }
    }

    /// Synchronize the countdown after one translated navigation-camera frame.
    pub(super) fn set_camera_transition_step(&mut self, steps_remaining: u8) {
        self.camera.camera_animation = if steps_remaining == u8::MIN {
            CameraViewAnimation::Unchanged
        } else {
            CameraViewAnimation::Transitioning { steps_remaining }
        };
    }

    /// Publish location-panel ownership to actor blockers and camera presentation.
    pub(super) fn set_location_panel_active(&mut self, active: bool) {
        self.location_panel.active = active;
        self.camera.location_panel_active = active;
    }

    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
        slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    ) -> Result<NavActorSlotUpdateOutcome> {
        if services.take_ship_travel_actor_clear_requested() {
            slots[BLACK_HOLE_ACTOR_SLOT].flags = Default::default();
        }
        let bridge_view_frame = services.bridge_view_frame()? as u16;
        let seek_requested_before = services.bridge_seek_requested()?;
        let seek_target_before = services.bridge_seek_target_arc()?;
        let mut mouse = NavActorMouseState {
            primary_pressed: lifecycle.primary_pointer_pressed,
            press_pending: lifecycle.pointer_press_pending != u8::MIN,
        };
        let mut seek = NavActorSeekState {
            target_arc: seek_target_before,
            requested: seek_requested_before,
        };
        self.playback.busy = seek_requested_before;
        self.playback.redraw_requested = lifecycle.modal_ui_busy();

        let busy = NavActorBusyState {
            presentation_active: lifecycle.presentation.active,
            scene_presentation_queued: lifecycle.presentation.c2_presentation_gate,
            choice_active: lifecycle.profile_change_blockers.navigation_choice_active,
            save_active: lifecycle.profile_change_blockers.save_active,
            load_active: lifecycle.profile_change_blockers.load_active,
            console_item_selected: services.bridge_console_item_selected(),
            target_selection_active: lifecycle.navigation_target_selected,
            transition_pending: lifecycle.navigation_transition_pending,
            choice_sound_active: services.confirm_dialog_state().navigation_choice_gate != u8::MIN,
        };
        let mode = services.bridge_presentation_mode();

        let outcome = {
            let mut backend = RuntimeBridgeActorBackend {
                services,
                lifecycle,
                mode,
                playback: &mut self.playback,
                camera: &mut self.camera,
                radio: &mut self.radio,
                panel: &mut self.panel,
                palette: &mut self.palette,
                black_hole: &mut self.black_hole,
                hyperjump: &mut self.hyperjump,
                location_panel: &mut self.location_panel,
                callback_error: None,
            };
            update_nav_actor_slots(
                busy,
                bridge_view_frame,
                &mut mouse,
                &mut seek,
                slots,
                &mut backend,
            )
        }?;

        lifecycle.primary_pointer_pressed = mouse.primary_pressed;
        if !mouse.press_pending {
            lifecycle.pointer_press_pending = u8::MIN;
        }
        if seek.requested && (!seek_requested_before || seek.target_arc != seek_target_before) {
            services.request_bridge_seek(seek.target_arc)?;
        }
        services.set_bridge_actor_redraw_requested(self.playback.redraw_requested)?;
        // Native UI bit 2 is both the actor redraw latch and the bridge menu clamp.
        lifecycle.set_modal_ui_busy(self.playback.redraw_requested);
        services.set_ship_travel_actor_ready(
            slots[BLACK_HOLE_ACTOR_SLOT].flags.executable_flags() == ACTIVE_ONLY_SLOT_FLAGS,
        );
        Ok(outcome)
    }
}

struct RuntimeBridgeActorBackend<'state, 'window> {
    services: &'state mut ModernGameServices<'window>,
    lifecycle: &'state mut GameLifecycleState,
    mode: Option<PresentationBridgeMode>,
    playback: &'state mut PresentationLinePlayback,
    camera: &'state mut CameraPresentationActorState<ScriptObjectId>,
    radio: &'state mut RadioActorState<ScriptObjectId>,
    panel: &'state mut PanelCloseActorState,
    palette: &'state mut ShipPaletteActorState,
    black_hole: &'state mut BlackHolePresentationActorState<ScriptObjectId>,
    hyperjump: &'state mut HyperjumpPresentationActorState<ScriptObjectId>,
    location_panel: &'state mut HyperjumpLocationPanelState,
    callback_error: Option<anyhow::Error>,
}

impl RuntimeBridgeActorBackend<'_, '_> {
    fn record_callback<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if self.callback_error.is_none() {
                    self.callback_error = Some(error);
                }
                None
            }
        }
    }

    fn finish_callbacks(&mut self) -> Result<()> {
        self.callback_error.take().map_or(Ok(()), Err)
    }

    fn run_camera(
        &mut self,
        line: &mut PresentationLine,
        mouse: &mut NavActorMouseState,
        seek: &NavActorSeekState,
        slots: &[NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    ) -> Result<()> {
        let mut state = std::mem::take(self.camera);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        state.mouse_primary_pressed = mouse.primary_pressed;
        state.camera_view_active = self.services.bridge_camera_view_active();
        state.location_panel_active = self.location_panel.active;
        state.redraw_requested = playback.redraw_requested;
        let result = update_camera_presentation_actor(
            self.mode == Some(PresentationBridgeMode::Outer),
            CameraPresentationBlockers {
                primary_actor_busy: slot_has_state(&slots[BLACK_HOLE_ACTOR_SLOT]),
                secondary_actor_busy: slot_has_state(&slots[HYPERJUMP_ACTOR_SLOT]),
            },
            line,
            &mut playback,
            &mut state,
            self,
        );

        mouse.primary_pressed = state.mouse_primary_pressed;
        if matches!(
            &result,
            Ok(CameraPresentationActorOutcome::Blocked
                | CameraPresentationActorOutcome::CameraViewActivated
                | CameraPresentationActorOutcome::CameraViewDeactivated)
        ) {
            *self.location_panel = HyperjumpLocationPanelState::default();
        } else {
            self.location_panel.active = state.location_panel_active;
        }
        self.services
            .set_bridge_camera_view_active(state.camera_view_active);
        if state.screen_rebuild_pending {
            self.lifecycle.navigation_rebuild_pending = true;
            state.screen_rebuild_pending = false;
        }
        playback.redraw_requested = state.redraw_requested;
        *self.camera = state;
        *self.playback = playback;
        result?;
        self.finish_callbacks()
    }

    fn run_radio(&mut self, line: &mut PresentationLine, seek: &NavActorSeekState) -> Result<()> {
        let mut state = std::mem::take(self.radio);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        state.set_redraw_requested(playback.redraw_requested);
        state.set_pending_record(self.services.pending_ship_presentation_owner());
        let outcome = update_radio_actor(
            self.mode == Some(PresentationBridgeMode::FirstBand),
            line,
            &mut playback,
            &mut state,
            self,
        );
        playback.redraw_requested = state.redraw_requested();
        if matches!(outcome, Ok(RadioActorOutcome::Completed)) {
            self.services.clear_pending_ship_presentation_owner();
            if let Some(record) = state.take_deferred_record() {
                self.services.defer_ship_actor_presentation(record);
            }
        }
        *self.radio = state;
        *self.playback = playback;
        outcome?;
        self.finish_callbacks()
    }

    fn run_panel(
        &mut self,
        line: &mut PresentationLine,
        mouse: &mut NavActorMouseState,
        seek: &NavActorSeekState,
    ) -> Result<()> {
        let panel_active = self.services.presentation_screen_state()?.active();
        let mut state = std::mem::take(self.panel);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        state.panel_active = panel_active;
        state.scene_queued = self.lifecycle.presentation.c2_presentation_gate;
        state.mouse_primary_pressed = mouse.primary_pressed;
        state.mouse_press_pending = mouse.press_pending;
        state.redraw_requested = playback.redraw_requested;
        let result = update_panel_close_actor(
            self.mode == Some(PresentationBridgeMode::SecondBand),
            line,
            &mut playback,
            &mut state,
            self,
        );
        let panel_sync = self
            .services
            .set_presentation_screen_active(state.panel_active);
        self.lifecycle.presentation.c2_presentation_gate = state.scene_queued;
        mouse.primary_pressed = state.mouse_primary_pressed;
        mouse.press_pending = state.mouse_press_pending;
        playback.redraw_requested = state.redraw_requested;
        *self.panel = state;
        *self.playback = playback;
        result?;
        panel_sync?;
        self.finish_callbacks()
    }

    fn run_palette(&mut self, line: &mut PresentationLine, seek: &NavActorSeekState) -> Result<()> {
        let mut state = std::mem::take(self.palette);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        let live_palette = self.services.bridge_actor_live_palette();
        let outcome = update_ship_palette_actor(
            matches!(
                self.mode,
                Some(PresentationBridgeMode::Outer | PresentationBridgeMode::ThirdBand)
            ),
            line,
            &mut playback,
            &live_palette,
            &mut state,
            self,
        );
        if matches!(outcome, Ok(ShipPaletteActorOutcome::Completed)) {
            self.services
                .apply_bridge_actor_palette(state.bridge_palette());
            self.services.ship_presentation_state_mut().flags = SHIP_ACTIVE_FLAGS;
            self.services.ship_presentation_state_mut().depth_offset = state.ship_depth_offset();
            self.lifecycle.presentation.ship_active = state.ship_active();
        }
        *self.palette = state;
        *self.playback = playback;
        outcome?;
        self.finish_callbacks()
    }

    fn run_black_hole(
        &mut self,
        line: &mut PresentationLine,
        seek: &NavActorSeekState,
        slots: &[NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    ) -> Result<()> {
        let enabled = self.mode == Some(PresentationBridgeMode::Outer);
        let target = enabled
            .then(|| self.services.current_arche_navigation_target())
            .transpose()?
            .map(|(record, kind)| BlackHoleNavigationTarget { record, kind });
        let mut state = std::mem::take(self.black_hole);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        let result = update_black_hole_presentation_actor(
            BlackHolePresentationActorContext {
                enabled,
                actor_busy: slot_has_state(&slots[HYPERJUMP_ACTOR_SLOT]),
                camera_state_enables_absent_line: self.services.bridge_camera_view_active(),
                current_target: target.as_ref(),
            },
            line,
            &mut playback,
            &mut state,
            self,
        );
        if let Some(record) = state.take_deferred_record() {
            self.services.reset_ship_travel_phase();
            self.services.defer_ship_travel_target(record);
        }
        *self.black_hole = state;
        *self.playback = playback;
        result?;
        self.finish_callbacks()
    }

    fn run_hyperjump(
        &mut self,
        line: &mut PresentationLine,
        seek: &NavActorSeekState,
        slots: &[NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    ) -> Result<()> {
        let enabled = self.mode == Some(PresentationBridgeMode::Outer);
        let deferred_record =
            (enabled && self.hyperjump.deferred_record.is_none() && line.flags.ready)
                .then(|| self.services.current_ship_navigation_target())
                .transpose()?;
        let mut state = std::mem::take(self.hyperjump);
        let mut playback = std::mem::take(self.playback);
        playback.busy = seek.requested;
        if deferred_record.is_some() {
            state.deferred_record = deferred_record;
        }
        let outcome = update_hyperjump_presentation_actor(
            enabled,
            slot_has_state(&slots[BLACK_HOLE_ACTOR_SLOT]),
            line,
            &mut playback,
            &mut state,
            self,
        );
        if matches!(
            outcome,
            Ok(HyperjumpPresentationActorOutcome::NavigationQueued)
        ) && let Some(record) = state.deferred_record
        {
            self.services.defer_ship_navigation_target(record);
            self.lifecycle.navigation_transition_pending = true;
        }
        *self.hyperjump = state;
        *self.playback = playback;
        outcome?;
        self.finish_callbacks()
    }
}

impl PresentationLineStepper for RuntimeBridgeActorBackend<'_, '_> {
    type Error = anyhow::Error;

    fn update_line(
        &mut self,
        line: &mut PresentationLine,
        playback: &mut PresentationLinePlayback,
    ) -> Result<PresentationLineOutcome> {
        update_presentation_line(line, playback, self.services.runtime_mut())
    }
}

impl NavActorSlotBackend for RuntimeBridgeActorBackend<'_, '_> {
    type Error = anyhow::Error;

    fn hit_test(&mut self, _slot_index: usize, slot: &mut NavActorSlot, mouse: NavActorMouseState) {
        let Some(region) = slot.hit_region else {
            return;
        };
        latch_primary_pointer_hit(
            PrimaryPointerSample {
                primary_pressed: mouse.primary_pressed,
                position: self.services.input().pointer_sample().position,
            },
            region,
            &mut slot.flags.auto_seek,
        );
    }

    fn reset_presentation_entity(&mut self, _slot_index: usize) -> Result<()> {
        self.services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ())
    }

    fn update_actor(
        &mut self,
        handler: NavActorHandler,
        slot_index: usize,
        mouse: &mut NavActorMouseState,
        seek: &NavActorSeekState,
        slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    ) -> Result<()> {
        let expected_slot = match handler {
            NavActorHandler::Five => CAMERA_ACTOR_SLOT,
            NavActorHandler::Four => RADIO_ACTOR_SLOT,
            NavActorHandler::Three => PANEL_ACTOR_SLOT,
            NavActorHandler::Two => PALETTE_ACTOR_SLOT,
            NavActorHandler::One => BLACK_HOLE_ACTOR_SLOT,
            NavActorHandler::Zero => HYPERJUMP_ACTOR_SLOT,
        };
        if slot_index != expected_slot {
            return Err(anyhow!(
                "bridge actor {handler:?} was dispatched for slot {slot_index}, expected {expected_slot}"
            ));
        }

        let mut line = slots[slot_index].presentation_line();
        let result = match handler {
            NavActorHandler::Five => self.run_camera(&mut line, mouse, seek, slots),
            NavActorHandler::Four => self.run_radio(&mut line, seek),
            NavActorHandler::Three => self.run_panel(&mut line, mouse, seek),
            NavActorHandler::Two => self.run_palette(&mut line, seek),
            NavActorHandler::One => self.run_black_hole(&mut line, seek, slots),
            NavActorHandler::Zero => self.run_hyperjump(&mut line, seek, slots),
        };
        slots[slot_index].apply_presentation_line(line);
        result
    }
}

impl CameraPresentationActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn request_camera_hand_animation(&mut self) {
        self.services
            .request_manu3_animation(Manu3AnimationSelector::CameraOrHyperjump);
    }

    fn mark_location_panel_entity_dirty(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(LOCATION_PANEL_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }

    fn flip_camera_page(&mut self) -> CameraPageFlipOutcome {
        let ship_active = self.lifecycle.presentation.ship_active;
        let result = self.services.flip_bridge_camera_page(ship_active);
        self.record_callback(result)
            .unwrap_or(CameraPageFlipOutcome::KeepCurrentView)
    }

    fn play_camera_transition_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(CAMERA_TRANSITION_CLIP);
        self.record_callback(result);
    }

    fn reset_ship_camera_and_palette(&mut self) {
        let result = self.services.snapshot_navigation_hud_palette_and_camera();
        self.record_callback(result);
    }

    fn mark_presentation_entity_dirty(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }
}

impl RadioActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn request_radio_hand_animation(&mut self) {
        self.services
            .request_manu3_animation(Manu3AnimationSelector::RadioOrb);
    }

    fn play_radio_completion_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(RADIO_COMPLETION_CLIP);
        self.record_callback(result);
    }

    fn reset_presentation_entity(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }

    fn reload_radio_sound_bank(&mut self) {
        let result = self.services.load_radio_sound_bank();
        self.record_callback(result);
    }
}

impl PanelCloseActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn request_panel_close_hand_animation(&mut self) {
        self.services
            .request_manu3_animation(Manu3AnimationSelector::PanelClose);
    }

    fn begin_panel_close_if_open(&mut self) -> bool {
        match self.services.begin_presentation_panel_close_if_open() {
            Ok(started) => started,
            Err(error) => {
                self.record_callback::<()>(Err(error));
                false
            }
        }
    }

    fn finalize_scene_presentation(&mut self) {
        self.services
            .finish_bridge_actor_scene_presentation(self.lifecycle);
    }

    fn reset_presentation_entity(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }
}

impl ShipPaletteActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn request_ship_palette_hand_animation(&mut self) {
        self.services
            .request_manu3_animation(Manu3AnimationSelector::ShipPalette);
    }

    fn play_ship_activation_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(SHIP_ACTIVATION_CLIP);
        self.record_callback(result);
    }
}

impl BlackHolePresentationActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn restart_black_hole_hand_animation(&mut self) {
        self.services
            .restart_manu3_animation(Manu3AnimationSelector::BlackHoleOrLeftChart);
    }

    fn presentation_blockers(&self) -> BlackHolePresentationBlockers {
        BlackHolePresentationBlockers {
            location_panel_active: self.location_panel.active,
            camera_presentation_active: self.camera.active,
        }
    }

    fn mark_presentation_entity_dirty(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }

    fn play_black_hole_transition_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(BLACK_HOLE_TRANSITION_CLIP);
        self.record_callback(result);
    }
}

impl HyperjumpPresentationActorBackend for RuntimeBridgeActorBackend<'_, '_> {
    fn restart_hyperjump_hand_animation(&mut self) {
        self.services
            .restart_manu3_animation(Manu3AnimationSelector::CameraOrHyperjump);
    }

    fn location_panel_state(&self) -> HyperjumpLocationPanelState {
        *self.location_panel
    }

    fn camera_transition_active(&self) -> bool {
        !matches!(self.camera.camera_animation, CameraViewAnimation::Unchanged)
    }

    fn start_camera_transition(&mut self, steps: u8) {
        self.camera.camera_animation = CameraViewAnimation::Transitioning {
            steps_remaining: steps,
        };
    }

    fn close_location_panel(&mut self) {
        *self.location_panel = HyperjumpLocationPanelState::default();
    }

    fn mark_location_panel_entity_dirty(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(LOCATION_PANEL_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }

    fn mark_presentation_entity_dirty(&mut self) {
        let result = self
            .services
            .transition_ship_view_entity(SHARED_PRESENTATION_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }

    fn play_hyperjump_transition_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(HYPERJUMP_TRANSITION_CLIP);
        self.record_callback(result);
    }
}

const fn slot_has_state(slot: &NavActorSlot) -> bool {
    slot.flags.executable_flags() != u8::MIN
}
