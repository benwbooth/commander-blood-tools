//! BBB executable comparison for the inherited C1 navigation-record handler.

use commander_blood_formats::code::{
    ScriptCodeOffset, ScriptDialect, decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    ScriptRecordStateOperand, decode_script_record_state_operation,
};
use commander_blood_formats::script::{
    ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateWordTriple,
    decode_script_directory, decode_script_state_for_dialect,
};
use serde::Deserialize;

use super::*;

const SCRIPT_CURSOR: usize = 0x9000;
const BRANCH_TARGET: usize = 0x5AA5;
const OBJECT_COUNT: usize = 15;
const TRACKED_SLOT_COUNT: usize = 5;
const PRIMARY_INDEX: usize = 1;
const SECONDARY_INDEX: usize = 2;
const DIRECT_OWNER_INDEX: usize = 3;
const SPECIAL_OWNER_INDEX: usize = 4;
const OPERAND_INDEX: usize = 5;
const NAV_QUERY_INDEX: usize = 6;
const NAV_SPECIAL_INDEX: usize = 7;
const NAV_OWNER_INDEX: usize = 8;
const NAV_REGULAR_INDEX: usize = 9;
const SOURCE_UNKNOWN_INDEX: usize = 10;
const SOURCE_PLAYER_INDEX: usize = 11;
const SOURCE_ACTOR_REJECT_INDEX: usize = 12;
const SOURCE_ACTOR_ACCEPT_INDEX: usize = 13;
const WRONG_PARENT_INDEX: usize = 14;

const KINDS: [ScriptObjectKind; OBJECT_COUNT] = [
    ScriptObjectKind::Auxiliary,
    ScriptObjectKind::WorldState,
    ScriptObjectKind::Actor,
    ScriptObjectKind::Actor,
    ScriptObjectKind::Location,
    ScriptObjectKind::Location,
    ScriptObjectKind::NavigationEntity,
    ScriptObjectKind::NavigationEntity,
    ScriptObjectKind::NavigationEntity,
    ScriptObjectKind::NavigationEntity,
    ScriptObjectKind::Location,
    ScriptObjectKind::Player,
    ScriptObjectKind::Actor,
    ScriptObjectKind::Actor,
    ScriptObjectKind::Location,
];

#[derive(Debug, Deserialize)]
struct EnteredHelpers {
    source: usize,
}

#[derive(Debug, Deserialize)]
struct Vector {
    family: String,
    query_before: u8,
    inverted: bool,
    owner_active: bool,
    operand: u16,
    target_offset: u16,
    source_mode: Option<String>,
    distance: Option<String>,
    parent: Option<String>,
    object_offsets: [u16; OBJECT_COUNT],
    tracked_offsets: [u16; TRACKED_SLOT_COUNT],
    records_before: [[u16; 3]; TRACKED_SLOT_COUNT],
    records_after: [[u16; 3]; TRACKED_SLOT_COUNT],
    query_after: u8,
    guard_depth_after: usize,
    related_operand_after: u16,
    source_entries: Vec<u16>,
    entered: EnteredHelpers,
    unrestored_frame: bool,
    failed: bool,
    written_offset: Option<u16>,
    cursor: usize,
}

struct Fixture {
    directory: ScriptDirectory,
    state: ScriptState,
    ids: [ScriptObjectId; OBJECT_COUNT],
    offsets: [u16; OBJECT_COUNT],
}

fn write_word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn set_parent(bytes: &mut [u8], offsets: &[u16; OBJECT_COUNT], index: usize, parent: u16) {
    let Some(field) = script_field_offset_for_dialect(
        KINDS[index],
        ScriptFieldSelector::HOLDER_OR_LOCATION,
        ScriptDialect::BigBugBang,
    ) else {
        return;
    };
    write_word(bytes, usize::from(offsets[index]) + field, parent);
}

fn set_position(bytes: &mut [u8], offsets: &[u16; OBJECT_COUNT], index: usize, position: [u16; 2]) {
    let field = script_field_offset_for_dialect(
        KINDS[index],
        ScriptFieldSelector::NAVIGATION_POSITION,
        ScriptDialect::BigBugBang,
    )
    .unwrap();
    let start = usize::from(offsets[index]) + field;
    write_word(bytes, start, position[0]);
    write_word(bytes, start + 2, position[1]);
}

fn fixture(vector: &Vector) -> Fixture {
    let mut offsets = [u16::MIN; OBJECT_COUNT];
    let mut cursor = usize::MIN;
    for (index, kind) in KINDS.iter().copied().enumerate() {
        offsets[index] = u16::try_from(cursor).unwrap();
        cursor += kind.record_size_for_dialect(ScriptDialect::BigBugBang);
    }
    assert_eq!(offsets, vector.object_offsets, "{vector:?}");

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
        set_parent(&mut state_bytes, &offsets, index, u16::MAX);
    }

    // The typed primary and secondary objects carry the same in-play behavior
    // as native C1's deliberately unaligned raw aliases 1 and 2.
    write_word(&mut state_bytes, usize::from(offsets[PRIMARY_INDEX]) + 2, 3);
    write_word(
        &mut state_bytes,
        usize::from(offsets[SECONDARY_INDEX]) + 2,
        3,
    );
    let alias_position = [offsets[NAV_SPECIAL_INDEX], 200];
    set_position(&mut state_bytes, &offsets, PRIMARY_INDEX, alias_position);
    set_parent(
        &mut state_bytes,
        &offsets,
        SECONDARY_INDEX,
        offsets[NAV_SPECIAL_INDEX],
    );
    set_position(
        &mut state_bytes,
        &offsets,
        NAV_SPECIAL_INDEX,
        alias_position,
    );

    set_parent(
        &mut state_bytes,
        &offsets,
        DIRECT_OWNER_INDEX,
        offsets[NAV_QUERY_INDEX],
    );
    write_word(
        &mut state_bytes,
        usize::from(offsets[DIRECT_OWNER_INDEX]) + 6,
        offsets[NAV_QUERY_INDEX],
    );
    set_parent(
        &mut state_bytes,
        &offsets,
        SPECIAL_OWNER_INDEX,
        if vector.parent.as_deref() == Some("wrong") {
            offsets[WRONG_PARENT_INDEX]
        } else {
            offsets[NAV_OWNER_INDEX]
        },
    );
    let owner_position = if vector.distance.as_deref() == Some("same") {
        alias_position
    } else {
        [offsets[NAV_SPECIAL_INDEX] + 8, 200]
    };
    set_position(&mut state_bytes, &offsets, NAV_OWNER_INDEX, owner_position);

    let owner_index = match vector.target_offset {
        190 => DIRECT_OWNER_INDEX,
        210 => SPECIAL_OWNER_INDEX,
        394 => NAV_REGULAR_INDEX,
        unknown => panic!("unknown C1 target offset {unknown}"),
    };
    write_word(
        &mut state_bytes,
        usize::from(offsets[owner_index]) + 2,
        u16::from(vector.owner_active),
    );

    let source_target = if vector.family == "special_set" {
        offsets[NAV_OWNER_INDEX]
    } else {
        offsets[NAV_REGULAR_INDEX]
    };
    let source_indices: &[usize] = match vector.source_mode.as_deref().unwrap_or("none") {
        "none" => &[],
        "unknown" => &[SOURCE_UNKNOWN_INDEX],
        "player_reject" | "player_accept" => &[SOURCE_PLAYER_INDEX],
        "actor_reject" => &[SOURCE_ACTOR_REJECT_INDEX],
        "actor_accept" => &[SOURCE_ACTOR_ACCEPT_INDEX],
        "actor_reject_accept" => &[SOURCE_ACTOR_REJECT_INDEX, SOURCE_ACTOR_ACCEPT_INDEX],
        unknown => panic!("unknown C1 source mode {unknown}"),
    };
    for index in source_indices {
        set_parent(&mut state_bytes, &offsets, *index, source_target);
    }
    write_word(
        &mut state_bytes,
        usize::from(offsets[OPERAND_INDEX]) + 2,
        if vector.source_mode.as_deref() == Some("player_accept") {
            3
        } else {
            1
        },
    );
    let link_field = script_field_offset_for_dialect(
        ScriptObjectKind::Actor,
        ScriptFieldSelector::OBJECT_LINKS,
        ScriptDialect::BigBugBang,
    )
    .unwrap();
    state_bytes[usize::from(offsets[SOURCE_ACTOR_ACCEPT_INDEX]) + link_field] =
        1 << (7 - OPERAND_INDEX);

    for (offset, words) in vector
        .tracked_offsets
        .iter()
        .copied()
        .zip(vector.records_before)
    {
        let start = usize::from(offset);
        for (word_index, word) in words.into_iter().enumerate() {
            write_word(&mut state_bytes, start + word_index * 2, word);
        }
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
    let index = fixture
        .offsets
        .iter()
        .position(|candidate| *candidate == offset)
        .unwrap();
    fixture.ids[index]
}

fn operand_from_raw(raw: u16, fixture: &Fixture) -> ScriptRecordStateOperand {
    match raw {
        1 => ScriptRecordStateOperand::PrimaryNavigationObject,
        2 => ScriptRecordStateOperand::SecondaryNavigationObject,
        offset => ScriptRecordStateOperand::Object(object_at_offset(fixture, offset)),
    }
}

fn record_from_words(words: [u16; 3], fixture: &Fixture) -> ScriptActionRecord {
    match words[0] {
        0 => ScriptActionRecord::Empty,
        0xC1 => ScriptActionRecord::Navigation(operand_from_raw(words[1], fixture)),
        _ => ScriptActionRecord::Occupied,
    }
}

fn slot_at_offset(fixture: &Fixture, offset: u16) -> ScriptStateWordTriple {
    fixture
        .state
        .resolve_word_triple_source_offset(offset)
        .unwrap()
}

#[test]
fn sequel_c1_uses_shared_typed_record_state() {
    const VECTOR_COUNT: usize = 624;

    let vectors: Vec<Vector> =
        include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_record_state.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), VECTOR_COUNT);

    for vector in vectors {
        let fixture = fixture(&vector);
        let mut bytes = vec![0xC1];
        if vector.inverted {
            bytes.push(0xA1);
        }
        bytes.extend_from_slice(&vector.target_offset.to_le_bytes());
        bytes.extend_from_slice(&vector.operand.to_le_bytes());
        bytes.push(u8::MAX);
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
        let token = &code.tokens()[0];
        let operation =
            decode_script_record_state_operation(token, &fixture.state, &fixture.directory)
                .unwrap();
        assert_eq!(
            operation.target,
            slot_at_offset(&fixture, vector.target_offset)
        );
        assert_eq!(
            operation.operand,
            operand_from_raw(vector.operand, &fixture)
        );
        assert_eq!(operation.inverted, vector.inverted);
        assert_eq!(vector.related_operand_after, vector.operand);

        let mut records = ScriptActionRecords::default();
        for (offset, words) in vector
            .tracked_offsets
            .iter()
            .copied()
            .zip(vector.records_before)
        {
            records.set_record(
                slot_at_offset(&fixture, offset),
                record_from_words(words, &fixture),
            );
        }

        if vector.entered.source != 0 {
            let target = if vector.family == "special_set" {
                fixture.ids[NAV_OWNER_INDEX]
            } else {
                fixture.ids[NAV_REGULAR_INDEX]
            };
            let source_offsets = navigation_source_objects(&fixture.state, target)
                .unwrap()
                .into_iter()
                .map(|object| fixture.offsets[object.index()])
                .collect::<Vec<_>>();
            assert_eq!(source_offsets, vector.source_entries, "{vector:?}");
        }

        let context = ScriptRecordStateNavigationContext {
            primary_object: fixture.ids[PRIMARY_INDEX],
            secondary_object: fixture.ids[SECONDARY_INDEX],
            arche: fixture.ids[NAV_OWNER_INDEX],
        };
        let mut runtime = ScriptRuntime::new();
        if vector.query_before & 1 != 0 {
            runtime.begin_root_guard(ScriptCodeOffset::new(BRANCH_TARGET));
        } else {
            runtime.arm_root_failure_target(ScriptCodeOffset::new(BRANCH_TARGET));
        }
        let outcome = apply_record_state_operation(
            operation,
            &fixture.state,
            &mut records,
            Some(context),
            &mut runtime,
        )
        .unwrap();

        for (offset, words) in vector
            .tracked_offsets
            .iter()
            .copied()
            .zip(vector.records_after)
        {
            assert_eq!(
                records.record(slot_at_offset(&fixture, offset)),
                record_from_words(words, &fixture),
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
            vector
                .written_offset
                .map(|offset| slot_at_offset(&fixture, offset)),
            "{vector:?}"
        );
        let cursor = match outcome.control {
            ScriptControl::Jump(target) => target.index(),
            ScriptControl::Continue => SCRIPT_CURSOR + token.encoded_bytes().len() - 1,
        };
        assert_eq!(cursor, vector.cursor, "{vector:?}");

        // The original skips its saved SI/DS restoration for successful
        // queries and exhausted source scans. Rust preserves the logical
        // Continue result without reproducing that corrupt machine frame.
        let native_unrestored = (vector.query_before & 1 != 0 && !vector.failed)
            || (vector.entered.source != 0 && !vector.failed && vector.written_offset.is_none());
        assert_eq!(vector.unrestored_frame, native_unrestored, "{vector:?}");
    }
}
