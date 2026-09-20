//! Host input and elapsed-time boundaries shared by live and offline lifecycles.

use anyhow::Result;

use super::{ModernGameServices, RuntimeAlienOverlayFrameInput, RuntimePlatformHost};
use crate::native::bloodprg::{GameLifecycleState, InputAction, PointerSample};

/// Platform operations used by the native lifecycle and nested presentation loops.
/// Implementations must preserve event ordering and account for each pacing wait.
pub trait RuntimePlatformDriver<'window> {
    /// Dispatch input at a blocking presentation boundary.
    fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>>;
    /// Dispatch input at an ordinary main-loop boundary.
    fn dispatch_game_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>>;
    /// Observe the completed native frame's semantic state before pacing.
    fn record_scenario_frame_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<()>;
    /// Sample one synchronous alien-overlay frame.
    fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput>;
    /// Acquire the alien overlay's virtual pointer.
    fn begin_alien_overlay_input(&mut self) -> Result<()>;
    /// Release that pointer, returning whether it was acquired.
    fn finish_alien_overlay_input(&mut self) -> bool;
    /// Publish the current pointer into the native input sampler.
    fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample;
    /// Return logical pointer coordinates for render interpolation.
    fn logical_pointer(&self) -> [i16; 2];
    /// Retain a pointer position selected by a native UI owner.
    fn synchronize_bridge_pointer(&mut self, position: [i16; 2]);
    /// Start the shared PIT accumulator.
    fn start_game_timer(&mut self);
    /// Stop delivery and discard a fractional PIT interval.
    fn stop_game_timer(&mut self);
    /// Consume every timer interrupt elapsed since the preceding boundary.
    fn take_game_timer_ticks(&mut self) -> u64;
    /// Consume accumulated horizontal pointer motion.
    fn take_bridge_horizontal_delta(&mut self) -> i32;
    /// Wait for an alien-overlay game frame after its completed GPU flip.
    fn pace_frame(&mut self, services: &mut ModernGameServices<'window>) -> Result<()>;
    /// Wait before a blocking presentation's next completed GPU flip.
    fn pace_presentation_frame(&mut self, services: &mut ModernGameServices<'window>)
    -> Result<()>;
    /// Wait to a render-only refresh, or return `None` at the native frame end.
    fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<Option<f32>>;
}

impl<'window> RuntimePlatformDriver<'window> for RuntimePlatformHost<'window> {
    fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        self.dispatch_events(services, state)
    }
    fn dispatch_game_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        self.dispatch_game_events(services, state)
    }
    fn record_scenario_frame_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        self.record_scenario_frame_boundary(services, state)
    }
    fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput> {
        self.poll_alien_overlay_frame(services)
    }
    fn begin_alien_overlay_input(&mut self) -> Result<()> {
        self.begin_alien_overlay_input()
    }
    fn finish_alien_overlay_input(&mut self) -> bool {
        self.finish_alien_overlay_input()
    }
    fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample {
        self.poll_pointer(services)
    }
    fn logical_pointer(&self) -> [i16; 2] {
        self.logical_pointer()
    }
    fn synchronize_bridge_pointer(&mut self, position: [i16; 2]) {
        self.synchronize_bridge_pointer(position);
    }
    fn start_game_timer(&mut self) {
        self.start_game_timer();
    }
    fn stop_game_timer(&mut self) {
        self.stop_game_timer();
    }
    fn take_game_timer_ticks(&mut self) -> u64 {
        self.take_game_timer_ticks()
    }
    fn take_bridge_horizontal_delta(&mut self) -> i32 {
        self.take_bridge_horizontal_delta()
    }
    fn pace_frame(&mut self, _services: &mut ModernGameServices<'window>) -> Result<()> {
        self.pace_frame()
    }
    fn pace_presentation_frame(
        &mut self,
        _services: &mut ModernGameServices<'window>,
    ) -> Result<()> {
        self.pace_presentation_frame()
    }
    fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<Option<f32>> {
        self.wait_for_visual_refresh(services)
    }
}
