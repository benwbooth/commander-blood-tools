//! Presentation cancellation over owned runtime state.

use super::{IndexedGamePalette, InputDispatchState, latch_input_text_byte};

/// First presentation line that blocks cancellation.
pub const CANCELLATION_BLOCKED_LINE_FIRST: usize = 8;
/// Last presentation line that blocks cancellation.
pub const CANCELLATION_BLOCKED_LINE_LAST: usize = 40;
/// Presentation line that marks dialogue ready after cancellation.
pub const CANCELLATION_DIALOGUE_READY_LINE: usize = 4;
/// Number of leading palette colors cleared by cancellation.
pub const CANCELLATION_PALETTE_COLOR_COUNT: usize = 128;
const PALETTE_COLOR_COMPONENT_COUNT: usize = 3;

/// Current read window and its rewind point in an owned resource stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationResourceCursor {
    /// Current byte position in the resource stream.
    pub read_position: usize,
    /// Bytes remaining in the current stream window.
    pub remaining: usize,
    /// Byte position restored by cancellation.
    pub rewind_position: usize,
    /// Remaining-byte count restored by cancellation.
    pub rewind_remaining: usize,
}

/// Presentation state affected by Escape cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputCancellationState {
    /// Whether a cancellable presentation is active.
    pub presentation_active: bool,
    /// Whether the dialogue phase has already reached its ready state.
    pub dialogue_ready: bool,
    /// Whether active ship behavior blocks cancellation.
    pub ship_active: bool,
    /// Current semantic presentation line.
    pub active_line: usize,
    /// Owned resource stream position and rewind point.
    pub resources: PresentationResourceCursor,
    /// Complete indexed scene palette.
    pub scene_palette: IndexedGamePalette,
    /// Whether the renderer must upload the modified palette.
    pub palette_dirty: bool,
}

/// External queue operation performed by a successful cancellation.
pub trait InputCancellationBackend {
    /// Reset pending presentation-list work.
    fn reset_presentation_queue(&mut self);
}

/// Result of handling one cancel command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCancellationOutcome {
    /// The active presentation was rewound and cleared.
    CancelledPresentation,
    /// Cancellation was unavailable, so Escape was forwarded to text input.
    ForwardedToText,
}

/// Cancel an eligible presentation or forward the source byte to text input.
///
/// This translates `input_action_cancel` at BLOODPRG routine offset `0x00224D`.
/// Typed booleans, an owned resource cursor, a fixed palette, and an explicit
/// queue backend replace packed gates, global positions, and a raw bulk clear.
pub fn cancel_input_action<Backend: InputCancellationBackend>(
    dispatch: &mut InputDispatchState,
    cancellation: &mut InputCancellationState,
    backend: &mut Backend,
    text_byte: u8,
) -> InputCancellationOutcome {
    dispatch.paused = false;
    let line_blocks_cancellation = (CANCELLATION_BLOCKED_LINE_FIRST
        ..=CANCELLATION_BLOCKED_LINE_LAST)
        .contains(&cancellation.active_line);
    let can_cancel = cancellation.presentation_active
        && !cancellation.dialogue_ready
        && !cancellation.ship_active
        && !line_blocks_cancellation;

    if !can_cancel {
        latch_input_text_byte(dispatch, text_byte);
        return InputCancellationOutcome::ForwardedToText;
    }

    cancellation.dialogue_ready = cancellation.active_line == CANCELLATION_DIALOGUE_READY_LINE;
    cancellation.resources.read_position = cancellation.resources.rewind_position;
    cancellation.resources.remaining = cancellation.resources.rewind_remaining;
    backend.reset_presentation_queue();
    cancellation.scene_palette[..CANCELLATION_PALETTE_COLOR_COUNT]
        .fill([u8::MIN; PALETTE_COLOR_COMPONENT_COUNT]);
    cancellation.palette_dirty = true;
    InputCancellationOutcome::CancelledPresentation
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const CANCELLATION_VECTOR_COUNT: usize = 4;
    const ESCAPE_TEXT_BYTE: u8 = b'\x1b';
    const TEST_PALETTE_VALUE: u8 = 165;
    const INITIAL_READ_POSITION: usize = 500;
    const INITIAL_REMAINING: usize = 600;
    const REWIND_POSITION: usize = 100;
    const REWIND_REMAINING: usize = 200;

    #[derive(Deserialize)]
    struct InputHandlerOracle {
        vectors: InputHandlerVectors,
    }

    #[derive(Deserialize)]
    struct InputHandlerVectors {
        cancel: Vec<CancelOracle>,
    }

    #[derive(Deserialize)]
    struct CancelOracle {
        name: String,
        presentation_active: bool,
        dialogue_ready_before: bool,
        ship_active: bool,
        active_line: usize,
        cancelled: bool,
        latched_key: u8,
        dialogue_ready: u8,
        calls: Vec<String>,
    }

    #[derive(Default)]
    struct QueueProbe {
        reset_count: usize,
    }

    impl InputCancellationBackend for QueueProbe {
        fn reset_presentation_queue(&mut self) {
            self.reset_count += 1;
        }
    }

    #[test]
    fn cancellation_matches_every_original_handler_vector() {
        let oracle = handler_oracle();
        assert_eq!(oracle.vectors.cancel.len(), CANCELLATION_VECTOR_COUNT);

        for vector in oracle.vectors.cancel {
            let mut dispatch = InputDispatchState {
                text_byte: None,
                paused: true,
                shutdown_requested: false,
            };
            let mut cancellation = InputCancellationState {
                presentation_active: vector.presentation_active,
                dialogue_ready: vector.dialogue_ready_before,
                ship_active: vector.ship_active,
                active_line: vector.active_line,
                resources: PresentationResourceCursor {
                    read_position: INITIAL_READ_POSITION,
                    remaining: INITIAL_REMAINING,
                    rewind_position: REWIND_POSITION,
                    rewind_remaining: REWIND_REMAINING,
                },
                scene_palette: [[TEST_PALETTE_VALUE; PALETTE_COLOR_COMPONENT_COUNT]; 256],
                palette_dirty: false,
            };
            let mut backend = QueueProbe::default();

            let outcome = cancel_input_action(
                &mut dispatch,
                &mut cancellation,
                &mut backend,
                ESCAPE_TEXT_BYTE,
            );

            assert!(!dispatch.paused, "{}", vector.name);
            assert_eq!(
                outcome == InputCancellationOutcome::CancelledPresentation,
                vector.cancelled,
                "{}",
                vector.name
            );
            assert_eq!(
                cancellation.dialogue_ready,
                vector.dialogue_ready != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                dispatch.text_byte.unwrap_or(u8::MIN),
                vector.latched_key,
                "{}",
                vector.name
            );
            assert_eq!(backend.reset_count, vector.calls.len(), "{}", vector.name);

            if vector.cancelled {
                assert_eq!(cancellation.resources.read_position, REWIND_POSITION);
                assert_eq!(cancellation.resources.remaining, REWIND_REMAINING);
                assert!(cancellation.palette_dirty);
                assert!(
                    cancellation.scene_palette[..CANCELLATION_PALETTE_COLOR_COUNT]
                        .iter()
                        .all(|color| *color == [u8::MIN; PALETTE_COLOR_COMPONENT_COUNT])
                );
                assert!(
                    cancellation.scene_palette[CANCELLATION_PALETTE_COLOR_COUNT..]
                        .iter()
                        .all(|color| {
                            *color == [TEST_PALETTE_VALUE; PALETTE_COLOR_COMPONENT_COUNT]
                        })
                );
            } else {
                assert_eq!(cancellation.resources.read_position, INITIAL_READ_POSITION);
                assert_eq!(cancellation.resources.remaining, INITIAL_REMAINING);
                assert!(!cancellation.palette_dirty);
            }
        }
    }

    fn handler_oracle() -> InputHandlerOracle {
        serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/input_action_handlers_natural.json"
        ))
        .unwrap()
    }
}
