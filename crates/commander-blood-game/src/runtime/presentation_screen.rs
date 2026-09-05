//! Concrete flat-memory host for the bridge's six-choice presentation panel.

use anyhow::{Context, Result};
use commander_blood_formats::descript::DescriptBackgroundSlot;
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    BridgeSpriteRect, CenteredSequenceSubtitleLine, DescriptMusicSelectionOutcome, GameSceneLink,
    Manu3AnimationSelector, PaletteRemapTable, PresentationChoiceNumber, PresentationDescriptPlan,
    PresentationMusicChange, PresentationPanelPhase, PresentationRenderRegion,
    PresentationRenderTarget, PresentationResourceId, PresentationSceneContext,
    PresentationSceneDispatchOutcome, PresentationSceneDispatchState, PresentationSceneStatus,
    PresentationScreenBackend, PresentationScreenOutcome, PresentationScreenState, RasterNoiseMode,
    RasterPoint, RasterSpanPaint, SceneTransitionLine, SceneTransitionPhase, SceneTransitionState,
    ScriptPresentationScanState, SequenceSubtitleOutcome, SequenceSubtitlePlayback,
    SequenceSubtitleRenderer, ShipPresentationState, build_banked_tint_table,
    decode_active_presentation_line, draw_framebuffer_noise_rect, draw_rect_outline,
    encode_active_presentation_line, fill_framebuffer_rect, present_sequence_subtitle,
    remap_framebuffer_rect, update_presentation_screen,
};

use super::game_lifecycle::native_scene_link_target;
use super::{ModernGameServices, RuntimePresentationScene};
use crate::ui::{RgbaUiOverlay, SequenceCaptionFont};

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
const SHIP_BRIDGE_REDRAW_FLAG: u16 = 8;
const BRIDGE_REDRAW_REQUESTED: u8 = 1;
const SHIP_DEPTH_TRANSITION_ACTIVE: u8 = 1;
const PRESENTATION_PANEL_SOUND_CLIP: u8 = 1;
const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;
const NOISE_RANDOM_MODULUS: u16 = u16::MAX;

/// Live state for the bridge panel, its HNM scene, and exact palette effects.
pub struct RuntimePresentationScreen {
    state: PresentationScreenState,
    scene_state: PresentationSceneDispatchState<DescriptBackgroundSlot>,
    scene: RuntimePresentationScene,
    console_tint: PaletteRemapTable,
    scene_frame_presented_output: Option<bool>,
    caption: RetainedSequenceCaption,
    channel: RgbaUiOverlay,
}

pub(super) struct RuntimeSceneTransitionDispatchContext<'state> {
    pub scene_link: GameSceneLink,
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
            scene_frame_presented_output: None,
            caption: RetainedSequenceCaption::new(),
            channel: RgbaUiOverlay::new(
                super::LOGICAL_FRAMEBUFFER_WIDTH,
                super::LOGICAL_FRAMEBUFFER_HEIGHT,
            ),
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

    /// Take the shared frame-ready write produced by this panel update, if any.
    pub fn take_scene_frame_presented_output(&mut self) -> Option<bool> {
        self.scene_frame_presented_output.take()
    }

    /// Retained caption pixels for production fidelity traces.
    pub(super) fn caption_rgba(&self) -> &[u8] {
        self.caption.overlay.pixels()
    }

    /// Resolve caption ownership at composition time, even when the bridge
    /// coordinator bypassed `screen_mode_update` on this game frame.
    pub(super) fn caption_overlay(&self) -> Option<&RgbaUiOverlay> {
        (self.state.active() && self.state.scene_status().queued).then_some(&self.caption.overlay)
    }

    pub(super) fn channel_overlay(&self) -> Option<&RgbaUiOverlay> {
        (self.state.active()
            && (self.state.scene_status().queued
                || self.state.phase() == PresentationPanelPhase::Active))
            .then_some(&self.channel)
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

    /// Invalidate the cached scene identity before navigation replaces its background.
    pub fn invalidate_scene_image(&mut self) {
        self.scene_state.loaded_scene_image = None;
    }

    /// Publish the shared scene row selected by the latest ship DESCRIPT lookup.
    pub fn set_ship_scene_vertical_offset(&mut self, vertical_offset: u16) {
        self.scene_state.present_policy.vertical_offset = usize::from(vertical_offset);
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
        scene_link: GameSceneLink,
        active_record_related: Option<ScriptObjectId>,
        scruter_jo_record: Option<ScriptObjectId>,
    ) -> Result<PresentationSceneDispatchOutcome> {
        import_ship_scene_state(ship, &mut self.scene_state);
        let outcome = self.scene.dispatch(
            services,
            &mut self.scene_state,
            native_scene_link_target(scene_link),
            active_record_related,
            scruter_jo_record,
            false,
            self.state.active(),
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
            scene_link,
            transition,
            presentation,
            lifecycle,
            active_record_related,
            scruter_jo_record,
            palette_transition_percent,
        } = context;
        let scene = &mut self.scene_state;
        scene.presentation.active_line = transition.active_line.map(SceneTransitionLine::number);
        import_scene_transition_queue_gate(&mut scene.presentation, presentation);
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
            native_scene_link_target(scene_link),
            Some(active_record_related),
            scruter_jo_record,
            render_snapshot_suppressed,
            self.state.active(),
        );

        transition.active_line = scene
            .presentation
            .active_line
            .map(SceneTransitionLine::from_number);
        transition.redraw_pending = scene.presentation.bridge_redraw_pending != u8::MIN;
        transition.bridge_blocked = scene.dispatch_blocked;
        export_scene_transition_queue_gate(&scene.presentation, presentation);
        lifecycle.presentation.request_flags =
            crate::native::bloodprg::PresentationRequestFlags::decode(
                scene.presentation.request_flags,
            );
        lifecycle.presentation.sequence_active = scene.sequence_active;
        lifecycle.presentation.active_line = scene.presentation.active_line;
        lifecycle.presentation.list_entry_metric = scene.entry_metric;
        lifecycle.presentation.list_read_wrap_index = scene.read_wrap_index;
        export_scene_transition_frame_presented(scene.frame_presented, lifecycle);
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
        self.scene_frame_presented_output = None;
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
        let secondary_presentation_mode = self.state.active();
        let mut backend = RuntimePresentationScreenBackend {
            services,
            scene: &mut self.scene,
            scene_state: &mut self.scene_state,
            console_tint: &self.console_tint,
            active_record_related,
            scruter_jo_record,
            scene_frame_presented_output: &mut self.scene_frame_presented_output,
            secondary_presentation_mode,
            deferred_error: None,
            caption: &mut self.caption,
            channel: &mut self.channel,
        };
        let outcome =
            update_presentation_screen(&mut self.state, &records, queued_scene_link, &mut backend);
        match (outcome, backend.deferred_error) {
            (Err(error), _) => Err(error),
            (Ok(_), Some(error)) => Err(error),
            (Ok(outcome), None) => {
                backend.caption.finish_frame(outcome);
                Ok(outcome)
            }
        }
    }
}

fn import_ship_scene_state(
    ship: &ShipPresentationState,
    scene: &mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
) {
    scene.presentation.active_line = decode_active_presentation_line(ship.active_line);
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
    ship.active_line = encode_active_presentation_line(scene.presentation.active_line);
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

fn import_scene_transition_queue_gate(
    scene: &mut crate::native::bloodprg::PresentationUpdateState,
    presentation: &ScriptPresentationScanState,
) {
    scene.gate_flags =
        (scene.gate_flags & !PRESENTATION_ACTIVE_GATE) | u8::from(presentation.c2_gate_active);
}

fn export_scene_transition_queue_gate(
    scene: &crate::native::bloodprg::PresentationUpdateState,
    presentation: &mut ScriptPresentationScanState,
) {
    presentation.c2_gate_active = scene.gate_flags & PRESENTATION_ACTIVE_GATE != u8::MIN;
}

fn export_scene_transition_frame_presented(
    frame_presented: bool,
    lifecycle: &mut crate::native::bloodprg::GameLifecycleState,
) {
    lifecycle.frame_presented = frame_presented;
}

struct RuntimePresentationScreenBackend<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    scene: &'services mut RuntimePresentationScene,
    scene_state: &'services mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
    console_tint: &'services PaletteRemapTable,
    active_record_related: Option<ScriptObjectId>,
    scruter_jo_record: Option<ScriptObjectId>,
    scene_frame_presented_output: &'services mut Option<bool>,
    secondary_presentation_mode: bool,
    deferred_error: Option<anyhow::Error>,
    caption: &'services mut RetainedSequenceCaption,
    channel: &'services mut RgbaUiOverlay,
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
        self.caption.overlay.clear();
        self.channel.clear();
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
        // The DOS `snd_stream_start` path resets the retained stream to page zero
        // even when DESCRIPT reused the same music name.
        let result = self.services.restart_navigation_music();
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
            self.caption.overlay.clear();
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
            presentation_context_link_target(context),
            self.active_record_related,
            self.scruter_jo_record,
            false,
            self.secondary_presentation_mode,
        )?;
        *self.scene_frame_presented_output = Some(self.scene_state.frame_presented);
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
        let visible_frame = match self.services.presentation_queue_metrics() {
            Ok(Some(metrics)) => metrics.sequence_index,
            // A temporarily unavailable queue clock is not an authored blank cue.
            Ok(None) => return,
            Err(error) => {
                self.record_error(Err(
                    error.context("reading the presentation subtitle sequence")
                ));
                return;
            }
        };
        let result = self.caption.update(
            &self.services.runtime().data().sequence_caption_font,
            &subtitles,
            playback,
            visible_frame,
        );
        self.record_error(result);
    }

    fn draw_choice_number(&mut self, choice: PresentationChoiceNumber) {
        self.channel.clear();
        self.services
            .runtime()
            .data()
            .dialogue_ui_assets
            .draw_channel(self.channel, choice);
    }

    fn cancel_scene_presentation(&mut self) {
        self.caption.overlay.clear();
        self.channel.clear();
        if release_scene_presentation(self.scene_state) {
            self.services.finish_presentation_sequence();
        }
    }

    fn prepare_choice_audio(&mut self) {
        self.services
            .restart_manu3_animation(Manu3AnimationSelector::PresentationChoice);
        let result = self.services.stop_audio();
        self.record_error(result);
    }

    fn reset_ship_camera(&mut self) {
        let result = self.services.reset_ship_hud();
        self.record_error(result);
    }
}

fn release_scene_presentation(
    scene: &mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
) -> bool {
    if scene.presentation.gate_flags & PRESENTATION_ACTIVE_GATE == u8::MIN {
        return false;
    }
    if scene.ship_active_flags & SHIP_BRIDGE_REDRAW_FLAG != u16::MIN {
        scene.presentation.bridge_redraw_pending = BRIDGE_REDRAW_REQUESTED;
    }
    scene.presentation.active_line = None;
    scene.presentation.gate_flags = u8::MIN;
    scene.presentation.request_flags &= !PRESENTATION_REQUEST_GATE;
    scene.frame_presented = false;
    true
}

fn presentation_context_link_target(context: &PresentationSceneContext<'_, GameSceneLink>) -> u16 {
    match context {
        PresentationSceneContext::Queued(scene_link) => native_scene_link_target(**scene_link),
        PresentationSceneContext::ContentPanel => PRESENTATION_CONTENT_TOP as u16,
    }
}

/// Cue advancement stays tied to decoded frames; its pixels survive page flips
/// and game frames where the video decoder has nothing new to present.
struct RetainedSequenceCaption {
    overlay: RgbaUiOverlay,
}

impl RetainedSequenceCaption {
    fn new() -> Self {
        Self {
            overlay: RgbaUiOverlay::new(
                super::LOGICAL_FRAMEBUFFER_WIDTH,
                super::LOGICAL_FRAMEBUFFER_HEIGHT,
            ),
        }
    }

    fn update(
        &mut self,
        font: &SequenceCaptionFont,
        subtitles: &[commander_blood_formats::descript::DescriptSequenceSubtitle],
        playback: &mut SequenceSubtitlePlayback,
        visible_frame: u16,
    ) -> Result<()> {
        let mut renderer = RuntimeSequenceSubtitleRenderer {
            font,
            overlay: &mut self.overlay,
            visible_frame,
            began_drawing: false,
        };
        let outcome = present_sequence_subtitle(subtitles, playback, &mut renderer)
            .context("drawing a DESCRIPT sequence subtitle")?;
        if outcome == SequenceSubtitleOutcome::Finished {
            self.overlay.clear();
        }
        Ok(())
    }

    fn finish_frame(&mut self, outcome: PresentationScreenOutcome) {
        if outcome != PresentationScreenOutcome::WaitingForScene {
            self.overlay.clear();
        }
    }
}

struct RuntimeSequenceSubtitleRenderer<'assets> {
    font: &'assets SequenceCaptionFont,
    overlay: &'assets mut RgbaUiOverlay,
    visible_frame: u16,
    began_drawing: bool,
}

impl SequenceSubtitleRenderer for RuntimeSequenceSubtitleRenderer<'_> {
    type Error = anyhow::Error;

    fn visible_frame(&self) -> u16 {
        self.visible_frame
    }

    fn draw_centered_line(&mut self, line: CenteredSequenceSubtitleLine<'_>) -> Result<()> {
        anyhow::ensure!(
            usize::from(line.color) == crate::ui::SEQUENCE_CAPTION_COLOR,
            "unexpected DESCRIPT caption color {}",
            line.color
        );
        u8::try_from(line.text.len())
            .context("DESCRIPT sequence subtitle line exceeds the BIOS byte limit")?;
        // The C planner's Waiting result draws nothing. Retain the last cue
        // until a real replacement (including the empty cue) is submitted.
        if !self.began_drawing {
            self.overlay.clear();
            self.began_drawing = true;
        }
        self.font
            .draw_text(self.overlay, line.text, line.position.map(i32::from));
        Ok(())
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
    use crate::runtime::VGA_BIOS_FONT_8X8;
    use commander_blood_formats::descript::DescriptSequenceSubtitle;

    const BIOS_GLYPH_WIDTH: usize = 8;
    const BIOS_GLYPH_HEIGHT: usize = 8;
    const BIOS_GLYPH_HIGHEST_BIT: u8 = 128;
    const TEST_SEQUENCE_CHARACTER: u8 = b'L';
    const TEST_SEQUENCE_POSITION: [u16; 2] = [16, 74];
    const TEST_SEQUENCE_COLOR: [u8; 4] = [69, 125, 190, 255];
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
    fn production_sequence_subtitle_uses_the_exact_bios_8x8_glyph() {
        let font = SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [17, 31, 47]).unwrap();
        let mut caption = RetainedSequenceCaption::new();
        let mut renderer = RuntimeSequenceSubtitleRenderer {
            font: &font,
            overlay: &mut caption.overlay,
            visible_frame: 1,
            began_drawing: false,
        };
        renderer
            .draw_centered_line(CenteredSequenceSubtitleLine {
                text: &[TEST_SEQUENCE_CHARACTER],
                position: TEST_SEQUENCE_POSITION,
                color: crate::ui::SEQUENCE_CAPTION_COLOR as u8,
            })
            .unwrap();

        let glyph = VGA_BIOS_FONT_8X8[usize::from(TEST_SEQUENCE_CHARACTER)];
        for (row, encoded) in glyph.into_iter().enumerate().take(BIOS_GLYPH_HEIGHT) {
            for column in usize::MIN..BIOS_GLYPH_WIDTH {
                let mask = BIOS_GLYPH_HIGHEST_BIT >> column;
                let expected = if encoded & mask != u8::MIN {
                    TEST_SEQUENCE_COLOR
                } else {
                    [0; 4]
                };
                let x = usize::from(TEST_SEQUENCE_POSITION[0]) + column;
                let y = usize::from(TEST_SEQUENCE_POSITION[1]) + row;
                assert_eq!(
                    &caption.overlay.pixels()
                        [(y * crate::runtime::LOGICAL_FRAMEBUFFER_WIDTH + x) * 4..][..4],
                    expected,
                    "BIOS glyph pixel differs at ({column}, {row})"
                );
            }
        }
    }

    #[test]
    fn caption_survives_refreshes_without_advancing_the_authored_cue_clock() {
        let font = SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [63; 3]).unwrap();
        let cues = [
            DescriptSequenceSubtitle::new(
                1,
                Box::from(b"CRYO Interactive Entertainment 1995".as_slice()),
            ),
            DescriptSequenceSubtitle::new(30, Box::from(b"Commander BLOOD  V 1.0".as_slice())),
            DescriptSequenceSubtitle::new(100, Box::from(b"".as_slice())),
        ];
        let mut playback = SequenceSubtitlePlayback::default();
        let mut caption = RetainedSequenceCaption::new();
        let mut ui = RgbaUiOverlay::new(320, 200);
        caption.update(&font, &cues, &mut playback, 30).unwrap();
        assert_eq!(playback.cue_index(), 1, "C draws before advancing a cue");
        caption.update(&font, &cues, &mut playback, 31).unwrap();
        let title = caption.overlay.pixels().to_vec();
        assert!(title.chunks_exact(4).any(|pixel| pixel[3] == 255));
        caption.update(&font, &cues, &mut playback, 0).unwrap();
        assert_eq!(
            caption.overlay.pixels(),
            title,
            "waiting for a queue clock erased the cue"
        );
        // The video/front/back pages may change while the caption remains owned
        // by the sequence. A render refresh must not rerun the cue planner.
        for _ in 0..120 {
            ui.clear();
            caption.finish_frame(PresentationScreenOutcome::WaitingForScene);
            ui.blit_overlay(&caption.overlay);
            assert_eq!(ui.pixels(), title);
            assert_eq!(playback.cue_index(), 1);
        }
        caption.update(&font, &cues, &mut playback, 100).unwrap();
        assert_eq!(caption.overlay.pixels(), title);
        assert_eq!(playback.cue_index(), 2);
        caption.update(&font, &cues, &mut playback, 101).unwrap();
        assert!(caption.overlay.pixels().iter().all(|&byte| byte == 0));
        ui.clear();
        ui.blit_overlay(&caption.overlay);
        assert!(ui.pixels().iter().all(|&byte| byte == 0));
    }

    #[test]
    fn final_composition_retains_caption_when_the_panel_update_is_bypassed() {
        let mut screen = RuntimePresentationScreen::new([[0; 3]; 256]).unwrap();
        screen.state.set_active(true);
        screen.state.set_scene_status(PresentationSceneStatus {
            queued: true,
            frame_presented: true,
        });
        let font = SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [63; 3]).unwrap();
        let cues = [DescriptSequenceSubtitle::new(
            1,
            Box::from(b"Commander BLOOD  V 1.0".as_slice()),
        )];
        screen
            .caption
            .update(&font, &cues, screen.state.subtitle_playback_mut(), 1)
            .unwrap();
        let expected = screen.caption_rgba().to_vec();
        let mut ui = RgbaUiOverlay::new(320, 200);
        // SceneDispatched/Inactive are bridge-coordinator early returns, not
        // caption-end events. Final composition must still recover the layer.
        for _ in 0..120 {
            ui.clear();
            screen.state.set_scene_status(PresentationSceneStatus {
                queued: true,
                frame_presented: false,
            });
            ui.blit_overlay(screen.caption_overlay().unwrap());
            assert_eq!(ui.pixels(), expected);
        }
        screen
            .state
            .set_scene_status(PresentationSceneStatus::default());
        assert!(screen.caption_overlay().is_none());
        screen.state.set_scene_status(PresentationSceneStatus {
            queued: true,
            frame_presented: false,
        });
        screen.state.set_active(false);
        assert!(screen.caption_overlay().is_none());
    }

    #[test]
    fn caption_is_cleared_on_exit_and_replacement_with_a_future_or_missing_cue() {
        let font = SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [63; 3]).unwrap();
        let cues = [DescriptSequenceSubtitle::new(
            1,
            Box::from(b"Title".as_slice()),
        )];
        let mut playback = SequenceSubtitlePlayback::default();
        let mut caption = RetainedSequenceCaption::new();
        for outcome in [
            PresentationScreenOutcome::Inactive,
            PresentationScreenOutcome::Initialized,
            PresentationScreenOutcome::Animated,
            PresentationScreenOutcome::WaitingForSelection,
            PresentationScreenOutcome::SceneLinesCompleted,
            PresentationScreenOutcome::InputAccepted,
            PresentationScreenOutcome::Finalized,
        ] {
            caption.update(&font, &cues, &mut playback, 1).unwrap();
            assert!(caption.overlay.pixels().iter().any(|&byte| byte != 0));
            caption.finish_frame(outcome);
            assert!(
                caption.overlay.pixels().iter().all(|&byte| byte == 0),
                "{outcome:?}"
            );
        }
        for replacement in [&cues[..], &[]] {
            caption.update(&font, &cues, &mut playback, 1).unwrap();
            playback.restart();
            // Loading a different DESCRIPT/video explicitly releases the old cue.
            caption.overlay.clear();
            caption
                .update(&font, replacement, &mut playback, 0)
                .unwrap();
            assert!(caption.overlay.pixels().iter().all(|&byte| byte == 0));
        }
    }

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

        assert_eq!(
            ship.active_line,
            crate::native::bloodprg::NO_PRESENTATION_LINE
        );
        assert_eq!(ship.presentation_gate, EXPORTED_PRESENTATION_GATE);
        assert!(!ship.scene_dispatch_blocked);
        assert_eq!(ship.flags, EXPORTED_SHIP_FLAGS);
        assert_eq!(ship.transition_percent, EXPORTED_TRANSITION_PERCENT);
        assert_eq!(ship.depth_opening_flags, EXPORTED_DEPTH_FLAGS);
        assert_eq!(ship.depth_step, EXPORTED_DEPTH_STEP);
        assert_eq!(ship.bridge_redraw_pending, u8::MIN);
    }

    #[test]
    fn scene_transition_queue_gate_does_not_alias_dialogue_activity() {
        const UNOWNED_GATE_BITS: u8 = 0b1010_0000;

        let mut scene = crate::native::bloodprg::PresentationUpdateState {
            gate_flags: UNOWNED_GATE_BITS,
            ..crate::native::bloodprg::PresentationUpdateState::default()
        };
        let mut presentation = ScriptPresentationScanState {
            active: true,
            c2_gate_active: false,
            ..ScriptPresentationScanState::default()
        };

        import_scene_transition_queue_gate(&mut scene, &presentation);
        assert_eq!(scene.gate_flags, UNOWNED_GATE_BITS);

        presentation.c2_gate_active = true;
        import_scene_transition_queue_gate(&mut scene, &presentation);
        assert_eq!(
            scene.gate_flags,
            UNOWNED_GATE_BITS | PRESENTATION_ACTIVE_GATE
        );

        scene.gate_flags &= !PRESENTATION_ACTIVE_GATE;
        export_scene_transition_queue_gate(&scene, &mut presentation);
        assert!(!presentation.c2_gate_active);
        assert!(presentation.active);
    }

    #[test]
    fn scene_transition_frame_readiness_overwrites_an_earlier_writer() {
        let mut lifecycle = crate::native::bloodprg::GameLifecycleState::default();
        lifecycle.frame_presented = true;

        export_scene_transition_frame_presented(false, &mut lifecycle);

        assert!(!lifecycle.frame_presented);
        export_scene_transition_frame_presented(true, &mut lifecycle);
        assert!(lifecycle.frame_presented);
    }

    #[test]
    fn panel_cancellation_reproduces_active_queue_cleanup_and_ship_redraw() {
        const UNOWNED_REQUEST_FLAG: u8 = 128;
        let mut scene = PresentationSceneDispatchState::<DescriptBackgroundSlot>::default();
        scene.presentation.active_line = Some(2);
        scene.presentation.gate_flags = PRESENTATION_ACTIVE_GATE;
        scene.presentation.request_flags = UNOWNED_REQUEST_FLAG | PRESENTATION_REQUEST_GATE;
        scene.ship_active_flags = SHIP_BRIDGE_REDRAW_FLAG;
        scene.frame_presented = true;

        assert!(release_scene_presentation(&mut scene));

        assert_eq!(scene.presentation.active_line, None);
        assert_eq!(scene.presentation.gate_flags, u8::MIN);
        assert_eq!(scene.presentation.request_flags, UNOWNED_REQUEST_FLAG);
        assert_eq!(
            scene.presentation.bridge_redraw_pending,
            BRIDGE_REDRAW_REQUESTED
        );
        assert!(!scene.frame_presented);
    }

    #[test]
    fn inactive_panel_cancellation_preserves_all_state() {
        let mut scene = PresentationSceneDispatchState::<DescriptBackgroundSlot>::default();
        scene.presentation.active_line = Some(7);
        scene.presentation.request_flags = PRESENTATION_REQUEST_GATE;
        scene.presentation.bridge_redraw_pending = 3;
        scene.ship_active_flags = SHIP_BRIDGE_REDRAW_FLAG;
        scene.frame_presented = true;
        let expected = scene.clone();

        assert!(!release_scene_presentation(&mut scene));
        assert_eq!(scene, expected);
    }

    #[test]
    fn panel_dispatch_preserves_queued_and_content_link_targets() {
        let queued = GameSceneLink::BridgePresentation(312);
        assert_eq!(
            presentation_context_link_target(&PresentationSceneContext::Queued(&queued)),
            312
        );
        assert_eq!(
            presentation_context_link_target(&PresentationSceneContext::ContentPanel),
            PRESENTATION_CONTENT_TOP as u16
        );
    }
}
