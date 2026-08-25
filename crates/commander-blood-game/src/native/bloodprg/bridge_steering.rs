//! Bridge camera steering and seek motion over flat, typed runtime state.

use super::NavActorSeekState;

/// Number of authored panorama frames around the bridge.
pub const BRIDGE_VIEW_FRAME_COUNT: u16 = 180;
/// Number of angular units in one complete bridge rotation.
pub const BRIDGE_ARC_UNIT_COUNT: u16 = 360;
/// Number of high-resolution cursor units in one complete bridge rotation.
pub const BRIDGE_CURSOR_RING_UNIT_COUNT: u16 = 1_440;

const HALF_BRIDGE_VIEW_FRAME_COUNT: i16 = 90;
const HALF_BRIDGE_ARC_UNIT_COUNT: i16 = 180;
const SEEK_DRAG_THRESHOLD: i16 = 40;
const STEERING_DEAD_ZONE: i16 = 31;
const MENU_CLAMP_DISTANCE: i16 = 40;
const STEERING_TRAIL: u16 = 30;
const CURSOR_UNITS_PER_ARC_UNIT_SHIFT: u32 = 2;
const ARC_UNITS_PER_FRAME_SHIFT: u32 = 1;
const CURSOR_UNITS_PER_FRAME_SHIFT: u32 = 3;
const HALF_LOGICAL_SCREEN_WIDTH: u16 = 160;
const CURSOR_FRAME_ALIGNMENT_MASK: u16 = !7;
const MINIMUM_SEEK_STEP: u16 = 1;

/// Direction most recently selected by free steering or an active seek.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeTurnDirection {
    /// Move toward lower-numbered panorama frames.
    TowardDecreasingFrames,
    /// Move toward higher-numbered panorama frames.
    TowardIncreasingFrames,
}

/// Input policy affecting pointer-driven bridge steering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeSteeringInteraction {
    /// The player can freely rotate the bridge view.
    #[default]
    Free,
    /// An open bridge menu clamps the pointer to a narrow arc around the view.
    MenuEngaged,
}

/// Mutable state owned by the modern bridge steering system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSteeringState {
    /// Current authored panorama frame in the 180-frame ring.
    pub view_frame: u16,
    /// Current high-resolution horizontal pointer position around the bridge ring.
    pub cursor_ring_position: u16,
    /// Quarter-resolution pointer arc published to bridge actors.
    pub cursor_arc: u16,
    /// Pointer position retained while a long automatic seek drags the input ring.
    pub cursor_drag_reference: u16,
    /// Current mouse-button bit field; automatic motion clears it after a seek step.
    pub pointer_buttons: u16,
    /// Initial seek distance retained for the long-seek cursor-drag decision.
    pub seek_initial_distance: u16,
    /// Direction published by the latest steering change, if one has occurred.
    pub turn_direction: Option<BridgeTurnDirection>,
    /// Bridge heading consumed by the ship projection matrix.
    pub projection_heading: u16,
    /// Cursor-ring origin corresponding to the current panorama frame.
    pub frame_angle_bias: u16,
}

/// Observable result of one bridge steering update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSteeringOutcome {
    /// Whether the panorama frame or projection heading changed.
    pub view_changed: bool,
    /// Scene link selected from the current view or menu clamp.
    pub presentation_link: u16,
}

/// Advance automatic seek motion and pointer-driven bridge steering.
///
/// This translates `bridge_steer_update` at BLOODPRG routine offset `0x009656`.
/// Ring arithmetic and all game-visible state transitions are preserved. SDL
/// supplies pointer motion to `cursor_ring_position`; the obsolete host cursor
/// recenter calls have no equivalent because they did not affect game state.
pub fn update_bridge_steering(
    state: &mut BridgeSteeringState,
    seek: &mut NavActorSeekState,
    interaction: BridgeSteeringInteraction,
    current_presentation_link: u16,
) -> BridgeSteeringOutcome {
    let mut frame = state.view_frame;
    let mut cursor_ring = state.cursor_ring_position;
    let mut presentation_link = current_presentation_link;
    let mut view_changed = false;

    if seek.requested {
        let target_frame = seek.target_arc >> ARC_UNITS_PER_FRAME_SHIFT;
        if frame == target_frame {
            seek.requested = false;
            state.seek_initial_distance = u16::MIN;
        } else {
            let mut distance = signed_absolute_difference(frame, target_frame);
            if distance as i16 >= HALF_BRIDGE_VIEW_FRAME_COUNT {
                distance = BRIDGE_VIEW_FRAME_COUNT.wrapping_sub(distance);
            }

            let positive_frame =
                normalize_signed_once(frame.wrapping_add(distance), BRIDGE_VIEW_FRAME_COUNT);
            let positive_arc = positive_frame.wrapping_shl(ARC_UNITS_PER_FRAME_SHIFT);
            if state.seek_initial_distance == u16::MIN {
                state.seek_initial_distance = distance;
            }
            let mut step = distance >> ARC_UNITS_PER_FRAME_SHIFT;
            if step == u16::MIN {
                step = MINIMUM_SEEK_STEP;
            }
            let mut drag = distance.wrapping_shl(CURSOR_UNITS_PER_ARC_UNIT_SHIFT);
            state.turn_direction = Some(BridgeTurnDirection::TowardIncreasingFrames);
            if positive_arc != seek.target_arc {
                state.turn_direction = Some(BridgeTurnDirection::TowardDecreasingFrames);
                step = step.wrapping_neg();
                drag = drag.wrapping_neg();
            }

            if state.seek_initial_distance as i16 >= SEEK_DRAG_THRESHOLD {
                cursor_ring = cursor_ring.wrapping_add(drag);
                state.cursor_drag_reference = state.cursor_drag_reference.wrapping_add(drag);
            }
            frame = normalize_signed_once(frame.wrapping_add(step), BRIDGE_VIEW_FRAME_COUNT);
            state.pointer_buttons = u16::MIN;
        }
    }

    let view_arc = frame.wrapping_shl(ARC_UNITS_PER_FRAME_SHIFT);
    cursor_ring = normalize_signed_once(
        cursor_ring.wrapping_sub(BRIDGE_CURSOR_RING_UNIT_COUNT),
        BRIDGE_CURSOR_RING_UNIT_COUNT,
    );
    let mut cursor_arc = cursor_ring >> CURSOR_UNITS_PER_ARC_UNIT_SHIFT;
    state.cursor_arc = cursor_arc;

    if seek.requested {
        view_changed = true;
    } else {
        presentation_link = view_arc;
        let distance = signed_absolute_difference(view_arc, cursor_arc);
        let distance = if distance as i16 >= HALF_BRIDGE_ARC_UNIT_COUNT {
            BRIDGE_ARC_UNIT_COUNT.wrapping_sub(distance)
        } else {
            distance
        };

        if distance as i16 > STEERING_DEAD_ZONE {
            match interaction {
                BridgeSteeringInteraction::MenuEngaged
                    if distance as i16 >= MENU_CLAMP_DISTANCE =>
                {
                    let positive_arc = normalize_signed_once(
                        cursor_arc.wrapping_add(distance),
                        BRIDGE_ARC_UNIT_COUNT,
                    );
                    let clamped_arc = if positive_arc == view_arc {
                        view_arc.wrapping_sub(MENU_CLAMP_DISTANCE as u16)
                    } else {
                        view_arc.wrapping_add(MENU_CLAMP_DISTANCE as u16)
                    };
                    let clamped_arc = normalize_signed_once(clamped_arc, BRIDGE_ARC_UNIT_COUNT);
                    presentation_link = clamped_arc.wrapping_shl(CURSOR_UNITS_PER_ARC_UNIT_SHIFT);
                    cursor_ring = presentation_link;
                }
                BridgeSteeringInteraction::Free => {
                    let positive_arc = normalize_signed_once(
                        cursor_arc.wrapping_add(distance),
                        BRIDGE_ARC_UNIT_COUNT,
                    );
                    if positive_arc == view_arc {
                        state.turn_direction = Some(BridgeTurnDirection::TowardDecreasingFrames);
                        cursor_arc = cursor_arc.wrapping_add(STEERING_TRAIL);
                    } else {
                        state.turn_direction = Some(BridgeTurnDirection::TowardIncreasingFrames);
                        cursor_arc = cursor_arc.wrapping_sub(STEERING_TRAIL);
                    }
                    cursor_arc = normalize_signed_once(cursor_arc, BRIDGE_ARC_UNIT_COUNT);
                    frame = cursor_arc >> ARC_UNITS_PER_FRAME_SHIFT;
                    view_changed = true;
                }
                BridgeSteeringInteraction::MenuEngaged => {}
            }
        }
    }

    if view_changed {
        state.projection_heading = frame;
        state.frame_angle_bias = frame
            .wrapping_shl(CURSOR_UNITS_PER_FRAME_SHIFT)
            .wrapping_sub(HALF_LOGICAL_SCREEN_WIDTH);
        cursor_ring &= CURSOR_FRAME_ALIGNMENT_MASK;
    }

    state.view_frame = frame;
    state.cursor_ring_position = normalize_signed_once(
        cursor_ring.wrapping_sub(state.frame_angle_bias),
        BRIDGE_CURSOR_RING_UNIT_COUNT,
    );
    BridgeSteeringOutcome {
        view_changed,
        presentation_link,
    }
}

fn signed_absolute_difference(left: u16, right: u16) -> u16 {
    if (left as i16) > (right as i16) {
        left.wrapping_sub(right)
    } else {
        right.wrapping_sub(left)
    }
}

fn normalize_signed_once(value: u16, modulus: u16) -> u16 {
    if (value as i16).is_negative() {
        value.wrapping_add(modulus)
    } else if (value as i16) >= modulus as i16 {
        value.wrapping_sub(modulus)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 21;
    const NATIVE_MENU_ENGAGED_FLAG: u16 = 4;
    const NATIVE_SEEK_REQUESTED_FLAG: u16 = 8;
    const NATIVE_DECREASING_DIRECTION: u8 = 0;
    const NATIVE_INCREASING_DIRECTION: u8 = 1;

    #[derive(Deserialize)]
    struct SteeringOracle {
        name: String,
        ui_before: u16,
        ui_after: u16,
        frame_before: u16,
        frame_after: u16,
        mouse_before: u16,
        mouse_after: u16,
        mouse_arc_before: u16,
        mouse_arc_after: u16,
        mouse_drag_reference_before: u16,
        mouse_drag_reference_after: u16,
        mouse_buttons_before: u16,
        mouse_buttons_after: u16,
        seek_target: u16,
        seek_initial_before: u16,
        seek_initial_after: u16,
        direction_before: u8,
        direction_after: u8,
        projection_angle_before: u16,
        projection_angle_after: u16,
        frame_angle_bias_before: u16,
        frame_angle_bias_after: u16,
        presentation_context_before: u16,
        presentation_context_after: u16,
        view_changed: bool,
    }

    #[test]
    fn steering_matches_every_original_state_vector_without_pointer_warping() {
        let vectors: Vec<SteeringOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9656_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut state = BridgeSteeringState {
                view_frame: vector.frame_before,
                cursor_ring_position: vector.mouse_before,
                cursor_arc: vector.mouse_arc_before,
                cursor_drag_reference: vector.mouse_drag_reference_before,
                pointer_buttons: vector.mouse_buttons_before,
                seek_initial_distance: vector.seek_initial_before,
                turn_direction: direction(vector.direction_before),
                projection_heading: vector.projection_angle_before,
                frame_angle_bias: vector.frame_angle_bias_before,
            };
            let mut seek = NavActorSeekState {
                target_arc: vector.seek_target,
                requested: vector.ui_before & NATIVE_SEEK_REQUESTED_FLAG != u16::MIN,
            };
            let interaction = if vector.ui_before & NATIVE_MENU_ENGAGED_FLAG != u16::MIN {
                BridgeSteeringInteraction::MenuEngaged
            } else {
                BridgeSteeringInteraction::Free
            };

            let outcome = update_bridge_steering(
                &mut state,
                &mut seek,
                interaction,
                vector.presentation_context_before,
            );

            let seek_flag = (seek.requested as u16) * NATIVE_SEEK_REQUESTED_FLAG;
            let menu_flag = (matches!(interaction, BridgeSteeringInteraction::MenuEngaged) as u16)
                * NATIVE_MENU_ENGAGED_FLAG;
            let final_ui = seek_flag | menu_flag;
            assert_eq!(final_ui, vector.ui_after, "{}", vector.name);
            assert_eq!(state.view_frame, vector.frame_after, "{}", vector.name);
            assert_eq!(
                state.cursor_ring_position, vector.mouse_after,
                "{}",
                vector.name
            );
            assert_eq!(state.cursor_arc, vector.mouse_arc_after, "{}", vector.name);
            assert_eq!(
                state.cursor_drag_reference, vector.mouse_drag_reference_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.pointer_buttons, vector.mouse_buttons_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.seek_initial_distance, vector.seek_initial_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.turn_direction,
                direction(vector.direction_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.projection_heading, vector.projection_angle_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.frame_angle_bias, vector.frame_angle_bias_after,
                "{}",
                vector.name
            );
            assert_eq!(outcome.view_changed, vector.view_changed, "{}", vector.name);
            assert_eq!(
                outcome.presentation_link, vector.presentation_context_after,
                "{}",
                vector.name
            );
        }
    }

    fn direction(native: u8) -> Option<BridgeTurnDirection> {
        match native {
            NATIVE_DECREASING_DIRECTION => Some(BridgeTurnDirection::TowardDecreasingFrames),
            NATIVE_INCREASING_DIRECTION => Some(BridgeTurnDirection::TowardIncreasingFrames),
            _ => None,
        }
    }
}
