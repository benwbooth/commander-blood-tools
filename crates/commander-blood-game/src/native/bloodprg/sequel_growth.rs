//! Big Bug Bang's D5 settlement and D6 growth, verified against native handlers.
//!
//! Numeric field positions are confined to binding typed owned VAR words. The
//! arithmetic preserves the native integer widths, not registers or DOS memory.

use std::fmt;

use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::instruction::{
    ScriptSequelGrowthOperation, ScriptSequelSettlementOperation,
};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference, ScriptStateWord,
};

use super::{
    ScriptControl, ScriptNavigationError, navigation_candidates, resolve_navigation_position,
};

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
const SETTLEMENT_MINIMUM_SOURCE_QUANTITY: i16 = 300;
const SETTLEMENT_INITIAL_QUANTITY: u16 = 10;
const LOCATION_PARENT_FIELD: usize = 20;
const LOCATION_SOURCE_ACTOR_FIELD: usize = 24;
const BODY_POSITION_FIELD: usize = 24;
const SETTLEMENT_RANGE: u32 = 250;
const CLOSEST_DISTANCE_INITIAL: i32 = 160000;

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

/// Native named-object bindings used by the D5 settlement path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequelSettlementContext {
    /// Countdown and excluded source location shared with D6.
    pub simulation: SequelSimulationContext,
    /// Lowercase `arche`, used by the original position resolver's fallback.
    pub arche: ScriptObjectId,
    /// Capitalized `Arche`, excluded from destination candidates.
    pub excluded_destination: ScriptObjectId,
    /// `Honk`, excluded from the recursively collected actor candidates.
    pub honk: ScriptObjectId,
}

/// Shared range override temporarily owned by D5 (native GS:0x6B72).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequelSettlementState {
    /// Whether location searches use the maximum range instead of actor relief.
    pub range_override_active: bool,
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
    /// A parent graph or live coordinate field cannot be resolved.
    Navigation(ScriptNavigationError),
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

/// Apply D5 (0x7367-0x7407), including its native location/traversal helpers.
///
/// The source actor remains in place. Matching descendant actors move in the
/// original depth-first order; only the first receives the participation flag.
/// The destination's extra sequel word refers to the source, not that first
/// moved actor. Query mode still writes, and only the host advances the clock.
pub fn apply_sequel_settlement(
    operation: ScriptSequelSettlementOperation,
    context: SequelSettlementContext,
    simulation: &mut SequelSettlementState,
    state: &mut ScriptState,
) -> Result<ScriptControl, SequelGrowthError> {
    if state.dialect() != ScriptDialect::BigBugBang {
        return Err(SequelGrowthError::WrongDialect);
    }
    if context.simulation.countdown != 0 {
        return Ok(ScriptControl::Continue);
    }
    let selected = select_actors(
        state,
        operation.group_mask,
        context.simulation.excluded_location,
    )?;
    simulation.range_override_active = true;
    for actor in selected {
        if (read(state, actor, QUANTITY_FIELD)? as i16) < SETTLEMENT_MINIMUM_SOURCE_QUANTITY {
            continue;
        }
        let origin = relationship(state, actor, LOCATION_FIELD)?;
        if read(state, origin, KIND_FIELD)? & ScriptObjectKind::Location.mask() == 0 {
            continue;
        }
        let Some(destination) = closest_unoccupied_location(state, actor, context)? else {
            continue;
        };
        // 0x8103 is the same depth-first collection/filter algorithm as the
        // Commander helper. Collect before changing any live parent links.
        let candidates = navigation_candidates(state, origin, context.honk)
            .map_err(SequelGrowthError::Navigation)?;
        let relief = read(state, actor, PRESSURE_RELIEF_FIELD)?;
        let mut first = true;
        for candidate in candidates {
            if candidate == actor
                || read(state, candidate, GROUP_FIELD)? & operation.group_mask == 0
            {
                continue;
            }
            write_relationship(state, candidate, LOCATION_FIELD, destination)?;
            write(state, candidate, PRESSURE_RELIEF_FIELD, relief)?;
            write(
                state,
                candidate,
                QUANTITY_FIELD,
                SETTLEMENT_INITIAL_QUANTITY,
            )?;
            write(state, candidate, GROWTH_BALANCE_FIELD, SCALE as u16)?;
            if first {
                let flags = read(state, candidate, FLAGS_FIELD)? | PARTICIPATING_FLAG;
                write(state, candidate, FLAGS_FIELD, flags)?;
                let flags = read(state, destination, FLAGS_FIELD)? | PARTICIPATING_FLAG;
                write(state, destination, FLAGS_FIELD, flags)?;
                write_relationship(state, destination, LOCATION_SOURCE_ACTOR_FIELD, actor)?;
                first = false;
            }
        }
    }
    simulation.range_override_active = false;
    Ok(ScriptControl::Continue)
}

fn closest_unoccupied_location(
    state: &ScriptState,
    actor: ScriptObjectId,
    context: SequelSettlementContext,
) -> Result<Option<ScriptObjectId>, SequelGrowthError> {
    // The range-square MUL clears DX before the native position helper, so its
    // contextual black-hole comparison is zero on this path.
    let position_field = resolve_navigation_position(state, actor, context.arche, 0)
        .map_err(SequelGrowthError::Navigation)?;
    let position = state
        .word_pair(position_field)
        .ok_or(SequelGrowthError::InvalidObject { object: actor })?;
    let mut candidates = Vec::new();
    for object in state.objects() {
        if read(state, object.id, KIND_FIELD)? & ScriptObjectKind::Location.mask() == 0
            || read(state, object.id, FLAGS_FIELD)? & IN_PLAY_FLAG == 0
            || object.id == context.excluded_destination
        {
            continue;
        }
        let body = relationship(state, object.id, LOCATION_PARENT_FIELD)?;
        let field = state
            .object_word_pair(body, BODY_POSITION_FIELD / WORD_BYTES)
            .ok_or(SequelGrowthError::InvalidObject { object: body })?;
        let coordinates = state
            .word_pair(field)
            .ok_or(SequelGrowthError::InvalidObject { object: body })?;
        let square_delta = |left: u16, right: u16| {
            let delta = left.wrapping_sub(right) as i16;
            let magnitude = delta.wrapping_abs() as u16;
            u32::from(magnitude) * u32::from(magnitude)
        };
        let distance = square_delta(coordinates[0], position[0])
            .wrapping_add(square_delta(coordinates[1], position[1])) as i32;
        // Native JG/JGE compare the summed square as signed, including overflow.
        if distance <= (SETTLEMENT_RANGE * SETTLEMENT_RANGE) as i32 {
            candidates.push((object.id, distance));
        }
    }
    let mut best = None;
    let mut best_distance = CLOSEST_DISTANCE_INITIAL;
    for (location, distance) in candidates {
        if read(state, location, FLAGS_FIELD)? & PARTICIPATING_FLAG == 0 && distance < best_distance
        {
            best = Some(location);
            best_distance = distance;
        }
    }
    Ok(best)
}

fn relationship(
    state: &ScriptState,
    object: ScriptObjectId,
    offset: usize,
) -> Result<ScriptObjectId, SequelGrowthError> {
    match state.object_reference(field(state, object, offset)?) {
        Some(ScriptStateObjectReference::Object(target)) => Ok(target),
        _ => Err(SequelGrowthError::InvalidLocation { actor: object }),
    }
}

fn write_relationship(
    state: &mut ScriptState,
    object: ScriptObjectId,
    offset: usize,
    target: ScriptObjectId,
) -> Result<(), SequelGrowthError> {
    let encoded = state
        .object(target)
        .and_then(|record| u16::try_from(record.source_offset()).ok())
        .ok_or(SequelGrowthError::InvalidObject { object: target })?;
    write(state, object, offset, encoded)
}

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

    #[derive(Deserialize)]
    struct SettlementOracle {
        name: String,
        token: Vec<u8>,
        countdown: u16,
        query_mode: u8,
        range_override_before: u8,
        range_override_after: u8,
        directory: Vec<u8>,
        state_before: Vec<u8>,
        state_after: Vec<u8>,
        native_handlers_called: Vec<usize>,
    }

    #[test]
    fn sequel_settlement_matches_complete_native_handler_and_graph_helpers() {
        const BEGIN_QUERY: [u8; 3] = [0xA0, 0, 0];
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let mut count = 0;
        let mut calls = std::collections::BTreeSet::new();
        for line in
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_settlement.jsonl")
                .lines()
        {
            let vector: SettlementOracle = serde_json::from_str(line).unwrap();
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
            let DecodedScriptInstruction::SequelSettlement(operation) =
                decode_complete_script_instruction(&token, &state, &directory, &dictionary)
                    .unwrap()
            else {
                panic!("{}: wrong instruction dispatch", vector.name);
            };
            let context = SequelSettlementContext {
                simulation: SequelSimulationContext {
                    countdown: vector.countdown,
                    excluded_location: directory.find_active_object(b"Trashlando").unwrap(),
                },
                arche: directory.find_active_object(b"arche").unwrap(),
                excluded_destination: directory.find_active_object(b"Arche").unwrap(),
                honk: directory.find_active_object(b"Honk").unwrap(),
            };
            let mut simulation = SequelSettlementState {
                range_override_active: vector.range_override_before != 0,
            };
            assert_eq!(
                apply_sequel_settlement(operation, context, &mut simulation, &mut state),
                Ok(ScriptControl::Continue),
                "{}",
                vector.name
            );
            assert_eq!(
                u8::from(simulation.range_override_active),
                vector.range_override_after,
                "{}",
                vector.name
            );
            assert_eq!(state.encode(), vector.state_after, "{}", vector.name);
            calls.extend(vector.native_handlers_called);
            count += 1;
        }
        assert_eq!(count, 100);
        assert_eq!(
            calls,
            [
                0x7367, 0x706E, 0x6F17, 0x6F52, 0x67B8, 0x6633, 0x8103, 0x685D
            ]
            .into_iter()
            .collect()
        );
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
