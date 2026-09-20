//! Device-free execution of the production main lifecycle for authored sequences.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::Value;

use super::input::INITIAL_LOGICAL_POINTER;
use super::platform::GamePitClock;
use super::{
    GAME_FRAME_DURATION, ModernGameServices, OfflinePresentationInterval, OfflinePresentationSink,
    PRESENTATION_FRAME_DURATION, RuntimeAlienOverlayFrameInput, RuntimeAudioHost,
    RuntimeGameLifecycleHost, RuntimePlatformDriver,
};
use crate::native::bloodprg::{
    GameLifecycleState, GameSession, InputAction, PointerButtons, PointerSample, ScriptClock,
    initialize_game_runtime, run_game_runtime_frame, shutdown_game,
};

pub(super) trait OfflineGameSink: OfflinePresentationSink {
    fn write_native_state(&mut self, time_ns: u64, state: &Value) -> Result<()>;
}

#[derive(Serialize)]
pub(super) struct OfflineStartupReport {
    pub presented_frames: u64,
    pub duration_ns: u64,
    pub audio_samples: u64,
    pub main_loop_frames: u64,
    pub completed_sequence_lists: u64,
    pub bootstrap_duration_ns: u64,
    pub total_timer_ticks: u64,
    pub sequence_record: String,
    pub authored_videos: Vec<String>,
    pub authored_music: Option<String>,
    pub caption_cues: Vec<Value>,
}

fn script_clock() -> Result<ScriptClock> {
    Ok(ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    })
}

pub(super) fn capture_startup_cinematic(
    services: ModernGameServices<'_>,
    max_frames: u64,
    sink: &mut dyn OfflineGameSink,
) -> Result<(OfflineStartupReport, Vec<u8>)> {
    ensure!(max_frames > 0, "offline frame cap must be nonzero");
    let mut host = RuntimeGameLifecycleHost::with_platform(
        services,
        OfflineGamePlatform::new(max_frames, sink),
        None,
        39,
        script_clock,
        None,
    );
    let mut lifecycle = GameLifecycleState::default();
    let mut session = GameSession::default();
    let capture = (|| {
        ensure!(
            initialize_game_runtime(&mut lifecycle, &mut host, &mut session)?.is_none(),
            "native initialization exited before the cinematic"
        );
        let initial_completions = host.services().completed_presentation_sequence_lists()?;
        host.platform_mut().begin_capture();
        for main_frame in 1..=max_frames {
            ensure!(
                run_game_runtime_frame(&mut lifecycle, &mut host, &mut session)?.is_none(),
                "native lifecycle exited before completing its cinematic sequence list"
            );
            let completed =
                host.services().completed_presentation_sequence_lists()? - initial_completions;
            if completed != 0 {
                let driver = host.platform();
                let services = host.services();
                let selected = services
                    .presentation_screen_state()?
                    .selected_choice()
                    .index();
                let records = services.presentation_sequence_records()?;
                let record = records[selected]
                    .as_ref()
                    .context("completed sequence has no record")?;
                let assets = services.script_backend().assets();
                let source_cues = assets.sequence_subtitles();
                let display_cues = services
                    .runtime()
                    .data()
                    .english_sequence_captions
                    .display(source_cues);
                let report = OfflineStartupReport {
                    presented_frames: driver.captured_waits,
                    duration_ns: driver.elapsed_ns - driver.capture_origin_ns,
                    audio_samples: driver.sample_cursor - driver.capture_origin_sample,
                    main_loop_frames: main_frame,
                    completed_sequence_lists: completed,
                    bootstrap_duration_ns: driver.capture_origin_ns,
                    total_timer_ticks: driver.timer_ticks,
                    sequence_record: String::from_utf8_lossy(record).into_owned(),
                    authored_videos: assets.sequence_videos().iter()
                        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()).collect(),
                    authored_music: assets.music().map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
                    caption_cues: source_cues.iter().zip(display_cues).map(|(source, display)| serde_json::json!({
                        "authored_frame": source.first_visible_frame(), "source_bytes": source.text(),
                        "display_text": String::from_utf8_lossy(display.text()),
                    })).collect(),
                };
                ensure!(report.presented_frames > 0, "cinematic emitted no frames");
                return Ok((report, host.services().read_offline_rgba()?));
            }
        }
        bail!("native cinematic exceeded its main-loop cap before completion")
    })();
    host.platform_mut().capturing = false;
    // Release native resources without appending an unrelated credits sequence.
    let cleanup = shutdown_game(&mut lifecycle, &mut host, session, false);
    match (capture, cleanup) {
        (Ok(capture), Ok(())) => Ok(capture),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("cleanup also failed: {cleanup:#}")))
        }
    }
}

struct OfflineGamePlatform<'sink> {
    epoch: Instant,
    pit: GamePitClock,
    elapsed_ns: u64,
    sample_cursor: u64,
    timer_ticks: u64,
    total_waits: u64,
    captured_waits: u64,
    max_frames: u64,
    capturing: bool,
    capture_origin_ns: u64,
    capture_origin_sample: u64,
    pointer: [i16; 2],
    alien_pointer: Option<[f32; 2]>,
    sink: &'sink mut dyn OfflineGameSink,
}

impl<'sink> OfflineGamePlatform<'sink> {
    fn new(max_frames: u64, sink: &'sink mut dyn OfflineGameSink) -> Self {
        Self {
            epoch: Instant::now(),
            pit: GamePitClock::default(),
            elapsed_ns: 0,
            sample_cursor: 0,
            timer_ticks: 0,
            total_waits: 0,
            captured_waits: 0,
            max_frames,
            capturing: false,
            capture_origin_ns: 0,
            capture_origin_sample: 0,
            pointer: INITIAL_LOGICAL_POINTER,
            alien_pointer: None,
            sink,
        }
    }

    fn begin_capture(&mut self) {
        self.capturing = true;
        self.capture_origin_ns = self.elapsed_ns;
        self.capture_origin_sample = self.sample_cursor;
    }

    fn wait(&mut self, services: &mut ModernGameServices<'_>, duration: Duration) -> Result<()> {
        ensure!(
            self.total_waits < self.max_frames,
            "offline lifecycle exceeded its frame cap"
        );
        let duration_ns = u64::try_from(duration.as_nanos())?;
        let end = self
            .elapsed_ns
            .checked_add(duration_ns)
            .context("offline timeline overflow")?;
        let sample_end = (u128::from(end) * u128::from(RuntimeAudioHost::output_sample_rate_hz())
            / 1_000_000_000) as u64;
        let mut audio = vec![0.0; usize::try_from(sample_end - self.sample_cursor)?];
        services.render_offline_audio(&mut audio)?;
        if self.capturing {
            let rgba = services.read_offline_rgba()?;
            self.sink.write_interval(OfflinePresentationInterval {
                start_ns: self.elapsed_ns - self.capture_origin_ns,
                duration_ns,
                rgba: &rgba,
                audio: &audio,
            })?;
            self.captured_waits += 1;
        }
        self.elapsed_ns = end;
        self.sample_cursor = sample_end;
        self.total_waits += 1;
        Ok(())
    }
}

impl<'window> RuntimePlatformDriver<'window> for OfflineGamePlatform<'_> {
    fn dispatch_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        services.dispatch_lifecycle_input(state)
    }
    fn dispatch_game_events(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<Option<InputAction>> {
        self.dispatch_events(services, state)
    }
    fn record_scenario_frame_boundary(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        if self.capturing {
            self.sink.write_native_state(
                self.elapsed_ns - self.capture_origin_ns,
                &services.semantic_trace_snapshot(state)?,
            )?;
        }
        Ok(())
    }
    fn poll_alien_overlay_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<RuntimeAlienOverlayFrameInput> {
        let pointer = self
            .alien_pointer
            .context("alien pointer is not acquired")?;
        Ok(RuntimeAlienOverlayFrameInput::from_driver_pointer(
            pointer,
            PointerButtons::NONE,
            services.input_mut().drain_alien_key_events(false),
        ))
    }
    fn begin_alien_overlay_input(&mut self) -> Result<()> {
        ensure!(
            self.alien_pointer.is_none(),
            "alien pointer already acquired"
        );
        self.alien_pointer = Some([320.0, 512.0]);
        Ok(())
    }
    fn finish_alien_overlay_input(&mut self) -> bool {
        self.alien_pointer.take().is_some()
    }
    fn poll_pointer(&mut self, services: &mut ModernGameServices<'window>) -> PointerSample {
        services.publish_lifecycle_logical_pointer(self.pointer, PointerButtons::NONE)
    }
    fn logical_pointer(&self) -> [i16; 2] {
        self.pointer
    }
    fn synchronize_bridge_pointer(&mut self, position: [i16; 2]) {
        self.pointer = position;
    }
    fn start_game_timer(&mut self) {
        self.pit
            .start(self.epoch + Duration::from_nanos(self.elapsed_ns));
    }
    fn stop_game_timer(&mut self) {
        self.pit.stop();
    }
    fn take_game_timer_ticks(&mut self) -> u64 {
        let ticks = self
            .pit
            .take_elapsed_ticks(self.epoch + Duration::from_nanos(self.elapsed_ns));
        self.timer_ticks += ticks;
        ticks
    }
    fn take_bridge_horizontal_delta(&mut self) -> i32 {
        0
    }
    fn pace_frame(&mut self, services: &mut ModernGameServices<'window>) -> Result<()> {
        self.wait(services, GAME_FRAME_DURATION)
    }
    fn pace_presentation_frame(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<()> {
        self.wait(services, PRESENTATION_FRAME_DURATION)
    }
    fn wait_for_visual_refresh(
        &mut self,
        services: &mut ModernGameServices<'window>,
    ) -> Result<Option<f32>> {
        let duration = if services.presentation_stream_active() {
            PRESENTATION_FRAME_DURATION
        } else {
            GAME_FRAME_DURATION
        };
        self.wait(services, duration)?;
        Ok(None)
    }
}
