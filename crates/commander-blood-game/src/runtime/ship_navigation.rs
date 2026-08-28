//! Concrete flat-memory host for the recovered ship-navigation coordinator.

use std::mem::size_of;

use anyhow::{Context, Result, bail};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference,
};

use crate::native::bloodprg::{
    BridgeSpriteRect, ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListHandRequest,
    ChoiceListPointer, ChoiceListRect, ChoiceListState, FramebufferTransitionState, GameFontFace,
    GameLifecycleState, PaletteRemapTable, PresentationRequestFlags, RasterPoint,
    ScriptFieldSelector, ShipNavigationAccessCounter, ShipNavigationCandidate,
    ShipNavigationContext, ShipNavigationHost, ShipNavigationOutcome, ShipNavigationRelation,
    ShipNavigationState, TransitionRect, advance_framebuffer_rect_transition,
    build_palette_blend_remap_table, encode_active_presentation_line, measure_game_text_width,
    navigation_candidates, remap_framebuffer_rect, script_field_offset, update_choice_list,
    update_ship_navigation,
};

use super::choice_list::{RuntimeChoiceListStyle, draw_choice_list_rows};
use super::{ModernGameServices, OriginalGameRuntime, RuntimePlatformHost};

const ACTIVE_FLAG: u8 = 1;
const SHIP_ACTIVE_FLAG: u16 = 1;
const ROOT_UNRESTRICTED_CANDIDATE_FLAG: u8 = 2;
const NAVIGATION_RECORD_FLAGS_BYTE_INDEX: usize = 2;
const ACCESS_COUNTER_WORD_INDEX: usize = 10;
const NAVIGATION_TRIGGER_CANCEL_LABEL: &[u8] = b"CANCEL";
const NAVIGATION_TRIGGER_TRANSITION_STEPS: u8 = 6;
const NAVIGATION_REMAP_PERCENT: u8 = 50;
const NAVIGATION_REMAP_TARGET: [u8; 3] = [u8::MIN; 3];
const NAVIGATION_SELECTION_SOUND_CLIP: u8 = u8::MIN;
const INITIAL_CHOICE_TARGET_RECT: ChoiceListRect = ChoiceListRect {
    origin: [100, 0],
    size: [0, 120],
};
const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: 320,
    top: 0,
    bottom: 200,
};

/// Persistent navigation state, list interaction, and palette remap storage.
pub struct RuntimeShipNavigation {
    state: Option<ShipNavigationState<ScriptObjectId>>,
    choice_list: ChoiceListState,
    transition: FramebufferTransitionState,
    current_rect: ChoiceListRect,
    target_rect: ChoiceListRect,
    last_frame: Option<ChoiceListFrame>,
    remap_table: PaletteRemapTable,
}

impl Default for RuntimeShipNavigation {
    fn default() -> Self {
        Self {
            state: None,
            choice_list: ChoiceListState::default(),
            transition: FramebufferTransitionState::default(),
            current_rect: ChoiceListRect::default(),
            target_rect: INITIAL_CHOICE_TARGET_RECT,
            last_frame: None,
            remap_table: [u8::MIN; 256],
        }
    }
}

impl RuntimeShipNavigation {
    /// Borrow the translated coordinator state after its first update.
    pub fn state(&self) -> Option<&ShipNavigationState<ScriptObjectId>> {
        self.state.as_ref()
    }

    /// Borrow the most recent cancel-list frame, when layout has run.
    pub fn last_frame(&self) -> Option<&ChoiceListFrame> {
        self.last_frame.as_ref()
    }

    /// Advance one complete navigation frame against decoded scripts and flat buffers.
    pub fn update<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        lifecycle: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<ShipNavigationOutcome> {
        let current_target = services.current_ship_navigation_target()?;
        let profile_context = resolve_profile_context(services, current_target)?;
        let mut state = match self.state.clone() {
            Some(state) => state,
            None => initial_state(
                services,
                lifecycle,
                current_target,
                profile_context.access_counter,
            )?,
        };
        import_live_state(
            &mut state,
            services,
            lifecycle,
            current_target,
            profile_context.access_counter,
        )?;
        self.transition = FramebufferTransitionState {
            total_steps: u8::try_from(state.transition_total_steps)
                .context("ship navigation transition duration exceeds byte-sized native state")?,
            current_step: u8::try_from(state.transition_step)
                .context("ship navigation transition step exceeds byte-sized native state")?,
        };
        self.target_rect = state.choice_target_rect;
        let trigger_before_update = state.trigger_requested;
        state.previous_presentation_actor_state = services.previous_manu3_animation().value();
        if trigger_before_update {
            services.restore_previous_manu3_animation();
        }
        let list_style = services.choice_list_style();
        let primary_pointer_pressed = lifecycle.primary_pointer_pressed;

        let native_outcome;
        let deferred_error;
        let description_applied;
        {
            let mut backend = RuntimeShipNavigationBackend {
                services,
                lifecycle,
                platform,
                choice_list: &mut self.choice_list,
                transition: &mut self.transition,
                current_rect: &mut self.current_rect,
                target_rect: &mut self.target_rect,
                last_frame: &mut self.last_frame,
                remap_table: &mut self.remap_table,
                root_source_offset: profile_context.root_source_offset,
                honk: profile_context.honk,
                primary_pointer_pressed,
                list_style,
                description_applied: false,
                deferred_error: None,
            };
            native_outcome = update_ship_navigation(
                &mut state,
                &ShipNavigationContext {
                    ark: profile_context.ark,
                    unrestricted_candidates: profile_context.unrestricted_candidates,
                },
                &mut backend,
            );
            description_applied = backend.description_applied;
            deferred_error = backend.deferred_error.take();
        }

        state.transition_total_steps = u16::from(self.transition.total_steps);
        state.transition_step = u16::from(self.transition.current_step);
        state.choice_target_rect = self.target_rect;
        if trigger_before_update {
            write_access_counter(
                services,
                profile_context.counter_owner,
                state.access_counter,
            )?;
        }
        if let Some(error) = deferred_error {
            self.state = Some(state);
            return Err(error);
        }
        if description_applied {
            import_description_text_state(&mut state, services.text_presentation());
        }
        if let Some(target) = state.deferred_navigation_record.take() {
            services.defer_ship_actor_presentation(target);
        }
        if native_outcome == ShipNavigationOutcome::ResetToBridge {
            services.request_ship_hud_reinitialization()?;
            services.reset_presentation_word_choice()?;
            services.finish_ship_navigation_reset();
        }
        if take_navigation_snapshot_request(&mut state.navigation_snapshot_pending) {
            services.request_quick_save()?;
        }
        export_live_state(&state, services, lifecycle)?;
        self.state = Some(state);
        Ok(native_outcome)
    }
}

#[derive(Clone, Copy)]
struct NavigationProfileContext {
    ark: ScriptObjectId,
    honk: ScriptObjectId,
    root_source_offset: usize,
    unrestricted_candidates: bool,
    counter_owner: ScriptObjectId,
    access_counter: ShipNavigationAccessCounter,
}

fn resolve_profile_context(
    services: &ModernGameServices<'_>,
    current_target: ScriptObjectId,
) -> Result<NavigationProfileContext> {
    let profile = services
        .runtime()
        .current_profile()
        .context("ship navigation requires a loaded BloodScript profile")?;
    let state = profile.state();
    let root = state
        .objects()
        .first()
        .context("loaded BloodScript profile has no root VAR record")?;
    let root_flags = state
        .object_byte(root.id, NAVIGATION_RECORD_FLAGS_BYTE_INDEX)
        .and_then(|field| state.byte(field))
        .context("root VAR record has no navigation filter flags")?;
    let builtins = profile.builtins();
    let ark = builtins
        .ark
        .context("loaded BloodScript profile has no Ark object")?;
    let honk = builtins
        .horn
        .context("loaded BloodScript profile has no Honk object")?;
    let (counter_owner, access_counter) = resolve_access_counter(state, current_target)?;
    Ok(NavigationProfileContext {
        ark,
        honk,
        root_source_offset: root.source_offset(),
        unrestricted_candidates: root_flags & ROOT_UNRESTRICTED_CANDIDATE_FLAG != u8::MIN,
        counter_owner,
        access_counter,
    })
}

fn resolve_access_counter(
    state: &ScriptState,
    current_target: ScriptObjectId,
) -> Result<(ScriptObjectId, ShipNavigationAccessCounter)> {
    let current = state
        .object(current_target)
        .with_context(|| format!("ship target {current_target:?} is absent from VAR state"))?;
    let redirected = current.kind == ScriptObjectKind::Location;
    let counter_owner = if redirected {
        let redirect = state
            .object_word(current_target, ACCESS_COUNTER_WORD_INDEX)
            .context("location target has no access-counter redirect field")?;
        match state.object_reference(redirect) {
            Some(ScriptStateObjectReference::Object(owner)) => owner,
            Some(ScriptStateObjectReference::Sentinel) => {
                bail!("location target uses the sentinel as its access-counter owner")
            }
            None => bail!("location target has an invalid access-counter owner"),
        }
    } else {
        current_target
    };
    let counter = state
        .object_word(counter_owner, ACCESS_COUNTER_WORD_INDEX)
        .and_then(|field| state.word(field))
        .with_context(|| format!("access-counter owner {counter_owner:?} has no counter word"))?;
    let access_counter = if redirected {
        ShipNavigationAccessCounter::Redirected(counter)
    } else {
        ShipNavigationAccessCounter::Direct(counter)
    };
    Ok((counter_owner, access_counter))
}

fn write_access_counter(
    services: &mut ModernGameServices<'_>,
    owner: ScriptObjectId,
    counter: ShipNavigationAccessCounter,
) -> Result<()> {
    let value = match counter {
        ShipNavigationAccessCounter::Direct(value)
        | ShipNavigationAccessCounter::Redirected(value) => value,
    };
    let profile = services
        .runtime_mut()
        .current_profile_mut()
        .context("ship navigation lost its loaded BloodScript profile")?;
    let field = profile
        .state()
        .object_word(owner, ACCESS_COUNTER_WORD_INDEX)
        .with_context(|| format!("access-counter owner {owner:?} lost its counter word"))?;
    if !profile.state_mut().set_word(field, value) {
        bail!("failed to update access counter for {owner:?}");
    }
    Ok(())
}

fn initial_state(
    services: &ModernGameServices<'_>,
    lifecycle: &GameLifecycleState,
    current_target: ScriptObjectId,
    access_counter: ShipNavigationAccessCounter,
) -> Result<ShipNavigationState<ScriptObjectId>> {
    let ship = *services.ship_presentation_state();
    let text = services.text_presentation();
    let palette = services.palette_transition().state();
    Ok(ShipNavigationState {
        trigger_requested: flag_is_active(ship.bridge_redraw_pending),
        sequence_active: lifecycle.presentation.sequence_active,
        exit_pending: false,
        depth_opening: flag_is_active(ship.depth_opening_flags),
        presentation_deferred: text.menu_deferred,
        presentation_active: lifecycle.presentation.active,
        current_target,
        access_counter,
        presentation_actor_state: services.manu3_hand_state().requested_animation,
        previous_presentation_actor_state: services.previous_manu3_animation().value(),
        deferred_navigation_record: None,
        ui_state: ship.ui_state,
        transition_step: u16::MIN,
        transition_total_steps: u16::MIN,
        choice_target_rect: INITIAL_CHOICE_TARGET_RECT,
        scene_image_cached: false,
        resource_vertical_offset: services.ship_navigation_scene_vertical_offset(),
        text_menu_pending: text.menu_pending,
        text_selection: import_text_selection(text.selected_line)?,
        depth_closing: flag_is_active(ship.depth_closing_flags),
        depth_step: ship.depth_step,
        frame_presented: lifecycle.frame_presented,
        navigation_palette_staged: false,
        bridge_seek_target_arc: u16::MIN,
        bridge_seek_initial_distance: u16::MIN,
        navigation_screen_rebuild_pending: lifecycle.navigation_rebuild_pending,
        navigation_snapshot_pending: false,
        ship_active_flags: ship.flags,
        active_line: lifecycle.presentation.active_line,
        presentation_gate: ship.presentation_gate,
        hud_initialized: false,
        text_display_active: text.subtitle_display_active,
        presentation_hold_ready: lifecycle.presentation.hold_ready,
        depth_band_enabled: ship.depth_band_enabled,
        presentation_request_flags: text.request_flags.bits(),
        word_choice_phase: Default::default(),
        bridge_palette_transition_staged: false,
        palette_transition_last: palette.last,
        palette_transition_percent: palette.percent,
        palette_transition_increment: palette.increment,
    })
}

fn import_live_state(
    state: &mut ShipNavigationState<ScriptObjectId>,
    services: &ModernGameServices<'_>,
    lifecycle: &GameLifecycleState,
    current_target: ScriptObjectId,
    access_counter: ShipNavigationAccessCounter,
) -> Result<()> {
    let ship = *services.ship_presentation_state();
    let text = services.text_presentation();
    let palette = services.palette_transition().state();
    state.trigger_requested = flag_is_active(ship.bridge_redraw_pending);
    state.sequence_active = lifecycle.presentation.sequence_active;
    state.depth_opening = flag_is_active(ship.depth_opening_flags);
    state.presentation_deferred = text.menu_deferred;
    state.presentation_active = lifecycle.presentation.active;
    state.current_target = current_target;
    state.access_counter = access_counter;
    state.ui_state = ship.ui_state;
    state.scene_image_cached = services.ship_navigation_scene_image_cached()?;
    state.resource_vertical_offset = services.ship_navigation_scene_vertical_offset();
    state.text_menu_pending = text.menu_pending;
    state.text_selection = import_text_selection(text.selected_line)?;
    state.depth_closing = flag_is_active(ship.depth_closing_flags);
    state.depth_step = ship.depth_step;
    state.frame_presented = lifecycle.frame_presented;
    state.navigation_screen_rebuild_pending = lifecycle.navigation_rebuild_pending;
    state.ship_active_flags = ship.flags;
    state.active_line = lifecycle.presentation.active_line;
    state.presentation_gate = ship.presentation_gate;
    state.hud_initialized = services.ship_hud_initialized()?;
    state.text_display_active = text.subtitle_display_active;
    state.presentation_hold_ready = lifecycle.presentation.hold_ready;
    state.depth_band_enabled = ship.depth_band_enabled;
    state.presentation_request_flags = text.request_flags.bits();
    state.word_choice_phase = services.presentation_word_choice_phase()?;
    state.palette_transition_last = palette.last;
    state.palette_transition_percent = palette.percent;
    state.palette_transition_increment = palette.increment;
    Ok(())
}

fn export_live_state(
    state: &ShipNavigationState<ScriptObjectId>,
    services: &mut ModernGameServices<'_>,
    lifecycle: &mut GameLifecycleState,
) -> Result<()> {
    {
        let ship = services.ship_presentation_state_mut();
        ship.flags = state.ship_active_flags;
        ship.ui_state = state.ui_state;
        ship.depth_opening_flags =
            replace_active_flag(ship.depth_opening_flags, state.depth_opening);
        ship.depth_closing_flags =
            replace_active_flag(ship.depth_closing_flags, state.depth_closing);
        ship.depth_step = state.depth_step;
        ship.depth_band_enabled = state.depth_band_enabled;
        ship.presentation_gate = state.presentation_gate;
        ship.bridge_redraw_pending = u8::from(state.trigger_requested);
        ship.active_line = encode_active_presentation_line(state.active_line);
    }
    {
        let text = services.text_presentation_mut();
        text.menu_pending = state.text_menu_pending;
        text.selected_line = export_text_selection(state.text_selection)?;
        text.subtitle_display_active = state.text_display_active;
        text.menu_deferred = state.presentation_deferred;
        text.hold_ready = state.presentation_hold_ready;
        text.request_flags = PresentationRequestFlags::decode(state.presentation_request_flags);
    }
    services.set_ship_navigation_scene_vertical_offset(state.resource_vertical_offset);
    services
        .palette_transition_mut()
        .set_progress_percent(state.palette_transition_percent);
    services
        .palette_transition_mut()
        .set_increment(state.palette_transition_increment);
    lifecycle.presentation.sequence_active = state.sequence_active;
    lifecycle.presentation.ship_active = state.ship_active_flags & SHIP_ACTIVE_FLAG != u16::MIN;
    lifecycle.presentation.active_line = state.active_line;
    lifecycle.presentation.subtitle_display_active = state.text_display_active;
    lifecycle.presentation.menu_deferred = state.presentation_deferred;
    lifecycle.presentation.hold_ready = state.presentation_hold_ready;
    lifecycle.presentation.request_flags =
        PresentationRequestFlags::decode(state.presentation_request_flags);
    lifecycle.presentation.text_menu_pending = state.text_menu_pending;
    lifecycle.frame_presented = state.frame_presented;
    lifecycle.navigation_rebuild_pending = state.navigation_screen_rebuild_pending;
    Ok(())
}

fn import_text_selection(selection: Option<i8>) -> Result<Option<usize>> {
    selection
        .map(|line| {
            usize::try_from(line)
                .context("negative BloodScript text selector is not a valid selected line")
        })
        .transpose()
}

fn export_text_selection(selection: Option<usize>) -> Result<Option<i8>> {
    selection
        .map(|line| {
            i8::try_from(line).context("ship-navigation text selector exceeds native signed byte")
        })
        .transpose()
}

fn take_navigation_snapshot_request(pending: &mut bool) -> bool {
    std::mem::take(pending)
}

fn import_description_text_state(
    state: &mut ShipNavigationState<ScriptObjectId>,
    text: &crate::native::bloodprg::TextPresentationState,
) {
    state.presentation_deferred = text.menu_deferred;
    state.text_display_active = text.subtitle_display_active;
    state.presentation_hold_ready = text.hold_ready;
    state.presentation_request_flags = text.request_flags.bits();
}

const fn flag_is_active(flags: u8) -> bool {
    flags & ACTIVE_FLAG != u8::MIN
}

const fn replace_active_flag(flags: u8, active: bool) -> u8 {
    (flags & !ACTIVE_FLAG) | active as u8
}

struct RuntimeShipNavigationBackend<'services, 'window, 'lifecycle, 'platform> {
    services: &'services mut ModernGameServices<'window>,
    lifecycle: &'lifecycle mut GameLifecycleState,
    platform: &'platform mut RuntimePlatformHost<'window>,
    choice_list: &'services mut ChoiceListState,
    transition: &'services mut FramebufferTransitionState,
    current_rect: &'services mut ChoiceListRect,
    target_rect: &'services mut ChoiceListRect,
    last_frame: &'services mut Option<ChoiceListFrame>,
    remap_table: &'services mut PaletteRemapTable,
    root_source_offset: usize,
    honk: ScriptObjectId,
    primary_pointer_pressed: bool,
    list_style: RuntimeChoiceListStyle,
    description_applied: bool,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeShipNavigationBackend<'_, '_, '_, '_> {
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

    fn update_trigger_list(&mut self, layout_only: bool) -> Result<ChoiceListFrame> {
        let fonts = self.services.runtime().data().font_resources().clone();
        let pointer = self.services.input().pointer_sample().position;
        let current_hand_animation = self.services.manu3_hand_state().current_animation;
        let mut backend = RuntimeNavigationListBackend {
            runtime: self.services.runtime_mut(),
            fonts: &fonts,
            remap_table: self.remap_table,
            pointer: ChoiceListPointer {
                position: pointer,
                primary_pressed: self.primary_pointer_pressed,
            },
            current_hand_animation,
            hand_requests: Vec::new(),
            deferred_error: None,
        };
        let labels: [&[u8]; 0] = [];
        let cancel_label = self
            .list_style
            .extra_cancel_entry
            .then_some(NAVIGATION_TRIGGER_CANCEL_LABEL);
        let config = ChoiceListConfig {
            center_x: self.list_style.center_x,
            preserve_individual_widths: self.list_style.preserve_individual_widths,
            cancel_label,
            layout_only,
        };
        let frame = update_choice_list(&labels, config, self.choice_list, &mut backend);
        if !layout_only {
            draw_choice_list_rows(&mut *backend.runtime, &fonts, &labels, cancel_label, &frame)?;
        }
        backend.finish()?;
        let hand_requests = backend.take_hand_requests();
        drop(backend);
        self.services.apply_choice_list_hand_requests(hand_requests);
        if !layout_only && (frame.cancelled || frame.selected_item.is_some()) {
            self.services
                .play_loaded_sound_bank_clip(NAVIGATION_SELECTION_SOUND_CLIP)?;
        }
        Ok(frame)
    }

    fn remap_transition_region(&mut self, region: crate::native::bloodprg::TransitionRenderRegion) {
        let result = remap_framebuffer_rect(
            self.services.runtime_mut().front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            RasterPoint {
                x: i32::from(region.x),
                y: i32::from(region.y),
            },
            region.width,
            region.height,
            self.remap_table,
        )
        .context("remapping the navigation cancel-list transition")
        .map(|_| ());
        self.record(result, ());
    }
}

impl ShipNavigationHost<ScriptObjectId> for RuntimeShipNavigationBackend<'_, '_, '_, '_> {
    fn build_navigation_candidates(
        &mut self,
        current_target: &ScriptObjectId,
    ) -> Vec<ShipNavigationCandidate<ScriptObjectId>> {
        let result = build_candidates(
            self.services.runtime(),
            *current_target,
            self.honk,
            self.root_source_offset,
        );
        self.record(result, Vec::new())
    }

    fn load_candidate_description(&mut self, candidate: &ScriptObjectId) {
        self.description_applied = true;
        let result = self
            .services
            .apply_ship_target_description(*candidate)
            .map(|_| ());
        self.record(result, ());
    }

    fn measure_navigation_trigger_list(&mut self) -> ChoiceListRect {
        let result = self.update_trigger_list(true);
        let frame = self.record(
            result,
            ChoiceListFrame {
                rect: ChoiceListRect::default(),
                rows: Vec::new(),
                selected_item: None,
                cancelled: false,
            },
        );
        *self.current_rect = frame.rect;
        self.target_rect.origin[0] = frame.rect.origin[0];
        self.target_rect.size[0] = frame.rect.size[0];
        self.transition.current_step = u8::MIN;
        self.transition.total_steps = NAVIGATION_TRIGGER_TRANSITION_STEPS;
        *self.last_frame = Some(frame.clone());
        frame.rect
    }

    fn clear_navigation_band(&mut self) {
        let result = self.services.clear_ship_navigation_band();
        self.record(result, ());
    }

    fn load_navigation_background(&mut self) {
        let result = self.services.stage_ship_navigation_background().map(|_| ());
        self.record(result, ());
    }

    fn build_navigation_palette_remap(&mut self) {
        let result = build_palette_blend_remap_table(
            self.services.runtime().live_palette(),
            self.remap_table,
            NAVIGATION_REMAP_PERCENT,
            NAVIGATION_REMAP_TARGET,
        )
        .context("building the ship navigation darkening table");
        self.record(result, ());
    }

    fn run_alien_overlay_cycle(&mut self) {
        let result = self
            .services
            .run_runtime_alien_overlay_cycle(self.lifecycle, self.platform)
            .map(|_| ());
        self.record(result, ());
    }

    fn update_bridge_steering(&mut self) {
        let result = self.services.render_ship_hud_bridge_frame();
        self.record(result, ());
    }

    fn present_navigation_frame(&mut self) {
        self.services.runtime_mut().restore_back_buffer();
    }

    fn advance_navigation_list_transition(&mut self) -> bool {
        self.transition.total_steps = NAVIGATION_TRIGGER_TRANSITION_STEPS;
        let result = advance_framebuffer_rect_transition(
            self.transition,
            transition_rect(*self.current_rect),
            transition_rect(*self.target_rect),
        )
        .context("advancing the navigation cancel-list transition");
        match self.record(result, None) {
            Some(region) => {
                self.remap_transition_region(region);
                false
            }
            None => true,
        }
    }

    fn navigation_trigger_selected(&mut self) -> bool {
        let result = self.update_trigger_list(false);
        let frame = self.record(
            result,
            ChoiceListFrame {
                rect: *self.current_rect,
                rows: Vec::new(),
                selected_item: None,
                cancelled: false,
            },
        );
        *self.current_rect = frame.rect;
        let selected = frame.cancelled || frame.selected_item.is_some();
        *self.last_frame = Some(frame);
        selected
    }

    fn clear_bridge_display(&mut self) {
        self.services.clear_ship_travel_display();
    }

    fn clear_scene_palette(&mut self) {
        self.services.clear_navigation_scene_palette();
    }

    fn initialize_bridge_back_buffer(&mut self) {
        let result = self
            .services
            .initialize_navigation_back_buffer()
            .map(|_| ());
        self.record(result, ());
    }

    fn snapshot_hud_palette_and_reset_camera(&mut self) {
        let result = self
            .services
            .snapshot_navigation_hud_palette_and_camera()
            .and_then(|()| {
                self.services
                    .configure_navigation_bridge_palette_transition()
            });
        self.record(result, ());
    }
}

fn build_candidates(
    runtime: &OriginalGameRuntime,
    current_target: ScriptObjectId,
    honk: ScriptObjectId,
    root_source_offset: usize,
) -> Result<Vec<ShipNavigationCandidate<ScriptObjectId>>> {
    let profile = runtime
        .current_profile()
        .context("ship navigation requires a loaded BloodScript profile")?;
    let state = profile.state();
    navigation_candidates(state, current_target, honk)
        .map_err(|error| anyhow::anyhow!("building ship navigation candidates: {error:?}"))?
        .into_iter()
        .map(|record| {
            let object = state
                .object(record)
                .with_context(|| format!("navigation candidate {record:?} is absent"))?;
            let relation_offset =
                script_field_offset(object.kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
                    .with_context(|| {
                        format!("navigation candidate {record:?} has no relation field")
                    })?;
            if !relation_offset.is_multiple_of(size_of::<u16>()) {
                bail!("navigation candidate {record:?} has an unaligned relation field");
            }
            let relation_field = state
                .object_word(record, relation_offset / size_of::<u16>())
                .with_context(|| {
                    format!("navigation candidate {record:?} has a truncated relation field")
                })?;
            let raw_relation = state
                .word(relation_field)
                .context("navigation candidate relation could not be read")?;
            let relation = if usize::from(raw_relation) == root_source_offset {
                ShipNavigationRelation::RecordDirectoryRoot
            } else {
                match state.object_reference(relation_field) {
                    Some(ScriptStateObjectReference::Object(object)) => {
                        ShipNavigationRelation::Object(object)
                    }
                    Some(ScriptStateObjectReference::Sentinel) => ShipNavigationRelation::Other,
                    None => {
                        bail!("navigation candidate {record:?} has invalid relation {raw_relation}")
                    }
                }
            };
            Ok(ShipNavigationCandidate { record, relation })
        })
        .collect()
}

struct RuntimeNavigationListBackend<'runtime> {
    runtime: &'runtime mut OriginalGameRuntime,
    fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
    remap_table: &'runtime PaletteRemapTable,
    pointer: ChoiceListPointer,
    current_hand_animation: u16,
    hand_requests: Vec<ChoiceListHandRequest>,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeNavigationListBackend<'_> {
    fn finish(&mut self) -> Result<()> {
        match self.deferred_error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn record_error(&mut self, result: Result<()>) {
        if self.deferred_error.is_none()
            && let Err(error) = result
        {
            self.deferred_error = Some(error);
        }
    }

    fn take_hand_requests(&mut self) -> Vec<ChoiceListHandRequest> {
        std::mem::take(&mut self.hand_requests)
    }
}

impl ChoiceListBackend for RuntimeNavigationListBackend<'_> {
    fn measure_label(&mut self, label: &[u8]) -> u16 {
        match measure_game_text_width(label, GameFontFace::SquareCaps, self.fonts)
            .context("measuring a navigation trigger-list label")
        {
            Ok(width) => width,
            Err(error) => {
                if self.deferred_error.is_none() {
                    self.deferred_error = Some(error);
                }
                u16::MIN
            }
        }
    }

    fn prepare_background(&mut self, rect: ChoiceListRect) {
        let result = remap_framebuffer_rect(
            self.runtime.front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            RasterPoint {
                x: i32::from(rect.origin[0]),
                y: i32::from(rect.origin[1]),
            },
            rect.size[0],
            rect.size[1],
            self.remap_table,
        )
        .context("remapping the navigation trigger-list background")
        .map(|_| ());
        self.record_error(result);
    }

    fn pointer(&mut self) -> ChoiceListPointer {
        self.pointer
    }

    fn current_hand_animation(&self) -> u16 {
        self.current_hand_animation
    }

    fn request_hand_animation(&mut self, request: ChoiceListHandRequest) {
        if request.restart_current {
            self.current_hand_animation = u16::MIN;
        }
        self.hand_requests.push(request);
    }
}

fn transition_rect(rect: ChoiceListRect) -> TransitionRect {
    TransitionRect::new(
        rect.origin[0],
        rect.origin[1],
        rect.size[0] as i16,
        rect.size[1] as i16,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use crate::native::bloodprg::ScriptProfileId;
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    use super::*;

    const PTERRA_OBJECT_NAME: &[u8] = b"Pterra";
    const EXPECTED_PTERRA_CANDIDATE_NAMES: [&[&[u8]]; 5] =
        [&[b"Scruter_K"], &[b"Scruter_Jo"], &[], &[b"Scruter_K"], &[]];
    const EXPECTED_NAVIGATION_CANDIDATE_OCCURRENCES: [usize; 5] = [60, 62, 70, 81, 60];

    #[test]
    fn navigation_snapshot_alias_is_consumed_once_for_quick_save() {
        let mut pending = true;

        assert!(take_navigation_snapshot_request(&mut pending));
        assert!(!take_navigation_snapshot_request(&mut pending));
    }

    #[test]
    fn text_selector_round_trips_until_native_navigation_explicitly_clears_it() {
        let selected = Some(5_i8);

        let native = import_text_selection(selected).unwrap();

        assert_eq!(native, Some(5));
        assert_eq!(export_text_selection(native).unwrap(), selected);
        assert!(import_text_selection(Some(-2)).is_err());
        assert!(export_text_selection(Some(usize::from(i8::MAX as u8) + 1)).is_err());
    }

    #[test]
    fn every_profile_resolves_pterra_navigation_candidates_through_flat_state() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);

        for profile_id in ScriptProfileId::all() {
            runtime.load_profile(profile_id).unwrap();
            let profile = runtime.current_profile().unwrap();
            let pterra = profile
                .directory()
                .find_active_object(PTERRA_OBJECT_NAME)
                .unwrap();
            let honk = profile.builtins().horn.unwrap();
            let root_source_offset = profile.state().objects()[usize::MIN].source_offset();
            let candidates = build_candidates(&runtime, pterra, honk, root_source_offset).unwrap();
            let mut unique_candidates = BTreeSet::new();
            let names = candidates
                .iter()
                .map(|candidate| {
                    assert!(unique_candidates.insert(candidate.record));
                    if let ShipNavigationRelation::Object(related) = candidate.relation {
                        assert!(profile.state().object(related).is_some());
                    }
                    let name = profile.directory().object(candidate.record).unwrap().name();
                    assert!(runtime.data().descript_database().lookup(name).is_some());
                    name
                })
                .collect::<Vec<_>>();
            assert_eq!(
                names,
                EXPECTED_PTERRA_CANDIDATE_NAMES[usize::from(profile_id.value())],
                "SCRIPT{} Pterra candidate mapping",
                profile_id.value() + 1
            );
            assert!(
                runtime
                    .data()
                    .descript_database()
                    .lookup(PTERRA_OBJECT_NAME)
                    .is_some()
            );
        }
    }

    #[test]
    fn every_authored_target_resolves_runtime_navigation_candidates() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);

        for profile_id in ScriptProfileId::all() {
            runtime.load_profile(profile_id).unwrap();
            let profile = runtime.current_profile().unwrap();
            let honk = profile.builtins().horn.unwrap();
            let root_source_offset = profile.state().objects()[usize::MIN].source_offset();
            let mut candidate_count = usize::MIN;
            for target in profile.state().objects() {
                let candidates = build_candidates(&runtime, target.id, honk, root_source_offset)
                    .unwrap_or_else(|error| {
                        panic!(
                            "SCRIPT{} target {:?} failed candidate resolution: {error:#}",
                            profile_id.value() + 1,
                            target.id
                        )
                    });
                let mut unique_candidates = BTreeSet::new();
                for candidate in candidates {
                    assert!(unique_candidates.insert(candidate.record));
                    assert!(profile.directory().object(candidate.record).is_some());
                    if let ShipNavigationRelation::Object(related) = candidate.relation {
                        assert!(profile.state().object(related).is_some());
                    }
                    candidate_count += 1;
                }
            }
            assert_eq!(
                candidate_count,
                EXPECTED_NAVIGATION_CANDIDATE_OCCURRENCES[usize::from(profile_id.value())],
                "SCRIPT{} navigation candidate count",
                profile_id.value() + 1
            );
        }
    }

    fn original_game_data() -> Option<OriginalGameData> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root| OriginalGameDataPaths::from_root(root).ok())
        .and_then(|paths| {
            OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).ok()
        })
    }
}
