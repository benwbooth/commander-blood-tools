//! Semantic BloodScript instructions decoded from losslessly framed COD tokens.

use std::fmt;

use crate::code::{ScriptCodeOffset, ScriptDecodingMode, ScriptOpcode, ScriptToken};
use crate::script::{
    ScriptDictionary, ScriptDirectory, ScriptProcedureId, ScriptState, ScriptStateWord,
    ScriptWordId,
};

const GUARD_BEGIN_OPCODE: u8 = 0xA0;
const GUARD_END_OPCODE: u8 = 0xA1;
const RANDOM_GUARD_OPCODE: u8 = 0xA2;
const CONCEPT_GUARD_OPCODE: u8 = 0xA3;
const JUMP_OPCODE: u8 = 0xA4;
const TIMER_STATE_OPCODE: u8 = 0xA5;
const TEXT_OPCODE: u8 = 0xA6;
const TOPIC_OFFER_OPCODE: u8 = 0xA7;
const SEQUENCE_REQUEST_OPCODE: u8 = 0xA8;
const PROCEDURE_GATE_OPCODE: u8 = 0xA9;
const YIELD_OPCODE: u8 = 0xAA;
const PROCEDURE_ACTIVATION_OPCODE: u8 = 0xAB;
const SHARED_STATE_A_OPCODE: u8 = 0xB1;
const SHARED_STATE_B_OPCODE: u8 = 0xB4;
const SHARED_STATE_C_OPCODE: u8 = 0xB5;
const SHARED_STATE_D_OPCODE: u8 = 0xB6;
const SHARED_STATE_E_OPCODE: u8 = 0xBE;
const SHARED_STATE_F_OPCODE: u8 = 0xBF;
const SHARED_STATE_G_OPCODE: u8 = 0xC0;
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
const TOPIC_OFFER_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const MINIMUM_SEQUENCE_REQUEST_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const PROCEDURE_GATE_SIZE: usize = OPCODE_SIZE + BYTE_SIZE + WORD_SIZE;
const PROCEDURE_ACTIVATION_SIZE: usize = OPCODE_SIZE + BYTE_SIZE + WORD_SIZE;
const YIELD_SIZE: usize = OPCODE_SIZE;
const ENABLED_FLAG_MASK: u8 = 1;
const SHARED_STATE_SIZE: usize = OPCODE_SIZE + WORD_SIZE + BYTE_SIZE + BYTE_SIZE + WORD_SIZE;
const INDIRECT_STATE_MODE_A: u8 = 0xC0;
const INDIRECT_STATE_MODE_B: u8 = 0xC2;
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

/// One optional dictionary topic offered by an A7 instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTopicOffer {
    /// Interned concept identity, or `None` for the native zero sentinel.
    pub topic: Option<ScriptWordId>,
}

/// Owned resource basename loaded by an A8 instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSequenceRequest {
    basename: Box<[u8]>,
}

/// One A9 procedure entry gate resolved through the companion directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProcedureGate {
    /// Procedure whose mutable enabled state controls this gate.
    pub procedure: ScriptProcedureId,
    /// Enabled state authored into the original COD image.
    pub initially_enabled: bool,
    /// Destination used when the procedure is disabled or its query fails.
    pub failure_target: ScriptCodeOffset,
}

/// One AB write to a procedure's typed mutable enabled state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProcedureActivation {
    /// Procedure whose enabled state changes.
    pub procedure: ScriptProcedureId,
    /// New enabled state derived from the written byte's only observed bit.
    pub enabled: bool,
}

/// Recovered signed-comparison or wrapping-assignment operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptStateOperator {
    /// Signed or unsigned inequality; both interpretations agree.
    NotEqual,
    /// Signed less-than comparison.
    LessThan,
    /// Signed greater-than comparison.
    GreaterThan,
    /// Signed less-than-or-equal comparison.
    LessThanOrEqual,
    /// Signed greater-than-or-equal comparison.
    GreaterThanOrEqual,
    /// Equality in query mode or assignment in set mode.
    EqualOrAssign,
    /// Wrapping addition in set mode; query mode fails.
    Add,
    /// Wrapping subtraction in set mode; query mode fails.
    Subtract,
    /// Any other original byte: query mode fails and set mode preserves the word.
    PreserveOrFail(u8),
}

/// Right-hand value of one shared VAR-state operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptStateOperand {
    /// Authored immediate word.
    Immediate(u16),
    /// Value read from another resolved state word.
    StateWord(ScriptStateWord),
}

/// One shared B1/B4/B5/B6/BE/BF/C0 state operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSharedStateOperation {
    /// State word read and optionally assigned.
    pub target: ScriptStateWord,
    /// Comparison or mutation selected by the authored operator byte.
    pub operator: ScriptStateOperator,
    /// Immediate or state-backed right-hand value.
    pub operand: ScriptStateOperand,
}

impl ScriptSequenceRequest {
    /// Construct a safe owned request from raw non-NUL basename bytes.
    pub fn new(basename: impl Into<Box<[u8]>>) -> Option<Self> {
        let basename = basename.into();
        (!basename.contains(&u8::MIN)).then_some(Self { basename })
    }

    /// Return the basename appended to the game's `sq/` resource directory.
    pub fn basename(&self) -> &[u8] {
        &self.basename
    }
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
    /// Stop the current execution pass.
    Yield,
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
    /// An A8 payload lacks its NUL terminator and consumed pad byte.
    MalformedSequenceRequest {
        /// Token position.
        source_offset: ScriptCodeOffset,
    },
    /// An A9 token is not the entry instruction of a declared procedure.
    InvalidProcedureGate {
        /// Token position.
        source_offset: ScriptCodeOffset,
    },
    /// An AB token targets no procedure enabled flag in the companion directory.
    InvalidProcedureActivationTarget {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Unresolved one-based procedure entry position.
        encoded_target: u16,
    },
    /// A shared-state operand is unaligned or outside all typed VAR regions.
    InvalidStateWord {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Unresolved VAR byte position.
        encoded_offset: u16,
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
        YIELD_OPCODE => {
            require_size(token, YIELD_SIZE)?;
            Ok(ScriptInstruction::Yield)
        }
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

/// Decode an A7 presentation-topic offer through the companion dictionary.
pub fn decode_script_topic_offer(
    token: &ScriptToken,
    dictionary: &ScriptDictionary,
) -> Result<ScriptTopicOffer, ScriptInstructionError> {
    if token.opcode().byte() != TOPIC_OFFER_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, TOPIC_OFFER_SIZE)?;
    let source_offset = read_word(token.encoded_bytes(), OPCODE_SIZE);
    let topic = if source_offset == u16::MIN {
        None
    } else {
        Some(dictionary.resolve_source_offset(source_offset).ok_or(
            ScriptInstructionError::InvalidDictionaryOffset {
                source_offset: token.source_offset(),
                dictionary_offset: source_offset,
            },
        )?)
    };
    Ok(ScriptTopicOffer { topic })
}

/// Decode an A8 NUL-terminated sequence basename and discard its format pad.
pub fn decode_script_sequence_request(
    token: &ScriptToken,
) -> Result<ScriptSequenceRequest, ScriptInstructionError> {
    if token.opcode().byte() != SEQUENCE_REQUEST_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    if bytes.len() < MINIMUM_SEQUENCE_REQUEST_SIZE
        || bytes[bytes.len() - WORD_SIZE] != u8::MIN
        || bytes[OPCODE_SIZE..bytes.len() - WORD_SIZE].contains(&u8::MIN)
    {
        return Err(ScriptInstructionError::MalformedSequenceRequest {
            source_offset: token.source_offset(),
        });
    }
    Ok(ScriptSequenceRequest {
        basename: Box::from(&bytes[OPCODE_SIZE..bytes.len() - WORD_SIZE]),
    })
}

/// Decode an A9 procedure gate without retaining its mutable COD byte.
pub fn decode_script_procedure_gate(
    token: &ScriptToken,
    directory: &ScriptDirectory,
) -> Result<ScriptProcedureGate, ScriptInstructionError> {
    if token.opcode().byte() != PROCEDURE_GATE_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, PROCEDURE_GATE_SIZE)?;
    let encoded_entry = token
        .source_offset()
        .index()
        .checked_add(OPCODE_SIZE)
        .and_then(|offset| u16::try_from(offset).ok())
        .ok_or(ScriptInstructionError::InvalidProcedureGate {
            source_offset: token.source_offset(),
        })?;
    let procedure = directory
        .resolve_procedure_activation_target(encoded_entry)
        .ok_or(ScriptInstructionError::InvalidProcedureGate {
            source_offset: token.source_offset(),
        })?;
    Ok(ScriptProcedureGate {
        procedure,
        initially_enabled: token.encoded_bytes()[OPCODE_SIZE] & ENABLED_FLAG_MASK != u8::MIN,
        failure_target: ScriptCodeOffset::new(usize::from(read_word(
            token.encoded_bytes(),
            OPCODE_SIZE + BYTE_SIZE,
        ))),
    })
}

/// Decode an AB procedure activation into a typed Boolean assignment.
pub fn decode_script_procedure_activation(
    token: &ScriptToken,
    directory: &ScriptDirectory,
) -> Result<ScriptProcedureActivation, ScriptInstructionError> {
    if token.opcode().byte() != PROCEDURE_ACTIVATION_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, PROCEDURE_ACTIVATION_SIZE)?;
    let enabled = token.encoded_bytes()[OPCODE_SIZE] & ENABLED_FLAG_MASK != u8::MIN;
    let encoded_target = read_word(token.encoded_bytes(), OPCODE_SIZE + BYTE_SIZE);
    let procedure = directory
        .resolve_procedure_activation_target(encoded_target)
        .ok_or(ScriptInstructionError::InvalidProcedureActivationTarget {
            source_offset: token.source_offset(),
            encoded_target,
        })?;
    Ok(ScriptProcedureActivation { procedure, enabled })
}

/// Decode the shared B1/B4/B5/B6/BE/BF/C0 handler family.
pub fn decode_script_shared_state_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptSharedStateOperation, ScriptInstructionError> {
    if !matches!(
        token.opcode().byte(),
        SHARED_STATE_A_OPCODE
            | SHARED_STATE_B_OPCODE
            | SHARED_STATE_C_OPCODE
            | SHARED_STATE_D_OPCODE
            | SHARED_STATE_E_OPCODE
            | SHARED_STATE_F_OPCODE
            | SHARED_STATE_G_OPCODE
    ) {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, SHARED_STATE_SIZE)?;
    let bytes = token.encoded_bytes();
    let target = resolve_state_word(token, state, read_word(bytes, OPCODE_SIZE))?;
    let operator = match bytes[OPCODE_SIZE + WORD_SIZE] {
        0xF0 => ScriptStateOperator::NotEqual,
        0xF1 => ScriptStateOperator::LessThan,
        0xF2 => ScriptStateOperator::GreaterThan,
        0xF3 => ScriptStateOperator::LessThanOrEqual,
        0xF4 => ScriptStateOperator::GreaterThanOrEqual,
        0xF5 => ScriptStateOperator::EqualOrAssign,
        0xF6 => ScriptStateOperator::Add,
        0xF7 => ScriptStateOperator::Subtract,
        other => ScriptStateOperator::PreserveOrFail(other),
    };
    let operand_offset = OPCODE_SIZE + WORD_SIZE + BYTE_SIZE + BYTE_SIZE;
    let encoded_operand = read_word(bytes, operand_offset);
    let operand = match bytes[OPCODE_SIZE + WORD_SIZE + BYTE_SIZE] {
        INDIRECT_STATE_MODE_A | INDIRECT_STATE_MODE_B => {
            ScriptStateOperand::StateWord(resolve_state_word(token, state, encoded_operand)?)
        }
        _ => ScriptStateOperand::Immediate(encoded_operand),
    };
    Ok(ScriptSharedStateOperation {
        target,
        operator,
        operand,
    })
}

fn resolve_state_word(
    token: &ScriptToken,
    state: &ScriptState,
    encoded_offset: u16,
) -> Result<ScriptStateWord, ScriptInstructionError> {
    state.resolve_word_source_offset(encoded_offset).ok_or(
        ScriptInstructionError::InvalidStateWord {
            source_offset: token.source_offset(),
            encoded_offset,
        },
    )
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
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use crate::code::decode_script_code;
    use crate::script::{decode_script_dictionary, decode_script_directory, decode_script_state};

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const CODE_END_MARKER: u8 = 0xFF;
    const EXPECTED_CONTROL_INSTRUCTION_COUNTS: [usize; PROFILE_COUNT] = [27, 782, 766, 318, 392];
    const EXPECTED_TEXT_COUNTS: [usize; PROFILE_COUNT] = [111, 1_157, 1_048, 719, 652];
    const EXPECTED_COD_SEQUENCE_REQUEST_COUNT: usize = 86;
    const EXPECTED_PROCEDURE_GATE_COUNTS: [usize; PROFILE_COUNT] = [13, 127, 166, 85, 89];
    const EXPECTED_PROCEDURE_ACTIVATION_COUNT: usize = 413;
    const EXPECTED_PROCEDURE_ENABLE_COUNT: usize = 149;
    const EXPECTED_PROCEDURE_DISABLE_COUNT: usize =
        EXPECTED_PROCEDURE_ACTIVATION_COUNT - EXPECTED_PROCEDURE_ENABLE_COUNT;
    const EXPECTED_SHARED_STATE_COUNTS: [usize; PROFILE_COUNT] = [2, 400, 301, 64, 172];
    const MAXIMUM_SHIPPED_SEQUENCE_BASENAME_LENGTH: usize = 12;

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
    fn every_shipped_a8_token_has_typed_sequence_semantics() {
        let mut sequence_request_count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();

            for token in code.tokens() {
                if token.opcode().byte() == SEQUENCE_REQUEST_OPCODE {
                    let request = decode_script_sequence_request(token).unwrap();
                    assert!(request.basename().ends_with(b".hnm"));
                    assert!(request.basename().len() <= MAXIMUM_SHIPPED_SEQUENCE_BASENAME_LENGTH);
                    sequence_request_count += 1;
                }
            }
        }

        assert_eq!(sequence_request_count, EXPECTED_COD_SEQUENCE_REQUEST_COUNT);
    }

    #[test]
    fn a7_topic_offer_decodes_its_bas_compatible_fixed_token() {
        let dictionary = decode_script_dictionary(b"\0topic\0").unwrap();
        let code = decode_script_code(&[TOPIC_OFFER_OPCODE, 1, 0, CODE_END_MARKER]).unwrap();
        let offer = decode_script_topic_offer(&code.tokens()[0], &dictionary).unwrap();
        assert_eq!(
            offer.topic,
            Some(dictionary.resolve_source_offset(1).unwrap())
        );

        let code = decode_script_code(&[TOPIC_OFFER_OPCODE, 0, 0, CODE_END_MARKER]).unwrap();
        let offer = decode_script_topic_offer(&code.tokens()[0], &dictionary).unwrap();
        assert_eq!(offer.topic, None);
    }

    #[test]
    fn every_shipped_a9_gate_and_ab_write_resolves_to_a_typed_procedure() {
        let mut activation_count = usize::MIN;
        let mut enable_count = usize::MIN;
        let mut disable_count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let gates = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == PROCEDURE_GATE_OPCODE)
                .map(|token| decode_script_procedure_gate(token, &directory).unwrap())
                .collect::<Vec<_>>();
            let gate_procedures = gates
                .iter()
                .map(|gate| gate.procedure)
                .collect::<BTreeSet<_>>();
            assert_eq!(gates.len(), EXPECTED_PROCEDURE_GATE_COUNTS[profile - 1]);
            assert_eq!(gates.len(), directory.procedures().count());
            assert_eq!(gate_procedures.len(), gates.len());

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == PROCEDURE_ACTIVATION_OPCODE)
            {
                let activation = decode_script_procedure_activation(token, &directory).unwrap();
                activation_count += 1;
                if activation.enabled {
                    enable_count += 1;
                } else {
                    disable_count += 1;
                }
            }
        }

        assert_eq!(activation_count, EXPECTED_PROCEDURE_ACTIVATION_COUNT);
        assert_eq!(enable_count, EXPECTED_PROCEDURE_ENABLE_COUNT);
        assert_eq!(disable_count, EXPECTED_PROCEDURE_DISABLE_COUNT);
    }

    #[test]
    fn every_shipped_shared_state_token_resolves_to_typed_var_words() {
        let mut counts = [usize::MIN; PROFILE_COUNT];
        let mut saw_object_word = false;
        let mut saw_trailing_state_word = false;

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code.tokens().iter().filter(|token| {
                matches!(
                    token.opcode().byte(),
                    SHARED_STATE_A_OPCODE
                        | SHARED_STATE_B_OPCODE
                        | SHARED_STATE_C_OPCODE
                        | SHARED_STATE_D_OPCODE
                        | SHARED_STATE_E_OPCODE
                        | SHARED_STATE_F_OPCODE
                        | SHARED_STATE_G_OPCODE
                )
            }) {
                let operation = decode_script_shared_state_operation(token, &state).unwrap();
                for word in std::iter::once(operation.target).chain(match operation.operand {
                    ScriptStateOperand::Immediate(_) => None,
                    ScriptStateOperand::StateWord(word) => Some(word),
                }) {
                    saw_object_word |= word.object().is_some();
                    saw_trailing_state_word |= word.is_trailing_state();
                }
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_SHARED_STATE_COUNTS);
        assert!(saw_object_word);
        assert!(saw_trailing_state_word);
    }

    #[test]
    fn aa_yield_has_explicit_typed_cod_semantics() {
        let code = decode_script_code(&[YIELD_OPCODE, CODE_END_MARKER]).unwrap();
        let dictionary = decode_script_dictionary(&[u8::MIN]).unwrap();
        assert_eq!(
            decode_script_instruction(&code.tokens()[0], &dictionary).unwrap(),
            ScriptInstruction::Yield
        );
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
