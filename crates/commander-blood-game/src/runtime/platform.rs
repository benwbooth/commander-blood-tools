//! SDL event handling and recovered game-clock pacing for the modern runtime.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use sdl3::EventPump;
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::mouse::{MouseButton, MouseUtil};
use sdl3::video::Window;

use crate::native::bloodprg::{
    GameLifecycleState, InputAction, PointerButton, PointerButtons, PointerSample,
};

use super::input::INITIAL_LOGICAL_POINTER;
use super::scenario::{
    RuntimeScenarioCadence, RuntimeScenarioDriver, RuntimeScenarioFrameInput, RuntimeScenarioKey,
};
use super::{ModernGameServices, RuntimeAlienOverlayFrameInput};

/// Input frequency of the IBM PC programmable interval timer.
const PIT_INPUT_FREQUENCY_HZ: u64 = 1_193_182;
/// Divisor programmed by the recovered timer setup routine.
const GAME_TIMER_DIVISOR: u64 = 5_958;
/// Minimum timer interrupts in the `bloodprg_main` frame-delay wait.
const GAME_FRAME_TIMER_TICKS: u64 = 8;
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
const PIT_TICK_SCALED_UNITS: u128 = GAME_TIMER_DIVISOR as u128 * NANOSECONDS_PER_SECOND as u128;
const MEASURED_GAME_FRAME_MILLISECONDS: u64 = 46;
const MEASURED_PRESENTATION_FRAME_MILLISECONDS: u64 = 68;
const DEFAULT_VISUAL_REFRESH_HZ: f64 = 60.0;
const MINIMUM_SURFACE_DIMENSION: u32 = 1;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: f32 = 4.0;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: f32 = 3.0;
const LOGICAL_SCREEN_WIDTH: f32 = 320.0;
const LOGICAL_SCREEN_HEIGHT: f32 = 200.0;
const ALIEN_DRIVER_WIDTH: f32 = 640.0;
const ALIEN_DRIVER_HEIGHT: f32 = 1_024.0;
const ALIEN_DRIVER_CENTER: [f32; 2] = [ALIEN_DRIVER_WIDTH / 2.0, ALIEN_DRIVER_HEIGHT / 2.0];
const MINIMUM_INTERPOLATION_FRACTION: f32 = 0.0;
const MAXIMUM_INTERPOLATION_FRACTION: f32 = 1.0;

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
    Duration::from_nanos(NANOSECONDS_PER_SECOND / DEFAULT_VISUAL_REFRESH_HZ as u64);

/// SDL-facing state owned by the production game lifecycle host.
pub struct RuntimePlatformHost<'window> {
    window: &'window Window,
    mouse: MouseUtil,
    events: EventPump,
    frame_clock: GameFrameClock,
    pit_clock: GamePitClock,
    bridge_horizontal_delta: f32,
    pointer_buttons: PointerButtons,
    logical_pointer: [f32; 2],
    pointer_inside_window: bool,
    pointer_position_locked: bool,
    mouse_capture_requested: bool,
    window_focused: bool,
    alien_pointer: Option<[f32; 2]>,
    scenario: Option<RuntimeScenarioDriver>,
}

impl<'window> RuntimePlatformHost<'window> {
    /// Bind SDL input with relative capture for the game's virtual hand.
    pub fn new(window: &'window Window, mouse: MouseUtil, events: EventPump) -> Self {
        Self::with_mouse_capture(window, mouse, events, true)
    }

    fn with_mouse_capture(
        window: &'window Window,
        mouse: MouseUtil,
        events: EventPump,
        mouse_capture_requested: bool,
    ) -> Self {
        mouse.set_relative_mouse_mode(window, mouse_capture_requested);
        Self {
            window,
            mouse,
            events,
            frame_clock: GameFrameClock::default(),
            pit_clock: GamePitClock::default(),
            bridge_horizontal_delta: 0.0,
            pointer_buttons: PointerButtons::NONE,
            logical_pointer: INITIAL_LOGICAL_POINTER.map(f32::from),
            pointer_inside_window: mouse_capture_requested,
            pointer_position_locked: false,
            mouse_capture_requested,
            window_focused: true,
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
        let mut platform = Self::with_mouse_capture(window, mouse, events, false);
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
        self.synchronize_pointer_lock(
            lifecycle_pointer_locked(state),
            services.input().pointer_sample().position,
        );
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
                self.apply_game_scenario_input(services, input)?;
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
        let mut platform_shutdown = self.pump_events(services);
        if let Some(scenario) = self.scenario.as_mut() {
            let input = scenario.advance(None, RuntimeScenarioCadence::BlockingPresentation)?;
            platform_shutdown |= input.request_shutdown;
            self.apply_alien_scenario_input(services, input)?;
        }
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

    fn apply_game_scenario_input(
        &mut self,
        services: &mut ModernGameServices<'window>,
        input: RuntimeScenarioFrameInput,
    ) -> Result<()> {
        if !self.pointer_position_locked
            && let Some(position) = input.pointer_position
        {
            self.bridge_horizontal_delta += f32::from(position[0]) - self.logical_pointer[0];
            self.logical_pointer = position.map(f32::from);
            self.pointer_inside_window = true;
        }
        if let Some(relative_motion) = input.relative_pointer_motion {
            let (width, height) = self.window.size();
            apply_runtime_pointer_motion(
                &mut self.alien_pointer,
                &mut self.logical_pointer,
                &mut self.bridge_horizontal_delta,
                &mut self.pointer_inside_window,
                self.pointer_position_locked,
                [width as f32, height as f32],
                relative_motion.map(|component| component as f32),
            );
        }
        self.apply_scenario_buttons_and_key(services, input)
    }

    fn apply_alien_scenario_input(
        &mut self,
        services: &mut ModernGameServices<'window>,
        input: RuntimeScenarioFrameInput,
    ) -> Result<()> {
        if let Some(position) = input.pointer_position {
            self.alien_pointer = Some(map_logical_to_alien_driver(position));
        }
        self.apply_scenario_buttons_and_key(services, input)
    }

    fn apply_scenario_buttons_and_key(
        &mut self,
        services: &mut ModernGameServices<'window>,
        input: RuntimeScenarioFrameInput,
    ) -> Result<()> {
        if let Some(procedure_offset) = input.contact_procedure_offset {
            services.prepare_contact_for_scenario(procedure_offset)?;
        }
        if let Some(target) = input.teleport_target.as_deref() {
            services.teleport_arche_to_navigation_target(target)?;
        }
        if input.trigger_alien_overlay {
            services.trigger_alien_overlay_for_scenario()?;
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
        Ok(())
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
        let released = self.alien_pointer.take().is_some();
        released
    }

    fn pump_events(&mut self, services: &mut ModernGameServices<'window>) -> bool {
        let window_id = self.window.id();
        let mut platform_shutdown = false;
        while let Some(event) = self.events.poll_event() {
            if self.consume_mouse_motion_event(&event) {
                continue;
            }
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
                    self.window_focused = false;
                    self.pointer_inside_window = false;
                    self.pointer_buttons = PointerButtons::NONE;
                    self.mouse.set_relative_mouse_mode(self.window, false);
                }
                Event::Window {
                    window_id: event_window_id,
                    win_event: WindowEvent::FocusGained,
                    ..
                } if event_window_id == window_id => {
                    self.window_focused = true;
                    self.mouse.set_relative_mouse_mode(
                        self.window,
                        self.scenario.is_none() && self.mouse_capture_requested,
                    );
                    self.pointer_inside_window =
                        self.scenario.is_some() || self.mouse_capture_requested;
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
                Event::MouseButtonDown {
                    window_id: event_window_id,
                    mouse_btn,
                    ..
                } if event_window_id == window_id => {
                    if should_recapture_mouse(self.scenario.is_some(), self.mouse_capture_requested)
                    {
                        self.mouse_capture_requested = true;
                        self.pointer_inside_window = true;
                        self.pointer_buttons = PointerButtons::NONE;
                        self.mouse
                            .set_relative_mouse_mode(self.window, self.window_focused);
                        continue;
                    }
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
                    repeat,
                    ..
                } if event_window_id == window_id => {
                    if let Some(capture_requested) = toggled_mouse_capture_request(
                        self.scenario.is_some(),
                        self.mouse_capture_requested,
                        keycode,
                        repeat,
                    ) {
                        if capture_requested != self.mouse_capture_requested {
                            self.mouse_capture_requested = capture_requested;
                            self.pointer_inside_window = self.mouse_capture_requested;
                            self.pointer_buttons = PointerButtons::NONE;
                            self.mouse.set_relative_mouse_mode(
                                self.window,
                                self.window_focused && self.mouse_capture_requested,
                            );
                        }
                        continue;
                    }
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

    /// Consume a production SDL relative-motion event through the recovered input gate.
    ///
    /// The DOS main loop continues polling during actor presentations (UI bit
    /// `0x0004`), but retains the previous position while navigation owns UI bit
    /// `0x0008`. Relative capture therefore stays enabled across either state;
    /// only logical movement is suppressed while `pointer_position_locked` is true.
    fn consume_mouse_motion_event(&mut self, event: &Event) -> bool {
        let Event::MouseMotion {
            window_id,
            xrel,
            yrel,
            ..
        } = event
        else {
            return false;
        };
        if *window_id != self.window.id()
            || self.scenario.is_some()
            || !self.mouse_capture_requested
            || !self.window_focused
        {
            return true;
        }

        let (width, height) = self.window.size();
        apply_runtime_pointer_motion(
            &mut self.alien_pointer,
            &mut self.logical_pointer,
            &mut self.bridge_horizontal_delta,
            &mut self.pointer_inside_window,
            self.pointer_position_locked,
            [width as f32, height as f32],
            [*xrel, *yrel],
        );
        true
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

    /// Retain the logical pointer selected by the current bridge input owner.
    pub fn synchronize_bridge_pointer(&mut self, position: [i16; 2]) {
        self.logical_pointer = position.map(f32::from);
    }

    /// Start the monotonic replacement for the recovered channel-zero PIT ISR.
    pub fn start_game_timer(&mut self) {
        self.pit_clock.start(Instant::now());
    }

    /// Stop timer delivery and discard any fractional interval still pending.
    pub fn stop_game_timer(&mut self) {
        self.pit_clock.stop();
    }

    /// Return every PIT interrupt elapsed since the previous runtime boundary.
    ///
    /// Scripted oracle campaigns retain their calibrated eight-tick semantic
    /// frame. Production uses elapsed monotonic time because the DOS ISR kept
    /// running after the main loop's eight-interrupt minimum had expired.
    pub fn take_game_timer_ticks(&mut self) -> u64 {
        let now = Instant::now();
        if self.scenario.is_some() {
            self.pit_clock.take_fixed_ticks(now, GAME_FRAME_TIMER_TICKS)
        } else {
            self.pit_clock.take_elapsed_ticks(now)
        }
    }

    /// Consume horizontal mouse motion retained since the previous bridge frame.
    pub fn take_bridge_horizontal_delta(&mut self) -> i32 {
        take_bridge_motion(
            &mut self.bridge_horizontal_delta,
            self.pointer_position_locked,
        )
    }

    fn synchronize_pointer_lock(&mut self, locked: bool, published_position: [i16; 2]) {
        self.pointer_position_locked = locked;
        if locked {
            retain_locked_pointer_position(
                &mut self.logical_pointer,
                &mut self.bridge_horizontal_delta,
                published_position,
            );
        }
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
    /// Returns the elapsed native-frame fraction for a visual refresh, or
    /// `None` once the recovered frame deadline has been reached. SDL events are
    /// retained immediately, but lifecycle input is still dispatched only by
    /// the next C simulation frame.
    pub fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<Option<f32>> {
        if self.scenario.is_some() {
            self.frame_clock.finish_frame();
            return Ok(None);
        }
        let frame_duration = if services.presentation_stream_active() {
            PRESENTATION_FRAME_DURATION
        } else {
            GAME_FRAME_DURATION
        };
        let now = Instant::now();
        let remaining = self
            .frame_clock
            .remaining(now, frame_duration)
            .context("visual refresh started without a game frame budget")?;
        let visual_refresh_duration = visual_refresh_duration(self.window);
        let until_refresh = self
            .frame_clock
            .remaining_to_visual_refresh(now, visual_refresh_duration)
            .context("visual refresh scheduled without a game frame budget")?;
        if until_refresh >= remaining {
            if !remaining.is_zero() {
                thread::sleep(remaining);
            }
            self.frame_clock.finish_frame();
            return Ok(None);
        }
        if !until_refresh.is_zero() {
            thread::sleep(until_refresh);
        }
        let refresh_now = Instant::now();
        if self
            .frame_clock
            .remaining(refresh_now, frame_duration)
            .is_some_and(|remaining| remaining.is_zero())
        {
            self.frame_clock.finish_frame();
            return Ok(None);
        }
        self.pump_events(services);
        let fraction = self
            .frame_clock
            .elapsed_fraction(refresh_now, frame_duration)
            .context("visual interpolation started without a game frame budget")?;
        self.frame_clock
            .schedule_next_visual_refresh(refresh_now, visual_refresh_duration);
        Ok(Some(fraction))
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
    next_visual_refresh_at: Option<Instant>,
}

#[derive(Default)]
struct GamePitClock {
    sampled_at: Option<Instant>,
    scaled_remainder: u128,
}

impl GamePitClock {
    fn start(&mut self, now: Instant) {
        self.sampled_at = Some(now);
        self.scaled_remainder = u128::MIN;
    }

    fn stop(&mut self) {
        self.sampled_at = None;
        self.scaled_remainder = u128::MIN;
    }

    fn take_elapsed_ticks(&mut self, now: Instant) -> u64 {
        let Some(sampled_at) = self.sampled_at.replace(now) else {
            return u64::MIN;
        };
        let elapsed_scaled = now
            .saturating_duration_since(sampled_at)
            .as_nanos()
            .saturating_mul(PIT_INPUT_FREQUENCY_HZ as u128);
        let accumulated = self.scaled_remainder.saturating_add(elapsed_scaled);
        self.scaled_remainder = accumulated % PIT_TICK_SCALED_UNITS;
        u64::try_from(accumulated / PIT_TICK_SCALED_UNITS).unwrap_or(u64::MAX)
    }

    fn take_fixed_ticks(&mut self, now: Instant, ticks: u64) -> u64 {
        if self.sampled_at.replace(now).is_none() {
            return u64::MIN;
        }
        self.scaled_remainder = u128::MIN;
        ticks
    }
}

impl GameFrameClock {
    fn begin_frame(&mut self, now: Instant) {
        self.started_at = Some(now);
        self.next_visual_refresh_at = None;
    }

    fn remaining(&self, now: Instant, duration: Duration) -> Option<Duration> {
        self.started_at
            .map(|started_at| (started_at + duration).saturating_duration_since(now))
    }

    fn elapsed_fraction(&self, now: Instant, duration: Duration) -> Option<f32> {
        self.started_at.map(|started_at| {
            let elapsed = now.saturating_duration_since(started_at).as_secs_f32();
            (elapsed / duration.as_secs_f32()).clamp(
                MINIMUM_INTERPOLATION_FRACTION,
                MAXIMUM_INTERPOLATION_FRACTION,
            )
        })
    }

    fn remaining_to_visual_refresh(
        &mut self,
        now: Instant,
        visual_refresh_duration: Duration,
    ) -> Option<Duration> {
        let started_at = self.started_at?;
        let deadline = self
            .next_visual_refresh_at
            .get_or_insert(started_at + visual_refresh_duration);
        Some(deadline.saturating_duration_since(now))
    }

    fn schedule_next_visual_refresh(&mut self, now: Instant, visual_refresh_duration: Duration) {
        if self.started_at.is_some() {
            self.next_visual_refresh_at = Some(now + visual_refresh_duration);
        }
    }

    fn finish_frame(&mut self) {
        self.started_at = None;
        self.next_visual_refresh_at = None;
    }
}

fn visual_refresh_duration(window: &Window) -> Duration {
    let refresh_rate = window
        .get_display()
        .ok()
        .and_then(|display| display.get_mode().ok())
        .map(|mode| mode.refresh_rate)
        .unwrap_or(DEFAULT_VISUAL_REFRESH_HZ as f32);
    visual_refresh_duration_for_rate(refresh_rate)
}

fn visual_refresh_duration_for_rate(refresh_rate: f32) -> Duration {
    if !refresh_rate.is_finite() || refresh_rate <= 0.0 {
        return VISUAL_FRAME_DURATION;
    }
    let duration = Duration::from_secs_f64(1.0 / f64::from(refresh_rate));
    if duration.is_zero() {
        Duration::from_nanos(1)
    } else {
        duration
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

fn toggled_mouse_capture_request(
    scripted: bool,
    capture_requested: bool,
    keycode: Keycode,
    repeat: bool,
) -> Option<bool> {
    if scripted || keycode != Keycode::F10 {
        return None;
    }
    Some(if repeat {
        capture_requested
    } else {
        !capture_requested
    })
}

const fn should_recapture_mouse(scripted: bool, capture_requested: bool) -> bool {
    !scripted && !capture_requested
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

fn take_bridge_motion(accumulated: &mut f32, pointer_position_locked: bool) -> i32 {
    if pointer_position_locked {
        *accumulated = 0.0;
        return 0;
    }
    take_whole_motion(accumulated)
}

fn lifecycle_pointer_locked(state: &GameLifecycleState) -> bool {
    state.pause_hud_active || state.pointer_position_locked || state.navigation_ui_busy()
}

fn retain_locked_pointer_position(
    logical_pointer: &mut [f32; 2],
    horizontal_delta: &mut f32,
    published_position: [i16; 2],
) {
    *logical_pointer = published_position.map(f32::from);
    *horizontal_delta = 0.0;
}

fn apply_bridge_pointer_motion(
    logical_pointer: &mut [f32; 2],
    horizontal_delta: &mut f32,
    pointer_position_locked: bool,
    output_size: [f32; 2],
    relative_motion: [f32; 2],
) {
    if pointer_position_locked {
        return;
    }
    let delta = map_motion_to_logical(output_size, relative_motion);
    *horizontal_delta += delta[0];
    logical_pointer[0] = (logical_pointer[0] + delta[0]).clamp(0.0, LOGICAL_SCREEN_WIDTH - 1.0);
    logical_pointer[1] = (logical_pointer[1] + delta[1]).clamp(0.0, LOGICAL_SCREEN_HEIGHT - 1.0);
}

fn apply_runtime_pointer_motion(
    alien_pointer: &mut Option<[f32; 2]>,
    logical_pointer: &mut [f32; 2],
    bridge_horizontal_delta: &mut f32,
    pointer_inside_window: &mut bool,
    pointer_position_locked: bool,
    output_size: [f32; 2],
    relative_motion: [f32; 2],
) {
    if let Some(pointer) = alien_pointer {
        let delta = map_motion_to_alien_driver(output_size, relative_motion);
        pointer[0] = (pointer[0] + delta[0]).clamp(0.0, ALIEN_DRIVER_WIDTH);
        pointer[1] = (pointer[1] + delta[1]).clamp(0.0, ALIEN_DRIVER_HEIGHT);
    } else {
        *pointer_inside_window = true;
        apply_bridge_pointer_motion(
            logical_pointer,
            bridge_horizontal_delta,
            pointer_position_locked,
            output_size,
            relative_motion,
        );
    }
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

fn map_logical_to_alien_driver(position: [i16; 2]) -> [f32; 2] {
    [
        f32::from(position[0]) * ALIEN_DRIVER_WIDTH / LOGICAL_SCREEN_WIDTH,
        f32::from(position[1]) * ALIEN_DRIVER_HEIGHT / LOGICAL_SCREEN_HEIGHT,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl3::mouse::MouseState;

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
    fn pit_clock_retains_fractional_ticks_across_measured_frames() {
        let start = Instant::now();
        let mut clock = GamePitClock::default();
        clock.start(start);

        assert_eq!(clock.take_elapsed_ticks(start + GAME_FRAME_DURATION), 9);
        assert_eq!(clock.take_elapsed_ticks(start + GAME_FRAME_DURATION * 2), 9);
        assert_eq!(
            clock.take_elapsed_ticks(start + Duration::from_secs(1)),
            182
        );

        let mut presentation_clock = GamePitClock::default();
        presentation_clock.start(start);
        assert_eq!(
            presentation_clock.take_elapsed_ticks(start + PRESENTATION_FRAME_DURATION),
            13
        );
        assert_eq!(
            presentation_clock.take_elapsed_ticks(start + PRESENTATION_FRAME_DURATION * 2),
            14
        );
    }

    #[test]
    fn scripted_pit_clock_retains_the_calibrated_frame_tick_count() {
        let start = Instant::now();
        let mut clock = GamePitClock::default();
        assert_eq!(clock.take_fixed_ticks(start, GAME_FRAME_TIMER_TICKS), 0);

        clock.start(start);
        assert_eq!(
            clock.take_fixed_ticks(start + PRESENTATION_FRAME_DURATION, GAME_FRAME_TIMER_TICKS),
            GAME_FRAME_TIMER_TICKS
        );
        clock.stop();
        assert_eq!(
            clock.take_fixed_ticks(start + Duration::from_secs(1), GAME_FRAME_TIMER_TICKS),
            0
        );
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
    fn visual_refresh_deadlines_do_not_advance_the_simulation_clock() {
        let start = Instant::now();
        let sample = start + Duration::from_millis(8);
        let mut clock = GameFrameClock::default();
        clock.begin_frame(start);
        let simulation_remaining = clock.remaining(sample, GAME_FRAME_DURATION);

        assert_eq!(
            clock.remaining_to_visual_refresh(sample, Duration::from_millis(5)),
            Some(Duration::ZERO)
        );
        clock.schedule_next_visual_refresh(sample, Duration::from_millis(5));

        assert_eq!(
            clock.remaining_to_visual_refresh(
                sample + Duration::from_millis(1),
                Duration::from_millis(5)
            ),
            Some(Duration::from_millis(4))
        );
        assert_eq!(
            clock.remaining(sample, GAME_FRAME_DURATION),
            simulation_remaining
        );
        assert_eq!(
            clock.elapsed_fraction(sample, GAME_FRAME_DURATION),
            Some(8.0 / 46.0)
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
    fn movie_visual_refreshes_preserve_the_full_presentation_budget() {
        let start = Instant::now();
        let refresh = Duration::from_secs_f64(1.0 / 240.0);
        let mut clock = GameFrameClock::default();
        clock.begin_frame(start);
        for ordinal in 1..=16 {
            let now = start + refresh * ordinal;
            assert_eq!(
                clock.remaining_to_visual_refresh(now, refresh),
                Some(Duration::ZERO)
            );
            clock.schedule_next_visual_refresh(now, refresh);
            assert_eq!(
                clock.remaining(now, PRESENTATION_FRAME_DURATION),
                Some(PRESENTATION_FRAME_DURATION - refresh * ordinal)
            );
            let fraction = clock
                .elapsed_fraction(now, PRESENTATION_FRAME_DURATION)
                .unwrap();
            assert!(fraction > 0.0 && fraction < 1.0);
        }
        assert_eq!(
            clock.remaining(
                start + PRESENTATION_FRAME_DURATION,
                PRESENTATION_FRAME_DURATION
            ),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn visual_refresh_rate_does_not_change_recovered_simulation_rate() {
        assert_eq!(
            VISUAL_FRAME_DURATION,
            Duration::from_nanos(NANOSECONDS_PER_SECOND / DEFAULT_VISUAL_REFRESH_HZ as u64)
        );
        assert!(VISUAL_FRAME_DURATION < GAME_FRAME_DURATION);
        assert_eq!(
            GAME_FRAME_DURATION.as_millis(),
            MEASURED_GAME_FRAME_MILLISECONDS as u128
        );
    }

    #[test]
    fn monitor_refresh_rate_can_raise_visual_refresh_frequency() {
        assert!(visual_refresh_duration_for_rate(144.0) < VISUAL_FRAME_DURATION);
        assert_eq!(visual_refresh_duration_for_rate(0.0), VISUAL_FRAME_DURATION);
        assert_eq!(
            visual_refresh_duration_for_rate(f32::NAN),
            VISUAL_FRAME_DURATION
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
    fn pointer_lock_discards_motion_and_retains_the_last_published_position() {
        let published_position = [160, 100];
        let mut logical_pointer = [240.0, 80.0];
        let mut horizontal_delta = 9.75;
        retain_locked_pointer_position(
            &mut logical_pointer,
            &mut horizontal_delta,
            published_position,
        );

        apply_bridge_pointer_motion(
            &mut logical_pointer,
            &mut horizontal_delta,
            true,
            WIDESCREEN_OUTPUT,
            [720.0, 0.0],
        );

        assert_eq!(logical_pointer, published_position.map(f32::from));
        assert_eq!(horizontal_delta, 0.0);

        horizontal_delta = 7.5;
        assert_eq!(take_bridge_motion(&mut horizontal_delta, true), 0);
        assert_eq!(horizontal_delta, 0.0);
        assert_eq!(take_bridge_motion(&mut horizontal_delta, false), 0);
    }

    #[test]
    fn uncaptured_host_motion_advances_the_virtual_game_cursor() {
        let mut pointer = [160.0, 100.0];
        let mut horizontal_delta = 0.0;

        apply_bridge_pointer_motion(
            &mut pointer,
            &mut horizontal_delta,
            false,
            WIDESCREEN_OUTPUT,
            [WIDESCREEN_VIEWPORT_WIDTH / 4.0, 0.0],
        );

        assert_eq!(pointer, [240.0, 100.0]);
        assert_eq!(horizontal_delta, 80.0);
    }

    #[test]
    fn bridge_motion_contains_only_host_delta_before_recovered_steering() {
        let mut horizontal_delta = 12.75;
        assert_eq!(take_bridge_motion(&mut horizontal_delta, false), 12);
        assert_eq!(horizontal_delta, 0.75);

        assert_eq!(take_bridge_motion(&mut horizontal_delta, false), 0);
        assert_eq!(horizontal_delta, 0.75);
    }

    #[test]
    fn native_pause_and_navigation_bit_lock_platform_pointer_motion() {
        let mut state = GameLifecycleState::default();
        assert!(!lifecycle_pointer_locked(&state));

        state.pause_hud_active = true;
        assert!(lifecycle_pointer_locked(&state));
        state.pause_hud_active = false;

        state.set_navigation_ui_busy(true);
        assert!(lifecycle_pointer_locked(&state));
        state.set_navigation_ui_busy(false);

        state.pointer_position_locked = true;
        assert!(lifecycle_pointer_locked(&state));
    }

    #[test]
    fn queued_sdl_motion_survives_bob_presentation_and_true_lock_release() {
        const OUTPUT_WIDTH: u32 = 640;
        const OUTPUT_HEIGHT: u32 = 400;
        const MOTION_X: f32 = 64.0;

        let sdl = sdl3::init().expect("SDL must initialize for the production input test");
        let video = sdl
            .video()
            .expect("SDL video must initialize for the production input test");
        let window = video
            .window(
                "Commander Blood production mouse test",
                OUTPUT_WIDTH,
                OUTPUT_HEIGHT,
            )
            .position_centered()
            .build()
            .expect("the production input test needs a real SDL window");
        let event_subsystem = sdl
            .event()
            .expect("the production input test needs the SDL event queue");
        let events = sdl
            .event_pump()
            .expect("the production input test needs an SDL event pump");
        let mut platform = RuntimePlatformHost::new(&window, sdl.mouse(), events);
        drain_sdl_events(&mut platform);

        assert!(platform.mouse_capture_requested);
        assert!(platform.window_focused);
        assert!(platform.mouse.relative_mouse_mode(&window));

        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_modal_ui_busy(true);
        assert!(lifecycle.modal_ui_busy());
        assert!(
            !lifecycle_pointer_locked(&lifecycle),
            "recovered actor-presentation UI bit 0x0004 does not freeze DOS mouse polling"
        );
        platform.synchronize_pointer_lock(
            lifecycle_pointer_locked(&lifecycle),
            INITIAL_LOGICAL_POINTER,
        );
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), MOTION_X);
        let during_bob = platform.logical_pointer();
        assert!(during_bob[0] > INITIAL_LOGICAL_POINTER[0]);
        assert!(platform.take_bridge_horizontal_delta() > 0);
        assert!(platform.mouse.relative_mouse_mode(&window));

        lifecycle.set_modal_ui_busy(false);
        platform.synchronize_pointer_lock(lifecycle_pointer_locked(&lifecycle), during_bob);
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), -MOTION_X);
        assert!(platform.logical_pointer()[0] < during_bob[0]);
        assert!(platform.take_bridge_horizontal_delta() < 0);

        let before_navigation_lock = platform.logical_pointer();
        lifecycle.set_navigation_ui_busy(true);
        platform
            .synchronize_pointer_lock(lifecycle_pointer_locked(&lifecycle), before_navigation_lock);
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), MOTION_X);
        assert_eq!(platform.logical_pointer(), before_navigation_lock);
        assert_eq!(platform.take_bridge_horizontal_delta(), 0);
        assert!(
            platform.mouse.relative_mouse_mode(&window),
            "logical lock must not release production SDL capture"
        );

        lifecycle.set_navigation_ui_busy(false);
        platform
            .synchronize_pointer_lock(lifecycle_pointer_locked(&lifecycle), before_navigation_lock);
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), MOTION_X);
        assert!(platform.logical_pointer()[0] > before_navigation_lock[0]);
        assert!(platform.take_bridge_horizontal_delta() > 0);

        let admitted_position = platform.logical_pointer();
        dispatch_queued_mouse_motion(
            &mut platform,
            &event_subsystem,
            window.id().wrapping_add(1),
            MOTION_X,
        );
        assert_eq!(platform.logical_pointer(), admitted_position);
        assert_eq!(platform.take_bridge_horizontal_delta(), 0);

        platform.mouse_capture_requested = false;
        platform.mouse.set_relative_mouse_mode(&window, false);
        assert!(!platform.mouse.relative_mouse_mode(&window));
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), MOTION_X);
        assert_eq!(platform.logical_pointer(), admitted_position);
        assert_eq!(platform.take_bridge_horizontal_delta(), 0);

        platform.mouse_capture_requested = true;
        platform.mouse.set_relative_mouse_mode(&window, true);
        assert!(platform.mouse.relative_mouse_mode(&window));
        platform.window_focused = false;
        platform.mouse.set_relative_mouse_mode(&window, false);
        assert!(!platform.mouse.relative_mouse_mode(&window));
        dispatch_queued_mouse_motion(&mut platform, &event_subsystem, window.id(), MOTION_X);
        assert_eq!(platform.logical_pointer(), admitted_position);
        assert_eq!(platform.take_bridge_horizontal_delta(), 0);
    }

    fn dispatch_queued_mouse_motion(
        platform: &mut RuntimePlatformHost<'_>,
        event_subsystem: &sdl3::EventSubsystem,
        window_id: u32,
        xrel: f32,
    ) {
        drain_sdl_events(platform);
        event_subsystem
            .push_event(Event::MouseMotion {
                timestamp: 0,
                window_id,
                which: 0,
                mousestate: MouseState::from_sdl_state(0),
                x: 0.0,
                y: 0.0,
                xrel,
                yrel: 0.0,
            })
            .expect("the synthetic SDL motion must enter SDL's real event queue");

        let mut observed = false;
        while let Some(event) = platform.events.poll_event() {
            if let Event::MouseMotion {
                window_id: observed_window_id,
                xrel: observed_xrel,
                ..
            } = &event
                && *observed_window_id == window_id
                && *observed_xrel == xrel
            {
                observed = true;
            }
            platform.consume_mouse_motion_event(&event);
        }
        assert!(observed, "SDL did not return the queued MouseMotion event");
    }

    fn drain_sdl_events(platform: &mut RuntimePlatformHost<'_>) {
        while platform.events.poll_event().is_some() {}
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
    fn f10_toggles_host_capture_without_repeating_or_entering_scripted_input() {
        assert_eq!(
            toggled_mouse_capture_request(false, true, Keycode::F10, false),
            Some(false)
        );
        assert_eq!(
            toggled_mouse_capture_request(false, false, Keycode::F10, false),
            Some(true)
        );
        assert_eq!(
            toggled_mouse_capture_request(false, true, Keycode::F10, true),
            Some(true)
        );
        assert_eq!(
            toggled_mouse_capture_request(true, false, Keycode::F10, false),
            None
        );
        assert_eq!(
            toggled_mouse_capture_request(false, true, Keycode::Escape, false),
            None
        );
    }

    #[test]
    fn first_click_after_release_recaptures_instead_of_reaching_the_game() {
        assert!(should_recapture_mouse(false, false));
        assert!(!should_recapture_mouse(false, true));
        assert!(!should_recapture_mouse(true, false));
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

    #[test]
    fn scripted_absolute_pointer_maps_across_the_full_alien_driver_domain() {
        assert_eq!(map_logical_to_alien_driver([0, 0]), [0.0, 0.0]);
        assert_eq!(
            map_logical_to_alien_driver([
                LOGICAL_SCREEN_WIDTH as i16,
                LOGICAL_SCREEN_HEIGHT as i16,
            ]),
            [ALIEN_DRIVER_WIDTH, ALIEN_DRIVER_HEIGHT]
        );
    }
}
