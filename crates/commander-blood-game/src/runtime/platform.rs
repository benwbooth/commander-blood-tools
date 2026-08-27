//! SDL event handling and recovered game-clock pacing for the modern runtime.

use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use sdl3::EventPump;
use sdl3::event::{Event, WindowEvent};
use sdl3::mouse::{MouseButton, MouseState};
use sdl3::video::Window;

use crate::native::bloodprg::{
    GameLifecycleState, InputAction, PointerButton, PointerButtons, PointerSample,
};

use super::{ModernGameServices, RuntimeAlienOverlayFrameInput};

/// Input frequency of the IBM PC programmable interval timer.
const PIT_INPUT_FREQUENCY_HZ: u64 = 1_193_182;
/// Divisor programmed by the recovered timer setup routine.
const GAME_TIMER_DIVISOR: u64 = 5_958;
/// Timer interrupts assigned to one main-loop frame by `bloodprg_main`.
const GAME_FRAME_TIMER_TICKS: u64 = 8;
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
const MINIMUM_SURFACE_DIMENSION: u32 = 1;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const LOGICAL_SCREEN_WIDTH: f32 = 320.0;
const ALIEN_DRIVER_WIDTH: f32 = 640.0;
const ALIEN_DRIVER_HEIGHT: f32 = 1_024.0;
const ALIEN_DRIVER_CENTER: [f32; 2] = [ALIEN_DRIVER_WIDTH / 2.0, ALIEN_DRIVER_HEIGHT / 2.0];

/// Exact recovered target duration of one original game update.
///
/// `BLOODPRG.EXE` programs PIT divisor 5,958 and reloads an eight-interrupt
/// frame counter at the head of its main loop. The rounded nanosecond duration
/// is about 39.95 ms, or 25.03 game updates per second.
pub const GAME_FRAME_DURATION: Duration = Duration::from_nanos(
    (GAME_TIMER_DIVISOR * GAME_FRAME_TIMER_TICKS * NANOSECONDS_PER_SECOND
        + PIT_INPUT_FREQUENCY_HZ / 2)
        / PIT_INPUT_FREQUENCY_HZ,
);

/// SDL-facing state owned by the production game lifecycle host.
pub struct RuntimePlatformHost<'window> {
    window: &'window Window,
    events: EventPump,
    frame_clock: GameFrameClock,
    bridge_horizontal_delta: f32,
    alien_pointer: Option<[f32; 2]>,
}

impl<'window> RuntimePlatformHost<'window> {
    /// Bind the SDL event pump to the game window without taking cursor control.
    pub fn new(window: &'window Window, events: EventPump) -> Self {
        Self {
            window,
            events,
            frame_clock: GameFrameClock::default(),
            bridge_horizontal_delta: 0.0,
            alien_pointer: None,
        }
    }

    /// Pump pending SDL events and dispatch one translated input action.
    ///
    /// A new recovered frame budget starts before game work, matching the
    /// original main loop. Escape remains an ordinary translated game input;
    /// only an SDL quit request directly asks the lifecycle to shut down.
    pub fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Option<InputAction> {
        self.frame_clock.begin_frame(Instant::now());
        self.pump_events(services);
        services.dispatch_lifecycle_input(state)
    }

    /// Pump one synchronous alien-overlay frame without dispatching ordinary UI actions.
    pub fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput> {
        self.frame_clock.begin_frame(Instant::now());
        let platform_shutdown = self.pump_events(services);
        let mouse = self.events.mouse_state();
        let key_events = services
            .input_mut()
            .drain_alien_key_events(platform_shutdown);
        let pointer = self
            .alien_pointer
            .context("alien pointer was not centered before overlay entry")?;
        Ok(RuntimeAlienOverlayFrameInput::from_driver_pointer(
            pointer,
            pointer_buttons(&mouse),
            key_events,
        ))
    }

    /// Center the overlay's virtual mouse driver without moving the real cursor.
    pub fn begin_alien_overlay_input(&mut self) -> Result<()> {
        if self.alien_pointer.is_some() {
            bail!("alien-overlay input is already active");
        }
        self.alien_pointer = Some(ALIEN_DRIVER_CENTER);
        Ok(())
    }

    /// Release the temporary virtual pointer after the XDB loop exits.
    pub fn finish_alien_overlay_input(&mut self) -> bool {
        self.alien_pointer.take().is_some()
    }

    fn pump_events(&mut self, services: &mut ModernGameServices<'window>) -> bool {
        let window_id = self.window.id();
        let mut platform_shutdown = false;
        for event in self.events.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    services.input_mut().request_shutdown();
                    platform_shutdown = true;
                }
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::PixelSizeChanged(_, _) | WindowEvent::Resized(_, _),
                    ..
                } if event_window_id == window_id => {
                    let (width, height) = self.window.size_in_pixels();
                    services.resize(
                        width.max(MINIMUM_SURFACE_DIMENSION),
                        height.max(MINIMUM_SURFACE_DIMENSION),
                    );
                }
                Event::MouseMotion {
                    window_id: event_window_id,
                    xrel,
                    yrel,
                    ..
                } if event_window_id == window_id => {
                    let (width, height) = self.window.size();
                    let output_size = [width as f32, height as f32];
                    if let Some(pointer) = &mut self.alien_pointer {
                        let delta = map_motion_to_alien_driver(output_size, [xrel, yrel]);
                        pointer[0] = (pointer[0] + delta[0]).clamp(0.0, ALIEN_DRIVER_WIDTH);
                        pointer[1] = (pointer[1] + delta[1]).clamp(0.0, ALIEN_DRIVER_HEIGHT);
                    } else {
                        self.bridge_horizontal_delta +=
                            map_horizontal_delta_to_logical(output_size, xrel);
                    }
                }
                Event::KeyDown {
                    window_id: event_window_id,
                    keycode: Some(keycode),
                    ..
                } if event_window_id == window_id => {
                    services.input_mut().queue_keycode(keycode);
                }
                Event::TextInput {
                    window_id: event_window_id,
                    text,
                    ..
                } if event_window_id == window_id => {
                    services.input_mut().queue_text(&text);
                }
                _ => {}
            }
        }
        platform_shutdown
    }

    /// Sample the current SDL pointer into the original logical viewport.
    pub fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample {
        let mouse = self.events.mouse_state();
        let (width, height) = self.window.size();
        services.poll_lifecycle_pointer(
            [width as f32, height as f32],
            [mouse.x(), mouse.y()],
            pointer_buttons(&mouse),
        )
    }

    /// Consume relative horizontal mouse motion in original logical pixels.
    pub fn take_bridge_horizontal_delta(&mut self) -> i32 {
        let delta = self.bridge_horizontal_delta.round() as i32;
        self.bridge_horizontal_delta = 0.0;
        delta
    }

    /// Sleep only for the unused portion of the current recovered frame budget.
    pub fn pace_frame(&mut self) -> Result<()> {
        let remaining = self
            .frame_clock
            .remaining(Instant::now())
            .context("game frame pacing started without a frame budget")?;
        if !remaining.is_zero() {
            thread::sleep(remaining);
        }
        self.frame_clock.finish_frame();
        Ok(())
    }
}

#[derive(Default)]
struct GameFrameClock {
    deadline: Option<Instant>,
}

impl GameFrameClock {
    fn begin_frame(&mut self, now: Instant) {
        self.deadline = Some(now + GAME_FRAME_DURATION);
    }

    fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    fn finish_frame(&mut self) {
        self.deadline = None;
    }
}

fn pointer_buttons(mouse: &MouseState) -> PointerButtons {
    let mut buttons = PointerButtons::NONE.bits();
    if mouse.is_mouse_button_pressed(MouseButton::Left) {
        buttons |= PointerButton::Primary as u16;
    }
    if mouse.is_mouse_button_pressed(MouseButton::Right) {
        buttons |= PointerButton::Secondary as u16;
    }
    PointerButtons::from_bits(buttons)
}

fn map_horizontal_delta_to_logical(output_size: [f32; 2], horizontal_delta: f32) -> f32 {
    let output_width = output_size[0].max(1.0);
    let output_height = output_size[1].max(1.0);
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    horizontal_delta * LOGICAL_SCREEN_WIDTH / viewport_width
}

fn map_motion_to_alien_driver(output_size: [f32; 2], motion: [f32; 2]) -> [f32; 2] {
    let output_width = output_size[0].max(1.0);
    let output_height = output_size[1].max(1.0);
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    let viewport_height = ORIGINAL_DISPLAY_ASPECT_HEIGHT * scale;
    [
        motion[0] * ALIEN_DRIVER_WIDTH / viewport_width,
        motion[1] * ALIEN_DRIVER_HEIGHT / viewport_height,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDESCREEN_OUTPUT: [f32; 2] = [1_920.0, 1_080.0];
    const WIDESCREEN_VIEWPORT_WIDTH: f32 = 1_440.0;
    const EXPECTED_FRAME_NANOSECONDS: u64 = 39_946_965;

    #[test]
    fn frame_duration_comes_from_the_recovered_pit_programming() {
        assert_eq!(
            GAME_FRAME_DURATION.as_nanos(),
            EXPECTED_FRAME_NANOSECONDS as u128
        );
        let update_rate = 1.0 / GAME_FRAME_DURATION.as_secs_f64();
        assert!((update_rate - 25.03).abs() < 0.01);
    }

    #[test]
    fn each_frame_gets_an_independent_budget_without_catch_up_bursts() {
        let start = Instant::now();
        let mut clock = GameFrameClock::default();
        clock.begin_frame(start);
        assert_eq!(clock.remaining(start), Some(GAME_FRAME_DURATION));
        assert_eq!(
            clock.remaining(start + GAME_FRAME_DURATION + Duration::from_millis(1)),
            Some(Duration::ZERO)
        );

        let next_start = start + GAME_FRAME_DURATION + Duration::from_millis(1);
        clock.begin_frame(next_start);
        assert_eq!(clock.remaining(next_start), Some(GAME_FRAME_DURATION));
    }

    #[test]
    fn relative_mouse_motion_scales_through_the_letterboxed_viewport() {
        assert_eq!(
            map_horizontal_delta_to_logical(WIDESCREEN_OUTPUT, WIDESCREEN_VIEWPORT_WIDTH,),
            LOGICAL_SCREEN_WIDTH
        );
    }

    #[test]
    fn alien_mouse_motion_scales_to_its_virtual_driver_without_cursor_warping() {
        assert_eq!(
            map_motion_to_alien_driver(
                WIDESCREEN_OUTPUT,
                [WIDESCREEN_VIEWPORT_WIDTH, WIDESCREEN_OUTPUT[1]],
            ),
            [ALIEN_DRIVER_WIDTH, ALIEN_DRIVER_HEIGHT]
        );
    }
}
