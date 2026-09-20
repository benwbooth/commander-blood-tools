//! Deterministic output driver for the existing blocking presentation runners.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::native::bloodprg::{
    GameLifecycleState, GameTimerState, InputAction, PresentationResourceId, PresentationRunExit,
    ScriptRuntime,
};

use super::platform::GamePitClock;
use super::presentation_run::{RuntimePresentationDriver, run_runtime_presentation_with_driver};
use super::{ModernGameServices, PRESENTATION_FRAME_DURATION, RuntimeAudioHost};

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// One interval during which a completed GPU frame remained visible.
pub struct OfflinePresentationInterval<'frame> {
    /// Start relative to entry into the blocking presentation runner.
    pub start_ns: u64,
    /// Duration of the production presentation wait.
    pub duration_ns: u64,
    /// Final composited RGBA pixels visible throughout this interval.
    pub rgba: &'frame [u8],
    /// Unmodified mono samples consumed by the shared production audio mixer.
    pub audio: &'frame [f32],
}

/// Synchronous writer; failures abort the export instead of dropping output.
pub trait OfflinePresentationSink {
    /// Store the visible frame and the audio consumed during exactly this wait.
    fn write_interval(&mut self, interval: OfflinePresentationInterval<'_>) -> Result<()>;
}

/// Accounting for a naturally completed native presentation loop.
#[derive(Debug, Serialize)]
pub struct OfflinePresentationReport {
    /// Number of presentation waits and final GPU flips.
    pub presented_frames: u64,
    /// Length of the captured half-open interval, excluding any invented tail.
    pub duration_ns: u64,
    /// Exact mixer sample count at the shared host rate.
    pub audio_samples: u64,
    /// PIT interrupts delivered using the same accumulator as live playback.
    pub timer_ticks: u64,
    /// Decoded source-frame count at the final GPU flip.
    pub final_decoded_frames: u64,
}

/// Export a complete opening (line 0) or credits (line 1) without mouse input.
///
/// Services must have initialized resources, viewport, and a presented entry
/// frame. Sound advances during the wait before each flip, as in the live host.
/// The final flip occurs exactly at the capture endpoint: callers should retain
/// that endpoint frame separately, or continue capturing the next native phase.
/// This driver never invents a duration for it or silently stops at a frame cap.
pub fn capture_offline_presentation(
    services: &mut ModernGameServices<'_>,
    line: PresentationResourceId,
    link_target: u16,
    max_frames: u64,
    sink: &mut dyn OfflinePresentationSink,
) -> Result<OfflinePresentationReport> {
    ensure!(
        matches!(line.get(), 0 | 1),
        "only native opening and credits runners are supported"
    );
    ensure!(max_frames > 0, "offline frame cap must be nonzero");
    ensure!(
        services.game_timer_tick() == 0,
        "offline presentation requires fresh timer state"
    );
    let visible = services.read_offline_rgba()?;
    let mut driver = OfflinePresentationDriver::new(visible, max_frames, sink);
    let mut lifecycle = GameLifecycleState::default();
    let mut timer = GameTimerState::default();
    timer.start();
    let outcome = run_runtime_presentation_with_driver(
        line,
        link_target,
        services,
        &mut driver,
        &mut lifecycle,
        &mut timer,
        &mut ScriptRuntime::default(),
    )?;
    ensure!(
        outcome.exit == PresentationRunExit::SceneCompleted && !outcome.shutdown_requested,
        "offline presentation did not finish naturally"
    );
    ensure!(driver.frames > 0, "offline presentation produced no frames");
    Ok(OfflinePresentationReport {
        presented_frames: driver.frames,
        duration_ns: driver.elapsed_ns,
        audio_samples: driver.audio_samples,
        timer_ticks: driver.timer_ticks,
        final_decoded_frames: driver.final_decoded_frames,
    })
}

struct OfflinePresentationDriver<'sink> {
    epoch: Instant,
    pit: GamePitClock,
    elapsed_ns: u64,
    audio_samples: u64,
    timer_ticks: u64,
    frames: u64,
    final_decoded_frames: u64,
    max_frames: u64,
    visible: Vec<u8>,
    sink: &'sink mut dyn OfflinePresentationSink,
}

impl<'sink> OfflinePresentationDriver<'sink> {
    fn new(
        visible: Vec<u8>,
        max_frames: u64,
        sink: &'sink mut dyn OfflinePresentationSink,
    ) -> Self {
        let epoch = Instant::now();
        let mut pit = GamePitClock::default();
        pit.start(epoch);
        Self {
            epoch,
            pit,
            elapsed_ns: 0,
            audio_samples: 0,
            timer_ticks: 0,
            frames: 0,
            final_decoded_frames: 0,
            max_frames,
            visible,
            sink,
        }
    }
}

fn sample_deadline(elapsed_ns: u64) -> u64 {
    (u128::from(elapsed_ns) * u128::from(RuntimeAudioHost::output_sample_rate_hz())
        / NANOS_PER_SECOND) as u64
}

impl<'window> RuntimePresentationDriver<'window> for OfflinePresentationDriver<'_> {
    fn take_game_timer_ticks(&mut self) -> u64 {
        let ticks = self
            .pit
            .take_elapsed_ticks(self.epoch + Duration::from_nanos(self.elapsed_ns));
        self.timer_ticks += ticks;
        ticks
    }

    fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        services.dispatch_lifecycle_input(state)
    }

    fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) {
        let pointer = services.input().pointer_sample();
        services.publish_lifecycle_logical_pointer(pointer.position, pointer.buttons);
    }

    fn present_frame(&mut self, services: &mut ModernGameServices<'window>) -> Result<()> {
        ensure!(
            self.frames < self.max_frames,
            "offline presentation exceeded its frame cap before completion"
        );
        services.submit_indexed_frame()?;
        let duration_ns = u64::try_from(PRESENTATION_FRAME_DURATION.as_nanos())?;
        let end = self
            .elapsed_ns
            .checked_add(duration_ns)
            .context("offline timeline overflow")?;
        let sample_end = sample_deadline(end);
        let mut audio = vec![0.0; usize::try_from(sample_end - self.audio_samples)?];
        services.render_offline_audio(&mut audio)?;
        self.sink.write_interval(OfflinePresentationInterval {
            start_ns: self.elapsed_ns,
            duration_ns,
            rgba: &self.visible,
            audio: &audio,
        })?;
        self.elapsed_ns = end;
        self.audio_samples = sample_end;
        services.present_artwork()?;
        self.visible = services.read_offline_rgba()?;
        self.final_decoded_frames = services.presentation_decoded_frame_count();
        self.frames += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_sample_deadlines_keep_fractional_time_without_drift() {
        assert_eq!(sample_deadline(0), 0);
        assert_eq!(sample_deadline(68_000_000), 3264);
        assert_eq!(sample_deadline(1_000_000_000), 48_000);
        let mut previous = 0;
        let mut samples = 0;
        for tick in 1..=1_000_000 {
            let deadline = sample_deadline(tick * 1001);
            samples += deadline - previous;
            previous = deadline;
        }
        assert_eq!(samples, sample_deadline(1_001_000_000));
    }

    #[test]
    fn offline_pit_uses_the_production_elapsed_clock_not_one_tick_per_frame() {
        let start = Instant::now();
        let mut pit = GamePitClock::default();
        pit.start(start);
        assert_eq!(pit.take_elapsed_ticks(start), 0);
        assert_eq!(
            pit.take_elapsed_ticks(start + PRESENTATION_FRAME_DURATION),
            13
        );
        assert_eq!(
            pit.take_elapsed_ticks(start + PRESENTATION_FRAME_DURATION * 2),
            14
        );
    }
}
