//! Concrete flat-memory host for the recovered ship HUD coordinator.

use std::ops::Range;

use anyhow::{Context, Result};
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    IndexedGamePalette, Manu3AnimationSelector, PaletteRemapTable, ShipHudCoordinatorHost,
    ShipHudCoordinatorOutcome, ShipHudCoordinatorState, ShipHudInitializationContext,
    ShipHudPaletteTransition, ShipHudTargetListState, ShipTargetSelectionOutcome,
    ShipTargetSelectionState, build_palette_blend_remap_table, decode_active_presentation_line,
    encode_active_presentation_line, update_ship_hud,
};

use super::ModernGameServices;

const ACTIVE_FLAG: u8 = 1;
const SHIP_PRESENTATION_ACTIVE_FLAG: u16 = 1;
const INITIAL_TARGET_LIST_PHASE: u8 = 1;
const TARGET_LIST_TRANSITION_STEPS: u16 = 10;
const SHIP_HUD_ACTIVE_LINE: u16 = 3;
const HUD_DARKEN_PERCENT: u8 = 50;
const BLACK_BLEND_TARGET: [u8; 3] = [u8::MIN; 3];

/// Persistent semantic state joining ship presentation, target selection, and rendering.
pub struct RuntimeShipHud {
    coordinator: Option<ShipHudCoordinatorState<ScriptObjectId>>,
    selector: Option<ShipTargetSelectionState<ScriptObjectId>>,
    objects_at_arche_position: Vec<ScriptObjectId>,
    selector_targets: Vec<ScriptObjectId>,
    remap_table: PaletteRemapTable,
    remap_palette: Option<IndexedGamePalette>,
    remap_rows: Option<Range<u16>>,
}

impl Default for RuntimeShipHud {
    fn default() -> Self {
        Self {
            coordinator: None,
            selector: None,
            objects_at_arche_position: Vec::new(),
            selector_targets: Vec::new(),
            remap_table: [u8::MIN; 256],
            remap_palette: None,
            remap_rows: None,
        }
    }
}

impl RuntimeShipHud {
    /// Borrow the recovered coordinator state after its first update.
    pub fn coordinator(&self) -> Option<&ShipHudCoordinatorState<ScriptObjectId>> {
        self.coordinator.as_ref()
    }

    /// Borrow the most recent typed output of `vm_state_record_processor`.
    pub fn objects_at_arche_position(&self) -> &[ScriptObjectId] {
        &self.objects_at_arche_position
    }

    /// Discard profile-specific HUD, selector, and palette-remap state.
    pub fn reset(&mut self) {
        self.coordinator = None;
        self.selector = None;
        self.objects_at_arche_position.clear();
        self.selector_targets.clear();
        self.remap_palette = None;
        self.remap_rows = None;
    }

    /// Force the next HUD frame through its recovered one-time initialization path.
    pub fn request_reinitialization(&mut self) {
        if let Some(state) = &mut self.coordinator {
            state.initialized = false;
        }
        self.selector = None;
    }

    /// Advance one complete HUD frame against concrete scripts, pixels, audio, and input.
    pub fn update<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        lifecycle: &mut crate::native::bloodprg::GameLifecycleState,
    ) -> Result<ShipHudCoordinatorOutcome> {
        let context = services.ship_hud_initialization_context()?;
        let mut state = self
            .coordinator
            .clone()
            .unwrap_or_else(|| initial_coordinator_state(&context, services, lifecycle));
        import_live_state(&mut state, services, lifecycle);
        if let Some(selector) = &mut self.selector {
            selector.depth_opening_flags = services.ship_presentation_state().depth_opening_flags;
            selector.depth_step = services.ship_presentation_state().depth_step;
        }
        let initialized_before = state.initialized;
        if !initialized_before {
            services.activate_ship_target_list_style();
        }

        let native_outcome;
        let deferred_error;
        let description_applied;
        {
            let mut backend = RuntimeShipHudBackend {
                services,
                selector: &mut self.selector,
                objects_at_arche_position: &mut self.objects_at_arche_position,
                selector_targets: &mut self.selector_targets,
                remap_table: &mut self.remap_table,
                remap_palette: &mut self.remap_palette,
                remap_rows: &mut self.remap_rows,
                description_applied: false,
                deferred_error: None,
            };
            native_outcome = update_ship_hud(&mut state, &context, &mut backend);
            description_applied = backend.description_applied;
            deferred_error = backend.deferred_error.take();
        }
        if let Some(error) = deferred_error {
            return Err(error);
        }
        if description_applied {
            import_description_text_state(&mut state, services.text_presentation());
        }
        let outcome = native_outcome
            .map_err(|error| anyhow::anyhow!("invalid recovered ship HUD state: {error:?}"))?;

        if !initialized_before && state.initialized {
            services
                .configure_ship_hud_palette_transition()
                .context("staging the recovered ship HUD palette")?;
            if state.manu3_animation_requested {
                services.request_manu3_animation(Manu3AnimationSelector::BridgeActive);
            }
        }
        services.synchronize_ship_hud_palette_progress(
            state.palette_transition.percent,
            state.palette_transition.increment,
        );
        if let Some(target) = state.deferred_navigation_target.take() {
            services.queue_ship_hud_navigation_target(target)?;
        }
        export_live_state(&state, services, lifecycle);
        self.coordinator = Some(state);
        Ok(outcome)
    }
}

fn initial_coordinator_state(
    context: &ShipHudInitializationContext<ScriptObjectId>,
    services: &ModernGameServices<'_>,
    lifecycle: &crate::native::bloodprg::GameLifecycleState,
) -> ShipHudCoordinatorState<ScriptObjectId> {
    let ship = *services.ship_presentation_state();
    let text = services.text_presentation();
    let palette = services.palette_transition().state();
    ShipHudCoordinatorState {
        initialized: false,
        initialization_pending: flag_is_active(ship.hud_initialization_pending),
        subtitle_display_mode: text.subtitle_word_list_mode,
        hud_palette_staged: false,
        bridge_seek_target_arc: u16::MIN,
        bridge_view_frame: u16::MIN,
        ui_state: ship.ui_state,
        manu3_animation_requested: false,
        target_list: ShipHudTargetListState::default(),
        presentable_targets: Vec::new(),
        current_target: context.arche,
        scene_dispatch_blocked: ship.scene_dispatch_blocked,
        active_line: decode_active_presentation_line(ship.active_line),
        depth_band_enabled: ship.depth_band_enabled,
        resource_vertical_offset: context.scene_top_row,
        presentation_gate: ship.presentation_gate,
        palette_transition: ShipHudPaletteTransition {
            staged: false,
            percent: palette.percent,
            increment: palette.increment,
            first: palette.first,
            last: palette.last,
        },
        exit_pending: false,
        depth_opening: flag_is_active(ship.depth_opening_flags),
        depth_step: ship.depth_step,
        clip_snapshot_ready: ship.clip_snapshot_ready,
        text_display_active: text.subtitle_display_active,
        text_reveal_complete: text_reveal_complete(text),
        frame_presented: lifecycle.frame_presented,
        music_source_changed: false,
        deferred_navigation_target: None,
        ship_active_flags: ship.flags,
        sequence_active: lifecycle.presentation.sequence_active,
        bridge_redraw_pending: flag_is_active(ship.bridge_redraw_pending),
    }
}

fn import_live_state(
    state: &mut ShipHudCoordinatorState<ScriptObjectId>,
    services: &ModernGameServices<'_>,
    lifecycle: &crate::native::bloodprg::GameLifecycleState,
) {
    let ship = *services.ship_presentation_state();
    let text = services.text_presentation();
    let palette = services.palette_transition().state();
    state.initialization_pending = flag_is_active(ship.hud_initialization_pending);
    state.subtitle_display_mode = text.subtitle_word_list_mode;
    state.ui_state = ship.ui_state;
    state.scene_dispatch_blocked = ship.scene_dispatch_blocked;
    state.active_line = decode_active_presentation_line(ship.active_line);
    state.depth_band_enabled = ship.depth_band_enabled;
    state.presentation_gate = ship.presentation_gate;
    state.palette_transition.percent = palette.percent;
    state.palette_transition.increment = palette.increment;
    state.palette_transition.first = palette.first;
    state.palette_transition.last = palette.last;
    state.depth_opening = flag_is_active(ship.depth_opening_flags);
    state.depth_step = ship.depth_step;
    state.clip_snapshot_ready = ship.clip_snapshot_ready;
    state.text_display_active = text.subtitle_display_active;
    state.text_reveal_complete = text_reveal_complete(text);
    state.frame_presented = lifecycle.frame_presented;
    state.ship_active_flags = ship.flags;
    state.sequence_active = lifecycle.presentation.sequence_active;
    state.bridge_redraw_pending = flag_is_active(ship.bridge_redraw_pending);
    state.deferred_navigation_target = None;
}

fn export_live_state(
    state: &ShipHudCoordinatorState<ScriptObjectId>,
    services: &mut ModernGameServices<'_>,
    lifecycle: &mut crate::native::bloodprg::GameLifecycleState,
) {
    {
        let ship = services.ship_presentation_state_mut();
        ship.flags = state.ship_active_flags;
        ship.clip_snapshot_ready = state.clip_snapshot_ready;
        ship.ui_state = state.ui_state;
        ship.scene_dispatch_blocked = state.scene_dispatch_blocked;
        ship.depth_opening_flags =
            replace_active_flag(ship.depth_opening_flags, state.depth_opening);
        ship.depth_step = state.depth_step;
        ship.depth_band_enabled = state.depth_band_enabled;
        ship.presentation_gate = state.presentation_gate;
        ship.hud_initialization_pending = u8::from(state.initialization_pending);
        ship.transition_percent = state.palette_transition.percent;
        ship.bridge_redraw_pending = u8::from(state.bridge_redraw_pending);
        ship.active_line = encode_active_presentation_line(state.active_line);
    }
    {
        let text = services.text_presentation_mut();
        text.subtitle_display_active = state.text_display_active;
        text.subtitle_word_list_mode = state.subtitle_display_mode;
    }
    lifecycle.frame_presented = state.frame_presented;
    lifecycle.presentation.ship_active =
        state.ship_active_flags & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN;
    lifecycle.presentation.subtitle_display_active = state.text_display_active;
    lifecycle.presentation.subtitle_word_list_mode = state.subtitle_display_mode;
    lifecycle.presentation.sequence_active = state.sequence_active;
    lifecycle.presentation.c2_presentation_gate = state.presentation_gate != u16::MIN;
    lifecycle.presentation.active_line = state.active_line;
}

fn text_reveal_complete(text: &crate::native::bloodprg::TextPresentationState) -> bool {
    text.subtitle_reveal_cursor == Some(text.subtitle_text.len())
}

fn import_description_text_state(
    state: &mut ShipHudCoordinatorState<ScriptObjectId>,
    text: &crate::native::bloodprg::TextPresentationState,
) {
    state.subtitle_display_mode = text.subtitle_word_list_mode;
    state.text_display_active = text.subtitle_display_active;
    state.text_reveal_complete = text_reveal_complete(text);
}

const fn flag_is_active(flags: u8) -> bool {
    flags & ACTIVE_FLAG != u8::MIN
}

const fn replace_active_flag(flags: u8, active: bool) -> u8 {
    (flags & !ACTIVE_FLAG) | active as u8
}

struct RuntimeShipHudBackend<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    selector: &'services mut Option<ShipTargetSelectionState<ScriptObjectId>>,
    objects_at_arche_position: &'services mut Vec<ScriptObjectId>,
    selector_targets: &'services mut Vec<ScriptObjectId>,
    remap_table: &'services mut PaletteRemapTable,
    remap_palette: &'services mut Option<IndexedGamePalette>,
    remap_rows: &'services mut Option<Range<u16>>,
    description_applied: bool,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeShipHudBackend<'_, '_> {
    fn record<T>(&mut self, result: Result<T>, fallback: T) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                if self.deferred_error.is_none() {
                    self.deferred_error = Some(error);
                }
                fallback
            }
        }
    }

    fn ensure_selector(&mut self, current_target: ScriptObjectId) {
        if let Some(state) = self.selector.as_mut() {
            state.current_target = current_target;
            return;
        }
        *self.selector = Some(ShipTargetSelectionState {
            phase: INITIAL_TARGET_LIST_PHASE,
            transition_step: u16::MIN,
            transition_total_steps: TARGET_LIST_TRANSITION_STEPS,
            fallback_active: false,
            current_target,
            depth_opening_flags: u8::MIN,
            depth_step: u8::MIN,
        });
    }
}

impl ShipHudCoordinatorHost<ScriptObjectId> for RuntimeShipHudBackend<'_, '_> {
    fn clear_back_buffer(&mut self) {
        self.services.clear_ship_hud_back_buffer();
    }

    fn initialize_bridge_view(&mut self, seek_target_arc: u16, view_frame: u16) {
        let result = self
            .services
            .initialize_ship_hud_bridge_view(seek_target_arc, view_frame);
        self.record(result, ());
    }

    fn process_vm_state(&mut self) {
        let result = self.services.ship_objects_at_arche_position();
        *self.objects_at_arche_position = self.record(result, Vec::new());
    }

    fn build_presentable_targets(&mut self, root: &ScriptObjectId) -> Vec<ScriptObjectId> {
        let result = self.services.presentable_ship_targets(*root);
        let targets = self.record(result, Vec::new());
        *self.selector_targets = targets.clone();
        targets
    }

    fn load_target_description(&mut self, target: &ScriptObjectId) -> bool {
        self.ensure_selector(*target);
        self.description_applied = true;
        let result = self.services.apply_ship_target_description(*target);
        self.record(result, false)
    }

    fn dispatch_ship_scene_line(&mut self, vertical_offset: u16) {
        {
            let ship = self.services.ship_presentation_state_mut();
            ship.scene_dispatch_blocked = true;
            ship.active_line = SHIP_HUD_ACTIVE_LINE;
            ship.depth_band_enabled = true;
            ship.presentation_gate = u16::MIN;
        }
        let result = self.services.dispatch_ship_scene().with_context(|| {
            format!("dispatching ship HUD scene at authored row {vertical_offset}")
        });
        self.record(result.map(|_| ()), ());
    }

    fn copy_display_to_back_buffer(&mut self) {
        self.services.capture_ship_depth_source();
    }

    fn compose_depth_band(&mut self) {
        let result = self.services.compose_ship_depth_bands();
        self.record(result, false);
    }

    fn update_bridge_steering(&mut self) {
        let result = self.services.render_ship_hud_bridge_frame();
        self.record(result, ());
    }

    fn prepare_hud_remap(&mut self, rows: Range<u16>) {
        *self.remap_rows = Some(rows);
        let palette = *self.services.runtime().live_palette();
        if self.remap_palette.as_ref() == Some(&palette) {
            return;
        }
        let result = build_palette_blend_remap_table(
            &palette,
            self.remap_table,
            HUD_DARKEN_PERCENT,
            BLACK_BLEND_TARGET,
        )
        .context("building the ship HUD darkening table");
        if result.is_ok() {
            *self.remap_palette = Some(palette);
        }
        self.record(result, ());
    }

    fn commit_ship_entities(&mut self, entities: Range<u16>) {
        let result = self.services.commit_ship_entities(entities).map(|_| ());
        self.record(result, ());
    }

    fn copy_dirty_regions(&mut self) {
        let result = self.services.copy_ship_dirty_regions().map(|_| ());
        self.record(result, ());
    }

    fn update_target_selection(
        &mut self,
    ) -> (ShipTargetSelectionOutcome<ScriptObjectId>, bool, u8) {
        let Some(mut selector) = self.selector.take() else {
            self.deferred_error = Some(anyhow::anyhow!(
                "ship target selector was not initialized by the active target description"
            ));
            return (ShipTargetSelectionOutcome::NoSelection, false, u8::MIN);
        };
        let result = self
            .services
            .update_ship_target_selection(&mut selector, self.selector_targets);
        let depth_opening = flag_is_active(selector.depth_opening_flags);
        let depth_step = selector.depth_step;
        *self.selector = Some(selector);
        let selection = self.record(
            result,
            super::RuntimeShipTargetSelection {
                outcome: ShipTargetSelectionOutcome::NoSelection,
                selection_sound_requested: false,
            },
        );
        (selection.outcome, depth_opening, depth_step)
    }

    fn reset_audio_driver(&mut self) {
        let result = self.services.stop_navigation_music();
        self.record(result, ());
    }

    fn load_music_source(&mut self) {
        let result = self.services.load_navigation_music();
        self.record(result, ());
    }

    fn start_audio_stream(&mut self) {
        let result = self.services.ensure_navigation_music();
        self.record(result, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::TextPresentationState;

    #[test]
    fn subtitle_terminal_position_maps_to_the_native_nul_gate() {
        let mut text = TextPresentationState {
            subtitle_text: Box::from(&b"Pterra"[..]),
            ..TextPresentationState::default()
        };
        assert!(!text_reveal_complete(&text));
        text.subtitle_reveal_cursor = Some(text.subtitle_text.len() - 1);
        assert!(!text_reveal_complete(&text));
        text.subtitle_reveal_cursor = Some(text.subtitle_text.len());
        assert!(text_reveal_complete(&text));
    }

    #[test]
    fn active_flag_updates_preserve_unrelated_bits() {
        let original = 0b1010_1010;
        assert_eq!(replace_active_flag(original, true), 0b1010_1011);
        assert_eq!(replace_active_flag(original, false), original);
    }
}
