//! Navigation confirmation dialog state and renderer-independent layout.

const CONFIRM_DIALOG_ACTIVE_FLAG: u8 = 1 << 1;
const CONFIRM_DIALOG_UI_FLAG: u16 = 1 << 2;
const CONFIRM_DIALOG_OPEN_STATE: u16 = 1;
const CONFIRM_DIALOG_CANCELLED_STATE: u16 = 11;

/// Palette index used to clear the confirmation panel.
pub const CONFIRM_DIALOG_BACKGROUND_PALETTE_INDEX: u8 = 226;
/// Palette index used for the panel outline and labels.
pub const CONFIRM_DIALOG_FOREGROUND_PALETTE_INDEX: u8 = 232;

const QUESTION_TEXT: &[u8] = b"ARE_YOU_SURE?";
const YES_TEXT: &[u8] = b"YES";
const NO_TEXT: &[u8] = b"NO";

/// One rectangle in the game's 320 by 200 logical coordinate space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmDialogRectangle {
    /// Horizontal origin.
    pub x: u16,
    /// Vertical origin.
    pub y: u16,
    /// Width in logical pixels.
    pub width: u16,
    /// Height in logical pixels.
    pub height: u16,
}

/// One square-cap font label in the navigation confirmation dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmDialogLabel {
    /// Original text bytes consumed by the game font renderer.
    pub text: &'static [u8],
    /// Logical text origin.
    pub position: [u16; 2],
}

/// Exact renderer-independent draw plan for the confirmation dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmDialogFrame {
    /// Rectangle first filled and then outlined by the native renderer.
    pub panel: ConfirmDialogRectangle,
    /// Palette index used for the fill pass.
    pub background_palette_index: u8,
    /// Palette index used for the outline and every label.
    pub foreground_palette_index: u8,
    /// Question, affirmative response, and negative response in draw order.
    pub labels: [ConfirmDialogLabel; 3],
}

impl ConfirmDialogFrame {
    /// Return the recovered static dialog geometry and text.
    pub const fn original() -> Self {
        Self {
            panel: ConfirmDialogRectangle {
                x: 90,
                y: 80,
                width: 140,
                height: 40,
            },
            background_palette_index: CONFIRM_DIALOG_BACKGROUND_PALETTE_INDEX,
            foreground_palette_index: CONFIRM_DIALOG_FOREGROUND_PALETTE_INDEX,
            labels: [
                ConfirmDialogLabel {
                    text: QUESTION_TEXT,
                    position: [100, 88],
                },
                ConfirmDialogLabel {
                    text: YES_TEXT,
                    position: [120, 105],
                },
                ConfirmDialogLabel {
                    text: NO_TEXT,
                    position: [180, 105],
                },
            ],
        }
    }
}

/// Mutable navigation and input latches touched by the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmDialogState {
    /// Bit-field controlling the pending navigation choice and sound.
    pub navigation_choice_gate: u8,
    /// Shared ship-navigation state code.
    pub navigation_state: u16,
    /// Shared presentation UI flags.
    pub ui_flags: u16,
    /// Whether the primary pointer press remains latched.
    pub primary_pointer_pressed: bool,
    /// Whether a pointer press remains pending for dispatch.
    pub pointer_press_pending: bool,
}

/// Hit-test results supplied by the SDL logical-coordinate input layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConfirmDialogHits {
    /// The pointer activated the affirmative response region.
    pub yes: bool,
    /// The pointer activated the negative response region.
    pub no: bool,
}

/// Result of one confirmation-dialog update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmDialogOutcome {
    /// The navigation gate did not request the dialog.
    Inactive,
    /// The dialog was drawn and remains open without a selection.
    AwaitingChoice(ConfirmDialogFrame),
    /// The affirmative region won hit-test priority and advanced the gate.
    Confirmed(ConfirmDialogFrame),
    /// The negative region dismissed the dialog and cleared pointer latches.
    Cancelled(ConfirmDialogFrame),
}

/// Update the authored navigation confirmation modal.
///
/// This translates `confirm_dialog_step` at BLOODPRG routine offset
/// `0x0014CA`. The exact draw order, YES-before-NO hit priority, full-byte gate
/// decrement, shared UI bit, navigation states, and pointer-latch reset remain.
/// Typed logical geometry and SDL hit results replace VGA callbacks and far
/// region records.
pub fn update_confirm_dialog(
    state: &mut ConfirmDialogState,
    hits: ConfirmDialogHits,
) -> ConfirmDialogOutcome {
    if state.navigation_choice_gate & CONFIRM_DIALOG_ACTIVE_FLAG == u8::MIN {
        return ConfirmDialogOutcome::Inactive;
    }

    state.navigation_state = CONFIRM_DIALOG_OPEN_STATE;
    state.ui_flags |= CONFIRM_DIALOG_UI_FLAG;
    let frame = ConfirmDialogFrame::original();

    if hits.yes {
        state.navigation_choice_gate = state.navigation_choice_gate.wrapping_sub(1);
        ConfirmDialogOutcome::Confirmed(frame)
    } else if hits.no {
        state.navigation_choice_gate = u8::MIN;
        state.ui_flags &= !CONFIRM_DIALOG_UI_FLAG;
        state.navigation_state = CONFIRM_DIALOG_CANCELLED_STATE;
        state.primary_pointer_pressed = false;
        state.pointer_press_pending = false;
        ConfirmDialogOutcome::Cancelled(frame)
    } else {
        ConfirmDialogOutcome::AwaitingChoice(frame)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 5;
    const INITIAL_NAVIGATION_STATE: u16 = 30_583;
    const INITIAL_UI_FLAGS: u16 = 42_401;

    #[derive(Deserialize)]
    struct DialogOracle {
        name: String,
        initial: InitialOracle,
        calls: Vec<serde_json::Value>,
        #[serde(rename = "final")]
        final_state: FinalOracle,
    }

    #[derive(Deserialize)]
    struct InitialOracle {
        gate: u8,
        hits: Vec<bool>,
    }

    #[derive(Deserialize)]
    struct FinalOracle {
        gate: u8,
        dialog_state: u16,
        ui_state: u16,
        mouse_primary: u8,
        mouse_pending: u8,
    }

    #[test]
    fn updates_match_every_original_dialog_vector() {
        let vectors: Vec<DialogOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_14ca_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut state = ConfirmDialogState {
                navigation_choice_gate: vector.initial.gate,
                navigation_state: INITIAL_NAVIGATION_STATE,
                ui_flags: INITIAL_UI_FLAGS,
                primary_pointer_pressed: true,
                pointer_press_pending: true,
            };
            let outcome = update_confirm_dialog(
                &mut state,
                ConfirmDialogHits {
                    yes: vector.initial.hits.first().copied().unwrap_or(false),
                    no: vector.initial.hits.get(1).copied().unwrap_or(false),
                },
            );

            assert_eq!(
                state.navigation_choice_gate, vector.final_state.gate,
                "{}",
                vector.name
            );
            assert_eq!(
                state.navigation_state, vector.final_state.dialog_state,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ui_flags, vector.final_state.ui_state,
                "{}",
                vector.name
            );
            assert_eq!(
                state.primary_pointer_pressed,
                vector.final_state.mouse_primary != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.pointer_press_pending,
                vector.final_state.mouse_pending != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                matches!(outcome, ConfirmDialogOutcome::Inactive),
                vector.calls.is_empty(),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn frame_retains_original_geometry_colors_and_labels() {
        let frame = ConfirmDialogFrame::original();
        assert_eq!(
            frame.panel,
            ConfirmDialogRectangle {
                x: 90,
                y: 80,
                width: 140,
                height: 40,
            }
        );
        assert_eq!(frame.background_palette_index, 226);
        assert_eq!(frame.foreground_palette_index, 232);
        assert_eq!(frame.labels[0].text, b"ARE_YOU_SURE?");
        assert_eq!(frame.labels[0].position, [100, 88]);
        assert_eq!(frame.labels[1].text, b"YES");
        assert_eq!(frame.labels[1].position, [120, 105]);
        assert_eq!(frame.labels[2].text, b"NO");
        assert_eq!(frame.labels[2].position, [180, 105]);
    }

    #[test]
    fn affirmative_hit_has_original_priority_when_both_regions_report_hit() {
        let mut state = ConfirmDialogState {
            navigation_choice_gate: CONFIRM_DIALOG_ACTIVE_FLAG,
            navigation_state: u16::MIN,
            ui_flags: u16::MIN,
            primary_pointer_pressed: true,
            pointer_press_pending: true,
        };

        let outcome = update_confirm_dialog(
            &mut state,
            ConfirmDialogHits {
                yes: true,
                no: true,
            },
        );

        assert!(matches!(outcome, ConfirmDialogOutcome::Confirmed(_)));
        assert_eq!(state.navigation_choice_gate, 1);
        assert!(state.primary_pointer_pressed);
        assert!(state.pointer_press_pending);
    }
}
