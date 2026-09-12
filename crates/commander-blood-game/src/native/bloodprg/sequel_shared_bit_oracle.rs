//! BBB executable comparison for the shared AE/B0 masked-bit handler.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::decode_script_shared_bit_operation;
use commander_blood_formats::script::{decode_script_directory, decode_script_state_for_dialect};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x40;
const TARGET_SOURCE_OFFSET: u16 = 2;
const BRANCH_TARGET: usize = 0x5AA5;

#[derive(Debug, Deserialize)]
struct Vector {
    opcode: u8,
    query_before: u8,
    query_after: u8,
    inverted: bool,
    field_before: u16,
    field_after: u16,
    mask: u16,
    failed: bool,
    guard_depth_after: usize,
    cursor: usize,
}

#[test]
fn sequel_ae_b0_use_shared_masked_bit_state() {
    const VECTOR_COUNT: usize = 480;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_shared_bit.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);
    let directory = decode_script_directory(&[]).unwrap();

    for vector in vectors {
        let mut bytes = vec![vector.opcode];
        if vector.inverted {
            bytes.push(0xA1);
        }
        bytes.extend_from_slice(&TARGET_SOURCE_OFFSET.to_le_bytes());
        bytes.extend_from_slice(&vector.mask.to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let mut state =
            decode_script_state_for_dialect(&[0; 8], &directory, ScriptDialect::BigBugBang)
                .unwrap();
        let operation = decode_script_shared_bit_operation(&code.tokens()[0], &state).unwrap();
        assert!(state.set_word(operation.target, vector.field_before));

        let mut runtime = ScriptRuntime::new();
        if vector.query_before & 1 != u8::MIN {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }
        let control = apply_shared_bit_operation(operation, &mut state, &mut runtime).unwrap();

        assert_eq!(state.word(operation.target), Some(vector.field_after));
        assert_eq!(runtime.query_mode(), vector.query_after & 1 != u8::MIN);
        assert_eq!(runtime.guard_depth(), vector.guard_depth_after);
        assert_eq!(
            control,
            if vector.failed {
                ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
            } else {
                ScriptControl::Continue
            }
        );
        let cursor = match control {
            ScriptControl::Jump(target) => target.index(),
            ScriptControl::Continue => SCRIPT_CURSOR + code.tokens()[0].encoded_bytes().len() - 1,
        };
        assert_eq!(cursor, vector.cursor, "{vector:?}");
    }
}
