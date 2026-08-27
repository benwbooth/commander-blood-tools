//! Presentation-line coordinators over semantic host operations.

const LINE_ZERO: u16 = 0;
const LINE_ONE: u16 = 1;
const ACTIVE_GATE_FLAG: u8 = 1;
const CLEAR_PALETTE_INDEX: u8 = 0;

/// Authored DOS path of the credits voice stream.
pub const CREDITS_VOICE_RESOURCE_PATH: &str = "mu\\credits.voc";

/// Shared presentation state touched by the line-zero and line-one runners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationRunState {
    /// Currently dispatched scene line, or no line after line-zero teardown.
    pub active_line: Option<u16>,
    /// Navigation-choice/input stop gate.
    pub input_stop_gate: u8,
    /// C2 scene-presentation continuation gate.
    pub presentation_gate: u8,
    /// Whether cropped ship-plane blits remain enabled.
    pub plane_blit_crop_enabled: bool,
    /// Vertical resource draw offset.
    pub resource_vertical_offset: u16,
}

/// Why one presentation loop stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationRunExit {
    /// Input set the low stop bit before another scene dispatch.
    InputRequested,
    /// Scene dispatch cleared the low presentation gate bit.
    SceneCompleted,
}

/// SDL, audio, scene, and renderer operations used by presentation runners.
pub trait PresentationRunHost {
    /// Host operation failure.
    type Error;

    /// Clear the native row-fill surface with one palette index.
    fn clear_row_surface(&mut self, palette_index: u8) -> Result<(), Self::Error>;
    /// Clear the complete back buffer with one palette index.
    fn clear_back_buffer(&mut self, palette_index: u8) -> Result<(), Self::Error>;
    /// Pump input and update the semantic input stop gate.
    fn dispatch_input(&mut self, state: &mut PresentationRunState) -> Result<(), Self::Error>;
    /// Advance the selected scene line and update its continuation gate.
    fn dispatch_scene(
        &mut self,
        line: u16,
        link_target: u16,
        state: &mut PresentationRunState,
    ) -> Result<(), Self::Error>;
    /// Submit the logical framebuffer and any dirty palette changes.
    fn present_frame(&mut self) -> Result<(), Self::Error>;
    /// Load the authored credits voice stream.
    fn load_credits_voice(&mut self, path: &str) -> Result<(), Self::Error>;
    /// Begin playback of the loaded stream.
    fn start_voice_stream(&mut self) -> Result<(), Self::Error>;
    /// Clear the live indexed palette before credits playback.
    fn clear_live_palette(&mut self) -> Result<(), Self::Error>;
    /// Refill streaming audio during one presented scene frame.
    fn refill_voice_stream(&mut self) -> Result<(), Self::Error>;
}

/// Run presentation line zero until input or scene completion.
///
/// This translates `presentation_line_zero_run` at BLOODPRG routine offset
/// `0x001EC1`. Initial buffer clears, input-before-scene ordering, low-bit gate
/// checks, per-frame presentation, and unconditional three-field teardown are
/// retained. Semantic host calls replace planar conversion, VGA page changes,
/// palette uploads, and global near callbacks.
pub fn run_presentation_line_zero<Host: PresentationRunHost>(
    state: &mut PresentationRunState,
    link_target: u16,
    host: &mut Host,
) -> Result<PresentationRunExit, Host::Error> {
    host.clear_row_surface(CLEAR_PALETTE_INDEX)?;
    host.clear_back_buffer(CLEAR_PALETTE_INDEX)?;
    state.active_line = Some(LINE_ZERO);

    let exit = loop {
        host.dispatch_input(state)?;
        if state.input_stop_gate & ACTIVE_GATE_FLAG != u8::MIN {
            break PresentationRunExit::InputRequested;
        }
        host.dispatch_scene(LINE_ZERO, link_target, state)?;
        if state.presentation_gate & ACTIVE_GATE_FLAG == u8::MIN {
            break PresentationRunExit::SceneCompleted;
        }
        host.present_frame()?;
    };

    state.input_stop_gate = u8::MIN;
    state.presentation_gate = u8::MIN;
    state.active_line = None;
    Ok(exit)
}

/// Run streamed presentation line one until input or scene completion.
///
/// This translates `presentation_line_one_stream_run` at BLOODPRG routine
/// offset `0x001F10`. Credits-stream setup, palette and surface clears,
/// input-before-scene ordering, audio refill before every presented frame, and
/// return-without-teardown semantics remain exact. SDL audio and wgpu frame
/// submission replace the DOS stream driver and VGA operations.
pub fn run_presentation_line_one_stream<Host: PresentationRunHost>(
    state: &mut PresentationRunState,
    link_target: u16,
    host: &mut Host,
) -> Result<PresentationRunExit, Host::Error> {
    state.input_stop_gate = u8::MIN;
    state.plane_blit_crop_enabled = false;
    state.resource_vertical_offset = u16::MIN;
    state.active_line = Some(LINE_ONE);

    host.load_credits_voice(CREDITS_VOICE_RESOURCE_PATH)?;
    host.start_voice_stream()?;
    host.clear_live_palette()?;
    host.clear_row_surface(CLEAR_PALETTE_INDEX)?;
    host.clear_back_buffer(CLEAR_PALETTE_INDEX)?;

    loop {
        host.dispatch_input(state)?;
        if state.input_stop_gate & ACTIVE_GATE_FLAG != u8::MIN {
            return Ok(PresentationRunExit::InputRequested);
        }
        host.dispatch_scene(LINE_ONE, link_target, state)?;
        if state.presentation_gate & ACTIVE_GATE_FLAG == u8::MIN {
            return Ok(PresentationRunExit::SceneCompleted);
        }
        host.refill_voice_stream()?;
        host.present_frame()?;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::convert::Infallible;

    use serde::Deserialize;

    use super::*;

    const VECTOR_COUNT: usize = 3;
    const INITIAL_PRESENTATION_GATE: u8 = 90;
    const INITIAL_VERTICAL_OFFSET: u16 = 30_600;

    #[derive(Deserialize)]
    struct RunOracle {
        name: String,
        link_target_offset: u16,
        calls: Vec<serde_json::Value>,
        final_state: FinalStateOracle,
    }

    #[derive(Deserialize)]
    struct FinalStateOracle {
        active_line: u16,
        input_stop: u8,
        presentation_gate: u8,
        crop_enabled: u8,
        vertical_offset: u16,
    }

    struct OracleHost {
        input_stops: VecDeque<u8>,
        presentation_gates: VecDeque<u8>,
        calls: Vec<&'static str>,
    }

    impl PresentationRunHost for OracleHost {
        type Error = Infallible;

        fn clear_row_surface(&mut self, color: u8) -> Result<(), Self::Error> {
            assert_eq!(color, CLEAR_PALETTE_INDEX);
            self.calls.push("blit_fill_row_5221");
            Ok(())
        }

        fn clear_back_buffer(&mut self, color: u8) -> Result<(), Self::Error> {
            assert_eq!(color, CLEAR_PALETTE_INDEX);
            self.calls.push("back_buffer_fill");
            Ok(())
        }

        fn dispatch_input(&mut self, state: &mut PresentationRunState) -> Result<(), Self::Error> {
            self.calls.push("input_action_dispatch");
            state.input_stop_gate = self.input_stops.pop_front().unwrap();
            Ok(())
        }

        fn dispatch_scene(
            &mut self,
            _line: u16,
            _link_target: u16,
            state: &mut PresentationRunState,
        ) -> Result<(), Self::Error> {
            self.calls.push("dlg_line_id_scene_dispatch");
            state.presentation_gate = self.presentation_gates.pop_front().unwrap();
            Ok(())
        }

        fn present_frame(&mut self) -> Result<(), Self::Error> {
            self.calls.extend([
                "chunky_to_planar_framebuffer",
                "page_offset_helper",
                "palette_upload_if_dirty",
            ]);
            Ok(())
        }

        fn load_credits_voice(&mut self, path: &str) -> Result<(), Self::Error> {
            assert_eq!(path, CREDITS_VOICE_RESOURCE_PATH);
            self.calls.push("snd_stream_source_load");
            Ok(())
        }

        fn start_voice_stream(&mut self) -> Result<(), Self::Error> {
            self.calls.push("snd_stream_start");
            Ok(())
        }

        fn clear_live_palette(&mut self) -> Result<(), Self::Error> {
            self.calls.push("vga_dac_clear");
            Ok(())
        }

        fn refill_voice_stream(&mut self) -> Result<(), Self::Error> {
            self.calls.push("snd_stream_refill");
            Ok(())
        }
    }

    #[test]
    fn line_zero_matches_every_original_run_vector() {
        verify_runs(
            include_str!("../../../../../re/tools/oracle_vectors/func_1ec1_natural.json"),
            run_presentation_line_zero,
        );
    }

    #[test]
    fn streamed_line_one_matches_every_original_run_vector() {
        verify_runs(
            include_str!("../../../../../re/tools/oracle_vectors/func_1f10_natural.json"),
            run_presentation_line_one_stream,
        );
    }

    #[test]
    fn credits_voice_uses_the_archive_authored_dos_path() {
        assert_eq!(CREDITS_VOICE_RESOURCE_PATH, "mu\\credits.voc");
    }

    fn verify_runs(
        input: &str,
        run: fn(
            &mut PresentationRunState,
            u16,
            &mut OracleHost,
        ) -> Result<PresentationRunExit, Infallible>,
    ) {
        let vectors: Vec<RunOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), VECTOR_COUNT);
        for vector in vectors {
            let (mut host, expected_calls) = host_for(&vector);
            let mut state = initial_state();
            run(&mut state, vector.link_target_offset, &mut host).unwrap();
            assert_eq!(host.calls, expected_calls, "{}", vector.name);
            assert_final_state(state, &vector);
        }
    }

    fn host_for(vector: &RunOracle) -> (OracleHost, Vec<&str>) {
        let mut input_stops = VecDeque::new();
        let mut presentation_gates = VecDeque::new();
        let mut expected_calls = Vec::new();
        for call in &vector.calls {
            let name = call["call"].as_str().unwrap();
            expected_calls.push(name);
            if name == "input_action_dispatch" {
                input_stops.push_back(call["stop"].as_u64().unwrap() as u8);
            } else if name == "dlg_line_id_scene_dispatch" {
                presentation_gates.push_back(call["gate"].as_u64().unwrap() as u8);
            }
        }
        (
            OracleHost {
                input_stops,
                presentation_gates,
                calls: Vec::new(),
            },
            expected_calls,
        )
    }

    fn initial_state() -> PresentationRunState {
        PresentationRunState {
            active_line: Some(47_891),
            input_stop_gate: 119,
            presentation_gate: INITIAL_PRESENTATION_GATE,
            plane_blit_crop_enabled: true,
            resource_vertical_offset: INITIAL_VERTICAL_OFFSET,
        }
    }

    fn assert_final_state(state: PresentationRunState, vector: &RunOracle) {
        let active_line =
            (vector.final_state.active_line != u16::MAX).then_some(vector.final_state.active_line);
        assert_eq!(state.active_line, active_line, "{}", vector.name);
        assert_eq!(
            state.input_stop_gate, vector.final_state.input_stop,
            "{}",
            vector.name
        );
        assert_eq!(
            state.presentation_gate, vector.final_state.presentation_gate,
            "{}",
            vector.name
        );
        assert_eq!(
            state.plane_blit_crop_enabled,
            vector.final_state.crop_enabled != u8::MIN,
            "{}",
            vector.name
        );
        assert_eq!(
            state.resource_vertical_offset, vector.final_state.vertical_offset,
            "{}",
            vector.name
        );
    }
}
