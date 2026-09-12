//! BBB executable comparisons for shared sequence, procedure, and yield handlers.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptInstruction, decode_script_instruction, decode_script_procedure_activation,
    decode_script_procedure_gate, decode_script_topic_offer,
};
use commander_blood_formats::script::{decode_script_dictionary, decode_script_directory};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x40;
const PROCEDURE_KIND: u16 = 2;

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum Vector {
    TopicOffer {
        active: u8,
        operand: u16,
        offered_before: u16,
        stored: bool,
        offered_after: u16,
        cursor: usize,
    },
    ProcedureGate {
        flags: u8,
        target: u16,
        query_before: u8,
        enabled: bool,
        query_after: u8,
        root_after: u16,
        depth_after: usize,
        cursor: usize,
    },
    ProcedureActivation {
        value: u8,
        target_before: u8,
        target_after: u8,
        cursor: usize,
    },
    Yield {
        yield_before: u8,
        yield_after: u8,
        cursor: usize,
    },
    SelectorYield {
        yield_before: u8,
        yield_after: u8,
        cursor: usize,
    },
}

fn procedure_directory() -> commander_blood_formats::script::ScriptDirectory {
    let mut encoded = [u8::MIN; 20];
    encoded[0] = b'p';
    encoded[16..18].copy_from_slice(&1u16.to_le_bytes());
    encoded[18..20].copy_from_slice(&PROCEDURE_KIND.to_le_bytes());
    decode_script_directory(&encoded).unwrap()
}

fn terminated(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(u8::MAX);
    bytes
}

#[test]
fn sequel_sequence_procedure_and_yield_handlers_use_shared_typed_state() {
    const VECTOR_COUNT: usize = 96;

    let vectors: Vec<Vector> = include_str!(
        "../../../../../re/tools/oracle_vectors/big_bug_bang_sequence_procedure.jsonl"
    )
    .lines()
    .map(|line| serde_json::from_str(line).unwrap())
    .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);
    let dictionary_data = vec![u8::MIN; usize::from(u16::MAX) + 1];
    let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
    let resolve = |offset: u16| {
        (offset != u16::MIN).then(|| dictionary.resolve_source_offset(offset).unwrap())
    };
    let directory = procedure_directory();

    for vector in vectors {
        match vector {
            Vector::TopicOffer {
                active,
                operand,
                offered_before,
                stored,
                offered_after,
                cursor,
            } => {
                let mut bytes = vec![0xA7];
                bytes.extend_from_slice(&operand.to_le_bytes());
                let code =
                    decode_script_code_for_dialect(&terminated(bytes), ScriptDialect::BigBugBang)
                        .unwrap();
                let offer = decode_script_topic_offer(&code.tokens()[0], &dictionary).unwrap();
                let mut state = SequencePresentationState {
                    presentation_active: active & 1 != u8::MIN,
                    offered_topic: resolve(offered_before),
                    ..SequencePresentationState::default()
                };
                assert_eq!(
                    offer_topic_if_presentation_active(offer, &mut state),
                    stored
                );
                assert_eq!(state.offered_topic, resolve(offered_after));
                assert_eq!(
                    SCRIPT_CURSOR + code.tokens()[0].encoded_bytes().len() - 1,
                    cursor
                );
            }
            Vector::ProcedureGate {
                flags,
                target,
                query_before,
                enabled,
                query_after,
                root_after,
                depth_after,
                cursor,
            } => {
                let mut bytes = vec![0xA9, flags];
                bytes.extend_from_slice(&target.to_le_bytes());
                let code =
                    decode_script_code_for_dialect(&terminated(bytes), ScriptDialect::BigBugBang)
                        .unwrap();
                let gate = decode_script_procedure_gate(&code.tokens()[0], &directory).unwrap();
                assert_eq!(gate.initially_enabled, enabled);
                let procedures = ScriptProcedureStates::from_gates(&[gate]).unwrap();
                let initial_root = ScriptCodeOffset::new(0x5AA5);
                let mut runtime = ScriptRuntime::new();
                if query_before == u8::MIN {
                    runtime.arm_root_failure_target(initial_root);
                } else {
                    runtime.begin_root_guard(initial_root);
                }
                let control = evaluate_procedure_gate(gate, &procedures, &mut runtime).unwrap();
                let expected_control = if enabled {
                    ScriptControl::Continue
                } else {
                    ScriptControl::Jump(ScriptCodeOffset::new(usize::from(target)))
                };
                assert_eq!(control, expected_control);
                assert_eq!(runtime.query_mode(), query_after != u8::MIN);
                assert_eq!(runtime.guard_depth(), depth_after);
                assert_eq!(
                    runtime.current_guard_target(),
                    Some(ScriptCodeOffset::new(usize::from(root_after)))
                );
                let rust_cursor = match control {
                    ScriptControl::Continue => {
                        SCRIPT_CURSOR + code.tokens()[0].encoded_bytes().len() - 1
                    }
                    ScriptControl::Jump(destination) => destination.index(),
                };
                assert_eq!(rust_cursor, cursor);
            }
            Vector::ProcedureActivation {
                value,
                target_before,
                target_after,
                cursor,
            } => {
                assert_eq!(target_after, value);
                let initially_enabled = target_before & 1 != u8::MIN;
                let mut bytes = vec![0xA9, u8::from(initially_enabled), 0, 0, 0xAB, value];
                bytes.extend_from_slice(&1u16.to_le_bytes());
                let code =
                    decode_script_code_for_dialect(&terminated(bytes), ScriptDialect::BigBugBang)
                        .unwrap();
                assert_eq!(code.tokens().len(), 2);
                let gate = decode_script_procedure_gate(&code.tokens()[0], &directory).unwrap();
                let activation =
                    decode_script_procedure_activation(&code.tokens()[1], &directory).unwrap();
                let mut procedures = ScriptProcedureStates::from_gates(&[gate]).unwrap();
                apply_procedure_activation(activation, &mut procedures).unwrap();
                assert_eq!(
                    procedures.is_enabled(gate.procedure).unwrap(),
                    target_after & 1 != u8::MIN
                );
                assert_eq!(activation.enabled, value & 1 != u8::MIN);
                assert_eq!(
                    SCRIPT_CURSOR + code.tokens()[1].encoded_bytes().len() - 1,
                    cursor
                );
            }
            Vector::Yield {
                yield_before,
                yield_after,
                cursor,
            } => {
                let code =
                    decode_script_code_for_dialect(&[0xAA, u8::MAX], ScriptDialect::BigBugBang)
                        .unwrap();
                let instruction =
                    decode_script_instruction(&code.tokens()[0], &dictionary).unwrap();
                assert_eq!(instruction, ScriptInstruction::Yield);
                let mut runtime = ScriptRuntime::new();
                if yield_before != u8::MIN {
                    runtime.request_yield();
                }
                assert_eq!(
                    runtime
                        .apply_instruction(
                            &instruction,
                            &mut crate::native::random::BloodPrng::default()
                        )
                        .unwrap(),
                    ScriptControl::Continue
                );
                assert_eq!(runtime.yield_requested(), yield_after != u8::MIN);
                assert_eq!(SCRIPT_CURSOR, cursor);
            }
            Vector::SelectorYield {
                yield_before,
                yield_after,
                cursor,
            } => {
                let mut runtime = ScriptRuntime::new();
                if yield_before != u8::MIN {
                    runtime.request_selector_yield();
                }
                runtime.request_selector_yield();
                assert_eq!(runtime.yield_requested(), yield_after != u8::MIN);
                assert_eq!(SCRIPT_CURSOR, cursor);
            }
        }
    }
}
