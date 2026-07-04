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
        assert!(
            dest[copied.byte_count..copied.second_dest_start]
                .iter()
                .all(|value| *value == 0xee)
        );
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
}
