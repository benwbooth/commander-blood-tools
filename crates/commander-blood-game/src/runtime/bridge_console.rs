//! Production bridge-console interaction over flat runtime state.

use anyhow::{Context, Result};
use commander_blood_formats::lbm::PALETTE_ENTRY_COUNT;
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    BridgeChoiceBackend, BridgeConsoleChoice, BridgeConsoleContext, BridgeConsoleDispatchOutcome,
    BridgeConsolePalettePlan, BridgeConsoleState, BridgeDeferredState, BridgeRecordChoice,
    BridgeRecordChoiceContext, BridgeRecordChoiceOutcome, BridgeRecordChoiceState,
    ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListPointer, ChoiceListRect,
    ChoiceListState, FramebufferTransitionState, GameLifecycleState, ImmediateBridgeChoiceOutcome,
    MusicOptionLabel, OptionMenuChoice, OptionMenuOutcome, OptionMenuState, PresentationChoiceItem,
    PresentationChoiceOutcome, PresentationChoiceState, RasterPoint, TransitionRect,
    activate_horn_choice, activate_radio_choice, advance_framebuffer_rect_transition,
    build_banked_tint_table, navigation_actor_targets, update_bridge_console_dispatch,
    update_choice_list, update_contact_choice, update_navigation_target_choice, update_option_menu,
    update_presentation_choice,
};

use super::choice_list::{RuntimeChoiceListBackend, draw_choice_list_rows};
use super::{ModernGameServices, OriginalGameRuntime};

const BRIDGE_CONSOLE_SELECTION_CLIP: u8 = 4;
const PANEL_TRANSITION_STEP_COUNT: u8 = 10;
const CONSOLE_TINT_FIRST_INDEX: u8 = 224;
const PRESENTATION_CHOICE_ACTIVE_FLAG: u8 = 1;
const PRESENTATION_CHOICE_LAYOUT_PHASE: u8 = 1;
const PRESENTATION_CHOICE_TRANSITION_PHASE: u8 = 1 << 1;
const OPTION_MENU_LABEL_COUNT: usize = 5;
const TEXT_SPEED_LABEL_COUNT: usize = 5;
const PANEL_CENTER_X: i16 = 100;

/// Persistent production state for the recovered five-command bridge console.
pub(super) struct RuntimeBridgeConsole {
    console: BridgeConsoleState,
    navigation: BridgeRecordChoiceState<ScriptObjectId>,
    contacts: BridgeRecordChoiceState<ScriptObjectId>,
    deferred: BridgeDeferredState<ScriptObjectId>,
    options: OptionMenuState,
    text_speed: RuntimeTextSpeedMenu,
    transition: FramebufferTransitionState,
}

impl RuntimeBridgeConsole {
    pub(super) fn new(initial_text_speed_step: u16) -> Self {
        Self {
            console: BridgeConsoleState::default(),
            navigation: BridgeRecordChoiceState::default(),
            contacts: BridgeRecordChoiceState::default(),
            deferred: BridgeDeferredState {
                record: None,
                redraw_requested: false,
            },
            options: OptionMenuState::default(),
            text_speed: RuntimeTextSpeedMenu::new(initial_text_speed_step),
            transition: FramebufferTransitionState::default(),
        }
    }

    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.options.text_options_active {
            self.publish_interface_ownership(lifecycle);
            return Ok(());
        }

        if self.console.selected.is_some() && self.console.interface_busy {
            self.console.interface_busy = services.bridge_seek_requested()?;
        }
        let pointer = services.input().pointer_sample();
        let outcome = update_bridge_console_dispatch(
            BridgeConsoleContext {
                aboard_transfer_pending: lifecycle.presentation.c2_presentation_gate,
                save_motion_active: lifecycle.profile_change_blockers.save_active,
                load_motion_active: lifecycle.profile_change_blockers.load_active,
                option_panel_active: self.options.text_options_active,
                sound_action_active: lifecycle.modal_ui_busy(),
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
        Ok(())
    }

    pub(super) fn update_presentation_choice(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.options.text_options_active {
            self.update_text_speed_menu(services)?;
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
            BridgeConsoleChoice::Navigation => self.update_navigation_menu(services)?,
            BridgeConsoleChoice::Contacts => self.update_contact_menu(services)?,
            BridgeConsoleChoice::Radio => self.activate_radio(services)?,
            BridgeConsoleChoice::Options => self.update_option_menu(services, lifecycle)?,
        }
        self.apply_deferred_record(services)
    }

    fn update_navigation_menu(&mut self, services: &mut ModernGameServices<'_>) -> Result<()> {
        let choices = navigation_choices(services)?;
        let cancel_label: Box<[u8]> = services
            .runtime()
            .data()
            .bridge_menu_text()
            .cancel_label()
            .into();
        let context = BridgeRecordChoiceContext {
            animation_target: selected_row_rect(self.console.panel_target_y),
            cancel_label: &cancel_label,
        };
        let fonts = services.runtime().data().font_resources().clone();
        let tint = choice_tint(services.runtime())?;
        let pointer = choice_pointer(services);
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            &tint,
            pointer,
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
        backend.finish()?;
        drop(backend);

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

    fn update_contact_menu(&mut self, services: &mut ModernGameServices<'_>) -> Result<()> {
        let choices = contact_choices(services)?;
        let cancel_label: Box<[u8]> = services
            .runtime()
            .data()
            .bridge_menu_text()
            .cancel_label()
            .into();
        let context = BridgeRecordChoiceContext {
            animation_target: selected_row_rect(self.console.panel_target_y),
            cancel_label: &cancel_label,
        };
        let fonts = services.runtime().data().font_resources().clone();
        let tint = choice_tint(services.runtime())?;
        let pointer = choice_pointer(services);
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            &tint,
            pointer,
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
        backend.finish()?;
        drop(backend);

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

    fn activate_radio(&mut self, services: &mut ModernGameServices<'_>) -> Result<()> {
        let record = required_builtin(services, |builtins| builtins.menu, "menu")?;
        let fonts = services.runtime().data().font_resources().clone();
        let tint = choice_tint(services.runtime())?;
        let pointer = choice_pointer(services);
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            &tint,
            pointer,
            &mut self.transition,
        );
        let outcome =
            activate_radio_choice(record, &mut self.console, &mut self.deferred, &mut backend);
        let effects = backend.effects();
        backend.finish()?;
        debug_assert_eq!(outcome, ImmediateBridgeChoiceOutcome::Activated);
        apply_backend_effects(services, effects)
    }

    fn update_option_menu(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        let music_active = services.navigation_music_position()?.is_some();
        self.options.music_supported = services.audio_is_initialized();
        self.options.music_active = music_active;
        self.options.music_label = if music_active {
            MusicOptionLabel::MusicOff
        } else {
            MusicOptionLabel::MusicOn
        };
        let labels = option_labels(services, self.options.music_label);
        let label_refs = labels.iter().map(Box::as_ref).collect::<Vec<_>>();
        let fonts = services.runtime().data().font_resources().clone();
        let tint = choice_tint(services.runtime())?;
        let pointer = choice_pointer(services);
        let mut backend = RuntimeBridgeChoiceBackend::new(
            services.runtime_mut(),
            &fonts,
            &tint,
            pointer,
            &mut self.transition,
        );
        let outcome = update_option_menu(
            &label_refs,
            selected_row_rect(self.console.panel_target_y),
            &mut self.console,
            &mut self.options,
            &mut backend,
        );
        let effects = backend.effects();
        backend.finish()?;
        drop(backend);

        if let OptionMenuOutcome::Interactive(frame) = &outcome {
            draw_runtime_choice_rows(services, &label_refs, None, frame)?;
        }
        if music_active && !self.options.music_active {
            services.stop_navigation_music()?;
        }
        apply_backend_effects(services, effects)?;

        if let OptionMenuOutcome::Selected(choice) = outcome {
            match choice {
                OptionMenuChoice::Text => {
                    self.text_speed
                        .begin(services.dialogue_word_delay()?, self.options.current_rect);
                }
                OptionMenuChoice::Music => {}
                OptionMenuChoice::Save => {
                    self.options.save_motion_requested = false;
                    services.request_save_menu()?;
                }
                OptionMenuChoice::Load => {
                    self.options.load_motion_requested = false;
                    services.request_load_menu()?;
                }
                OptionMenuChoice::Quit => {
                    self.options.quit_requested = false;
                    services.confirm_dialog_state_mut().navigation_choice_gate = 2;
                    lifecycle.set_modal_ui_busy(true);
                }
            }
        }
        Ok(())
    }

    fn update_text_speed_menu(&mut self, services: &mut ModernGameServices<'_>) -> Result<()> {
        let labels = services
            .runtime()
            .data()
            .bridge_menu_text()
            .text_speed_labels()
            .clone();
        let label_refs = labels.iter().map(Box::as_ref).collect::<Vec<_>>();
        let fonts = services.runtime().data().font_resources().clone();
        let tint = choice_tint(services.runtime())?;
        let pointer = choice_pointer(services);
        let mut backend =
            RuntimeChoiceListBackend::new(services.runtime_mut(), &fonts, &tint, pointer);

        let layout_pending = self.text_speed.state.phase & PRESENTATION_CHOICE_LAYOUT_PHASE != 0;
        if layout_pending {
            self.text_speed.current_rect = update_choice_list(
                &label_refs,
                text_speed_list_config(true),
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
                text_speed_list_config(false),
                &mut self.text_speed.list,
                &mut backend,
            )
        });
        let selected = frame.as_ref().and_then(|frame| frame.selected_item);
        let items = [PresentationChoiceItem::Selectable; TEXT_SPEED_LABEL_COUNT];
        let outcome = update_presentation_choice(
            &mut self.text_speed.state,
            &items,
            selected,
            transition_rect(self.text_speed.current_rect),
            transition_rect(self.text_speed.animation_target),
        )?;
        if let PresentationChoiceOutcome::Transitioning { region, .. } = outcome {
            backend.remap_region(
                RasterPoint {
                    x: i32::from(region.x),
                    y: i32::from(region.y),
                },
                region.width,
                region.height,
            );
        }
        backend.finish()?;
        drop(backend);

        if let Some(frame) = &frame {
            draw_runtime_choice_rows(services, &label_refs, None, frame)?;
        }
        if let PresentationChoiceOutcome::Closed {
            published_result: Some(step),
            ..
        } = outcome
        {
            services.set_dialogue_word_delay(step)?;
            self.options.text_options_active = false;
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
            services.defer_ship_actor_presentation(deferred.record);
            Ok(())
        }
    }

    fn publish_interface_ownership(&self, lifecycle: &mut GameLifecycleState) {
        let active = self.console.selected.is_some() || self.options.text_options_active;
        lifecycle.profile_change_blockers.navigation_choice_active = active;
        lifecycle.set_navigation_ui_busy(active);
    }
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
        tint: &'runtime [u8; PALETTE_ENTRY_COUNT],
        pointer: ChoiceListPointer,
        transition: &'runtime mut FramebufferTransitionState,
    ) -> RuntimeBridgeChoiceBackend<'runtime> {
        RuntimeBridgeChoiceBackend {
            list: RuntimeChoiceListBackend::new(runtime, fonts, tint, pointer),
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
    services: &ModernGameServices<'_>,
    music_label: MusicOptionLabel,
) -> [Box<[u8]>; OPTION_MENU_LABEL_COUNT] {
    let text = services.runtime().data().bridge_menu_text();
    let mut labels = text.option_labels().clone();
    if music_label == MusicOptionLabel::MusicOn {
        labels[1] = text.music_on_label().into();
    }
    labels
}

fn selected_row_rect(target_y: u16) -> ChoiceListRect {
    ChoiceListRect {
        origin: [i16::default(), target_y as i16],
        size: [u16::MIN, u16::MIN],
    }
}

fn text_speed_list_config(layout_only: bool) -> ChoiceListConfig<'static> {
    ChoiceListConfig {
        center_x: PANEL_CENTER_X,
        preserve_individual_widths: true,
        cancel_label: None,
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

fn choice_pointer(services: &ModernGameServices<'_>) -> ChoiceListPointer {
    let pointer = services.input().pointer_sample();
    ChoiceListPointer {
        position: pointer.position,
        primary_pressed: pointer
            .buttons
            .contains(crate::native::bloodprg::PointerButton::Primary),
    }
}

fn choice_tint(runtime: &OriginalGameRuntime) -> Result<[u8; PALETTE_ENTRY_COUNT]> {
    let mut tint = [u8::MIN; PALETTE_ENTRY_COUNT];
    build_banked_tint_table(runtime.live_palette(), &mut tint, CONSOLE_TINT_FIRST_INDEX)
        .context("building the bridge-console tint table")?;
    Ok(tint)
}

fn draw_runtime_choice_rows(
    services: &mut ModernGameServices<'_>,
    labels: &[&[u8]],
    cancel_label: Option<&[u8]>,
    frame: &ChoiceListFrame,
) -> Result<()> {
    let fonts = services.runtime().data().font_resources().clone();
    draw_choice_list_rows(services.runtime_mut(), &fonts, labels, cancel_label, frame)
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
}
