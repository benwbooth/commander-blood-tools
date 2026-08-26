//! Per-frame execution of one decoded BloodScript COD program.

use std::fmt;

use commander_blood_formats::code::{ScriptCode, ScriptCodeOffset, ScriptToken};
use commander_blood_formats::instruction::DecodedScriptInstruction;

use super::ScriptRuntime;

/// Control returned by one translated COD instruction handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptFrameFlow {
    /// Continue ordinary execution and apply an authored token skip or rewind.
    Continue,
    /// Continue after presentation work while discarding a pending token skip.
    ContinueAfterPresentation,
    /// Save the next instruction for a later resumed presentation pass.
    SaveResumeCursor,
}

/// Typed result returned by one COD instruction handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptFrameStep {
    /// Next decoded instruction position selected by the handler.
    pub next_instruction: ScriptCodeOffset,
    /// Frame-level control associated with that destination.
    pub flow: ScriptFrameFlow,
}

impl ScriptFrameStep {
    /// Continue ordinary execution at an explicit decoded position.
    pub const fn continue_at(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptFrameFlow::Continue,
        }
    }

    /// Continue after publishing presentation state and clear pending skips.
    pub const fn continue_after_presentation(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptFrameFlow::ContinueAfterPresentation,
        }
    }

    /// Save an explicit decoded position for the next resumed frame.
    pub const fn save_resume_cursor(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptFrameFlow::SaveResumeCursor,
        }
    }
}

/// Semantic operations surrounding the decoded COD traversal.
pub trait ScriptFrameHost {
    /// Typed failure returned by state preparation, dispatch, or post-scans.
    type Error;

    /// Update transient object state before the first instruction executes.
    fn prepare_script_state(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;

    /// Execute one decoded instruction and select its next source position.
    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, Self::Error>;

    /// Commit any concept selected during this frame after traversal stops.
    fn commit_selected_concept(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;

    /// Scan presentation records after selected-concept processing.
    fn scan_presentation(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;
}

/// Semantic operations surrounding traversal of a pre-decoded COD program.
pub trait DecodedScriptFrameHost {
    /// Typed failure returned by state preparation, dispatch, or post-scans.
    type Error;

    /// Update transient object state before the first instruction executes.
    fn prepare_script_state(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;

    /// Execute one pre-bound instruction and select its next source position.
    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        instruction: &DecodedScriptInstruction,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, Self::Error>;

    /// Commit any concept selected during this frame after traversal stops.
    fn commit_selected_concept(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;

    /// Scan presentation records after selected-concept processing.
    fn scan_presentation(&mut self, runtime: &mut ScriptRuntime) -> Result<(), Self::Error>;
}

/// Reason one script frame completed without an execution error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptFrameEnd {
    /// Per-frame script execution was disabled.
    ExecutionDisabled,
    /// The decoded COD end marker terminated this pass.
    EndMarker,
    /// Selector-active execution reached its authored loop boundary.
    ResumeBoundary,
}

/// Observable summary of one script-frame traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptFrameOutcome {
    /// Why the frame stopped.
    pub end: ScriptFrameEnd,
    /// Position at which execution stopped, absent on a disabled frame.
    pub next_instruction: Option<ScriptCodeOffset>,
    /// Number of translated instruction handlers invoked.
    pub executed_instructions: usize,
    /// Number of decoded instructions bypassed by authored skip state.
    pub skipped_instructions: usize,
    /// Number of handlers that published presentation work.
    pub presentation_yields: usize,
}

/// Invalid typed traversal state or translated host failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptFrameError<HostError> {
    /// The retained semantic stream is not parallel to the framed COD tokens.
    InstructionCountMismatch {
        /// Number of losslessly framed tokens.
        token_count: usize,
        /// Number of retained semantic instructions.
        instruction_count: usize,
    },
    /// A framed source position has no corresponding retained instruction.
    MissingDecodedInstruction {
        /// Source position lacking semantic state.
        source_offset: ScriptCodeOffset,
    },
    /// Selector-active execution lacks its saved restart position.
    MissingResumeCursor,
    /// Presentation requested a saved cursor before any loop target existed.
    MissingResumeTarget,
    /// No decoded instruction begins at a requested position.
    MissingInstruction {
        /// Missing serialized source position.
        source_offset: ScriptCodeOffset,
    },
    /// An authored skip attempted to pass the program end marker.
    SkipCrossesProgramEnd {
        /// End marker reached before the skip count was exhausted.
        source_offset: ScriptCodeOffset,
    },
    /// State preparation, instruction dispatch, or a post-scan failed.
    Host(HostError),
}

impl<HostError: fmt::Debug> fmt::Display for ScriptFrameError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HostError: fmt::Debug> std::error::Error for ScriptFrameError<HostError> {}

/// Execute one complete decoded COD pass through translated semantic handlers.
///
/// This translates `vm_run_wrapper` at BLOODPRG file offset `0x0055A4`.
/// Loaded profiles already own all five decoded script resources, so the
/// native per-frame far-pointer resolution is deliberately absent. Typed
/// source positions replace the mutable 16-bit cursor and its wraparound.
pub fn execute_script_frame<Host: ScriptFrameHost>(
    code: &ScriptCode,
    execution_enabled: bool,
    runtime: &mut ScriptRuntime,
    host: &mut Host,
) -> Result<ScriptFrameOutcome, ScriptFrameError<Host::Error>> {
    execute_script_frame_inner(code, execution_enabled, runtime, &mut RawFrameHost(host))
}

/// Execute one COD pass using the typed stream retained by a loaded profile.
pub fn execute_decoded_script_frame<Host: DecodedScriptFrameHost>(
    code: &ScriptCode,
    instructions: &[DecodedScriptInstruction],
    execution_enabled: bool,
    runtime: &mut ScriptRuntime,
    host: &mut Host,
) -> Result<ScriptFrameOutcome, ScriptFrameError<Host::Error>> {
    if code.tokens().len() != instructions.len() {
        return Err(ScriptFrameError::InstructionCountMismatch {
            token_count: code.tokens().len(),
            instruction_count: instructions.len(),
        });
    }
    execute_script_frame_inner(
        code,
        execution_enabled,
        runtime,
        &mut TypedFrameHost {
            tokens: code.tokens(),
            instructions,
            host,
        },
    )
}

trait FrameHost {
    type Error;

    fn prepare_script_state(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>>;

    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, ScriptFrameError<Self::Error>>;

    fn commit_selected_concept(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>>;

    fn scan_presentation(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>>;
}

struct RawFrameHost<'a, Host>(&'a mut Host);

impl<Host: ScriptFrameHost> FrameHost for RawFrameHost<'_, Host> {
    type Error = Host::Error;

    fn prepare_script_state(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.0
            .prepare_script_state(runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, ScriptFrameError<Self::Error>> {
        self.0
            .execute_instruction(token, runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn commit_selected_concept(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.0
            .commit_selected_concept(runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn scan_presentation(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.0
            .scan_presentation(runtime)
            .map_err(ScriptFrameError::Host)
    }
}

struct TypedFrameHost<'a, Host> {
    tokens: &'a [ScriptToken],
    instructions: &'a [DecodedScriptInstruction],
    host: &'a mut Host,
}

impl<Host: DecodedScriptFrameHost> FrameHost for TypedFrameHost<'_, Host> {
    type Error = Host::Error;

    fn prepare_script_state(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.host
            .prepare_script_state(runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn execute_instruction(
        &mut self,
        token: &ScriptToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptFrameStep, ScriptFrameError<Self::Error>> {
        let index = self
            .tokens
            .binary_search_by_key(&token.source_offset(), ScriptToken::source_offset)
            .map_err(|_| ScriptFrameError::MissingDecodedInstruction {
                source_offset: token.source_offset(),
            })?;
        let instruction =
            self.instructions
                .get(index)
                .ok_or(ScriptFrameError::MissingDecodedInstruction {
                    source_offset: token.source_offset(),
                })?;
        self.host
            .execute_instruction(token, instruction, runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn commit_selected_concept(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.host
            .commit_selected_concept(runtime)
            .map_err(ScriptFrameError::Host)
    }

    fn scan_presentation(
        &mut self,
        runtime: &mut ScriptRuntime,
    ) -> Result<(), ScriptFrameError<Self::Error>> {
        self.host
            .scan_presentation(runtime)
            .map_err(ScriptFrameError::Host)
    }
}

fn execute_script_frame_inner<Host: FrameHost>(
    code: &ScriptCode,
    execution_enabled: bool,
    runtime: &mut ScriptRuntime,
    host: &mut Host,
) -> Result<ScriptFrameOutcome, ScriptFrameError<Host::Error>> {
    if !execution_enabled {
        return Ok(ScriptFrameOutcome {
            end: ScriptFrameEnd::ExecutionDisabled,
            next_instruction: None,
            executed_instructions: usize::MIN,
            skipped_instructions: usize::MIN,
            presentation_yields: usize::MIN,
        });
    }

    host.prepare_script_state(runtime)?;

    let mut cursor = if runtime.selector_resume_active() {
        runtime
            .saved_resume_cursor()
            .ok_or(ScriptFrameError::MissingResumeCursor)?
    } else {
        code.tokens()
            .first()
            .map_or(code.end_marker_offset(), ScriptToken::source_offset)
    };
    let mut executed_instructions = usize::MIN;
    let mut skipped_instructions = usize::MIN;
    let mut presentation_yields = usize::MIN;

    let end = loop {
        if cursor == code.end_marker_offset() {
            break ScriptFrameEnd::EndMarker;
        }
        let token = token_at(code, cursor).ok_or(ScriptFrameError::MissingInstruction {
            source_offset: cursor,
        })?;
        let step = host.execute_instruction(token, runtime)?;
        executed_instructions += 1;
        cursor = step.next_instruction;

        match step.flow {
            ScriptFrameFlow::Continue => {
                if let Some(skip_count) = runtime.take_actionable_skip_count() {
                    for _ in u8::MIN..skip_count {
                        if cursor == code.end_marker_offset() {
                            return Err(ScriptFrameError::SkipCrossesProgramEnd {
                                source_offset: cursor,
                            });
                        }
                        let skipped =
                            token_at(code, cursor).ok_or(ScriptFrameError::MissingInstruction {
                                source_offset: cursor,
                            })?;
                        cursor = skipped.end_offset();
                        skipped_instructions += 1;
                    }
                } else if let Some(target) = runtime.take_loop_resume_target() {
                    cursor = target;
                    continue;
                }
            }
            ScriptFrameFlow::ContinueAfterPresentation => {
                runtime.clear_pending_skip_count();
                presentation_yields += 1;
            }
            ScriptFrameFlow::SaveResumeCursor => {
                if !runtime.save_resume_cursor(cursor) {
                    return Err(ScriptFrameError::MissingResumeTarget);
                }
                presentation_yields += 1;
            }
        }

        if runtime.selector_resume_active() {
            let target = runtime
                .resume_target()
                .expect("selector-active resume retains its typed target");
            if cursor >= target {
                break ScriptFrameEnd::ResumeBoundary;
            }
        }
    };

    host.commit_selected_concept(runtime)?;
    host.scan_presentation(runtime)?;

    Ok(ScriptFrameOutcome {
        end,
        next_instruction: Some(cursor),
        executed_instructions,
        skipped_instructions,
        presentation_yields,
    })
}

fn token_at(code: &ScriptCode, source_offset: ScriptCodeOffset) -> Option<&ScriptToken> {
    code.tokens()
        .binary_search_by_key(&source_offset, ScriptToken::source_offset)
        .ok()
        .map(|index| &code.tokens()[index])
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::decode_complete_script_instruction;
    use commander_blood_formats::script::{
        decode_script_dictionary, decode_script_directory, decode_script_state,
    };
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::ScriptResumePhase;

    const ORACLE_VECTOR_COUNT: usize = 14;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const TEST_OPCODE: u8 = 0xAA;
    const END_OPCODE: u8 = 0xFF;

    #[derive(Deserialize)]
    struct FrameOracle {
        name: String,
        enabled: bool,
        flow: Vec<String>,
        state_after: FrameOracleState,
        result: u16,
    }

    #[derive(Deserialize)]
    struct FrameOracleState {
        resume: u8,
        skip: u8,
        lock: u8,
    }

    #[derive(Clone, Copy)]
    enum HostAction {
        Continue,
        ArmSkip(u8),
        ContinueAfterPresentation,
        SaveResumeCursor,
        InvalidLegacyYield,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestHostError {
        InvalidLegacyYield,
    }

    #[derive(Default)]
    struct RecordingHost {
        actions: VecDeque<HostAction>,
        events: Vec<&'static str>,
    }

    impl RecordingHost {
        fn with_actions(actions: impl IntoIterator<Item = HostAction>) -> Self {
            Self {
                actions: actions.into_iter().collect(),
                events: Vec::new(),
            }
        }
    }

    impl ScriptFrameHost for RecordingHost {
        type Error = TestHostError;

        fn prepare_script_state(
            &mut self,
            _runtime: &mut ScriptRuntime,
        ) -> Result<(), Self::Error> {
            self.events.push("state_processor");
            Ok(())
        }

        fn execute_instruction(
            &mut self,
            token: &ScriptToken,
            runtime: &mut ScriptRuntime,
        ) -> Result<ScriptFrameStep, Self::Error> {
            self.events.push("handler");
            match self.actions.pop_front().unwrap_or(HostAction::Continue) {
                HostAction::Continue => Ok(ScriptFrameStep::continue_at(token.end_offset())),
                HostAction::ArmSkip(count) => {
                    runtime.arm_skip(count);
                    Ok(ScriptFrameStep::continue_at(token.end_offset()))
                }
                HostAction::ContinueAfterPresentation => Ok(
                    ScriptFrameStep::continue_after_presentation(token.end_offset()),
                ),
                HostAction::SaveResumeCursor => {
                    Ok(ScriptFrameStep::save_resume_cursor(token.end_offset()))
                }
                HostAction::InvalidLegacyYield => {
                    self.events.push("error");
                    Err(TestHostError::InvalidLegacyYield)
                }
            }
        }

        fn commit_selected_concept(
            &mut self,
            _runtime: &mut ScriptRuntime,
        ) -> Result<(), Self::Error> {
            self.events.push("flag_test");
            Ok(())
        }

        fn scan_presentation(&mut self, _runtime: &mut ScriptRuntime) -> Result<(), Self::Error> {
            self.events.push("presentation_scan");
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingDecodedHost {
        opcodes: Vec<u8>,
    }

    impl DecodedScriptFrameHost for RecordingDecodedHost {
        type Error = Infallible;

        fn prepare_script_state(
            &mut self,
            _runtime: &mut ScriptRuntime,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn execute_instruction(
            &mut self,
            token: &ScriptToken,
            _instruction: &DecodedScriptInstruction,
            _runtime: &mut ScriptRuntime,
        ) -> Result<ScriptFrameStep, Self::Error> {
            self.opcodes.push(token.opcode().byte());
            Ok(ScriptFrameStep::continue_at(token.end_offset()))
        }

        fn commit_selected_concept(
            &mut self,
            _runtime: &mut ScriptRuntime,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn scan_presentation(&mut self, _runtime: &mut ScriptRuntime) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn code_with_token_count(count: usize) -> ScriptCode {
        let mut bytes = vec![TEST_OPCODE; count];
        bytes.push(END_OPCODE);
        decode_script_code(&bytes).unwrap()
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn normalized_original_flow(flow: &[String]) -> Vec<&str> {
        flow.iter()
            .filter_map(|event| match event.as_str() {
                "rtc_time" | "rtc_date" | "resource" | "token" => None,
                event if event.starts_with("handler:") => Some("handler"),
                event => Some(event),
            })
            .collect()
    }

    #[test]
    fn script_frame_accounts_for_every_original_natural_vector() {
        let vectors: Vec<FrameOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_55a4_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut runtime = ScriptRuntime::new();
            let (code, actions) = match vector.name.as_str() {
                "execution_disabled_is_a_noop"
                | "immediate_end_runs_post_scan"
                | "unloaded_handles_retain_previous_pointer" => {
                    (code_with_token_count(usize::MIN), Vec::new())
                }
                "two_handlers_dispatch_in_order" => {
                    (code_with_token_count(2), vec![HostAction::Continue; 2])
                }
                "resume_window_stops_at_loop_target" => {
                    let code = code_with_token_count(3);
                    runtime.arm_resume(ScriptCodeOffset::new(2), u16::MIN);
                    assert!(runtime.save_resume_cursor(ScriptCodeOffset::new(usize::MIN)));
                    (code, vec![HostAction::Continue; 2])
                }
                "resume_state_one_rewinds_to_loop_target" => {
                    runtime.arm_resume(ScriptCodeOffset::new(1), u16::MIN);
                    (code_with_token_count(1), vec![HostAction::Continue])
                }
                "skip_count_advances_two_tokens" => {
                    (code_with_token_count(3), vec![HostAction::ArmSkip(2)])
                }
                "skip_high_nibble_alone_does_not_advance" => {
                    runtime.arm_skip(16);
                    (code_with_token_count(1), vec![HostAction::Continue])
                }
                "yield_two_sets_lock_and_clears_skip" => {
                    runtime.arm_skip(7);
                    (
                        code_with_token_count(1),
                        vec![HostAction::ContinueAfterPresentation],
                    )
                }
                "yield_three_saves_resume_cursor" => {
                    runtime.retain_resume_target(ScriptCodeOffset::new(32));
                    (code_with_token_count(1), vec![HostAction::SaveResumeCursor])
                }
                "yield_three_arms_resume_hold_immediately" => {
                    runtime.arm_resume(ScriptCodeOffset::new(1), u16::MIN);
                    (code_with_token_count(1), vec![HostAction::SaveResumeCursor])
                }
                "yield_one_is_a_coding_error" | "yield_four_is_a_coding_error" => (
                    code_with_token_count(1),
                    vec![HostAction::InvalidLegacyYield],
                ),
                "script_pointer_offset_wraps" => {
                    (code_with_token_count(1), vec![HostAction::Continue])
                }
                unknown => panic!("unaccounted 0x0055A4 oracle vector {unknown}"),
            };
            let mut host = RecordingHost::with_actions(actions);
            let result = execute_script_frame(&code, vector.enabled, &mut runtime, &mut host);

            if vector.result == u16::MAX {
                assert_eq!(
                    result,
                    Err(ScriptFrameError::Host(TestHostError::InvalidLegacyYield)),
                    "{}",
                    vector.name
                );
            } else {
                let outcome = result.unwrap();
                assert_eq!(
                    outcome.end == ScriptFrameEnd::ExecutionDisabled,
                    !vector.enabled,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                host.events,
                normalized_original_flow(&vector.flow),
                "{}",
                vector.name
            );
            let resume_phase = runtime.resume_state().map(|resume| resume.phase);
            let expected_phase = match vector.state_after.resume {
                0 => None,
                1 => Some(ScriptResumePhase::LoopArmed),
                2 => Some(ScriptResumePhase::SelectorResumeActive),
                value => panic!("unexpected native resume state {value}"),
            };
            assert_eq!(resume_phase, expected_phase, "{}", vector.name);
            assert_eq!(
                runtime.pending_skip_count().unwrap_or(u8::MIN),
                vector.state_after.skip,
                "{}",
                vector.name
            );
            assert_eq!(
                vector.state_after.lock != u8::MIN,
                vector.name.starts_with("yield_"),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn every_shipped_cod_image_traverses_to_its_decoded_end() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let mut runtime = ScriptRuntime::new();
            let mut host = RecordingHost::default();
            let outcome = execute_script_frame(&code, true, &mut runtime, &mut host).unwrap();

            assert_eq!(
                outcome.end,
                ScriptFrameEnd::EndMarker,
                "SCRIPT{profile}.COD"
            );
            assert_eq!(
                outcome.executed_instructions,
                code.tokens().len(),
                "SCRIPT{profile}.COD"
            );
            assert_eq!(outcome.skipped_instructions, usize::MIN);
            assert_eq!(outcome.presentation_yields, usize::MIN);
        }
    }

    #[test]
    fn every_shipped_cod_image_traverses_its_parallel_semantic_stream() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let dictionary = decode_script_dictionary(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();
            let instructions = code
                .tokens()
                .iter()
                .map(|token| {
                    decode_complete_script_instruction(token, &state, &directory, &dictionary)
                })
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut runtime = ScriptRuntime::new();
            let mut host = RecordingDecodedHost::default();
            let outcome =
                execute_decoded_script_frame(&code, &instructions, true, &mut runtime, &mut host)
                    .unwrap();

            assert_eq!(outcome.end, ScriptFrameEnd::EndMarker);
            assert_eq!(outcome.executed_instructions, instructions.len());
            assert_eq!(
                host.opcodes,
                code.tokens()
                    .iter()
                    .map(|token| token.opcode().byte())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn typed_frame_rejects_a_nonparallel_instruction_stream() {
        let code = code_with_token_count(1);
        let mut runtime = ScriptRuntime::new();
        let mut host = RecordingDecodedHost::default();

        assert_eq!(
            execute_decoded_script_frame(&code, &[], true, &mut runtime, &mut host),
            Err(ScriptFrameError::InstructionCountMismatch {
                token_count: 1,
                instruction_count: 0,
            })
        );
        assert!(host.opcodes.is_empty());
    }

    #[test]
    fn malformed_targets_and_skips_fail_without_post_scans() {
        let code = code_with_token_count(1);
        let mut runtime = ScriptRuntime::new();
        runtime.arm_skip(1);
        let mut host = RecordingHost::default();
        assert_eq!(
            execute_script_frame(&code, true, &mut runtime, &mut host),
            Err(ScriptFrameError::SkipCrossesProgramEnd {
                source_offset: code.end_marker_offset(),
            })
        );
        assert_eq!(host.events, ["state_processor", "handler"]);

        let mut runtime = ScriptRuntime::new();
        runtime.arm_resume(ScriptCodeOffset::new(1), u16::MIN);
        assert!(runtime.activate_selector_resume());
        let mut host = RecordingHost::default();
        assert_eq!(
            execute_script_frame(&code, true, &mut runtime, &mut host),
            Err(ScriptFrameError::MissingResumeCursor)
        );
        assert_eq!(host.events, ["state_processor"]);
    }
}
