pub const SHIP_3D_MAX_DEPTH_OFFSET: u16 = 0x41;
pub const SHIP_3D_TRANSITION_OPEN_STEP: u8 = 0x04;
pub const SHIP_3D_TRANSITION_CLOSE_STEP: u8 = 0x08;
pub const SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD: u16 = 0x78;
pub const SHIP_3D_PLANE_ROW_BYTES: usize = 0x50;
pub const SHIP_3D_PLANE_PAGE_BYTES: usize = 0x1f40;
pub const SHIP_3D_PLANE_BASE_ROWS: usize = 0x23;
pub const SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET: usize = 0xc000;
pub const SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET: usize = 0xdf40;
pub const SHIP_3D_PLANE_DEST_BYTES: usize = SHIP_3D_PLANE_PAGE_BYTES * 2;
pub const SHIP_3D_SCROLL_MODE_HOLD: u16 = 0x000a;
pub const SHIP_3D_TARGET_EXIT_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_TARGET_RECORD_HEADER_BYTES: u16 = 0x0004;
pub const SHIP_3D_TARGET_OPEN_STEP: u8 = 0x06;
pub const SHIP_3D_INTERPOLATION_WORDS: usize = 4;
pub const SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH: u16 = 0x64;
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH: u16 = 0x37;
pub const SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING: u16 = 0x14;
pub const SHIP_3D_TARGET_LAYOUT_ROW_STEP: u16 = 0x0b;
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT: u16 = 0x0a;
pub const SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING: u16 = 0x08;
pub const SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT: u16 = 0xc8;
pub const SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN: u16 = 0xffff;
pub const SHIP_3D_TARGET_HIT_TEST_TOP_INSET: u16 = 0x04;
pub const SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET: u16 = 0x08;
pub const SHIP_3D_TARGET_HOVER_PRESENTATION_MODE: u16 = 0x0006;
pub const SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE: u16 = 0x0007;
pub const SHIP_3D_TARGET_IDLE_PRESENTATION_MODE: u16 = 0x0001;
pub const SHIP_3D_TARGET_DRAW_X_INSET: u16 = 0x0a;
pub const SHIP_3D_TARGET_DEFAULT_TEXT_COLOR: u8 = 0xe8;
pub const SHIP_3D_TARGET_HOVER_TEXT_COLOR: u8 = 0xef;
pub const SHIP_3D_TARGET_ACTIVE_TEXT_COLOR: u8 = 0xfe;
pub const SHIP_3D_TARGET_EXTRA_LABEL_OFFSET: u16 = 0x0174;
pub const SHIP_3D_TARGET_ALIAS_LABEL_OFFSET: u16 = 0x273b;
pub const SHIP_3D_NAV_CHOICE_MIN_GATE: u16 = 0x28;
pub const SHIP_3D_NAV_CHOICE_MAX_GATE: u16 = 0x3c;
pub const SHIP_3D_NAV_CHOICE_AXIS_BIAS: u16 = 0x2d;
pub const SHIP_3D_NAV_CHOICE_RIGHT_BASE: u16 = 0x011f;
pub const SHIP_3D_NAV_CHOICE_X_WIDTH: u16 = 0x006e;
pub const SHIP_3D_NAV_CHOICE_Y_BASE: u16 = 0x0048;
pub const SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE: u8 = 0x12;
pub const SHIP_3D_NAV_CHOICE_COUNT: u8 = 5;
pub const SHIP_3D_NAV_CHOICE_PALETTE_FIRST: u8 = 0x7b;
pub const SHIP_3D_NAV_CHOICE_PRESENTATION_MODE: u16 = 0x0005;
pub const SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS: u8 = 0x0c;
pub const SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG: u8 = 0x08;
pub const SHIP_3D_NAV_CHOICE_HOLD_TICKS: u16 = 0x005a;
pub const SHIP_3D_NAV_CHOICE_HANDLER_PHASE: u8 = 0x01;
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_BASE: u16 = 0x0050;
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_STEP: u16 = 0x0012;
pub const SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X: u16 = 0x0064;
pub const SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION: u8 = 0x0a;
pub const SHIP_3D_NAV_CHOICE_SELECT_SOUND: u16 = 0x0004;
pub const SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE: u16 = 0x00c3;
pub const SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG: u8 = 0x04;
pub const SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING: u8 = 0x02;
pub const SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET: u16 = 0x0d16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTransitionState {
    pub hold_ticks: u16,
    pub transition_armed: bool,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dDepthState {
    pub depth_offset: u16,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dPlaneBandCopy {
    pub row_count: usize,
    pub byte_count: usize,
    pub first_source_start: usize,
    pub first_dest_start: usize,
    pub second_source_start: usize,
    pub second_dest_start: usize,
    pub new_scroll_value: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetSelectorState {
    pub current_target: u16,
    pub target_select_phase: u8,
    pub target_fallback: bool,
    pub target_animation_tick: u8,
    pub opening: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetSelection {
    pub ax: u16,
    pub used_fallback_table: bool,
    pub ran_layout_prepass: bool,
    pub phase_gate_blocked: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dInterpolationGate {
    pub duration_ticks: u8,
    pub current_tick: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dInterpolationStep {
    Active([u16; SHIP_3D_INTERPOLATION_WORDS]),
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetListLayout {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub max_label_width: u16,
    pub label_count: usize,
    pub has_extra_entry: bool,
    pub selector_mode_return_ax: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetHitState {
    pub hover_row: u8,
    pub selected_row: u8,
    pub presentation_state: u16,
    pub requested_presentation_state: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetHitResult {
    pub inside: bool,
    pub activated: bool,
    pub hover_row: u8,
    pub selected_row: u8,
    pub return_ax: u16,
    pub play_select_sound: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dTargetTextSegment {
    TargetList,
    GameData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawCommand {
    pub row_index: usize,
    pub string_segment: Ship3dTargetTextSegment,
    pub string_offset: u16,
    pub x: u16,
    pub y: u16,
    pub color: u8,
    pub measured_width: u16,
    pub extra_entry: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawResult {
    pub commands: Vec<Ship3dTargetDrawCommand>,
    pub final_hover_counter: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceState {
    pub selected_choice: u16,
    pub hud_flags: u8,
    pub handler_phase: u8,
    pub requested_presentation_state: u16,
    pub hold_ticks: u16,
    pub target_y: u16,
    pub target_layout_preserve_widths: bool,
    pub target_layout_center_x: u16,
    pub target_layout_extra_entry: bool,
    pub interpolation_duration_ticks: u8,
    pub interpolation_current_tick: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceGates {
    pub c2_presentation_gate: bool,
    pub left_motion_gate: bool,
    pub right_motion_gate: bool,
    pub menu_gate: bool,
    pub sound_gate: bool,
    pub presentation_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavChoiceInput {
    pub gate_value: u16,
    pub dynamic_axis: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
    pub activate: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceResult {
    pub gated: bool,
    pub reset_palette_range: bool,
    pub hovered_choice: Option<u8>,
    pub highlighted_palette_index: Option<u8>,
    pub committed_choice: Option<u8>,
    pub dispatched_choice: Option<u8>,
    pub play_select_sound: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceHandlerEffect {
    pub deferred_record_type: Option<u16>,
    pub deferred_record_related: Option<u16>,
    pub cleared_handler_phase: bool,
    pub ran_layout_prepass: bool,
    pub adjusted_target_records: bool,
    pub phase_gate_blocked: bool,
    pub cleared_selected_choice: bool,
    pub cleared_hud_target_list_flag: bool,
    pub load_snd_bank_path: Option<u16>,
    pub reset_interpolation_tick: bool,
    pub rebuilt_target_records: bool,
    pub set_input_gate_b: bool,
}

pub fn update_ship_3d_transition_state(state: &mut Ship3dTransitionState, random_gate_zero: bool) {
    if !state.transition_armed {
        if state.hold_ticks > SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD {
            state.depth_step = SHIP_3D_TRANSITION_OPEN_STEP;
            state.opening = true;
            state.transition_armed = true;
        }
        return;
    }

    if state.hold_ticks == 0 {
        start_closing_transition(state);
        return;
    }

    if !state.opening && random_gate_zero {
        start_closing_transition(state);
    }
}

pub fn step_ship_3d_depth_scroll(state: &mut Ship3dDepthState) {
    if state.opening {
        if state.depth_offset == SHIP_3D_MAX_DEPTH_OFFSET {
            state.opening = false;
            return;
        }

        let next = add_to_low_byte(state.depth_offset, state.depth_step);
        state.depth_offset = if (next as i16) < SHIP_3D_MAX_DEPTH_OFFSET as i16 {
            next
        } else {
            SHIP_3D_MAX_DEPTH_OFFSET
        };
        return;
    }

    if !state.closing {
        return;
    }

    if state.depth_offset == 0 {
        state.closing = false;
        return;
    }

    let next_low = (state.depth_offset as u8).wrapping_sub(state.depth_step);
    state.depth_offset = if next_low & 0x80 == 0 {
        (state.depth_offset & 0xff00) | next_low as u16
    } else {
        0
    };
}

pub fn copy_ship_3d_plane_bands(
    dest: &mut [u8],
    video_segment: &[u8],
    depth_offset: u16,
    plane_copy_enabled: bool,
    scroll_mode: u16,
) -> Option<Ship3dPlaneBandCopy> {
    if !plane_copy_enabled {
        return None;
    }

    let byte_count = ship_3d_plane_band_byte_count(depth_offset);
    if byte_count > SHIP_3D_PLANE_PAGE_BYTES {
        return None;
    }

    let first_source_start =
        SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + (SHIP_3D_PLANE_PAGE_BYTES - byte_count);
    let first_source_end = first_source_start.checked_add(byte_count)?;
    let second_source_start = SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET;
    let second_source_end = second_source_start.checked_add(byte_count)?;
    let second_dest_start = SHIP_3D_PLANE_DEST_BYTES.checked_sub(byte_count)?;
    let second_dest_end = second_dest_start.checked_add(byte_count)?;

    let first_source = video_segment.get(first_source_start..first_source_end)?;
    let second_source = video_segment.get(second_source_start..second_source_end)?;
    dest.get_mut(0..byte_count)?.copy_from_slice(first_source);
    dest.get_mut(second_dest_start..second_dest_end)?
        .copy_from_slice(second_source);

    Some(Ship3dPlaneBandCopy {
        row_count: byte_count / SHIP_3D_PLANE_ROW_BYTES,
        byte_count,
        first_source_start,
        first_dest_start: 0,
        second_source_start,
        second_dest_start,
        new_scroll_value: (scroll_mode != SHIP_3D_SCROLL_MODE_HOLD)
            .then(|| ship_3d_scroll_value(depth_offset)),
    })
}

pub fn step_ship_3d_interpolation_gate(
    gate: &mut Ship3dInterpolationGate,
    source: [u16; SHIP_3D_INTERPOLATION_WORDS],
    dest: [u16; SHIP_3D_INTERPOLATION_WORDS],
) -> Option<Ship3dInterpolationStep> {
    if gate.duration_ticks == gate.current_tick {
        return Some(Ship3dInterpolationStep::Complete);
    }

    if gate.duration_ticks == 0 {
        return None;
    }

    gate.current_tick = gate.current_tick.wrapping_add(1);
    let mut interpolated = [0u16; SHIP_3D_INTERPOLATION_WORDS];
    for index in 0..SHIP_3D_INTERPOLATION_WORDS {
        let delta = source[index].wrapping_sub(dest[index]) as i16;
        let quotient = checked_i16_div_i8_to_i8(delta, gate.duration_ticks as i8)?;
        let step = (quotient as i16).wrapping_mul(gate.current_tick as i8 as i16);
        interpolated[index] = dest[index].wrapping_add(step as u16);
    }
    Some(Ship3dInterpolationStep::Active(interpolated))
}

pub fn layout_ship_3d_target_list(
    measured_label_widths: &[u16],
    center_x: u16,
    has_extra_entry: bool,
) -> Ship3dTargetListLayout {
    let mut max_label_width = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH
    } else {
        SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH
    };
    let mut height_accumulator = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT
    } else {
        0
    };

    for width in measured_label_widths {
        if *width >= max_label_width {
            max_label_width = *width;
        }
        height_accumulator = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    let width = max_label_width.wrapping_add(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let height = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING);
    let x = center_x.wrapping_sub(width >> 1);
    let y = SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT.wrapping_sub(height) >> 1;

    Ship3dTargetListLayout {
        x,
        y,
        width,
        height,
        max_label_width,
        label_count: measured_label_widths.len(),
        has_extra_entry,
        selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
    }
}

pub fn hit_test_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    mouse_x: u16,
    mouse_y: u16,
    activate: bool,
) -> Option<Ship3dTargetHitResult> {
    state.hover_row = 0;
    state.selected_row = 0;
    let mut inside = false;
    let mut activated = false;
    let mut play_select_sound = false;

    if signed_i16(mouse_x) >= signed_i16(layout.x) {
        let right = layout.x.wrapping_add(layout.width);
        if signed_i16(mouse_x) <= signed_i16(right) {
            let row_origin = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
            let row_offset = mouse_y.wrapping_sub(row_origin);
            if signed_i16(row_offset) >= 0 {
                let hit_height = layout
                    .height
                    .wrapping_sub(SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET);
                if signed_i16(row_offset) < signed_i16(hit_height) {
                    let row =
                        checked_u16_div_u8_to_u8(row_offset, SHIP_3D_TARGET_LAYOUT_ROW_STEP as u8)?
                            .wrapping_add(1);
                    state.hover_row = row;
                    inside = true;

                    if state.presentation_state != SHIP_3D_TARGET_HOVER_PRESENTATION_MODE {
                        state.presentation_state = 0;
                        state.requested_presentation_state = SHIP_3D_TARGET_HOVER_PRESENTATION_MODE;
                    }

                    if activate {
                        state.requested_presentation_state =
                            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE;
                        state.selected_row = row;
                        activated = true;
                        play_select_sound = true;
                    }
                }
            }
        }
    }

    if !inside && state.presentation_state != SHIP_3D_TARGET_IDLE_PRESENTATION_MODE {
        state.presentation_state = 0;
        state.requested_presentation_state = SHIP_3D_TARGET_IDLE_PRESENTATION_MODE;
    }

    let return_ax = (state.selected_row as u8).wrapping_sub(1) as i8 as i16 as u16;
    Some(Ship3dTargetHitResult {
        inside,
        activated,
        hover_row: state.hover_row,
        selected_row: state.selected_row,
        return_ax,
        play_select_sound,
    })
}

pub fn update_ship_3d_nav_choice_dispatch(
    state: &mut Ship3dNavChoiceState,
    gates: Ship3dNavChoiceGates,
    input: Ship3dNavChoiceInput,
) -> Option<Ship3dNavChoiceResult> {
    let mut result = Ship3dNavChoiceResult::default();
    if gates.blocks_nav_choice() {
        result.gated = true;
        return Some(result);
    }

    if state.selected_choice == 0 {
        if input.gate_value > SHIP_3D_NAV_CHOICE_MAX_GATE
            || input.gate_value < SHIP_3D_NAV_CHOICE_MIN_GATE
        {
            return Some(result);
        }

        result.reset_palette_range = true;
        if let Some(choice_index) =
            hit_test_ship_3d_nav_choice(input.dynamic_axis, input.mouse_x, input.mouse_y)?
        {
            let choice = choice_index.wrapping_add(1);
            result.hovered_choice = Some(choice);
            result.highlighted_palette_index =
                Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST.wrapping_add(choice_index));

            if input.activate {
                state.requested_presentation_state = SHIP_3D_NAV_CHOICE_PRESENTATION_MODE;
                state.selected_choice = choice as u16;
                state.hud_flags |= SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS;
                state.hold_ticks = SHIP_3D_NAV_CHOICE_HOLD_TICKS;
                state.handler_phase = SHIP_3D_NAV_CHOICE_HANDLER_PHASE;
                state.target_y = SHIP_3D_NAV_CHOICE_TARGET_Y_BASE.wrapping_add(
                    (choice as u16 - 1).wrapping_mul(SHIP_3D_NAV_CHOICE_TARGET_Y_STEP),
                );
                state.target_layout_preserve_widths = true;
                state.target_layout_center_x = SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X;
                state.target_layout_extra_entry = true;
                state.interpolation_duration_ticks = SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION;
                result.committed_choice = Some(choice);
                result.play_select_sound = Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND);
            }
        }
    }

    if state.selected_choice != 0 && state.hud_flags & SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG == 0 {
        let choice = u8::try_from(state.selected_choice).ok()?;
        if choice == 0 || choice > SHIP_3D_NAV_CHOICE_COUNT {
            return None;
        }
        result.dispatched_choice = Some(choice);
    }

    Some(result)
}

pub fn run_ship_3d_nav_choice_handler_0(
    state: &mut Ship3dNavChoiceState,
    named_honk_object: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(named_honk_object),
        cleared_handler_phase: true,
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

pub fn run_ship_3d_nav_choice_handler_1(
    state: &mut Ship3dNavChoiceState,
    target_records: &mut [u16],
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        state.interpolation_current_tick = 0;
        adjust_nav_choice_target_records(target_records);
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.adjusted_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection_ax == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection_ax);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_type = Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE);
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.load_snd_bank_path = Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET);
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

pub fn run_ship_3d_nav_choice_handler_2(
    state: &mut Ship3dNavChoiceState,
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
    interpolation_complete: bool,
    query_selection_ax: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        rebuild_nav_choice_special_target_records(special_slots, target_records)?;
        state.interpolation_current_tick = 0;
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.rebuilt_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection_ax == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection_ax);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.set_input_gate_b = true;
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

pub fn run_ship_3d_nav_choice_handler_3(
    state: &mut Ship3dNavChoiceState,
    related_record: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(related_record),
        cleared_handler_phase: true,
        load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

pub fn draw_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    label_offsets: &[u16],
    width_table: &[u16],
    activate: bool,
    alias_source_offset: Option<u16>,
) -> Option<Ship3dTargetDrawResult> {
    let inner_width = layout
        .width
        .wrapping_sub(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let x_origin = layout.x.wrapping_add(SHIP_3D_TARGET_DRAW_X_INSET);
    let mut y = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
    let mut commands = Vec::new();

    for (row_index, label_offset) in label_offsets.iter().copied().enumerate() {
        if label_offset == 0 || label_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::TargetList,
            string_offset: if Some(label_offset) == alias_source_offset {
                SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
            } else {
                label_offset
            },
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: false,
        });
        y = y.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    if layout.has_extra_entry {
        let row_index = commands.len();
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::GameData,
            string_offset: SHIP_3D_TARGET_EXTRA_LABEL_OFFSET,
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: true,
        });
    }

    Some(Ship3dTargetDrawResult {
        commands,
        final_hover_counter: state.hover_row,
    })
}

pub fn select_ship_3d_target_record(
    state: &mut Ship3dTargetSelectorState,
    primary_targets: &[u16],
    fallback_targets: &[u16],
    query_index_ax: u16,
    phase_gate_complete: bool,
) -> Option<Ship3dTargetSelection> {
    state.target_fallback = false;
    let mut targets = primary_targets;
    if primary_targets.first().copied() == Some(SHIP_3D_TARGET_EXIT_SENTINEL) {
        targets = fallback_targets;
        state.target_fallback = true;
    }
    let used_fallback_table = state.target_fallback;

    let mut ran_layout_prepass = false;
    if state.target_select_phase & 1 != 0 {
        ran_layout_prepass = true;
        state.target_animation_tick = 0;
        state.target_select_phase = state.target_select_phase.wrapping_add(1);
    }

    if state.target_select_phase & 2 != 0 {
        if !phase_gate_complete {
            return Some(Ship3dTargetSelection {
                ax: 0,
                used_fallback_table,
                ran_layout_prepass,
                phase_gate_blocked: true,
            });
        }
        state.target_select_phase = 0;
    }

    if query_index_ax == SHIP_3D_TARGET_EXIT_SENTINEL {
        return Some(Ship3dTargetSelection {
            ax: 0,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let selected = targets.get(query_index_ax as usize).copied()?;
    if selected == SHIP_3D_TARGET_EXIT_SENTINEL {
        state.opening = true;
        state.depth_step = SHIP_3D_TARGET_OPEN_STEP;
        return Some(Ship3dTargetSelection {
            ax: SHIP_3D_TARGET_EXIT_SENTINEL,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let ax = if state.target_fallback {
        state.current_target
    } else {
        selected.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES)
    };
    Some(Ship3dTargetSelection {
        ax,
        used_fallback_table,
        ran_layout_prepass,
        phase_gate_blocked: false,
    })
}

pub fn ship_3d_plane_band_byte_count(depth_offset: u16) -> usize {
    let rows = (depth_offset as u8).wrapping_add(SHIP_3D_PLANE_BASE_ROWS as u8) as usize;
    rows * SHIP_3D_PLANE_ROW_BYTES
}

pub fn ship_3d_scroll_value(depth_offset: u16) -> u16 {
    let doubled = depth_offset.wrapping_mul(2);
    let capped = if (doubled as i16) > 0x64 {
        0x64
    } else {
        doubled
    };
    0x64u16.wrapping_sub(capped)
}

fn start_closing_transition(state: &mut Ship3dTransitionState) {
    state.depth_step = SHIP_3D_TRANSITION_CLOSE_STEP;
    state.closing = true;
    state.transition_armed = false;
}

fn add_to_low_byte(value: u16, addend: u8) -> u16 {
    (value & 0xff00) | value.to_le_bytes()[0].wrapping_add(addend) as u16
}

fn checked_i16_div_i8_to_i8(dividend: i16, divisor: i8) -> Option<i8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as i16;
    i8::try_from(quotient).ok()
}

fn checked_u16_div_u8_to_u8(dividend: u16, divisor: u8) -> Option<u8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as u16;
    u8::try_from(quotient).ok()
}

fn signed_i16(value: u16) -> i16 {
    value as i16
}

fn target_list_draw_x(x_origin: u16, inner_width: u16, measured_width: u16) -> u16 {
    x_origin.wrapping_add(inner_width.wrapping_sub(measured_width) >> 1)
}

impl Ship3dNavChoiceGates {
    fn blocks_nav_choice(self) -> bool {
        self.c2_presentation_gate
            || self.left_motion_gate
            || self.right_motion_gate
            || self.menu_gate
            || self.sound_gate
            || self.presentation_active
    }
}

fn hit_test_ship_3d_nav_choice(
    dynamic_axis: u16,
    mouse_x: u16,
    mouse_y: u16,
) -> Option<Option<u8>> {
    let relative_axis = dynamic_axis.wrapping_sub(SHIP_3D_NAV_CHOICE_AXIS_BIAS);
    let right = SHIP_3D_NAV_CHOICE_RIGHT_BASE.wrapping_sub(relative_axis.wrapping_shl(3));
    if signed_i16(mouse_x) > signed_i16(right) {
        return Some(None);
    }

    let left = right.wrapping_sub(SHIP_3D_NAV_CHOICE_X_WIDTH);
    if signed_i16(left) < 0 || signed_i16(mouse_x) < signed_i16(left) {
        return Some(None);
    }

    let abs_axis = if signed_i16(relative_axis) < 0 {
        0u16.wrapping_sub(relative_axis)
    } else {
        relative_axis
    };
    let quarter_axis = abs_axis >> 2;
    let y_origin = SHIP_3D_NAV_CHOICE_Y_BASE
        .wrapping_add(abs_axis)
        .wrapping_add(quarter_axis);
    let row_height = SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE.wrapping_sub((quarter_axis as u8) >> 1);
    let row_offset = mouse_y.wrapping_sub(y_origin);
    if signed_i16(row_offset) < 0 {
        return Some(None);
    }

    let choice = checked_u16_div_u8_to_u8(row_offset, row_height)?;
    if choice >= SHIP_3D_NAV_CHOICE_COUNT {
        return Some(None);
    }
    Some(Some(choice))
}

fn adjust_nav_choice_target_records(target_records: &mut [u16]) {
    for target_record in target_records {
        if *target_record == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        *target_record = target_record.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES);
    }
}

fn rebuild_nav_choice_special_target_records(
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
) -> Option<()> {
    target_records.clear();
    for special_slot in special_slots {
        if *special_slot == 0 {
            continue;
        }
        if *special_slot == SHIP_3D_TARGET_EXIT_SENTINEL {
            target_records.push(SHIP_3D_TARGET_EXIT_SENTINEL);
            return Some(());
        }
        target_records.push(special_slot.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
    }
    None
}

fn next_target_list_draw_color(state: &mut Ship3dTargetHitState, activate: bool) -> u8 {
    state.hover_row = state.hover_row.wrapping_sub(1);
    if state.hover_row == 0 {
        if activate {
            SHIP_3D_TARGET_ACTIVE_TEXT_COLOR
        } else {
            SHIP_3D_TARGET_HOVER_TEXT_COLOR
        }
    } else {
        SHIP_3D_TARGET_DEFAULT_TEXT_COLOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_segment() -> Vec<u8> {
        (0..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES)
            .map(|idx| (idx & 0xff) as u8)
            .collect()
    }

    #[test]
    fn transition_state_starts_opening_after_hold_threshold() {
        let mut state = Ship3dTransitionState {
            hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut state, true);
        assert_eq!(
            state,
            Ship3dTransitionState {
                hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
                ..Ship3dTransitionState::default()
            }
        );

        state.hold_ticks = SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD + 1;
        update_ship_3d_transition_state(&mut state, false);
        assert_eq!(state.depth_step, SHIP_3D_TRANSITION_OPEN_STEP);
        assert!(state.opening);
        assert!(state.transition_armed);
        assert!(!state.closing);
    }

    #[test]
    fn transition_state_starts_closing_when_armed_timer_expires_or_random_gate_hits() {
        let mut expired = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 0,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut expired, false);
        assert_eq!(expired.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(expired.closing);
        assert!(!expired.transition_armed);

        let mut gated = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 1,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut gated, false);
        assert_eq!(
            gated,
            Ship3dTransitionState {
                transition_armed: true,
                hold_ticks: 1,
                ..Ship3dTransitionState::default()
            }
        );
        update_ship_3d_transition_state(&mut gated, true);
        assert_eq!(gated.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(gated.closing);
        assert!(!gated.transition_armed);
    }

    #[test]
    fn depth_scroll_opens_to_max_then_clears_opening_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 0x3c,
            opening: true,
            depth_step: SHIP_3D_TRANSITION_OPEN_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0x40);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(!state.opening);
    }

    #[test]
    fn depth_scroll_closes_to_zero_then_clears_closing_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 5,
            closing: true,
            depth_step: SHIP_3D_TRANSITION_CLOSE_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(state.closing);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(!state.closing);
    }

    #[test]
    fn depth_scroll_uses_8086_low_byte_arithmetic() {
        let mut closing = Ship3dDepthState {
            depth_offset: 0x0101,
            closing: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut closing);
        assert_eq!(closing.depth_offset, 0x0100);

        let mut opening = Ship3dDepthState {
            depth_offset: 0xff00,
            opening: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut opening);
        assert_eq!(opening.depth_offset, 0xff01);

        assert_eq!(
            ship_3d_plane_band_byte_count(0x0100),
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
    }

    #[test]
    fn plane_band_copy_uses_depth_plus_35_planar_rows() {
        let source = source_segment();
        let mut dest = vec![0xee; SHIP_3D_PLANE_DEST_BYTES];
        let copied =
            copy_ship_3d_plane_bands(&mut dest, &source, 0, true, 0).expect("ship 3D plane copy");

        assert_eq!(copied.row_count, SHIP_3D_PLANE_BASE_ROWS);
        assert_eq!(
            copied.byte_count,
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
        assert_eq!(
            copied.first_source_start,
            SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES - copied.byte_count
        );
        assert_eq!(
            &dest[0..copied.byte_count],
            &source[copied.first_source_start..copied.first_source_start + copied.byte_count]
        );
        assert_eq!(
            &dest[copied.second_dest_start..copied.second_dest_start + copied.byte_count],
            &source[copied.second_source_start..copied.second_source_start + copied.byte_count]
        );
        assert!(dest[copied.byte_count..copied.second_dest_start]
            .iter()
            .all(|value| *value == 0xee));
        assert_eq!(copied.new_scroll_value, Some(0x64));
    }

    #[test]
    fn plane_band_copy_at_max_depth_copies_two_full_planar_pages() {
        let source = source_segment();
        let mut dest = vec![0; SHIP_3D_PLANE_DEST_BYTES];
        let copied = copy_ship_3d_plane_bands(
            &mut dest,
            &source,
            SHIP_3D_MAX_DEPTH_OFFSET,
            true,
            SHIP_3D_SCROLL_MODE_HOLD,
        )
        .expect("ship 3D plane copy");

        assert_eq!(copied.row_count, 100);
        assert_eq!(copied.byte_count, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(copied.first_source_start, SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET);
        assert_eq!(copied.second_dest_start, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(
            &dest[0..SHIP_3D_PLANE_PAGE_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(
            &dest[SHIP_3D_PLANE_PAGE_BYTES..SHIP_3D_PLANE_DEST_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(copied.new_scroll_value, None);
    }

    #[test]
    fn plane_band_copy_reports_scroll_value_like_binary_math() {
        assert_eq!(ship_3d_scroll_value(0), 0x64);
        assert_eq!(ship_3d_scroll_value(30), 40);
        assert_eq!(ship_3d_scroll_value(50), 0);
        assert_eq!(ship_3d_scroll_value(SHIP_3D_MAX_DEPTH_OFFSET), 0);
        assert_eq!(copy_ship_3d_plane_bands(&mut [], &[], 0, false, 0), None);
    }

    #[test]
    fn target_selector_runs_phase_prepass_and_blocks_while_gate_is_active() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 1,
            target_animation_tick: 7,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(&mut state, &[0x1200], &[], 0, false).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0,
                used_fallback_table: false,
                ran_layout_prepass: true,
                phase_gate_blocked: true,
            }
        );
        assert_eq!(state.target_select_phase, 2);
        assert_eq!(state.target_animation_tick, 0);
    }

    #[test]
    fn target_selector_returns_primary_target_after_phase_gate_completes() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 2,
            ..Ship3dTargetSelectorState::default()
        };

        let selected =
            select_ship_3d_target_record(&mut state, &[0x1200, 0x2345], &[], 1, true).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0x2341,
                used_fallback_table: false,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert_eq!(state.target_select_phase, 0);
        assert!(!state.opening);
    }

    #[test]
    fn target_selector_fallback_table_returns_current_target() {
        let mut state = Ship3dTargetSelectorState {
            current_target: 0x4567,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(
            &mut state,
            &[SHIP_3D_TARGET_EXIT_SENTINEL],
            &[0x2222],
            0,
            true,
        )
        .unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                ax: 0x4567,
                used_fallback_table: true,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert!(state.target_fallback);
    }

    #[test]
    fn target_selector_exit_sentinel_arms_opening_transition() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[],
            1,
            true,
        )
        .unwrap();

        assert_eq!(selected.ax, SHIP_3D_TARGET_EXIT_SENTINEL);
        assert!(state.opening);
        assert_eq!(state.depth_step, SHIP_3D_TARGET_OPEN_STEP);
    }

    #[test]
    fn target_selector_no_query_selection_returns_zero_ax() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200],
            &[],
            SHIP_3D_TARGET_EXIT_SENTINEL,
            true,
        )
        .unwrap();

        assert_eq!(selected.ax, 0);
        assert!(!state.opening);
    }

    #[test]
    fn interpolation_gate_reports_complete_without_advancing_at_duration() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 6,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [10, 20, 30, 40], [0, 0, 0, 0]);

        assert_eq!(step, Some(Ship3dInterpolationStep::Complete));
        assert_eq!(gate.current_tick, 6);
    }

    #[test]
    fn interpolation_gate_increments_tick_and_interpolates_four_words() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 1,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [60, 66, 72, 78], [0, 6, 12, 18]);

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([20, 26, 32, 38]))
        );
        assert_eq!(gate.current_tick, 2);
    }

    #[test]
    fn interpolation_gate_uses_signed_truncating_division() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 2,
        };

        let step = step_ship_3d_interpolation_gate(
            &mut gate,
            [0xfff0, 0x0000, 0x0031, 0x0000],
            [0, 31, 0, 0],
        );

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([
                0xfffa, // (-16 / 6) * 3 = -6, added to 0.
                0x0010, // (-31 / 6) * 3 = -15, added to 31.
                0x0018, // (49 / 6) * 3 = 24.
                0,
            ]))
        );
        assert_eq!(gate.current_tick, 3);
    }

    #[test]
    fn interpolation_gate_rejects_binary_idiv_error_shapes() {
        let mut zero_duration = Ship3dInterpolationGate {
            duration_ticks: 0,
            current_tick: 1,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(&mut zero_duration, [1, 0, 0, 0], [0, 0, 0, 0]),
            None
        );

        let mut quotient_overflow = Ship3dInterpolationGate {
            duration_ticks: 1,
            current_tick: 0,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(
                &mut quotient_overflow,
                [0x0100, 0, 0, 0],
                [0, 0, 0, 0]
            ),
            None
        );
    }

    #[test]
    fn target_list_layout_uses_binary_default_width_floor_and_centering() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 20,
                y: 85,
                width: 120,
                height: 30,
                max_label_width: SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH,
                label_count: 2,
                has_extra_entry: false,
                selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_grows_to_widest_label() {
        let layout = layout_ship_3d_target_list(&[120, 50], 0x50, false);

        assert_eq!(layout.max_label_width, 120);
        assert_eq!(layout.width, 140);
        assert_eq!(layout.x, 10);
        assert_eq!(layout.height, 30);
        assert_eq!(layout.y, 85);
    }

    #[test]
    fn target_list_layout_extra_entry_uses_shorter_width_and_height_seed() {
        let layout = layout_ship_3d_target_list(&[], 0x50, true);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 43,
                y: 91,
                width: 75,
                height: 18,
                max_label_width: SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH,
                label_count: 0,
                has_extra_entry: true,
                selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_preserves_binary_wrapping_for_tall_lists() {
        let widths = vec![1; 20];
        let layout = layout_ship_3d_target_list(&widths, 0x50, false);

        assert_eq!(layout.height, 228);
        assert_eq!(layout.y, 0x7ff2);
    }

    #[test]
    fn target_hit_test_commits_selection_only_when_active() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let hover = hit_test_ship_3d_target_list(&mut state, layout, 30, 90, false).unwrap();
        assert_eq!(
            hover,
            Ship3dTargetHitResult {
                inside: true,
                activated: false,
                hover_row: 1,
                selected_row: 0,
                return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 1);
        assert_eq!(state.selected_row, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_HOVER_PRESENTATION_MODE
        );

        let active = hit_test_ship_3d_target_list(&mut state, layout, 30, 101, true).unwrap();
        assert_eq!(
            active,
            Ship3dTargetHitResult {
                inside: true,
                activated: true,
                hover_row: 2,
                selected_row: 2,
                return_ax: 1,
                play_select_sound: true,
            }
        );
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_uses_inclusive_x_and_exclusive_bottom_y() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET - 1,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width + 1,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
    }

    #[test]
    fn target_hit_test_clears_selection_then_requests_idle_when_outside() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            selected_row: 2,
            presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            requested_presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            ..Ship3dTargetHitState::default()
        };

        let result = hit_test_ship_3d_target_list(&mut state, layout, 0, 0, false).unwrap();

        assert_eq!(
            result,
            Ship3dTargetHitResult {
                inside: false,
                activated: false,
                hover_row: 0,
                selected_row: 0,
                return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 0);
        assert_eq!(state.selected_row, 0);
        assert_eq!(state.presentation_state, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_IDLE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_rejects_binary_div_overflow_shape() {
        let layout = Ship3dTargetListLayout {
            x: 0,
            y: 0,
            width: 1,
            height: 0x0b09,
            max_label_width: 1,
            label_count: 256,
            has_extra_entry: false,
            selector_mode_return_ax: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        };
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            hit_test_ship_3d_target_list(&mut state, layout, 0, 0x0b04, false),
            None
        );
    }

    #[test]
    fn target_draw_centers_rows_from_width_table_and_highlights_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[20, 80],
            false,
            None,
        )
        .unwrap();

        assert_eq!(
            drawn.commands,
            vec![
                Ship3dTargetDrawCommand {
                    row_index: 0,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x1000,
                    x: 70,
                    y: 89,
                    color: SHIP_3D_TARGET_DEFAULT_TEXT_COLOR,
                    measured_width: 20,
                    extra_entry: false,
                },
                Ship3dTargetDrawCommand {
                    row_index: 1,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x2000,
                    x: 40,
                    y: 100,
                    color: SHIP_3D_TARGET_HOVER_TEXT_COLOR,
                    measured_width: 80,
                    extra_entry: false,
                },
            ]
        );
        assert_eq!(drawn.final_hover_counter, 0);
        assert_eq!(state.hover_row, 0);
    }

    #[test]
    fn target_draw_uses_active_color_and_keeps_decrementing_after_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80, 40], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 1,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, 0x3000],
            &[20, 80, 40],
            true,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands[0].color, SHIP_3D_TARGET_ACTIVE_TEXT_COLOR);
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.commands[2].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.final_hover_counter, 0xfe);
    }

    #[test]
    fn target_draw_stops_at_sentinel_then_draws_cancel_extra_entry() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, true);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000],
            &[20, SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH],
            false,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands.len(), 2);
        assert_eq!(drawn.commands[0].string_offset, 0x1000);
        assert_eq!(drawn.commands[1].row_index, 1);
        assert_eq!(
            drawn.commands[1].string_segment,
            Ship3dTargetTextSegment::GameData
        );
        assert_eq!(
            drawn.commands[1].string_offset,
            SHIP_3D_TARGET_EXTRA_LABEL_OFFSET
        );
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_HOVER_TEXT_COLOR);
        assert!(drawn.commands[1].extra_entry);
    }

    #[test]
    fn target_draw_applies_alias_blank_label_offset() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let drawn =
            draw_ship_3d_target_list(&mut state, layout, &[0x4444], &[20], false, Some(0x4444))
                .unwrap();

        assert_eq!(
            drawn.commands[0].string_offset,
            SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
        );
    }

    #[test]
    fn target_draw_requires_matching_width_table_entries() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            draw_ship_3d_target_list(&mut state, layout, &[0x1000, 0x2000], &[20], false, None),
            None
        );
    }

    #[test]
    fn nav_choice_hover_maps_mouse_to_palette_highlight() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 2,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: false,
                reset_palette_range: true,
                hovered_choice: Some(3),
                highlighted_palette_index: Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST + 2),
                committed_choice: None,
                dispatched_choice: None,
                play_select_sound: None,
            }
        );
        assert_eq!(state, Ship3dNavChoiceState::default());
    }

    #[test]
    fn nav_choice_activation_sets_binary_state_without_dispatching_yet() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MAX_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result.hovered_choice, Some(4));
        assert_eq!(result.committed_choice, Some(4));
        assert_eq!(result.dispatched_choice, None);
        assert_eq!(
            result.play_select_sound,
            Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND)
        );
        assert_eq!(state.selected_choice, 4);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_NAV_CHOICE_PRESENTATION_MODE
        );
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS);
        assert_eq!(state.hold_ticks, SHIP_3D_NAV_CHOICE_HOLD_TICKS);
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        assert_eq!(
            state.target_y,
            SHIP_3D_NAV_CHOICE_TARGET_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3
        );
        assert!(state.target_layout_preserve_widths);
        assert_eq!(
            state.target_layout_center_x,
            SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X
        );
        assert!(state.target_layout_extra_entry);
        assert_eq!(
            state.interpolation_duration_ticks,
            SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION
        );
    }

    #[test]
    fn nav_choice_existing_selection_dispatches_after_hud_bit_clears() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: 0,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: 0,
                dynamic_axis: 0,
                mouse_x: 0,
                mouse_y: 0,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(result.reset_palette_range, false);
        assert_eq!(result.hovered_choice, None);
        assert_eq!(result.dispatched_choice, Some(2));
    }

    #[test]
    fn nav_choice_gates_block_hit_test_and_dispatch() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates {
                presentation_active: true,
                ..Ship3dNavChoiceGates::default()
            },
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: true,
                ..Ship3dNavChoiceResult::default()
            }
        );
        assert_eq!(state.selected_choice, 2);
    }

    #[test]
    fn nav_choice_rejects_out_of_range_gate_before_palette_reset() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE - 1,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result, Ship3dNavChoiceResult::default());
        assert_eq!(state.selected_choice, 0);
    }

    #[test]
    fn nav_choice_uses_dynamic_axis_for_slanted_bounds() {
        let mut state = Ship3dNavChoiceState::default();

        let outside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0090,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(outside.reset_palette_range, true);
        assert_eq!(outside.hovered_choice, None);

        let inside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0091,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(inside.hovered_choice, Some(1));
    }

    #[test]
    fn nav_choice_handler_0_defers_honk_record_link_and_clears_phase() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_duration_ticks: SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6754),
                cleared_handler_phase: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
    }

    #[test]
    fn nav_choice_handler_0_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: 0x02,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, 0x02);
    }

    #[test]
    fn nav_choice_handler_1_adjusts_records_and_waits_for_interpolation() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                adjusted_target_records: true,
                phase_gate_blocked: true,
                reset_interpolation_tick: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_1_selects_target_after_interpolation() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x2000),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_exit_sentinel_clears_choice_without_deferred_record() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_no_selection_leaves_state_armed() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.selected_choice, 2);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
    }

    #[test]
    fn nav_choice_handler_2_rebuilds_targets_from_special_slots_and_waits() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 7,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa, 0xbbbb];

        let effect = run_ship_3d_nav_choice_handler_2(
            &mut state,
            &[0, 0x1200, 0, 0x3400, SHIP_3D_TARGET_EXIT_SENTINEL, 0x5600],
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                rebuilt_target_records: true,
                reset_interpolation_tick: true,
                phase_gate_blocked: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_2_selects_special_slot_target_and_sets_input_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_related: Some(0x3400),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                set_input_gate_b: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_exit_sentinel_clears_choice_without_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_requires_special_slot_sentinel_when_rebuilding() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa];

        assert_eq!(
            run_ship_3d_nav_choice_handler_2(
                &mut state,
                &[0, 0x1200],
                &mut target_records,
                false,
                SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            ),
            None
        );
        assert_eq!(target_records, vec![0x1204]);
    }

    #[test]
    fn nav_choice_handler_3_defers_static_record_link_and_reloads_radio_bank() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 4,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 8,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6756),
                cleared_handler_phase: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 4);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
        assert_eq!(state.interpolation_current_tick, 8);
    }

    #[test]
    fn nav_choice_handler_3_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
    }
}
