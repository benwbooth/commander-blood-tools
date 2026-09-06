//! Presentation choice-list transition and result mapping.

use std::fmt;

use super::{
    FramebufferTransitionError, FramebufferTransitionState, TransitionRect, TransitionRenderRegion,
    advance_framebuffer_rect_transition,
};

const PRESENTATION_CHOICE_ACTIVE_FLAG: u8 = 1;
const PRESENTATION_CHOICE_LAYOUT_PHASE: u8 = 1;
const PRESENTATION_CHOICE_TRANSITION_PHASE: u8 = 1 << 1;
const PRESENTATION_CHOICE_UI_FLAG: u8 = 1 << 2;
const PRESENTATION_CHOICE_TRANSITION_STEP_COUNT: u8 = 6;
const SPECIAL_CHOICE_INDEX: usize = 4;
const SPECIAL_CHOICE_RESULT: u16 = 7;
const FIRST_ORDINARY_CHOICE_RESULT: usize = 1;

/// Whether one authored list position carries a selectable choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationChoiceItem {
    /// The table position contains a text item.
    Selectable,
    /// The table position publishes an explicit authored value (sequel speed menu).
    Value(u16),
    /// The table position contains the native `0xFFFF` sentinel.
    Sentinel,
}

/// Mutable state owned by the presentation choice coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationChoiceState {
    /// Full native activation byte; only its low bit gates an update.
    pub activation_flags: u8,
    /// Layout and transition phase bits.
    pub phase: u8,
    /// Shared rectangle-transition progress.
    pub transition: FramebufferTransitionState,
    /// Whether the list widget is in its layout-only mode.
    pub layout_only: bool,
    /// Shared presentation UI flags.
    pub ui_flags: u8,
    /// Last accepted presentation result.
    pub result: u16,
}

/// Work and lifecycle result from one choice-list update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationChoiceOutcome {
    /// The low activation bit was clear and no state changed.
    Inactive,
    /// A transition region must be remapped before the next update.
    Transitioning {
        /// Whether this update also performed the list's layout-only pass.
        layout_prepared: bool,
        /// Logical region emitted by the recovered rectangle interpolator.
        region: TransitionRenderRegion,
    },
    /// No list selection was made and the choice remains active.
    AwaitingSelection {
        /// Whether this update also performed the list's layout-only pass.
        layout_prepared: bool,
    },
    /// A list position was selected and the choice UI closed.
    Closed {
        /// Whether this update also performed the list's layout-only pass.
        layout_prepared: bool,
        /// Newly published result, or `None` for a sentinel list position.
        published_result: Option<u16>,
    },
}

/// Invalid typed input or rectangle arithmetic in the choice coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationChoiceError {
    /// The list backend returned an index outside the owned item table.
    MissingItem {
        /// Invalid selected index.
        selected: usize,
        /// Number of authored table positions.
        item_count: usize,
    },
    /// An ordinary selected index cannot be represented by the native result.
    ResultOutsideNativeRange {
        /// Selected zero-based index.
        selected: usize,
    },
    /// The shared rectangle interpolator rejected malformed arithmetic state.
    Transition(FramebufferTransitionError),
}

impl fmt::Display for PresentationChoiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation choice update: {self:?}")
    }
}

impl std::error::Error for PresentationChoiceError {}

impl From<FramebufferTransitionError> for PresentationChoiceError {
    fn from(error: FramebufferTransitionError) -> Self {
        Self::Transition(error)
    }
}

/// Advance the presentation choice list and consume an optional selection.
///
/// This translates `presentation_choice_transition_step` at BLOODPRG routine
/// offset `0x001AD3`. It retains the low-bit activation gate, layout-pass
/// ordering, six-step transition, pre-step completion decision, sentinel close,
/// one-based result mapping, special index-four result 7, and UI-bit lifetime.
/// Typed items and logical rectangles replace near tables and framebuffer
/// callbacks.
pub fn update_presentation_choice(
    state: &mut PresentationChoiceState,
    items: &[PresentationChoiceItem],
    selected_index: Option<usize>,
    transition_source: TransitionRect,
    transition_target: TransitionRect,
) -> Result<PresentationChoiceOutcome, PresentationChoiceError> {
    if state.activation_flags & PRESENTATION_CHOICE_ACTIVE_FLAG == u8::MIN {
        return Ok(PresentationChoiceOutcome::Inactive);
    }

    state.ui_flags |= PRESENTATION_CHOICE_UI_FLAG;
    let mut layout_prepared = false;
    if state.phase & PRESENTATION_CHOICE_LAYOUT_PHASE != u8::MIN {
        state.layout_only = true;
        layout_prepared = true;
        state.layout_only = false;
        state.transition.current_step = u8::MIN;
        state.transition.total_steps = PRESENTATION_CHOICE_TRANSITION_STEP_COUNT;
        state.phase = state.phase.wrapping_add(1);
    }

    if state.phase & PRESENTATION_CHOICE_TRANSITION_PHASE != u8::MIN {
        let transition_complete = state.transition.total_steps == state.transition.current_step;
        let region = advance_framebuffer_rect_transition(
            &mut state.transition,
            transition_source,
            transition_target,
        )?;
        if !transition_complete {
            return Ok(PresentationChoiceOutcome::Transitioning {
                layout_prepared,
                region: region.expect("an incomplete transition emits one region"),
            });
        }
        state.phase = u8::MIN;
    }

    let Some(selected_index) = selected_index else {
        return Ok(PresentationChoiceOutcome::AwaitingSelection { layout_prepared });
    };
    let item = items
        .get(selected_index)
        .ok_or(PresentationChoiceError::MissingItem {
            selected: selected_index,
            item_count: items.len(),
        })?;
    let published_result = match item {
        PresentationChoiceItem::Sentinel => None,
        PresentationChoiceItem::Value(value) => Some(*value),
        PresentationChoiceItem::Selectable if selected_index == SPECIAL_CHOICE_INDEX => {
            Some(SPECIAL_CHOICE_RESULT)
        }
        PresentationChoiceItem::Selectable => Some(
            u16::try_from(
                selected_index
                    .checked_add(FIRST_ORDINARY_CHOICE_RESULT)
                    .ok_or(PresentationChoiceError::ResultOutsideNativeRange {
                        selected: selected_index,
                    })?,
            )
            .map_err(|_| PresentationChoiceError::ResultOutsideNativeRange {
                selected: selected_index,
            })?,
        ),
    };
    if let Some(result) = published_result {
        state.result = result;
    }
    state.ui_flags &= !PRESENTATION_CHOICE_UI_FLAG;
    state.activation_flags = u8::MIN;

    Ok(PresentationChoiceOutcome::Closed {
        layout_prepared,
        published_result,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 8;
    const INITIAL_LAYOUT_BYTE: u8 = 85;
    const INITIAL_UI_FLAGS: u8 = 161;
    const INITIAL_RESULT: u16 = 30_583;
    const TEST_SOURCE: TransitionRect = TransitionRect::new(12, 24, 48, 64);
    const TEST_TARGET: TransitionRect = TransitionRect::new(6, 12, 24, 32);

    #[derive(Deserialize)]
    struct SpeedOracle {
        executable_sha256: String,
        values: [u16; 3],
        labels: [Vec<u8>; 3],
        initial_value: u16,
        cases: Vec<SpeedCase>,
    }

    #[derive(Debug, Deserialize)]
    struct SpeedCase {
        selected: i16,
        previous: u16,
        ui_flags: u8,
        result: u16,
        active: u8,
        final_ui_flags: u8,
    }

    #[test]
    fn sequel_simulation_speed_matches_original_selection_tail() {
        let oracle: SpeedOracle = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_speed_choice.json"
        ))
        .unwrap();
        assert_eq!(
            oracle.executable_sha256,
            "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
        );
        assert_eq!(oracle.values, [100, 10, 1]);
        assert_eq!(oracle.initial_value, 1);
        assert_eq!(
            oracle.labels,
            [b"LENT".to_vec(), b"NORMAL".to_vec(), b"RAPIDE".to_vec()]
        );
        assert_eq!(oracle.cases.len(), 90);
        let mut items = oracle.values.map(PresentationChoiceItem::Value).to_vec();
        items.push(PresentationChoiceItem::Sentinel);
        for case in oracle.cases {
            let mut state = PresentationChoiceState {
                activation_flags: 1,
                phase: 0,
                transition: FramebufferTransitionState::default(),
                layout_only: false,
                ui_flags: case.ui_flags,
                result: case.previous,
            };
            let outcome = update_presentation_choice(
                &mut state,
                &items,
                usize::try_from(case.selected).ok(),
                TEST_SOURCE,
                TEST_TARGET,
            )
            .unwrap();
            assert_eq!(state.result, case.result, "{case:?}");
            assert_eq!(state.activation_flags, case.active, "{case:?}");
            assert_eq!(state.ui_flags, case.final_ui_flags, "{case:?}");
            if case.selected < 0 {
                assert_eq!(
                    outcome,
                    PresentationChoiceOutcome::AwaitingSelection {
                        layout_prepared: false
                    }
                );
            } else {
                assert_eq!(
                    outcome,
                    PresentationChoiceOutcome::Closed {
                        layout_prepared: false,
                        published_result: (case.selected < 3).then_some(case.result),
                    }
                );
            }
        }
    }

    #[test]
    fn explicit_value_overrides_special_text_speed_row() {
        let mut state = PresentationChoiceState {
            activation_flags: 1,
            phase: 0,
            transition: FramebufferTransitionState::default(),
            layout_only: false,
            ui_flags: 0,
            result: 42,
        };
        let items = [PresentationChoiceItem::Value(0); 5];
        let outcome =
            update_presentation_choice(&mut state, &items, Some(4), TEST_SOURCE, TEST_TARGET)
                .unwrap();
        assert_eq!(state.result, 0);
        assert_eq!(
            outcome,
            PresentationChoiceOutcome::Closed {
                layout_prepared: false,
                published_result: Some(0),
            }
        );
    }

    #[derive(Deserialize)]
    struct ChoiceOracle {
        name: String,
        initial: InitialOracle,
        calls: Vec<serde_json::Value>,
        #[serde(rename = "final")]
        final_state: FinalOracle,
    }

    #[derive(Deserialize)]
    struct InitialOracle {
        active: u8,
        phase: u8,
        current: u8,
        total: u8,
    }

    #[derive(Deserialize)]
    struct FinalOracle {
        active: u8,
        phase: u8,
        current: u8,
        total: u8,
        editing: u8,
        ui_flags: u8,
        result: u16,
    }

    #[test]
    fn coordinator_matches_every_original_choice_vector() {
        let vectors: Vec<ChoiceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1ad3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut items = [PresentationChoiceItem::Selectable; 5];
            let selected_index = vector.calls.iter().rev().find_map(interactive_selection);
            if vector.name == "sentinel_selection_closes_without_result" {
                items[selected_index.unwrap()] = PresentationChoiceItem::Sentinel;
            }
            let mut state = PresentationChoiceState {
                activation_flags: vector.initial.active,
                phase: vector.initial.phase,
                transition: FramebufferTransitionState {
                    total_steps: vector.initial.total,
                    current_step: vector.initial.current,
                },
                layout_only: INITIAL_LAYOUT_BYTE != u8::MIN,
                ui_flags: INITIAL_UI_FLAGS,
                result: INITIAL_RESULT,
            };

            let outcome = update_presentation_choice(
                &mut state,
                &items,
                selected_index,
                TEST_SOURCE,
                TEST_TARGET,
            )
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(
                state.activation_flags, vector.final_state.active,
                "{}",
                vector.name
            );
            assert_eq!(state.phase, vector.final_state.phase, "{}", vector.name);
            assert_eq!(
                state.transition.current_step, vector.final_state.current,
                "{}",
                vector.name
            );
            assert_eq!(
                state.transition.total_steps, vector.final_state.total,
                "{}",
                vector.name
            );
            assert_eq!(
                state.layout_only,
                vector.final_state.editing != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ui_flags, vector.final_state.ui_flags,
                "{}",
                vector.name
            );
            assert_eq!(state.result, vector.final_state.result, "{}", vector.name);
            assert_eq!(
                matches!(outcome, PresentationChoiceOutcome::Inactive),
                vector.calls.is_empty(),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn invalid_flat_selection_is_rejected_without_closing_the_choice() {
        let mut state = PresentationChoiceState {
            activation_flags: PRESENTATION_CHOICE_ACTIVE_FLAG,
            phase: u8::MIN,
            transition: FramebufferTransitionState::default(),
            layout_only: false,
            ui_flags: u8::MIN,
            result: u16::MIN,
        };

        assert_eq!(
            update_presentation_choice(
                &mut state,
                &[PresentationChoiceItem::Selectable],
                Some(1),
                TEST_SOURCE,
                TEST_TARGET,
            ),
            Err(PresentationChoiceError::MissingItem {
                selected: 1,
                item_count: 1,
            })
        );
        assert_eq!(state.activation_flags, PRESENTATION_CHOICE_ACTIVE_FLAG);
    }

    fn interactive_selection(call: &serde_json::Value) -> Option<usize> {
        if call["call"] != "list_widget_layout_unified" || call["editing"] == 1 {
            return None;
        }
        let result = call["result"].as_u64()? as u16;
        (result as i16 >= 0).then_some(usize::from(result))
    }
}
