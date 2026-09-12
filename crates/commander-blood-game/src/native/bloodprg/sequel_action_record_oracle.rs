//! BBB executable comparison for the inherited C3-C8 action-record handlers.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    decode_script_active_object_record_operation, decode_script_actor_record_operation,
    decode_script_opaque_marker_record_operation, decode_script_presentation_queue_operation,
    decode_script_travel_record_operation, decode_script_world_state_record_operation,
};
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateWordTriple,
    decode_script_directory, decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x40;
const BRANCH_TARGET: usize = 0x5AA5;
const OWNER_INDEX: usize = 0;
const RELATED_INDEX: usize = 1;
const ALTERNATE_INDEX: usize = 2;

#[derive(Debug, Deserialize)]
struct Vector {
    opcode: u8,
    query_before: u8,
    query_after: u8,
    inverted: bool,
    owner_kind: u16,
    owner_flags: u16,
    related_kind: u16,
    related_flags: u16,
    target_offset: u16,
    related_offset: u16,
    alternate_offset: u16,
    reciprocal_offset: Option<u16>,
    target_before: [u16; 3],
    target_after: [u16; 3],
    reciprocal_before: [u16; 3],
    reciprocal_after: Option<[u16; 3]>,
    owner_lookup_called: bool,
    field_lookup_called: bool,
    failed: bool,
    guard_depth_after: usize,
    cursor: usize,
}

struct Fixture {
    directory: ScriptDirectory,
    state: ScriptState,
    ids: [ScriptObjectId; 3],
    offsets: [u16; 3],
}

fn fixture(vector: &Vector) -> Fixture {
    let owner_kind = ScriptObjectKind::decode(vector.owner_kind).unwrap();
    let related_kind = ScriptObjectKind::decode(vector.related_kind).unwrap();
    let kinds = [owner_kind, related_kind, ScriptObjectKind::Location];
    let mut offsets = [u16::MIN; 3];
    let mut cursor = usize::MIN;
    for (index, kind) in kinds.iter().copied().enumerate() {
        offsets[index] = u16::try_from(cursor).unwrap();
        cursor += kind.record_size_for_dialect(ScriptDialect::BigBugBang);
    }

    let mut directory_bytes = Vec::new();
    let mut state_bytes = vec![u8::MIN; cursor];
    for (index, kind) in kinds.iter().copied().enumerate() {
        let mut entry = [u8::MIN; 20];
        let name = format!("object{index}");
        entry[..name.len()].copy_from_slice(name.as_bytes());
        entry[16..18].copy_from_slice(&offsets[index].to_le_bytes());
        entry[18..20].copy_from_slice(&1u16.to_le_bytes());
        directory_bytes.extend_from_slice(&entry);

        let offset = usize::from(offsets[index]);
        state_bytes[offset..offset + 2].copy_from_slice(&kind.mask().to_le_bytes());
        let flags = match index {
            OWNER_INDEX => vector.owner_flags,
            RELATED_INDEX => vector.related_flags,
            ALTERNATE_INDEX => 1,
            _ => unreachable!(),
        };
        state_bytes[offset + 2..offset + 4].copy_from_slice(&flags.to_le_bytes());
    }
    let mut sentinel = [u8::MIN; 20];
    sentinel[16..18].copy_from_slice(&u16::try_from(cursor).unwrap().to_le_bytes());
    directory_bytes.extend_from_slice(&sentinel);

    let directory = decode_script_directory(&directory_bytes).unwrap();
    let state =
        decode_script_state_for_dialect(&state_bytes, &directory, ScriptDialect::BigBugBang)
            .unwrap();
    let ids: Vec<_> = state.objects().iter().map(|object| object.id).collect();
    assert_eq!(offsets[OWNER_INDEX], 0);
    assert_eq!(offsets[RELATED_INDEX], vector.related_offset);
    assert_eq!(offsets[ALTERNATE_INDEX], vector.alternate_offset);
    Fixture {
        directory,
        state,
        ids: ids.try_into().unwrap(),
        offsets,
    }
}

fn object_at_offset(fixture: &Fixture, offset: u16) -> ScriptObjectId {
    fixture
        .offsets
        .iter()
        .position(|candidate| *candidate == offset)
        .map(|index| fixture.ids[index])
        .unwrap()
}

fn record_from_words(words: [u16; 3], fixture: &Fixture) -> ScriptActionRecord {
    match words[0] {
        0 => ScriptActionRecord::Empty,
        0xC3 => ScriptActionRecord::PresentationQueue(object_at_offset(fixture, words[1])),
        0xC4 => ScriptActionRecord::ActorPresentation(object_at_offset(fixture, words[1])),
        0xC5 => ScriptActionRecord::WorldStateLink(object_at_offset(fixture, words[1])),
        0xC6 => ScriptActionRecord::Travel(object_at_offset(fixture, words[1])),
        0xC7 => ScriptActionRecord::ActiveObjectLink(object_at_offset(fixture, words[1])),
        0xC8 => ScriptActionRecord::OpaqueMarker(words[1]),
        _ => ScriptActionRecord::Occupied,
    }
}

fn configure_records(
    vector: &Vector,
    fixture: &Fixture,
    target: ScriptStateWordTriple,
) -> ScriptActionRecords {
    let mut records = ScriptActionRecords::default();
    records.set_record(target, record_from_words(vector.target_before, fixture));
    if let Some(offset) = vector.reciprocal_offset {
        let related = fixture.ids[RELATED_INDEX];
        let relative = usize::from(offset - vector.related_offset) / std::mem::size_of::<u16>();
        let reciprocal = fixture.state.object_word_triple(related, relative).unwrap();
        records.set_record(
            reciprocal,
            record_from_words(vector.reciprocal_before, fixture),
        );
    }
    records
}

fn assert_common(
    vector: &Vector,
    token_size: usize,
    target: ScriptStateWordTriple,
    fixture: &Fixture,
    records: &ScriptActionRecords,
    runtime: &ScriptRuntime,
    outcome: ScriptRecordStateOutcome,
) {
    assert_eq!(
        records.record(target),
        record_from_words(vector.target_after, fixture),
        "{vector:?}"
    );
    if let (Some(offset), Some(expected)) = (vector.reciprocal_offset, vector.reciprocal_after) {
        let related = fixture.ids[RELATED_INDEX];
        let relative = usize::from(offset - vector.related_offset) / std::mem::size_of::<u16>();
        let reciprocal = fixture.state.object_word_triple(related, relative).unwrap();
        assert_eq!(
            records.record(reciprocal),
            record_from_words(expected, fixture),
            "{vector:?}"
        );
    }
    assert_eq!(
        runtime.query_mode(),
        vector.query_after & 1 != 0,
        "{vector:?}"
    );
    assert_eq!(
        runtime.guard_depth(),
        vector.guard_depth_after,
        "{vector:?}"
    );
    assert_eq!(
        outcome.control,
        if vector.failed {
            ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
        } else {
            ScriptControl::Continue
        },
        "{vector:?}"
    );
    assert_eq!(
        outcome.written_slot,
        (vector.query_before & 1 == 0 && !vector.failed).then_some(target),
        "{vector:?}"
    );
    let cursor = match outcome.control {
        ScriptControl::Jump(target) => target.index(),
        ScriptControl::Continue => SCRIPT_CURSOR + token_size - 1,
    };
    assert_eq!(cursor, vector.cursor, "{vector:?}");

    assert_eq!(
        vector.owner_lookup_called,
        matches!(vector.opcode, 0xC3 | 0xC4)
    );
    let field_lookup_expected = vector.opcode == 0xC4
        && vector.query_before & 1 == 0
        && vector.owner_flags & 1 != 0
        && vector.related_flags & 1 != 0
        && vector.owner_kind != 1
        && vector.related_kind != 1
        && vector.target_before[0] != 0xC4;
    assert_eq!(
        vector.field_lookup_called, field_lookup_expected,
        "{vector:?}"
    );
}

#[test]
fn sequel_c3_c8_use_shared_typed_action_record_state() {
    const VECTOR_COUNT: usize = 996;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_action_record.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        let fixture = fixture(&vector);
        let mut bytes = vec![vector.opcode];
        if vector.inverted {
            bytes.push(0xA1);
        }
        bytes.extend_from_slice(&vector.target_offset.to_le_bytes());
        bytes.extend_from_slice(&vector.related_offset.to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let token = &code.tokens()[0];
        let mut runtime = ScriptRuntime::new();
        if vector.query_before & 1 != 0 {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }

        let target = fixture
            .state
            .resolve_word_triple_source_offset(vector.target_offset)
            .unwrap();
        let mut records = configure_records(&vector, &fixture, target);
        let outcome = match vector.opcode {
            0xC3 => {
                let operation = decode_script_presentation_queue_operation(
                    token,
                    &fixture.state,
                    &fixture.directory,
                )
                .unwrap();
                assert_eq!(operation.related, fixture.ids[RELATED_INDEX]);
                apply_presentation_queue_operation(
                    operation,
                    &fixture.state,
                    &mut records,
                    &mut runtime,
                )
                .unwrap()
            }
            0xC4 => {
                let operation =
                    decode_script_actor_record_operation(token, &fixture.state, &fixture.directory)
                        .unwrap();
                assert_eq!(operation.related, fixture.ids[RELATED_INDEX]);
                apply_actor_record_operation(operation, &fixture.state, &mut records, &mut runtime)
                    .unwrap()
            }
            0xC5 => {
                let operation = decode_script_world_state_record_operation(
                    token,
                    &fixture.state,
                    &fixture.directory,
                )
                .unwrap();
                assert_eq!(operation.related, fixture.ids[RELATED_INDEX]);
                apply_world_state_record_operation(
                    operation,
                    &fixture.state,
                    &mut records,
                    &mut runtime,
                )
                .unwrap()
            }
            0xC6 => {
                let operation = decode_script_travel_record_operation(
                    token,
                    &fixture.state,
                    &fixture.directory,
                )
                .unwrap();
                assert_eq!(operation.destination, fixture.ids[RELATED_INDEX]);
                apply_travel_record_operation(operation, &mut records, &mut runtime).unwrap()
            }
            0xC7 => {
                let operation = decode_script_active_object_record_operation(
                    token,
                    &fixture.state,
                    &fixture.directory,
                )
                .unwrap();
                assert_eq!(operation.related, fixture.ids[RELATED_INDEX]);
                apply_active_object_record_operation(
                    operation,
                    &fixture.state,
                    &mut records,
                    &mut runtime,
                )
                .unwrap()
            }
            0xC8 => {
                let operation =
                    decode_script_opaque_marker_record_operation(token, &fixture.state).unwrap();
                assert_eq!(operation.comparison_word, vector.related_offset);
                apply_opaque_marker_record_operation(operation, &mut records, &mut runtime).unwrap()
            }
            _ => unreachable!(),
        };
        assert_common(
            &vector,
            token.encoded_bytes().len(),
            target,
            &fixture,
            &records,
            &runtime,
            outcome,
        );
    }
}
