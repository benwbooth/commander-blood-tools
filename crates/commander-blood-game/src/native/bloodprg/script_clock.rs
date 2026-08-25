//! Real-time BloodScript conditions evaluated against typed host clock values.

use commander_blood_formats::instruction::{
    ScriptDateGuard, ScriptHourGuard, ScriptTemporalRelation,
};

use super::{ScriptControl, ScriptRuntime, ScriptRuntimeError};

/// Host clock fields observed by the original CA and CB handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptClock {
    /// Current signed hour value.
    pub hour: i16,
    /// Current signed day-of-month byte.
    pub day: i8,
    /// Current signed month byte.
    pub month: i8,
}

impl ScriptClock {
    /// Apply `vm_op_ca_compare_var` using signed hour comparison.
    pub fn evaluate_hour_guard(
        self,
        guard: ScriptHourGuard,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        apply_condition(
            relation_matches(guard.relation(), guard.hour(), self.hour),
            runtime,
        )
    }

    /// Apply `vm_op_cb_compare_byte` using signed month/day comparison.
    ///
    /// The token's encoded year is intentionally absent from [`ScriptClock`]:
    /// the native routine consumes that word but never compares it.
    pub fn evaluate_date_guard(
        self,
        guard: ScriptDateGuard,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        let authored = (guard.month(), guard.day());
        let current = (self.month, self.day);
        apply_condition(
            relation_matches(guard.relation(), authored, current),
            runtime,
        )
    }
}

fn relation_matches<T: Ord>(relation: ScriptTemporalRelation, authored: T, current: T) -> bool {
    match relation {
        ScriptTemporalRelation::After => authored > current,
        ScriptTemporalRelation::Before => authored < current,
        ScriptTemporalRelation::Equal => authored == current,
    }
}

fn apply_condition(
    condition: bool,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptRuntimeError> {
    if condition {
        Ok(ScriptControl::Continue)
    } else {
        runtime.fail_guard()
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::{ScriptCodeOffset, decode_script_code};
    use commander_blood_formats::instruction::{
        decode_script_date_guard, decode_script_hour_guard,
    };
    use serde::Deserialize;

    use super::*;

    const HOUR_GUARD_OPCODE: u8 = 0xCA;
    const DATE_GUARD_OPCODE: u8 = 0xCB;
    const CODE_END_MARKER: u8 = 0xFF;

    #[derive(Deserialize)]
    struct HourGuardOracle {
        tag_word: u16,
        effective_tag: u8,
        value: i16,
        compare: i16,
        comparison_passed: bool,
    }

    #[derive(Deserialize)]
    struct DateGuardOracle {
        tag: u8,
        pair_low: i8,
        pair_high: i8,
        compare_low: i8,
        compare_high: i8,
        padding_word: u16,
        comparison_passed: bool,
    }

    fn assert_control(passed: bool, control: ScriptControl, failure_target: ScriptCodeOffset) {
        assert_eq!(
            control,
            if passed {
                ScriptControl::Continue
            } else {
                ScriptControl::Jump(failure_target)
            }
        );
    }

    #[test]
    fn hour_guards_match_every_original_signed_comparison_vector() {
        let vectors: Vec<HourGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64e5_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let mut bytes = vec![HOUR_GUARD_OPCODE];
            bytes.extend_from_slice(&vector.tag_word.to_le_bytes());
            bytes.extend_from_slice(&(vector.value as u16).to_le_bytes());
            bytes.push(CODE_END_MARKER);
            let code = decode_script_code(&bytes).unwrap();
            let guard = decode_script_hour_guard(&code.tokens()[0]).unwrap();
            let failure_target = ScriptCodeOffset::new(1);
            let mut runtime = ScriptRuntime::new();
            runtime.begin_root_guard(failure_target);

            let control = ScriptClock {
                hour: vector.compare,
                day: i8::MIN,
                month: i8::MIN,
            }
            .evaluate_hour_guard(guard, &mut runtime)
            .unwrap();

            assert_eq!(vector.tag_word as u8, vector.effective_tag);
            assert_control(vector.comparison_passed, control, failure_target);
        }
    }

    #[test]
    fn date_guards_match_every_original_signed_lexicographic_vector() {
        let vectors: Vec<DateGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6510_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let mut bytes = vec![
                DATE_GUARD_OPCODE,
                vector.tag,
                vector.pair_low as u8,
                vector.pair_high as u8,
            ];
            bytes.extend_from_slice(&vector.padding_word.to_le_bytes());
            bytes.push(CODE_END_MARKER);
            let code = decode_script_code(&bytes).unwrap();
            let guard = decode_script_date_guard(&code.tokens()[0]).unwrap();
            let failure_target = ScriptCodeOffset::new(1);
            let mut runtime = ScriptRuntime::new();
            runtime.begin_root_guard(failure_target);

            let control = ScriptClock {
                hour: i16::MIN,
                day: vector.compare_low,
                month: vector.compare_high,
            }
            .evaluate_date_guard(guard, &mut runtime)
            .unwrap();

            assert_eq!(guard.encoded_year(), vector.padding_word);
            assert_control(vector.comparison_passed, control, failure_target);
        }
    }
}
