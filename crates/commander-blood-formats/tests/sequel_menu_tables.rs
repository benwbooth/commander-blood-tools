use commander_blood_formats::bloodprg::{
    BloodprgBridgeMenuTextError, decode_blood2pg_bridge_menu_text,
};

const DATA: usize = 0xF7F0;
const OPTION_POINTERS: [u16; 7] = [0x27C9, 0x27D1, 0x27F9, 0x27E3, 0x2804, 0x280B, 0x2813];
const OPTION_LABELS: [&[u8]; 7] = [
    b"VITESSE",
    b"TEXTES",
    b"VOYAGE_OFF",
    b"MUSIQUE_OFF",
    b"SAUVER",
    b"CHARGER",
    b"QUITTER",
];
const TEXT_POINTERS: [u16; 5] = [0x2833, 0x283F, 0x2846, 0x284D, 0x2852];
const TEXT_LABELS: [&[u8]; 5] = [b"TRES_RAPIDE", b"RAPIDE", b"NORMAL", b"LENT", b"TRES_LENT"];

fn word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; DATA + 0x2860];
    bytes[..2].copy_from_slice(b"MZ");
    for (offset, pointers) in [
        (0x27B9, OPTION_POINTERS.as_slice()),
        (0x281D, TEXT_POINTERS.as_slice()),
        (0x282B, &[0x284D, 0x2846, 0x283F]),
    ] {
        for (index, pointer) in pointers.iter().chain(&[0xFFFF]).enumerate() {
            word(&mut bytes, DATA + offset + index * 2, *pointer);
        }
    }
    for (pointer, label) in OPTION_POINTERS
        .into_iter()
        .zip(OPTION_LABELS)
        .chain(TEXT_POINTERS.into_iter().zip(TEXT_LABELS))
        .chain([
            (0x179, b"ANNULER".as_slice()),
            (0x27D8, b"MUSIQUE_ON"),
            (0x27EF, b"VOYAGE_ON"),
        ])
    {
        let start = DATA + usize::from(pointer);
        bytes[start..start + label.len()].copy_from_slice(label);
    }
    word(&mut bytes, DATA + 0xCC2, 2);
    word(&mut bytes, DATA + 0xCC4, 1);
    for (index, speed) in [100, 10, 1].into_iter().enumerate() {
        word(&mut bytes, 0x1D12 + index * 2, speed);
    }
    bytes
}

fn check(bytes: &[u8]) {
    let menu = decode_blood2pg_bridge_menu_text(bytes).unwrap();
    assert_eq!(
        menu.option_labels()
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>(),
        OPTION_LABELS
    );
    assert_eq!(
        menu.text_speed_labels()
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>(),
        TEXT_LABELS
    );
    assert_eq!(menu.music_option_row(), 3);
    assert_eq!(menu.music_on_label(), b"MUSIQUE_ON");
    assert_eq!(menu.cancel_label(), b"ANNULER");
    assert_eq!(menu.initial_text_speed_step(), 2);
    let controls = menu.sequel_controls().unwrap();
    assert_eq!(
        controls
            .speed_labels
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>(),
        [b"LENT".as_slice(), b"NORMAL", b"RAPIDE"]
    );
    assert_eq!(controls.speed_values, [100, 10, 1]);
    assert_eq!(controls.initial_speed, 1);
    assert!(!controls.initial_travel_enabled);
    assert_eq!(controls.travel_on_label.as_ref(), b"VOYAGE_ON");
}

#[test]
fn sequel_menu_tables_decode_all_seven_commands_and_both_speed_lists() {
    check(&fixture());
}

#[test]
fn sequel_menu_tables_reject_malformed_images() {
    let bytes = fixture();
    assert!(matches!(
        decode_blood2pg_bridge_menu_text(&bytes[..DATA + 0x2832]),
        Err(BloodprgBridgeMenuTextError::TruncatedExecutable { .. })
    ));
    let mut invalid = bytes.clone();
    invalid[0] = 0;
    assert!(matches!(
        decode_blood2pg_bridge_menu_text(&invalid),
        Err(BloodprgBridgeMenuTextError::InvalidExecutableSignature)
    ));
    for offset in [0x27C7, 0x2827, 0x2831] {
        let mut invalid = bytes.clone();
        word(&mut invalid, DATA + offset, 0);
        assert!(matches!(
            decode_blood2pg_bridge_menu_text(&invalid),
            Err(BloodprgBridgeMenuTextError::MissingPointerListSentinel { .. })
        ));
    }
    let mut invalid = bytes.clone();
    word(&mut invalid, DATA + 0x27B9, 0xFFFF);
    assert!(matches!(
        decode_blood2pg_bridge_menu_text(&invalid),
        Err(BloodprgBridgeMenuTextError::LabelPointerOutsideExecutable { .. })
    ));
    let mut invalid = bytes.clone();
    invalid[DATA + 0x2852..].fill(b'x');
    assert!(matches!(
        decode_blood2pg_bridge_menu_text(&invalid),
        Err(BloodprgBridgeMenuTextError::MissingLabelTerminator { .. })
    ));
    let mut invalid = bytes;
    invalid[DATA + 0x27C9] = 1;
    assert!(matches!(
        decode_blood2pg_bridge_menu_text(&invalid),
        Err(BloodprgBridgeMenuTextError::InvalidLabelByte { .. })
    ));
}

#[test]
fn sequel_menu_tables_preserve_authored_speed_values_and_travel_low_bit() {
    let mut bytes = fixture();
    word(&mut bytes, 0x1D12, 1234);
    word(&mut bytes, DATA + 0xCC4, 5678);
    for flags in [0, 1, 2, 3, 254, 255] {
        bytes[DATA + 0xCF1] = flags;
        let menu = decode_blood2pg_bridge_menu_text(&bytes).unwrap();
        let controls = menu.sequel_controls().unwrap();
        assert_eq!(controls.speed_values[0], 1234);
        assert_eq!(controls.initial_speed, 5678);
        assert_eq!(controls.initial_travel_enabled, flags & 1 != 0);
    }
}

#[test]
#[ignore = "requires the original Big Bug Bang executable"]
fn sequel_menu_tables_match_original_executable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
    check(&std::fs::read(path).unwrap());
}
