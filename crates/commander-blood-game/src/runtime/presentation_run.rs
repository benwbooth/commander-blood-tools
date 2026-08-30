//! SDL, audio, and wgpu host for the recovered blocking presentation runners.

use anyhow::{Context, Result, bail};

use crate::native::bloodprg::{
    CREDITS_VOICE_RESOURCE_PATH, GameLifecycleState, GameTimerContext, GameTimerState, InputAction,
    PresentationPresentPolicy, PresentationResourceId, PresentationRunExit, PresentationRunHost,
    PresentationRunState, ScriptRuntime, advance_game_timer_tick, run_presentation_line_one_stream,
    run_presentation_line_zero,
};

use super::game_lifecycle::arm_requested_speaker_pulse;
use super::{ModernGameServices, RuntimePlatformHost};

const OPENING_PRESENTATION_LINE: u16 = 0;
const CREDITS_PRESENTATION_LINE: u16 = 1;
const PRESENTATION_GATE_ACTIVE: u8 = 1;

/// Terminal state returned by one recovered blocking presentation loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimePresentationRunOutcome {
    /// Native loop gate that ended playback.
    pub exit: PresentationRunExit,
    /// Whether SDL requested application shutdown while the presentation ran.
    pub shutdown_requested: bool,
}

/// Run line zero or line one through the translated queue and modern host services.
pub fn run_runtime_presentation<'window>(
    line: PresentationResourceId,
    link_target: u16,
    services: &mut ModernGameServices<'window>,
    platform: &mut RuntimePlatformHost<'window>,
    timer: &mut GameTimerState,
    startup_timer_runtime: &mut ScriptRuntime,
) -> Result<RuntimePresentationRunOutcome> {
    let mut host = RuntimePresentationRunHost {
        services,
        platform,
        input_state: GameLifecycleState::default(),
        timer,
        startup_timer_runtime,
        active_policy: None,
    };
    let mut state = PresentationRunState {
        active_line: None,
        input_stop_gate: u8::MIN,
        presentation_gate: u8::MIN,
        plane_blit_crop_enabled: true,
        resource_vertical_offset: u16::MIN,
    };
    let exit = match line.get() {
        OPENING_PRESENTATION_LINE => {
            run_presentation_line_zero(&mut state, link_target, &mut host)?
        }
        CREDITS_PRESENTATION_LINE => {
            run_presentation_line_one_stream(&mut state, link_target, &mut host)?
        }
        other => bail!("presentation runner has no recovered blocking loop for line {other}"),
    };
    Ok(RuntimePresentationRunOutcome {
        exit,
        shutdown_requested: host.input_state.exit_requested,
    })
}

struct RuntimePresentationRunHost<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    platform: &'services mut RuntimePlatformHost<'window>,
    input_state: GameLifecycleState,
    timer: &'services mut GameTimerState,
    startup_timer_runtime: &'services mut ScriptRuntime,
    active_policy: Option<PresentationPresentPolicy>,
}

impl RuntimePresentationRunHost<'_, '_> {
    fn advance_timer(&mut self) -> Result<()> {
        let elapsed_ticks = self.platform.take_game_timer_ticks();
        arm_requested_speaker_pulse(&mut self.input_state, self.timer);
        self.services.export_game_timer_state(self.timer)?;
        let mut speaker_gate = None;
        if let Some(profile) = self.services.runtime_mut().current_profile_mut() {
            for _ in u64::MIN..elapsed_ticks {
                speaker_gate = advance_game_timer_tick(
                    self.timer,
                    profile.runtime_mut(),
                    GameTimerContext::default(),
                )
                .speaker_gate
                .or(speaker_gate);
            }
        } else {
            for _ in u64::MIN..elapsed_ticks {
                speaker_gate = advance_game_timer_tick(
                    self.timer,
                    self.startup_timer_runtime,
                    GameTimerContext::default(),
                )
                .speaker_gate
                .or(speaker_gate);
            }
        }
        if let Some(action) = speaker_gate {
            self.services.apply_speaker_gate(action)?;
        }
        self.services.import_game_timer_state(self.timer)
    }
}

fn import_blocking_presentation_input(
    run: &PresentationRunState,
    lifecycle: &mut GameLifecycleState,
) {
    lifecycle.presentation.active_line = run.active_line;
    lifecycle.presentation.c2_presentation_gate =
        run.presentation_gate & PRESENTATION_GATE_ACTIVE != u8::MIN;
}

fn export_blocking_presentation_stop_gate(
    run: &mut PresentationRunState,
    lifecycle: &GameLifecycleState,
) {
    run.input_stop_gate = u8::from(lifecycle.exit_requested);
}

impl PresentationRunHost for RuntimePresentationRunHost<'_, '_> {
    type Error = anyhow::Error;

    fn clear_row_surface(&mut self, palette_index: u8) -> Result<()> {
        self.services
            .runtime_mut()
            .front_buffer_mut()
            .clear(palette_index);
        Ok(())
    }

    fn clear_back_buffer(&mut self, palette_index: u8) -> Result<()> {
        let (front, back) = self.services.runtime_mut().presentation_buffers_mut();
        let _ = front;
        back.fill(palette_index);
        Ok(())
    }

    fn dispatch_input(&mut self, state: &mut PresentationRunState) -> Result<()> {
        self.advance_timer()?;
        import_blocking_presentation_input(state, &mut self.input_state);
        let action = self
            .platform
            .dispatch_events(self.services, &mut self.input_state)?;
        if action == Some(InputAction::Cancel) {
            self.services
                .cancel_lifecycle_presentation(&mut self.input_state)?;
        }
        export_blocking_presentation_stop_gate(state, &self.input_state);
        Ok(())
    }

    fn dispatch_scene(
        &mut self,
        line: u16,
        _link_target: u16,
        state: &mut PresentationRunState,
    ) -> Result<()> {
        if self.active_policy.is_none() {
            let vertical_offset = usize::from(state.resource_vertical_offset);
            let (policy, _secondary_request_pending) =
                PresentationPresentPolicy::for_presentation_line(
                    line,
                    self.services
                        .runtime()
                        .data()
                        .presentation_catalog()
                        .unclamped_line_ids(),
                    vertical_offset,
                );
            self.services.load_presentation_sequence(
                PresentationResourceId::new(line),
                crate::native::bloodprg::PresentationSceneSource::Owned,
                policy,
                self.services.game_timer_tick(),
                false,
            )?;
            self.active_policy = Some(policy);
        } else {
            let timer_tick = self.services.game_timer_tick();
            self.services
                .service_presentation_sequence(timer_tick, false, false)?;
        }

        if self.services.presentation_stream_active() {
            state.presentation_gate = PRESENTATION_GATE_ACTIVE;
        } else {
            self.services.finish_presentation_sequence();
            self.active_policy = None;
            state.presentation_gate = u8::MIN;
        }
        Ok(())
    }

    fn present_frame(&mut self) -> Result<()> {
        self.services.submit_indexed_frame()?;
        self.platform.pace_presentation_frame()?;
        self.services.present_artwork()
    }

    fn load_credits_voice(&mut self, path: &str) -> Result<()> {
        if path != CREDITS_VOICE_RESOURCE_PATH {
            bail!("unexpected credits voice path {path}");
        }
        self.services.load_streamed_voice_resource(path.as_bytes())
    }

    fn start_voice_stream(&mut self) -> Result<()> {
        self.services.start_loaded_streamed_voice()
    }

    fn clear_live_palette(&mut self) -> Result<()> {
        self.services.clear_live_palette();
        Ok(())
    }

    fn refill_voice_stream(&mut self) -> Result<()> {
        self.services
            .refill_navigation_music()
            .context("servicing the SDL credits voice stream")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commander_blood_formats::bloodprg::BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT;

    const SHIPPED_UNCLAMPED_LINES: [u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT] =
        [41, 42, 0, 1, 4, 5, 6, 44];

    #[test]
    fn blocking_lines_use_the_recovered_scene_dispatch_policies() {
        let (opening, opening_request_pending) = PresentationPresentPolicy::for_presentation_line(
            OPENING_PRESENTATION_LINE,
            &SHIPPED_UNCLAMPED_LINES,
            usize::MIN,
        );
        assert_eq!(
            opening,
            PresentationPresentPolicy {
                skip_back_buffer_present: true,
                unclamped_rows: true,
                ..PresentationPresentPolicy::default()
            }
        );
        assert!(opening_request_pending);

        let (credits, credits_request_pending) = PresentationPresentPolicy::for_presentation_line(
            CREDITS_PRESENTATION_LINE,
            &SHIPPED_UNCLAMPED_LINES,
            usize::MIN,
        );
        assert!(credits.skip_back_buffer_present);
        assert!(credits.unclamped_rows);
        assert!(credits_request_pending);
    }

    #[test]
    fn blocking_cancel_state_does_not_alias_the_shutdown_gate() {
        let mut run = PresentationRunState {
            active_line: Some(OPENING_PRESENTATION_LINE),
            input_stop_gate: u8::MIN,
            presentation_gate: PRESENTATION_GATE_ACTIVE,
            plane_blit_crop_enabled: true,
            resource_vertical_offset: u16::MIN,
        };
        let mut lifecycle = GameLifecycleState::default();

        import_blocking_presentation_input(&run, &mut lifecycle);
        export_blocking_presentation_stop_gate(&mut run, &lifecycle);

        assert_eq!(lifecycle.presentation.active_line, run.active_line);
        assert!(lifecycle.presentation.c2_presentation_gate);
        assert_eq!(run.input_stop_gate, u8::MIN);

        lifecycle.exit_requested = true;
        export_blocking_presentation_stop_gate(&mut run, &lifecycle);
        assert_eq!(run.input_stop_gate, PRESENTATION_GATE_ACTIVE);
    }
}
