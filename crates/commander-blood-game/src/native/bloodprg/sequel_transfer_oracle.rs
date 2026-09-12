//! BBB executable comparison for the inherited CD transfer handler.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{ScriptRecordValue, decode_script_transfer};
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, decode_script_directory,
    decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::*;

const OBJECT_COUNT: usize = 21;
const ROSTER_CAPACITY: usize = 16;
const SCRIPT_START: usize = 0x9000;
const BRANCH_TARGETS: [usize; 2] = [0x5AA5, 0x6BB6];

#[derive(Debug, Deserialize)]
struct Calls {
    owner: usize,
    field: usize,
    remove: usize,
    insert: usize,
    failure: usize,
    descriptor: usize,
}

#[derive(Debug, Deserialize)]
struct Vector {
    query_before: u8,
    query_after: u8,
    inverted: bool,
    source: String,
    destination: String,
    item_kind: u16,
    item_active: bool,
    destination_active: bool,
    stored_kind: u16,
    stored_item: u16,
    stored_destination: u16,
    ui_state: u8,
    request_before: u8,
    request_after: u8,
    descriptor_available: bool,
    descriptor_checked: bool,
    presentation_requested: bool,
    guard_depth_before: usize,
    guard_depth_after: usize,
    failed: bool,
    holder_changed: bool,
    source_record: u16,
    item_offset: u16,
    destination_offset: u16,
    alternate_offset: u16,
    object_offsets: [u16; OBJECT_COUNT],
    roster_before: [u16; ROSTER_CAPACITY],
    roster_after: [u16; ROSTER_CAPACITY],
    holder_before: u16,
    holder_after: u16,
    c2_gate_after: u8,
    active_line_after: u16,
    cursor: usize,
    calls: Calls,
}

struct Fixture {
    directory: ScriptDirectory,
    state: ScriptState,
    ids: [ScriptObjectId; OBJECT_COUNT],
    offsets: [u16; OBJECT_COUNT],
}

fn item_kind(raw: u16) -> ScriptObjectKind {
    match raw {
        0x0002 => ScriptObjectKind::Actor,
        0x0080 => ScriptObjectKind::Location,
        0x0400 => ScriptObjectKind::InventoryItem,
        _ => panic!("unexpected transfer item kind {raw:#x}"),
    }
}

fn fixture(vector: &Vector) -> Fixture {
    let kinds = [
        ScriptObjectKind::Player,
        ScriptObjectKind::Actor,
        item_kind(vector.item_kind),
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
    let mut offsets = [u16::MIN; OBJECT_COUNT];
    let mut cursor = usize::MIN;
    for (index, kind) in kinds.iter().copied().enumerate() {
        offsets[index] = u16::try_from(cursor).unwrap();
        cursor += kind.record_size_for_dialect(ScriptDialect::BigBugBang);
    }
    assert_eq!(offsets, vector.object_offsets, "{vector:?}");

    let mut directory_bytes = Vec::new();
    let mut state_bytes = vec![u8::MIN; cursor];
    for (index, kind) in kinds.iter().copied().enumerate() {
        let mut entry = [u8::MIN; 20];
        let name = if index == 2 {
            "item".to_owned()
        } else {
            format!("object{index}")
        };
        entry[..name.len()].copy_from_slice(name.as_bytes());
        entry[16..18].copy_from_slice(&offsets[index].to_le_bytes());
        entry[18..20].copy_from_slice(&1u16.to_le_bytes());
        directory_bytes.extend_from_slice(&entry);

        let offset = usize::from(offsets[index]);
        state_bytes[offset..offset + 2].copy_from_slice(&kind.mask().to_le_bytes());
        let active = if index == 2 {
            vector.item_active
        } else if index == 0 && vector.destination == "special" || index == 3 {
            vector.destination_active
        } else {
            true
        };
        state_bytes[offset + 2..offset + 4].copy_from_slice(&u16::from(active).to_le_bytes());
    }
    state_bytes[usize::from(vector.item_offset) + 4..usize::from(vector.item_offset) + 9]
        .copy_from_slice(b"item\0");
    let holder_offset = match item_kind(vector.item_kind) {
        ScriptObjectKind::Actor => 24,
        ScriptObjectKind::InventoryItem | ScriptObjectKind::Location => 20,
        _ => unreachable!(),
    };
    let holder = usize::from(vector.item_offset) + holder_offset;
    state_bytes[holder..holder + 2].copy_from_slice(&vector.holder_before.to_le_bytes());
    let source = usize::from(vector.source_record);
    for (index, value) in [
        vector.stored_kind,
        vector.stored_item,
        vector.stored_destination,
    ]
    .into_iter()
    .enumerate()
    {
        state_bytes[source + index * 2..source + index * 2 + 2]
            .copy_from_slice(&value.to_le_bytes());
    }
    let mut sentinel = [u8::MIN; 20];
    sentinel[16..18].copy_from_slice(&u16::try_from(cursor).unwrap().to_le_bytes());
    directory_bytes.extend_from_slice(&sentinel);

    let directory = decode_script_directory(&directory_bytes).unwrap();
    let state =
        decode_script_state_for_dialect(&state_bytes, &directory, ScriptDialect::BigBugBang)
            .unwrap();
    let ids: Vec<_> = state.objects().iter().map(|object| object.id).collect();
    Fixture {
        directory,
        state,
        ids: ids.try_into().unwrap(),
        offsets,
    }
}

fn object_at_offset(fixture: &Fixture, offset: u16) -> ScriptObjectId {
    fixture.ids[fixture
        .offsets
        .iter()
        .position(|candidate| *candidate == offset)
        .unwrap()]
}

fn roster(offsets: [u16; ROSTER_CAPACITY], fixture: &Fixture) -> AboardObjectRoster {
    AboardObjectRoster::from_test_slots(
        offsets.map(|offset| (offset != u16::MIN).then(|| object_at_offset(fixture, offset))),
    )
}

#[test]
fn sequel_cd_uses_shared_typed_transfer_state() {
    const VECTOR_COUNT: usize = 600;
    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_transfer.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        let fixture = fixture(&vector);
        let mut token_bytes = vec![0xCD];
        if vector.inverted {
            token_bytes.push(0xA1);
        }
        token_bytes.extend_from_slice(&vector.source_record.to_le_bytes());
        token_bytes.extend_from_slice(&vector.item_offset.to_le_bytes());
        token_bytes.extend_from_slice(&vector.destination_offset.to_le_bytes());
        token_bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&token_bytes, ScriptDialect::BigBugBang).unwrap();
        let token = &code.tokens()[0];
        let transfer = decode_script_transfer(token, &fixture.state, &fixture.directory).unwrap();
        let source_record = fixture
            .state
            .resolve_word_source_offset(vector.source_record)
            .unwrap();
        let item = object_at_offset(&fixture, vector.item_offset);
        let destination = object_at_offset(&fixture, vector.destination_offset);
        assert_eq!(transfer.source_record, source_record, "{vector:?}");
        assert_eq!(transfer.item, item, "{vector:?}");
        assert_eq!(transfer.destination, destination, "{vector:?}");
        assert_eq!(transfer.inverted, vector.inverted, "{vector:?}");

        let mut transfer_records = ScriptTransferRecords::default();
        if vector.stored_kind == 0xCD {
            transfer_records.set_record(
                source_record,
                ScriptTransferRecord {
                    item: object_at_offset(&fixture, vector.stored_item),
                    destination: object_at_offset(&fixture, vector.stored_destination),
                },
            );
        }
        let holder_offset = match item_kind(vector.item_kind) {
            ScriptObjectKind::Actor => 24,
            ScriptObjectKind::InventoryItem | ScriptObjectKind::Location => 20,
            _ => unreachable!(),
        };
        let holder = fixture
            .state
            .object_word(item, holder_offset / size_of::<u16>())
            .unwrap();
        let mut fields = ScriptRecordFields::default();
        fields.set_value(holder, ScriptRecordValue::NativeWord(vector.holder_before));
        let mut record_runtime = ScriptRecordRuntime::new(fixture.ids[0]);
        *record_runtime.aboard_objects_mut() = roster(vector.roster_before, &fixture);
        let expected_roster = roster(vector.roster_after, &fixture);
        let mut request_flags = PresentationRequestFlags::decode(vector.request_before);
        let mut presentation = ScriptTransferPresentationState {
            presentation_gate_active: true,
            ..ScriptTransferPresentationState::default()
        };
        let context = ScriptTransferContext {
            ship_interface_active: vector.ui_state & 1 != 0,
            descriptor_available: vector.descriptor_available,
        };
        let mut runtime = ScriptRuntime::new();
        let failure_target = ScriptCodeOffset::new(BRANCH_TARGETS[vector.guard_depth_before - 1]);
        if vector.query_before & 1 != 0 {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGETS[0]));
            if vector.guard_depth_before == 2 {
                runtime.begin_guard(failure_target);
            }
        } else {
            runtime.arm_root_failure_target(failure_target);
        }

        let outcome = apply_transfer(
            transfer,
            &fixture.state,
            &transfer_records,
            &mut fields,
            &mut record_runtime,
            context,
            &mut request_flags,
            &mut presentation,
            &mut runtime,
        )
        .unwrap();

        let expected_control = if vector.failed {
            ScriptControl::Jump(failure_target)
        } else {
            ScriptControl::Continue
        };
        assert_eq!(outcome.control, expected_control, "{vector:?}");
        assert_eq!(outcome.holder_changed, vector.holder_changed, "{vector:?}");
        assert_eq!(
            outcome.descriptor_checked, vector.descriptor_checked,
            "{vector:?}"
        );
        assert_eq!(
            outcome.presentation_requested, vector.presentation_requested,
            "{vector:?}"
        );
        assert_eq!(
            fields.value(holder),
            Some(if vector.holder_after == u16::MAX {
                ScriptRecordValue::Aboard
            } else if vector.holder_after == vector.destination_offset {
                ScriptRecordValue::Object(destination)
            } else {
                ScriptRecordValue::NativeWord(vector.holder_after)
            }),
            "{vector:?}"
        );
        assert_eq!(
            record_runtime.aboard_objects(),
            &expected_roster,
            "{vector:?}"
        );
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
        assert_eq!(request_flags.bits(), vector.request_after, "{vector:?}");
        assert_eq!(
            presentation.presentation_gate_active,
            vector.c2_gate_after != 0,
            "{vector:?}"
        );
        assert_eq!(
            presentation.active_line,
            (vector.active_line_after == 43)
                .then_some(ScriptTransferPresentationLine::InventoryMoved),
            "{vector:?}"
        );
        let cursor = match expected_control {
            ScriptControl::Jump(target) => target.index(),
            ScriptControl::Continue => SCRIPT_START + token.encoded_bytes().len(),
        };
        assert_eq!(cursor, vector.cursor, "{vector:?}");

        let query = vector.query_before & 1 != 0;
        assert_eq!(vector.calls.owner, usize::from(!query), "{vector:?}");
        assert_eq!(vector.calls.field, usize::from(!query) * 2, "{vector:?}");
        assert_eq!(
            vector.calls.remove,
            usize::from(!query && vector.source == "special"),
            "{vector:?}"
        );
        assert_eq!(
            vector.calls.insert,
            usize::from(!query && vector.destination == "special"),
            "{vector:?}"
        );
        assert_eq!(
            vector.calls.failure,
            usize::from(vector.failed),
            "{vector:?}"
        );
        assert_eq!(
            vector.calls.descriptor,
            usize::from(vector.descriptor_checked),
            "{vector:?}"
        );
        assert_eq!(fixture.offsets[4], vector.alternate_offset, "{vector:?}");
    }
}
