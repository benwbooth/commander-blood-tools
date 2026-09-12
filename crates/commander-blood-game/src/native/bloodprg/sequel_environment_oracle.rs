//! BBB executable comparison for the inherited CE-D2 environment handlers.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptEnvironmentInstruction, decode_script_environment_instruction,
    decode_script_profile_request,
};
use commander_blood_formats::script::decode_script_dictionary;
use serde::Deserialize;

use super::*;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Vector {
    Guard {
        opcode: u8,
        flag_offset: u16,
        flag_value: u8,
        query_before: u8,
        query_after: u8,
        guard_depth_before: usize,
        guard_depth_after: usize,
        failure_target: usize,
        branch_taken: bool,
        failure_calls: usize,
        cursor_before: usize,
        cursor_after: usize,
    },
    Clear {
        opcode: u8,
        resume_state_before: u8,
        resume_value_before: u16,
        resume_state_after: u8,
        resume_value_after: u16,
        cursor_before: usize,
        cursor_after: usize,
    },
    ProfileRequest {
        opcode: u8,
        operand: u8,
        request_before: u16,
        request_after: u16,
        cursor_before: usize,
        cursor_after: usize,
    },
}

fn environment_instruction(opcode: u8) -> ScriptEnvironmentInstruction {
    let code =
        decode_script_code_for_dialect(&[opcode, u8::MAX], ScriptDialect::BigBugBang).unwrap();
    let token = &code.tokens()[0];
    assert_eq!(token.encoded_bytes().len(), 1);
    decode_script_environment_instruction(token).unwrap()
}

#[test]
fn sequel_ce_d2_handlers_match_original_environment_vectors() {
    const VECTOR_COUNT: usize = 362;
    const BRIDGE_FLAG: u16 = 0x2A33;
    const TRAVEL_FLAG: u16 = 0x277C;
    const CONTACT_FLAG: u16 = 0x29DD;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_environment.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    let dictionary = decode_script_dictionary(b"alternate\0").unwrap();
    let alternate = dictionary.resolve_source_offset(u16::MIN).unwrap();
    let mut request_operands = [false; 256];

    for vector in vectors {
        match vector {
            Vector::Guard {
                opcode,
                flag_offset,
                flag_value,
                query_before,
                query_after,
                guard_depth_before,
                guard_depth_after,
                failure_target,
                branch_taken,
                failure_calls,
                cursor_before,
                cursor_after,
            } => {
                assert!(query_before & 1 != 0);
                let active = flag_value & 1 != 0;
                let activity = match opcode {
                    0xCE => {
                        assert_eq!(flag_offset, BRIDGE_FLAG);
                        ScriptEnvironmentActivity {
                            bridge_active: active,
                            ..ScriptEnvironmentActivity::default()
                        }
                    }
                    0xD0 => {
                        assert_eq!(flag_offset, TRAVEL_FLAG);
                        ScriptEnvironmentActivity {
                            travel_active: active,
                            ..ScriptEnvironmentActivity::default()
                        }
                    }
                    0xD1 => {
                        assert_eq!(flag_offset, CONTACT_FLAG);
                        ScriptEnvironmentActivity {
                            contact_active: active,
                            ..ScriptEnvironmentActivity::default()
                        }
                    }
                    _ => panic!("unexpected environment guard opcode {opcode:#x}"),
                };
                let first_target = ScriptCodeOffset::new(0x5AA5);
                let failure_target = ScriptCodeOffset::new(failure_target);
                let mut runtime = ScriptRuntime::new();
                runtime.begin_root_guard(first_target);
                if guard_depth_before == 2 {
                    runtime.begin_guard(failure_target);
                } else {
                    assert_eq!(failure_target, first_target);
                }
                assert_eq!(runtime.guard_depth(), guard_depth_before);

                let control = activity
                    .apply(environment_instruction(opcode), &mut runtime)
                    .unwrap();

                assert_eq!(branch_taken, failure_calls == 1);
                assert_eq!(runtime.query_mode(), query_after & 1 != 0);
                assert_eq!(runtime.guard_depth(), guard_depth_after);
                assert_eq!(
                    control,
                    if branch_taken {
                        ScriptControl::Jump(failure_target)
                    } else {
                        ScriptControl::Continue
                    }
                );
                let cursor = match control {
                    ScriptControl::Continue => cursor_before,
                    ScriptControl::Jump(target) => target.index(),
                };
                assert_eq!(cursor, cursor_after);
            }
            Vector::Clear {
                opcode,
                resume_state_before,
                resume_value_before,
                resume_state_after,
                resume_value_after,
                cursor_before,
                cursor_after,
            } => {
                assert_eq!(opcode, 0xCF);
                let mut runtime = ScriptRuntime::new();
                runtime.set_alternate_concept(Some(alternate));
                if resume_state_before != 0 {
                    runtime.arm_resume(ScriptCodeOffset::new(1), resume_value_before);
                }

                let control = ScriptEnvironmentActivity::default()
                    .apply(environment_instruction(opcode), &mut runtime)
                    .unwrap();

                assert_eq!(control, ScriptControl::Continue);
                assert_eq!(runtime.alternate_concept(), None);
                assert_eq!(runtime.resume_state(), None);
                assert_eq!(resume_state_after, 0);
                assert_eq!(resume_value_after, 0);
                assert_eq!(cursor_after, cursor_before);
            }
            Vector::ProfileRequest {
                opcode,
                operand,
                request_before,
                request_after,
                cursor_before,
                cursor_after,
            } => {
                assert_eq!(opcode, 0xD2);
                assert_eq!(request_before, 0xA55A);
                assert!(!request_operands[usize::from(operand)]);
                request_operands[usize::from(operand)] = true;
                let code = decode_script_code_for_dialect(
                    &[opcode, operand, u8::MAX],
                    ScriptDialect::BigBugBang,
                )
                .unwrap();
                let token = &code.tokens()[0];
                assert_eq!(
                    token.encoded_bytes().len() - 1,
                    cursor_after - cursor_before
                );
                let request = decode_script_profile_request(token).unwrap();
                let mut slot = ScriptProfileRequestSlot::default();
                let pending = slot.schedule_for_dialect(request, ScriptDialect::BigBugBang);
                assert_eq!(pending.raw_zero_based_index() as u16, request_after);
            }
        }
    }
    assert!(request_operands.into_iter().all(|seen| seen));
}
