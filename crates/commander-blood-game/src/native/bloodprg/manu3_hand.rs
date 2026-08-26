//! Main-loop coordination for the MANU3 three-dimensional hand.

use crate::native::manu3::animation::CursorPosition;
use crate::native::manu3::model::Manu3FrameRequest;

const PRESENTATION_DELAY_FRAME_COUNT: u8 = 2;
const DISABLED_SELECTOR_BOUNDARY: i16 = 0;

/// Mutable selector and delay state retained between hand frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manu3HandFrameState {
    /// One-shot animation selector requested by game logic.
    pub requested_animation: u16,
    /// Most recent nonzero selector accepted by the hand.
    pub current_animation: u16,
    /// Frames still suppressed after a presentation request.
    pub presentation_delay: u8,
}

/// Game state read while deciding whether to render the hand this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manu3HandFrameContext {
    /// A full-screen presentation currently owns the display.
    pub presentation_mode_active: bool,
    /// The main loop is refreshing HUD state instead of the hand.
    pub hud_refresh_active: bool,
    /// Ship-scene handling allows a pending presentation to render immediately.
    pub ship_scene_dispatch_blocked: bool,
    /// A presentation request requires two clear frames before hand rendering.
    pub presentation_request_pending: bool,
    /// Current pointer position in the original 320 by 200 coordinate space.
    pub cursor: CursorPosition,
}

/// Advance hand selector state and produce the next MANU3 frame request.
///
/// This translates `manu3_hand_frame_dispatch` at BLOODPRG file offset
/// `0x001610`. It retains the signed disabled-selector gate, repeated-selector
/// clearing, nonzero selector latch, presentation delay arm and countdown, and
/// original cursor coordinates. The unreachable mouse-button block is omitted,
/// and wgpu render-target ownership replaces the native VGA page offset.
pub fn update_manu3_hand_frame(
    state: &mut Manu3HandFrameState,
    context: Manu3HandFrameContext,
) -> Option<Manu3FrameRequest> {
    if context.presentation_mode_active || context.hud_refresh_active {
        return None;
    }

    let mut selector = state.requested_animation;
    if (selector as i16) < DISABLED_SELECTOR_BOUNDARY {
        return None;
    }

    if selector == state.current_animation {
        selector = u16::MIN;
        state.requested_animation = u16::MIN;
    } else if selector != u16::MIN {
        state.current_animation = selector;
    }

    if !context.ship_scene_dispatch_blocked && context.presentation_request_pending {
        state.presentation_delay = PRESENTATION_DELAY_FRAME_COUNT;
        return None;
    }
    if state.presentation_delay != u8::MIN {
        state.presentation_delay -= 1;
        return None;
    }

    Some(Manu3FrameRequest {
        cursor: context.cursor,
        animation_selector: selector,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 11;
    const NATIVE_REQUEST_BYTE_COUNT: usize = 8;
    const CURSOR_X_OFFSET: usize = 0;
    const CURSOR_Y_OFFSET: usize = 2;
    const ANIMATION_SELECTOR_OFFSET: usize = 4;
    const FRAMEBUFFER_WINDOW_OFFSET: usize = 6;
    const WORD_BYTE_COUNT: usize = 2;

    #[derive(Deserialize)]
    struct HandFrameOracle {
        name: String,
        inputs: HandFrameInputs,
        result: HandFrameResult,
    }

    #[derive(Deserialize)]
    struct HandFrameInputs {
        presentation_mode: u8,
        hud_mode: u8,
        requested_selector: u16,
        current_selector: u16,
        scene_blocked: u8,
        presentation_flags: u8,
        delay: u8,
        mouse_x: i16,
        mouse_y: i16,
        framebuffer_window_offset: u16,
    }

    #[derive(Deserialize)]
    struct HandFrameResult {
        requested_selector: u16,
        current_selector: u16,
        delay: u8,
        stack_request_hex: String,
        callback_calls: Vec<serde_json::Value>,
    }

    fn decode_request(encoded: &str) -> [u8; NATIVE_REQUEST_BYTE_COUNT] {
        let bytes = encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        bytes.try_into().unwrap()
    }

    fn encoded_word(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(bytes[offset..offset + WORD_BYTE_COUNT].try_into().unwrap())
    }

    #[test]
    fn hand_dispatch_matches_every_original_state_vector() {
        let vectors: Vec<HandFrameOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1610_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut state = Manu3HandFrameState {
                requested_animation: vector.inputs.requested_selector,
                current_animation: vector.inputs.current_selector,
                presentation_delay: vector.inputs.delay,
            };
            let request = update_manu3_hand_frame(
                &mut state,
                Manu3HandFrameContext {
                    presentation_mode_active: vector.inputs.presentation_mode & 1 != u8::MIN,
                    hud_refresh_active: vector.inputs.hud_mode & 1 != u8::MIN,
                    ship_scene_dispatch_blocked: vector.inputs.scene_blocked & 1 != u8::MIN,
                    presentation_request_pending: vector.inputs.presentation_flags & 2 != u8::MIN,
                    cursor: CursorPosition {
                        x: vector.inputs.mouse_x,
                        y: vector.inputs.mouse_y,
                    },
                },
            );

            assert_eq!(
                state.requested_animation, vector.result.requested_selector,
                "{}",
                vector.name
            );
            assert_eq!(
                state.current_animation, vector.result.current_selector,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation_delay, vector.result.delay,
                "{}",
                vector.name
            );
            assert_eq!(
                request.is_some(),
                !vector.result.callback_calls.is_empty(),
                "{}",
                vector.name
            );

            if let Some(request) = request {
                let encoded = decode_request(&vector.result.stack_request_hex);
                assert_eq!(
                    request.cursor.x as u16,
                    encoded_word(&encoded, CURSOR_X_OFFSET)
                );
                assert_eq!(
                    request.cursor.y as u16,
                    encoded_word(&encoded, CURSOR_Y_OFFSET)
                );
                assert_eq!(
                    request.animation_selector,
                    encoded_word(&encoded, ANIMATION_SELECTOR_OFFSET)
                );
                assert_eq!(
                    vector.inputs.framebuffer_window_offset,
                    encoded_word(&encoded, FRAMEBUFFER_WINDOW_OFFSET)
                );
            }
        }
    }
}
