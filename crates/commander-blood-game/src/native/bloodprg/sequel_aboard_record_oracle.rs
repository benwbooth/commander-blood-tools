//! BBB executable comparison for the inherited C2 aboard-record handler.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptRecordValue, decode_script_aboard_record_operation,
};
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, decode_script_directory,
    decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x9000;
const BRANCH_TARGET: usize = 0x5AA5;
const OWNER_INDEX: usize = 0;
const RELATED_INDEX: usize = 1;
const ALTERNATE_INDEX: usize = 2;
const OBJECT_COUNT: usize = 19;
const INITIAL_HOLDER: u16 = 0x1234;
const INITIAL_C2_GATE: u8 = 0xA5;
const INITIAL_ACTIVE_LINE: u16 = 0x1357;

#[derive(Debug, Deserialize)]
struct Vector {
    query_before: u8,
    query_after: u8,
    inverted: bool,
    owner_flags: u16,
    related_kind: u16,
    related_flags: u16,
    target_offset: u16,
    related_offset: u16,
    alternate_offset: u16,
    object_offsets: [u16; OBJECT_COUNT],
    target_before: [u16; 3],
    roster_before: [u16; ABOARD_OBJECT_CAPACITY],
    roster_after: [u16; ABOARD_OBJECT_CAPACITY],
    ui_state: u8,
    request_before: u8,
    request_after: u8,
    descriptor_available: bool,
    descriptor_called: bool,
    descriptor_result: u8,
    c2_gate_after: u8,
    active_line_after: u16,
    holder_after: u16,
    owner_lookup_called: bool,
    slot_insert_called: bool,
    slot_insert_succeeded: bool,
    field_lookup_called: bool,
    failed: bool,
    guard_depth_after: usize,
    cursor: usize,
}

struct Fixture {
    directory: ScriptDirectory,
    state: ScriptState,
    ids: [ScriptObjectId; OBJECT_COUNT],
    offsets: [u16; OBJECT_COUNT],
}

fn fixture(vector: &Vector) -> Fixture {
    let related_kind = ScriptObjectKind::decode(vector.related_kind).unwrap();
    let mut kinds = vec![
        ScriptObjectKind::Actor,
        related_kind,
        ScriptObjectKind::Location,
    ];
    kinds.extend(std::iter::repeat_n(
        ScriptObjectKind::Location,
        ABOARD_OBJECT_CAPACITY,
    ));
    assert_eq!(kinds.len(), OBJECT_COUNT);

    let mut offsets = [u16::MIN; OBJECT_COUNT];
    let mut cursor = usize::MIN;
    for (index, kind) in kinds.iter().copied().enumerate() {
        offsets[index] = u16::try_from(cursor).unwrap();
        cursor += kind.record_size_for_dialect(ScriptDialect::BigBugBang);
    }
    assert_eq!(offsets, vector.object_offsets);

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
            _ => 1,
        };
        state_bytes[offset + 2..offset + 4].copy_from_slice(&flags.to_le_bytes());
    }
    state_bytes[usize::from(vector.target_offset)..usize::from(vector.target_offset) + 6]
        .copy_from_slice(&vector.target_before.map(u16::to_le_bytes).concat());
    let holder_offset =
        script_field_offset(related_kind, ScriptFieldSelector::HOLDER_OR_LOCATION).unwrap();
    let holder = usize::from(vector.related_offset) + holder_offset;
    state_bytes[holder..holder + 2].copy_from_slice(&INITIAL_HOLDER.to_le_bytes());

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
        0xC2 => ScriptActionRecord::AboardRequest(object_at_offset(fixture, words[1])),
        0xC3 => ScriptActionRecord::PresentationQueue(object_at_offset(fixture, words[1])),
        _ => ScriptActionRecord::Occupied,
    }
}

fn roster_from_offsets(
    offsets: [u16; ABOARD_OBJECT_CAPACITY],
    fixture: &Fixture,
) -> AboardObjectRoster {
    AboardObjectRoster::from_test_slots(
        offsets.map(|offset| (offset != u16::MIN).then(|| object_at_offset(fixture, offset))),
    )
}

fn expected_line(number: u16) -> Option<ScriptAboardPresentationLine> {
    match number {
        39 => Some(ScriptAboardPresentationLine::ActorArrived),
        43 => Some(ScriptAboardPresentationLine::InventoryArrived),
        INITIAL_ACTIVE_LINE => None,
        unknown => panic!("unknown C2 oracle presentation line {unknown}"),
    }
}

#[test]
fn sequel_c2_uses_shared_aboard_record_state() {
    const VECTOR_COUNT: usize = 684;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_aboard_record.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        let fixture = fixture(&vector);
        let mut bytes = vec![0xC2];
        if vector.inverted {
            bytes.push(0xA1);
        }
        bytes.extend_from_slice(&vector.target_offset.to_le_bytes());
        bytes.extend_from_slice(&vector.related_offset.to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let token = &code.tokens()[0];
        let operation =
            decode_script_aboard_record_operation(token, &fixture.state, &fixture.directory)
                .unwrap();
        assert_eq!(operation.target.object(), Some(fixture.ids[OWNER_INDEX]));
        assert_eq!(operation.related, fixture.ids[RELATED_INDEX]);

        let mut records = ScriptActionRecords::default();
        records.set_record(
            operation.target,
            record_from_words(vector.target_before, &fixture),
        );
        let record_before = records.record(operation.target);
        let mut fields = ScriptRecordFields::default();
        let mut roster = roster_from_offsets(vector.roster_before, &fixture);
        let expected_roster = roster_from_offsets(vector.roster_after, &fixture);
        let mut request_flags = PresentationRequestFlags::decode(vector.request_before);
        let mut presentation = ScriptAboardPresentationState {
            presentation_gate_active: true,
            active_line: None,
        };
        let context = ScriptAboardRecordContext {
            ship_interface_active: vector.ui_state & 1 != 0,
            descriptor_available: vector.descriptor_available,
        };
        let mut runtime = ScriptRuntime::new();
        if vector.query_before & 1 != 0 {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }

        let outcome = apply_aboard_record_operation(
            operation,
            &fixture.state,
            &records,
            &mut fields,
            &mut roster,
            context,
            &mut request_flags,
            &mut presentation,
            &mut runtime,
        )
        .unwrap();

        let query_mode = vector.query_before & 1 != 0;
        assert!(vector.owner_lookup_called);
        assert_eq!(
            vector.slot_insert_called,
            !query_mode && vector.owner_flags & 1 != 0 && vector.related_flags & 0x20 != 0,
            "{vector:?}"
        );
        assert_eq!(vector.field_lookup_called, vector.slot_insert_succeeded);
        assert_eq!(outcome.holder_changed, vector.slot_insert_succeeded);
        assert_eq!(outcome.descriptor_checked, vector.descriptor_called);
        assert_eq!(
            vector.descriptor_result != 0,
            vector.descriptor_called && vector.descriptor_available
        );
        assert_eq!(roster, expected_roster, "{vector:?}");
        assert_eq!(records.record(operation.target), record_before);

        let related_kind = fixture.state.object(operation.related).unwrap().kind;
        let holder_offset =
            script_field_offset(related_kind, ScriptFieldSelector::HOLDER_OR_LOCATION).unwrap();
        let holder = fixture
            .state
            .object_word(
                operation.related,
                holder_offset / std::mem::size_of::<u16>(),
            )
            .unwrap();
        assert_eq!(fixture.state.word(holder), Some(INITIAL_HOLDER));
        assert_eq!(
            fields.value(holder),
            (vector.holder_after == u16::MAX).then_some(ScriptRecordValue::Aboard),
            "{vector:?}"
        );
        assert_eq!(request_flags.bits(), vector.request_after, "{vector:?}");
        assert_eq!(
            presentation.presentation_gate_active,
            vector.c2_gate_after == INITIAL_C2_GATE,
            "{vector:?}"
        );
        let line = expected_line(vector.active_line_after);
        assert_eq!(presentation.active_line, line, "{vector:?}");
        assert_eq!(outcome.presentation_requested, line.is_some(), "{vector:?}");

        assert_eq!(runtime.query_mode(), vector.query_after & 1 != 0);
        assert_eq!(runtime.guard_depth(), vector.guard_depth_after);
        assert_eq!(
            outcome.control,
            if vector.failed {
                ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
            } else {
                ScriptControl::Continue
            },
            "{vector:?}"
        );
        let cursor = match outcome.control {
            ScriptControl::Jump(target) => target.index(),
            ScriptControl::Continue => SCRIPT_CURSOR + token.encoded_bytes().len() - 1,
        };
        assert_eq!(cursor, vector.cursor, "{vector:?}");
    }
}
