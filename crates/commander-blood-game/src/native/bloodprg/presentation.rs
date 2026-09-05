//! Typed conditional logic shared by BloodScript text presentation.

use std::fmt;

use commander_blood_formats::instruction::{ScriptText, ScriptTextWord};
use commander_blood_formats::script::ScriptWordId;

const HISTORY_WORD_COUNT: usize = 8;
const HISTORY_REQUIRED_MASK: u8 = 0x07;

/// Fixed eight-word concept history used by A6 conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptWordHistory {
    entries: [Option<ScriptWordId>; HISTORY_WORD_COUNT],
    next_index: usize,
}

impl ScriptWordHistory {
    /// Construct a history ring when the next insertion index is valid.
    pub const fn new(
        entries: [Option<ScriptWordId>; HISTORY_WORD_COUNT],
        next_index: usize,
    ) -> Option<Self> {
        if next_index < HISTORY_WORD_COUNT {
            Some(Self {
                entries,
                next_index,
            })
        } else {
            None
        }
    }

    /// Return the ring in physical storage order.
    pub const fn entries(&self) -> &[Option<ScriptWordId>; HISTORY_WORD_COUNT] {
        &self.entries
    }
}

/// Side effects produced after every A6 condition succeeds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextConditionEffects {
    /// Whether accepted dictionary words should become spoken subtitle text.
    pub spoken_word_mode: bool,
    /// Whether execution yields after publishing post-separator words.
    pub yield_requested: bool,
    /// Typed post-separator concept words published for presentation logic.
    pub presentation_words: Vec<ScriptWordId>,
}

/// Missing or malformed typed input required by an A6 condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextConditionError {
    /// The random gate was enabled without a recovered PRNG result.
    MissingRandomResult,
    /// The record comparison was enabled without its resolved field value.
    MissingRecordValue,
    /// The record comparison operand was absent from the decoded instruction.
    MissingRecordOperand,
    /// A history condition was enabled without a history ring.
    MissingHistory,
    /// A condition requires a `0xFFFF` section that is absent.
    MissingWordSection,
}

impl fmt::Display for TextConditionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TextConditionError {}

/// Evaluate the recovered `vm_condition_5` logic over typed flat data.
pub fn evaluate_text_conditions(
    text: &ScriptText,
    random_result: Option<u16>,
    record_value: Option<u16>,
    history: Option<&ScriptWordHistory>,
    effects: &mut TextConditionEffects,
) -> Result<bool, TextConditionError> {
    if text.control.uses_random_gate()
        && random_result.ok_or(TextConditionError::MissingRandomResult)? != u16::MIN
    {
        return Ok(false);
    }

    if text.control.uses_record_condition() {
        let record_value = record_value.ok_or(TextConditionError::MissingRecordValue)?;
        let operand = text
            .record_condition_operand
            .ok_or(TextConditionError::MissingRecordOperand)?;
        let accepted = if text.control.uses_record_equality() {
            record_value == operand
        } else {
            (record_value as i16) > (operand as i16)
        };
        if !accepted {
            return Ok(false);
        }
    }

    let sections = word_sections(&text.words);
    if text.control.uses_history_condition() {
        let history = history.ok_or(TextConditionError::MissingHistory)?;
        let candidates = sections
            .get(1)
            .ok_or(TextConditionError::MissingWordSection)?;
        let required = text.control.detail() & HISTORY_REQUIRED_MASK;
        if required == u8::MIN {
            if !recent_words_are_listed(history, candidates) {
                return Ok(false);
            }
        } else if !history_contains_required_matches(history, candidates, required) {
            return Ok(false);
        }
    }

    if text.control.emits_spoken_text() {
        effects.spoken_word_mode = true;
    }
    if text.control.arms_resume() {
        let section_index = if text.control.uses_history_condition() {
            2
        } else {
            1
        };
        effects.presentation_words = sections
            .get(section_index)
            .ok_or(TextConditionError::MissingWordSection)?
            .clone();
        effects.yield_requested = true;
    }
    Ok(true)
}

fn word_sections(words: &[ScriptTextWord]) -> Vec<Vec<ScriptWordId>> {
    let mut sections = vec![Vec::new()];
    for word in words {
        match *word {
            ScriptTextWord::Dictionary(word) => sections
                .last_mut()
                .expect("one section always exists")
                .push(word),
            ScriptTextWord::SectionSeparator => sections.push(Vec::new()),
        }
    }
    sections
}

fn recent_words_are_listed(history: &ScriptWordHistory, candidates: &[ScriptWordId]) -> bool {
    (1..=candidates.len()).all(|distance| {
        let index = (history.next_index + HISTORY_WORD_COUNT - distance) % HISTORY_WORD_COUNT;
        history.entries[index].is_some_and(|word| candidates.contains(&word))
    })
}

fn history_contains_required_matches(
    history: &ScriptWordHistory,
    candidates: &[ScriptWordId],
    required: u8,
) -> bool {
    let mut remaining = usize::from(required);
    for candidate in candidates {
        for history_word in history.entries.iter().flatten() {
            if history_word == candidate {
                remaining = remaining.saturating_sub(1);
                if remaining == usize::MIN {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::instruction::{ScriptLineRecordOffset, ScriptTextControl};
    use commander_blood_formats::script::{ScriptDictionary, decode_script_dictionary};
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct ConditionOracle {
        name: String,
        control: u8,
        detail: u8,
        prng_result: Option<u16>,
        record_word: Option<u16>,
        history_words: Option<Vec<u16>>,
        text_word_list_mode: u8,
        yield_flag: u8,
        presentation_words: Vec<u16>,
        success_carry: bool,
    }

    fn words() -> Vec<ScriptWordId> {
        let dictionary = decode_script_dictionary(b"zero\0one\0two\0three\0four\0").unwrap();
        [0, 5, 9, 13, 19]
            .into_iter()
            .map(|offset| dictionary.resolve_source_offset(offset).unwrap())
            .collect()
    }

    fn text(control: u16, operand: Option<u16>, words: Vec<ScriptTextWord>) -> ScriptText {
        let control = ScriptTextControl::decode(control);
        ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 0,
            control,
            resume_target: control.arms_resume().then_some(ScriptCodeOffset::new(0)),
            record_condition_operand: operand,
            words: words.into_boxed_slice(),
        }
    }

    fn dictionary_word(dictionary: &ScriptDictionary, source_offset: u16) -> ScriptWordId {
        dictionary.resolve_source_offset(source_offset).unwrap()
    }

    fn oracle_record_operand(name: &str) -> Option<u16> {
        match name {
            "field_equality_succeeds" => Some(0x1234),
            "field_equality_fails" => Some(0x5678),
            "field_signed_greater_succeeds" | "field_inverted_order_succeeds" => Some(u16::MAX),
            "field_signed_greater_fails" => Some(u16::MIN),
            "combined_cursor_and_side_effect_paths" => Some(0x2222),
            _ => None,
        }
    }

    fn oracle_words(name: &str, dictionary: &ScriptDictionary) -> Vec<ScriptTextWord> {
        let word = |offset| ScriptTextWord::Dictionary(dictionary_word(dictionary, offset));
        match name {
            "history_list_accepts_recent_words" | "history_list_rejects_missing_recent_word" => {
                vec![ScriptTextWord::SectionSeparator, word(0x1111), word(0x2222)]
            }
            "duplicate_history_slots_satisfy_required_count" => {
                vec![ScriptTextWord::SectionSeparator, word(0x3333)]
            }
            "required_history_hits_zero_sentinel" => {
                vec![ScriptTextWord::SectionSeparator, word(0x1111)]
            }
            "presentation_words_copy_to_stack_segment" => {
                vec![ScriptTextWord::SectionSeparator, word(0x1234), word(0x5678)]
            }
            "combined_cursor_and_side_effect_paths" => vec![
                ScriptTextWord::SectionSeparator,
                word(0x3333),
                ScriptTextWord::SectionSeparator,
                word(0x4444),
            ],
            _ => Vec::new(),
        }
    }

    fn oracle_history(
        vector: &ConditionOracle,
        dictionary: &ScriptDictionary,
    ) -> Option<ScriptWordHistory> {
        vector.history_words.as_ref().map(|source_offsets| {
            let entries: [Option<ScriptWordId>; HISTORY_WORD_COUNT] = source_offsets
                .iter()
                .map(|offset| Some(dictionary_word(dictionary, *offset)))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let next_index = if vector.name.starts_with("history_list_") {
                2
            } else {
                0
            };
            ScriptWordHistory::new(entries, next_index).unwrap()
        })
    }

    #[test]
    fn record_comparison_matches_both_original_binaries_including_flag_priority() {
        #[derive(Deserialize)]
        struct RecordConditionOracle {
            game: String,
            flags: u16,
            record: u16,
            operand: u16,
            accepted: bool,
        }
        let vectors: Vec<RecordConditionOracle> =
            include_str!("../../../../../re/tools/oracle_vectors/text_record_condition.jsonl")
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(vectors.len(), 288);
        let mut game_counts = [0; 2];
        for vector in vectors {
            let index = match vector.game.as_str() {
                "commander" => 0,
                "sequel" => 1,
                other => panic!("unknown original game {other}"),
            };
            game_counts[index] += 1;
            let mut effects = TextConditionEffects::default();
            let accepted = evaluate_text_conditions(
                &text(vector.flags, Some(vector.operand), Vec::new()),
                None,
                Some(vector.record),
                None,
                &mut effects,
            )
            .unwrap();
            assert_eq!(
                accepted, vector.accepted,
                "{} flags={} record={} operand={}",
                vector.game, vector.flags, vector.record, vector.operand
            );
            assert_eq!(effects, TextConditionEffects::default());
        }
        assert_eq!(game_counts, [144, 144]);
    }

    #[test]
    fn random_and_signed_record_conditions_short_circuit() {
        let mut effects = TextConditionEffects::default();
        assert!(
            !evaluate_text_conditions(
                &text(0x0002, None, Vec::new()),
                Some(1),
                None,
                None,
                &mut effects,
            )
            .unwrap()
        );
        assert!(
            evaluate_text_conditions(
                &text(0x0004, Some(u16::MAX), Vec::new()),
                None,
                Some(0),
                None,
                &mut effects,
            )
            .unwrap()
        );
        assert!(
            !evaluate_text_conditions(
                &text(0x0104, Some(7), Vec::new()),
                None,
                Some(8),
                None,
                &mut effects,
            )
            .unwrap()
        );
    }

    #[test]
    fn history_and_presentation_sections_use_interned_words() {
        let words = words();
        let history = ScriptWordHistory::new(
            [
                Some(words[1]),
                Some(words[2]),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            2,
        )
        .unwrap();
        let instruction = text(
            0x0070,
            None,
            vec![
                ScriptTextWord::Dictionary(words[0]),
                ScriptTextWord::SectionSeparator,
                ScriptTextWord::Dictionary(words[1]),
                ScriptTextWord::Dictionary(words[2]),
                ScriptTextWord::SectionSeparator,
                ScriptTextWord::Dictionary(words[3]),
            ],
        );
        let mut effects = TextConditionEffects::default();
        assert!(
            evaluate_text_conditions(&instruction, None, None, Some(&history), &mut effects,)
                .unwrap()
        );
        assert!(effects.spoken_word_mode);
        assert!(effects.yield_requested);
        assert_eq!(effects.presentation_words, [words[3]]);
    }

    #[test]
    fn duplicate_history_entries_satisfy_required_count() {
        let words = words();
        let history = ScriptWordHistory::new(
            [
                Some(words[1]),
                Some(words[1]),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            2,
        )
        .unwrap();
        let instruction = text(
            0x0240,
            None,
            vec![
                ScriptTextWord::SectionSeparator,
                ScriptTextWord::Dictionary(words[1]),
            ],
        );
        assert!(
            evaluate_text_conditions(
                &instruction,
                None,
                None,
                Some(&history),
                &mut TextConditionEffects::default(),
            )
            .unwrap()
        );
    }

    #[test]
    fn every_original_condition_vector_matches_flat_typed_logic() {
        let dictionary_data = vec![u8::MIN; usize::from(u16::MAX) + 1];
        let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
        let vectors: Vec<ConditionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6339_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let encoded_control =
                u16::from(vector.control) | (u16::from(vector.detail) << u8::BITS);
            let instruction = text(
                encoded_control,
                oracle_record_operand(&vector.name),
                oracle_words(&vector.name, &dictionary),
            );
            let history = oracle_history(&vector, &dictionary);
            let mut effects = TextConditionEffects::default();
            let accepted = evaluate_text_conditions(
                &instruction,
                vector.prng_result,
                vector.record_word,
                history.as_ref(),
                &mut effects,
            )
            .unwrap();

            assert_eq!(accepted, vector.success_carry, "{}", vector.name);
            assert_eq!(
                effects.spoken_word_mode,
                vector.text_word_list_mode != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                effects.yield_requested,
                vector.yield_flag != u8::MIN,
                "{}",
                vector.name
            );
            let expected_words: Vec<ScriptWordId> = vector
                .presentation_words
                .into_iter()
                .take_while(|word| *word != u16::MIN)
                .map(|word| dictionary_word(&dictionary, word))
                .collect();
            assert_eq!(
                effects.presentation_words, expected_words,
                "{}",
                vector.name
            );
        }
    }
}
