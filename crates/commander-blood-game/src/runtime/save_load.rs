//! Concrete save/load menu, editor, codec, and writable-root adapter.

use anyhow::{Context, Result};
use commander_blood_formats::lbm::PALETTE_ENTRY_COUNT;

use crate::native::bloodprg::{
    BridgeSpriteRect, ChoiceListConfig, ChoiceListRect, ChoiceListState, FontPoint,
    FontVerticalBand, FramebufferTransitionState, GameLifecycleState, InputAction,
    OriginalSaveGame, OriginalSaveSlotDirectory, RasterPoint, SaveLoadHost, SaveLoadListPass,
    SaveLoadMenuOutcome, SaveLoadMenuState, SaveLoadSelection, SaveMenuState, SaveProfileBackend,
    SaveSlotEditorLayout, SaveSlotEditorOutcome, SaveSlotName, TransitionRect,
    advance_framebuffer_rect_transition, build_banked_tint_table, draw_square_caps_text,
    fill_framebuffer_rect, move_input_selection_next, move_input_selection_previous,
    remap_framebuffer_rect, update_save_load_menu, update_save_slot_editor,
};

use super::choice_list::RuntimeChoiceListStyle;
use super::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, ModernGameServices};

const SAVE_SLOT_DIRECTORY_NAME: &[u8] = b"BLOOD.SAV";
const SAVE_LIST_CANCEL_LABEL: &[u8] = b"CANCEL";
const SAVE_EDITOR_ENTER_KEY: u8 = b'\r';
const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;
const SAVE_LIST_TRANSITION_TARGET: ChoiceListRect = ChoiceListRect {
    origin: [100, 0],
    size: [0, 120],
};
const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i32,
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32,
};
const FULL_LOGICAL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32 - 1,
};

/// Persistent state for the exact save/load coordinator and its shared list widget.
pub struct RuntimeSaveLoad {
    state: SaveLoadMenuState,
    choice_list: ChoiceListState,
    current_rect: ChoiceListRect,
    target_rect: ChoiceListRect,
    pending_input: Option<InputAction>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SaveLoadFrameEffects {
    redraw_requested: bool,
    palette_upload_requested: bool,
}

impl Default for RuntimeSaveLoad {
    fn default() -> Self {
        Self {
            state: SaveLoadMenuState::default(),
            choice_list: ChoiceListState::default(),
            current_rect: ChoiceListRect::default(),
            target_rect: SAVE_LIST_TRANSITION_TARGET,
            pending_input: None,
        }
    }
}

impl RuntimeSaveLoad {
    /// Borrow the recovered coordinator state.
    pub const fn state(&self) -> &SaveLoadMenuState {
        &self.state
    }

    /// Request the ordinary save-slot editor.
    pub fn request_save(&mut self) {
        self.state.request_save();
    }

    /// Request the ordinary load-slot selector.
    pub fn request_load(&mut self) {
        self.state.request_load();
    }

    /// Request an immediate save to the original reserved tenth slot.
    pub fn request_quick_save(&mut self) {
        self.state.request_quick_save();
    }

    /// Queue one translated key action when save/load currently owns input.
    pub fn queue_input(&mut self, action: InputAction) -> bool {
        if !self.state.is_active() {
            return false;
        }
        self.pending_input = Some(action);
        true
    }

    /// Advance one exact save/load frame over flat state and the writable data root.
    pub fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<SaveLoadMenuOutcome> {
        let editor_key = self.apply_pending_input(services)?;
        let mut directory = services
            .runtime()
            .save_slots()
            .cloned()
            .context("save/load update requires the loaded BLOOD.SAV directory")?;
        let profile_snapshot = services.runtime().current_profile().cloned();
        let style = services.choice_list_style();
        let mut profiles = DeferredSaveProfileBackend {
            profile_snapshot,
            restore_data: None,
        };
        let outcome = {
            let mut backend = RuntimeSaveLoadBackend {
                services,
                choice_list: &mut self.choice_list,
                current_rect: &mut self.current_rect,
                target_rect: self.target_rect,
                style,
                primary_pointer_pressed: lifecycle.primary_pointer_pressed,
                editor_key,
                created_filename: None,
            };
            update_save_load_menu(&mut self.state, &mut directory, &mut backend, &mut profiles)
                .map_err(anyhow::Error::new)?
        };

        *services
            .runtime_mut()
            .save_slots_mut()
            .context("loaded save-slot directory disappeared during update")? = directory;
        if let Some(data) = profiles.restore_data {
            services.restore_original_save_game(&data, lifecycle)?;
        }

        lifecycle.profile_change_blockers.save_active = self.state.requests.save;
        lifecycle.profile_change_blockers.load_active = self.state.requests.load;
        lifecycle.set_modal_ui_busy(self.state.requests.save || self.state.requests.load);
        let effects = self.take_frame_effects();
        if effects.redraw_requested {
            lifecycle.navigation_rebuild_pending = true;
        }
        if effects.palette_upload_requested {
            services
                .palette_transition_mut()
                .request_visual_color_update();
        }
        Ok(outcome)
    }

    fn take_frame_effects(&mut self) -> SaveLoadFrameEffects {
        SaveLoadFrameEffects {
            redraw_requested: std::mem::take(&mut self.state.redraw_pending),
            palette_upload_requested: std::mem::take(&mut self.state.palette_dirty),
        }
    }

    fn apply_pending_input(&mut self, services: &ModernGameServices<'_>) -> Result<Option<u8>> {
        let Some(action) = self.pending_input.take() else {
            return Ok(None);
        };
        match action {
            InputAction::Accept => Ok(Some(SAVE_EDITOR_ENTER_KEY)),
            InputAction::LatchTextByte(byte) | InputAction::TogglePause(byte) => Ok(Some(byte)),
            InputAction::Cancel => {
                self.state.cancel();
                Ok(None)
            }
            InputAction::MovePrevious | InputAction::MoveNext => {
                self.move_selection(services, action)?;
                Ok(None)
            }
            InputAction::Ignored(_) => Ok(None),
        }
    }

    fn move_selection(
        &mut self,
        services: &ModernGameServices<'_>,
        action: InputAction,
    ) -> Result<()> {
        if !self.state.requests.save && !self.state.requests.load {
            return Ok(());
        }
        let directory = services
            .runtime()
            .save_slots()
            .context("save-slot movement requires the loaded directory")?;
        let selected_slot = self
            .state
            .selected_slot
            .or(self.state.active_slot)
            .unwrap_or(usize::MIN);
        let mut menu = SaveMenuState {
            selected_slot,
            slot_names: directory
                .slots()
                .iter()
                .map(|slot| slot.display_name())
                .collect(),
            edit_name: self.state.edit_name,
        };
        let profile = services
            .runtime()
            .current_profile()
            .context("save-slot movement requires a loaded BloodScript profile")?;
        match action {
            InputAction::MovePrevious => {
                move_input_selection_previous(None, Some(&mut menu))?;
            }
            InputAction::MoveNext => {
                move_input_selection_next(
                    None,
                    Some(&mut menu),
                    profile.directory(),
                    profile.directory(),
                )?;
            }
            _ => unreachable!("only movement actions reach save-slot movement"),
        }
        self.state.selected_slot = Some(menu.selected_slot);
        self.state.active_slot = Some(menu.selected_slot);
        self.state.edit_name = menu.edit_name;
        Ok(())
    }
}

struct RuntimeSaveLoadBackend<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    choice_list: &'services mut ChoiceListState,
    current_rect: &'services mut ChoiceListRect,
    target_rect: ChoiceListRect,
    style: RuntimeChoiceListStyle,
    primary_pointer_pressed: bool,
    editor_key: Option<u8>,
    created_filename: Option<Box<[u8]>>,
}

impl SaveLoadHost for RuntimeSaveLoadBackend<'_, '_> {
    fn layout_save_slots(
        &mut self,
        pass: SaveLoadListPass,
        directory: &OriginalSaveSlotDirectory,
        active_slot: Option<usize>,
        edit_name: SaveSlotName,
    ) -> Result<SaveLoadSelection> {
        let mut labels = directory
            .slots()
            .iter()
            .map(|slot| slot.display_name().bytes())
            .collect::<Vec<_>>();
        if let Some(active_slot) = active_slot
            && let Some(label) = labels.get_mut(active_slot)
        {
            *label = edit_name.bytes();
        }
        let label_slices = labels
            .iter()
            .map(|label| label.as_slice())
            .collect::<Vec<_>>();
        let cancel_label = self
            .style
            .extra_cancel_entry
            .then_some(SAVE_LIST_CANCEL_LABEL);
        let frame = self.services.update_choice_list(
            &label_slices,
            ChoiceListConfig {
                center_x: self.style.center_x,
                preserve_individual_widths: self.style.preserve_individual_widths,
                cancel_label,
                layout_only: pass == SaveLoadListPass::MeasureEditingName,
            },
            self.choice_list,
            self.primary_pointer_pressed,
        )?;
        *self.current_rect = frame.rect;
        Ok(if frame.cancelled {
            SaveLoadSelection::Close
        } else if let Some(slot) = frame.selected_item {
            SaveLoadSelection::Slot(slot)
        } else {
            SaveLoadSelection::None
        })
    }

    fn advance_save_transition(
        &mut self,
        transition: &mut FramebufferTransitionState,
    ) -> Result<()> {
        let Some(region) = advance_framebuffer_rect_transition(
            transition,
            transition_rect(*self.current_rect),
            transition_rect(self.target_rect),
        )?
        else {
            return Ok(());
        };
        let mut tint = [u8::MIN; PALETTE_ENTRY_COUNT];
        build_banked_tint_table(
            self.services.runtime().live_palette(),
            &mut tint,
            BRIDGE_CONSOLE_TINT_FIRST,
        )
        .context("building the save-list transition tint table")?;
        remap_framebuffer_rect(
            self.services.runtime_mut().front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            RasterPoint {
                x: i32::from(region.x),
                y: i32::from(region.y),
            },
            region.width,
            region.height,
            &tint,
        )
        .context("remapping the save-list transition region")?;
        Ok(())
    }

    fn edit_save_slot_name(
        &mut self,
        selected_slot: Option<usize>,
        edit_name: &mut SaveSlotName,
        name_length: usize,
    ) -> Result<bool> {
        let selected_slot = selected_slot.context("save editor has no selected slot")?;
        let directory = self
            .services
            .runtime()
            .save_slots()
            .context("save editor requires the loaded slot directory")?;
        let mut menu = SaveMenuState {
            selected_slot,
            slot_names: directory
                .slots()
                .iter()
                .map(|slot| slot.display_name())
                .collect(),
            edit_name: *edit_name,
        };
        let outcome = update_save_slot_editor(
            &mut menu,
            name_length,
            self.editor_key.take(),
            SaveSlotEditorLayout {
                row_x: self.current_rect.origin[0] as u16,
                row_width: self.current_rect.size[0],
            },
        )?;
        *edit_name = menu.edit_name;
        let SaveSlotEditorOutcome::Editing(frame) = outcome else {
            return Ok(true);
        };

        fill_framebuffer_rect(
            self.services.runtime_mut().front_buffer_mut().pixels_mut(),
            LOGICAL_DISPLAY_CLIP,
            RasterPoint {
                x: i32::from(frame.clear_region.x),
                y: i32::from(frame.clear_region.y),
            },
            frame.clear_region.width,
            frame.clear_region.height,
            frame.background_palette_index,
        )
        .context("clearing the active save-slot editor row")?;
        let fonts = self.services.runtime().data().font_resources().clone();
        draw_square_caps_text(
            self.services.runtime_mut().front_buffer_mut().pixels_mut(),
            &fonts,
            &frame.name.bytes(),
            FontPoint {
                x: i32::from(frame.text_position[0]),
                y: i32::from(frame.text_position[1]),
            },
            FULL_LOGICAL_FONT_BAND,
            frame.text_palette_index,
        )
        .context("drawing the active save-slot editor name")?;
        Ok(false)
    }

    fn create_save_file(&mut self, filename: &[u8]) -> Result<bool> {
        match self.services.runtime().write_save_file(filename, &[]) {
            Ok(_) => {
                self.created_filename = Some(Box::from(filename));
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    fn write_created_save_file(&mut self, data: &[u8]) -> Result<()> {
        let filename = self
            .created_filename
            .take()
            .context("save data was produced without a created destination")?;
        self.services.runtime().write_save_file(&filename, data)?;
        Ok(())
    }

    fn read_save_file(&mut self, filename: &[u8]) -> Result<Option<Box<[u8]>>> {
        self.services.runtime().load_save_file(filename)
    }

    fn write_save_slot_directory(
        &mut self,
        data: &[u8; crate::native::bloodprg::ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT],
    ) -> Result<()> {
        let name =
            commander_blood_formats::archive::BloodResourceName::new(SAVE_SLOT_DIRECTORY_NAME)?;
        self.services
            .runtime()
            .data()
            .resource_store()
            .write_loose(&name, data)?;
        Ok(())
    }
}

struct DeferredSaveProfileBackend {
    profile_snapshot: Option<crate::native::bloodprg::LoadedScriptProfile>,
    restore_data: Option<Box<[u8]>>,
}

impl SaveProfileBackend for DeferredSaveProfileBackend {
    fn capture_save_game(&mut self) -> Result<OriginalSaveGame> {
        OriginalSaveGame::capture(
            self.profile_snapshot
                .as_ref()
                .context("cannot save without a loaded BloodScript profile")?,
        )
        .map_err(Into::into)
    }

    fn restore_save_game(&mut self, data: &[u8]) -> Result<()> {
        OriginalSaveGame::decode_profile(data).context("decoding saved profile identity")?;
        self.restore_data = Some(Box::from(data));
        Ok(())
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

#[cfg(test)]
mod tests {
    use commander_blood_formats::lbm::RGB_COMPONENT_COUNT;

    use crate::native::bloodprg::SaveLoadMenuPhase;
    use crate::runtime::RuntimePaletteTransition;

    use super::*;

    #[test]
    fn requests_and_cancel_share_the_recovered_menu_state() {
        let mut runtime = RuntimeSaveLoad::default();
        runtime.request_save();
        assert!(runtime.state().requests.save);
        assert_eq!(runtime.state().phase, SaveLoadMenuPhase::LayoutPending);
        assert!(runtime.queue_input(InputAction::Cancel));
        runtime.state.cancel();
        assert!(!runtime.state().is_active());

        runtime.request_load();
        assert!(runtime.state().requests.load);
        assert_eq!(runtime.state().phase, SaveLoadMenuPhase::LayoutPending);
    }

    #[test]
    fn transition_target_retains_the_executable_field_order() {
        assert_eq!(
            transition_rect(SAVE_LIST_TRANSITION_TARGET),
            TransitionRect::new(100, 0, 0, 120)
        );
    }

    #[test]
    fn completed_save_load_effects_are_published_once() {
        let mut runtime = RuntimeSaveLoad::default();
        runtime.state.redraw_pending = true;
        runtime.state.palette_dirty = true;
        let mut lifecycle = GameLifecycleState::default();
        let mut palette_transition = RuntimePaletteTransition::default();

        let effects = runtime.take_frame_effects();
        lifecycle.navigation_rebuild_pending |= effects.redraw_requested;
        if effects.palette_upload_requested {
            palette_transition.request_visual_color_update();
        }

        assert!(lifecycle.navigation_rebuild_pending);
        assert_ne!(palette_transition.state().dirty_flags, u8::MIN);
        let mut live_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        palette_transition
            .update(&mut live_palette, &mut lifecycle)
            .unwrap();
        lifecycle.navigation_rebuild_pending = false;

        let effects = runtime.take_frame_effects();
        lifecycle.navigation_rebuild_pending |= effects.redraw_requested;
        if effects.palette_upload_requested {
            palette_transition.request_visual_color_update();
        }

        assert_eq!(effects, SaveLoadFrameEffects::default());
        assert!(!lifecycle.navigation_rebuild_pending);
        assert_eq!(palette_transition.state().dirty_flags, u8::MIN);
    }
}
