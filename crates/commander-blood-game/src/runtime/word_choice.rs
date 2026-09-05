//! Concrete flat-memory host for dialogue concept word choices.

use anyhow::{Context, Result};
use commander_blood_formats::script::{ScriptDictionary, ScriptWordId};

use crate::native::bloodprg::{
    ChoiceListBackend, ChoiceListHandRequest, ChoiceListPointer, ChoiceListRect,
    FramebufferTransitionState, GameLifecycleState, PresentationChoiceId, PresentationWordChoice,
    PresentationWordChoiceBackend, PresentationWordChoiceContext, PresentationWordChoiceOutcome,
    PresentationWordChoicePhase, PresentationWordChoiceState, RasterPoint, TransitionRect,
    WORD_CHOICE_TRANSITION_STEPS, advance_framebuffer_rect_transition,
    update_presentation_word_choice,
};

use super::ModernGameServices;
use super::choice_list::{RuntimeChoiceListBackend, draw_choice_list_rows};

const COLLAPSED_CHOICE_LIST_X: i16 = 0;
const COLLAPSED_CHOICE_LIST_Y: i16 = 100;
const COLLAPSED_CHOICE_LIST_WIDTH: u16 = u16::MIN;
const COLLAPSED_CHOICE_LIST_HEIGHT: u16 = u16::MIN;
const CHOICE_LIST_SELECTION_SOUND_CLIP: u8 = u8::MIN;

/// Persistent dialogue-choice state and its recovered rectangle interpolator.
#[derive(Default)]
pub struct RuntimePresentationWordChoice {
    state: PresentationWordChoiceState,
    transition: FramebufferTransitionState,
}

impl RuntimePresentationWordChoice {
    /// Borrow the semantic state shared with the main lifecycle.
    pub const fn state(&self) -> &PresentationWordChoiceState {
        &self.state
    }

    /// Close the dialogue chooser and discard its profile-local transition state.
    pub fn reset(&mut self) {
        self.state = PresentationWordChoiceState::default();
        self.transition = FramebufferTransitionState::default();
    }

    /// Advance and draw one exact dialogue-choice frame.
    pub fn update<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<PresentationWordChoiceOutcome> {
        self.import_lifecycle_state(services, lifecycle)?;
        let interface_active_before_update = self.state.interface_active;
        let phase_before_update = self.state.phase;
        let inventory_before_update = self.state.inventory_cancel_label.is_some();
        if self.state.phase == PresentationWordChoicePhase::Closed {
            self.transition = FramebufferTransitionState {
                total_steps: WORD_CHOICE_TRANSITION_STEPS as u8,
                current_step: u8::MIN,
            };
        }

        let fonts = services.runtime().data().font_resources().clone();
        let pointer = services.input().pointer_sample().position;
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeWordChoiceBackend {
            list: RuntimeChoiceListBackend::new(
                services.runtime_mut(),
                &fonts,
                ChoiceListPointer {
                    position: pointer,
                    primary_pressed: lifecycle.primary_pointer_pressed,
                },
                current_hand_animation,
            ),
            transition: &mut self.transition,
        };
        let outcome = update_presentation_word_choice(
            PresentationWordChoiceContext {
                presentation_active: lifecycle.presentation.active,
                request_busy: lifecycle
                    .presentation
                    .request_flags
                    .secondary_request_pending(),
                animation_target: ChoiceListRect {
                    origin: [COLLAPSED_CHOICE_LIST_X, COLLAPSED_CHOICE_LIST_Y],
                    size: [COLLAPSED_CHOICE_LIST_WIDTH, COLLAPSED_CHOICE_LIST_HEIGHT],
                },
            },
            &mut self.state,
            &mut backend,
        );
        backend.list.finish()?;
        let hand_requests = backend.list.take_hand_requests();
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);
        if phase_before_update == PresentationWordChoicePhase::Closed
            && self.state.phase != PresentationWordChoicePhase::Closed
        {
            services.activate_presentation_word_choice_style();
        }
        if inventory_before_update && !matches!(outcome, PresentationWordChoiceOutcome::Gated(_)) {
            services.set_choice_list_cancel_entry(
                phase_before_update != PresentationWordChoicePhase::Closed
                    && self.state.inventory_cancel_label.is_some(),
            );
        }

        match &outcome {
            PresentationWordChoiceOutcome::AwaitingSelection(frame) => {
                self.draw_frame(services, frame)?;
            }
            PresentationWordChoiceOutcome::Selected { frame, .. }
            | PresentationWordChoiceOutcome::CancelSelected(frame) => {
                self.transition.current_step = u8::MIN;
                services.play_loaded_sound_bank_clip(CHOICE_LIST_SELECTION_SOUND_CLIP)?;
                self.draw_frame(services, frame)?;
            }
            PresentationWordChoiceOutcome::Completed(PresentationChoiceId::Dictionary(word)) => {
                services.complete_word_choice(*word, lifecycle)?;
            }
            PresentationWordChoiceOutcome::Completed(PresentationChoiceId::Inventory(object)) => {
                services.complete_inventory_choice(Some(*object), lifecycle)?;
            }
            PresentationWordChoiceOutcome::Cancelled => {
                services.complete_inventory_choice(None, lifecycle)?;
            }
            PresentationWordChoiceOutcome::Gated(_)
            | PresentationWordChoiceOutcome::Opening
            | PresentationWordChoiceOutcome::Closing => {}
        }
        publish_lifecycle_state(lifecycle, &self.state, interface_active_before_update);
        Ok(outcome)
    }

    fn import_lifecycle_state<'window>(
        &mut self,
        services: &ModernGameServices<'window>,
        lifecycle: &GameLifecycleState,
    ) -> Result<()> {
        import_lifecycle_latches(&mut self.state, lifecycle);
        self.state.dialect = services.runtime().data().game().script_dialect();
        if !self.state.active
            || self.state.phase != PresentationWordChoicePhase::Closed
            || !self.state.choices.is_empty()
        {
            return Ok(());
        }

        let profile = services
            .runtime()
            .current_profile()
            .context("dialogue choices require a loaded BloodScript profile")?;
        if !profile.selector_state().inventory().choices().is_empty() {
            anyhow::ensure!(
                profile
                    .selector_state()
                    .pending_presentation_words()
                    .is_empty(),
                "inventory choices cannot alias a simultaneous dictionary choice list"
            );
            self.state.inventory_cancel_label = Some(
                services
                    .runtime()
                    .data()
                    .game()
                    .decode_inventory_cancel_label(services.runtime().data().executable())?,
            );
            self.state.choices = profile
                .selector_state()
                .inventory()
                .presentation_choices(profile.state())?;
        } else {
            anyhow::ensure!(
                profile.selector_state().inventory().saved_line().is_none()
                    || profile
                        .selector_state()
                        .pending_presentation_words()
                        .is_empty(),
                "dictionary choices cannot use a retained inventory-line owner"
            );
            self.state.inventory_cancel_label = None;
            self.state.choices = decode_pending_presentation_choices(
                profile.dictionary(),
                profile.selector_state().pending_presentation_words(),
            )?;
        }
        Ok(())
    }

    fn draw_frame<'window>(
        &self,
        services: &mut ModernGameServices<'window>,
        frame: &crate::native::bloodprg::ChoiceListFrame,
    ) -> Result<()> {
        let labels = self
            .state
            .choices
            .iter()
            .map(|choice| choice.label.as_ref())
            .collect::<Vec<_>>();
        draw_choice_list_rows(
            services.runtime_mut(),
            &labels,
            self.state.inventory_cancel_label.as_deref(),
            frame,
        )
    }
}

fn publish_lifecycle_state(
    lifecycle: &mut GameLifecycleState,
    state: &PresentationWordChoiceState,
    interface_active_before_update: bool,
) {
    lifecycle.vm_execution_enabled = state.vm_execution_enabled;
    lifecycle.presentation.word_choice_active = state.active;
    if !interface_active_before_update && state.interface_active {
        lifecycle.set_modal_ui_busy(true);
    }
}

fn import_lifecycle_latches(
    state: &mut PresentationWordChoiceState,
    lifecycle: &GameLifecycleState,
) {
    state.interface_active = lifecycle.modal_ui_busy();
    state.vm_execution_enabled = lifecycle.vm_execution_enabled;
    state.active = lifecycle.presentation.word_choice_active;
    state.presentation_deferred = lifecycle.presentation.menu_deferred;
    state.text_display_active = lifecycle.presentation.subtitle_display_active;
    state.dialogue_hold_complete = lifecycle.presentation.dialogue_hold_complete;
    state.request_pending = lifecycle.presentation.request_flags.text_request_pending();
}

fn decode_pending_presentation_choices(
    dictionary: &ScriptDictionary,
    words: &[ScriptWordId],
) -> Result<Vec<PresentationWordChoice>> {
    words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            let label = dictionary.word(*word).with_context(|| {
                format!(
                    "dialogue choice {index} with dictionary ID {} is absent from the dictionary",
                    word.index()
                )
            })?;
            Ok(PresentationWordChoice::new(*word, label))
        })
        .collect()
}

struct RuntimeWordChoiceBackend<'runtime, 'transition> {
    list: RuntimeChoiceListBackend<'runtime>,
    transition: &'transition mut FramebufferTransitionState,
}

impl ChoiceListBackend for RuntimeWordChoiceBackend<'_, '_> {
    fn measure_label(&mut self, label: &[u8]) -> u16 {
        self.list.measure_label(label)
    }

    fn prepare_background(&mut self, rect: ChoiceListRect) {
        self.list.prepare_background(rect);
    }

    fn pointer(&mut self) -> ChoiceListPointer {
        self.list.pointer()
    }

    fn current_hand_animation(&self) -> u16 {
        self.list.current_hand_animation()
    }

    fn request_hand_animation(&mut self, request: ChoiceListHandRequest) {
        self.list.request_hand_animation(request);
    }
}

impl PresentationWordChoiceBackend for RuntimeWordChoiceBackend<'_, '_> {
    fn advance_word_choice_transition(
        &mut self,
        source: ChoiceListRect,
        target: ChoiceListRect,
    ) -> bool {
        let result = advance_framebuffer_rect_transition(
            self.transition,
            transition_rect(source),
            transition_rect(target),
        )
        .context("advancing a dialogue choice-list transition");
        match result {
            Ok(Some(region)) => {
                self.list.darken_region(
                    RasterPoint {
                        x: i32::from(region.x),
                        y: i32::from(region.y),
                    },
                    region.width,
                    region.height,
                );
                false
            }
            Ok(None) => true,
            Err(error) => {
                self.list.record_error(Err(error));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::script::decode_script_dictionary;

    use super::*;

    #[test]
    fn choice_labels_come_from_the_selector_buffer_not_revealed_dialogue_words() {
        let dictionary = decode_script_dictionary(b"Click\0quick\0explanations\0game\0").unwrap();
        let explanations = dictionary.resolve_source_offset(12).unwrap();
        let game = dictionary.resolve_source_offset(25).unwrap();

        let choices =
            decode_pending_presentation_choices(&dictionary, &[explanations, game]).unwrap();

        assert_eq!(choices.len(), 2);
        assert_eq!(
            choices[0].identity,
            PresentationChoiceId::Dictionary(explanations)
        );
        assert_eq!(choices[0].label.as_ref(), b"explanations");
        assert_eq!(choices[1].identity, PresentationChoiceId::Dictionary(game));
        assert_eq!(choices[1].label.as_ref(), b"game");
    }

    #[test]
    fn reset_clears_every_persistent_word_choice_owner() {
        let mut word_choice = RuntimePresentationWordChoice::default();
        word_choice.state.active = true;
        word_choice.state.phase = PresentationWordChoicePhase::Selecting;
        word_choice.state.interface_active = true;
        word_choice.state.presentation_deferred = true;
        word_choice.state.text_display_active = true;
        word_choice.state.dialogue_hold_complete = true;
        word_choice.state.request_pending = true;
        word_choice.transition = FramebufferTransitionState {
            total_steps: 9,
            current_step: 4,
        };

        word_choice.reset();

        assert_eq!(word_choice.state, PresentationWordChoiceState::default());
        assert_eq!(
            word_choice.transition,
            FramebufferTransitionState::default()
        );
    }

    #[test]
    fn completed_word_choice_leaves_the_modal_bit_for_the_next_bridge_owner() {
        let mut lifecycle = GameLifecycleState::default();
        let mut state = PresentationWordChoiceState {
            active: true,
            interface_active: true,
            ..PresentationWordChoiceState::default()
        };

        publish_lifecycle_state(&mut lifecycle, &state, false);
        assert!(lifecycle.presentation.word_choice_active);
        assert!(lifecycle.modal_ui_busy());

        state.active = false;
        publish_lifecycle_state(&mut lifecycle, &state, true);
        assert!(!lifecycle.presentation.word_choice_active);
        assert!(lifecycle.modal_ui_busy());
    }

    #[test]
    fn reopening_imports_the_current_modal_bit_and_vm_latch() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.presentation.word_choice_active = true;
        lifecycle.vm_execution_enabled = true;
        let mut state = PresentationWordChoiceState {
            interface_active: true,
            ..Default::default()
        };
        import_lifecycle_latches(&mut state, &lifecycle);
        assert!(!state.interface_active);
        assert!(state.vm_execution_enabled);
        // A new opening latches the modal bit even after a previous chooser did so.
        state.interface_active = true;
        state.vm_execution_enabled = false;
        publish_lifecycle_state(&mut lifecycle, &state, false);
        assert!(lifecycle.modal_ui_busy());
        assert!(!lifecycle.vm_execution_enabled);
    }

    #[test]
    fn inactive_word_choice_does_not_clear_another_modal_ui_owner() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);

        publish_lifecycle_state(
            &mut lifecycle,
            &PresentationWordChoiceState::default(),
            false,
        );

        assert!(lifecycle.modal_ui_busy());
    }
}

fn transition_rect(rect: ChoiceListRect) -> TransitionRect {
    TransitionRect::new(
        rect.origin[0],
        rect.origin[1],
        rect.size[0] as i16,
        rect.size[1] as i16,
    )
}
