//! Production bridge-console interaction over flat runtime state.

use anyhow::{Context, Result};
use commander_blood_formats::bloodprg::BloodprgBridgeMenuText;
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    BridgeChoiceBackend, BridgeConsoleChoice, BridgeConsoleContext, BridgeConsoleDispatchOutcome,
    BridgeConsolePalettePlan, BridgeConsoleState, BridgeDeferredActionKind, BridgeDeferredState,
    BridgeRecordChoice, BridgeRecordChoiceContext, BridgeRecordChoiceOutcome,
    BridgeRecordChoiceState, ChoiceListBackend, ChoiceListConfig, ChoiceListFrame,
    ChoiceListHandRequest, ChoiceListPointer, ChoiceListRect, ChoiceListState,
    FramebufferTransitionState, GameLifecycleState, ImmediateBridgeChoiceOutcome,
    Manu3AnimationSelector, MusicOptionLabel, OptionMenuChoice, OptionMenuOutcome,
    PresentationChoiceItem, PresentationChoiceOutcome, PresentationChoiceState, RasterPoint,
    SequelOptionMenuChoice, SequelOptionMenuState, TransitionRect, activate_horn_choice,
    activate_radio_choice, advance_framebuffer_rect_transition, navigation_actor_targets,
    update_bridge_console_dispatch, update_choice_list, update_contact_choice,
    update_navigation_target_choice, update_option_menu, update_presentation_choice,
    update_sequel_option_menu,
};

use super::choice_list::{RuntimeChoiceListBackend, draw_choice_list_rows};
use super::{ModernGameServices, OriginalGameRuntime};

const BRIDGE_CONSOLE_SELECTION_CLIP: u8 = 4;
const PANEL_TRANSITION_STEP_COUNT: u8 = 10;
const PRESENTATION_CHOICE_ACTIVE_FLAG: u8 = 1;
const PRESENTATION_CHOICE_LAYOUT_PHASE: u8 = 1;
const PRESENTATION_CHOICE_TRANSITION_PHASE: u8 = 1 << 1;
const PRESENTATION_CHOICE_MODAL_UI_FLAG: u8 = 1 << 2;
const TEXT_SPEED_LABEL_COUNT: usize = 5;
const PANEL_CENTER_X: i16 = 100;

/// Persistent production state for the recovered five-command bridge console.
pub(super) struct RuntimeBridgeConsole {
    console: BridgeConsoleState,
    navigation: BridgeRecordChoiceState<ScriptObjectId>,
    contacts: BridgeRecordChoiceState<ScriptObjectId>,
    deferred: BridgeDeferredState<ScriptObjectId>,
    options: SequelOptionMenuState,
    text_speed: RuntimeTextSpeedMenu,
    transition: FramebufferTransitionState,
}

impl RuntimeBridgeConsole {
    pub(super) fn new(initial_text_speed_step: u16, initial_travel_enabled: bool) -> Self {
        Self {
            console: BridgeConsoleState::default(),
            navigation: BridgeRecordChoiceState::default(),
            contacts: BridgeRecordChoiceState::default(),
            deferred: BridgeDeferredState {
                record: None,
                redraw_requested: false,
            },
            options: SequelOptionMenuState {
                travel_enabled: initial_travel_enabled,
                ..Default::default()
            },
            text_speed: RuntimeTextSpeedMenu::new(initial_text_speed_step),
            transition: FramebufferTransitionState::default(),
        }
    }

    pub(super) const fn selected_item_active(&self) -> bool {
        self.console.selected.is_some()
    }

    pub(super) fn semantic_trace_snapshot(
        &self,
        menu_text: &BloodprgBridgeMenuText,
    ) -> serde_json::Value {
        let selected = self.console.selected.map(bridge_console_choice_name);
        let panel_phase = bridge_choice_panel_phase_name(self.console.panel_phase);
        let (choice_records, choice_labels) = match self.console.selected {
            Some(BridgeConsoleChoice::Navigation) => record_choice_trace(&self.navigation.choices),
            Some(BridgeConsoleChoice::Contacts) => record_choice_trace(&self.contacts.choices),
            Some(BridgeConsoleChoice::Options) => {
                let mut labels = option_labels(
                    menu_text,
                    self.options.common.music_label,
                    self.options.travel_enabled,
                )
                .iter()
                .map(|label| String::from_utf8_lossy(label).into_owned())
                .collect::<Vec<_>>();
                labels.push(
                    String::from_utf8_lossy(&menu_label(menu_text, menu_text.cancel_label()))
                        .into_owned(),
                );
                (Vec::new(), labels)
            }
            Some(BridgeConsoleChoice::Horn | BridgeConsoleChoice::Radio) | None => {
                (Vec::new(), Vec::new())
            }
        };
        serde_json::json!({
            "selected": selected,
            "panel_phase": panel_phase,
            "interface_busy": self.console.interface_busy,
            "interface_active": self.console.interface_active,
            "panel_target_y": self.console.panel_target_y,
            "choice_records": choice_records,
            "choice_labels": choice_labels,
            "text_options_active": self.options.common.text_options_active,
            "simulation_options_active": self.options.simulation_options_active,
            "travel_enabled": self.options.travel_enabled,
        })
    }

    pub(super) fn clear_selected_item_alias(&mut self) {
        self.console.selected = None;
    }

    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.speed_menu_active() {
            self.publish_interface_ownership(lifecycle);
            return Ok(());
        }

        let console_owned_modal_ui_before_update = self.console.selected.is_some();
        if self.console.interface_busy {
            synchronize_console_seek(&mut self.console, services.bridge_seek_requested()?);
        }
        let pointer = services.input().pointer_sample();
        let outcome = update_bridge_console_dispatch(
            BridgeConsoleContext {
                aboard_transfer_pending: lifecycle.presentation.c2_presentation_gate,
                save_motion_active: lifecycle.profile_change_blockers.save_active,
                load_motion_active: lifecycle.profile_change_blockers.load_active,
                option_panel_active: self.speed_menu_active(),
                sound_action_active: services.confirm_dialog_state().navigation_choice_gate
                    != u8::MIN,
                presentation_active: lifecycle.presentation.active,
                bridge_view_frame: services.bridge_view_frame()?,
                pointer: pointer.position,
                primary_pressed: lifecycle.primary_pointer_pressed,
            },
            &mut self.console,
        );

        match outcome {
            BridgeConsoleDispatchOutcome::PointerOutside(palette)
            | BridgeConsoleDispatchOutcome::Hovered { palette, .. } => {
                apply_console_palette(services.runtime_mut(), palette);
            }
            BridgeConsoleDispatchOutcome::Activated {
                palette,
                play_selection_clip,
                ..
            } => {
                apply_console_palette(services.runtime_mut(), palette);
                services.request_manu3_animation(Manu3AnimationSelector::NavigationChoice);
                services.activate_bridge_console_list_style();
                self.transition = FramebufferTransitionState::default();
                services.request_bridge_seek(self.console.hold_ticks)?;
                if play_selection_clip {
                    services.play_loaded_sound_bank_clip(BRIDGE_CONSOLE_SELECTION_CLIP)?;
                }
            }
            BridgeConsoleDispatchOutcome::HandlerRequested(choice) => {
                self.update_selected_handler(choice, services, lifecycle)?;
            }
            BridgeConsoleDispatchOutcome::Gated(_) => {}
        }
        self.publish_interface_ownership(lifecycle);
        publish_console_modal_ui(
            lifecycle,
            console_owned_modal_ui_before_update,
            &self.console,
        );
        Ok(())
    }

    pub(super) fn update_presentation_choice(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.speed_menu_active() {
            import_text_speed_modal_ui(&mut self.text_speed.state, lifecycle);
            self.update_text_speed_menu(services, lifecycle.primary_pointer_pressed)?;
            export_text_speed_modal_ui(&self.text_speed.state, lifecycle);
        }
        self.publish_interface_ownership(lifecycle);
        Ok(())
    }

    fn update_selected_handler(
        &mut self,
        choice: BridgeConsoleChoice,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        match choice {
            BridgeConsoleChoice::Horn => {
                let record = required_builtin(services, |builtins| builtins.horn, "Honk")?;
                let outcome = activate_horn_choice(record, &mut self.console, &mut self.deferred);
                debug_assert_eq!(outcome, ImmediateBridgeChoiceOutcome::Activated);
            }
            BridgeConsoleChoice::Navigation => {
                self.update_navigation_menu(services, lifecycle.primary_pointer_pressed)?
            }
            BridgeConsoleChoice::Contacts => {
                self.update_contact_menu(services, lifecycle.primary_pointer_pressed)?
            }
            BridgeConsoleChoice::Radio => {
                self.activate_radio(services, lifecycle.primary_pointer_pressed)?
            }
            BridgeConsoleChoice::Options => self.update_option_menu(services, lifecycle)?,
        }
        self.apply_deferred_record(services)
    }

    fn update_navigation_menu(
        &mut self,
        services: &mut ModernGameServices<'_>,
        primary_pointer_pressed: bool,
    ) -> Result<()> {
        let choices = navigation_choices(services)?;
        let menu_text = services.runtime().data().bridge_menu_text();
        let cancel_label = menu_label(menu_text, menu_text.cancel_label());
        let context = BridgeRecordChoiceContext {
            animation_target: selected_row_rect(self.console.panel_target_y),
            cancel_label: &cancel_label,
        };
        let fonts = services.runtime().data().font_resources().clone();
        let pointer = choice_pointer(services, primary_pointer_pressed);
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            pointer,
            current_hand_animation,
            &mut self.transition,
        );
        let outcome = update_navigation_target_choice(
            &choices,
            context,
            &mut self.console,
            &mut self.navigation,
            &mut self.deferred,
            &mut backend,
        );
        let effects = backend.effects();
        let hand_requests = backend.take_hand_requests();
        backend.finish()?;
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);

        if let BridgeRecordChoiceOutcome::Interactive(frame) = &outcome {
            let labels = self
                .navigation
                .choices
                .iter()
                .map(|choice| choice.label.as_ref())
                .collect::<Vec<_>>();
            draw_runtime_choice_rows(services, &labels, Some(context.cancel_label), frame)?;
        }
        apply_backend_effects(services, effects)
    }

    fn update_contact_menu(
        &mut self,
        services: &mut ModernGameServices<'_>,
        primary_pointer_pressed: bool,
    ) -> Result<()> {
        let choices = contact_choices(services)?;
        let menu_text = services.runtime().data().bridge_menu_text();
        let cancel_label = menu_label(menu_text, menu_text.cancel_label());
        let context = BridgeRecordChoiceContext {
            animation_target: selected_row_rect(self.console.panel_target_y),
            cancel_label: &cancel_label,
        };
        let fonts = services.runtime().data().font_resources().clone();
        let pointer = choice_pointer(services, primary_pointer_pressed);
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            pointer,
            current_hand_animation,
            &mut self.transition,
        );
        let outcome = update_contact_choice(
            &choices,
            context,
            &mut self.console,
            &mut self.contacts,
            &mut self.deferred,
            &mut backend,
        );
        let effects = backend.effects();
        let hand_requests = backend.take_hand_requests();
        backend.finish()?;
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);

        if let BridgeRecordChoiceOutcome::Interactive(frame) = &outcome {
            let labels = self
                .contacts
                .choices
                .iter()
                .map(|choice| choice.label.as_ref())
                .collect::<Vec<_>>();
            draw_runtime_choice_rows(services, &labels, Some(context.cancel_label), frame)?;
        }
        apply_backend_effects(services, effects)
    }

    fn activate_radio(
        &mut self,
        services: &mut ModernGameServices<'_>,
        primary_pointer_pressed: bool,
    ) -> Result<()> {
        let record = required_builtin(services, |builtins| builtins.menu, "menu")?;
        let fonts = services.runtime().data().font_resources().clone();
        let pointer = choice_pointer(services, primary_pointer_pressed);
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            pointer,
            current_hand_animation,
            &mut self.transition,
        );
        let outcome =
            activate_radio_choice(record, &mut self.console, &mut self.deferred, &mut backend);
        let effects = backend.effects();
        let hand_requests = backend.take_hand_requests();
        backend.finish()?;
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);
        debug_assert_eq!(outcome, ImmediateBridgeChoiceOutcome::Activated);
        apply_backend_effects(services, effects)
    }

    fn update_option_menu(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        let music_enabled = services.navigation_music_enabled()?;
        let sequel = services.sequel_travel_enabled();
        if let Some(enabled) = sequel {
            self.options.travel_enabled = enabled;
        }
        self.options.primary_pointer_pressed = lifecycle.primary_pointer_pressed;
        self.options.secondary_pointer_pressed = lifecycle.secondary_pointer_pressed;
        self.options.common.music_supported = services.audio_is_initialized();
        self.options.common.music_active = music_enabled;
        self.options.common.music_label = if music_enabled {
            MusicOptionLabel::MusicOff
        } else {
            MusicOptionLabel::MusicOn
        };
        let labels = option_labels(
            services.runtime().data().bridge_menu_text(),
            self.options.common.music_label,
            self.options.travel_enabled,
        );
        let label_refs = labels.iter().map(Box::as_ref).collect::<Vec<_>>();
        let menu_text = services.runtime().data().bridge_menu_text();
        let cancel_label = menu_label(menu_text, menu_text.cancel_label());
        let fonts = services.runtime().data().font_resources().clone();
        let pointer = choice_pointer(services, lifecycle.primary_pointer_pressed);
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            pointer,
            current_hand_animation,
            &mut self.transition,
        );
        let outcome = if sequel.is_some() {
            update_sequel_option_menu(
                &label_refs,
                &cancel_label,
                selected_row_rect(self.console.panel_target_y),
                &mut self.console,
                &mut self.options,
                &mut backend,
            )
        } else {
            match update_option_menu(
                &label_refs,
                &cancel_label,
                selected_row_rect(self.console.panel_target_y),
                &mut self.console,
                &mut self.options.common,
                &mut backend,
            ) {
                OptionMenuOutcome::Inactive => OptionMenuOutcome::Inactive,
                OptionMenuOutcome::Transitioning => OptionMenuOutcome::Transitioning,
                OptionMenuOutcome::Cancelled => OptionMenuOutcome::Cancelled,
                OptionMenuOutcome::Interactive(frame) => OptionMenuOutcome::Interactive(frame),
                OptionMenuOutcome::Selected(choice) => {
                    OptionMenuOutcome::Selected(SequelOptionMenuChoice::Common(choice))
                }
            }
        };
        let effects = backend.effects();
        let hand_requests = backend.take_hand_requests();
        backend.finish()?;
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);

        if let OptionMenuOutcome::Interactive(frame) = &outcome {
            draw_runtime_choice_rows(services, &label_refs, Some(&cancel_label), frame)?;
        }
        if music_enabled != self.options.common.music_active {
            services.set_navigation_music_enabled(self.options.common.music_active)?;
        }
        apply_backend_effects(services, effects)?;

        if let OptionMenuOutcome::Selected(choice) = outcome {
            let choice = match choice {
                SequelOptionMenuChoice::SimulationSpeed => {
                    self.text_speed.begin(
                        services.sequel_simulation_speed()?,
                        self.options.common.current_rect,
                    );
                    return Ok(());
                }
                SequelOptionMenuChoice::Travel => {
                    services.set_sequel_travel_enabled(self.options.travel_enabled)?;
                    return Ok(());
                }
                SequelOptionMenuChoice::Common(choice) => choice,
            };
            match choice {
                OptionMenuChoice::Text => {
                    self.text_speed.begin(
                        services.dialogue_word_delay()?,
                        self.options.common.current_rect,
                    );
                }
                OptionMenuChoice::Music => {}
                OptionMenuChoice::Save => {
                    self.options.common.save_motion_requested = false;
                    services.request_save_menu()?;
                }
                OptionMenuChoice::Load => {
                    self.options.common.load_motion_requested = false;
                    services.request_load_menu()?;
                }
                OptionMenuChoice::Quit => {
                    self.options.common.quit_requested = false;
                    if sequel.is_some() {
                        lifecycle.primary_pointer_pressed = self.options.primary_pointer_pressed;
                        lifecycle.secondary_pointer_pressed =
                            self.options.secondary_pointer_pressed;
                    }
                    activate_quit_confirmation(
                        lifecycle,
                        &mut services.confirm_dialog_state_mut().navigation_choice_gate,
                    );
                }
            }
        }
        Ok(())
    }

    fn update_text_speed_menu(
        &mut self,
        services: &mut ModernGameServices<'_>,
        primary_pointer_pressed: bool,
    ) -> Result<()> {
        let text = services.runtime().data().bridge_menu_text();
        let (labels, mut items) = if self.options.simulation_options_active {
            let controls = text
                .sequel_controls()
                .context("simulation menu requires sequel controls")?;
            (
                controls.speed_labels.to_vec(),
                controls
                    .speed_values
                    .into_iter()
                    .map(PresentationChoiceItem::Value)
                    .collect::<Vec<_>>(),
            )
        } else {
            (
                text.text_speed_labels().to_vec(),
                vec![PresentationChoiceItem::Selectable; TEXT_SPEED_LABEL_COUNT],
            )
        };
        items.push(PresentationChoiceItem::Sentinel);
        let labels = labels
            .iter()
            .map(|label| menu_label(text, label))
            .collect::<Vec<_>>();
        let label_refs = labels.iter().map(Box::as_ref).collect::<Vec<_>>();
        let cancel_label = menu_label(text, text.cancel_label());
        let fonts = services.runtime().data().font_resources().clone();
        let pointer = choice_pointer(services, primary_pointer_pressed);
        let current_hand_animation = services.manu3_hand_state().current_animation;
        let mut backend = RuntimeChoiceListBackend::new(
            services.runtime_mut(),
            &fonts,
            pointer,
            current_hand_animation,
        );

        let layout_pending = self.text_speed.state.phase & PRESENTATION_CHOICE_LAYOUT_PHASE != 0;
        if layout_pending {
            self.text_speed.current_rect = update_choice_list(
                &label_refs,
                text_speed_list_config(&cancel_label, true),
                &mut self.text_speed.list,
                &mut backend,
            )
            .rect;
        }
        let transition = self.text_speed.state.transition;
        let interaction_ready = !layout_pending
            && (self.text_speed.state.phase & PRESENTATION_CHOICE_TRANSITION_PHASE == 0
                || transition.current_step == transition.total_steps);
        let frame = interaction_ready.then(|| {
            update_choice_list(
                &label_refs,
                text_speed_list_config(&cancel_label, false),
                &mut self.text_speed.list,
                &mut backend,
            )
        });
        let selected = frame.as_ref().and_then(|frame| {
            frame
                .selected_item
                .or_else(|| frame.cancelled.then_some(labels.len()))
        });
        let outcome = update_presentation_choice(
            &mut self.text_speed.state,
            &items,
            selected,
            transition_rect(self.text_speed.current_rect),
            transition_rect(self.text_speed.animation_target),
        )?;
        if self.options.simulation_options_active {
            self.options.simulation_options_phase = self.text_speed.state.phase;
        } else {
            self.options.text_options_phase = self.text_speed.state.phase;
        }
        if let PresentationChoiceOutcome::Transitioning { region, .. } = outcome {
            backend.darken_region(
                RasterPoint {
                    x: i32::from(region.x),
                    y: i32::from(region.y),
                },
                region.width,
                region.height,
            );
        }
        let hand_requests = backend.take_hand_requests();
        backend.finish()?;
        drop(backend);
        services.apply_choice_list_hand_requests(hand_requests);

        if let Some(frame) = &frame {
            draw_runtime_choice_rows(services, &label_refs, Some(&cancel_label), frame)?;
        }
        if let PresentationChoiceOutcome::Closed {
            published_result, ..
        } = outcome
        {
            if let Some(step) = published_result {
                if self.options.simulation_options_active {
                    services.set_sequel_simulation_speed(step)?;
                } else {
                    services.set_dialogue_word_delay(step)?;
                }
            }
            self.options.common.text_options_active = false;
            self.options.simulation_options_active = false;
        }
        Ok(())
    }

    fn apply_deferred_record(&mut self, services: &mut ModernGameServices<'_>) -> Result<()> {
        let Some(deferred) = self.deferred.record.take() else {
            return Ok(());
        };
        if self.deferred.redraw_requested {
            self.deferred.redraw_requested = false;
            services.request_scene_transition(deferred.record)
        } else {
            match deferred.action {
                BridgeDeferredActionKind::PresentationQueue => {
                    services.defer_ship_presentation_queue(deferred.record);
                }
            }
            Ok(())
        }
    }

    fn speed_menu_active(&self) -> bool {
        self.options.common.text_options_active || self.options.simulation_options_active
    }

    fn publish_interface_ownership(&self, lifecycle: &mut GameLifecycleState) {
        let active = self.console.selected.is_some() || self.speed_menu_active();
        lifecycle.profile_change_blockers.navigation_choice_active = active;
        lifecycle.set_navigation_ui_busy(self.console.interface_busy);
    }
}

fn synchronize_console_seek(state: &mut BridgeConsoleState, seek_requested: bool) {
    if state.interface_busy {
        state.interface_busy = seek_requested;
    }
}

fn import_text_speed_modal_ui(state: &mut PresentationChoiceState, lifecycle: &GameLifecycleState) {
    state.ui_flags = (state.ui_flags & !PRESENTATION_CHOICE_MODAL_UI_FLAG)
        | if lifecycle.modal_ui_busy() {
            PRESENTATION_CHOICE_MODAL_UI_FLAG
        } else {
            u8::MIN
        };
}

fn publish_console_modal_ui(
    lifecycle: &mut GameLifecycleState,
    console_owned_before_update: bool,
    state: &BridgeConsoleState,
) {
    if console_owned_before_update || state.selected.is_some() {
        lifecycle.set_modal_ui_busy(state.interface_active);
    }
}

fn export_text_speed_modal_ui(state: &PresentationChoiceState, lifecycle: &mut GameLifecycleState) {
    lifecycle.set_modal_ui_busy(state.ui_flags & PRESENTATION_CHOICE_MODAL_UI_FLAG != u8::MIN);
}

struct RuntimeTextSpeedMenu {
    state: PresentationChoiceState,
    list: ChoiceListState,
    current_rect: ChoiceListRect,
    animation_target: ChoiceListRect,
}

impl RuntimeTextSpeedMenu {
    fn new(initial_step: u16) -> Self {
        Self {
            state: PresentationChoiceState {
                activation_flags: u8::MIN,
                phase: u8::MIN,
                transition: FramebufferTransitionState::default(),
                layout_only: false,
                ui_flags: u8::MIN,
                result: initial_step,
            },
            list: ChoiceListState::default(),
            current_rect: ChoiceListRect::default(),
            animation_target: ChoiceListRect::default(),
        }
    }

    fn begin(&mut self, current_step: u16, animation_target: ChoiceListRect) {
        self.state.activation_flags = PRESENTATION_CHOICE_ACTIVE_FLAG;
        self.state.phase = PRESENTATION_CHOICE_LAYOUT_PHASE;
        self.state.transition = FramebufferTransitionState::default();
        self.state.result = current_step;
        self.animation_target = animation_target;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RuntimeBridgeBackendEffects {
    reload_radio_sound_bank: bool,
    start_music_stream: bool,
}

struct RuntimeBridgeChoiceBackend<'runtime> {
    list: RuntimeChoiceListBackend<'runtime>,
    transition: &'runtime mut FramebufferTransitionState,
    effects: RuntimeBridgeBackendEffects,
}

impl RuntimeBridgeChoiceBackend<'_> {
    fn new<'runtime>(
        runtime: &'runtime mut OriginalGameRuntime,
        fonts: &'runtime commander_blood_formats::bloodprg::BloodprgFontResources,
        pointer: ChoiceListPointer,
        current_hand_animation: u16,
        transition: &'runtime mut FramebufferTransitionState,
    ) -> RuntimeBridgeChoiceBackend<'runtime> {
        RuntimeBridgeChoiceBackend {
            list: RuntimeChoiceListBackend::new(runtime, fonts, pointer, current_hand_animation),
            transition,
            effects: RuntimeBridgeBackendEffects::default(),
        }
    }

    fn effects(&self) -> RuntimeBridgeBackendEffects {
        self.effects
    }

    fn finish(&mut self) -> Result<()> {
        self.list.finish()
    }

    fn take_hand_requests(&mut self) -> Vec<ChoiceListHandRequest> {
        self.list.take_hand_requests()
    }
}

impl ChoiceListBackend for RuntimeBridgeChoiceBackend<'_> {
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

impl BridgeChoiceBackend for RuntimeBridgeChoiceBackend<'_> {
    fn advance_panel_transition(&mut self, source: ChoiceListRect, target: ChoiceListRect) -> bool {
        if self.transition.total_steps == u8::MIN {
            self.transition.total_steps = PANEL_TRANSITION_STEP_COUNT;
            self.transition.current_step = u8::MIN;
        }
        match advance_framebuffer_rect_transition(
            self.transition,
            transition_rect(source),
            transition_rect(target),
        ) {
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
                self.list.record_error(Err(error.into()));
                false
            }
        }
    }

    fn reload_radio_sound_bank(&mut self) {
        self.effects.reload_radio_sound_bank = true;
    }

    fn start_music_stream(&mut self) {
        self.effects.start_music_stream = true;
    }
}

fn navigation_choices(
    services: &ModernGameServices<'_>,
) -> Result<Vec<BridgeRecordChoice<ScriptObjectId>>> {
    let profile = services
        .runtime()
        .current_profile()
        .context("navigation choices require a loaded BloodScript profile")?;
    let builtins = profile.builtins();
    let horn = builtins.horn.context("loaded profile has no Honk object")?;
    let radio = builtins.menu.context("loaded profile has no menu object")?;
    navigation_actor_targets(profile.state(), horn, radio)
        .into_iter()
        .map(|record| record_choice(profile, record))
        .collect()
}

fn contact_choices(
    services: &ModernGameServices<'_>,
) -> Result<Vec<Option<BridgeRecordChoice<ScriptObjectId>>>> {
    let profile = services
        .runtime()
        .current_profile()
        .context("contact choices require a loaded BloodScript profile")?;
    profile
        .record_state()
        .record_runtime
        .aboard_objects()
        .slots()
        .iter()
        .copied()
        .map(|record| {
            record
                .map(|record| record_choice(profile, record))
                .transpose()
        })
        .collect()
}

fn record_choice(
    profile: &crate::native::bloodprg::LoadedScriptProfile,
    record: ScriptObjectId,
) -> Result<BridgeRecordChoice<ScriptObjectId>> {
    let label = profile
        .directory()
        .object(record)
        .with_context(|| format!("bridge record {record:?} has no directory entry"))?
        .name();
    if label.is_empty() {
        anyhow::bail!("bridge record {record:?} has an empty directory label");
    }
    Ok(BridgeRecordChoice::new(record, label))
}

fn required_builtin(
    services: &ModernGameServices<'_>,
    select: impl FnOnce(crate::native::bloodprg::ScriptProfileBuiltins) -> Option<ScriptObjectId>,
    name: &str,
) -> Result<ScriptObjectId> {
    let profile = services
        .runtime()
        .current_profile()
        .context("bridge command requires a loaded BloodScript profile")?;
    select(profile.builtins()).with_context(|| format!("loaded profile has no {name} object"))
}

fn option_labels(
    text: &BloodprgBridgeMenuText,
    music_label: MusicOptionLabel,
    travel_enabled: bool,
) -> Vec<Box<[u8]>> {
    let mut labels = text.option_labels().to_vec();
    if music_label == MusicOptionLabel::MusicOn {
        labels[text.music_option_row()] = text.music_on_label().into();
    }
    if travel_enabled && let Some(controls) = text.sequel_controls() {
        labels[2] = controls.travel_on_label.clone();
    }
    labels.iter().map(|label| menu_label(text, label)).collect()
}

fn menu_label(text: &BloodprgBridgeMenuText, label: &[u8]) -> Box<[u8]> {
    if text.sequel_controls().is_none() {
        return label.into();
    }
    // Only decoded sequel menu labels enter this display-only mapping.
    let english: &[u8] = match label {
        b"VITESSE" => b"SIMULATION_SPEED",
        b"TEXTES" => b"TEXT_SPEED",
        b"VOYAGE_OFF" => b"TRAVEL_OFF",
        b"VOYAGE_ON" => b"TRAVEL_ON",
        b"MUSIQUE_OFF" => b"MUSIC_OFF",
        b"MUSIQUE_ON" => b"MUSIC_ON",
        b"SAUVER" => b"SAVE",
        b"CHARGER" => b"LOAD",
        b"QUITTER" => b"QUIT",
        b"ANNULER" => b"CANCEL",
        b"TRES_RAPIDE" => b"VERY_FAST",
        b"RAPIDE" => b"FAST",
        b"LENT" => b"SLOW",
        b"TRES_LENT" => b"VERY_SLOW",
        _ => label,
    };
    english.into()
}

const fn bridge_console_choice_name(choice: BridgeConsoleChoice) -> &'static str {
    match choice {
        BridgeConsoleChoice::Horn => "horn",
        BridgeConsoleChoice::Navigation => "navigation",
        BridgeConsoleChoice::Contacts => "contacts",
        BridgeConsoleChoice::Radio => "radio",
        BridgeConsoleChoice::Options => "options",
    }
}

const fn bridge_choice_panel_phase_name(
    phase: crate::native::bloodprg::BridgeChoicePanelPhase,
) -> &'static str {
    use crate::native::bloodprg::BridgeChoicePanelPhase;

    match phase {
        BridgeChoicePanelPhase::Closed => "closed",
        BridgeChoicePanelPhase::NeedsLayout => "needs_layout",
        BridgeChoicePanelPhase::Transitioning => "transitioning",
        BridgeChoicePanelPhase::Interactive => "interactive",
    }
}

fn record_choice_trace(
    choices: &[BridgeRecordChoice<ScriptObjectId>],
) -> (Vec<usize>, Vec<String>) {
    (
        choices.iter().map(|choice| choice.record.index()).collect(),
        choices
            .iter()
            .map(|choice| String::from_utf8_lossy(&choice.label).into_owned())
            .collect(),
    )
}

fn selected_row_rect(target_y: u16) -> ChoiceListRect {
    ChoiceListRect {
        origin: [i16::default(), target_y as i16],
        size: [u16::MIN, u16::MIN],
    }
}

fn text_speed_list_config(cancel_label: &[u8], layout_only: bool) -> ChoiceListConfig<'_> {
    ChoiceListConfig {
        center_x: PANEL_CENTER_X,
        preserve_individual_widths: true,
        cancel_label: Some(cancel_label),
        layout_only,
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

fn choice_pointer(
    services: &ModernGameServices<'_>,
    primary_pointer_pressed: bool,
) -> ChoiceListPointer {
    let pointer = services.input().pointer_sample();
    edge_choice_pointer(pointer.position, primary_pointer_pressed)
}

const fn edge_choice_pointer(
    position: [i16; 2],
    primary_pointer_pressed: bool,
) -> ChoiceListPointer {
    ChoiceListPointer {
        position,
        primary_pressed: primary_pointer_pressed,
    }
}

fn activate_quit_confirmation(lifecycle: &mut GameLifecycleState, navigation_choice_gate: &mut u8) {
    lifecycle.primary_pointer_pressed = false;
    lifecycle.pointer_press_pending = u8::MIN;
    *navigation_choice_gate = 2;
    lifecycle.set_modal_ui_busy(true);
}

fn draw_runtime_choice_rows(
    services: &mut ModernGameServices<'_>,
    labels: &[&[u8]],
    cancel_label: Option<&[u8]>,
    frame: &ChoiceListFrame,
) -> Result<()> {
    draw_choice_list_rows(services.runtime_mut(), labels, cancel_label, frame)
}

fn apply_console_palette(runtime: &mut OriginalGameRuntime, plan: BridgeConsolePalettePlan) {
    let first = usize::from(plan.first_index);
    for (entry, color) in runtime.live_palette_mut()[first..]
        .iter_mut()
        .zip(plan.rows)
    {
        *entry = [color.red, color.green, color.blue];
    }
}

fn apply_backend_effects(
    services: &mut ModernGameServices<'_>,
    effects: RuntimeBridgeBackendEffects,
) -> Result<()> {
    if effects.reload_radio_sound_bank {
        services.load_radio_sound_bank()?;
    }
    if effects.start_music_stream {
        services.restart_navigation_music()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the original Big Bug Bang executable"]
    fn sequel_menu_music_toggle_preserves_the_other_six_rows() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
        let bytes = std::fs::read(path).unwrap();
        let text = crate::game::GameVariant::BigBugBang
            .decode_bridge_menu_text(&bytes)
            .unwrap();
        let enabled = option_labels(&text, MusicOptionLabel::MusicOff, false);
        let disabled = option_labels(&text, MusicOptionLabel::MusicOn, false);
        assert_eq!(enabled.len(), 7);
        assert_eq!(enabled[3].as_ref(), b"MUSIC_OFF");
        assert_eq!(disabled[3].as_ref(), b"MUSIC_ON");
        for row in [0, 1, 2, 4, 5, 6] {
            assert_eq!(enabled[row], disabled[row]);
        }
        assert_eq!(text.option_labels()[3].as_ref(), b"MUSIQUE_OFF");
        assert_eq!(text.option_labels()[2].as_ref(), b"VOYAGE_OFF");
        let travel_on = option_labels(&text, MusicOptionLabel::MusicOff, true);
        assert_eq!(travel_on[2].as_ref(), b"TRAVEL_ON");
        for row in [0, 1, 3, 4, 5, 6] {
            assert_eq!(enabled[row], travel_on[row]);
        }
    }

    #[test]
    fn selected_console_rows_open_from_the_native_zero_width_anchor() {
        assert_eq!(
            selected_row_rect(116),
            ChoiceListRect {
                origin: [0, 116],
                size: [0, 0],
            }
        );
    }

    #[test]
    fn text_speed_menu_starts_with_the_recovered_layout_phase() {
        let mut menu = RuntimeTextSpeedMenu::new(2);
        let target = ChoiceListRect {
            origin: [40, 60],
            size: [120, 70],
        };
        menu.begin(7, target);

        assert_eq!(menu.state.activation_flags, PRESENTATION_CHOICE_ACTIVE_FLAG);
        assert_eq!(menu.state.phase, PRESENTATION_CHOICE_LAYOUT_PHASE);
        assert_eq!(menu.state.result, 7);
        assert_eq!(menu.animation_target, target);
    }

    #[test]
    fn text_speed_choice_synchronizes_the_native_modal_ui_bit() {
        let mut state = RuntimeTextSpeedMenu::new(u16::MIN).state;
        state.ui_flags = u8::MAX & !PRESENTATION_CHOICE_MODAL_UI_FLAG;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);

        import_text_speed_modal_ui(&mut state, &lifecycle);
        assert_ne!(state.ui_flags & PRESENTATION_CHOICE_MODAL_UI_FLAG, u8::MIN);

        state.ui_flags &= !PRESENTATION_CHOICE_MODAL_UI_FLAG;
        export_text_speed_modal_ui(&state, &mut lifecycle);
        assert!(!lifecycle.modal_ui_busy());
    }

    #[test]
    fn selected_console_command_owns_the_native_modal_ui_bit() {
        let mut lifecycle = GameLifecycleState::default();
        let active = BridgeConsoleState {
            selected: Some(BridgeConsoleChoice::Contacts),
            interface_active: true,
            ..BridgeConsoleState::default()
        };

        publish_console_modal_ui(&mut lifecycle, false, &active);
        assert!(lifecycle.modal_ui_busy());

        let closed = BridgeConsoleState::default();
        publish_console_modal_ui(&mut lifecycle, true, &closed);
        assert!(!lifecycle.modal_ui_busy());
    }

    #[test]
    fn inactive_console_does_not_clear_another_modal_ui_owner() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);

        publish_console_modal_ui(&mut lifecycle, false, &BridgeConsoleState::default());

        assert!(lifecycle.modal_ui_busy());
    }

    #[test]
    fn interactive_console_keeps_profile_ownership_without_relocking_the_pointer() {
        let mut console = RuntimeBridgeConsole::new(u16::MIN, false);
        console.console.selected = Some(BridgeConsoleChoice::Options);
        console.console.interface_active = true;
        console.console.interface_busy = false;
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_navigation_ui_busy(true);

        console.publish_interface_ownership(&mut lifecycle);

        assert!(lifecycle.profile_change_blockers.navigation_choice_active);
        assert!(!lifecycle.navigation_ui_busy());

        console.console.interface_busy = true;
        console.publish_interface_ownership(&mut lifecycle);
        assert!(lifecycle.navigation_ui_busy());
    }

    #[test]
    fn cleared_selection_alias_does_not_retain_a_completed_seek() {
        let mut console = BridgeConsoleState {
            selected: None,
            interface_busy: true,
            ..BridgeConsoleState::default()
        };

        synchronize_console_seek(&mut console, false);

        assert!(!console.interface_busy);
    }

    #[test]
    fn held_pointer_level_cannot_replace_the_recovered_press_edge() {
        let pointer = edge_choice_pointer([120, 80], false);

        assert_eq!(pointer.position, [120, 80]);
        assert!(!pointer.primary_pressed);
    }

    #[test]
    fn quit_confirmation_consumes_the_click_that_opened_it() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.primary_pointer_pressed = true;
        lifecycle.pointer_press_pending = 1;
        let mut navigation_choice_gate = u8::MIN;

        activate_quit_confirmation(&mut lifecycle, &mut navigation_choice_gate);

        assert!(!lifecycle.primary_pointer_pressed);
        assert_eq!(lifecycle.pointer_press_pending, u8::MIN);
        assert_eq!(navigation_choice_gate, 2);
        assert!(lifecycle.modal_ui_busy());
    }

    #[test]
    fn text_speed_list_retains_the_executable_cancel_row() {
        let config = text_speed_list_config(b"CANCEL", false);

        assert_eq!(config.cancel_label, Some(b"CANCEL".as_slice()));
        assert!(!config.layout_only);
    }
}
