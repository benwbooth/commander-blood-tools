//! BBB executable comparison for the inherited direct-record handler family.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptRecordValue, decode_script_direct_record_operation,
};
use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState,
    decode_script_dictionary, decode_script_directory, decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x9000;
const BRANCH_TARGET: usize = 0x5AA5;
const OBJECT_COUNT: usize = 22;
const SPECIAL_INDEX: usize = 2;
const OWNER_INDEX: usize = 3;
const ORDINARY_INDEX: usize = 4;
const ALTERNATE_INDEX: usize = 5;
const ABOARD_CAPACITY: usize = 16;

const KINDS: [ScriptObjectKind; OBJECT_COUNT] = [
    ScriptObjectKind::Auxiliary,
    ScriptObjectKind::Auxiliary,
    ScriptObjectKind::Player,
    ScriptObjectKind::Actor,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
];

#[derive(Debug, Deserialize)]
struct Vector {
    opcode: u8,
    query_before: u8,
    query_after: u8,
    inverted: bool,
    requested_kind: String,
    requested_value: u16,
    field_before_kind: String,
    field_before: u16,
    field_after_kind: String,
    field_after: u16,
    object_offsets: [u16; OBJECT_COUNT],
    target_offset: u16,
    special_offset: u16,
    ordinary_offset: u16,
    alternate_offset: u16,
    roster_before: [u16; ABOARD_CAPACITY],
    roster_after: [u16; ABOARD_CAPACITY],
    published_before: u16,
    published_after: u16,
    guard_depth_after: usize,
    failed: bool,
    write_succeeded: bool,
    cursor: usize,
}

struct Fixture {
    dictionary: ScriptDictionary,
    directory: ScriptDirectory,
    state: ScriptState,
    ids: [ScriptObjectId; OBJECT_COUNT],
    offsets: [u16; OBJECT_COUNT],
}

fn write_word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn fixture(vector: &Vector) -> Fixture {
    let mut offsets = [u16::MIN; OBJECT_COUNT];
    let mut cursor = usize::MIN;
    for (index, kind) in KINDS.iter().copied().enumerate() {
        offsets[index] = u16::try_from(cursor).unwrap();
        cursor += kind.record_size_for_dialect(ScriptDialect::BigBugBang);
    }
    assert_eq!(offsets, vector.object_offsets, "{vector:?}");
    assert_eq!(offsets[SPECIAL_INDEX], vector.special_offset, "{vector:?}");

    let mut directory_bytes = Vec::new();
    let mut state_bytes = vec![u8::MIN; cursor];
    for (index, kind) in KINDS.iter().copied().enumerate() {
        let mut entry = [u8::MIN; 20];
        let name = format!("object{index}");
        entry[..name.len()].copy_from_slice(name.as_bytes());
        entry[16..18].copy_from_slice(&offsets[index].to_le_bytes());
        entry[18..20].copy_from_slice(&1u16.to_le_bytes());
        directory_bytes.extend_from_slice(&entry);

        let offset = usize::from(offsets[index]);
        write_word(&mut state_bytes, offset, kind.mask());
        write_word(&mut state_bytes, offset + 2, 1);
    }
    write_word(
        &mut state_bytes,
        usize::from(vector.target_offset),
        vector.field_before,
    );
    let mut sentinel = [u8::MIN; 20];
    sentinel[16..18].copy_from_slice(&u16::try_from(cursor).unwrap().to_le_bytes());
    directory_bytes.extend_from_slice(&sentinel);

    let dictionary = decode_script_dictionary(b"talk\0secret\0").unwrap();
    let directory = decode_script_directory(&directory_bytes).unwrap();
    let state =
        decode_script_state_for_dialect(&state_bytes, &directory, ScriptDialect::BigBugBang)
            .unwrap();
    let ids: Vec<_> = state.objects().iter().map(|object| object.id).collect();
    Fixture {
        dictionary,
        directory,
        state,
        ids: ids.try_into().unwrap(),
        offsets,
    }
}

fn object_at_offset(fixture: &Fixture, offset: u16) -> ScriptObjectId {
    let index = fixture
        .offsets
        .iter()
        .position(|candidate| *candidate == offset)
        .unwrap();
    fixture.ids[index]
}

fn typed_value(kind: &str, raw: u16, fixture: &Fixture) -> ScriptRecordValue {
    match kind {
        "special" => ScriptRecordValue::Object(fixture.ids[SPECIAL_INDEX]),
        "object" => ScriptRecordValue::Object(object_at_offset(fixture, raw)),
        "aboard" => ScriptRecordValue::Aboard,
        "native" => ScriptRecordValue::NativeWord(raw),
        "topic" => ScriptRecordValue::Topic(fixture.dictionary.resolve_source_offset(raw).unwrap()),
        unknown => panic!("unknown direct-record value kind {unknown}"),
    }
}

fn roster_from_offsets(offsets: [u16; ABOARD_CAPACITY], fixture: &Fixture) -> AboardObjectRoster {
    AboardObjectRoster::from_test_slots(
        offsets.map(|offset| (offset != u16::MIN).then(|| object_at_offset(fixture, offset))),
    )
}

#[test]
fn sequel_direct_record_aliases_use_shared_typed_state() {
    const VECTOR_COUNT: usize = 912;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_direct_record.jsonl")
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
        bytes.extend_from_slice(&vector.requested_value.to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let token = &code.tokens()[0];
        let operation = decode_script_direct_record_operation(
            token,
            &fixture.state,
            &fixture.directory,
            &fixture.dictionary,
        )
        .unwrap();
        let target = fixture
            .state
            .resolve_word_source_offset(vector.target_offset)
            .unwrap();
        let requested = typed_value(&vector.requested_kind, vector.requested_value, &fixture);
        assert_eq!(operation.target, target, "{vector:?}");
        assert_eq!(operation.value, requested, "{vector:?}");
        assert_eq!(operation.inverted, vector.inverted, "{vector:?}");
        assert_eq!(
            operation.publishes_value,
            vector.opcode == 0xBC,
            "{vector:?}"
        );

        let mut fields = ScriptRecordFields::default();
        fields.set_value(
            target,
            typed_value(&vector.field_before_kind, vector.field_before, &fixture),
        );
        let mut record_runtime = ScriptRecordRuntime::new(fixture.ids[SPECIAL_INDEX]);
        *record_runtime.aboard_objects_mut() = roster_from_offsets(vector.roster_before, &fixture);
        let expected_roster = roster_from_offsets(vector.roster_after, &fixture);
        let mut script_runtime = ScriptRuntime::new();
        if vector.query_before & 1 != 0 {
            script_runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            script_runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }

        let control = apply_direct_record_operation(
            operation,
            &mut fields,
            &mut record_runtime,
            &mut script_runtime,
        )
        .unwrap();

        assert_eq!(
            fields.value(target),
            Some(typed_value(
                &vector.field_after_kind,
                vector.field_after,
                &fixture,
            )),
            "{vector:?}"
        );
        assert_eq!(
            record_runtime.aboard_objects(),
            &expected_roster,
            "{vector:?}"
        );
        let published =
            (vector.opcode == 0xBC && vector.query_before & 1 == 0).then_some(requested);
        assert_eq!(record_runtime.published_value(), published, "{vector:?}");
        assert_eq!(
            vector.published_after,
            published.map_or(vector.published_before, |_| vector.requested_value),
            "{vector:?}"
        );
        assert_eq!(
            script_runtime.query_mode(),
            vector.query_after & 1 != 0,
            "{vector:?}"
        );
        assert_eq!(
            script_runtime.guard_depth(),
            vector.guard_depth_after,
            "{vector:?}"
        );
        assert_eq!(
            control,
            if vector.failed {
                ScriptControl::Jump(ScriptCodeOffset::new(BRANCH_TARGET))
            } else {
                ScriptControl::Continue
            },
            "{vector:?}"
        );
        let cursor = match control {
            ScriptControl::Jump(target) => target.index(),
            ScriptControl::Continue => SCRIPT_CURSOR + token.encoded_bytes().len(),
        };
        assert_eq!(cursor, vector.cursor, "{vector:?}");
        if !vector.write_succeeded && vector.query_before & 1 == 0 {
            assert_eq!(vector.field_after, vector.field_before, "{vector:?}");
        }

        assert_eq!(
            fixture.ids[OWNER_INDEX],
            target.object().unwrap(),
            "{vector:?}"
        );
        assert_eq!(
            fixture.ids[ORDINARY_INDEX],
            object_at_offset(&fixture, vector.ordinary_offset)
        );
        assert_eq!(
            fixture.ids[ALTERNATE_INDEX],
            object_at_offset(&fixture, vector.alternate_offset)
        );
    }
}
