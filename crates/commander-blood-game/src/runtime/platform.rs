//! SDL event handling and recovered game-clock pacing for the modern runtime.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use sdl3::EventPump;
use sdl3::event::{Event, WindowEvent};
use sdl3::mouse::{MouseButton, MouseUtil};
use sdl3::video::Window;

use crate::native::bloodprg::{
    GameLifecycleState, InputAction, PointerButton, PointerButtons, PointerSample,
};

use super::input::{INITIAL_LOGICAL_POINTER, map_host_pointer_to_logical};
use super::scenario::{RuntimeScenarioCadence, RuntimeScenarioDriver, RuntimeScenarioKey};
use super::{ModernGameServices, RuntimeAlienOverlayFrameInput};

/// Input frequency of the IBM PC programmable interval timer.
const PIT_INPUT_FREQUENCY_HZ: u64 = 1_193_182;
/// Divisor programmed by the recovered timer setup routine.
const GAME_TIMER_DIVISOR: u64 = 5_958;
/// Timer interrupts assigned to one main-loop frame by `bloodprg_main`.
const GAME_FRAME_TIMER_TICKS: u64 = 8;
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
const MEASURED_GAME_FRAME_MILLISECONDS: u64 = 46;
const MEASURED_PRESENTATION_FRAME_MILLISECONDS: u64 = 68;
const MODERN_VISUAL_REFRESH_HZ: u64 = 60;
const MINIMUM_SURFACE_DIMENSION: u32 = 1;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const LOGICAL_SCREEN_WIDTH: f32 = 320.0;
const LOGICAL_SCREEN_HEIGHT: f32 = 200.0;
const BRIDGE_EDGE_SCROLL_ZONE_WIDTH: f32 = 32.0;
const BRIDGE_EDGE_SCROLL_MINIMUM_DELTA: f32 = 8.0;
const BRIDGE_EDGE_SCROLL_MAXIMUM_DELTA: f32 = 32.0;
const ALIEN_DRIVER_WIDTH: f32 = 640.0;
const ALIEN_DRIVER_HEIGHT: f32 = 1_024.0;
const ALIEN_DRIVER_CENTER: [f32; 2] = [ALIEN_DRIVER_WIDTH / 2.0, ALIEN_DRIVER_HEIGHT / 2.0];

/// Exact recovered timer budget of one original game update.
///
/// `BLOODPRG.EXE` programs PIT divisor 5,958 and reloads an eight-interrupt
/// frame counter at the head of its main loop. The rounded nanosecond duration
/// is about 39.95 ms, or 25.03 game updates per second. The shipped game
/// regularly overruns this budget on its target hardware.
pub const RECOVERED_FRAME_BUDGET: Duration = Duration::from_nanos(
    (GAME_TIMER_DIVISOR * GAME_FRAME_TIMER_TICKS * NANOSECONDS_PER_SECOND
        + PIT_INPUT_FREQUENCY_HZ / 2)
        / PIT_INPUT_FREQUENCY_HZ,
);

/// Measured duration of a presented gameplay frame in the DOS runtime.
///
/// The binary-oracle page-flip probe records approximately 21.6 presented
/// frames per second at the bridge hub. Simulation follows that observed
/// cadence; modern rendering can interpolate between simulation updates
/// independently when a higher visual frame rate is added.
pub const GAME_FRAME_DURATION: Duration = Duration::from_millis(MEASURED_GAME_FRAME_MILLISECONDS);

/// Measured duration of one software-clocked HNM presentation frame.
///
/// A dense DOSBox-X oracle capture places the 263-frame `MIND.HNM` stream at
/// approximately 18 seconds, including the original decoder's workload. This
/// cadence is intentionally separate from ordinary bridge simulation updates.
pub const PRESENTATION_FRAME_DURATION: Duration =
    Duration::from_millis(MEASURED_PRESENTATION_FRAME_MILLISECONDS);

/// Render-only refresh interval used between recovered C simulation ticks.
pub const VISUAL_FRAME_DURATION: Duration =
    Duration::from_nanos(NANOSECONDS_PER_SECOND / MODERN_VISUAL_REFRESH_HZ);

/// SDL-facing state owned by the production game lifecycle host.
pub struct RuntimePlatformHost<'window> {
    window: &'window Window,
    mouse: MouseUtil,
    events: EventPump,
    frame_clock: GameFrameClock,
    bridge_horizontal_delta: f32,
    pointer_buttons: PointerButtons,
    logical_pointer: [f32; 2],
    pointer_inside_window: bool,
    alien_pointer: Option<[f32; 2]>,
    scenario: Option<RuntimeScenarioDriver>,
}

impl<'window> RuntimePlatformHost<'window> {
    /// Bind SDL input without capturing the desktop pointer.
    ///
    /// The ordinary bridge uses the host pointer directly. Relative capture is
    /// scoped to the synchronous alien overlay, whose recovered driver consumes
    /// unbounded motion rather than a window position.
    pub fn new(window: &'window Window, mouse: MouseUtil, events: EventPump) -> Self {
        mouse.set_relative_mouse_mode(window, false);
        Self {
            window,
            mouse,
            events,
            frame_clock: GameFrameClock::default(),
            bridge_horizontal_delta: 0.0,
            pointer_buttons: PointerButtons::NONE,
            logical_pointer: INITIAL_LOGICAL_POINTER.map(f32::from),
            pointer_inside_window: false,
            alien_pointer: None,
            scenario: None,
        }
    }

    /// Bind SDL for a deterministic, uncaptured original-oracle scenario.
    pub fn new_scripted(
        window: &'window Window,
        mouse: MouseUtil,
        events: EventPump,
        scenario_path: &Path,
        trace_path: &Path,
    ) -> Result<Self> {
        let mut platform = Self::new(window, mouse, events);
        platform.pointer_inside_window = true;
        platform.scenario = Some(RuntimeScenarioDriver::load(scenario_path, trace_path)?);
        Ok(platform)
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
    ) -> Result<Option<InputAction>> {
        self.dispatch_events_with_boundary(services, state, true)
    }

    /// Pump one ordinary game frame, whose action boundary is recorded at frame end.
    pub fn dispatch_game_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        self.dispatch_events_with_boundary(services, state, false)
    }

    fn dispatch_events_with_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
        record_completed_boundary: bool,
    ) -> Result<Option<InputAction>> {
        self.frame_clock.begin_frame(Instant::now());
        self.pump_events(services);
        if self.scenario.is_some() {
            let semantic = services.semantic_trace_snapshot(state)?;
            let scenario = self
                .scenario
                .as_mut()
                .expect("scenario presence was checked");
            let finished = if record_completed_boundary {
                scenario.record_due_boundaries(&semantic)?
            } else {
                scenario.record_initial_boundary(&semantic)?;
                false
            };
            if finished {
                self.pointer_buttons = PointerButtons::NONE;
                services.input_mut().request_shutdown();
            } else {
                let bridge_frame = services.current_bridge_view_frame();
                let input = self
                    .scenario
                    .as_mut()
                    .expect("scenario presence was checked")
                    .advance(
                        bridge_frame,
                        if record_completed_boundary {
                            RuntimeScenarioCadence::BlockingPresentation
                        } else {
                            RuntimeScenarioCadence::GameLoop
                        },
                    )?;
                if let Some(position) = input.pointer_position {
                    self.logical_pointer = position.map(f32::from);
                    self.pointer_inside_window = true;
                }
                self.pointer_buttons = if input.primary_pressed {
                    PointerButtons::from_bits(PointerButton::Primary as u16)
                } else {
                    PointerButtons::NONE
                };
                if let Some(key) = input.key {
                    queue_scenario_key(services, key);
                }
                if input.request_shutdown {
                    services.input_mut().request_shutdown();
                }
            }
        }
        Ok(services.dispatch_lifecycle_input(state))
    }

    /// Record a completed scripted action after the ordinary frame tail.
    pub fn record_scenario_frame_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        let Some(scenario) = self.scenario.as_mut() else {
            return Ok(());
        };
        let semantic = services.semantic_trace_snapshot(state)?;
        if scenario.record_due_boundaries(&semantic)? {
            self.pointer_buttons = PointerButtons::NONE;
            services.input_mut().request_shutdown();
        }
        Ok(())
    }

    /// Pump one synchronous alien-overlay frame without dispatching ordinary UI actions.
    pub fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput> {
        self.frame_clock.begin_frame(Instant::now());
        let platform_shutdown = self.pump_events(services);
        let key_events = services
            .input_mut()
            .drain_alien_key_events(platform_shutdown);
        let pointer = self
            .alien_pointer
            .context("alien pointer was not centered before overlay entry")?;
        Ok(RuntimeAlienOverlayFrameInput::from_driver_pointer(
            pointer,
            self.pointer_buttons,
            key_events,
        ))
    }

    /// Center the overlay's virtual mouse driver without moving the real cursor.
    pub fn begin_alien_overlay_input(&mut self) -> Result<()> {
        if self.alien_pointer.is_some() {
            bail!("alien-overlay input is already active");
        }
        self.alien_pointer = Some(ALIEN_DRIVER_CENTER);
        if self.scenario.is_none() {
            self.mouse.set_relative_mouse_mode(self.window, true);
        }
        Ok(())
    }

    /// Release the temporary virtual pointer after the XDB loop exits.
    pub fn finish_alien_overlay_input(&mut self) -> bool {
        let released = self.alien_pointer.take().is_some();
        if released && self.scenario.is_none() {
            self.mouse.set_relative_mouse_mode(self.window, false);
        }
        released
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
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::FocusLost,
                    ..
                } if event_window_id == window_id => {
                    self.pointer_inside_window = false;
                    self.pointer_buttons = PointerButtons::NONE;
                    self.mouse.set_relative_mouse_mode(self.window, false);
                }
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::FocusGained,
                    ..
                } if event_window_id == window_id => {
                    self.mouse
                        .set_relative_mouse_mode(self.window, self.alien_pointer.is_some());
                }
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::MouseEnter,
                    ..
                } if event_window_id == window_id => {
                    self.pointer_inside_window = true;
                }
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::MouseLeave,
                    ..
                } if event_window_id == window_id => {
                    self.pointer_inside_window = false;
                    self.pointer_buttons = PointerButtons::NONE;
                }
                Event::MouseMotion {
                    window_id: event_window_id,
                    x,
                    y,
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
                        self.pointer_inside_window = true;
                        let delta = map_motion_to_logical(output_size, [xrel, yrel]);
                        self.bridge_horizontal_delta += delta[0];
                        self.logical_pointer =
                            map_host_pointer_to_logical(output_size, [x, y]).map(f32::from);
                    }
                }
                Event::MouseButtonDown {
                    window_id: event_window_id,
                    mouse_btn,
                    ..
                } if event_window_id == window_id => {
                    self.pointer_inside_window = true;
                    set_pointer_button(&mut self.pointer_buttons, mouse_btn, true);
                }
                Event::MouseButtonUp {
                    window_id: event_window_id,
                    mouse_btn,
                    ..
                } if event_window_id == window_id => {
                    set_pointer_button(&mut self.pointer_buttons, mouse_btn, false);
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

    /// Publish the window-relative host pointer into the recovered input sampler.
    pub fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample {
        let buttons =
            pointer_buttons_inside_window(self.pointer_inside_window, self.pointer_buttons);
        services.publish_lifecycle_logical_pointer(self.logical_pointer(), buttons)
    }

    /// Current flat logical pointer mapped through the aspect-correct viewport.
    pub fn logical_pointer(&self) -> [i16; 2] {
        self.logical_pointer.map(|coordinate| coordinate as i16)
    }

    /// Consume horizontal mouse motion plus uncaptured edge-scroll velocity.
    pub fn take_bridge_horizontal_delta(&mut self) -> i32 {
        self.bridge_horizontal_delta += edge_scroll_delta(
            self.logical_pointer[0],
            self.pointer_inside_window && self.alien_pointer.is_none(),
        );
        take_whole_motion(&mut self.bridge_horizontal_delta)
    }

    /// Sleep only for the unused portion of the current recovered frame budget.
    pub fn pace_frame(&mut self) -> Result<()> {
        self.pace_frame_for(GAME_FRAME_DURATION)
    }

    /// Sleep for the unused portion of the measured DOS HNM presentation frame.
    pub fn pace_presentation_frame(&mut self) -> Result<()> {
        self.pace_frame_for(PRESENTATION_FRAME_DURATION)
    }

    /// Wait for one render-only refresh opportunity inside the current game tick.
    ///
    /// Returns `false` once the recovered frame deadline has been reached. SDL
    /// events are retained immediately, but lifecycle input is still dispatched
    /// only by the next C simulation frame.
    pub fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<bool> {
        if self.scenario.is_some() {
            self.frame_clock.finish_frame();
            return Ok(false);
        }
        let remaining = self
            .frame_clock
            .remaining(Instant::now(), GAME_FRAME_DURATION)
            .context("visual refresh started without a game frame budget")?;
        if remaining <= VISUAL_FRAME_DURATION {
            if !remaining.is_zero() {
                thread::sleep(remaining);
            }
            self.frame_clock.finish_frame();
            return Ok(false);
        }
        thread::sleep(VISUAL_FRAME_DURATION);
        self.pump_events(services);
        Ok(true)
    }

    fn pace_frame_for(&mut self, duration: Duration) -> Result<()> {
        if self.scenario.is_some() {
            self.frame_clock.finish_frame();
            return Ok(());
        }
        let remaining = self
            .frame_clock
            .remaining(Instant::now(), duration)
            .context("game frame pacing started without a frame budget")?;
        if !remaining.is_zero() {
            thread::sleep(remaining);
        }
        self.frame_clock.finish_frame();
        Ok(())
    }
}

fn queue_scenario_key(services: &mut ModernGameServices<'_>, key: RuntimeScenarioKey) {
    match key {
        RuntimeScenarioKey::Character(character) => {
            services.input_mut().queue_text(&character.to_string());
        }
        RuntimeScenarioKey::Enter => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Return);
        }
        RuntimeScenarioKey::Escape => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Escape);
        }
        RuntimeScenarioKey::Backspace => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Backspace);
        }
        RuntimeScenarioKey::Space => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Space);
        }
        RuntimeScenarioKey::Delete => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Delete);
        }
        RuntimeScenarioKey::ArrowUp => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Up);
        }
        RuntimeScenarioKey::ArrowDown => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Down);
        }
        RuntimeScenarioKey::ArrowLeft => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Left);
        }
        RuntimeScenarioKey::ArrowRight => {
            services
                .input_mut()
                .queue_keycode(sdl3::keyboard::Keycode::Right);
        }
    }
}

impl Drop for RuntimePlatformHost<'_> {
    fn drop(&mut self) {
        self.mouse.set_relative_mouse_mode(self.window, false);
    }
}

#[derive(Default)]
struct GameFrameClock {
    started_at: Option<Instant>,
}

impl GameFrameClock {
    fn begin_frame(&mut self, now: Instant) {
        self.started_at = Some(now);
    }

    fn remaining(&self, now: Instant, duration: Duration) -> Option<Duration> {
        self.started_at
            .map(|started_at| (started_at + duration).saturating_duration_since(now))
    }

    fn finish_frame(&mut self) {
        self.started_at = None;
    }
}

fn pointer_buttons_inside_window(
    pointer_inside_window: bool,
    buttons: PointerButtons,
) -> PointerButtons {
    if pointer_inside_window {
        buttons
    } else {
        PointerButtons::NONE
    }
}

fn set_pointer_button(buttons: &mut PointerButtons, button: MouseButton, pressed: bool) {
    let bit = match button {
        MouseButton::Left => PointerButton::Primary as u16,
        MouseButton::Right => PointerButton::Secondary as u16,
        _ => return,
    };
    let updated = if pressed {
        buttons.bits() | bit
    } else {
        buttons.bits() & !bit
    };
    *buttons = PointerButtons::from_bits(updated);
}

fn take_whole_motion(accumulated: &mut f32) -> i32 {
    let whole = accumulated.trunc();
    *accumulated -= whole;
    whole as i32
}

fn edge_scroll_delta(logical_x: f32, pointer_inside_window: bool) -> f32 {
    if !pointer_inside_window {
        return 0.0;
    }
    if logical_x < BRIDGE_EDGE_SCROLL_ZONE_WIDTH {
        return -edge_scroll_speed(BRIDGE_EDGE_SCROLL_ZONE_WIDTH - logical_x);
    }
    let right_zone_start = LOGICAL_SCREEN_WIDTH - BRIDGE_EDGE_SCROLL_ZONE_WIDTH;
    if logical_x >= right_zone_start {
        return edge_scroll_speed(logical_x - right_zone_start);
    }
    0.0
}

fn edge_scroll_speed(edge_depth: f32) -> f32 {
    let normalized = (edge_depth / BRIDGE_EDGE_SCROLL_ZONE_WIDTH).clamp(0.0, 1.0);
    BRIDGE_EDGE_SCROLL_MINIMUM_DELTA
        + normalized * (BRIDGE_EDGE_SCROLL_MAXIMUM_DELTA - BRIDGE_EDGE_SCROLL_MINIMUM_DELTA)
}

fn map_motion_to_logical(output_size: [f32; 2], motion: [f32; 2]) -> [f32; 2] {
    let output_width = output_size[0].max(1.0);
    let output_height = output_size[1].max(1.0);
    let scale = (output_width / ORIGINAL_DISPLAY_ASPECT_WIDTH)
        .min(output_height / ORIGINAL_DISPLAY_ASPECT_HEIGHT);
    let viewport_width = ORIGINAL_DISPLAY_ASPECT_WIDTH * scale;
    let viewport_height = ORIGINAL_DISPLAY_ASPECT_HEIGHT * scale;
    [
        motion[0] * LOGICAL_SCREEN_WIDTH / viewport_width,
        motion[1] * LOGICAL_SCREEN_HEIGHT / viewport_height,
    ]
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
    const EXPECTED_FRAME_BUDGET_NANOSECONDS: u64 = 39_946_965;
    const EXPECTED_MEASURED_UPDATE_RATE: f64 = 1_000.0 / 46.0;
    const EXPECTED_PRESENTATION_UPDATE_RATE: f64 = 1_000.0 / 68.0;
    const OPENING_PRESENTATION_FRAME_COUNT: u32 = 263;
    const ORACLE_OPENING_DURATION_MINIMUM: Duration = Duration::from_millis(17_500);
    const ORACLE_OPENING_DURATION_MAXIMUM: Duration = Duration::from_millis(18_500);

    #[test]
    fn frame_budget_comes_from_the_recovered_pit_programming() {
        assert_eq!(
            RECOVERED_FRAME_BUDGET.as_nanos(),
            EXPECTED_FRAME_BUDGET_NANOSECONDS as u128
        );
        let update_rate = 1.0 / RECOVERED_FRAME_BUDGET.as_secs_f64();
        assert!((update_rate - 25.03).abs() < 0.01);
    }

    #[test]
    fn simulation_uses_the_measured_dos_page_flip_cadence() {
        let update_rate = 1.0 / GAME_FRAME_DURATION.as_secs_f64();
        assert!((update_rate - EXPECTED_MEASURED_UPDATE_RATE).abs() < 0.01);
        assert!(GAME_FRAME_DURATION > RECOVERED_FRAME_BUDGET);
    }

    #[test]
    fn hnm_pacing_matches_the_dense_dos_opening_oracle() {
        let update_rate = 1.0 / PRESENTATION_FRAME_DURATION.as_secs_f64();
        assert!((update_rate - EXPECTED_PRESENTATION_UPDATE_RATE).abs() < 0.01);
        let opening_duration = PRESENTATION_FRAME_DURATION * OPENING_PRESENTATION_FRAME_COUNT;
        assert!(opening_duration >= ORACLE_OPENING_DURATION_MINIMUM);
        assert!(opening_duration <= ORACLE_OPENING_DURATION_MAXIMUM);
    }

    #[test]
    fn each_frame_gets_an_independent_budget_without_catch_up_bursts() {
        let start = Instant::now();
        let mut clock = GameFrameClock::default();
        clock.begin_frame(start);
        assert_eq!(
            clock.remaining(start, GAME_FRAME_DURATION),
            Some(GAME_FRAME_DURATION)
        );
        assert_eq!(
            clock.remaining(
                start + GAME_FRAME_DURATION + Duration::from_millis(1),
                GAME_FRAME_DURATION,
            ),
            Some(Duration::ZERO)
        );

        let next_start = start + GAME_FRAME_DURATION + Duration::from_millis(1);
        clock.begin_frame(next_start);
        assert_eq!(
            clock.remaining(next_start, PRESENTATION_FRAME_DURATION),
            Some(PRESENTATION_FRAME_DURATION)
        );
    }

    #[test]
    fn relative_mouse_motion_scales_through_the_letterboxed_viewport() {
        assert_eq!(
            map_motion_to_logical(
                WIDESCREEN_OUTPUT,
                [
                    WIDESCREEN_VIEWPORT_WIDTH,
                    WIDESCREEN_VIEWPORT_WIDTH * 3.0 / 4.0
                ],
            ),
            [LOGICAL_SCREEN_WIDTH, LOGICAL_SCREEN_HEIGHT]
        );
    }

    #[test]
    fn visual_refresh_rate_does_not_change_recovered_simulation_rate() {
        assert_eq!(
            VISUAL_FRAME_DURATION,
            Duration::from_nanos(NANOSECONDS_PER_SECOND / MODERN_VISUAL_REFRESH_HZ)
        );
        assert!(VISUAL_FRAME_DURATION < GAME_FRAME_DURATION);
        assert_eq!(
            GAME_FRAME_DURATION.as_millis(),
            MEASURED_GAME_FRAME_MILLISECONDS as u128
        );
    }

    #[test]
    fn subpixel_bridge_motion_is_retained_across_frames_in_both_directions() {
        let mut positive = 0.0;
        for _ in 0..3 {
            positive += 0.25;
            assert_eq!(take_whole_motion(&mut positive), 0);
        }
        positive += 0.25;
        assert_eq!(take_whole_motion(&mut positive), 1);
        assert_eq!(positive, 0.0);

        let mut negative = 0.0;
        for _ in 0..3 {
            negative -= 0.25;
            assert_eq!(take_whole_motion(&mut negative), 0);
        }
        negative -= 0.25;
        assert_eq!(take_whole_motion(&mut negative), -1);
        assert_eq!(negative, 0.0);
    }

    #[test]
    fn uncaptured_pointer_scrolls_continuously_at_bridge_edges() {
        assert_eq!(edge_scroll_delta(LOGICAL_SCREEN_WIDTH / 2.0, true), 0.0);
        assert_eq!(edge_scroll_delta(0.0, false), 0.0);
        assert_eq!(
            edge_scroll_delta(0.0, true),
            -BRIDGE_EDGE_SCROLL_MAXIMUM_DELTA
        );
        assert_eq!(
            edge_scroll_delta(LOGICAL_SCREEN_WIDTH - 1.0, true),
            edge_scroll_speed(BRIDGE_EDGE_SCROLL_ZONE_WIDTH - 1.0)
        );
        assert!(
            edge_scroll_delta(BRIDGE_EDGE_SCROLL_ZONE_WIDTH - 1.0, true)
                <= -BRIDGE_EDGE_SCROLL_MINIMUM_DELTA
        );
        assert!(
            edge_scroll_delta(LOGICAL_SCREEN_WIDTH - BRIDGE_EDGE_SCROLL_ZONE_WIDTH, true)
                >= BRIDGE_EDGE_SCROLL_MINIMUM_DELTA
        );
    }

    #[test]
    fn mouse_button_events_publish_stable_semantic_button_bits() {
        let mut buttons = PointerButtons::NONE;
        set_pointer_button(&mut buttons, MouseButton::Left, true);
        assert_eq!(buttons.bits(), PointerButton::Primary as u16);
        set_pointer_button(&mut buttons, MouseButton::Right, true);
        assert_eq!(
            buttons.bits(),
            PointerButton::Primary as u16 | PointerButton::Secondary as u16
        );
        set_pointer_button(&mut buttons, MouseButton::Left, false);
        assert_eq!(buttons.bits(), PointerButton::Secondary as u16);
        set_pointer_button(&mut buttons, MouseButton::Right, false);
        assert_eq!(buttons, PointerButtons::NONE);
    }

    #[test]
    fn desktop_button_state_cannot_activate_the_game_outside_its_window() {
        let pressed = PointerButtons::from_bits(PointerButton::Primary as u16);

        assert_eq!(
            pointer_buttons_inside_window(false, pressed),
            PointerButtons::NONE
        );
        assert_eq!(pointer_buttons_inside_window(true, pressed), pressed);
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
