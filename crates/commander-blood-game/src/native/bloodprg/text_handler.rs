//! Flat runtime translation of the BloodScript A6 text handler.

use std::fmt;

use commander_blood_formats::instruction::{ScriptText, ScriptTextWord};
use commander_blood_formats::script::{ScriptDictionary, ScriptWordId};

use super::{
    evaluate_text_conditions, ScriptRuntime, ScriptWordHistory, TextConditionEffects,
    TextConditionError,
};

const SUBTITLE_LINE_LIMIT: u8 = 35;
const TEXT_YIELD_SIGNAL_INCREMENT: u8 = 2;
const CONDITION_YIELD_SIGNAL: u8 = 1;
const TEXT_REQUEST_PENDING: u8 = 1;
const CHARACTER_LENGTH_INCREMENT: u8 = 1;

/// Typed state replacing the active bit that the DOS handler changed in COD bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextInstructionState {
    active: bool,
}

impl TextInstructionState {
    /// Initialize mutable execution state from one decoded A6 instruction.
    pub const fn new(text: &ScriptText) -> Self {
        Self {
            active: text.control.is_active(),
        }
    }

    /// Return whether this authored line may still run.
    pub const fn is_active(self) -> bool {
        self.active
    }
}

/// Semantic class of the line record resolved before entering the handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextLineKind {
    /// Record accepted by the native handler's presentation-field gate.
    Presentation,
    /// Any other record class.
    Other,
}

/// Mutable state belonging to one resolved presentation line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextLineState {
    /// Semantic record class already resolved from the profile state.
    pub kind: TextLineKind,
    /// Whether this line has already published a presentation.
    pub already_shown: bool,
}

/// Bit flags consumed by the wider presentation scheduler.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationRequestFlags(u8);

impl PresentationRequestFlags {
    /// Decode preserved scheduler flags from a saved game or native oracle.
    pub const fn decode(bits: u8) -> Self {
        Self(bits)
    }

    /// Return the complete preserved flag byte.
    pub const fn bits(self) -> u8 {
        self.0
    }

    fn request_text(&mut self) {
        self.0 |= TEXT_REQUEST_PENDING;
    }
}

/// Flat presentation state shared with audio, subtitle, and menu systems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPresentationState {
    /// A subtitle is currently being revealed or held.
    pub subtitle_display_active: bool,
    /// A raw menu-word presentation is waiting for progressive reveal.
    pub menu_deferred: bool,
    /// The surrounding presentation owner has entered its hold phase.
    pub hold_ready: bool,
    /// Scheduler request bits, including the text-presentation request.
    pub request_flags: PresentationRequestFlags,
    /// Signal interpreted by the outer BloodScript dispatcher after this handler.
    pub yield_signal: u8,
    /// The accepted word list should be assembled into subtitle text.
    pub subtitle_word_list_mode: bool,
    /// Signed selector identifying the accepted dialogue line.
    pub selected_line: Option<i8>,
    /// Audio should choose a short voice reaction for the subtitle.
    pub subtitle_voice_trigger: bool,
    /// Audio should derive its chatter seed from the current menu words.
    pub dialogue_chatter_seed_pending: bool,
    /// Seeded dialogue chatter is currently eligible for playback.
    pub dialogue_chatter_active: bool,
    /// Number of subtitle bytes already exposed by the reveal animation.
    pub subtitle_reveal_cursor: usize,
    /// The main presentation loop has not yet consumed the new menu.
    pub menu_pending: bool,
    /// Number of leading menu words before the first section separator.
    pub menu_word_count: usize,
    /// Number of menu words currently exposed by progressive reveal.
    pub menu_reveal_count: usize,
    /// Post-condition concept words published by the resume control.
    pub condition_presentation_words: Box<[ScriptWordId]>,
    /// Subtitle bytes with carriage-return line separators and no C terminator.
    pub subtitle_text: Box<[u8]>,
    /// Interned words and authored section separators used by the menu renderer.
    pub menu_words: Box<[ScriptTextWord]>,
}

impl Default for TextPresentationState {
    fn default() -> Self {
        Self {
            subtitle_display_active: false,
            menu_deferred: false,
            hold_ready: false,
            request_flags: PresentationRequestFlags::default(),
            yield_signal: u8::MIN,
            subtitle_word_list_mode: false,
            selected_line: None,
            subtitle_voice_trigger: false,
            dialogue_chatter_seed_pending: false,
            dialogue_chatter_active: false,
            subtitle_reveal_cursor: usize::MIN,
            menu_pending: false,
            menu_word_count: usize::MIN,
            menu_reveal_count: usize::MIN,
            condition_presentation_words: Box::new([]),
            subtitle_text: Box::new([]),
            menu_words: Box::new([]),
        }
    }
}

/// Already-resolved data used by optional A6 conditions.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextConditionInputs<'a> {
    /// Result of the native modulo-five random draw, when requested.
    pub random_result: Option<u16>,
    /// Value of the typed line-record field selected by the control word.
    pub record_value: Option<u16>,
    /// Recent interned concept-word history, when requested.
    pub history: Option<&'a ScriptWordHistory>,
}

/// Reason an A6 instruction was ignored before evaluating its conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextHandlerGate {
    /// Its mutable authored active state is clear.
    Inactive,
    /// Another subtitle currently owns the display.
    SubtitleActive,
    /// A menu presentation is already deferred.
    MenuDeferred,
    /// This line has already been shown.
    AlreadyShown,
    /// The resolved state record is not a presentation line.
    WrongLineKind,
}

/// Semantic result of one complete A6 handler call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextHandlerOutcome {
    /// The handler stopped at a precondition gate.
    Gated(TextHandlerGate),
    /// Optional random, record, or history conditions rejected the line.
    ConditionRejected,
    /// A subtitle payload was assembled and published.
    SubtitlePublished,
    /// A raw menu-word payload was published.
    MenuPublished,
}

/// Invalid typed state supplied to an A6 instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextHandlerError {
    /// The resume flag was present without its decoded COD destination.
    MissingResumeTarget,
    /// An optional text condition lacked required typed input.
    Condition(TextConditionError),
    /// A word identity did not belong to the supplied dictionary.
    UnknownDictionaryWord(ScriptWordId),
}

impl fmt::Display for TextHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TextHandlerError {}

impl From<TextConditionError> for TextHandlerError {
    fn from(error: TextConditionError) -> Self {
        Self::Condition(error)
    }
}

/// Apply the recovered `vm_op_a6_text` behavior to typed flat runtime state.
pub fn handle_text_instruction(
    text: &ScriptText,
    instruction_state: &mut TextInstructionState,
    line: &mut TextLineState,
    dictionary: &ScriptDictionary,
    script: &mut ScriptRuntime,
    presentation: &mut TextPresentationState,
    conditions: TextConditionInputs<'_>,
) -> Result<TextHandlerOutcome, TextHandlerError> {
    if let Some(skip_count) = text.control.rejection_skip_count() {
        script.arm_skip(skip_count);
    }
    if text.control.arms_resume() {
        let target = text
            .resume_target
            .ok_or(TextHandlerError::MissingResumeTarget)?;
        script.arm_resume(target, u16::MIN);
    }

    if !instruction_state.active {
        return Ok(TextHandlerOutcome::Gated(TextHandlerGate::Inactive));
    }
    if presentation.subtitle_display_active {
        return Ok(TextHandlerOutcome::Gated(TextHandlerGate::SubtitleActive));
    }
    if presentation.menu_deferred {
        return Ok(TextHandlerOutcome::Gated(TextHandlerGate::MenuDeferred));
    }
    if line.already_shown {
        return Ok(TextHandlerOutcome::Gated(TextHandlerGate::AlreadyShown));
    }
    if line.kind != TextLineKind::Presentation {
        return Ok(TextHandlerOutcome::Gated(TextHandlerGate::WrongLineKind));
    }

    let mut condition_effects = TextConditionEffects::default();
    if !evaluate_text_conditions(
        text,
        conditions.random_result,
        conditions.record_value,
        conditions.history,
        &mut condition_effects,
    )? {
        return Ok(TextHandlerOutcome::ConditionRejected);
    }

    let subtitle_mode = presentation.subtitle_word_list_mode || condition_effects.spoken_word_mode;
    let subtitle = subtitle_mode
        .then(|| assemble_subtitle(&text.words, dictionary))
        .transpose()?;

    presentation.selected_line = Some(text.presentation_selector);
    if !text.control.preserves_active() {
        instruction_state.active = false;
    }
    if condition_effects.spoken_word_mode {
        presentation.subtitle_word_list_mode = true;
    }
    if condition_effects.yield_requested {
        presentation.yield_signal = CONDITION_YIELD_SIGNAL;
        presentation.condition_presentation_words =
            condition_effects.presentation_words.into_boxed_slice();
    }
    line.already_shown = true;

    if let Some(subtitle) = subtitle {
        presentation.subtitle_voice_trigger = true;
        presentation.dialogue_chatter_active = false;
        presentation.menu_deferred = false;
        presentation.subtitle_word_list_mode = false;
        presentation.subtitle_display_active = true;
        presentation.subtitle_reveal_cursor = usize::MIN;
        presentation.yield_signal = presentation
            .yield_signal
            .wrapping_add(TEXT_YIELD_SIGNAL_INCREMENT);
        presentation.hold_ready = false;
        presentation.request_flags.request_text();
        presentation.subtitle_text = subtitle;
        Ok(TextHandlerOutcome::SubtitlePublished)
    } else {
        let menu_words = text.words.clone();
        presentation.subtitle_display_active = false;
        presentation.dialogue_chatter_seed_pending = true;
        presentation.request_flags.request_text();
        presentation.yield_signal = presentation
            .yield_signal
            .wrapping_add(TEXT_YIELD_SIGNAL_INCREMENT);
        presentation.menu_deferred = true;
        presentation.hold_ready = false;
        presentation.menu_pending = true;
        presentation.menu_word_count = menu_words
            .iter()
            .take_while(|word| matches!(word, ScriptTextWord::Dictionary(_)))
            .count();
        presentation.menu_reveal_count = usize::MIN;
        presentation.menu_words = menu_words;
        Ok(TextHandlerOutcome::MenuPublished)
    }
}

fn assemble_subtitle(
    words: &[ScriptTextWord],
    dictionary: &ScriptDictionary,
) -> Result<Box<[u8]>, TextHandlerError> {
    let spoken_words: Vec<ScriptWordId> = words
        .iter()
        .take_while(|word| matches!(word, ScriptTextWord::Dictionary(_)))
        .filter_map(|word| match word {
            ScriptTextWord::Dictionary(word) => Some(*word),
            ScriptTextWord::SectionSeparator => None,
        })
        .collect();
    let mut output = Vec::new();
    let mut line_length = u8::MIN;

    for (index, word) in spoken_words.iter().copied().enumerate() {
        let bytes = dictionary
            .word(word)
            .ok_or(TextHandlerError::UnknownDictionaryWord(word))?;
        output.extend_from_slice(bytes);
        for _ in bytes {
            line_length = line_length.wrapping_add(CHARACTER_LENGTH_INCREMENT);
        }

        let next_bytes = match spoken_words.get(index + 1).copied() {
            Some(next_word) => dictionary
                .word(next_word)
                .ok_or(TextHandlerError::UnknownDictionaryWord(next_word))?,
            None => &[],
        };
        if next_bytes
            .first()
            .is_some_and(|byte| is_attached_punctuation(*byte))
        {
            continue;
        }

        output.push(b' ');
        line_length = line_length.wrapping_add(CHARACTER_LENGTH_INCREMENT);
        let next_length = next_bytes.len() as u8;
        if next_length.wrapping_add(line_length) >= SUBTITLE_LINE_LIMIT {
            line_length = u8::MIN;
            output.push(b'\r');
        }
    }

    output.push(b'\r');
    Ok(output.into_boxed_slice())
}

const fn is_attached_punctuation(byte: u8) -> bool {
    matches!(byte, b',' | b'.' | b'?' | b'!' | b':')
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::instruction::{ScriptLineRecordOffset, ScriptTextControl};
    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;

    const INITIAL_SKIP_COUNT: u8 = 0x55;
    const INITIAL_RESUME_TARGET: usize = 0x9ABC;
    const INITIAL_RESUME_VALUE: u16 = 0x5678;
    const INITIAL_REQUEST_FLAGS: u8 = 0xA0;
    const INITIAL_YIELD_SIGNAL: u8 = 0x40;
    const INITIAL_REVEAL_CURSOR: usize = 0x7777;

    #[derive(Deserialize)]
    struct TextHandlerOracle {
        name: String,
        path: String,
        selector: i8,
        control_word: u16,
        line_flags_before: u16,
        line_flags_after: u16,
        token_b5_after: u8,
        condition_called: bool,
    }

    fn dictionary() -> ScriptDictionary {
        let mut bytes = vec![u8::MIN; usize::from(u16::MAX) + 1];
        for (offset, word) in [
            (0x0100, b"HELLO\0".as_slice()),
            (0x0110, b",\0".as_slice()),
            (0x0120, b"WORLD\0".as_slice()),
            (0x0200, b"12345678901234567890\0".as_slice()),
            (0x0220, b"abcdefghijklmnop\0".as_slice()),
            (0x0300, b"CHOICE\0".as_slice()),
            (0x0320, b"MENU\0".as_slice()),
        ] {
            bytes[offset..offset + word.len()].copy_from_slice(word);
        }
        decode_script_dictionary(&bytes).unwrap()
    }

    fn dictionary_word(dictionary: &ScriptDictionary, offset: u16) -> ScriptTextWord {
        ScriptTextWord::Dictionary(dictionary.resolve_source_offset(offset).unwrap())
    }

    fn instruction_words(name: &str, dictionary: &ScriptDictionary) -> Vec<ScriptTextWord> {
        let word = |offset| dictionary_word(dictionary, offset);
        match name {
            "inactive_still_arms_skip_and_loop"
            | "random_condition_rejects"
            | "accepted_raw_word_list" => vec![word(0x0100), word(0x0200)],
            "display_active_gate"
            | "presentation_defer_gate"
            | "already_shown_gate"
            | "wrong_presentation_record_gate"
            | "accepted_preserved_with_extra_control" => vec![word(0x0100)],
            "assembled_punctuation_spacing" => {
                vec![word(0x0100), word(0x0110), word(0x0120)]
            }
            "assembled_wraps_before_next_word" => vec![word(0x0200), word(0x0220)],
            "assembled_stops_at_menu_separator" => {
                vec![word(0x0300), ScriptTextWord::SectionSeparator, word(0x0320)]
            }
            _ => panic!("unknown text-handler oracle {name}"),
        }
    }

    fn instruction(vector: &TextHandlerOracle, dictionary: &ScriptDictionary) -> ScriptText {
        let control = ScriptTextControl::decode(vector.control_word);
        ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: vector.selector,
            control,
            resume_target: control
                .arms_resume()
                .then_some(ScriptCodeOffset::new(0x3333)),
            record_condition_operand: (vector.name == "accepted_preserved_with_extra_control")
                .then_some(5),
            words: instruction_words(&vector.name, dictionary).into_boxed_slice(),
        }
    }

    fn expected_gate(path: &str) -> Option<TextHandlerGate> {
        match path {
            "inactive" => Some(TextHandlerGate::Inactive),
            "display_gate" => Some(TextHandlerGate::SubtitleActive),
            "defer_gate" => Some(TextHandlerGate::MenuDeferred),
            "shown" => Some(TextHandlerGate::AlreadyShown),
            "wrong_record" => Some(TextHandlerGate::WrongLineKind),
            _ => None,
        }
    }

    fn expected_subtitle(name: &str) -> Option<&'static [u8]> {
        match name {
            "assembled_punctuation_spacing" => Some(b"HELLO, WORLD \r"),
            "assembled_wraps_before_next_word" => {
                Some(b"12345678901234567890 \rabcdefghijklmnop \r")
            }
            "assembled_stops_at_menu_separator" => Some(b"CHOICE \r"),
            _ => None,
        }
    }

    #[test]
    fn every_original_a6_vector_matches_flat_typed_state() {
        let dictionary = dictionary();
        let vectors: Vec<TextHandlerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_660c_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let text = instruction(&vector, &dictionary);
            let expected_menu_words = text.words.clone();
            let mut instruction_state = TextInstructionState::new(&text);
            let mut line = TextLineState {
                kind: if vector.path == "wrong_record" {
                    TextLineKind::Other
                } else {
                    TextLineKind::Presentation
                },
                already_shown: vector.line_flags_before & 0x8000 != u16::MIN,
            };
            let mut script = ScriptRuntime::new();
            script.arm_skip(INITIAL_SKIP_COUNT);
            script.arm_resume(
                ScriptCodeOffset::new(INITIAL_RESUME_TARGET),
                INITIAL_RESUME_VALUE,
            );
            let mut presentation = TextPresentationState {
                subtitle_display_active: vector.path == "display_gate",
                menu_deferred: vector.path == "defer_gate",
                hold_ready: true,
                request_flags: PresentationRequestFlags::decode(INITIAL_REQUEST_FLAGS),
                yield_signal: INITIAL_YIELD_SIGNAL,
                subtitle_reveal_cursor: INITIAL_REVEAL_CURSOR,
                menu_pending: true,
                ..TextPresentationState::default()
            };
            let conditions = TextConditionInputs {
                random_result: (vector.path == "random_reject").then_some(1),
                record_value: (vector.path == "raw_extra").then_some(10),
                history: None,
            };

            let outcome = handle_text_instruction(
                &text,
                &mut instruction_state,
                &mut line,
                &dictionary,
                &mut script,
                &mut presentation,
                conditions,
            )
            .unwrap();

            let condition_called = !matches!(outcome, TextHandlerOutcome::Gated(_));
            assert_eq!(condition_called, vector.condition_called, "{}", vector.name);
            assert_eq!(
                instruction_state.is_active(),
                vector.token_b5_after & 0x80 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                line.already_shown,
                vector.line_flags_after & 0x8000 != u16::MIN,
                "{}",
                vector.name
            );

            if let Some(gate) = expected_gate(&vector.path) {
                assert_eq!(outcome, TextHandlerOutcome::Gated(gate), "{}", vector.name);
            } else if vector.path == "random_reject" {
                assert_eq!(outcome, TextHandlerOutcome::ConditionRejected);
            } else if vector.path == "assembled" {
                assert_eq!(outcome, TextHandlerOutcome::SubtitlePublished);
            } else {
                assert_eq!(outcome, TextHandlerOutcome::MenuPublished);
            }

            if vector.name == "inactive_still_arms_skip_and_loop" {
                assert_eq!(script.pending_skip_count(), Some(8));
                let resume = script.resume_state().unwrap();
                assert_eq!(resume.target, ScriptCodeOffset::new(0x3333));
                assert_eq!(resume.value, u16::MIN);
            } else {
                assert_eq!(script.pending_skip_count(), Some(INITIAL_SKIP_COUNT));
                let resume = script.resume_state().unwrap();
                assert_eq!(resume.target, ScriptCodeOffset::new(INITIAL_RESUME_TARGET));
                assert_eq!(resume.value, INITIAL_RESUME_VALUE);
            }

            let accepted = matches!(
                outcome,
                TextHandlerOutcome::SubtitlePublished | TextHandlerOutcome::MenuPublished
            );
            assert_eq!(
                presentation.selected_line,
                accepted.then_some(vector.selector),
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.request_flags.bits(),
                INITIAL_REQUEST_FLAGS | u8::from(accepted),
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.yield_signal,
                if accepted {
                    INITIAL_YIELD_SIGNAL.wrapping_add(TEXT_YIELD_SIGNAL_INCREMENT)
                } else {
                    INITIAL_YIELD_SIGNAL
                },
                "{}",
                vector.name
            );
            assert_eq!(presentation.hold_ready, !accepted, "{}", vector.name);

            if let Some(expected) = expected_subtitle(&vector.name) {
                assert!(presentation.subtitle_display_active);
                assert!(!presentation.menu_deferred);
                assert!(presentation.subtitle_voice_trigger);
                assert_eq!(presentation.subtitle_reveal_cursor, usize::MIN);
                assert_eq!(presentation.subtitle_text.as_ref(), expected);
                assert!(presentation.menu_words.is_empty());
            } else if accepted {
                assert!(!presentation.subtitle_display_active);
                assert!(presentation.menu_deferred);
                assert!(presentation.dialogue_chatter_seed_pending);
                assert!(presentation.menu_pending);
                assert_eq!(presentation.menu_reveal_count, usize::MIN);
                assert_eq!(
                    presentation.menu_word_count,
                    expected_menu_words
                        .iter()
                        .take_while(|word| matches!(word, ScriptTextWord::Dictionary(_)))
                        .count()
                );
                assert_eq!(presentation.menu_words, expected_menu_words);
                assert!(presentation.subtitle_text.is_empty());
            } else {
                assert!(presentation.subtitle_text.is_empty(), "{}", vector.name);
                assert!(presentation.menu_words.is_empty(), "{}", vector.name);
                assert_eq!(
                    presentation.subtitle_reveal_cursor, INITIAL_REVEAL_CURSOR,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn missing_typed_inputs_fail_without_address_fallbacks() {
        let dictionary = dictionary();
        let text = ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 0,
            control: ScriptTextControl::decode(0x8010),
            resume_target: None,
            record_condition_operand: None,
            words: Box::new([]),
        };
        let mut instruction_state = TextInstructionState::new(&text);
        let mut line = TextLineState {
            kind: TextLineKind::Presentation,
            already_shown: false,
        };

        assert_eq!(
            handle_text_instruction(
                &text,
                &mut instruction_state,
                &mut line,
                &dictionary,
                &mut ScriptRuntime::new(),
                &mut TextPresentationState::default(),
                TextConditionInputs::default(),
            )
            .unwrap_err(),
            TextHandlerError::MissingResumeTarget
        );
    }

    #[test]
    fn subtitle_and_menu_buffers_remain_independent() {
        let dictionary = dictionary();
        let prior_menu = vec![dictionary_word(&dictionary, 0x0300)].into_boxed_slice();
        let spoken = ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 1,
            control: ScriptTextControl::decode(0x8020),
            resume_target: None,
            record_condition_operand: None,
            words: vec![dictionary_word(&dictionary, 0x0100)].into_boxed_slice(),
        };
        let mut spoken_state = TextPresentationState {
            menu_words: prior_menu.clone(),
            ..TextPresentationState::default()
        };
        handle_text_instruction(
            &spoken,
            &mut TextInstructionState::new(&spoken),
            &mut TextLineState {
                kind: TextLineKind::Presentation,
                already_shown: false,
            },
            &dictionary,
            &mut ScriptRuntime::new(),
            &mut spoken_state,
            TextConditionInputs::default(),
        )
        .unwrap();
        assert_eq!(spoken_state.menu_words, prior_menu);
        assert_eq!(spoken_state.subtitle_text.as_ref(), b"HELLO \r");

        let prior_subtitle: Box<[u8]> = Box::from(b"PRIOR \r".as_slice());
        let menu = ScriptText {
            line_record: ScriptLineRecordOffset::decode(0),
            presentation_selector: 2,
            control: ScriptTextControl::decode(0x8000),
            resume_target: None,
            record_condition_operand: None,
            words: vec![dictionary_word(&dictionary, 0x0320)].into_boxed_slice(),
        };
        let mut menu_state = TextPresentationState {
            subtitle_text: prior_subtitle.clone(),
            ..TextPresentationState::default()
        };
        handle_text_instruction(
            &menu,
            &mut TextInstructionState::new(&menu),
            &mut TextLineState {
                kind: TextLineKind::Presentation,
                already_shown: false,
            },
            &dictionary,
            &mut ScriptRuntime::new(),
            &mut menu_state,
            TextConditionInputs::default(),
        )
        .unwrap();
        assert_eq!(menu_state.subtitle_text, prior_subtitle);
        assert_eq!(
            menu_state.menu_words.as_ref(),
            [dictionary_word(&dictionary, 0x0320)]
        );
    }
}
