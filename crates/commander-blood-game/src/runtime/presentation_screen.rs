//! Concrete flat-memory host for the bridge's six-choice presentation panel.

use anyhow::{Context, Result};
use commander_blood_formats::descript::DescriptBackgroundSlot;
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    BridgeSpriteRect, CenteredSequenceSubtitleLine, DescriptMusicSelectionOutcome, FontPoint,
    GameSceneLink, PaletteRemapTable, PresentationChoiceNumber, PresentationDescriptPlan,
    PresentationMusicChange, PresentationPanelPhase, PresentationRenderRegion,
    PresentationRenderTarget, PresentationResourceId, PresentationSceneContext,
    PresentationSceneDispatchOutcome, PresentationSceneDispatchState, PresentationSceneStatus,
    PresentationScreenBackend, PresentationScreenOutcome, PresentationScreenState, RasterNoiseMode,
    RasterPoint, RasterSpanPaint, SceneTransitionLine, SceneTransitionPhase, SceneTransitionState,
    ScriptPresentationScanState, SequenceSubtitlePlayback, SequenceSubtitleRenderer,
    ShipPresentationState, build_banked_tint_table, draw_framebuffer_noise_rect, draw_rect_outline,
    fill_framebuffer_rect, present_sequence_subtitle, remap_framebuffer_rect,
    update_presentation_screen,
};

use super::{ModernGameServices, OriginalGameRuntime, RuntimePresentationScene};

const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: 320,
    top: 0,
    bottom: 200,
};
const PRESENTATION_CONTENT_TOP: usize = 10;
const SEQUENCE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(2);
const PRESENTATION_ACTIVE_GATE: u8 = 1;
const PRESENTATION_REQUEST_GATE: u8 = 2;
const SHIP_DEPTH_TRANSITION_ACTIVE: u8 = 1;
const PRESENTATION_PANEL_SOUND_CLIP: u8 = 1;
const PRESENTATION_SELECTION_ANIMATION: u16 = 14;
const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;
const NOISE_RANDOM_MODULUS: u16 = u16::MAX;

/// Live state for the bridge panel, its HNM scene, and exact palette effects.
pub struct RuntimePresentationScreen {
    state: PresentationScreenState,
    scene_state: PresentationSceneDispatchState<DescriptBackgroundSlot>,
    scene: RuntimePresentationScene,
    console_tint: PaletteRemapTable,
}

pub(super) struct RuntimeSceneTransitionDispatchContext<'state> {
    pub transition: &'state mut SceneTransitionState,
    pub presentation: &'state mut ScriptPresentationScanState,
    pub lifecycle: &'state mut crate::native::bloodprg::GameLifecycleState,
    pub active_record_related: ScriptObjectId,
    pub scruter_jo_record: Option<ScriptObjectId>,
    pub palette_transition_percent: &'state mut u16,
}

impl RuntimePresentationScreen {
    /// Construct the panel from the current complete indexed palette.
    pub fn new(initial_palette: crate::native::bloodprg::IndexedGamePalette) -> Result<Self> {
        let mut console_tint = [u8::MIN; 256];
        build_banked_tint_table(
            &initial_palette,
            &mut console_tint,
            BRIDGE_CONSOLE_TINT_FIRST,
        )
        .context("building the initial bridge console tint table")?;
        Ok(Self {
            state: PresentationScreenState::default(),
            scene_state: PresentationSceneDispatchState::default(),
            scene: RuntimePresentationScene::new(initial_palette),
            console_tint,
        })
    }

    /// Borrow the semantic panel state synchronized with the game lifecycle.
    pub const fn state(&self) -> &PresentationScreenState {
        &self.state
    }

    /// Mutably borrow the semantic panel state for lifecycle input synchronization.
    pub fn state_mut(&mut self) -> &mut PresentationScreenState {
        &mut self.state
    }

    /// Borrow the underlying general presentation-scene dispatcher state.
    pub const fn scene_state(&self) -> &PresentationSceneDispatchState<DescriptBackgroundSlot> {
        &self.scene_state
    }

    /// Return the armed and pending flags consumed by the alien-overlay coordinator.
    pub(super) const fn alien_overlay_flags(&self) -> (bool, bool) {
        (
            self.scene_state.alien_overlay_armed,
            self.scene_state.temporary_sound_trigger,
        )
    }

    /// Publish the coordinator's consumed alien-overlay flags.
    pub(super) fn set_alien_overlay_flags(&mut self, armed: bool, pending: bool) {
        self.scene_state.alien_overlay_armed = armed;
        self.scene_state.temporary_sound_trigger = pending;
    }

    /// Return the DESCRIPT background that must be restored after an overlay.
    pub(super) const fn loaded_scene_image(&self) -> Option<DescriptBackgroundSlot> {
        self.scene_state.loaded_scene_image
    }

    /// Invalidate the cached scene identity before navigation replaces its background.
    pub fn invalidate_scene_image(&mut self) {
        self.scene_state.loaded_scene_image = None;
    }

    /// Publish the PBM palette staged by ship navigation to the scene dispatcher.
    pub fn stage_navigation_palette(
        &mut self,
        palette: &crate::native::bloodprg::IndexedGamePalette,
    ) {
        self.scene.stage_navigation_palette(palette);
    }

    /// Synchronize colors changed through a native global-palette alias.
    pub(super) fn synchronize_scene_palette(
        &mut self,
        palette: crate::native::bloodprg::IndexedGamePalette,
    ) {
        self.scene.set_scene_palette(palette);
    }

    /// Dispatch the ship coordinator's current line through the shared scene owner.
    pub fn dispatch_ship_scene<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        ship: &mut ShipPresentationState,
        active_record_related: Option<ScriptObjectId>,
        scruter_jo_record: Option<ScriptObjectId>,
    ) -> Result<PresentationSceneDispatchOutcome> {
        import_ship_scene_state(ship, &mut self.scene_state);
        let outcome = self.scene.dispatch(
            services,
            &mut self.scene_state,
            active_record_related,
            scruter_jo_record,
            false,
        );
        export_ship_scene_state(&self.scene_state, ship);
        outcome
    }

    /// Dispatch a contact transition through the shared scene and stream owner.
    pub(super) fn dispatch_scene_transition<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        context: RuntimeSceneTransitionDispatchContext<'_>,
    ) -> Result<PresentationSceneDispatchOutcome> {
        let RuntimeSceneTransitionDispatchContext {
            transition,
            presentation,
            lifecycle,
            active_record_related,
            scruter_jo_record,
            palette_transition_percent,
        } = context;
        let scene = &mut self.scene_state;
        scene.presentation.active_line = transition.active_line.map(SceneTransitionLine::number);
        scene.presentation.gate_flags = (scene.presentation.gate_flags & !PRESENTATION_ACTIVE_GATE)
            | u8::from(presentation.active);
        scene.presentation.bridge_redraw_pending = u8::from(transition.redraw_pending);
        scene.presentation.request_flags = lifecycle.presentation.request_flags.bits();
        scene.sequence_active = lifecycle.presentation.sequence_active;
        scene.scene_gate = transition.scene_gate_active;
        scene.dispatch_blocked = transition.bridge_blocked;
        scene.present_policy.vertical_offset = usize::from(transition.image_vertical_offset);
        scene.palette_transition_percent = *palette_transition_percent;
        let render_snapshot_suppressed = transition.phase != SceneTransitionPhase::Inactive;

        let outcome = self.scene.dispatch(
            services,
            scene,
            Some(active_record_related),
            scruter_jo_record,
            render_snapshot_suppressed,
        );

        transition.active_line = scene
            .presentation
            .active_line
            .map(SceneTransitionLine::from_number);
        transition.redraw_pending = scene.presentation.bridge_redraw_pending != u8::MIN;
        transition.bridge_blocked = scene.dispatch_blocked;
        presentation.active = scene.presentation.gate_flags & PRESENTATION_ACTIVE_GATE != u8::MIN;
        lifecycle.presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(
                scene.presentation.request_flags,
            );
        lifecycle.presentation.sequence_active = scene.sequence_active;
        lifecycle.presentation.active_line = scene.presentation.active_line;
        lifecycle.presentation.list_entry_metric = scene.entry_metric;
        lifecycle.presentation.list_read_wrap_index = scene.read_wrap_index;
        lifecycle.frame_presented |= scene.frame_presented;
        *palette_transition_percent = scene.palette_transition_percent;
        outcome
    }

    /// Advance one exact panel frame using original resources and modern host services.
    pub fn update<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        queued_scene_link: &GameSceneLink,
        active_record_related: Option<ScriptObjectId>,
        scruter_jo_record: Option<ScriptObjectId>,
    ) -> Result<PresentationScreenOutcome> {
        if self.state.active() && self.state.phase() == PresentationPanelPhase::Begin {
            build_banked_tint_table(
                services.runtime().live_palette(),
                &mut self.console_tint,
                BRIDGE_CONSOLE_TINT_FIRST,
            )
            .context("refreshing the bridge console tint table")?;
        }
        let records = if self.state.active() {
            services.presentation_sequence_records()?
        } else {
            std::array::from_fn(|_| None)
        };
        let mut backend = RuntimePresentationScreenBackend {
            services,
            scene: &mut self.scene,
            scene_state: &mut self.scene_state,
            console_tint: &self.console_tint,
            active_record_related,
            scruter_jo_record,
            deferred_error: None,
        };
        let outcome =
            update_presentation_screen(&mut self.state, &records, queued_scene_link, &mut backend);
        match (outcome, backend.deferred_error) {
            (Err(error), _) => Err(error),
            (Ok(_), Some(error)) => Err(error),
            (Ok(outcome), None) => Ok(outcome),
        }
    }
}

fn import_ship_scene_state(
    ship: &ShipPresentationState,
    scene: &mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
) {
    scene.presentation.active_line = (ship.active_line != u16::MIN).then_some(ship.active_line);
    scene.presentation.gate_flags = (scene.presentation.gate_flags & !PRESENTATION_ACTIVE_GATE)
        | u8::from(ship.presentation_gate & u16::from(PRESENTATION_ACTIVE_GATE) != u16::MIN);
    scene.presentation.bridge_redraw_pending = ship.bridge_redraw_pending;
    scene.dispatch_blocked = ship.scene_dispatch_blocked;
    scene.ship_active_flags = ship.flags;
    scene.palette_transition_percent = ship.transition_percent;
    scene.depth_opening = ship.depth_opening_flags & SHIP_DEPTH_TRANSITION_ACTIVE != u8::MIN;
    scene.depth_step = ship.depth_step;
}

fn export_ship_scene_state(
    scene: &PresentationSceneDispatchState<DescriptBackgroundSlot>,
    ship: &mut ShipPresentationState,
) {
    ship.active_line = scene.presentation.active_line.unwrap_or(u16::MIN);
    ship.presentation_gate = (ship.presentation_gate & !u16::from(PRESENTATION_ACTIVE_GATE))
        | u16::from(scene.presentation.gate_flags & PRESENTATION_ACTIVE_GATE);
    ship.bridge_redraw_pending = scene.presentation.bridge_redraw_pending;
    ship.scene_dispatch_blocked = scene.dispatch_blocked;
    ship.flags = scene.ship_active_flags;
    ship.transition_percent = scene.palette_transition_percent;
    ship.depth_opening_flags =
        (ship.depth_opening_flags & !SHIP_DEPTH_TRANSITION_ACTIVE) | u8::from(scene.depth_opening);
    ship.depth_step = scene.depth_step;
}

struct RuntimePresentationScreenBackend<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    scene: &'services mut RuntimePresentationScene,
    scene_state: &'services mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
    console_tint: &'services PaletteRemapTable,
    active_record_related: Option<ScriptObjectId>,
    scruter_jo_record: Option<ScriptObjectId>,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimePresentationScreenBackend<'_, '_> {
    fn record_error(&mut self, result: Result<()>) {
        if self.deferred_error.is_none()
            && let Err(error) = result
        {
            self.deferred_error = Some(error);
        }
    }

    fn check_deferred_error(&mut self) -> Result<()> {
        match self.deferred_error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn target_pixels(&mut self, target: PresentationRenderTarget) -> &mut [u8] {
        match target {
            PresentationRenderTarget::Front => {
                self.services.runtime_mut().front_buffer_mut().pixels_mut()
            }
            PresentationRenderTarget::Back => {
                let (_front, back) = self.services.runtime_mut().presentation_buffers_mut();
                back
            }
        }
    }

    fn ensure_music_stream(&mut self) -> Result<()> {
        if self.services.navigation_music_position()?.is_none()
            && self.services.script_backend().assets().music().is_some()
        {
            self.services.restart_navigation_music()
        } else {
            self.services.check_audio()
        }
    }
}

impl PresentationScreenBackend for RuntimePresentationScreenBackend<'_, '_> {
    type RecordName = Box<[u8]>;
    type SceneLink = GameSceneLink;
    type Error = anyhow::Error;

    fn fill_region(
        &mut self,
        target: PresentationRenderTarget,
        color: u8,
        region: PresentationRenderRegion,
    ) {
        let result = fill_framebuffer_rect(
            self.target_pixels(target),
            LOGICAL_DISPLAY_CLIP,
            region_origin(region),
            region.size[0],
            region.size[1],
            color,
        )
        .context("filling a presentation panel region")
        .map(|_| ());
        self.record_error(result);
    }

    fn frame_region(
        &mut self,
        target: PresentationRenderTarget,
        color: u8,
        region: PresentationRenderRegion,
    ) {
        let result = draw_rect_outline(
            self.target_pixels(target),
            LOGICAL_DISPLAY_CLIP,
            region_origin(region),
            region.size[0],
            region.size[1],
            RasterSpanPaint::Solid(color),
        )
        .context("drawing a presentation panel frame")
        .map(|_| ());
        self.record_error(result);
    }

    fn remap_palette(
        &mut self,
        target: PresentationRenderTarget,
        region: PresentationRenderRegion,
    ) {
        let remap = *self.console_tint;
        let result = remap_framebuffer_rect(
            self.target_pixels(target),
            LOGICAL_DISPLAY_CLIP,
            region_origin(region),
            region.size[0],
            region.size[1],
            &remap,
        )
        .context("remapping a presentation panel region")
        .map(|_| ());
        self.record_error(result);
    }

    fn draw_noise(
        &mut self,
        target: PresentationRenderTarget,
        mode: u8,
        region: PresentationRenderRegion,
    ) {
        let random_pattern = self.services.next_random(NOISE_RANDOM_MODULUS);
        let result = draw_framebuffer_noise_rect(
            self.target_pixels(target),
            LOGICAL_DISPLAY_CLIP,
            RasterNoiseMode::from_native_word(u16::from(mode)),
            region_origin(region),
            region.size[0],
            region.size[1],
            |_| random_pattern,
        )
        .context("drawing presentation panel noise")
        .map(|_| ());
        self.record_error(result);
    }

    fn transition_presentation_entity(&mut self) {
        let result = self
            .services
            .transition_presentation_panel_entity()
            .map(|_| ());
        self.record_error(result);
    }

    fn play_presentation_clip(&mut self) {
        let result = self
            .services
            .play_loaded_sound_bank_clip(PRESENTATION_PANEL_SOUND_CLIP);
        self.record_error(result);
    }

    fn load_descript(&mut self, record: &Self::RecordName) -> Result<PresentationDescriptPlan> {
        self.check_deferred_error()?;
        let application = self
            .services
            .apply_presentation_description(record)?
            .with_context(|| {
                format!(
                    "no DESCRIPT record named {}",
                    String::from_utf8_lossy(record)
                )
            })?;
        let music = match application.music_selection() {
            Some(DescriptMusicSelectionOutcome::Changed) => PresentationMusicChange::Reload,
            Some(DescriptMusicSelectionOutcome::Reused) | None => PresentationMusicChange::Retained,
        };
        let scene_lines = self
            .services
            .script_backend()
            .assets()
            .sequence_videos()
            .iter()
            .map(|video| Box::from(video.as_bytes()))
            .collect();
        Ok(PresentationDescriptPlan::new(music, scene_lines))
    }

    fn reload_descript_music(&mut self) -> Result<()> {
        self.check_deferred_error()?;
        self.services.restart_navigation_music()
    }

    fn start_music_stream(&mut self) {
        let result = self.ensure_music_stream();
        self.record_error(result);
    }

    fn dispatch_scene(
        &mut self,
        context: &PresentationSceneContext<'_, Self::SceneLink>,
        line: Option<&[u8]>,
    ) -> Result<PresentationSceneStatus> {
        self.check_deferred_error()?;
        if self.scene_state.presentation.active_line.is_none() {
            let Some(line) = line else {
                return Ok(PresentationSceneStatus::default());
            };
            self.services.select_descript_sequence_video(line)?;
            self.scene_state.present_policy.vertical_offset = match context {
                PresentationSceneContext::Queued(_) => {
                    self.scene_state.present_policy.vertical_offset
                }
                PresentationSceneContext::ContentPanel => PRESENTATION_CONTENT_TOP,
            };
            self.scene_state.presentation.active_line = Some(SEQUENCE_PRESENTATION_LINE.get());
        }

        self.scene.dispatch(
            self.services,
            self.scene_state,
            self.active_record_related,
            self.scruter_jo_record,
            false,
        )?;
        Ok(PresentationSceneStatus {
            queued: self.scene_state.presentation.active_line.is_some()
                || self.scene_state.presentation.gate_flags & PRESENTATION_ACTIVE_GATE != u8::MIN,
            frame_presented: self.scene_state.frame_presented,
        })
    }

    fn draw_sequence_subtitle(&mut self, playback: &mut SequenceSubtitlePlayback) {
        let subtitles = self
            .services
            .script_backend()
            .assets()
            .sequence_subtitles()
            .to_vec();
        let visible_frame = self.services.presentation_decoded_frame_count() as u16;
        let mut renderer = RuntimeSequenceSubtitleRenderer {
            runtime: self.services.runtime_mut(),
            visible_frame,
        };
        let result = present_sequence_subtitle(&subtitles, playback, &mut renderer)
            .context("drawing a DESCRIPT sequence subtitle")
            .map(|_| ());
        self.record_error(result);
    }

    fn draw_choice_number(&mut self, choice: PresentationChoiceNumber) {
        let result = self
            .services
            .runtime_mut()
            .draw_presentation_choice(choice)
            .map(|_| ());
        self.record_error(result);
    }

    fn cancel_scene_presentation(&mut self) {
        self.services.finish_presentation_sequence();
        self.scene_state.presentation.active_line = None;
        self.scene_state.presentation.gate_flags = u8::MIN;
        self.scene_state.presentation.request_flags &= !PRESENTATION_REQUEST_GATE;
        self.scene_state.frame_presented = false;
    }

    fn prepare_choice_audio(&mut self) {
        self.services
            .request_manu3_animation(PRESENTATION_SELECTION_ANIMATION);
        let result = self.services.stop_audio();
        self.record_error(result);
    }

    fn reset_ship_camera(&mut self) {
        let result = self.services.reset_ship_hud();
        self.record_error(result);
    }
}

struct RuntimeSequenceSubtitleRenderer<'runtime> {
    runtime: &'runtime mut OriginalGameRuntime,
    visible_frame: u16,
}

impl SequenceSubtitleRenderer for RuntimeSequenceSubtitleRenderer<'_> {
    type Error = anyhow::Error;

    fn visible_frame(&self) -> u16 {
        self.visible_frame
    }

    fn draw_centered_line(&mut self, line: CenteredSequenceSubtitleLine<'_>) -> Result<()> {
        self.runtime
            .draw_small_font_line(
                line.text,
                FontPoint {
                    x: i32::from(line.position[0]),
                    y: i32::from(line.position[1]),
                },
                line.color,
            )
            .map(|_| ())
    }
}

fn region_origin(region: PresentationRenderRegion) -> RasterPoint {
    RasterPoint {
        x: i32::from(region.origin[0]),
        y: i32::from(region.origin[1]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL_LINE: u16 = 42;
    const INITIAL_PRESENTATION_GATE: u16 = 257;
    const INITIAL_SCENE_GATE_FLAGS: u8 = 128;
    const INITIAL_SHIP_FLAGS: u16 = 5;
    const INITIAL_TRANSITION_PERCENT: u16 = 75;
    const INITIAL_DEPTH_FLAGS: u8 = 129;
    const INITIAL_DEPTH_STEP: u8 = 6;
    const INITIAL_BRIDGE_REDRAW: u8 = 1;
    const EXPORTED_PRESENTATION_GATE: u16 = 256;
    const EXPORTED_SHIP_FLAGS: u16 = 17;
    const EXPORTED_TRANSITION_PERCENT: u16 = 100;
    const EXPORTED_DEPTH_FLAGS: u8 = 128;
    const EXPORTED_DEPTH_STEP: u8 = 2;

    #[test]
    fn inactive_screen_does_not_require_a_loaded_profile() {
        let state = PresentationScreenState::default();
        assert!(!state.active());
        assert_eq!(state.phase(), PresentationPanelPhase::Begin);
    }

    #[test]
    fn ship_scene_adapter_preserves_unowned_flag_bits_in_both_directions() {
        let mut ship = ShipPresentationState {
            flags: INITIAL_SHIP_FLAGS,
            scene_dispatch_blocked: true,
            active_line: INITIAL_LINE,
            presentation_gate: INITIAL_PRESENTATION_GATE,
            transition_percent: INITIAL_TRANSITION_PERCENT,
            depth_opening_flags: INITIAL_DEPTH_FLAGS,
            depth_step: INITIAL_DEPTH_STEP,
            bridge_redraw_pending: INITIAL_BRIDGE_REDRAW,
            ..ShipPresentationState::default()
        };
        let mut scene = PresentationSceneDispatchState::<DescriptBackgroundSlot>::default();
        scene.presentation.gate_flags = INITIAL_SCENE_GATE_FLAGS;

        import_ship_scene_state(&ship, &mut scene);

        assert_eq!(scene.presentation.active_line, Some(INITIAL_LINE));
        assert_eq!(
            scene.presentation.gate_flags,
            INITIAL_SCENE_GATE_FLAGS | PRESENTATION_ACTIVE_GATE
        );
        assert!(scene.dispatch_blocked);
        assert_eq!(scene.ship_active_flags, INITIAL_SHIP_FLAGS);
        assert_eq!(scene.palette_transition_percent, INITIAL_TRANSITION_PERCENT);
        assert!(scene.depth_opening);
        assert_eq!(scene.depth_step, INITIAL_DEPTH_STEP);
        assert_eq!(
            scene.presentation.bridge_redraw_pending,
            INITIAL_BRIDGE_REDRAW
        );

        scene.presentation.active_line = None;
        scene.presentation.gate_flags &= !PRESENTATION_ACTIVE_GATE;
        scene.dispatch_blocked = false;
        scene.ship_active_flags = EXPORTED_SHIP_FLAGS;
        scene.palette_transition_percent = EXPORTED_TRANSITION_PERCENT;
        scene.depth_opening = false;
        scene.depth_step = EXPORTED_DEPTH_STEP;
        scene.presentation.bridge_redraw_pending = u8::MIN;
        export_ship_scene_state(&scene, &mut ship);

        assert_eq!(ship.active_line, u16::MIN);
        assert_eq!(ship.presentation_gate, EXPORTED_PRESENTATION_GATE);
        assert!(!ship.scene_dispatch_blocked);
        assert_eq!(ship.flags, EXPORTED_SHIP_FLAGS);
        assert_eq!(ship.transition_percent, EXPORTED_TRANSITION_PERCENT);
        assert_eq!(ship.depth_opening_flags, EXPORTED_DEPTH_FLAGS);
        assert_eq!(ship.depth_step, EXPORTED_DEPTH_STEP);
        assert_eq!(ship.bridge_redraw_pending, u8::MIN);
    }
}
