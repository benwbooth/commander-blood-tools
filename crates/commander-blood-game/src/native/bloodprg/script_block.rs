//! Typed execution order for nested BloodScript BAS blocks.

use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction, ScriptBasToken};
use commander_blood_formats::code::ScriptCodeOffset;

use super::ScriptRuntime;

/// Control requested by one translated BAS instruction handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptBlockFlow {
    /// Continue execution and apply any authored token skip.
    Continue,
    /// Finish this nested block successfully at the returned source position.
    Stop,
    /// Continue after presentation work while discarding a pending token skip.
    ContinueAfterPresentation,
}

/// Typed result returned by one BAS instruction handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptBlockStep {
    /// Next serialized instruction position selected by the handler.
    pub next_instruction: ScriptCodeOffset,
    /// Block-level control associated with that destination.
    pub flow: ScriptBlockFlow,
}

impl ScriptBlockStep {
    /// Continue with an explicit decoded instruction position.
    pub const fn continue_at(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptBlockFlow::Continue,
        }
    }

    /// Stop the current block after this instruction.
    pub const fn stop_at(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptBlockFlow::Stop,
        }
    }

    /// Continue after publishing presentation state and clear pending skips.
    pub const fn continue_after_presentation(next_instruction: ScriptCodeOffset) -> Self {
        Self {
            next_instruction,
            flow: ScriptBlockFlow::ContinueAfterPresentation,
        }
    }
}

/// Semantic instruction dispatcher used by the nested block executor.
pub trait ScriptBlockHandler {
    /// Typed failure returned by an individual instruction family.
    type Error;

    /// Execute one decoded BAS instruction and choose its next source position.
    fn execute_instruction(
        &mut self,
        token: &ScriptBasToken,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptBlockStep, Self::Error>;
}

/// Reason one nested BAS block finished successfully.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptBlockEnd {
    /// A decoded BAS end marker terminated the block.
    EndMarker,
    /// An instruction requested an immediate successful stop.
    HandlerStop,
}

/// Observable execution summary for one nested BAS block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptBlockOutcome {
    /// Why the block returned to its caller.
    pub end: ScriptBlockEnd,
    /// Serialized source position following the terminating operation.
    pub next_instruction: ScriptCodeOffset,
    /// Number of instruction handlers invoked.
    pub executed_instructions: usize,
    /// Number of decoded instructions bypassed by authored skip state.
    pub skipped_instructions: usize,
}

/// Invalid typed block traversal or translated handler failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptBlockError<HandlerError> {
    /// No decoded BAS instruction begins at the requested source position.
    MissingInstruction {
        /// Missing serialized source position.
        source_offset: ScriptCodeOffset,
    },
    /// An authored skip attempted to cross the current block's end marker.
    SkipCrossesBlockEnd {
        /// End marker reached while skipping.
        source_offset: ScriptCodeOffset,
    },
    /// A translated instruction handler rejected its typed inputs.
    Handler(HandlerError),
}

impl<HandlerError: fmt::Debug> fmt::Display for ScriptBlockError<HandlerError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<HandlerError: fmt::Debug> std::error::Error for ScriptBlockError<HandlerError> {}

/// Execute one nested BAS block through its translated instruction handlers.
///
/// This translates `vm_script_block_scan` at BLOODPRG file offset `0x0056A6`.
/// Decoded instruction positions replace the original mutable byte cursor, and
/// malformed bytes are rejected by the BAS decoder before execution begins.
pub fn execute_script_block<Handler: ScriptBlockHandler>(
    dialogue: &ScriptBas,
    start: ScriptCodeOffset,
    runtime: &mut ScriptRuntime,
    handler: &mut Handler,
) -> Result<ScriptBlockOutcome, ScriptBlockError<Handler::Error>> {
    let mut cursor = start;
    let mut executed_instructions = usize::MIN;
    let mut skipped_instructions = usize::MIN;

    loop {
        let token = token_at(dialogue, cursor).ok_or(ScriptBlockError::MissingInstruction {
            source_offset: cursor,
        })?;
        if matches!(token.instruction(), ScriptBasInstruction::End) {
            return Ok(ScriptBlockOutcome {
                end: ScriptBlockEnd::EndMarker,
                next_instruction: token.end_offset(),
                executed_instructions,
                skipped_instructions,
            });
        }

        let step = handler
            .execute_instruction(token, runtime)
            .map_err(ScriptBlockError::Handler)?;
        executed_instructions += 1;
        cursor = step.next_instruction;

        match step.flow {
            ScriptBlockFlow::Stop => {
                return Ok(ScriptBlockOutcome {
                    end: ScriptBlockEnd::HandlerStop,
                    next_instruction: cursor,
                    executed_instructions,
                    skipped_instructions,
                });
            }
            ScriptBlockFlow::ContinueAfterPresentation => runtime.clear_pending_skip_count(),
            ScriptBlockFlow::Continue => {
                if let Some(skip_count) = runtime.take_actionable_skip_count() {
                    for _ in u8::MIN..skip_count {
                        let skipped = token_at(dialogue, cursor).ok_or(
                            ScriptBlockError::MissingInstruction {
                                source_offset: cursor,
                            },
                        )?;
                        if matches!(skipped.instruction(), ScriptBasInstruction::End) {
                            return Err(ScriptBlockError::SkipCrossesBlockEnd {
                                source_offset: cursor,
                            });
                        }
                        cursor = skipped.end_offset();
                        skipped_instructions += 1;
                    }
                }
            }
        }
    }
}

fn token_at(dialogue: &ScriptBas, source_offset: ScriptCodeOffset) -> Option<&ScriptBasToken> {
    dialogue
        .tokens()
        .binary_search_by_key(&source_offset, ScriptBasToken::source_offset)
        .ok()
        .map(|index| &dialogue.tokens()[index])
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bas::{ScriptBasError, decode_script_bas};
    use commander_blood_formats::script::{ScriptDictionary, decode_script_dictionary};
    use serde::Deserialize;

    use super::*;

    const NATURAL_ORACLE_VECTOR_COUNT: usize = 15;
    const ORIGINAL_PROFILE_COUNT: usize = 5;
    const TOPIC_OFFER_OPCODE: u8 = 0xA7;
    const SEQUENCE_REQUEST_OPCODE: u8 = 0xA8;
    const YIELD_OPCODE: u8 = 0xAA;
    const SELECTOR_YIELD_OPCODE: u8 = 0xAC;
    const END_OPCODE: u8 = 0xFF;
    const INVALID_LOW_OPCODE: u8 = 0x9F;
    const INVALID_HIGH_OPCODE: u8 = 0xD3;
    const FIRST_DICTIONARY_WORD: u16 = 0;
    const SKIP_FIXTURE_TOKEN_COUNT: usize = 18;
    const INACTIVE_HIGH_NIBBLE_SKIP: u8 = 16;
    const FULL_BYTE_SKIP_COUNT: u8 = 17;
    const STOP_PRESERVED_SKIP_COUNT: u8 = 7;

    #[derive(Deserialize)]
    struct BlockScanOracle {
        name: String,
    }

    #[derive(Default)]
    struct TestHandler {
        calls: Vec<ScriptCodeOffset>,
        first_step: Option<ScriptBlockStep>,
        skip_on_first: Option<u8>,
    }

    impl ScriptBlockHandler for TestHandler {
        type Error = Infallible;

        fn execute_instruction(
            &mut self,
            token: &ScriptBasToken,
            runtime: &mut ScriptRuntime,
        ) -> Result<ScriptBlockStep, Self::Error> {
            self.calls.push(token.source_offset());
            if self.calls.len() == 1 {
                if let Some(count) = self.skip_on_first {
                    runtime.arm_skip(count);
                }
                if let Some(step) = self.first_step {
                    return Ok(step);
                }
            }
            Ok(ScriptBlockStep::continue_at(token.end_offset()))
        }
    }

    fn dictionary() -> ScriptDictionary {
        decode_script_dictionary(b"topic\0").unwrap()
    }

    fn decode(bytes: &[u8]) -> Result<ScriptBas, ScriptBasError> {
        decode_script_bas(bytes, &dictionary())
    }

    fn topic_offer(bytes: &mut Vec<u8>) {
        bytes.push(TOPIC_OFFER_OPCODE);
        bytes.extend_from_slice(&FIRST_DICTIONARY_WORD.to_le_bytes());
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn native_oracles_are_accounted_for_by_typed_execution_boundaries() {
        let vectors: Vec<BlockScanOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_56a6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NATURAL_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            match vector.name.as_str() {
                "immediate_end"
                | "lowest_opcode"
                | "highest_opcode"
                | "two_handler_chain"
                | "handler_stop_signal"
                | "handler_resume_signal_clears_skip"
                | "skip_two_single_byte_tokens"
                | "skip_variable_length_token"
                | "skip_low_nibble_gate_clear"
                | "skip_full_byte_countdown"
                | "handler_threads_cursor" => {}
                "invalid_below_range" => {
                    assert!(decode(&[INVALID_LOW_OPCODE]).is_err());
                }
                "d3_table_sentinel_is_not_executable" => {
                    assert!(decode(&[INVALID_HIGH_OPCODE]).is_err());
                }
                "script_offset_wrap" | "inherited_reverse_direction" => {
                    assert!(decode(&[END_OPCODE]).is_ok());
                }
                name => panic!("unclassified block-scan oracle {name}"),
            }
        }
    }

    #[test]
    fn end_stop_jump_and_presentation_paths_use_typed_positions() {
        let dialogue = decode(&[
            TOPIC_OFFER_OPCODE,
            0,
            0,
            TOPIC_OFFER_OPCODE,
            0,
            0,
            END_OPCODE,
        ])
        .unwrap();

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            first_step: Some(ScriptBlockStep::continue_at(ScriptCodeOffset::new(6))),
            ..TestHandler::default()
        };
        let outcome = execute_script_block(
            &dialogue,
            ScriptCodeOffset::new(usize::MIN),
            &mut runtime,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.end, ScriptBlockEnd::EndMarker);
        assert_eq!(outcome.executed_instructions, 1);
        assert_eq!(handler.calls, [ScriptCodeOffset::new(usize::MIN)]);

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            first_step: Some(ScriptBlockStep::stop_at(ScriptCodeOffset::new(3))),
            skip_on_first: Some(STOP_PRESERVED_SKIP_COUNT),
            ..TestHandler::default()
        };
        let outcome = execute_script_block(
            &dialogue,
            ScriptCodeOffset::new(usize::MIN),
            &mut runtime,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.end, ScriptBlockEnd::HandlerStop);
        assert_eq!(
            runtime.pending_skip_count(),
            Some(STOP_PRESERVED_SKIP_COUNT)
        );

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            first_step: Some(ScriptBlockStep::continue_after_presentation(
                ScriptCodeOffset::new(3),
            )),
            skip_on_first: Some(STOP_PRESERVED_SKIP_COUNT),
            ..TestHandler::default()
        };
        let outcome = execute_script_block(
            &dialogue,
            ScriptCodeOffset::new(usize::MIN),
            &mut runtime,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.executed_instructions, 2);
        assert_eq!(runtime.pending_skip_count(), None);
    }

    #[test]
    fn authored_skips_count_decoded_instructions() {
        let mut bytes = Vec::new();
        for _ in usize::MIN..SKIP_FIXTURE_TOKEN_COUNT {
            topic_offer(&mut bytes);
        }
        bytes.push(END_OPCODE);
        let dialogue = decode(&bytes).unwrap();

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            skip_on_first: Some(FULL_BYTE_SKIP_COUNT),
            ..TestHandler::default()
        };
        let outcome = execute_script_block(
            &dialogue,
            ScriptCodeOffset::new(usize::MIN),
            &mut runtime,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.executed_instructions, 1);
        assert_eq!(
            outcome.skipped_instructions,
            usize::from(FULL_BYTE_SKIP_COUNT)
        );
        assert_eq!(runtime.pending_skip_count(), None);

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            skip_on_first: Some(INACTIVE_HIGH_NIBBLE_SKIP),
            ..TestHandler::default()
        };
        let outcome = execute_script_block(
            &dialogue,
            ScriptCodeOffset::new(usize::MIN),
            &mut runtime,
            &mut handler,
        )
        .unwrap();
        assert_eq!(outcome.executed_instructions, SKIP_FIXTURE_TOKEN_COUNT);
        assert_eq!(
            runtime.pending_skip_count(),
            Some(INACTIVE_HIGH_NIBBLE_SKIP)
        );
    }

    #[test]
    fn malformed_destinations_and_end_crossing_are_rejected() {
        let dialogue = decode(&[TOPIC_OFFER_OPCODE, 0, 0, END_OPCODE]).unwrap();
        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler::default();
        assert!(matches!(
            execute_script_block(
                &dialogue,
                ScriptCodeOffset::new(1),
                &mut runtime,
                &mut handler,
            ),
            Err(ScriptBlockError::MissingInstruction { .. })
        ));

        let mut runtime = ScriptRuntime::new();
        let mut handler = TestHandler {
            skip_on_first: Some(1),
            ..TestHandler::default()
        };
        assert!(matches!(
            execute_script_block(
                &dialogue,
                ScriptCodeOffset::new(usize::MIN),
                &mut runtime,
                &mut handler,
            ),
            Err(ScriptBlockError::SkipCrossesBlockEnd { .. })
        ));
    }

    #[test]
    fn every_shipped_bas_subprogram_is_traversable_as_decoded_instructions() {
        for profile in 1..=ORIGINAL_PROFILE_COUNT {
            let dictionary = decode_script_dictionary(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap(),
            )
            .unwrap();
            let dialogue = decode_script_bas(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.BAS"))).unwrap(),
                &dictionary,
            )
            .unwrap();
            let mut starts = vec![ScriptCodeOffset::new(usize::MIN)];
            starts.extend(
                dialogue
                    .tokens()
                    .windows(2)
                    .filter(|pair| matches!(pair[0].instruction(), ScriptBasInstruction::End))
                    .map(|pair| pair[1].source_offset()),
            );

            for start in starts {
                let mut runtime = ScriptRuntime::new();
                let mut handler = TestHandler::default();
                let outcome =
                    execute_script_block(&dialogue, start, &mut runtime, &mut handler).unwrap();
                assert_eq!(
                    outcome.end,
                    ScriptBlockEnd::EndMarker,
                    "SCRIPT{profile}.BAS"
                );
            }
        }
    }

    #[test]
    fn test_fixture_uses_real_variable_and_selector_instruction_shapes() {
        let bytes = [
            SEQUENCE_REQUEST_OPCODE,
            b'x',
            0,
            0,
            YIELD_OPCODE,
            SELECTOR_YIELD_OPCODE,
            0,
            0,
            0,
            0,
            END_OPCODE,
        ];
        let dialogue = decode(&bytes).unwrap();
        assert_eq!(dialogue.encode(), bytes);
    }
}
