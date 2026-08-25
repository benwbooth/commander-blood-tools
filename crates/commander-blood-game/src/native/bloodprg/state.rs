//! Typed shared operations over flat, owned BloodScript VAR state.

use std::fmt;

use commander_blood_formats::instruction::{
    ScriptSharedStateOperation, ScriptStateOperand, ScriptStateOperator,
};
use commander_blood_formats::script::{ScriptState, ScriptStateWord};

use super::{ScriptControl, ScriptRuntime, ScriptRuntimeError};

/// Invalid state or control flow while applying a shared VAR operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptStateOperationError {
    /// A resolved word belongs to a different or truncated profile state.
    MissingStateWord {
        /// Typed word that could not be accessed.
        word: ScriptStateWord,
    },
    /// A failed query had no procedure or nested guard destination.
    Control(ScriptRuntimeError),
}

impl fmt::Display for ScriptStateOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptStateOperationError {}

/// Apply `vm_op_shared_state_marker` to typed object or trailing-state words.
pub fn apply_shared_state_operation(
    operation: ScriptSharedStateOperation,
    state: &mut ScriptState,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptStateOperationError> {
    let current = read_state_word(state, operation.target)?;
    let operand = match operation.operand {
        ScriptStateOperand::Immediate(value) => value,
        ScriptStateOperand::StateWord(word) => read_state_word(state, word)?,
    };

    if runtime.query_mode() {
        let passes = match operation.operator {
            ScriptStateOperator::NotEqual => current != operand,
            ScriptStateOperator::LessThan => (current as i16) < operand as i16,
            ScriptStateOperator::GreaterThan => (current as i16) > operand as i16,
            ScriptStateOperator::LessThanOrEqual => (current as i16) <= operand as i16,
            ScriptStateOperator::GreaterThanOrEqual => (current as i16) >= operand as i16,
            ScriptStateOperator::EqualOrAssign => current == operand,
            ScriptStateOperator::Add
            | ScriptStateOperator::Subtract
            | ScriptStateOperator::PreserveOrFail(_) => false,
        };
        if passes {
            Ok(ScriptControl::Continue)
        } else {
            runtime
                .fail_guard()
                .map_err(ScriptStateOperationError::Control)
        }
    } else {
        let updated = match operation.operator {
            ScriptStateOperator::EqualOrAssign => operand,
            ScriptStateOperator::Add => current.wrapping_add(operand),
            ScriptStateOperator::Subtract => current.wrapping_sub(operand),
            ScriptStateOperator::NotEqual
            | ScriptStateOperator::LessThan
            | ScriptStateOperator::GreaterThan
            | ScriptStateOperator::LessThanOrEqual
            | ScriptStateOperator::GreaterThanOrEqual
            | ScriptStateOperator::PreserveOrFail(_) => current,
        };
        if !state.set_word(operation.target, updated) {
            return Err(ScriptStateOperationError::MissingStateWord {
                word: operation.target,
            });
        }
        Ok(ScriptControl::Continue)
    }
}

fn read_state_word(
    state: &ScriptState,
    word: ScriptStateWord,
) -> Result<u16, ScriptStateOperationError> {
    state
        .word(word)
        .ok_or(ScriptStateOperationError::MissingStateWord { word })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::{decode_script_code, ScriptCodeOffset};
    use commander_blood_formats::instruction::decode_script_shared_state_operation;
    use commander_blood_formats::script::{decode_script_directory, decode_script_state};
    use serde::Deserialize;

    use super::*;

    const SHARED_STATE_OPCODE: u8 = 0xB1;
    const END_MARKER: u8 = 0xFF;
    const TARGET_SOURCE_OFFSET: u16 = 2;
    const OPERAND_SOURCE_OFFSET: u16 = 4;
    const INDIRECT_STATE_MODE_A: u8 = 0xC0;
    const INDIRECT_STATE_MODE_B: u8 = 0xC2;
    const QUERY_MODE_MASK: u8 = 1;
    const BRANCH_TARGET: usize = 9_320;
    const SHARED_STATE_VECTOR_COUNT: usize = 20;

    #[derive(Deserialize)]
    struct SharedStateOracle {
        operation: u8,
        rhs_mode: u8,
        current_before: u16,
        resolved_rhs: u16,
        field_after: u16,
        query_mode_before: u8,
        query_mode_after: u8,
        branch_failed: bool,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn shared_state_matches_every_original_handler_vector() {
        let vectors: Vec<SharedStateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6863_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SHARED_STATE_VECTOR_COUNT);
        let directory_data = std::fs::read(original_asset("SCRIPT1.DEB")).unwrap();
        let state_data = std::fs::read(original_asset("SCRIPT1.VAR")).unwrap();
        let directory = decode_script_directory(&directory_data).unwrap();

        for vector in vectors {
            let encoded_operand = if matches!(
                vector.rhs_mode,
                INDIRECT_STATE_MODE_A | INDIRECT_STATE_MODE_B
            ) {
                OPERAND_SOURCE_OFFSET
            } else {
                vector.resolved_rhs
            };
            let token_data = [
                SHARED_STATE_OPCODE,
                TARGET_SOURCE_OFFSET as u8,
                (TARGET_SOURCE_OFFSET >> u8::BITS) as u8,
                vector.operation,
                vector.rhs_mode,
                encoded_operand as u8,
                (encoded_operand >> u8::BITS) as u8,
                END_MARKER,
            ];
            let code = decode_script_code(&token_data).unwrap();
            let mut state = decode_script_state(&state_data, &directory).unwrap();
            let operation =
                decode_script_shared_state_operation(&code.tokens()[0], &state).unwrap();
            assert!(state.set_word(operation.target, vector.current_before));
            if let ScriptStateOperand::StateWord(operand) = operation.operand {
                assert!(state.set_word(operand, vector.resolved_rhs));
            }
            let mut runtime = ScriptRuntime::new();
            if vector.query_mode_before & QUERY_MODE_MASK != u8::MIN {
                runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
            }

            let control =
                apply_shared_state_operation(operation, &mut state, &mut runtime).unwrap();

            assert_eq!(state.word(operation.target), Some(vector.field_after));
            assert_eq!(
                runtime.query_mode(),
                vector.query_mode_after & QUERY_MODE_MASK != u8::MIN
            );
            assert_eq!(
                control,
                if vector.branch_failed {
                    ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
                } else {
                    ScriptControl::Continue
                }
            );
        }
    }
}
