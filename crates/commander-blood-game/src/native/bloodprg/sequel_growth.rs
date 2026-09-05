//! Big Bug Bang's D6 actor growth, verified against the original native handler.
//!
//! Numeric field positions are confined to binding typed owned VAR words. The
//! arithmetic preserves the native integer widths, not registers or DOS memory.

use std::fmt;

use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::instruction::ScriptSequelGrowthOperation;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference, ScriptStateWord,
};

use super::ScriptControl;

const WORD_BYTES: usize = 2;
const KIND_FIELD: usize = 0;
const FLAGS_FIELD: usize = 2;
const GROUP_FIELD: usize = 20;
const QUANTITY_FIELD: usize = 22;
const LOCATION_FIELD: usize = 24;
const AGGRESSIVENESS_FIELD: usize = 50;
const GROWTH_BALANCE_FIELD: usize = 52;
const PRESSURE_RELIEF_FIELD: usize = 56;
const IN_PLAY_FLAG: u16 = 1;
const PARTICIPATING_FLAG: u16 = 4;
const ENGAGED_FLAG: u16 = 8;
const SCALE: i16 = 1000;
const BALANCE_RECOVERY: i16 = 10;
const PRESSURE_DIVISOR: u32 = 10000;
const GROWTH_DIVISOR: u32 = 100000;
const MINIMUM_QUANTITY: i16 = 5;

/// Main-loop inputs shared by the sequel's simulation handlers.
///
/// The native loop decrements the countdown at file 0x10CA and reloads it at
/// 0x5B46 after COD/presentation processing. D6 itself never advances it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequelSimulationContext {
    /// Remaining simulation delay (native GS:0x0CC6); zero permits updates.
    pub countdown: u16,
    /// `Trashlando`, bound by name at 0x59B7 and excluded by helper 0x706E.
    pub excluded_location: ScriptObjectId,
}

/// Invalid sequel state or the native arithmetic's explicit failure boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelGrowthError {
    /// Commander records must not be interpreted as sequel simulation records.
    WrongDialect,
    /// A required typed object or word is absent.
    InvalidObject {
        /// Object that could not supply its field.
        object: ScriptObjectId,
    },
    /// A selected actor's location does not resolve to an owned object.
    InvalidLocation {
        /// Actor with the invalid relationship.
        actor: ScriptObjectId,
    },
    /// Native word DIV cannot represent the computed pressure quotient.
    PressureOverflow {
        /// Actor at which original execution faults.
        actor: ScriptObjectId,
        /// Mathematical quotient that exceeds a word.
        quotient: u32,
    },
}

impl fmt::Display for SequelGrowthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SequelGrowthError {}

/// Apply the complete D6 handler (0x728B-0x7366) and selection helper (0x706E).
///
/// Updates occur in directory order, including in query mode. A native divide
/// error leaves earlier actor updates and the current actor's preceding clamps
/// visible. Do not make this operation transactional or advance its clock here.
pub fn apply_sequel_growth(
    operation: ScriptSequelGrowthOperation,
    context: SequelSimulationContext,
    state: &mut ScriptState,
) -> Result<ScriptControl, SequelGrowthError> {
    if state.dialect() != ScriptDialect::BigBugBang {
        return Err(SequelGrowthError::WrongDialect);
    }
    if context.countdown != 0 {
        return Ok(ScriptControl::Continue);
    }
    let selected = select_actors(state, operation.group_mask, context.excluded_location)?;
    for actor in selected {
        let aggression = read(state, actor, AGGRESSIVENESS_FIELD)? as i16;
        write(
            state,
            actor,
            AGGRESSIVENESS_FIELD,
            aggression.clamp(0, SCALE) as u16,
        )?;
        if read(state, actor, FLAGS_FIELD)? & ENGAGED_FLAG != 0 {
            continue;
        }
        // Relief has only an upper clamp in the original. Negative authored
        // values therefore participate in the unsigned word pressure product.
        let relief = (read(state, actor, PRESSURE_RELIEF_FIELD)? as i16).min(SCALE);
        write(state, actor, PRESSURE_RELIEF_FIELD, relief as u16)?;
        let quantity = read(state, actor, QUANTITY_FIELD)?;
        let pressure =
            u32::from(SCALE.wrapping_sub(relief) as u16) * u32::from(quantity) / PRESSURE_DIVISOR;
        let pressure =
            u16::try_from(pressure).map_err(|_| SequelGrowthError::PressureOverflow {
                actor,
                quotient: pressure,
            })?;
        let balance = (read(state, actor, GROWTH_BALANCE_FIELD)? as i16)
            .min(SCALE)
            .wrapping_add(BALANCE_RECOVERY)
            .wrapping_sub(pressure as i16)
            .clamp(-SCALE, SCALE);
        write(state, actor, GROWTH_BALANCE_FIELD, balance as u16)?;
        let updated = if balance < 0 {
            quantity / 2
        } else {
            // The two signed products retain their low 32 bits. Native XOR
            // EDX,EDX then makes IDIV consume a zero-extended numerator, not a
            // sign-extended one. Preserve this even for negative input rates.
            let product = i32::from(quantity as i16)
                .wrapping_mul(i32::from(operation.rate))
                .wrapping_mul(i32::from(balance));
            let increment = ((product as u32) / GROWTH_DIVISOR).max(1) as u16;
            quantity.wrapping_add(increment)
        };
        write(
            state,
            actor,
            QUANTITY_FIELD,
            (updated as i16).max(MINIMUM_QUANTITY) as u16,
        )?;
    }
    Ok(ScriptControl::Continue)
}

fn select_actors(
    state: &ScriptState,
    group_mask: u16,
    excluded_location: ScriptObjectId,
) -> Result<Vec<ScriptObjectId>, SequelGrowthError> {
    let mut selected = Vec::new();
    for object in state.objects() {
        if read(state, object.id, KIND_FIELD)? & ScriptObjectKind::Actor.mask() == 0
            || read(state, object.id, GROUP_FIELD)? & group_mask == 0
        {
            continue;
        }
        let flags = read(state, object.id, FLAGS_FIELD)?;
        if flags & IN_PLAY_FLAG == 0 || flags & PARTICIPATING_FLAG == 0 {
            continue;
        }
        let field = field(state, object.id, LOCATION_FIELD)?;
        let Some(ScriptStateObjectReference::Object(location)) = state.object_reference(field)
        else {
            return Err(SequelGrowthError::InvalidLocation { actor: object.id });
        };
        if location != excluded_location && read(state, location, FLAGS_FIELD)? & IN_PLAY_FLAG != 0
        {
            selected.push(object.id);
        }
    }
    Ok(selected)
}

fn field(
    state: &ScriptState,
    object: ScriptObjectId,
    offset: usize,
) -> Result<ScriptStateWord, SequelGrowthError> {
    state
        .object_word(object, offset / WORD_BYTES)
        .ok_or(SequelGrowthError::InvalidObject { object })
}

fn read(
    state: &ScriptState,
    object: ScriptObjectId,
    offset: usize,
) -> Result<u16, SequelGrowthError> {
    state
        .word(field(state, object, offset)?)
        .ok_or(SequelGrowthError::InvalidObject { object })
}

fn write(
    state: &mut ScriptState,
    object: ScriptObjectId,
    offset: usize,
    value: u16,
) -> Result<(), SequelGrowthError> {
    let field = field(state, object, offset)?;
    if state.set_word(field, value) {
        Ok(())
    } else {
        Err(SequelGrowthError::InvalidObject { object })
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::{
        ScriptCodeOffset, ScriptTokenDecoder, decode_script_token,
    };
    use commander_blood_formats::instruction::{
        DecodedScriptInstruction, decode_complete_script_instruction,
    };
    use commander_blood_formats::script::{
        decode_script_dictionary, decode_script_directory, decode_script_state_for_dialect,
    };
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct GrowthOracle {
        name: String,
        token: Vec<u8>,
        countdown: u16,
        query_mode: u8,
        excluded_location: usize,
        directory: Vec<u8>,
        state_before: Vec<u8>,
        state_after: Vec<u8>,
        divide_error: bool,
    }

    #[test]
    fn sequel_growth_matches_complete_native_handler_and_selection_helper() {
        const BEGIN_QUERY: [u8; 3] = [0xA0, 0, 0];
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let mut cases = 0;
        let mut divide_errors = 0;
        for line in
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_growth.jsonl").lines()
        {
            let vector: GrowthOracle = serde_json::from_str(line).unwrap();
            let directory = decode_script_directory(&vector.directory).unwrap();
            let mut state = decode_script_state_for_dialect(
                &vector.state_before,
                &directory,
                ScriptDialect::BigBugBang,
            )
            .unwrap();
            let mut decoder = ScriptTokenDecoder::new(ScriptDialect::BigBugBang);
            if vector.query_mode != 0 {
                decode_script_token(&BEGIN_QUERY, ScriptCodeOffset::new(0), &mut decoder).unwrap();
            }
            let token =
                decode_script_token(&vector.token, ScriptCodeOffset::new(0), &mut decoder).unwrap();
            let DecodedScriptInstruction::SequelGrowth(operation) =
                decode_complete_script_instruction(&token, &state, &directory, &dictionary)
                    .unwrap()
            else {
                panic!("{}: wrong instruction dispatch", vector.name);
            };
            let excluded_location = state
                .objects()
                .iter()
                .find(|object| object.source_offset() == vector.excluded_location)
                .unwrap()
                .id;
            let result = apply_sequel_growth(
                operation,
                SequelSimulationContext {
                    countdown: vector.countdown,
                    excluded_location,
                },
                &mut state,
            );
            assert!(vector.query_mode <= 1);
            if vector.divide_error {
                assert!(
                    matches!(result, Err(SequelGrowthError::PressureOverflow { .. })),
                    "{}: {result:?}",
                    vector.name
                );
                divide_errors += 1;
            } else {
                assert_eq!(result, Ok(ScriptControl::Continue), "{}", vector.name);
            }
            assert_eq!(state.encode(), vector.state_after, "{}", vector.name);
            cases += 1;
        }
        assert_eq!(cases, 126);
        assert_eq!(divide_errors, 18);
    }
}
