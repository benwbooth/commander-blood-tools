//! Flat SDL input state shared by the translated game lifecycle and scene hosts.

use std::collections::VecDeque;

use sdl3::keyboard::Keycode;

use crate::native::bloodprg::{
    GameLifecycleState, HostInputKey, InputAction, InputArrowKey, InputDispatchState,
    InputFunctionKey, PointerButtonEdges, PointerButtonState, PointerButtons, PointerSample,
    PointerSampleState, dispatch_input_key, latch_input_text_byte, request_input_shutdown,
    toggle_input_pause, translate_input_key, update_pointer_button_edges, update_pointer_sample,
};

const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const LOGICAL_SCREEN_WIDTH: f32 = 320.0;
const LOGICAL_SCREEN_HEIGHT: f32 = 200.0;
const CENTERING_DIVISOR: f32 = 2.0;
const MINIMUM_OUTPUT_DIMENSION: f32 = 1.0;
const POINTER_PRESS_LATCHED: u8 = 1;
const BIOS_ESCAPE_KEY: u16 = 0x011b;
const BIOS_BACKSPACE_KEY: u16 = 0x0e08;
const BIOS_ENTER_KEY: u16 = 0x1c0d;
const BIOS_SPACE_KEY: u16 = 0x3920;
const BIOS_DELETE_KEY: u16 = 0x5300;
const BIOS_ARROW_UP_KEY: u16 = 0x4800;
const BIOS_ARROW_DOWN_KEY: u16 = 0x5000;
const BIOS_ARROW_LEFT_KEY: u16 = 0x4b00;
const BIOS_ARROW_RIGHT_KEY: u16 = 0x4d00;
const BIOS_FUNCTION_KEY_BASE: u16 = 0x3b00;
const BIOS_SCAN_CODE_SHIFT: u32 = 8;
const BIOS_P_SCAN_CODE: u16 = 0x19;
const ASCII_ESCAPE: u8 = 27;

/// Logical pointer position installed by `bloodprg_main` after line zero.
///
/// The DOS mouse driver receives `(720, 150)`. Bridge steering translates the
/// horizontal ring coordinate back to screen x 160 before MANU3 consumes it.
pub(super) const INITIAL_LOGICAL_POINTER: [i16; 2] = [160, 150];

/// SDL-facing input state with typed keys and logical pointer coordinates.
///
/// Key events remain queued until the translated main loop consumes them. This
/// avoids dropping input when SDL publishes multiple events during one game
/// frame while preserving the original one-key-per-dispatch behavior.
#[derive(Debug)]
pub struct RuntimeInputHost {
    pending_keys: VecDeque<HostInputKey>,
    dispatch: InputDispatchState,
    pointer_sample: PointerSampleState,
    pointer_buttons: PointerButtonState,
    motion_idle_counter: u16,
}

impl RuntimeInputHost {
    /// Construct input state at one logical pointer position.
    pub fn new(initial_position: [i16; 2]) -> Self {
        let pointer_sample = PointerSample {
            position: initial_position,
            buttons: PointerButtons::NONE,
        };
        Self {
            pending_keys: VecDeque::new(),
            dispatch: InputDispatchState::default(),
            pointer_sample: PointerSampleState {
                current: pointer_sample,
                previous_position: initial_position,
            },
            pointer_buttons: PointerButtonState::default(),
            motion_idle_counter: u16::MIN,
        }
    }

    /// Queue one SDL key whose meaning is independent of keyboard layout text.
    pub fn queue_keycode(&mut self, keycode: Keycode) -> bool {
        let Some(key) = host_key_for_sdl_keycode(keycode) else {
            return false;
        };
        self.pending_keys.push_back(key);
        true
    }

    /// Queue printable text from one SDL text-input event.
    pub fn queue_text(&mut self, text: &str) -> usize {
        let prior_count = self.pending_keys.len();
        self.pending_keys
            .extend(text.chars().filter_map(|character| {
                let key = HostInputKey::Character(character);
                translate_input_key(key).map(|_| key)
            }));
        self.pending_keys.len() - prior_count
    }

    /// Publish a window-close or other platform shutdown request.
    pub fn request_shutdown(&mut self) {
        request_input_shutdown(&mut self.dispatch);
    }

    /// Dispatch at most one queued key and update recovered input latches.
    pub fn dispatch_next(&mut self, save_menu_active: bool) -> Option<InputAction> {
        let action = dispatch_input_key(&mut self.dispatch, self.pending_keys.pop_front());
        match action {
            Some(InputAction::LatchTextByte(text_byte)) => {
                latch_input_text_byte(&mut self.dispatch, text_byte);
            }
            Some(InputAction::TogglePause(text_byte)) => {
                toggle_input_pause(&mut self.dispatch, save_menu_active, text_byte);
            }
            _ => {}
        }
        action
    }

    /// Dispatch one key and publish pause and shutdown latches to the main lifecycle.
    pub fn dispatch_lifecycle_input(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Option<InputAction> {
        let action = self.dispatch_next(state.profile_change_blockers.save_active);
        state.pause_hud_active = self.dispatch.paused;
        state.exit_requested |= self.dispatch.shutdown_requested;
        action
    }

    /// Drain queued host keys as the BIOS words consumed by an alien XDB loop.
    ///
    /// Unlike ordinary lifecycle dispatch, the overlay drains every available
    /// key after each rendered frame. A platform shutdown contributes Escape
    /// so the synchronous overlay returns before the outer lifecycle handles
    /// its already-latched shutdown request.
    pub fn drain_alien_key_events(&mut self, platform_shutdown: bool) -> Vec<u16> {
        let mut events: Vec<_> = self.pending_keys.drain(..).map(alien_bios_key).collect();
        if platform_shutdown && !events.iter().any(|event| *event as u8 == ASCII_ESCAPE) {
            events.push(BIOS_ESCAPE_KEY);
        }
        events
    }

    /// Current translated key, pause, and shutdown latches.
    pub const fn dispatch_state(&self) -> &InputDispatchState {
        &self.dispatch
    }

    /// Number of SDL keys still waiting for main-loop dispatch.
    pub fn pending_key_count(&self) -> usize {
        self.pending_keys.len()
    }

    /// Sample a host pointer into the original 320 by 200 logical surface.
    pub fn poll_pointer(
        &mut self,
        output_size: [f32; 2],
        host_position: [f32; 2],
        buttons: PointerButtons,
    ) -> PointerSample {
        self.publish_logical_pointer(
            map_host_pointer_to_logical(output_size, host_position),
            buttons,
        )
    }

    /// Publish a pointer already expressed in the original logical surface.
    pub fn publish_logical_pointer(
        &mut self,
        position: [i16; 2],
        buttons: PointerButtons,
    ) -> PointerSample {
        let sample = PointerSample { position, buttons };
        update_pointer_sample(
            &mut self.pointer_sample,
            sample,
            &mut self.motion_idle_counter,
        );
        sample
    }

    /// Update persistent primary and secondary press latches.
    pub fn update_pointer_buttons(&mut self) -> PointerButtonEdges {
        update_pointer_button_edges(
            &mut self.pointer_buttons,
            self.pointer_sample.current.buttons,
        );
        self.pointer_buttons.edges
    }

    /// Clear pointer presses after the owning interaction consumes them.
    pub fn consume_pointer_presses(&mut self) -> PointerButtonEdges {
        let edges = self.pointer_buttons.edges;
        self.pointer_buttons.edges = PointerButtonEdges::default();
        edges
    }

    /// Transfer newly detected pointer edges into the lifecycle's one-frame latches.
    pub fn transfer_lifecycle_pointer_edges(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> PointerButtonEdges {
        self.update_pointer_buttons();
        let edges = self.consume_pointer_presses();
        state.primary_pointer_pressed |= edges.primary_pressed;
        state.secondary_pointer_pressed |= edges.secondary_pressed;
        if edges.press_pending {
            state.pointer_press_pending = POINTER_PRESS_LATCHED;
        }
        edges
    }

    /// Most recent complete logical pointer sample.
    pub const fn pointer_sample(&self) -> PointerSample {
        self.pointer_sample.current
    }

    /// Frames or ticks retained since the latest logical pointer movement.
    pub const fn motion_idle_counter(&self) -> u16 {
        self.motion_idle_counter
    }

    /// Increment the movement-idle counter using native wrapping semantics.
    pub fn advance_motion_idle_counter(&mut self) {
        self.motion_idle_counter = self.motion_idle_counter.wrapping_add(1);
    }
}

/// Map one SDL pointer position through the aspect-correct 4:3 viewport.
pub fn map_host_pointer_to_logical(output_size: [f32; 2], host_position: [f32; 2]) -> [i16; 2] {
    let output_width = output_size[0].max(MINIMUM_OUTPUT_DIMENSION);
    let output_height = output_size[1].max(MINIMUM_OUTPUT_DIMENSION);
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    let viewport_height = ORIGINAL_DISPLAY_ASPECT_HEIGHT * scale;
    let viewport_x = (output_width - viewport_width) / CENTERING_DIVISOR;
    let viewport_y = (output_height - viewport_height) / CENTERING_DIVISOR;
    let x = ((host_position[0] - viewport_x) * LOGICAL_SCREEN_WIDTH / viewport_width)
        .clamp(0.0, LOGICAL_SCREEN_WIDTH - 1.0);
    let y = ((host_position[1] - viewport_y) * LOGICAL_SCREEN_HEIGHT / viewport_height)
        .clamp(0.0, LOGICAL_SCREEN_HEIGHT - 1.0);
    [x as i16, y as i16]
}

fn host_key_for_sdl_keycode(keycode: Keycode) -> Option<HostInputKey> {
    match keycode {
        Keycode::Backspace | Keycode::KpBackspace => Some(HostInputKey::Backspace),
        Keycode::Return | Keycode::Return2 | Keycode::KpEnter => Some(HostInputKey::Enter),
        Keycode::Escape => Some(HostInputKey::Escape),
        Keycode::Space => Some(HostInputKey::Space),
        Keycode::Delete => Some(HostInputKey::Delete),
        Keycode::Up => Some(HostInputKey::Arrow(InputArrowKey::Up)),
        Keycode::Down => Some(HostInputKey::Arrow(InputArrowKey::Down)),
        Keycode::Left => Some(HostInputKey::Arrow(InputArrowKey::Left)),
        Keycode::Right => Some(HostInputKey::Arrow(InputArrowKey::Right)),
        Keycode::F1 => Some(HostInputKey::Function(InputFunctionKey::F1)),
        Keycode::F2 => Some(HostInputKey::Function(InputFunctionKey::F2)),
        Keycode::F3 => Some(HostInputKey::Function(InputFunctionKey::F3)),
        Keycode::F4 => Some(HostInputKey::Function(InputFunctionKey::F4)),
        Keycode::F5 => Some(HostInputKey::Function(InputFunctionKey::F5)),
        Keycode::F6 => Some(HostInputKey::Function(InputFunctionKey::F6)),
        Keycode::F7 => Some(HostInputKey::Function(InputFunctionKey::F7)),
        _ => None,
    }
}

fn alien_bios_key(key: HostInputKey) -> u16 {
    match key {
        HostInputKey::Character(character) => {
            let ascii = character as u8;
            let scan_code = u16::from(matches!(character, 'p' | 'P')) * BIOS_P_SCAN_CODE;
            scan_code << BIOS_SCAN_CODE_SHIFT | u16::from(ascii)
        }
        HostInputKey::Backspace => BIOS_BACKSPACE_KEY,
        HostInputKey::Enter => BIOS_ENTER_KEY,
        HostInputKey::Escape => BIOS_ESCAPE_KEY,
        HostInputKey::Space => BIOS_SPACE_KEY,
        HostInputKey::Delete => BIOS_DELETE_KEY,
        HostInputKey::Arrow(InputArrowKey::Up) => BIOS_ARROW_UP_KEY,
        HostInputKey::Arrow(InputArrowKey::Down) => BIOS_ARROW_DOWN_KEY,
        HostInputKey::Arrow(InputArrowKey::Left) => BIOS_ARROW_LEFT_KEY,
        HostInputKey::Arrow(InputArrowKey::Right) => BIOS_ARROW_RIGHT_KEY,
        HostInputKey::Function(function) => {
            BIOS_FUNCTION_KEY_BASE + (function as u16) * (1 << BIOS_SCAN_CODE_SHIFT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{IgnoredInputAction, PointerButton};

    const INITIAL_POSITION: [i16; 2] = [160, 100];
    const WIDESCREEN_OUTPUT: [f32; 2] = [1_920.0, 1_080.0];
    const WIDESCREEN_CENTER: [f32; 2] = [960.0, 540.0];

    #[test]
    fn key_bursts_remain_ordered_across_dispatch_frames() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        assert!(input.queue_keycode(Keycode::Up));
        assert!(input.queue_keycode(Keycode::Down));
        assert!(input.queue_keycode(Keycode::F3));

        assert_eq!(input.dispatch_next(false), Some(InputAction::MovePrevious));
        assert_eq!(input.pending_key_count(), 2);
        assert_eq!(input.dispatch_next(false), Some(InputAction::MoveNext));
        assert_eq!(
            input.dispatch_next(false),
            Some(InputAction::Ignored(IgnoredInputAction::Function(
                InputFunctionKey::F3
            )))
        );
        assert_eq!(input.dispatch_next(false), None);
    }

    #[test]
    fn text_input_filters_unicode_but_preserves_authored_ascii() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        assert_eq!(input.queue_text("A p\u{00e9}"), 2);

        assert_eq!(
            input.dispatch_next(false),
            Some(InputAction::LatchTextByte(b'A'))
        );
        assert_eq!(input.dispatch_state().text_byte, Some(b'A'));
        assert_eq!(
            input.dispatch_next(false),
            Some(InputAction::TogglePause(b'p'))
        );
        assert!(input.dispatch_state().paused);
        assert_eq!(input.dispatch_state().text_byte, Some(b'p'));
        assert_eq!(input.dispatch_next(false), None);
        assert_eq!(input.dispatch_state().text_byte, None);
    }

    #[test]
    fn active_save_menu_blocks_pause_without_losing_the_text_latch() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        assert_eq!(input.queue_text("P"), 1);
        assert_eq!(
            input.dispatch_next(true),
            Some(InputAction::TogglePause(b'P'))
        );
        assert!(!input.dispatch_state().paused);
        assert_eq!(input.dispatch_state().text_byte, Some(b'P'));
    }

    #[test]
    fn lifecycle_dispatch_receives_pause_and_shutdown_without_losing_the_action() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        let mut lifecycle = GameLifecycleState::default();
        assert_eq!(input.queue_text("P"), usize::from(true));

        assert_eq!(
            input.dispatch_lifecycle_input(&mut lifecycle),
            Some(InputAction::TogglePause(b'P'))
        );
        assert!(lifecycle.pause_hud_active);
        assert!(!lifecycle.exit_requested);

        input.request_shutdown();
        assert_eq!(input.dispatch_lifecycle_input(&mut lifecycle), None);
        assert!(lifecycle.pause_hud_active);
        assert!(lifecycle.exit_requested);
    }

    #[test]
    fn alien_overlay_drains_bios_words_in_arrival_order() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        assert!(input.queue_keycode(Keycode::Up));
        assert!(input.queue_keycode(Keycode::Space));
        assert_eq!(input.queue_text("p"), 1);

        assert_eq!(
            input.drain_alien_key_events(false),
            [BIOS_ARROW_UP_KEY, BIOS_SPACE_KEY, 0x1970]
        );
        assert_eq!(input.pending_key_count(), 0);
    }

    #[test]
    fn platform_shutdown_exits_an_alien_overlay_without_duplicate_escape() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        assert_eq!(input.drain_alien_key_events(true), [BIOS_ESCAPE_KEY]);

        assert!(input.queue_keycode(Keycode::Escape));
        assert_eq!(input.drain_alien_key_events(true), [BIOS_ESCAPE_KEY]);
    }

    #[test]
    fn pointer_mapping_follows_the_letterboxed_original_surface() {
        assert_eq!(
            map_host_pointer_to_logical(WIDESCREEN_OUTPUT, WIDESCREEN_CENTER),
            INITIAL_POSITION
        );
        assert_eq!(
            map_host_pointer_to_logical(WIDESCREEN_OUTPUT, [0.0, 0.0]),
            [0, 0]
        );
        assert_eq!(
            map_host_pointer_to_logical(WIDESCREEN_OUTPUT, WIDESCREEN_OUTPUT),
            [319, 199]
        );
    }

    #[test]
    fn pointer_polling_retains_native_edge_latches_until_consumed() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        input.poll_pointer(
            WIDESCREEN_OUTPUT,
            WIDESCREEN_CENTER,
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        let edges = input.update_pointer_buttons();
        assert!(edges.primary_pressed);
        assert!(edges.press_pending);
        assert_eq!(input.update_pointer_buttons(), edges);
        assert_eq!(input.consume_pointer_presses(), edges);
        assert_eq!(
            input.consume_pointer_presses(),
            PointerButtonEdges::default()
        );
    }

    #[test]
    fn logical_pointer_publication_bypasses_host_viewport_mapping() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        let expected = [319, 150];
        let sample = input.publish_logical_pointer(expected, PointerButtons::NONE);

        assert_eq!(sample.position, expected);
        assert_eq!(input.pointer_sample(), sample);
    }

    #[test]
    fn lifecycle_pointer_transfer_owns_each_new_press_once() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        let mut lifecycle = GameLifecycleState::default();
        input.poll_pointer(
            WIDESCREEN_OUTPUT,
            WIDESCREEN_CENTER,
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );

        let primary = input.transfer_lifecycle_pointer_edges(&mut lifecycle);
        assert!(primary.primary_pressed);
        assert!(lifecycle.primary_pointer_pressed);
        assert!(!lifecycle.secondary_pointer_pressed);
        assert_eq!(lifecycle.pointer_press_pending, POINTER_PRESS_LATCHED);
        assert_eq!(
            input.consume_pointer_presses(),
            PointerButtonEdges::default()
        );

        lifecycle.primary_pointer_pressed = false;
        lifecycle.pointer_press_pending = u8::MIN;
        input.poll_pointer(WIDESCREEN_OUTPUT, WIDESCREEN_CENTER, PointerButtons::NONE);
        assert_eq!(
            input.transfer_lifecycle_pointer_edges(&mut lifecycle),
            PointerButtonEdges::default()
        );
        input.poll_pointer(
            WIDESCREEN_OUTPUT,
            WIDESCREEN_CENTER,
            PointerButtons::from_bits(PointerButton::Secondary as u16),
        );
        let secondary = input.transfer_lifecycle_pointer_edges(&mut lifecycle);
        assert!(secondary.secondary_pressed);
        assert!(!lifecycle.primary_pointer_pressed);
        assert!(lifecycle.secondary_pointer_pressed);
        assert_eq!(lifecycle.pointer_press_pending, POINTER_PRESS_LATCHED);
    }

    #[test]
    fn shutdown_is_an_explicit_host_latch() {
        let mut input = RuntimeInputHost::new(INITIAL_POSITION);
        input.request_shutdown();
        assert!(input.dispatch_state().shutdown_requested);
    }
}
