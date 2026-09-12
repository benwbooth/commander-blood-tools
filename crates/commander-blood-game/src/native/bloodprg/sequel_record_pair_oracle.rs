//! BBB executable comparison for the shared B8/B9/BD record-pair handler.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::decode_script_record_pair_operation;
use commander_blood_formats::script::{decode_script_directory, decode_script_state_for_dialect};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x40;
const BRANCH_TARGET: usize = 0x5AA5;
const OWNER_OFFSET: u16 = 34;
const PAIR_OFFSET: u16 = OWNER_OFFSET + 24;

#[derive(Debug, Deserialize)]
struct Vector {
    opcode: u8,
    query_before: u8,
    query_after: u8,
    requested_pair: [u16; 2],
    pair_before: [u16; 2],
    pair_after: [u16; 2],
    reference_before: String,
    reference_after: String,
    owner_lookup_called: bool,
    failed: bool,
    guard_depth_after: usize,
    cursor: usize,
}

fn directory() -> commander_blood_formats::script::ScriptDirectory {
    let mut encoded = [u8::MIN; 60];
    encoded[..5].copy_from_slice(b"other");
    encoded[16..18].copy_from_slice(&0u16.to_le_bytes());
    encoded[18..20].copy_from_slice(&1u16.to_le_bytes());
    encoded[20..25].copy_from_slice(b"owner");
    encoded[36..38].copy_from_slice(&OWNER_OFFSET.to_le_bytes());
    encoded[38..40].copy_from_slice(&1u16.to_le_bytes());
    encoded[40..48].copy_from_slice(b"sentinel");
    encoded[56..58].copy_from_slice(&108u16.to_le_bytes());
    decode_script_directory(&encoded).unwrap()
}

fn reference(
    name: &str,
    owner: commander_blood_formats::script::ScriptObjectId,
    other: commander_blood_formats::script::ScriptObjectId,
) -> Option<commander_blood_formats::script::ScriptObjectId> {
    match name {
        "none" => None,
        "owner" => Some(owner),
        "other" => Some(other),
        _ => panic!("invalid reference {name}"),
    }
}

#[test]
fn sequel_b8_b9_bd_use_shared_record_pair_state() {
    const VECTOR_COUNT: usize = 720;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_record_pair.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);
    let directory = directory();
    let owner = directory.find_active_object(b"owner").unwrap();
    let other = directory.find_active_object(b"other").unwrap();

    for vector in vectors {
        let mut bytes = vec![vector.opcode];
        bytes.extend_from_slice(&PAIR_OFFSET.to_le_bytes());
        bytes.extend_from_slice(&vector.requested_pair[0].to_le_bytes());
        bytes.extend_from_slice(&vector.requested_pair[1].to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let mut state_bytes = vec![u8::MIN; 108];
        state_bytes[..2].copy_from_slice(&1u16.to_le_bytes());
        state_bytes[usize::from(OWNER_OFFSET)..usize::from(OWNER_OFFSET) + 2]
            .copy_from_slice(&2u16.to_le_bytes());
        let mut state =
            decode_script_state_for_dialect(&state_bytes, &directory, ScriptDialect::BigBugBang)
                .unwrap();
        let operation = decode_script_record_pair_operation(&code.tokens()[0], &state).unwrap();
        assert_eq!(operation.target.object(), Some(owner), "{vector:?}");
        assert!(state.set_word_pair(operation.target, vector.pair_before));
        let mut active_reference =
            ScriptRecordPairReference::new(reference(&vector.reference_before, owner, other));

        let mut runtime = ScriptRuntime::new();
        if vector.query_before & 1 != u8::MIN {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }
        let control =
            apply_record_pair_operation(operation, &mut state, &mut active_reference, &mut runtime)
                .unwrap();

        assert_eq!(state.word_pair(operation.target), Some(vector.pair_after));
        assert_eq!(
            active_reference.object(),
            reference(&vector.reference_after, owner, other)
        );
        assert_eq!(vector.owner_lookup_called, vector.query_before & 1 == 0);
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
