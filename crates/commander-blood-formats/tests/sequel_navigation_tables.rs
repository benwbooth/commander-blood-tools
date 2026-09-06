use commander_blood_formats::bloodprg::{
    decode_blood2pg_confirm_dialog_regions, decode_blood2pg_hyperspace_resources,
    decode_blood2pg_navigation_resources,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Oracle {
    regions: Vec<Region>,
    labels: Vec<Vec<u8>>,
    wipe_endpoints: Vec<[u16; 2]>,
    hyperspace: Vec<Travel>,
}

#[derive(Deserialize)]
struct Region {
    origin: [i16; 2],
    size: [i16; 2],
}

#[derive(Deserialize)]
struct Travel {
    index: u16,
    name: Vec<u8>,
}

fn oracle() -> Oracle {
    serde_json::from_str(include_str!(
        "../../../re/tools/oracle_vectors/big_bug_bang_navigation_tables.json"
    ))
    .unwrap()
}

fn fixture() -> Vec<u8> {
    let expected = oracle();
    let mut bytes = vec![0; 0x121D0 + 36];
    bytes[..2].copy_from_slice(b"MZ");
    for (index, region) in expected.regions.iter().enumerate() {
        for (field, value) in region.origin.iter().chain(&region.size).enumerate() {
            let start = 0x11F97 + index * 8 + field * 2;
            bytes[start..start + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    for (offset, label) in [0x12D, 0x137, 0x142, 0x14E]
        .into_iter()
        .zip(expected.labels)
    {
        bytes[0xF7F0 + offset..0xF7F0 + offset + label.len()].copy_from_slice(&label);
    }
    for (index, endpoint) in expected.wipe_endpoints.iter().enumerate() {
        for (field, value) in endpoint.iter().enumerate() {
            let start = 0x121D0 + index * 4 + field * 2;
            bytes[start..start + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    for travel in expected.hyperspace.iter().take(8) {
        let start = 0x11960 + usize::from(travel.index) * 16;
        bytes[start..start + travel.name.len()].copy_from_slice(&travel.name);
    }
    bytes
}

fn check(bytes: &[u8]) {
    let expected = oracle();
    let regions = decode_blood2pg_confirm_dialog_regions(bytes).unwrap();
    for (actual, expected) in [regions.yes, regions.no].iter().zip(expected.regions) {
        assert_eq!(actual.origin, expected.origin);
        assert_eq!(actual.size, expected.size);
    }
    let navigation = decode_blood2pg_navigation_resources(bytes).unwrap();
    assert_eq!(
        navigation.wipe_endpoints().as_slice(),
        expected.wipe_endpoints
    );
    let labels = navigation.labels();
    for (actual, expected) in [
        labels.planet(),
        labels.ship(),
        labels.black_hole(),
        labels.life_support(),
    ]
    .into_iter()
    .zip(expected.labels)
    {
        assert_eq!(actual, expected);
    }
    let travel = decode_blood2pg_hyperspace_resources(bytes).unwrap();
    assert_eq!(travel.sequence_names().len(), 8);
    for expected in expected.hyperspace {
        assert_eq!(
            travel.sequence_names()[usize::from(expected.index & 7)].as_ref(),
            expected.name
        );
    }
}

#[test]
fn sequel_navigation_tables_match_captured_native_data() {
    check(&fixture());
}

#[test]
fn sequel_navigation_tables_reject_malformed_images() {
    let bytes = fixture();
    assert!(decode_blood2pg_confirm_dialog_regions(&bytes[..0x11FA6]).is_err());
    assert!(decode_blood2pg_hyperspace_resources(&bytes[..0x119DF]).is_err());
    assert!(decode_blood2pg_navigation_resources(&bytes[..bytes.len() - 1]).is_err());
    let mut invalid = bytes.clone();
    invalid[0] = 0;
    assert!(decode_blood2pg_confirm_dialog_regions(&invalid).is_err());
    assert!(decode_blood2pg_hyperspace_resources(&invalid).is_err());
    assert!(decode_blood2pg_navigation_resources(&invalid).is_err());
    let mut invalid = bytes.clone();
    invalid[0x11960..0x11970].fill(b'x');
    assert!(decode_blood2pg_hyperspace_resources(&invalid).is_err());
    let mut invalid = bytes.clone();
    invalid[0x1196F] = b'x';
    assert!(decode_blood2pg_hyperspace_resources(&invalid).is_err());
    let mut invalid = bytes.clone();
    invalid[0xF91D..0xF927].fill(b'x');
    assert!(decode_blood2pg_navigation_resources(&invalid).is_err());
    let mut invalid = bytes;
    invalid[0xF91D] = 1;
    assert!(decode_blood2pg_navigation_resources(&invalid).is_err());
}

#[test]
#[ignore = "requires the original Big Bug Bang executable"]
fn sequel_navigation_tables_match_original_executable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
    check(&std::fs::read(path).unwrap());
}
