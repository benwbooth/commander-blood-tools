//! Progressive inline layout for BloodScript concept-menu words.

use std::fmt;

use commander_blood_formats::instruction::ScriptTextWord;
use commander_blood_formats::script::{ScriptDictionary, ScriptState, ScriptWordId};

use super::TextPresentationState;

const MENU_LEFT: u16 = 10;
const MENU_TOP: u16 = 8;
const MENU_ROW_HEIGHT: u16 = 8;
const MENU_WORD_GAP: u16 = 6;
const MENU_RIGHT: i16 = 300;
const MENU_COLOR: u8 = 239;

/// Width provider used by the backend-independent menu layout step.
pub trait InlineMenuTextMetrics {
    /// Return the width left by rendering one visible word in the main font.
    fn rendered_width(&mut self, text: &[u8]) -> u16;

    /// Return the width of the following word in the lookahead font.
    fn lookahead_width(&mut self, text: Option<&[u8]>) -> u16;
}

/// One word that the renderer must draw for the current reveal frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineMenuWordPlacement {
    /// Text resolved for this frame, including live numeric substitutions.
    pub text: Box<[u8]>,
    /// Wrapped original pixel coordinates.
    pub position: [u16; 2],
    /// Width reported by the main-font renderer.
    pub width: u16,
    /// Recovered palette index.
    pub color: u8,
}

/// Gate that prevented the inline menu from being redrawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineMenuRevealGate {
    /// No deferred menu exists and its presentation hold is not ready.
    HoldNotReady,
    /// The ready hold belongs to another presentation.
    WrongOwner,
}

/// Observable state and draw work produced by one accepted reveal step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineMenuRevealFrame {
    /// Visible words in draw order.
    pub placements: Vec<InlineMenuWordPlacement>,
    /// Layout cursor after the last visible word.
    pub cursor: [u16; 2],
    /// This call exposed one additional word.
    pub reveal_advanced: bool,
    /// This call armed the final completion hold.
    pub completion_armed: bool,
}

/// Result of one reveal call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineMenuRevealOutcome {
    /// Presentation ownership rejected the call without changing state.
    Gated(InlineMenuRevealGate),
    /// The menu was laid out or advanced.
    Frame(InlineMenuRevealFrame),
}

/// Invalid typed data supplied to the menu reveal step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineMenuRevealError {
    /// A menu word belongs to another dictionary.
    UnknownDictionaryWord(ScriptWordId),
    /// A numeric substitution cannot be bound to an owned VAR word.
    InvalidStateNumber {
        /// Serialized VAR byte position that could not be resolved.
        source_offset: u16,
    },
    /// The authored operand count cannot participate in native 16-bit timing.
    WordCountOutOfRange {
        /// Unrepresentable owned word count.
        count: usize,
    },
}

impl fmt::Display for InlineMenuRevealError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InlineMenuRevealError {}

/// Redraw and advance one frame of the inline concept-menu presentation.
///
/// This translates `dlg_menu_words_inline_reveal_step` at BLOODPRG file
/// offset `0x0072A8`. Interned words and a reveal count replace dictionary and
/// menu pointers. Pixel and delay arithmetic retains the original 16-bit wrap.
pub fn reveal_inline_menu_step<M: InlineMenuTextMetrics>(
    presentation: &mut TextPresentationState,
    dictionary: &ScriptDictionary,
    state: Option<&ScriptState>,
    owner_matches: bool,
    word_delay: u16,
    metrics: &mut M,
) -> Result<InlineMenuRevealOutcome, InlineMenuRevealError> {
    if !presentation.menu_deferred {
        if !presentation.hold_ready {
            return Ok(InlineMenuRevealOutcome::Gated(
                InlineMenuRevealGate::HoldNotReady,
            ));
        }
        if !owner_matches {
            return Ok(InlineMenuRevealOutcome::Gated(
                InlineMenuRevealGate::WrongOwner,
            ));
        }
    }

    let mut placements = Vec::new();
    let mut x = MENU_LEFT;
    let mut y = MENU_TOP;
    let mut cursor = usize::MIN;
    let mut encoded_cursor = usize::MIN;

    loop {
        let Some(current) = presentation.menu_words.get(cursor).copied() else {
            return complete_menu(presentation, placements, x, y, word_delay);
        };
        let text = match current {
            ScriptTextWord::Dictionary(word) => dictionary
                .word(word)
                .ok_or(InlineMenuRevealError::UnknownDictionaryWord(word))?,
            ScriptTextWord::StateNumber(number) => {
                let source_offset = number.source_offset();
                let value = state
                    .and_then(|state| {
                        state
                            .resolve_word_source_offset(source_offset)
                            .and_then(|field| state.word(field))
                    })
                    .ok_or(InlineMenuRevealError::InvalidStateNumber { source_offset })?;
                presentation.menu_number_text =
                    (value as i16).to_string().into_bytes().into_boxed_slice();
                &presentation.menu_number_text
            }
            ScriptTextWord::SectionSeparator => {
                return complete_menu(presentation, placements, x, y, word_delay);
            }
        };
        let width = metrics.rendered_width(text);
        placements.push(InlineMenuWordPlacement {
            text: Box::from(text),
            position: [x, y],
            width,
            color: MENU_COLOR,
        });

        cursor = cursor.saturating_add(1);
        encoded_cursor += current.encoded_word_count();
        let next = match presentation.menu_words.get(cursor).copied() {
            Some(ScriptTextWord::Dictionary(next)) => Some(
                dictionary
                    .word(next)
                    .ok_or(InlineMenuRevealError::UnknownDictionaryWord(next))?,
            ),
            // Native lookahead reads DIC slot 1 before the next substitution
            // formats its value. Reuse the last number; do not read ahead in VAR.
            Some(ScriptTextWord::StateNumber(_)) => Some(presentation.menu_number_text.as_ref()),
            Some(ScriptTextWord::SectionSeparator) | None => None,
        };
        if next
            .and_then(|text| text.first().copied())
            .is_some_and(is_attached_punctuation)
        {
            x = x.wrapping_add(width);
        } else {
            x = x.wrapping_add(width).wrapping_add(MENU_WORD_GAP);
            let next_width = metrics.lookahead_width(next);
            if x.wrapping_add(next_width) as i16 >= MENU_RIGHT {
                x = MENU_LEFT;
                y = y.wrapping_add(MENU_ROW_HEIGHT);
            }
        }

        if encoded_cursor >= presentation.menu_reveal_count {
            let reveal_advanced = presentation.dialogue_hold_countdown == u16::MIN;
            if reveal_advanced {
                presentation.menu_reveal_count = presentation.menu_reveal_count.saturating_add(1);
                presentation.dialogue_hold_countdown = word_delay;
            }
            return Ok(InlineMenuRevealOutcome::Frame(InlineMenuRevealFrame {
                placements,
                cursor: [x, y],
                reveal_advanced,
                completion_armed: false,
            }));
        }
    }
}

fn complete_menu(
    presentation: &mut TextPresentationState,
    placements: Vec<InlineMenuWordPlacement>,
    x: u16,
    y: u16,
    word_delay: u16,
) -> Result<InlineMenuRevealOutcome, InlineMenuRevealError> {
    let completion_armed = !presentation.hold_ready && !presentation.dialogue_hold_complete;
    if completion_armed {
        let word_count = u16::try_from(presentation.menu_word_count).map_err(|_| {
            InlineMenuRevealError::WordCountOutOfRange {
                count: presentation.menu_word_count,
            }
        })?;
        presentation.dialogue_hold_countdown = word_count
            .wrapping_mul(word_delay >> 1)
            .wrapping_add(MENU_WORD_GAP);
        presentation.dialogue_hold_complete = true;
    }
    Ok(InlineMenuRevealOutcome::Frame(InlineMenuRevealFrame {
        placements,
        cursor: [x, y],
        reveal_advanced: false,
        completion_armed,
    }))
}

const fn is_attached_punctuation(byte: u8) -> bool {
    matches!(byte, b'.' | b',' | b':' | b'!' | b'?')
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 16;
    const NATIVE_MENU_OWNER: u16 = 26_544;
    const FLAT_NONZERO_BASE_CURSOR: [u16; 2] = [29, MENU_TOP];

    #[derive(Deserialize)]
    struct NumericMenuOracle {
        name: String,
        menu: Vec<u16>,
        numbers: std::collections::BTreeMap<u16, i16>,
        reveal: usize,
        countdown: u16,
        delay: u16,
        scratch: Vec<u8>,
        output: NumericMenuOutput,
    }

    #[derive(Deserialize)]
    struct NumericMenuOutput {
        draws: Vec<NumericMenuDraw>,
        x: u16,
        y: u16,
        reveal: usize,
        countdown: u16,
        complete: bool,
        scratch: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct NumericMenuDraw {
        text: Vec<u8>,
        position: [u16; 2],
        width: u16,
    }

    struct SequelFontMetrics {
        fonts: commander_blood_formats::bloodprg::BloodprgFontResources,
        surface: Vec<u8>,
    }

    impl InlineMenuTextMetrics for SequelFontMetrics {
        fn rendered_width(&mut self, text: &[u8]) -> u16 {
            use crate::native::bloodprg::{FontPoint, FontVerticalBand, draw_planar_dialogue_text};
            draw_planar_dialogue_text(
                &mut self.surface,
                &self.fonts,
                text,
                FontPoint { x: 0, y: 0 },
                FontVerticalBand {
                    top: 0,
                    bottom: 199,
                },
                239,
            )
            .unwrap()
            .draw_width
        }

        fn lookahead_width(&mut self, text: Option<&[u8]>) -> u16 {
            use crate::native::bloodprg::{GameFontFace, measure_game_text_width};
            measure_game_text_width(text.unwrap_or_default(), GameFontFace::Main, &self.fonts)
                .unwrap()
        }
    }

    #[test]
    #[ignore = "requires original sequel fonts and loose SCRIPT1 state"]
    fn sequel_numeric_menu_matches_complete_original_renderer_and_helpers() {
        use commander_blood_formats::code::ScriptDialect;
        use commander_blood_formats::instruction::ScriptTextStateNumber;
        use commander_blood_formats::script::{
            decode_script_directory, decode_script_state_for_dialect,
        };
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang");
        let executable = std::fs::read(root.join("disc/BLOOD2PG.EXE")).unwrap();
        let fonts = crate::game::GameVariant::BigBugBang
            .decode_fonts(&executable)
            .unwrap();
        let resources = root.join("imported-assets/resources");
        let directory =
            decode_script_directory(&std::fs::read(resources.join("SCRIPT1.DEB")).unwrap())
                .unwrap();
        let original_state = decode_script_state_for_dialect(
            &std::fs::read(resources.join("SCRIPT1.VAR")).unwrap(),
            &directory,
            ScriptDialect::BigBugBang,
        )
        .unwrap();
        let vectors: Vec<NumericMenuOracle> =
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_menu_number.jsonl")
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(vectors.len(), 59);
        let mut dictionary_bytes = vec![0; 128];
        for (offset, value) in [
            (16, b"VALUE".as_slice()),
            (32, b","),
            (48, b"NEXT"),
            (64, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            (96, b"."),
        ] {
            dictionary_bytes[offset..offset + value.len()].copy_from_slice(value);
        }
        let dictionary = decode_script_dictionary(&dictionary_bytes).unwrap();
        let mut metrics = SequelFontMetrics {
            fonts,
            surface: vec![0; 320 * 200],
        };
        for vector in vectors {
            let mut state = original_state.clone();
            for (offset, value) in &vector.numbers {
                let field = state.resolve_word_source_offset(*offset).unwrap();
                assert!(state.set_word(field, *value as u16));
            }
            let mut words = Vec::new();
            let mut encoded = vector.menu.iter();
            while let Some(word) = encoded.next() {
                words.push(if *word == 1 {
                    ScriptTextWord::StateNumber(ScriptTextStateNumber::decode(
                        *encoded.next().unwrap(),
                    ))
                } else {
                    ScriptTextWord::Dictionary(dictionary.resolve_source_offset(*word).unwrap())
                });
            }
            let mut presentation = TextPresentationState {
                menu_deferred: true,
                menu_words: words.into_boxed_slice(),
                menu_word_count: vector.menu.len(),
                menu_reveal_count: vector.reveal,
                dialogue_hold_countdown: vector.countdown,
                menu_number_text: vector.scratch.into_boxed_slice(),
                ..Default::default()
            };
            let outcome = reveal_inline_menu_step(
                &mut presentation,
                &dictionary,
                Some(&state),
                true,
                vector.delay,
                &mut metrics,
            )
            .unwrap();
            let InlineMenuRevealOutcome::Frame(frame) = outcome else {
                panic!("{}: gated", vector.name)
            };
            assert_eq!(
                frame.placements.len(),
                vector.output.draws.len(),
                "{}",
                vector.name
            );
            for (actual, expected) in frame.placements.iter().zip(&vector.output.draws) {
                assert_eq!(actual.text.as_ref(), expected.text, "{}", vector.name);
                assert_eq!(actual.position, expected.position, "{}", vector.name);
                assert_eq!(actual.width, expected.width, "{}", vector.name);
            }
            assert_eq!(
                frame.cursor,
                [vector.output.x, vector.output.y],
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.menu_reveal_count, vector.output.reveal,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.dialogue_hold_countdown, vector.output.countdown,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.dialogue_hold_complete, vector.output.complete,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.menu_number_text.as_ref(),
                vector.output.scratch,
                "{}",
                vector.name
            );
        }
    }
    const FLAT_SPLIT_STATE_CURSOR: [u16; 2] = [22, MENU_TOP];

    #[derive(Deserialize)]
    struct RevealOracle {
        name: String,
        gates: RevealGates,
        calls: Vec<RevealCall>,
        x_after: u16,
        hold_after: u16,
        complete_after: u8,
    }

    #[derive(Deserialize)]
    struct RevealGates {
        defer: u8,
        ready: u8,
        owner: u16,
    }

    #[derive(Deserialize)]
    struct RevealCall {
        call: String,
        x: Option<u16>,
        y: Option<u16>,
        width: Option<u16>,
    }

    struct RevealCase {
        menu: Vec<ScriptTextWord>,
        reveal_count: usize,
        hold_before: u16,
        complete_before: bool,
        word_count: usize,
        delay: u16,
        rendered_widths: Vec<u16>,
        lookahead_widths: Vec<u16>,
        expected_cursor: Option<[u16; 2]>,
        expected_reveal_count: usize,
        expected_reveal_advanced: bool,
        expected_completion_armed: bool,
    }

    struct OracleMetrics {
        rendered_widths: VecDeque<u16>,
        lookahead_widths: VecDeque<u16>,
    }

    impl InlineMenuTextMetrics for OracleMetrics {
        fn rendered_width(&mut self, _text: &[u8]) -> u16 {
            self.rendered_widths.pop_front().unwrap()
        }

        fn lookahead_width(&mut self, _word: Option<&[u8]>) -> u16 {
            self.lookahead_widths.pop_front().unwrap()
        }
    }

    #[test]
    fn reveal_step_matches_every_original_semantic_vector() {
        let vectors: Vec<RevealOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_72a8_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let dictionary = decode_script_dictionary(b"A\0,\0B\0.\0").unwrap();
        let a = dictionary.resolve_source_offset(0).unwrap();
        let comma = dictionary.resolve_source_offset(2).unwrap();
        let b = dictionary.resolve_source_offset(4).unwrap();

        for vector in vectors {
            let case = reveal_case(&vector.name, a, comma, b);
            let mut presentation = TextPresentationState {
                menu_deferred: vector.gates.defer & 1 != u8::MIN,
                hold_ready: vector.gates.ready & 1 != u8::MIN,
                menu_word_count: case.word_count,
                menu_reveal_count: case.reveal_count,
                dialogue_hold_countdown: case.hold_before,
                dialogue_hold_complete: case.complete_before,
                menu_words: case.menu.into_boxed_slice(),
                ..TextPresentationState::default()
            };
            let mut metrics = OracleMetrics {
                rendered_widths: case.rendered_widths.into(),
                lookahead_widths: case.lookahead_widths.into(),
            };
            let outcome = reveal_inline_menu_step(
                &mut presentation,
                &dictionary,
                None,
                vector.gates.owner == NATIVE_MENU_OWNER,
                case.delay,
                &mut metrics,
            )
            .unwrap();

            match (case.expected_cursor, outcome) {
                (None, InlineMenuRevealOutcome::Gated(_)) => {}
                (Some(expected_cursor), InlineMenuRevealOutcome::Frame(frame)) => {
                    if !matches!(
                        vector.name.as_str(),
                        "nonzero_dictionary_base_preserves_asymmetric_peek"
                            | "split_data_segments_expose_shipped_alias"
                    ) {
                        assert_eq!(expected_cursor[0], vector.x_after, "{}", vector.name);
                    }
                    let expected_placements = expected_placements(&vector);
                    let actual_placements = frame
                        .placements
                        .iter()
                        .map(|placement| {
                            (
                                placement.position[0],
                                placement.position[1],
                                placement.width,
                            )
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(actual_placements, expected_placements, "{}", vector.name);
                    assert_eq!(frame.cursor, expected_cursor, "{}", vector.name);
                    assert_eq!(
                        frame.reveal_advanced, case.expected_reveal_advanced,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        frame.completion_armed, case.expected_completion_armed,
                        "{}",
                        vector.name
                    );
                }
                (expected, actual) => panic!(
                    "{}: unexpected outcome {actual:?} for cursor {expected:?}",
                    vector.name
                ),
            }

            assert!(metrics.rendered_widths.is_empty(), "{}", vector.name);
            assert!(metrics.lookahead_widths.is_empty(), "{}", vector.name);
            assert_eq!(
                presentation.menu_reveal_count, case.expected_reveal_count,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.dialogue_hold_countdown, vector.hold_after,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.dialogue_hold_complete,
                vector.complete_after != u8::MIN,
                "{}",
                vector.name
            );
        }
    }

    fn expected_placements(vector: &RevealOracle) -> Vec<(u16, u16, u16)> {
        match vector.name.as_str() {
            "nonzero_dictionary_base_preserves_asymmetric_peek" => {
                vec![(MENU_LEFT, MENU_TOP, 9), (19, MENU_TOP, 4)]
            }
            "split_data_segments_expose_shipped_alias" => vec![(MENU_LEFT, MENU_TOP, 6)],
            _ => vector
                .calls
                .iter()
                .filter(|call| call.call == "draw")
                .map(|call| (call.x.unwrap(), call.y.unwrap(), call.width.unwrap()))
                .collect(),
        }
    }

    fn reveal_case(
        name: &str,
        a: ScriptWordId,
        comma: ScriptWordId,
        b: ScriptWordId,
    ) -> RevealCase {
        let word = |word| ScriptTextWord::Dictionary(word);
        let base = |menu: Vec<ScriptTextWord>, reveal_count, delay| RevealCase {
            expected_reveal_count: reveal_count,
            menu,
            reveal_count,
            hold_before: u16::MIN,
            complete_before: false,
            word_count: 2,
            delay,
            rendered_widths: vec![],
            lookahead_widths: vec![],
            expected_cursor: Some([MENU_LEFT, MENU_TOP]),
            expected_reveal_advanced: false,
            expected_completion_armed: false,
        };

        match name {
            "inactive_gates_return" | "ready_owner_mismatch_returns" => RevealCase {
                expected_cursor: None,
                ..base(vec![word(a)], 1, 8)
            },
            "ready_owner_allows_but_ready_blocks_completion" => base(vec![], 0, 8),
            "zero_sentinel_completes" => RevealCase {
                word_count: 3,
                delay: 10,
                expected_completion_armed: true,
                ..base(vec![], 0, 10)
            },
            "ffff_sentinel_wrapped_hold_math" => RevealCase {
                menu: vec![ScriptTextWord::SectionSeparator],
                word_count: usize::from(u16::MAX),
                delay: u16::MAX,
                expected_completion_armed: true,
                ..base(vec![], 0, u16::MAX)
            },
            "completion_flag_blocks_duplicate_completion" => RevealCase {
                hold_before: 9_320,
                complete_before: true,
                ..base(vec![], 0, 8)
            },
            "single_word_advances_reveal_end" => RevealCase {
                rendered_widths: vec![7],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some([23, MENU_TOP]),
                expected_reveal_count: 2,
                expected_reveal_advanced: true,
                ..base(vec![word(a)], 1, 12)
            },
            "existing_hold_blocks_reveal_advance" => RevealCase {
                hold_before: 5,
                rendered_widths: vec![7],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some([23, MENU_TOP]),
                ..base(vec![word(a)], 1, 12)
            },
            "punctuation_peek_at_exclusive_end" => RevealCase {
                rendered_widths: vec![8],
                expected_cursor: Some([18, MENU_TOP]),
                expected_reveal_count: 2,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(comma)], 1, 4)
            },
            "visible_punctuation_uses_no_leading_gap" => RevealCase {
                rendered_widths: vec![8, 3],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some([27, MENU_TOP]),
                expected_reveal_count: 3,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(comma)], 2, 4)
            },
            "next_word_wraps_to_second_row" => RevealCase {
                rendered_widths: vec![280, 5],
                lookahead_widths: vec![10, u16::MAX - 1],
                expected_cursor: Some([21, MENU_TOP + MENU_ROW_HEIGHT]),
                expected_reveal_count: 3,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(b)], 2, 6)
            },
            "signed_layout_sum_avoids_false_wrap" => RevealCase {
                rendered_widths: vec![32_752, 5],
                lookahead_widths: vec![20, u16::MAX - 1],
                expected_cursor: Some([32_779, MENU_TOP]),
                expected_reveal_count: 3,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(b)], 2, 6)
            },
            "sentinel_before_end_completes_after_draw" => RevealCase {
                menu: vec![word(a), ScriptTextWord::SectionSeparator, word(b)],
                word_count: 5,
                delay: 8,
                rendered_widths: vec![7],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some([23, MENU_TOP]),
                expected_completion_armed: true,
                ..base(vec![], 3, 8)
            },
            "nonzero_dictionary_base_preserves_asymmetric_peek" => RevealCase {
                rendered_widths: vec![9, 4],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some(FLAT_NONZERO_BASE_CURSOR),
                expected_reveal_count: 3,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(comma)], 2, 7)
            },
            "split_data_segments_expose_shipped_alias" => RevealCase {
                rendered_widths: vec![6],
                lookahead_widths: vec![u16::MAX - 1],
                expected_cursor: Some(FLAT_SPLIT_STATE_CURSOR),
                expected_reveal_count: 2,
                expected_reveal_advanced: true,
                ..base(vec![word(a)], 1, 9)
            },
            "unsigned_cursor_wrap_stops_at_high_offset" => RevealCase {
                rendered_widths: vec![5],
                lookahead_widths: vec![12],
                expected_cursor: Some([21, MENU_TOP]),
                expected_reveal_count: 2,
                expected_reveal_advanced: true,
                ..base(vec![word(a), word(b)], 1, 3)
            },
            _ => panic!("unknown inline-menu reveal oracle {name}"),
        }
    }
}
