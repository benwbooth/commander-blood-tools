use commander_blood_formats::name_area_effect::{
    NameAreaEffectOperation, decode_blood2pg_name_area_effect_sequences,
};
use commander_blood_formats::palette::decode_blood2pg_default_vga_palette;
use commander_blood_formats::world_art::decode_blood2pg_world_artwork_layout;
use serde::Deserialize;

const DATA: usize = 0xF7F0;
const PALETTE: usize = 0x15718;
const EFFECTS: usize = 0x12281;
const ARTWORK: usize = 0x12787;

#[derive(Deserialize)]
struct Oracle {
    palette: Vec<[u8; 3]>,
    effects: Vec<Effect>,
    artwork: Vec<Artwork>,
}

#[derive(Deserialize)]
struct Effect {
    index: usize,
    pointer: usize,
    operation: u8,
    frames: Vec<Frame>,
}

#[derive(Deserialize)]
struct Frame {
    origin: [u16; 2],
    size: [u16; 2],
}

#[derive(Deserialize)]
struct Artwork {
    pointer: usize,
    name: Vec<u8>,
    resource_id: u16,
    entity_id: u16,
    active: bool,
}

fn oracle() -> Oracle {
    serde_json::from_str(include_str!(
        "../../../re/tools/oracle_vectors/big_bug_bang_startup_tables.json"
    ))
    .unwrap()
}

fn word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn fixture() -> Vec<u8> {
    let expected = oracle();
    let mut bytes = vec![0; PALETTE + 768];
    bytes[..2].copy_from_slice(b"MZ");
    for (index, color) in expected.palette.iter().enumerate() {
        bytes[PALETTE + index * 3..PALETTE + index * 3 + 3].copy_from_slice(color);
    }
    for effect in expected.effects {
        word(
            &mut bytes,
            EFFECTS + effect.index * 2,
            effect.pointer as u16,
        );
        let start = DATA + effect.pointer;
        bytes[start] = effect.operation;
        bytes[start + 1] = effect.frames.len() as u8;
        for (index, frame) in effect.frames.iter().enumerate() {
            for (field, value) in frame.origin.iter().chain(&frame.size).enumerate() {
                word(&mut bytes, start + 2 + index * 8 + field * 2, *value);
            }
        }
    }
    for row in expected.artwork {
        let start = DATA + row.pointer;
        bytes[start..start + row.name.len()].copy_from_slice(&row.name);
        word(&mut bytes, start + 16, row.resource_id);
        word(&mut bytes, start + 18, row.entity_id);
        bytes[start + 20] = u8::from(row.active);
    }
    bytes
}

fn check(bytes: &[u8]) {
    let expected = oracle();
    let palette = decode_blood2pg_default_vga_palette(bytes).unwrap();
    assert_eq!(palette.as_slice(), expected.palette);
    let effects = decode_blood2pg_name_area_effect_sequences(bytes).unwrap();
    assert_eq!(effects.len(), 10);
    assert_eq!(effects.len(), expected.effects.len());
    for (actual, expected) in effects.iter().zip(expected.effects) {
        let operation = match expected.operation {
            0 => NameAreaEffectOperation::CollapseToFirst,
            1 => NameAreaEffectOperation::CollapseToLast,
            2 => NameAreaEffectOperation::CycleForward,
            3 => NameAreaEffectOperation::FadeBackward,
            other => panic!("unexpected authored operation {other}"),
        };
        assert_eq!(actual.operation, operation);
        assert_eq!(actual.frames.len(), expected.frames.len());
        for (actual, expected) in actual.frames.iter().zip(expected.frames) {
            assert_eq!(actual.origin, expected.origin);
            assert_eq!(actual.size, expected.size);
        }
    }
    let artwork = decode_blood2pg_world_artwork_layout(bytes).unwrap();
    assert_eq!(artwork.len(), 42);
    assert_eq!(artwork.len(), expected.artwork.len());
    for (actual, expected) in artwork.iter().zip(expected.artwork) {
        assert_eq!(actual.name(), expected.name);
        assert_eq!(actual.resource_id, expected.resource_id);
        assert_eq!(actual.entity_id, expected.entity_id);
        assert_eq!(actual.active, expected.active);
    }
}

#[test]
fn sequel_startup_tables_match_all_native_captures_without_original_assets() {
    check(&fixture());
}

#[test]
fn sequel_startup_tables_reject_truncation_and_invalid_fields() {
    let bytes = fixture();
    assert!(decode_blood2pg_default_vga_palette(&bytes[..PALETTE + 767]).is_none());
    assert!(decode_blood2pg_name_area_effect_sequences(&bytes[..EFFECTS + 19]).is_none());
    assert!(decode_blood2pg_world_artwork_layout(&bytes[..ARTWORK + 42 * 22 + 1]).is_none());
    let mut invalid = bytes.clone();
    invalid[PALETTE + 767] = 64;
    assert!(decode_blood2pg_default_vga_palette(&invalid).is_none());
    let mut invalid = bytes.clone();
    word(&mut invalid, EFFECTS, u16::MAX);
    assert!(decode_blood2pg_name_area_effect_sequences(&invalid).is_none());
    let mut invalid = bytes.clone();
    invalid[DATA + oracle().effects[0].pointer + 1] = 0;
    assert!(decode_blood2pg_name_area_effect_sequences(&invalid).is_none());
    let mut invalid = bytes.clone();
    invalid[ARTWORK + 42 * 22] = 1;
    assert!(decode_blood2pg_world_artwork_layout(&invalid).is_none());
    let mut invalid = bytes;
    invalid[ARTWORK] = 0;
    assert!(decode_blood2pg_world_artwork_layout(&invalid).is_none());
}

#[test]
#[ignore = "requires the original Big Bug Bang executable"]
fn sequel_startup_tables_match_original_executable_and_native_captures() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
    check(&std::fs::read(path).unwrap());
}
