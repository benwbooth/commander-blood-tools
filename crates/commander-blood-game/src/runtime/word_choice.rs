//! Concrete flat-memory host for dialogue concept word choices.

use anyhow::{Context, Result, bail};
use commander_blood_formats::instruction::ScriptTextWord;

use crate::native::bloodprg::{
    ChoiceListBackend, ChoiceListHandRequest, ChoiceListPointer, ChoiceListRect,
    FramebufferTransitionState, GameLifecycleState, PaletteRemapTable, PresentationWordChoice,
    PresentationWordChoiceBackend, PresentationWordChoiceContext, PresentationWordChoiceOutcome,
    PresentationWordChoicePhase, PresentationWordChoiceState, RasterPoint, TransitionRect,
    WORD_CHOICE_TRANSITION_STEPS, advance_framebuffer_rect_transition, build_banked_tint_table,
    update_presentation_word_choice,
};

use super::ModernGameServices;
use super::choice_list::{RuntimeChoiceListBackend, draw_choice_list_rows};

const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;
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
        let phase_before_update = self.state.phase;
        if self.state.phase == PresentationWordChoicePhase::Closed {
            self.transition = FramebufferTransitionState {
                total_steps: WORD_CHOICE_TRANSITION_STEPS as u8,
                current_step: u8::MIN,
            };
        }

        let fonts = services.runtime().data().font_resources().clone();
        let mut tint: PaletteRemapTable = [u8::MIN; 256];
        build_banked_tint_table(
            services.runtime().live_palette(),
            &mut tint,
            BRIDGE_CONSOLE_TINT_FIRST,
        )
        .context("building the dialogue choice-list tint table")?;
        let pointer = services.input().pointer_sample().position;
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeWordChoiceBackend {
            list: RuntimeChoiceListBackend::new(
                services.runtime_mut(),
                &fonts,
                &tint,
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

        match &outcome {
            PresentationWordChoiceOutcome::AwaitingSelection(frame) => {
                self.draw_frame(services, &fonts, frame)?;
            }
            PresentationWordChoiceOutcome::Selected { frame, .. } => {
                self.transition.current_step = u8::MIN;
                services.play_loaded_sound_bank_clip(CHOICE_LIST_SELECTION_SOUND_CLIP)?;
                self.draw_frame(services, &fonts, frame)?;
            }
            PresentationWordChoiceOutcome::Completed(word) => {
                services.complete_word_choice(*word, lifecycle)?;
            }
            PresentationWordChoiceOutcome::Gated(_)
            | PresentationWordChoiceOutcome::Opening
            | PresentationWordChoiceOutcome::Closing => {}
        }
        lifecycle.presentation.word_choice_active = self.state.active;
        Ok(outcome)
    }

    fn import_lifecycle_state<'window>(
        &mut self,
        services: &ModernGameServices<'window>,
        lifecycle: &GameLifecycleState,
    ) -> Result<()> {
        self.state.active = lifecycle.presentation.word_choice_active;
        self.state.presentation_deferred = lifecycle.presentation.menu_deferred;
        self.state.text_display_active = lifecycle.presentation.subtitle_display_active;
        self.state.dialogue_hold_complete = lifecycle.presentation.dialogue_hold_complete;
        self.state.request_pending = lifecycle.presentation.request_flags.text_request_pending();
        if !self.state.active
            || self.state.phase != PresentationWordChoicePhase::Closed
            || !self.state.choices.is_empty()
        {
            return Ok(());
        }

        let text = services.text_presentation();
        if text.menu_word_count > text.menu_words.len() {
            bail!(
                "dialogue choice count {} exceeds the {} live menu words",
                text.menu_word_count,
                text.menu_words.len()
            );
        }
        let profile = services
            .runtime()
            .current_profile()
            .context("dialogue choices require a loaded BloodScript profile")?;
        let dictionary = profile.dictionary();
        self.state.choices = text
            .menu_words
            .iter()
            .take(text.menu_word_count)
            .enumerate()
            .map(|(index, word)| {
                let ScriptTextWord::Dictionary(word) = word else {
                    bail!("dialogue choice {index} is an unexpected section separator");
                };
                let label = dictionary.word(*word).with_context(|| {
                    format!(
                        "dialogue choice {} is absent from the dictionary",
                        word.index()
                    )
                })?;
                Ok(PresentationWordChoice::new(*word, label))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    fn draw_frame<'window>(
        &self,
        services: &mut ModernGameServices<'window>,
        fonts: &commander_blood_formats::bloodprg::BloodprgFontResources,
        frame: &crate::native::bloodprg::ChoiceListFrame,
    ) -> Result<()> {
        let labels = self
            .state
            .choices
            .iter()
            .map(|choice| choice.label.as_ref())
            .collect::<Vec<_>>();
        draw_choice_list_rows(services.runtime_mut(), fonts, &labels, None, frame)
    }
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
                self.list.remap_region(
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

fn transition_rect(rect: ChoiceListRect) -> TransitionRect {
    TransitionRect::new(
        rect.origin[0],
        rect.origin[1],
        rect.size[0] as i16,
        rect.size[1] as i16,
    )
}
