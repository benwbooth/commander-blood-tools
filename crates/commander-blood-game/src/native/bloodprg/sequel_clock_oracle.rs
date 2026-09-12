//! BBB executable comparison for the inherited CA/CB host-clock guards.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{decode_script_date_guard, decode_script_hour_guard};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x40;
const BRANCH_TARGET: usize = 0x5AA5;

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum Vector {
    Hour {
        tag_word: u16,
        authored: i16,
        current: i16,
        query_before: u8,
        query_after: u8,
        failed: bool,
        guard_depth_after: usize,
        cursor: usize,
    },
    Date {
        tag: u8,
        authored_day: i8,
        authored_month: i8,
        encoded_year: u16,
        current_day: i8,
        current_month: i8,
        query_before: u8,
        query_after: u8,
        failed: bool,
        guard_depth_after: usize,
        cursor: usize,
    },
}

fn assert_outcome(
    vector: &Vector,
    token_size: usize,
    query_before: u8,
    query_after: u8,
    failed: bool,
    guard_depth_after: usize,
    cursor: usize,
    control: ScriptControl,
    runtime: &ScriptRuntime,
) {
    assert_ne!(query_before & 1, 0, "{vector:?}");
    assert_eq!(runtime.query_mode(), query_after & 1 != 0, "{vector:?}");
    assert_eq!(runtime.guard_depth(), guard_depth_after, "{vector:?}");
    assert_eq!(
        control,
        if failed {
            ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
        } else {
            ScriptControl::Continue
        },
        "{vector:?}"
    );
    let typed_cursor = match control {
        ScriptControl::Jump(target) => target.index(),
        ScriptControl::Continue => SCRIPT_CURSOR + token_size - 1,
    };
    assert_eq!(typed_cursor, cursor, "{vector:?}");
}

#[test]
fn sequel_ca_cb_use_shared_signed_host_clock_guards() {
    const VECTOR_COUNT: usize = 1_764;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_clock_guard.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        match &vector {
            Vector::Hour {
                tag_word,
                authored,
                current,
                query_before,
                query_after,
                failed,
                guard_depth_after,
                cursor,
            } => {
                let mut bytes = vec![0xCA];
                bytes.extend_from_slice(&tag_word.to_le_bytes());
                bytes.extend_from_slice(&authored.to_le_bytes());
                bytes.push(u8::MAX);
                let code =
                    decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
                let guard = decode_script_hour_guard(&code.tokens()[0]).unwrap();
                let mut runtime = ScriptRuntime::new();
                runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
                let control = ScriptClock {
                    hour: *current,
                    day: i8::MIN,
                    month: i8::MIN,
                }
                .evaluate_hour_guard(guard, &mut runtime)
                .unwrap();

                assert_outcome(
                    &vector,
                    code.tokens()[0].encoded_bytes().len(),
                    *query_before,
                    *query_after,
                    *failed,
                    *guard_depth_after,
                    *cursor,
                    control,
                    &runtime,
                );
            }
            Vector::Date {
                tag,
                authored_day,
                authored_month,
                encoded_year,
                current_day,
                current_month,
                query_before,
                query_after,
                failed,
                guard_depth_after,
                cursor,
            } => {
                let mut bytes = vec![0xCB, *tag, *authored_day as u8, *authored_month as u8];
                bytes.extend_from_slice(&encoded_year.to_le_bytes());
                bytes.push(u8::MAX);
                let code =
                    decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
                let guard = decode_script_date_guard(&code.tokens()[0]).unwrap();
                assert_eq!(guard.encoded_year(), *encoded_year, "{vector:?}");
                let mut runtime = ScriptRuntime::new();
                runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
                let control = ScriptClock {
                    hour: i16::MIN,
                    day: *current_day,
                    month: *current_month,
                }
                .evaluate_date_guard(guard, &mut runtime)
                .unwrap();

                assert_outcome(
                    &vector,
                    code.tokens()[0].encoded_bytes().len(),
                    *query_before,
                    *query_after,
                    *failed,
                    *guard_depth_after,
                    *cursor,
                    control,
                    &runtime,
                );
            }
        }
    }
}
