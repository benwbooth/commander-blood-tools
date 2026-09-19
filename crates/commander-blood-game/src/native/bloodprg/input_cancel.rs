//! Presentation cancellation over owned runtime state.

use super::{
    GameLifecycleState, IndexedGamePalette, InputDispatchState, TextPresentationState,
    latch_input_text_byte,
};

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
    if !can_cancel_presentation(cancellation) {
        latch_input_text_byte(dispatch, text_byte);
        return InputCancellationOutcome::ForwardedToText;
    }

    cancel_presentation(cancellation, backend);
    InputCancellationOutcome::CancelledPresentation
}

fn can_cancel_presentation(cancellation: &InputCancellationState) -> bool {
    cancellation.presentation_active
        && !cancellation.dialogue_ready
        && !cancellation.ship_active
        && !(CANCELLATION_BLOCKED_LINE_FIRST..=CANCELLATION_BLOCKED_LINE_LAST)
            .contains(&cancellation.active_line)
}

fn cancel_presentation(
    cancellation: &mut InputCancellationState,
    backend: &mut impl InputCancellationBackend,
) {
    cancellation.dialogue_ready = cancellation.active_line == CANCELLATION_DIALOGUE_READY_LINE;
    cancellation.resources.read_position = cancellation.resources.rewind_position;
    cancellation.resources.remaining = cancellation.resources.rewind_remaining;
    backend.reset_presentation_queue();
    cancellation.scene_palette[..CANCELLATION_PALETTE_COLOR_COUNT]
        .fill([u8::MIN; PALETTE_COLOR_COMPONENT_COUNT]);
    cancellation.palette_dirty = true;
}

/// Result of BBB's secondary-pointer handler, before the script frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelSecondaryPointerOutcome {
    /// A native guard prevented any state changes.
    Unchanged,
    /// Subtitle/menu work was dismissed and the VM released.
    DismissedText,
    /// The active scene was rewound and its lower palette cleared.
    CancelledScene,
}

/// Translate the complete BBB-only entry at `0x1446..0x1502`.
/// Unlike Escape, this never changes pause, keyboard, or pointer latches.
pub fn handle_sequel_secondary_pointer(
    lifecycle: &mut GameLifecycleState,
    text: &mut TextPresentationState,
    panel_active: bool,
    cancellation: &mut InputCancellationState,
    backend: &mut impl InputCancellationBackend,
) -> SequelSecondaryPointerOutcome {
    if !lifecycle.secondary_pointer_pressed || panel_active {
        return SequelSecondaryPointerOutcome::Unchanged;
    }
    let presentation = &mut lifecycle.presentation;
    if presentation.active {
        if presentation.text_menu_pending || presentation.word_choice_active {
            return SequelSecondaryPointerOutcome::Unchanged;
        }
        if presentation.menu_deferred || presentation.subtitle_display_active {
            lifecycle.vm_execution_enabled = true;
            presentation.menu_deferred = false;
            presentation.subtitle_display_active = false;
            presentation.hold_ready = false;
            presentation.dialogue_hold_complete = false;
            presentation.c2_presentation_gate = false;
            presentation.subtitle_voice_trigger = false;
            presentation.request_flags.clear_pending_requests();
            text.menu_deferred = false;
            text.subtitle_display_active = false;
            text.hold_ready = false;
            text.dialogue_hold_complete = false;
            text.subtitle_voice_trigger = false;
            text.subtitle_reveal_cursor = None;
            text.request_flags.clear_pending_requests();
            backend.reset_presentation_queue();
            return SequelSecondaryPointerOutcome::DismissedText;
        }
    }
    if !can_cancel_presentation(cancellation) {
        return SequelSecondaryPointerOutcome::Unchanged;
    }
    cancel_presentation(cancellation, backend);
    lifecycle.vm_execution_enabled = true;
    SequelSecondaryPointerOutcome::CancelledScene
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

    #[derive(Deserialize)]
    struct SequelInputHandlerOracle {
        vectors: SequelInputHandlerVectors,
    }

    #[derive(Deserialize)]
    struct SequelInputHandlerVectors {
        cancel: Vec<SequelCancelOracle>,
    }

    #[derive(Deserialize)]
    struct SequelCancelOracle {
        name: String,
        presentation_active: bool,
        dialogue_ready_before: bool,
        ship_active: bool,
        active_line: usize,
        cancelled: bool,
        latched_key_before: u8,
        latched_key: u8,
        dialogue_ready: u8,
        read_position_before: usize,
        remaining_before: usize,
        rewind_position: usize,
        rewind_remaining: usize,
        read_position: usize,
        remaining: usize,
        palette_dirty: bool,
        presentation_complete: bool,
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
    fn sequel_secondary_pointer_matches_complete_assembly_state_transitions() {
        for mask in 0_u16..1024 {
            for line in [0, 3, 4, 7, 8, 39, 40, 41, 0x8000, 0xffff] {
                let bit = |index: u32| (mask & (1_u16 << index)) != 0_u16;
                let mut lifecycle = GameLifecycleState::default();
                lifecycle.secondary_pointer_pressed = bit(0);
                lifecycle.pause_hud_active = true;
                lifecycle.primary_pointer_pressed = true;
                lifecycle.pointer_press_pending = 7;
                let p = &mut lifecycle.presentation;
                p.active = bit(2);
                p.text_menu_pending = bit(3);
                p.word_choice_active = bit(4);
                p.menu_deferred = bit(5);
                p.subtitle_display_active = bit(6);
                p.c2_presentation_gate = bit(7);
                p.hold_ready = true;
                p.dialogue_hold_complete = true;
                p.subtitle_voice_trigger = true;
                p.dialogue_chatter_active = true;
                p.dialogue_hold_countdown = 37;
                p.request_flags = super::super::PresentationRequestFlags::decode(0xff);
                let mut text = TextPresentationState {
                    menu_deferred: p.menu_deferred,
                    subtitle_display_active: p.subtitle_display_active,
                    hold_ready: true,
                    dialogue_hold_complete: true,
                    subtitle_voice_trigger: true,
                    subtitle_reveal_cursor: Some(71),
                    request_flags: p.request_flags,
                    dialogue_chatter_active: true,
                    dialogue_hold_countdown: 37,
                    subtitle_text: b"retained".to_vec().into_boxed_slice(),
                    ..Default::default()
                };
                let mut cancellation = InputCancellationState {
                    presentation_active: bit(7),
                    dialogue_ready: bit(8),
                    ship_active: bit(9),
                    active_line: line,
                    resources: PresentationResourceCursor {
                        read_position: 500,
                        remaining: 600,
                        rewind_position: 100,
                        rewind_remaining: 200,
                    },
                    scene_palette: [[165; 3]; 256],
                    palette_dirty: false,
                };
                let mut expected_lifecycle = lifecycle.clone();
                let mut expected_text = text.clone();
                let mut expected_cancel = cancellation.clone();
                let blocked = !bit(0) || bit(1) || (bit(2) && (bit(3) || bit(4)));
                let outcome = if blocked {
                    SequelSecondaryPointerOutcome::Unchanged
                } else if bit(2) && (bit(5) || bit(6)) {
                    expected_lifecycle.vm_execution_enabled = true;
                    let p = &mut expected_lifecycle.presentation;
                    p.menu_deferred = false;
                    p.subtitle_display_active = false;
                    p.hold_ready = false;
                    p.dialogue_hold_complete = false;
                    p.c2_presentation_gate = false;
                    p.subtitle_voice_trigger = false;
                    p.request_flags = super::super::PresentationRequestFlags::decode(0xfc);
                    expected_text.menu_deferred = false;
                    expected_text.subtitle_display_active = false;
                    expected_text.hold_ready = false;
                    expected_text.dialogue_hold_complete = false;
                    expected_text.subtitle_voice_trigger = false;
                    expected_text.subtitle_reveal_cursor = None;
                    expected_text.request_flags = p.request_flags;
                    SequelSecondaryPointerOutcome::DismissedText
                } else if bit(7) && !bit(8) && !bit(9) && !(8..=40).contains(&line) {
                    expected_lifecycle.vm_execution_enabled = true;
                    expected_cancel.dialogue_ready = line == 4;
                    expected_cancel.resources.read_position = 100;
                    expected_cancel.resources.remaining = 200;
                    expected_cancel.scene_palette[..128].fill([0; 3]);
                    expected_cancel.palette_dirty = true;
                    SequelSecondaryPointerOutcome::CancelledScene
                } else {
                    SequelSecondaryPointerOutcome::Unchanged
                };
                let mut backend = QueueProbe::default();
                assert_eq!(
                    handle_sequel_secondary_pointer(
                        &mut lifecycle,
                        &mut text,
                        bit(1),
                        &mut cancellation,
                        &mut backend,
                    ),
                    outcome,
                    "mask={mask:#x} line={line}"
                );
                assert_eq!(lifecycle, expected_lifecycle, "mask={mask:#x} line={line}");
                assert_eq!(text, expected_text, "mask={mask:#x} line={line}");
                assert_eq!(cancellation, expected_cancel, "mask={mask:#x} line={line}");
                assert_eq!(
                    backend.reset_count,
                    usize::from(outcome != SequelSecondaryPointerOutcome::Unchanged)
                );
            }
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

    #[test]
    fn sequel_cancellation_matches_original_vectors() {
        let oracle: SequelInputHandlerOracle = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_input_handlers.json"
        ))
        .unwrap();
        assert_eq!(oracle.vectors.cancel.len(), 8);

        for vector in oracle.vectors.cancel {
            let mut dispatch = InputDispatchState {
                text_byte: Some(vector.latched_key_before),
                paused: true,
                shutdown_requested: false,
            };
            let mut cancellation = InputCancellationState {
                presentation_active: vector.presentation_active,
                dialogue_ready: vector.dialogue_ready_before,
                ship_active: vector.ship_active,
                active_line: vector.active_line,
                resources: PresentationResourceCursor {
                    read_position: vector.read_position_before,
                    remaining: vector.remaining_before,
                    rewind_position: vector.rewind_position,
                    rewind_remaining: vector.rewind_remaining,
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
            assert_eq!(cancellation.resources.read_position, vector.read_position);
            assert_eq!(cancellation.resources.remaining, vector.remaining);
            assert_eq!(cancellation.palette_dirty, vector.palette_dirty);
            assert_eq!(backend.reset_count, vector.calls.len(), "{}", vector.name);
            assert_eq!(vector.presentation_complete, vector.cancelled);

            if vector.cancelled {
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
                assert!(cancellation.scene_palette.iter().all(|color| {
                    *color == [TEST_PALETTE_VALUE; PALETTE_COLOR_COMPONENT_COUNT]
                }));
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
