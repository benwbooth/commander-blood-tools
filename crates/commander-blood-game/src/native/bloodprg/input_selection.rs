//! Typed directory and save-menu movement for keyboard input.

use std::error::Error;
use std::fmt;

use commander_blood_formats::script::{ScriptDirectory, ScriptSymbolKind};

use super::{InputDispatchState, latch_input_text_byte};

/// Number of directory rows visible in the original selection window.
pub const INPUT_SELECTION_VISIBLE_ROWS: usize = 15;
/// Bytes retained for one editable save-slot name.
pub const SAVE_SLOT_NAME_LENGTH: usize = 16;
const SELECTION_STEP: usize = 1;
const SAVE_SLOT_CHARACTER_LIMIT: usize = 14;
const SAVE_SLOT_ENTER_KEY: u8 = b'\r';
const SAVE_SLOT_BACKSPACE_KEY: u8 = 8;
const SAVE_SLOT_ROW_HEIGHT: u16 = 10;
const SAVE_SLOT_ROW_PITCH: u16 = 11;
const SAVE_SLOT_ROW_TOP: u16 = 39;
const SAVE_SLOT_TEXT_INSET: u16 = 10;
const SAVE_SLOT_TEXT_Y_INSET: u16 = 1;
const SAVE_SLOT_BACKGROUND_PALETTE_INDEX: u8 = 232;
const SAVE_SLOT_TEXT_PALETTE_INDEX: u8 = 239;
const ERASED_SAVE_SLOT_CHARACTER: u8 = b' ';

/// Directory supplying rows to the active selection window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputSelectionSource {
    /// Directory decoded from the active script profile.
    Profile,
    /// Built-in fallback directory decoded from the executable.
    Builtin,
}

/// Stable identity of one selected directory row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputDirectoryRowId(usize);

impl InputDirectoryRowId {
    /// Construct a row identity from a validated zero-based selection index.
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    /// Return the zero-based directory row.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Mutable row and viewport state for an active directory selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputSelectionState {
    /// Which decoded directory owns the displayed rows.
    pub source: InputSelectionSource,
    /// Currently highlighted row.
    pub selected: usize,
    /// First row visible in the fixed-height viewport.
    pub first_visible: usize,
    /// Accepted row, if this selector has already committed a choice.
    pub committed: Option<InputDirectoryRowId>,
}

/// Fixed-size editable save-slot name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveSlotName([u8; SAVE_SLOT_NAME_LENGTH]);

impl SaveSlotName {
    /// Construct one name from its complete fixed-width byte field.
    pub const fn from_bytes(bytes: [u8; SAVE_SLOT_NAME_LENGTH]) -> Self {
        Self(bytes)
    }

    /// Return the complete editable byte field.
    pub const fn bytes(self) -> [u8; SAVE_SLOT_NAME_LENGTH] {
        self.0
    }
}

impl Default for SaveSlotName {
    fn default() -> Self {
        Self([u8::MIN; SAVE_SLOT_NAME_LENGTH])
    }
}

/// Owned save-menu selection and editable name state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveMenuState {
    /// Currently selected slot.
    pub selected_slot: usize,
    /// Names belonging to every editable slot.
    pub slot_names: Vec<SaveSlotName>,
    /// Name currently shown in the editor.
    pub edit_name: SaveSlotName,
}

/// Logical geometry supplied by the surrounding save/load menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveSlotEditorLayout {
    /// Horizontal origin of each save-slot row.
    pub row_x: u16,
    /// Width of each save-slot row.
    pub row_width: u16,
}

/// Logical rectangle cleared behind the active save-slot name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveSlotEditorRectangle {
    /// Horizontal origin.
    pub x: u16,
    /// Vertical origin.
    pub y: u16,
    /// Width in logical pixels.
    pub width: u16,
    /// Height in logical pixels.
    pub height: u16,
}

/// Renderer-independent draw plan for the active save-slot row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveSlotEditorFrame {
    /// Row cleared before rendering the edited name.
    pub clear_region: SaveSlotEditorRectangle,
    /// Palette index used to clear the row.
    pub background_palette_index: u8,
    /// Current fixed-width name rendered in the row.
    pub name: SaveSlotName,
    /// Logical text origin.
    pub text_position: [u16; 2],
    /// Palette index used by the square-cap font.
    pub text_palette_index: u8,
}

/// Result of one save-slot editor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveSlotEditorOutcome {
    /// The edit remains active and the returned row must be rendered.
    Editing(SaveSlotEditorFrame),
    /// Enter committed the complete fixed-width name without another draw.
    Committed,
}

/// Invalid typed input-selection state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputSelectionError {
    /// The save-menu selection does not identify one owned slot name.
    MissingSaveSlot {
        /// Invalid selected slot.
        selected: usize,
        /// Number of owned slot names.
        slot_count: usize,
    },
    /// The shared native name length would address outside the owned name.
    InvalidSaveSlotNameLength {
        /// Invalid externally computed length.
        length: usize,
        /// Number of bytes in one owned slot name.
        capacity: usize,
    },
}

impl fmt::Display for InputSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid input selection state: {self:?}")
    }
}

impl Error for InputSelectionError {}

/// Move to the preceding directory row or save slot.
///
/// This translates `input_action_move_previous` at BLOODPRG routine offset
/// `0x002140`. Optional typed UI owners replace mode bits, sentinel words,
/// fixed record strides, name pointers, and an unchecked byte copy.
pub fn move_input_selection_previous(
    selection: Option<&mut InputSelectionState>,
    save_menu: Option<&mut SaveMenuState>,
) -> Result<bool, InputSelectionError> {
    if let Some(selection) = selection {
        if selection.committed.is_some() || selection.selected == usize::MIN {
            return Ok(false);
        }

        selection.selected -= SELECTION_STEP;
        if selection.selected < selection.first_visible {
            selection.first_visible = selection.first_visible.saturating_sub(SELECTION_STEP);
        }
        return Ok(true);
    }

    let Some(save_menu) = save_menu else {
        return Ok(false);
    };
    validate_save_slot(save_menu)?;
    if save_menu.selected_slot == usize::MIN {
        return Ok(false);
    }

    save_menu.selected_slot -= SELECTION_STEP;
    save_menu.edit_name = save_menu.slot_names[save_menu.selected_slot];
    Ok(true)
}

/// Move to the following directory row or save slot.
///
/// This translates `input_action_move_next` at BLOODPRG routine offset
/// `0x00218D`. Decoded directory entries and owned slot names replace both
/// directory ownership modes, record arithmetic, and mutable name pointers.
pub fn move_input_selection_next(
    selection: Option<&mut InputSelectionState>,
    save_menu: Option<&mut SaveMenuState>,
    profile_directory: &ScriptDirectory,
    builtin_directory: &ScriptDirectory,
) -> Result<bool, InputSelectionError> {
    if let Some(selection) = selection {
        if selection.committed.is_some() {
            return Ok(false);
        }

        let Some(next) = selection.selected.checked_add(SELECTION_STEP) else {
            return Ok(false);
        };
        let directory = match selection.source {
            InputSelectionSource::Profile => profile_directory,
            InputSelectionSource::Builtin => builtin_directory,
        };
        let Some(entry) = directory.entries().get(next) else {
            return Ok(false);
        };
        if entry.kind == ScriptSymbolKind::Sentinel {
            return Ok(false);
        }

        selection.selected = next;
        if selection.selected.saturating_sub(selection.first_visible)
            >= INPUT_SELECTION_VISIBLE_ROWS
        {
            selection.first_visible = selection.first_visible.saturating_add(SELECTION_STEP);
        }
        return Ok(true);
    }

    let Some(save_menu) = save_menu else {
        return Ok(false);
    };
    validate_save_slot(save_menu)?;
    let Some(next) = save_menu.selected_slot.checked_add(SELECTION_STEP) else {
        return Ok(false);
    };
    let Some(name) = save_menu.slot_names.get(next).copied() else {
        return Ok(false);
    };

    save_menu.selected_slot = next;
    save_menu.edit_name = name;
    Ok(true)
}

/// Latch Enter and commit an active profile-directory row.
///
/// This translates `input_action_accept` at BLOODPRG routine offset
/// `0x002224`. A stable row identity replaces the byte-truncated index and
/// serialized record position; built-in and inactive rows remain uncommitted.
pub fn accept_input_selection(
    dispatch: &mut InputDispatchState,
    selection: Option<&mut InputSelectionState>,
    profile_directory: &ScriptDirectory,
    text_byte: u8,
) -> Option<InputDirectoryRowId> {
    latch_input_text_byte(dispatch, text_byte);
    let selection = selection?;
    if selection.source != InputSelectionSource::Profile {
        return None;
    }
    let entry = profile_directory.entries().get(selection.selected)?;
    if entry.kind != ScriptSymbolKind::Object {
        return None;
    }

    let committed = InputDirectoryRowId::from_index(selection.selected);
    selection.committed = Some(committed);
    Some(committed)
}

/// Apply one key and produce the active save-slot row draw plan.
///
/// This translates `save_slot_name_edit_step` at BLOODPRG routine offset
/// `0x001DD8`. It retains the lowercase-and-digit input domain, the authored
/// length-14 insertion stop, Backspace replacement with a space, full 16-byte
/// Enter commit, low-byte row selection, wrapping screen coordinates, and the
/// exact row colors. Owned names and logical rectangles replace unchecked near
/// copies and direct VGA draw calls.
pub fn update_save_slot_editor(
    state: &mut SaveMenuState,
    name_length: usize,
    key: Option<u8>,
    layout: SaveSlotEditorLayout,
) -> Result<SaveSlotEditorOutcome, InputSelectionError> {
    if name_length > SAVE_SLOT_NAME_LENGTH {
        return Err(InputSelectionError::InvalidSaveSlotNameLength {
            length: name_length,
            capacity: SAVE_SLOT_NAME_LENGTH,
        });
    }

    if let Some(key) = key {
        if key == SAVE_SLOT_ENTER_KEY {
            if name_length != usize::MIN {
                validate_save_slot(state)?;
                state.slot_names[state.selected_slot] = state.edit_name;
                return Ok(SaveSlotEditorOutcome::Committed);
            }
        } else if key.is_ascii_digit() || key.is_ascii_lowercase() {
            if name_length != SAVE_SLOT_CHARACTER_LIMIT {
                let Some(character) = state.edit_name.0.get_mut(name_length) else {
                    return Err(InputSelectionError::InvalidSaveSlotNameLength {
                        length: name_length,
                        capacity: SAVE_SLOT_NAME_LENGTH,
                    });
                };
                *character = key;
            }
        } else if key == SAVE_SLOT_BACKSPACE_KEY && name_length != usize::MIN {
            state.edit_name.0[name_length - 1] = ERASED_SAVE_SLOT_CHARACTER;
        }
    }

    let row_y = u16::from(state.selected_slot as u8)
        .wrapping_mul(SAVE_SLOT_ROW_PITCH)
        .wrapping_add(SAVE_SLOT_ROW_TOP);
    Ok(SaveSlotEditorOutcome::Editing(SaveSlotEditorFrame {
        clear_region: SaveSlotEditorRectangle {
            x: layout.row_x,
            y: row_y,
            width: layout.row_width,
            height: SAVE_SLOT_ROW_HEIGHT,
        },
        background_palette_index: SAVE_SLOT_BACKGROUND_PALETTE_INDEX,
        name: state.edit_name,
        text_position: [
            layout.row_x.wrapping_add(SAVE_SLOT_TEXT_INSET),
            row_y.wrapping_add(SAVE_SLOT_TEXT_Y_INSET),
        ],
        text_palette_index: SAVE_SLOT_TEXT_PALETTE_INDEX,
    }))
}

fn validate_save_slot(save_menu: &SaveMenuState) -> Result<(), InputSelectionError> {
    if save_menu.selected_slot < save_menu.slot_names.len() {
        Ok(())
    } else {
        Err(InputSelectionError::MissingSaveSlot {
            selected: save_menu.selected_slot,
            slot_count: save_menu.slot_names.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use commander_blood_formats::script::decode_script_directory;

    use super::*;

    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const DIRECTORY_KIND_FIELD: usize = 18;
    const DIRECTORY_FIXTURE_ENTRY_COUNT: usize = 20;
    const OBJECT_ENTRY_KIND: u16 = 1;
    const SENTINEL_ENTRY_KIND: u16 = 0;
    const MOVE_PREVIOUS_VECTOR_COUNT: usize = 6;
    const MOVE_NEXT_VECTOR_COUNT: usize = 4;
    const ACCEPT_VECTOR_COUNT: usize = 3;
    const ENTER_KEY_BYTE: u8 = b'\r';
    const ORIGINAL_NO_COMMIT_SENTINEL: u16 = 0x7777;
    const SAVE_EDITOR_VECTOR_COUNT: usize = 10;
    const INITIAL_EDIT_NAME: [u8; SAVE_SLOT_NAME_LENGTH] = *b"alpha beta      ";
    const INITIAL_ACTIVE_NAME: [u8; SAVE_SLOT_NAME_LENGTH] = [b'm'; SAVE_SLOT_NAME_LENGTH];

    #[derive(Deserialize)]
    struct InputHandlerOracle {
        vectors: InputHandlerVectors,
    }

    #[derive(Deserialize)]
    struct SaveEditorOracle {
        name: String,
        key: u8,
        name_length: usize,
        selected_index: usize,
        edit_buffer_hex: String,
        active_name_hex: String,
        committed: bool,
        calls: Vec<serde_json::Value>,
    }

    #[derive(Deserialize)]
    struct InputHandlerVectors {
        move_previous: Vec<MovementOracle>,
        move_next: Vec<MovementOracle>,
        accept: Vec<AcceptOracle>,
    }

    #[test]
    fn save_slot_editor_matches_every_original_vector() {
        let vectors: Vec<SaveEditorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1dd8_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SAVE_EDITOR_VECTOR_COUNT);

        for vector in vectors {
            let mut state = SaveMenuState {
                selected_slot: vector.selected_index,
                slot_names: vec![
                    SaveSlotName::from_bytes(INITIAL_ACTIVE_NAME);
                    vector.selected_index.min(4) + 1
                ],
                edit_name: SaveSlotName::from_bytes(INITIAL_EDIT_NAME),
            };
            let first_call = vector.calls.first();
            let layout = SaveSlotEditorLayout {
                row_x: first_call
                    .and_then(|call| call.get("x"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(50) as u16,
                row_width: first_call
                    .and_then(|call| call.get("width"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(140) as u16,
            };
            let outcome = update_save_slot_editor(
                &mut state,
                vector.name_length,
                (vector.key != u8::MIN).then_some(vector.key),
                layout,
            )
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(
                state.edit_name.bytes(),
                decode_fixed_name(&vector.edit_buffer_hex),
                "{}",
                vector.name
            );
            assert_eq!(
                matches!(outcome, SaveSlotEditorOutcome::Committed),
                vector.committed,
                "{}",
                vector.name
            );
            if vector.committed {
                assert_eq!(
                    state.slot_names[state.selected_slot].bytes(),
                    decode_fixed_name(&vector.active_name_hex),
                    "{}",
                    vector.name
                );
                assert!(vector.calls.is_empty(), "{}", vector.name);
            } else {
                let SaveSlotEditorOutcome::Editing(frame) = outcome else {
                    unreachable!()
                };
                let clear = &vector.calls[0];
                let text = &vector.calls[1];
                assert_eq!(
                    frame.clear_region.x,
                    clear["x"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.clear_region.y,
                    clear["y"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.clear_region.width,
                    clear["width"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.clear_region.height,
                    clear["height"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.background_palette_index,
                    clear["color"].as_u64().unwrap() as u8,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.text_position[0],
                    text["x"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.text_position[1],
                    text["y"].as_u64().unwrap() as u16,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    frame.text_palette_index,
                    text["color"].as_u64().unwrap() as u8,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn save_slot_editor_rejects_a_flat_name_index_outside_the_owned_field() {
        let mut state = SaveMenuState {
            selected_slot: usize::MIN,
            slot_names: vec![SaveSlotName::default()],
            edit_name: SaveSlotName::default(),
        };

        assert_eq!(
            update_save_slot_editor(
                &mut state,
                SAVE_SLOT_NAME_LENGTH + 1,
                Some(b'a'),
                SaveSlotEditorLayout {
                    row_x: u16::MIN,
                    row_width: u16::MIN,
                },
            ),
            Err(InputSelectionError::InvalidSaveSlotNameLength {
                length: SAVE_SLOT_NAME_LENGTH + 1,
                capacity: SAVE_SLOT_NAME_LENGTH,
            })
        );
    }

    fn decode_fixed_name(encoded: &str) -> [u8; SAVE_SLOT_NAME_LENGTH] {
        assert_eq!(encoded.len(), SAVE_SLOT_NAME_LENGTH * 2);
        let mut bytes = [u8::MIN; SAVE_SLOT_NAME_LENGTH];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16).unwrap();
        }
        bytes
    }

    #[derive(Deserialize)]
    struct MovementOracle {
        name: String,
        mode: String,
        source: Option<String>,
        committed: Option<bool>,
        selected_before: Option<usize>,
        selected: Option<usize>,
        first_visible_before: Option<usize>,
        first_visible: Option<usize>,
        next_entry_kind: Option<u16>,
        slot_before: Option<usize>,
        slot: Option<usize>,
        edit_name_bytes: Option<[u8; SAVE_SLOT_NAME_LENGTH]>,
    }

    #[derive(Deserialize)]
    struct AcceptOracle {
        name: String,
        profile_selection: bool,
        selected_index: usize,
        record_kind: u16,
        committed_offset: u16,
        latched_key: u8,
    }

    #[test]
    fn previous_movement_matches_every_original_handler_vector() {
        let oracle = handler_oracle();
        assert_eq!(
            oracle.vectors.move_previous.len(),
            MOVE_PREVIOUS_VECTOR_COUNT
        );

        for vector in oracle.vectors.move_previous {
            let mut selection = selection_from_oracle(&vector);
            let mut save_menu = save_menu_from_oracle(&vector);

            move_input_selection_previous(selection.as_mut(), save_menu.as_mut()).unwrap();

            assert_movement_result(&vector, selection.as_ref(), save_menu.as_ref());
        }
    }

    #[test]
    fn next_movement_matches_every_original_handler_vector() {
        let oracle = handler_oracle();
        assert_eq!(oracle.vectors.move_next.len(), MOVE_NEXT_VECTOR_COUNT);

        for vector in oracle.vectors.move_next {
            let mut selection = selection_from_oracle(&vector);
            let mut save_menu = save_menu_from_oracle(&vector);
            let next_index = vector
                .selected_before
                .and_then(|selected| selected.checked_add(SELECTION_STEP));
            let profile = directory_with_kind(next_index, vector.next_entry_kind);
            let builtin = directory_with_kind(next_index, vector.next_entry_kind);

            move_input_selection_next(selection.as_mut(), save_menu.as_mut(), &profile, &builtin)
                .unwrap();

            assert_movement_result(&vector, selection.as_ref(), save_menu.as_ref());
        }
    }

    #[test]
    fn acceptance_matches_every_original_handler_vector() {
        let oracle = handler_oracle();
        assert_eq!(oracle.vectors.accept.len(), ACCEPT_VECTOR_COUNT);

        for vector in oracle.vectors.accept {
            let directory =
                directory_with_kind(Some(vector.selected_index), Some(vector.record_kind));
            let mut selection = InputSelectionState {
                source: if vector.profile_selection {
                    InputSelectionSource::Profile
                } else {
                    InputSelectionSource::Builtin
                },
                selected: vector.selected_index,
                first_visible: usize::MIN,
                committed: None,
            };
            let mut dispatch = InputDispatchState::default();

            let committed = accept_input_selection(
                &mut dispatch,
                Some(&mut selection),
                &directory,
                ENTER_KEY_BYTE,
            );

            let original_committed = vector.committed_offset != ORIGINAL_NO_COMMIT_SENTINEL;
            assert_eq!(committed.is_some(), original_committed, "{}", vector.name);
            assert_eq!(selection.committed, committed, "{}", vector.name);
            assert_eq!(
                dispatch.text_byte,
                Some(vector.latched_key),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_save_selection_is_rejected_without_indexing_owned_names() {
        let mut save_menu = SaveMenuState {
            selected_slot: 1,
            slot_names: Vec::new(),
            edit_name: SaveSlotName::default(),
        };

        assert!(matches!(
            move_input_selection_previous(None, Some(&mut save_menu)),
            Err(InputSelectionError::MissingSaveSlot { .. })
        ));
    }

    fn handler_oracle() -> InputHandlerOracle {
        serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/input_action_handlers_natural.json"
        ))
        .unwrap()
    }

    fn selection_from_oracle(vector: &MovementOracle) -> Option<InputSelectionState> {
        (vector.mode == "selection").then(|| InputSelectionState {
            source: match vector.source.as_deref().unwrap() {
                "profile" => InputSelectionSource::Profile,
                "builtin" => InputSelectionSource::Builtin,
                other => panic!("unknown selection source {other}"),
            },
            selected: vector.selected_before.unwrap(),
            first_visible: vector.first_visible_before.unwrap(),
            committed: vector
                .committed
                .unwrap()
                .then_some(InputDirectoryRowId::from_index(usize::MIN)),
        })
    }

    fn save_menu_from_oracle(vector: &MovementOracle) -> Option<SaveMenuState> {
        (vector.mode == "save_menu").then(|| {
            let selected_slot = vector.slot_before.unwrap();
            let result_slot = vector.slot.unwrap();
            let mut names =
                vec![SaveSlotName::default(); selected_slot.max(result_slot) + SELECTION_STEP];
            names[result_slot] = SaveSlotName::from_bytes(vector.edit_name_bytes.unwrap());
            SaveMenuState {
                selected_slot,
                slot_names: names,
                edit_name: SaveSlotName::default(),
            }
        })
    }

    fn assert_movement_result(
        vector: &MovementOracle,
        selection: Option<&InputSelectionState>,
        save_menu: Option<&SaveMenuState>,
    ) {
        match vector.mode.as_str() {
            "selection" => {
                let selection = selection.unwrap();
                assert_eq!(
                    selection.selected,
                    vector.selected.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    selection.first_visible,
                    vector.first_visible.unwrap(),
                    "{}",
                    vector.name
                );
            }
            "save_menu" => {
                let save_menu = save_menu.unwrap();
                assert_eq!(
                    save_menu.selected_slot,
                    vector.slot.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    save_menu.edit_name.bytes(),
                    vector.edit_name_bytes.unwrap(),
                    "{}",
                    vector.name
                );
            }
            "inactive" => assert!(selection.is_none() && save_menu.is_none()),
            other => panic!("unknown movement mode {other}"),
        }
    }

    fn directory_with_kind(row: Option<usize>, kind: Option<u16>) -> ScriptDirectory {
        let mut bytes = vec![u8::MIN; DIRECTORY_FIXTURE_ENTRY_COUNT * DIRECTORY_ENTRY_SIZE];
        for entry in bytes.chunks_exact_mut(DIRECTORY_ENTRY_SIZE) {
            entry[DIRECTORY_KIND_FIELD..DIRECTORY_KIND_FIELD + size_of::<u16>()]
                .copy_from_slice(&OBJECT_ENTRY_KIND.to_le_bytes());
        }
        if let Some(row) = row {
            let kind = kind.unwrap_or(SENTINEL_ENTRY_KIND);
            let start = row * DIRECTORY_ENTRY_SIZE + DIRECTORY_KIND_FIELD;
            bytes[start..start + size_of::<u16>()].copy_from_slice(&kind.to_le_bytes());
        }
        decode_script_directory(&bytes).unwrap()
    }
}
