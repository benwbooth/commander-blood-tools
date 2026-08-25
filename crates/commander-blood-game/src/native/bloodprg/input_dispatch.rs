//! Semantic keyboard dispatch for the modern SDL event loop.

const BACKSPACE_TEXT_BYTE: u8 = b'\x08';
const DELETE_TEXT_BYTE: u8 = b'\x7f';
const FIRST_PRINTABLE_CHARACTER: char = '!';
const LAST_PRINTABLE_CHARACTER: char = '~';
const PAUSE_CHARACTER_LOWER: char = 'p';
const PAUSE_CHARACTER_UPPER: char = 'P';

/// Arrow keys recognized by the original input policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputArrowKey {
    /// Move toward the preceding row.
    Up,
    /// Move toward the following row.
    Down,
    /// Authored but deliberately inert horizontal movement.
    Left,
    /// Authored but deliberately inert horizontal movement.
    Right,
}

/// Function keys represented by the shipped input table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputFunctionKey {
    /// First function key.
    F1,
    /// Second function key.
    F2,
    /// Third function key.
    F3,
    /// Fourth function key.
    F4,
    /// Fifth function key.
    F5,
    /// Sixth function key.
    F6,
    /// Seventh function key.
    F7,
}

/// One platform-independent key supplied by the host event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostInputKey {
    /// One Unicode character; only the original printable ASCII domain is used.
    Character(char),
    /// Backspace text-editing key.
    Backspace,
    /// Accept or confirm key.
    Enter,
    /// Cancel key.
    Escape,
    /// Spacebar, which shares the original cancel action.
    Space,
    /// Delete text-editing key.
    Delete,
    /// One directional key.
    Arrow(InputArrowKey),
    /// One authored function key.
    Function(InputFunctionKey),
}

/// Proven inert actions retained so authored key bindings remain explicit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgnoredInputAction {
    /// Inert left movement command.
    MoveLeft,
    /// Inert right movement command.
    MoveRight,
    /// Inert authored function-key command.
    Function(InputFunctionKey),
}

/// Semantic input command consumed by higher-level game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction {
    /// Select the preceding row.
    MovePrevious,
    /// Select the following row.
    MoveNext,
    /// Accept the current selection.
    Accept,
    /// Cancel the current interaction.
    Cancel,
    /// Publish one byte to the active text consumer.
    LatchTextByte(u8),
    /// Toggle pause and publish the source byte.
    TogglePause(u8),
    /// Preserve one explicitly authored inert binding.
    Ignored(IgnoredInputAction),
}

/// Input latches shared by the main loop and text widgets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputDispatchState {
    /// Most recent byte waiting for an active text consumer.
    pub text_byte: Option<u8>,
    /// Whether presentation updates are paused.
    pub paused: bool,
    /// Programmatic shutdown request; the shipped key table has no binding.
    pub shutdown_requested: bool,
}

/// Clear the prior text latch and translate one optional host key.
///
/// This is the flat host translation of `input_action_dispatch` at BLOODPRG
/// routine offset `0x00210E`. The original BIOS key word and signed byte table
/// are decoder evidence only; runtime callers provide a typed SDL key.
pub fn dispatch_input_key(
    state: &mut InputDispatchState,
    key: Option<HostInputKey>,
) -> Option<InputAction> {
    state.text_byte = None;
    key.and_then(translate_input_key)
}

/// Translate one typed host key through the shipped keyboard policy.
pub fn translate_input_key(key: HostInputKey) -> Option<InputAction> {
    match key {
        HostInputKey::Backspace => Some(InputAction::LatchTextByte(BACKSPACE_TEXT_BYTE)),
        HostInputKey::Enter => Some(InputAction::Accept),
        HostInputKey::Escape => Some(InputAction::Cancel),
        HostInputKey::Space => Some(InputAction::Cancel),
        HostInputKey::Delete => Some(InputAction::LatchTextByte(DELETE_TEXT_BYTE)),
        HostInputKey::Arrow(InputArrowKey::Up) => Some(InputAction::MovePrevious),
        HostInputKey::Arrow(InputArrowKey::Down) => Some(InputAction::MoveNext),
        HostInputKey::Arrow(InputArrowKey::Left) => {
            Some(InputAction::Ignored(IgnoredInputAction::MoveLeft))
        }
        HostInputKey::Arrow(InputArrowKey::Right) => {
            Some(InputAction::Ignored(IgnoredInputAction::MoveRight))
        }
        HostInputKey::Function(function) => {
            Some(InputAction::Ignored(IgnoredInputAction::Function(function)))
        }
        HostInputKey::Character(character)
            if matches!(character, PAUSE_CHARACTER_LOWER | PAUSE_CHARACTER_UPPER) =>
        {
            Some(InputAction::TogglePause(character as u8))
        }
        HostInputKey::Character(character)
            if (FIRST_PRINTABLE_CHARACTER..=LAST_PRINTABLE_CHARACTER).contains(&character) =>
        {
            Some(InputAction::LatchTextByte(character as u8))
        }
        HostInputKey::Character(_) => None,
    }
}

/// Set the otherwise-unbound shutdown request.
///
/// This translates `input_action_request_shutdown` at BLOODPRG routine offset
/// `0x002203`. It remains a programmatic command because no shipped key maps to
/// its action slot.
pub fn request_input_shutdown(state: &mut InputDispatchState) {
    state.shutdown_requested = true;
}

/// Publish one byte for an active text consumer.
///
/// This translates `input_action_latch_text_key` at BLOODPRG routine offset
/// `0x0022D0` using an optional typed latch instead of a shared byte.
pub fn latch_input_text_byte(state: &mut InputDispatchState, text_byte: u8) {
    state.text_byte = Some(text_byte);
}

/// Toggle presentation pause when the save UI is not active, then latch `P`.
///
/// This translates `input_action_toggle_pause` at BLOODPRG routine offset
/// `0x0022B2`. Boolean pause and save-menu state replace unrelated packed bits.
pub fn toggle_input_pause(state: &mut InputDispatchState, save_menu_active: bool, text_byte: u8) {
    if !save_menu_active {
        state.paused = !state.paused;
    }
    latch_input_text_byte(state, text_byte);
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const TRANSLATION_TABLE_ENTRY_COUNT: usize = 256;
    const ENTER_TEXT_BYTE: u8 = b'\r';
    const ESCAPE_TEXT_BYTE: u8 = b'\x1b';
    const SPACE_TEXT_BYTE: u8 = b' ';
    const UNMAPPED_ACTION_INDEX: u8 = u8::MAX;
    const MOVE_PREVIOUS_ACTION_INDEX: u8 = 0;
    const MOVE_NEXT_ACTION_INDEX: u8 = 1;
    const MOVE_RIGHT_ACTION_INDEX: u8 = 2;
    const MOVE_LEFT_ACTION_INDEX: u8 = 3;
    const FUNCTION_ONE_ACTION_INDEX: u8 = 5;
    const ACCEPT_ACTION_INDEX: u8 = 6;
    const CANCEL_ACTION_INDEX: u8 = 7;
    const TEXT_ACTION_INDEX: u8 = 8;
    const FUNCTION_TWO_ACTION_INDEX: u8 = 9;
    const FUNCTION_THREE_ACTION_INDEX: u8 = 10;
    const FUNCTION_FOUR_ACTION_INDEX: u8 = 11;
    const FUNCTION_FIVE_ACTION_INDEX: u8 = 12;
    const FUNCTION_SIX_ACTION_INDEX: u8 = 13;
    const FUNCTION_SEVEN_ACTION_INDEX: u8 = 14;
    const TOGGLE_PAUSE_ACTION_INDEX: u8 = 15;
    const EXPECTED_SIMPLE_HANDLER_VECTOR_COUNT: usize = 11;
    const EXPECTED_PAUSE_VECTOR_COUNT: usize = 4;

    #[derive(Deserialize)]
    struct InputHandlerOracle {
        inventory: InputInventory,
        vectors: InputHandlerVectors,
    }

    #[derive(Deserialize)]
    struct InputInventory {
        translation_table: Vec<u8>,
        unmapped_handler_indices: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct InputHandlerVectors {
        simple_handlers: Vec<SimpleHandlerOracle>,
        toggle_pause: Vec<PauseOracle>,
    }

    #[derive(Deserialize)]
    struct SimpleHandlerOracle {
        name: String,
        action_index: u8,
        memory: Option<String>,
        shutdown_latch: Option<u8>,
        latched_key: Option<u8>,
    }

    #[derive(Deserialize)]
    struct PauseOracle {
        name: String,
        save_active: u8,
        pause_before: u8,
        pause_after: u8,
        latched_key: u8,
    }

    #[test]
    fn typed_host_keys_match_every_shipped_translation_table_entry() {
        let oracle = handler_oracle();
        assert_eq!(
            oracle.inventory.translation_table.len(),
            TRANSLATION_TABLE_ENTRY_COUNT
        );

        for (translated_code, expected) in oracle.inventory.translation_table.iter().enumerate() {
            let actual = host_key_for_translated_code(translated_code as u8)
                .and_then(translate_input_key)
                .map(action_index)
                .unwrap_or(UNMAPPED_ACTION_INDEX);
            assert_eq!(actual, *expected, "translated code {translated_code:#04x}");
        }
        assert_eq!(
            oracle.inventory.unmapped_handler_indices,
            [request_shutdown_action_index()]
        );
    }

    #[test]
    fn dispatcher_clears_the_previous_text_latch_before_polling() {
        let mut state = InputDispatchState {
            text_byte: Some(b'x'),
            paused: false,
            shutdown_requested: false,
        };

        assert_eq!(dispatch_input_key(&mut state, None), None);
        assert_eq!(state.text_byte, None);
        assert_eq!(
            dispatch_input_key(&mut state, Some(HostInputKey::Character('a'))),
            Some(InputAction::LatchTextByte(b'a'))
        );
        assert_eq!(state.text_byte, None);
    }

    #[test]
    fn proven_inert_and_simple_handlers_preserve_semantic_state() {
        let oracle = handler_oracle();
        assert_eq!(
            oracle.vectors.simple_handlers.len(),
            EXPECTED_SIMPLE_HANDLER_VECTOR_COUNT
        );

        for vector in oracle.vectors.simple_handlers {
            let mut state = InputDispatchState::default();
            match vector.name.as_str() {
                "input_action_request_shutdown" => {
                    request_input_shutdown(&mut state);
                    assert_eq!(
                        state.shutdown_requested,
                        vector.shutdown_latch == Some(1),
                        "{}",
                        vector.name
                    );
                }
                "input_action_latch_text_key" => {
                    let text_byte = vector.latched_key.unwrap();
                    latch_input_text_byte(&mut state, text_byte);
                    assert_eq!(state.text_byte, Some(text_byte), "{}", vector.name);
                }
                _ => {
                    let before = state;
                    let key = host_key_for_inert_action(vector.action_index).unwrap();
                    assert!(matches!(
                        translate_input_key(key),
                        Some(InputAction::Ignored(_))
                    ));
                    assert_eq!(state, before, "{}", vector.name);
                    assert_eq!(vector.memory.as_deref(), Some("unchanged"));
                }
            }
        }
    }

    #[test]
    fn pause_toggle_matches_every_original_handler_vector() {
        let oracle = handler_oracle();
        assert_eq!(
            oracle.vectors.toggle_pause.len(),
            EXPECTED_PAUSE_VECTOR_COUNT
        );

        for vector in oracle.vectors.toggle_pause {
            let mut state = InputDispatchState {
                text_byte: None,
                paused: vector.pause_before != u8::MIN,
                shutdown_requested: false,
            };
            toggle_input_pause(
                &mut state,
                vector.save_active != u8::MIN,
                vector.latched_key,
            );
            assert_eq!(
                state.paused,
                vector.pause_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(state.text_byte, Some(vector.latched_key), "{}", vector.name);
        }
    }

    fn handler_oracle() -> InputHandlerOracle {
        serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/input_action_handlers_natural.json"
        ))
        .unwrap()
    }

    fn host_key_for_translated_code(code: u8) -> Option<HostInputKey> {
        match code {
            BACKSPACE_TEXT_BYTE => Some(HostInputKey::Backspace),
            ENTER_TEXT_BYTE => Some(HostInputKey::Enter),
            ESCAPE_TEXT_BYTE => Some(HostInputKey::Escape),
            SPACE_TEXT_BYTE => Some(HostInputKey::Space),
            DELETE_TEXT_BYTE => Some(HostInputKey::Delete),
            0xBB => Some(HostInputKey::Function(InputFunctionKey::F1)),
            0xBC => Some(HostInputKey::Function(InputFunctionKey::F2)),
            0xBD => Some(HostInputKey::Function(InputFunctionKey::F3)),
            0xBE => Some(HostInputKey::Function(InputFunctionKey::F4)),
            0xBF => Some(HostInputKey::Function(InputFunctionKey::F5)),
            0xC0 => Some(HostInputKey::Function(InputFunctionKey::F6)),
            0xC1 => Some(HostInputKey::Function(InputFunctionKey::F7)),
            0xC8 => Some(HostInputKey::Arrow(InputArrowKey::Up)),
            0xCB => Some(HostInputKey::Arrow(InputArrowKey::Left)),
            0xCD => Some(HostInputKey::Arrow(InputArrowKey::Right)),
            0xD0 => Some(HostInputKey::Arrow(InputArrowKey::Down)),
            printable if printable.is_ascii_graphic() => {
                Some(HostInputKey::Character(printable as char))
            }
            _ => None,
        }
    }

    fn host_key_for_inert_action(action: u8) -> Option<HostInputKey> {
        match action {
            MOVE_RIGHT_ACTION_INDEX => Some(HostInputKey::Arrow(InputArrowKey::Right)),
            MOVE_LEFT_ACTION_INDEX => Some(HostInputKey::Arrow(InputArrowKey::Left)),
            FUNCTION_ONE_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F1)),
            FUNCTION_TWO_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F2)),
            FUNCTION_THREE_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F3)),
            FUNCTION_FOUR_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F4)),
            FUNCTION_FIVE_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F5)),
            FUNCTION_SIX_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F6)),
            FUNCTION_SEVEN_ACTION_INDEX => Some(HostInputKey::Function(InputFunctionKey::F7)),
            _ => None,
        }
    }

    fn action_index(action: InputAction) -> u8 {
        match action {
            InputAction::MovePrevious => MOVE_PREVIOUS_ACTION_INDEX,
            InputAction::MoveNext => MOVE_NEXT_ACTION_INDEX,
            InputAction::Accept => ACCEPT_ACTION_INDEX,
            InputAction::Cancel => CANCEL_ACTION_INDEX,
            InputAction::LatchTextByte(_) => TEXT_ACTION_INDEX,
            InputAction::TogglePause(_) => TOGGLE_PAUSE_ACTION_INDEX,
            InputAction::Ignored(IgnoredInputAction::MoveRight) => MOVE_RIGHT_ACTION_INDEX,
            InputAction::Ignored(IgnoredInputAction::MoveLeft) => MOVE_LEFT_ACTION_INDEX,
            InputAction::Ignored(IgnoredInputAction::Function(function)) => match function {
                InputFunctionKey::F1 => FUNCTION_ONE_ACTION_INDEX,
                InputFunctionKey::F2 => FUNCTION_TWO_ACTION_INDEX,
                InputFunctionKey::F3 => FUNCTION_THREE_ACTION_INDEX,
                InputFunctionKey::F4 => FUNCTION_FOUR_ACTION_INDEX,
                InputFunctionKey::F5 => FUNCTION_FIVE_ACTION_INDEX,
                InputFunctionKey::F6 => FUNCTION_SIX_ACTION_INDEX,
                InputFunctionKey::F7 => FUNCTION_SEVEN_ACTION_INDEX,
            },
        }
    }

    const fn request_shutdown_action_index() -> u8 {
        4
    }
}
