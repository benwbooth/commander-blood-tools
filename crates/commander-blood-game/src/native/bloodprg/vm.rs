//! Typed helper logic used by the BloodScript runtime.

use commander_blood_formats::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptWordId,
};

const MAXIMUM_ORIGINAL_OPERAND_COUNT: usize = u16::MAX as usize;
const POSITIVE_OPERAND_BOUNDARY: i16 = 0;

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

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use commander_blood_formats::script::{decode_script_dictionary, decode_script_directory};

    use super::*;

    const LOOKUP_ORACLE_VECTOR_COUNT: usize = 8;
    const OPERAND_SCAN_ORACLE_VECTOR_COUNT: usize = 10;
    const DIRECTORY_NAME_CAPACITY: usize = 16;

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
