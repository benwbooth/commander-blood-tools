//! SDL, audio, and wgpu host for the recovered blocking presentation runners.

use anyhow::{Context, Result, bail};

use crate::native::bloodprg::{
    CREDITS_VOICE_RESOURCE_PATH, GameLifecycleState, InputAction, PresentationPresentPolicy,
    PresentationResourceId, PresentationRunExit, PresentationRunHost, PresentationRunState,
    run_presentation_line_one_stream, run_presentation_line_zero,
};

use super::{ModernGameServices, RuntimePlatformHost};

const OPENING_PRESENTATION_LINE: u16 = 0;
const CREDITS_PRESENTATION_LINE: u16 = 1;
const PRESENTATION_GATE_ACTIVE: u8 = 1;
const TIMER_TICK_INCREMENT: u16 = 1;

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
) -> Result<RuntimePresentationRunOutcome> {
    let mut host = RuntimePresentationRunHost {
        services,
        platform,
        input_state: GameLifecycleState::default(),
        timer_tick: u16::MIN,
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
    timer_tick: u16,
    active_policy: Option<PresentationPresentPolicy>,
}

impl RuntimePresentationRunHost<'_, '_> {
    fn advance_timer(&mut self) -> u16 {
        self.timer_tick = self.timer_tick.wrapping_add(TIMER_TICK_INCREMENT);
        self.timer_tick
    }

    fn audio_position(&self) -> Result<u16> {
        let position = self
            .services
            .foreground_audio_position()?
            .unwrap_or(u64::MIN);
        Ok(position as u16)
    }
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
        let action = self
            .platform
            .dispatch_events(self.services, &mut self.input_state);
        let cancelled = action == Some(InputAction::Cancel);
        if cancelled {
            self.services.finish_presentation_sequence();
            self.active_policy = None;
        }
        state.input_stop_gate = u8::from(self.input_state.exit_requested || cancelled);
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
                policy,
                self.timer_tick,
                false,
            )?;
            self.active_policy = Some(policy);
        } else {
            let audio_position = self.audio_position()?;
            let timer_tick = self.advance_timer();
            self.services
                .service_presentation_sequence(audio_position, timer_tick, false)?;
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
        self.services.load_voice_resource(path.as_bytes())
    }

    fn start_voice_stream(&mut self) -> Result<()> {
        self.services.start_loaded_voice()
    }

    fn clear_live_palette(&mut self) -> Result<()> {
        self.services.clear_live_palette();
        Ok(())
    }

    fn refill_voice_stream(&mut self) -> Result<()> {
        self.services
            .check_audio()
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
}
