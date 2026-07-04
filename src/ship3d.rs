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
}
