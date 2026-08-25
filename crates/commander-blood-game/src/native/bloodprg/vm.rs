//! Typed helper logic used by the BloodScript runtime.

use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptObjectKind, ScriptState, ScriptWordId,
};

const MAXIMUM_ORIGINAL_OPERAND_COUNT: usize = u16::MAX as usize;
const POSITIVE_OPERAND_BOUNDARY: i16 = 0;
const FIELD_SELECTOR_COUNT: usize = 21;
const OBJECT_KIND_COUNT: usize = 9;
const OBJECT_FLAGS_BYTE_OFFSET: usize = 2;
const OBJECT_HEADER_WORD_SIZE: usize = std::mem::size_of::<u16>();
const OBJECT_IN_PLAY_FLAG: u16 = 2;

const FIELD_OFFSETS: [[u8; OBJECT_KIND_COUNT]; FIELD_SELECTOR_COUNT] = [
    [2, 2, 2, 2, 2, 2, 2, 2, 2],
    [4, 22, 0, 0, 0, 0, 0, 0, 0],
    [0, 26, 0, 0, 0, 0, 0, 0, 0],
    [0, 50, 0, 0, 0, 0, 0, 0, 0],
    [0, 52, 0, 0, 0, 0, 0, 0, 0],
    [0, 30, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 56, 0, 0, 0, 0, 0, 0, 0],
    [0, 54, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 24, 0, 0],
    [0, 0, 0, 0, 0, 0, 28, 0, 0],
    [0, 0, 24, 24, 0, 0, 0, 6, 0],
    [0, 0, 0, 0, 0, 0, 20, 0, 0],
    [0, 0, 0, 0, 0, 0, 22, 0, 0],
    [32, 68, 28, 34, 0, 22, 0, 16, 22],
    [0, 70, 0, 0, 0, 0, 0, 0, 0],
    [0, 20, 20, 20, 0, 0, 0, 0, 0],
    [6, 24, 22, 22, 0, 20, 0, 4, 20],
    [0, 0, 0, 0, 0, 0, 0, 0, 0],
    [8, 58, 0, 28, 0, 0, 0, 10, 0],
    [16, 0, 0, 0, 0, 0, 0, 0, 0],
];

/// Typed selector row in the recovered VM object-field matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptFieldSelector(u8);

impl ScriptFieldSelector {
    /// Field used by presentation handoff logic.
    pub const PRESENTATION_HANDOFF: Self = Self(2);
    /// Per-actor encounter counter.
    pub const ENCOUNTER_COUNT: Self = Self(8);
    /// Per-character high-bit-first object-link set.
    pub const OBJECT_LINKS: Self = Self(5);
    /// Object holder or current location.
    pub const HOLDER_OR_LOCATION: Self = Self(17);
    /// Talk, action, or reciprocal presentation link.
    pub const ACTION: Self = Self(19);

    /// Construct any selector represented by the recovered matrix.
    pub const fn new(value: u8) -> Option<Self> {
        if value < FIELD_SELECTOR_COUNT as u8 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Return the selector's zero-based matrix row.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Resolve a typed object field to its byte position within the object's record.
///
/// This translates `vm_field_offset` at BLOODPRG file offset `0x006023`.
/// Absent matrix cells return `None`; signed table bytes and arbitrary selector
/// indexing from malformed machine state are outside the shipped data domain.
pub fn script_field_offset(kind: ScriptObjectKind, selector: ScriptFieldSelector) -> Option<usize> {
    let offset = FIELD_OFFSETS[selector.index()][object_kind_index(kind)];
    (offset != u8::MIN).then_some(usize::from(offset))
}

/// Return the active object whose state record begins immediately below a threshold.
///
/// This translates `vm_record_lookup_by_threshold` at BLOODPRG file offset
/// `0x006034`. Thresholds at or below the first object return `None` instead of
/// reading the record preceding the directory.
pub fn object_before_threshold(
    directory: &ScriptDirectory,
    threshold: u16,
) -> Option<ScriptObjectId> {
    directory
        .active_objects()
        .take_while(|(_object, entry)| entry.value < threshold)
        .map(|(object, _entry)| object)
        .last()
}

/// Return every decoded profile object carrying the native in-play flag.
///
/// This translates `active_object_list_build` at BLOODPRG file offset
/// `0x00604E`. The decoded [`ScriptState`] already represents the directory's
/// contiguous active-object prefix, so the DOS sentinel and offset list become
/// an owned sequence of stable object identities.
pub fn active_objects_in_play(state: &ScriptState) -> Vec<ScriptObjectId> {
    state
        .objects()
        .iter()
        .filter(|object| object_flags(object.bytes()) & OBJECT_IN_PLAY_FLAG != u16::MIN)
        .map(|object| object.id)
        .collect()
}

/// Resolve an interned dictionary word to an active script object.
///
/// This is the flat, typed translation of `dic_word_lookup` at BLOODPRG file
/// offset `0x006433`. A failed lookup is `None`; the inactive sentinel's raw
/// value is not exposed as a false-result side channel.
pub fn resolve_dictionary_object(
    dictionary: &ScriptDictionary,
    word: ScriptWordId,
    directory: &ScriptDirectory,
) -> Option<ScriptObjectId> {
    dictionary
        .word(word)
        .and_then(|name| directory.find_active_object(name))
}

/// Count the leading strictly positive words in an instruction operand list.
///
/// This translates `scan_zero_word` at BLOODPRG file offset `0x00647b`.
/// The original loop's maximum count remains explicit, while a Rust slice
/// replaces its unbounded word cursor.
pub fn count_positive_operands(operands: &[i16]) -> usize {
    operands
        .iter()
        .take(MAXIMUM_ORIGINAL_OPERAND_COUNT)
        .take_while(|operand| **operand > POSITIVE_OPERAND_BOUNDARY)
        .count()
}

const fn object_kind_index(kind: ScriptObjectKind) -> usize {
    match kind {
        ScriptObjectKind::Player => 0,
        ScriptObjectKind::Actor => 1,
        ScriptObjectKind::CelestialBody => 2,
        ScriptObjectKind::NavigationEntity => 3,
        ScriptObjectKind::Auxiliary => 4,
        ScriptObjectKind::Location => 5,
        ScriptObjectKind::BlackHole => 6,
        ScriptObjectKind::WorldState => 7,
        ScriptObjectKind::InventoryItem => 8,
    }
}

fn object_flags(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(
        bytes[OBJECT_FLAGS_BYTE_OFFSET..OBJECT_FLAGS_BYTE_OFFSET + OBJECT_HEADER_WORD_SIZE]
            .try_into()
            .expect("decoded state objects contain their fixed header"),
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use commander_blood_formats::script::{
        decode_script_dictionary, decode_script_directory, decode_script_state,
    };

    use super::*;

    const LOOKUP_ORACLE_VECTOR_COUNT: usize = 8;
    const OPERAND_SCAN_ORACLE_VECTOR_COUNT: usize = 10;
    const FIELD_ORACLE_VECTOR_COUNT: usize = 8;
    const THRESHOLD_ORACLE_VECTOR_COUNT: usize = 9;
    const ACTIVE_OBJECT_ORACLE_VECTOR_COUNT: usize = 5;
    const DIRECTORY_NAME_CAPACITY: usize = 16;
    const DIRECTORY_ENTRY_SIZE: usize = 20;
    const ACTOR_RECORD_SIZE: usize = 72;
    const ACTOR_KIND: u16 = 2;
    const ORIGINAL_FIELD_TABLE_FILE_OFFSET: usize = 0x14180;
    const ORIGINAL_FIELD_TABLE_KIND_COUNT: usize = 16;
    const SHIPPED_OBJECT_KINDS: [ScriptObjectKind; OBJECT_KIND_COUNT] = [
        ScriptObjectKind::Player,
        ScriptObjectKind::Actor,
        ScriptObjectKind::CelestialBody,
        ScriptObjectKind::NavigationEntity,
        ScriptObjectKind::Auxiliary,
        ScriptObjectKind::Location,
        ScriptObjectKind::BlackHole,
        ScriptObjectKind::WorldState,
        ScriptObjectKind::InventoryItem,
    ];

    #[derive(Deserialize)]
    struct LookupOracleVector {
        name: String,
        object_offset: u16,
        matched_carry: bool,
    }

    #[derive(Deserialize)]
    struct ScanOracleVector {
        count: usize,
        final_ax: u16,
    }

    #[derive(Deserialize)]
    struct FieldOracleVector {
        selector: u16,
        kind_mask: u16,
        lowest_set_bit: usize,
    }

    #[derive(Deserialize)]
    struct ThresholdOracleVector {
        threshold: u16,
        entries: Vec<u16>,
        stop_index: usize,
        ax: u16,
    }

    #[derive(Deserialize)]
    struct ActiveObjectOracleVector {
        name: String,
        entries: Vec<ActiveObjectOracleEntry>,
        active_objects: Vec<u16>,
    }

    #[derive(Deserialize)]
    struct ActiveObjectOracleEntry {
        object_offset: u16,
        entry_kind: u16,
        flags: u16,
    }

    #[test]
    fn field_selector_indexing_matches_every_original_vector() {
        let vectors: Vec<FieldOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6023_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), FIELD_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            assert_eq!(
                vector.kind_mask.trailing_zeros() as usize,
                vector.lowest_set_bit
            );
            if let (Some(selector), Some(kind)) = (
                u8::try_from(vector.selector)
                    .ok()
                    .and_then(ScriptFieldSelector::new),
                ScriptObjectKind::decode(vector.kind_mask & vector.kind_mask.wrapping_neg()),
            ) {
                let offset = script_field_offset(kind, selector);
                assert!(offset.is_none_or(|value| value < kind.record_size()));
            }
        }
    }

    #[test]
    fn field_matrix_matches_every_shipped_kind_in_the_original_binary() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");

        for selector_index in 0..FIELD_SELECTOR_COUNT {
            let selector = ScriptFieldSelector::new(selector_index as u8).unwrap();
            for kind in SHIPPED_OBJECT_KINDS {
                let original_kind_index = kind.mask().trailing_zeros() as usize;
                let original_offset = executable[ORIGINAL_FIELD_TABLE_FILE_OFFSET
                    + selector_index * ORIGINAL_FIELD_TABLE_KIND_COUNT
                    + original_kind_index];
                let expected = (original_offset != u8::MIN).then_some(usize::from(original_offset));
                assert_eq!(
                    script_field_offset(kind, selector),
                    expected,
                    "selector {selector_index}, kind {kind:?}"
                );
            }
        }
    }

    #[test]
    fn threshold_lookup_matches_valid_results_and_rejects_predecessor_reads() {
        let vectors: Vec<ThresholdOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6034_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), THRESHOLD_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let entries = vector
                .entries
                .iter()
                .map(|value| DirectoryFixture {
                    name: b"object",
                    value: *value,
                    active: true,
                })
                .collect::<Vec<_>>();
            let directory = decode_script_directory(&directory_image(&entries)).unwrap();
            let result = object_before_threshold(&directory, vector.threshold);
            if vector.stop_index == usize::MIN {
                assert_eq!(result, None);
            } else {
                let object = result.unwrap();
                assert_eq!(directory.object(object).unwrap().value, vector.ax);
            }
        }
    }

    #[test]
    fn active_object_filter_matches_every_original_vector() {
        let vectors: Vec<ActiveObjectOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_604e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ACTIVE_OBJECT_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let active_entries = vector
                .entries
                .iter()
                .take_while(|entry| entry.entry_kind == 1)
                .collect::<Vec<_>>();
            let mut directory_data = Vec::new();
            let mut state_data = Vec::new();
            for (index, entry) in active_entries.iter().enumerate() {
                let mut directory_entry = [u8::MIN; DIRECTORY_ENTRY_SIZE];
                let state_offset = u16::try_from(index * ACTOR_RECORD_SIZE).unwrap();
                directory_entry[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + 2]
                    .copy_from_slice(&state_offset.to_le_bytes());
                directory_entry[DIRECTORY_NAME_CAPACITY + 2..]
                    .copy_from_slice(&1_u16.to_le_bytes());
                directory_data.extend_from_slice(&directory_entry);

                let mut object = [u8::MIN; ACTOR_RECORD_SIZE];
                object[..OBJECT_HEADER_WORD_SIZE].copy_from_slice(&ACTOR_KIND.to_le_bytes());
                object
                    [OBJECT_FLAGS_BYTE_OFFSET..OBJECT_FLAGS_BYTE_OFFSET + OBJECT_HEADER_WORD_SIZE]
                    .copy_from_slice(&entry.flags.to_le_bytes());
                state_data.extend_from_slice(&object);
            }
            directory_data.extend_from_slice(&[u8::MIN; DIRECTORY_ENTRY_SIZE]);
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            let actual = active_objects_in_play(&state)
                .into_iter()
                .map(|object| active_entries[object.index()].object_offset)
                .collect::<Vec<_>>();
            assert_eq!(actual, vector.active_objects, "{}", vector.name);
        }
    }

    #[test]
    fn dictionary_lookup_matches_every_original_semantic_vector() {
        let vectors: Vec<LookupOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6433_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), LOOKUP_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let (word, entries) = lookup_case(&vector.name);
            let dictionary = decode_script_dictionary(&dictionary_image(word)).unwrap();
            let directory = decode_script_directory(&directory_image(&entries)).unwrap();
            let word = dictionary.resolve_source_offset(u16::MIN).unwrap();
            let result = resolve_dictionary_object(&dictionary, word, &directory);
            assert_eq!(result.is_some(), vector.matched_carry, "{}", vector.name);
            if let Some(object) = result {
                assert_eq!(
                    directory.object(object).unwrap().value,
                    vector.object_offset
                );
            }
        }
    }

    #[test]
    fn operand_scan_matches_every_original_count_vector() {
        let vectors: Vec<ScanOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_647b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OPERAND_SCAN_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut operands = vec![i16::MAX; vector.count];
            if vector.count < MAXIMUM_ORIGINAL_OPERAND_COUNT {
                operands.push(vector.final_ax as i16);
            }
            assert_eq!(count_positive_operands(&operands), vector.count);
        }
    }

    struct DirectoryFixture<'a> {
        name: &'a [u8],
        value: u16,
        active: bool,
    }

    fn lookup_case(name: &str) -> (&'static [u8], Vec<DirectoryFixture<'static>>) {
        match name {
            "first_active_entry_matches" => {
                (b"HELLO", vec![active(b"HELLO", 4_369), inactive(61_166)])
            }
            "second_active_entry_matches" => (
                b"BETA",
                vec![
                    active(b"ALPHA", 4_369),
                    active(b"BETA", 8_738),
                    inactive(61_166),
                ],
            ),
            "first_entry_inactive" => (
                b"ANY",
                vec![DirectoryFixture {
                    name: b"ANY",
                    value: 13_107,
                    active: false,
                }],
            ),
            "active_miss_returns_inactive_object" => {
                (b"MISSING", vec![active(b"OTHER", 4_369), inactive(17_476)])
            }
            "prefix_is_not_a_match" => (b"ABC", vec![active(b"ABCD", 4_369), inactive(21_845)]),
            "high_bytes_compare_unsigned" => (
                b"\x80\xfe",
                vec![active(b"\x80\xfe", 26_214), inactive(61_166)],
            ),
            "dictionary_offset_wraps" => (b"WRAP", vec![active(b"WRAP", 30_583), inactive(61_166)]),
            "directory_stride_wraps" => {
                (b"TARGET", vec![active(b"OTHER", 4_369), inactive(34_952)])
            }
            _ => panic!("unknown oracle case {name}"),
        }
    }

    fn active(name: &'static [u8], value: u16) -> DirectoryFixture<'static> {
        DirectoryFixture {
            name,
            value,
            active: true,
        }
    }

    fn inactive(value: u16) -> DirectoryFixture<'static> {
        DirectoryFixture {
            name: b"",
            value,
            active: false,
        }
    }

    fn dictionary_image(word: &[u8]) -> Vec<u8> {
        let mut image = word.to_vec();
        image.push(u8::MIN);
        image
    }

    fn directory_image(entries: &[DirectoryFixture<'_>]) -> Vec<u8> {
        let mut image = Vec::new();
        for entry in entries {
            let mut name = [u8::MIN; DIRECTORY_NAME_CAPACITY];
            name[..entry.name.len()].copy_from_slice(entry.name);
            image.extend_from_slice(&name);
            image.extend_from_slice(&entry.value.to_le_bytes());
            image.extend_from_slice(&u16::from(entry.active).to_le_bytes());
        }
        image
    }
}
