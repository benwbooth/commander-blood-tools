//! Typed dialogue word-choice opening, selection, and closing flow.

use commander_blood_formats::script::ScriptWordId;

use super::{
    ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListRect, ChoiceListState,
    update_choice_list,
};

/// Number of original interpolation steps used by the dialogue choice panel.
pub const WORD_CHOICE_TRANSITION_STEPS: u16 = 4;

const WORD_CHOICE_CENTER_X: i16 = 225;

/// One interned dialogue concept and its decoded display label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationWordChoice {
    /// Stable identity in the decoded script dictionary.
    pub word: ScriptWordId,
    /// Owned original game-font label.
    pub label: Box<[u8]>,
}

impl PresentationWordChoice {
    /// Build one typed dialogue choice.
    pub fn new(word: ScriptWordId, label: impl Into<Box<[u8]>>) -> Self {
        Self {
            word,
            label: label.into(),
        }
    }
}

/// Semantic lifecycle of the dialogue word-choice panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationWordChoicePhase {
    /// The next accepted update initializes the panel.
    #[default]
    Closed,
    /// The panel is moving into view.
    Opening,
    /// The panel accepts a word selection.
    Selecting,
    /// The panel is moving out of view.
    Closing,
}

/// Host operations required by the word-choice presentation.
pub trait PresentationWordChoiceBackend: ChoiceListBackend {
    /// Advance one opening or closing transition.
    fn advance_word_choice_transition(
        &mut self,
        source: ChoiceListRect,
        target: ChoiceListRect,
    ) -> bool;
}

/// Read-only gates and fixed target geometry for one update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationWordChoiceContext {
    /// Dialogue presentation currently owns the screen.
    pub presentation_active: bool,
    /// Another presentation request blocks word selection.
    pub request_busy: bool,
    /// Vertical position and height of the panel's open target.
    pub animation_target: ChoiceListRect,
}

/// Flat owned state for one dialogue word-choice request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationWordChoiceState {
    /// Whether BloodScript currently requests a word choice.
    pub active: bool,
    /// Semantic panel lifecycle.
    pub phase: PresentationWordChoicePhase,
    /// Decoded words offered by the current script presentation.
    pub choices: Vec<PresentationWordChoice>,
    /// Shared list-widget interaction state.
    pub list: ChoiceListState,
    /// Rectangle calculated by the initial layout pass.
    pub current_rect: ChoiceListRect,
    /// Opening target with x and width resolved from the measured list.
    pub animation_target: ChoiceListRect,
    /// Word selected while the panel closes.
    pub selected_word: Option<ScriptWordId>,
    /// Word published back to BloodScript after closing.
    pub published_word: Option<ScriptWordId>,
    /// Whether this panel currently owns shared interface presentation.
    pub interface_active: bool,
    /// Existing presentation defer latch cleared on completion.
    pub presentation_deferred: bool,
    /// Dialogue text display latch cleared on completion.
    pub text_display_active: bool,
    /// Dialogue hold completion latch cleared on completion.
    pub dialogue_hold_complete: bool,
    /// Pending presentation request cleared on completion.
    pub request_pending: bool,
}

/// Gate that prevented dialogue choice work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationWordChoiceGate {
    /// Dialogue presentation is inactive.
    PresentationInactive,
    /// BloodScript has not requested a word choice.
    ChoiceInactive,
    /// Another presentation request owns the screen.
    RequestBusy,
    /// The decoded choice list is empty.
    EmptyChoices,
}

/// Result of one dialogue word-choice update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationWordChoiceOutcome {
    /// A gate rejected the update without changing state.
    Gated(PresentationWordChoiceGate),
    /// Opening interpolation remains in progress.
    Opening,
    /// The list remains open without a selection.
    AwaitingSelection(ChoiceListFrame),
    /// One interned word was selected and closing began.
    Selected {
        /// Interned concept selected by the player.
        word: ScriptWordId,
        /// Final interactive list frame drawn before closing.
        frame: ChoiceListFrame,
    },
    /// Closing interpolation remains in progress.
    Closing,
    /// The selected word was published and all presentation latches cleared.
    Completed(ScriptWordId),
}

/// Update one dialogue word-choice request.
///
/// This translates `presentation_ready_gate` at BLOODPRG routine offset
/// `0x008963`. Interned word IDs and owned labels replace dictionary offsets,
/// a sentinel-terminated word buffer, and unchecked wrapped indexing. Explicit
/// phases and booleans replace packed UI/request bytes and numeric phase masks.
pub fn update_presentation_word_choice<Backend: PresentationWordChoiceBackend>(
    context: PresentationWordChoiceContext,
    state: &mut PresentationWordChoiceState,
    backend: &mut Backend,
) -> PresentationWordChoiceOutcome {
    if !context.presentation_active {
        return PresentationWordChoiceOutcome::Gated(
            PresentationWordChoiceGate::PresentationInactive,
        );
    }
    if !state.active {
        return PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::ChoiceInactive);
    }
    if context.request_busy {
        return PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::RequestBusy);
    }
    if state.choices.is_empty() {
        return PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::EmptyChoices);
    }

    let labels = state
        .choices
        .iter()
        .map(|choice| choice.label.as_ref())
        .collect::<Vec<_>>();
    if state.phase == PresentationWordChoicePhase::Closed {
        state.interface_active = true;
        state.current_rect =
            update_choice_list(&labels, choice_list_config(true), &mut state.list, backend).rect;
        state.animation_target = ChoiceListRect {
            origin: [
                state.current_rect.origin[0],
                context.animation_target.origin[1],
            ],
            size: [state.current_rect.size[0], context.animation_target.size[1]],
        };
        state.phase = PresentationWordChoicePhase::Opening;
    }

    if state.phase == PresentationWordChoicePhase::Opening {
        if !backend.advance_word_choice_transition(state.current_rect, state.animation_target) {
            return PresentationWordChoiceOutcome::Opening;
        }
        state.phase = PresentationWordChoicePhase::Selecting;
    }

    if state.phase == PresentationWordChoicePhase::Selecting {
        let frame =
            update_choice_list(&labels, choice_list_config(false), &mut state.list, backend);
        let Some(index) = frame.selected_item else {
            return PresentationWordChoiceOutcome::AwaitingSelection(frame);
        };
        let Some(choice) = state.choices.get(index) else {
            return PresentationWordChoiceOutcome::AwaitingSelection(frame);
        };
        state.selected_word = Some(choice.word);
        state.phase = PresentationWordChoicePhase::Closing;
        return PresentationWordChoiceOutcome::Selected {
            word: choice.word,
            frame,
        };
    }

    if !backend.advance_word_choice_transition(state.animation_target, state.current_rect) {
        return PresentationWordChoiceOutcome::Closing;
    }

    let selected_word = state
        .selected_word
        .expect("closing word-choice state always owns a selected word");
    state.published_word = Some(selected_word);
    state.active = false;
    state.phase = PresentationWordChoicePhase::Closed;
    state.choices.clear();
    state.interface_active = false;
    state.presentation_deferred = false;
    state.text_display_active = false;
    state.dialogue_hold_complete = false;
    state.request_pending = false;
    PresentationWordChoiceOutcome::Completed(selected_word)
}

const fn choice_list_config(layout_only: bool) -> ChoiceListConfig<'static> {
    ChoiceListConfig {
        center_x: WORD_CHOICE_CENTER_X,
        preserve_individual_widths: false,
        cancel_label: None,
        layout_only,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::ChoiceListPointer;

    const ORACLE_VECTOR_COUNT: usize = 16;
    const TARGET_RECT: ChoiceListRect = ChoiceListRect {
        origin: [12, 40],
        size: [34, 70],
    };

    #[derive(Deserialize)]
    struct WordChoiceOracle {
        name: String,
        status: String,
        active: u8,
        word_choice_gate_before: u8,
        request_flags_before: u8,
        phase_before: u8,
        calls: Vec<serde_json::Value>,
    }

    #[test]
    fn word_choice_matches_every_flat_original_semantic_vector() {
        let vectors: Vec<WordChoiceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8963_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let dictionary = decode_script_dictionary(b"FIRST\0SECOND\0THIRD\0").unwrap();
        let words = [
            dictionary.resolve_source_offset(0).unwrap(),
            dictionary.resolve_source_offset(6).unwrap(),
            dictionary.resolve_source_offset(13).unwrap(),
        ];

        for vector in vectors {
            let native_selection = list_selection(&vector.calls);
            if native_selection.is_some_and(|index| index >= words.len()) {
                assert!(
                    native_selection.unwrap() >= words.len(),
                    "{} must remain rejected by flat checked indexing",
                    vector.name
                );
                continue;
            }
            let choices = if vector.name == "empty_word_list" {
                Vec::new()
            } else {
                words
                    .into_iter()
                    .enumerate()
                    .map(|(index, word)| {
                        PresentationWordChoice::new(word, format!("WORD{index}").into_bytes())
                    })
                    .collect()
            };
            let mut state = PresentationWordChoiceState {
                active: vector.word_choice_gate_before & 1 != 0,
                phase: phase_from_native(vector.phase_before),
                choices,
                selected_word: Some(words[0]),
                published_word: Some(words[1]),
                presentation_deferred: true,
                text_display_active: true,
                dialogue_hold_complete: true,
                request_pending: vector.request_flags_before & 1 != 0,
                ..PresentationWordChoiceState::default()
            };
            let mut backend =
                OracleBackend::new(native_selection, transition_results(&vector.calls));
            let outcome = update_presentation_word_choice(
                PresentationWordChoiceContext {
                    presentation_active: vector.active & 1 != 0,
                    request_busy: vector.request_flags_before & 2 != 0,
                    animation_target: TARGET_RECT,
                },
                &mut state,
                &mut backend,
            );
            assert!(
                outcome_matches_status(&outcome, &vector.status),
                "{}: {outcome:?}",
                vector.name
            );
            if let PresentationWordChoiceOutcome::Selected { word, .. } = outcome {
                assert_eq!(
                    Some(word),
                    native_selection.map(|index| words[index]),
                    "{}",
                    vector.name
                );
            }
            if let PresentationWordChoiceOutcome::Completed(word) = outcome {
                assert_eq!(state.published_word, Some(word), "{}", vector.name);
                assert!(!state.active && state.choices.is_empty(), "{}", vector.name);
                assert!(
                    !state.presentation_deferred && !state.text_display_active,
                    "{}",
                    vector.name
                );
                assert!(
                    !state.dialogue_hold_complete && !state.request_pending,
                    "{}",
                    vector.name
                );
            }
        }
    }

    fn list_selection(calls: &[serde_json::Value]) -> Option<usize> {
        calls
            .iter()
            .find(|call| call["call"] == "list_widget_layout_unified" && call["editing"] != 1)
            .and_then(|call| {
                let result = call["result"].as_u64()? as u16;
                ((result as i16) >= 0).then_some(usize::from(result))
            })
    }

    fn transition_results(calls: &[serde_json::Value]) -> VecDeque<bool> {
        calls
            .iter()
            .filter(|call| call["call"] == "framebuffer_rect_interpolate_and_remap_step")
            .map(|call| call["complete"].as_bool().unwrap())
            .collect()
    }

    const fn phase_from_native(phase: u8) -> PresentationWordChoicePhase {
        match phase & 7 {
            0 => PresentationWordChoicePhase::Closed,
            1 => PresentationWordChoicePhase::Opening,
            2 => PresentationWordChoicePhase::Selecting,
            3..=7 => PresentationWordChoicePhase::Closing,
            _ => PresentationWordChoicePhase::Closed,
        }
    }

    fn outcome_matches_status(outcome: &PresentationWordChoiceOutcome, status: &str) -> bool {
        match outcome {
            PresentationWordChoiceOutcome::Gated(
                PresentationWordChoiceGate::PresentationInactive,
            ) => status == "presentation_active",
            PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::ChoiceInactive) => {
                status == "word_choice_gate"
            }
            PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::RequestBusy) => {
                status == "request_busy"
            }
            PresentationWordChoiceOutcome::Gated(PresentationWordChoiceGate::EmptyChoices) => {
                status == "empty_list"
            }
            PresentationWordChoiceOutcome::Opening => status == "opening_incomplete",
            PresentationWordChoiceOutcome::AwaitingSelection(_) => status == "negative_selection",
            PresentationWordChoiceOutcome::Selected { .. } => status == "selected",
            PresentationWordChoiceOutcome::Closing => status == "closing_incomplete",
            PresentationWordChoiceOutcome::Completed(_) => status == "complete",
        }
    }

    struct OracleBackend {
        requested_selection: Option<usize>,
        pointer: ChoiceListPointer,
        transitions: VecDeque<bool>,
    }

    impl OracleBackend {
        fn new(requested_selection: Option<usize>, transitions: VecDeque<bool>) -> Self {
            Self {
                requested_selection,
                pointer: ChoiceListPointer::default(),
                transitions,
            }
        }
    }

    impl ChoiceListBackend for OracleBackend {
        fn measure_label(&mut self, _label: &[u8]) -> u16 {
            30
        }

        fn prepare_background(&mut self, rect: ChoiceListRect) {
            if let Some(row) = self.requested_selection {
                self.pointer = ChoiceListPointer {
                    position: [
                        rect.origin[0].wrapping_add((rect.size[0] >> 1) as i16),
                        rect.origin[1]
                            .wrapping_add(4)
                            .wrapping_add(i16::try_from(row).unwrap_or(i16::MAX).wrapping_mul(11)),
                    ],
                    primary_pressed: true,
                };
            }
        }

        fn pointer(&mut self) -> ChoiceListPointer {
            self.pointer
        }
    }

    impl PresentationWordChoiceBackend for OracleBackend {
        fn advance_word_choice_transition(
            &mut self,
            _source: ChoiceListRect,
            _target: ChoiceListRect,
        ) -> bool {
            self.transitions.pop_front().unwrap_or(true)
        }
    }
}
