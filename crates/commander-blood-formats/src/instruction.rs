//! Semantic BloodScript instructions decoded from losslessly framed COD tokens.

use std::fmt;

use crate::code::{ScriptCodeOffset, ScriptDecodingMode, ScriptOpcode, ScriptToken};
use crate::script::{ScriptDictionary, ScriptWordId};

const GUARD_BEGIN_OPCODE: u8 = 0xA0;
const GUARD_END_OPCODE: u8 = 0xA1;
const RANDOM_GUARD_OPCODE: u8 = 0xA2;
const CONCEPT_GUARD_OPCODE: u8 = 0xA3;
const JUMP_OPCODE: u8 = 0xA4;
const TIMER_STATE_OPCODE: u8 = 0xA5;
const TEXT_OPCODE: u8 = 0xA6;
const INVERTED_CONDITION_PREFIX: u8 = GUARD_END_OPCODE;
const OPCODE_SIZE: usize = 1;
const BYTE_SIZE: usize = 1;
const WORD_SIZE: usize = 2;
const GUARD_BEGIN_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const GUARD_END_SIZE: usize = OPCODE_SIZE;
const RANDOM_GUARD_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const CONCEPT_GUARD_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const INVERTED_CONCEPT_GUARD_SIZE: usize = CONCEPT_GUARD_SIZE + BYTE_SIZE;
const JUMP_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const TIMER_GUARD_SIZE: usize = OPCODE_SIZE + BYTE_SIZE;
const TIMER_ASSIGNMENT_SIZE: usize = TIMER_GUARD_SIZE + WORD_SIZE;
const TIMER_SLOT_COUNT: u8 = 128;
const TEXT_FIXED_HEADER_SIZE: usize = OPCODE_SIZE + WORD_SIZE + BYTE_SIZE + WORD_SIZE;
const TEXT_PRESERVE_ACTIVE: u16 = 0x0001;
const TEXT_RANDOM_GATE: u16 = 0x0002;
const TEXT_RECORD_CONDITION: u16 = 0x0004;
const TEXT_CONDITIONAL_SKIP: u16 = 0x0008;
const TEXT_RESUME_AND_POST_WORDS: u16 = 0x0010;
const TEXT_SPOKEN_WORDS: u16 = 0x0020;
const TEXT_HISTORY_CONDITION: u16 = 0x0040;
const TEXT_ACTIVE: u16 = 0x8000;
const TEXT_SKIP_COUNT_SHIFT: u32 = 12;
const TEXT_SKIP_COUNT_MASK: u16 = 0x0007;
const TEXT_WORD_SECTION_SEPARATOR: u16 = u16::MAX;
const TEXT_WORD_TERMINATOR: u16 = u16::MIN;

/// Index in the 128-word transient countdown/state table saved with the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptTimerSlot(u8);

impl ScriptTimerSlot {
    /// Number of words in the transient countdown/state table.
    pub const COUNT: usize = TIMER_SLOT_COUNT as usize;

    /// Decode a slot in the table's proven nonnegative domain.
    pub const fn decode(encoded: u8) -> Option<Self> {
        if encoded < TIMER_SLOT_COUNT {
            Some(Self(encoded))
        } else {
            None
        }
    }

    /// Return the zero-based slot index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Byte offset of one line record within the profile's owned VAR state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptLineRecordOffset(u16);

impl ScriptLineRecordOffset {
    /// Decode an authored line-record byte offset.
    pub const fn decode(encoded: u16) -> Self {
        Self(encoded)
    }

    /// Return the encoded byte offset.
    pub const fn byte_offset(self) -> usize {
        self.0 as usize
    }
}

/// Recovered control flags carried by one A6 text instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTextControl(u16);

impl ScriptTextControl {
    /// Decode an authored A6 control word.
    pub const fn decode(encoded: u16) -> Self {
        Self(encoded)
    }

    /// Return the exact encoded flag word.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Return the high detail byte used by field and history conditions.
    pub const fn detail(self) -> u8 {
        (self.0 >> u8::BITS) as u8
    }

    /// Return whether accepting the line leaves its active bit set.
    pub const fn preserves_active(self) -> bool {
        self.0 & TEXT_PRESERVE_ACTIVE != u16::MIN
    }

    /// Return whether the line passes only on a zero PRNG result modulo five.
    pub const fn uses_random_gate(self) -> bool {
        self.0 & TEXT_RANDOM_GATE != u16::MIN
    }

    /// Return whether a record-field comparison operand precedes the word list.
    pub const fn uses_record_condition(self) -> bool {
        self.0 & TEXT_RECORD_CONDITION != u16::MIN
    }

    /// Return the number of following tokens skipped when the line is rejected.
    pub const fn rejection_skip_count(self) -> Option<u8> {
        if self.0 & TEXT_CONDITIONAL_SKIP == u16::MIN {
            None
        } else {
            Some((((self.0 >> TEXT_SKIP_COUNT_SHIFT) & TEXT_SKIP_COUNT_MASK) + 1) as u8)
        }
    }

    /// Return whether a resume target precedes the word list.
    pub const fn arms_resume(self) -> bool {
        self.0 & TEXT_RESUME_AND_POST_WORDS != u16::MIN
    }

    /// Return whether accepted words are assembled as spoken subtitle text.
    pub const fn emits_spoken_text(self) -> bool {
        self.0 & TEXT_SPOKEN_WORDS != u16::MIN
    }

    /// Return whether word-history conditions are evaluated around a separator.
    pub const fn uses_history_condition(self) -> bool {
        self.0 & TEXT_HISTORY_CONDITION != u16::MIN
    }

    /// Return whether this authored line is currently eligible for display.
    pub const fn is_active(self) -> bool {
        self.0 & TEXT_ACTIVE != u16::MIN
    }
}

/// One semantic entry in an A6 instruction's terminated word list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTextWord {
    /// Interned word from the companion DIC image.
    Dictionary(ScriptWordId),
    /// Authored `0xFFFF` boundary between spoken, condition, or menu sections.
    SectionSeparator,
}

/// Complete typed structure of one A6 text/presentation instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptText {
    /// Line-record byte offset used for shown-state and presentation gating.
    pub line_record: ScriptLineRecordOffset,
    /// Signed selector stored by the native visual-presentation path.
    pub presentation_selector: i8,
    /// Recovered text and condition flags.
    pub control: ScriptTextControl,
    /// Optional destination armed for resumed execution.
    pub resume_target: Option<ScriptCodeOffset>,
    /// Optional record-field comparison operand consumed before dictionary words.
    pub record_condition_operand: Option<u16>,
    /// Interned dictionary words and explicit authored section boundaries.
    pub words: Box<[ScriptTextWord]>,
}

/// Typed instruction semantics for the first recovered VM control family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptInstruction {
    /// Enter query mode and push the target used when a nested guard fails.
    GuardBegin {
        /// Branch destination encoded by the script.
        failure_target: ScriptCodeOffset,
    },
    /// Leave query mode and discard the current nested guard target.
    GuardEnd,
    /// Continue only when the native random result for this modulus is zero.
    RandomGuard {
        /// Modulus passed to Commander Blood's recovered PRNG.
        modulus: u16,
    },
    /// Compare the selected concept with one interned dictionary word.
    ConceptGuard {
        /// Required concept identity.
        expected: ScriptWordId,
        /// Whether equality fails instead of succeeds.
        inverted: bool,
    },
    /// Jump directly and clear pending resume state.
    Jump {
        /// Destination in the same COD source image.
        target: ScriptCodeOffset,
    },
    /// Continue only while one transient timer/state word is zero.
    TimerGuard {
        /// Word tested by the guard.
        slot: ScriptTimerSlot,
    },
    /// Assign one transient timer/state word.
    TimerAssignment {
        /// Word receiving the value.
        slot: ScriptTimerSlot,
        /// Authored value.
        value: u16,
    },
}

/// Failure while converting a framed token into known instruction semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptInstructionError {
    /// The opcode's native handler has not yet been translated into this IR.
    UntranslatedOpcode {
        /// Opcode awaiting translation.
        opcode: ScriptOpcode,
    },
    /// A framed token has the wrong byte count for its selected semantic form.
    InvalidOperandLength {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Token opcode.
        opcode: ScriptOpcode,
        /// Required total byte count.
        expected: usize,
        /// Actual total byte count.
        actual: usize,
    },
    /// A concept operand does not begin an entry in the companion dictionary.
    InvalidDictionaryOffset {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Encoded dictionary byte position.
        dictionary_offset: u16,
    },
    /// An A5 token uses a signed negative index outside the actual state table.
    InvalidTimerSlot {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Original signed index.
        encoded: i8,
    },
    /// An A6 token's optional controls and terminated word list are inconsistent.
    MalformedText {
        /// Token position.
        source_offset: ScriptCodeOffset,
    },
}

impl fmt::Display for ScriptInstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptInstructionError {}

/// Decode one framed token using recovered native handler semantics.
///
/// Opcodes whose handlers have not reached the typed Rust port return an
/// explicit error; no raw instruction silently executes as a no-op.
pub fn decode_script_instruction(
    token: &ScriptToken,
    dictionary: &ScriptDictionary,
) -> Result<ScriptInstruction, ScriptInstructionError> {
    let bytes = token.encoded_bytes();
    match token.opcode().byte() {
        GUARD_BEGIN_OPCODE => {
            require_size(token, GUARD_BEGIN_SIZE)?;
            Ok(ScriptInstruction::GuardBegin {
                failure_target: ScriptCodeOffset::new(usize::from(read_word(bytes, OPCODE_SIZE))),
            })
        }
        GUARD_END_OPCODE => {
            require_size(token, GUARD_END_SIZE)?;
            Ok(ScriptInstruction::GuardEnd)
        }
        RANDOM_GUARD_OPCODE => {
            require_size(token, RANDOM_GUARD_SIZE)?;
            Ok(ScriptInstruction::RandomGuard {
                modulus: read_word(bytes, OPCODE_SIZE),
            })
        }
        CONCEPT_GUARD_OPCODE => decode_concept_guard(token, dictionary),
        JUMP_OPCODE => {
            require_size(token, JUMP_SIZE)?;
            Ok(ScriptInstruction::Jump {
                target: ScriptCodeOffset::new(usize::from(read_word(bytes, OPCODE_SIZE))),
            })
        }
        TIMER_STATE_OPCODE => decode_timer_state(token),
        _ => Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        }),
    }
}

/// Decode the complete authored structure of one A6 text token.
pub fn decode_script_text(
    token: &ScriptToken,
    dictionary: &ScriptDictionary,
) -> Result<ScriptText, ScriptInstructionError> {
    if token.opcode().byte() != TEXT_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    if bytes.len() < TEXT_FIXED_HEADER_SIZE + WORD_SIZE {
        return Err(ScriptInstructionError::MalformedText {
            source_offset: token.source_offset(),
        });
    }

    let line_record = ScriptLineRecordOffset::decode(read_word(bytes, OPCODE_SIZE));
    let presentation_selector = bytes[OPCODE_SIZE + WORD_SIZE] as i8;
    let control = ScriptTextControl::decode(read_word(bytes, OPCODE_SIZE + WORD_SIZE + BYTE_SIZE));
    let mut cursor = TEXT_FIXED_HEADER_SIZE;
    let resume_target = if control.arms_resume() {
        let target = read_text_word(token, cursor)?;
        cursor += WORD_SIZE;
        Some(ScriptCodeOffset::new(usize::from(target)))
    } else {
        None
    };
    let record_condition_operand = if control.uses_record_condition() {
        let operand = read_text_word(token, cursor)?;
        cursor += WORD_SIZE;
        Some(operand)
    } else {
        None
    };

    let mut words = Vec::new();
    loop {
        let word = read_text_word(token, cursor)?;
        cursor += WORD_SIZE;
        if word == TEXT_WORD_TERMINATOR {
            if cursor != bytes.len() {
                return Err(ScriptInstructionError::MalformedText {
                    source_offset: token.source_offset(),
                });
            }
            break;
        }
        if word == TEXT_WORD_SECTION_SEPARATOR {
            words.push(ScriptTextWord::SectionSeparator);
            continue;
        }
        let dictionary_word = dictionary.resolve_source_offset(word).ok_or(
            ScriptInstructionError::InvalidDictionaryOffset {
                source_offset: token.source_offset(),
                dictionary_offset: word,
            },
        )?;
        words.push(ScriptTextWord::Dictionary(dictionary_word));
    }

    Ok(ScriptText {
        line_record,
        presentation_selector,
        control,
        resume_target,
        record_condition_operand,
        words: words.into_boxed_slice(),
    })
}

fn decode_concept_guard(
    token: &ScriptToken,
    dictionary: &ScriptDictionary,
) -> Result<ScriptInstruction, ScriptInstructionError> {
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let expected_size = if inverted {
        INVERTED_CONCEPT_GUARD_SIZE
    } else {
        CONCEPT_GUARD_SIZE
    };
    require_size(token, expected_size)?;
    let operand_offset = OPCODE_SIZE + usize::from(inverted);
    let dictionary_offset = read_word(bytes, operand_offset);
    let expected = dictionary.resolve_source_offset(dictionary_offset).ok_or(
        ScriptInstructionError::InvalidDictionaryOffset {
            source_offset: token.source_offset(),
            dictionary_offset,
        },
    )?;
    Ok(ScriptInstruction::ConceptGuard { expected, inverted })
}

fn decode_timer_state(token: &ScriptToken) -> Result<ScriptInstruction, ScriptInstructionError> {
    let expected_size = match token.mode_before() {
        ScriptDecodingMode::Normal => TIMER_ASSIGNMENT_SIZE,
        ScriptDecodingMode::Query => TIMER_GUARD_SIZE,
    };
    require_size(token, expected_size)?;
    let encoded = token.encoded_bytes()[OPCODE_SIZE] as i8;
    let slot = ScriptTimerSlot::decode(encoded as u8)
        .filter(|_| encoded >= 0)
        .ok_or(ScriptInstructionError::InvalidTimerSlot {
            source_offset: token.source_offset(),
            encoded,
        })?;
    match token.mode_before() {
        ScriptDecodingMode::Normal => Ok(ScriptInstruction::TimerAssignment {
            slot,
            value: read_word(token.encoded_bytes(), OPCODE_SIZE + BYTE_SIZE),
        }),
        ScriptDecodingMode::Query => Ok(ScriptInstruction::TimerGuard { slot }),
    }
}

fn require_size(token: &ScriptToken, expected: usize) -> Result<(), ScriptInstructionError> {
    let actual = token.encoded_bytes().len();
    if actual == expected {
        Ok(())
    } else {
        Err(ScriptInstructionError::InvalidOperandLength {
            source_offset: token.source_offset(),
            opcode: token.opcode(),
            expected,
            actual,
        })
    }
}

fn read_text_word(token: &ScriptToken, offset: usize) -> Result<u16, ScriptInstructionError> {
    if offset.saturating_add(WORD_SIZE) > token.encoded_bytes().len() {
        Err(ScriptInstructionError::MalformedText {
            source_offset: token.source_offset(),
        })
    } else {
        Ok(read_word(token.encoded_bytes(), offset))
    }
}

fn read_word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        data[offset..offset + WORD_SIZE]
            .try_into()
            .expect("validated instruction operands"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::code::decode_script_code;
    use crate::script::decode_script_dictionary;

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const CODE_END_MARKER: u8 = 0xFF;
    const EXPECTED_CONTROL_INSTRUCTION_COUNTS: [usize; PROFILE_COUNT] = [27, 782, 766, 318, 392];
    const EXPECTED_TEXT_COUNTS: [usize; PROFILE_COUNT] = [111, 1_157, 1_048, 719, 652];

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn every_shipped_a0_through_a5_token_has_typed_semantics() {
        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let dictionary_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
            let decoded = code
                .tokens()
                .iter()
                .filter(|token| {
                    (GUARD_BEGIN_OPCODE..=TIMER_STATE_OPCODE).contains(&token.opcode().byte())
                })
                .map(|token| decode_script_instruction(token, &dictionary))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(
                decoded.len(),
                EXPECTED_CONTROL_INSTRUCTION_COUNTS[profile - 1]
            );
        }
    }

    #[test]
    fn signed_indices_outside_the_state_table_are_rejected() {
        let token_data = [TIMER_STATE_OPCODE, u8::MAX, 1, 0, CODE_END_MARKER];
        let code = decode_script_code(&token_data).unwrap();
        let dictionary = decode_script_dictionary(&[u8::MIN]).unwrap();
        assert_eq!(
            decode_script_instruction(&code.tokens()[0], &dictionary).unwrap_err(),
            ScriptInstructionError::InvalidTimerSlot {
                source_offset: ScriptCodeOffset::new(0),
                encoded: -1,
            }
        );
    }

    #[test]
    fn every_shipped_a6_token_resolves_to_interned_words() {
        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let dictionary_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
            let decoded = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == TEXT_OPCODE)
                .map(|token| decode_script_text(token, &dictionary))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(decoded.len(), EXPECTED_TEXT_COUNTS[profile - 1]);
        }
    }

    #[test]
    fn text_controls_cannot_consume_the_word_list_terminator() {
        let token_data = [
            TEXT_OPCODE,
            0,
            0,
            0,
            TEXT_RESUME_AND_POST_WORDS as u8,
            0,
            0,
            0,
            CODE_END_MARKER,
        ];
        let code = decode_script_code(&token_data).unwrap();
        let dictionary = decode_script_dictionary(&[u8::MIN]).unwrap();
        assert_eq!(
            decode_script_text(&code.tokens()[0], &dictionary).unwrap_err(),
            ScriptInstructionError::MalformedText {
                source_offset: ScriptCodeOffset::new(0),
            }
        );
    }
}
