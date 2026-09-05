//! Semantic BloodScript instructions decoded from losslessly framed COD tokens.

use std::fmt;

use crate::code::{ScriptCodeOffset, ScriptDecodingMode, ScriptDialect, ScriptOpcode, ScriptToken};
use crate::script::{
    ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptProcedureId, ScriptState,
    ScriptStateByte, ScriptStateWord, ScriptStateWordPair, ScriptStateWordTriple, ScriptWordId,
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
const DIRECT_RECORD_A_OPCODE: u8 = 0xAD;
const SHARED_BIT_STATE_A_OPCODE: u8 = 0xAE;
const DIRECT_RECORD_B_OPCODE: u8 = 0xAF;
const SHARED_BIT_STATE_B_OPCODE: u8 = 0xB0;
const SHARED_STATE_A_OPCODE: u8 = 0xB1;
const DIRECT_RECORD_C_OPCODE: u8 = 0xB2;
const DIRECT_RECORD_D_OPCODE: u8 = 0xB3;
const SHARED_STATE_B_OPCODE: u8 = 0xB4;
const SHARED_STATE_C_OPCODE: u8 = 0xB5;
const SHARED_STATE_D_OPCODE: u8 = 0xB6;
const BIT_FLAG_OPCODE: u8 = 0xB7;
const PAIR_RECORD_A_OPCODE: u8 = 0xB8;
const PAIR_RECORD_B_OPCODE: u8 = 0xB9;
const DIRECT_RECORD_E_OPCODE: u8 = 0xBA;
const DIRECT_RECORD_F_OPCODE: u8 = 0xBB;
const DIRECT_RECORD_TOPIC_OPCODE: u8 = 0xBC;
const PAIR_RECORD_C_OPCODE: u8 = 0xBD;
const SHARED_STATE_E_OPCODE: u8 = 0xBE;
const SHARED_STATE_F_OPCODE: u8 = 0xBF;
const SHARED_STATE_G_OPCODE: u8 = 0xC0;
const RECORD_STATE_OPCODE: u8 = 0xC1;
const ABOARD_RECORD_OPCODE: u8 = 0xC2;
const PRESENTATION_QUEUE_OPCODE: u8 = 0xC3;
const ACTOR_RECORD_OPCODE: u8 = 0xC4;
const WORLD_STATE_RECORD_OPCODE: u8 = 0xC5;
const TRAVEL_RECORD_OPCODE: u8 = 0xC6;
const ACTIVE_OBJECT_RECORD_OPCODE: u8 = 0xC7;
const OPAQUE_MARKER_RECORD_OPCODE: u8 = 0xC8;
const RECORD_CLEAR_OPCODE: u8 = 0xC9;
const HOUR_GUARD_OPCODE: u8 = 0xCA;
const DATE_GUARD_OPCODE: u8 = 0xCB;
const SEQUENCE_SLOT_ASSIGNMENT_OPCODE: u8 = 0xCC;
const TRANSFER_OPCODE: u8 = 0xCD;
const BRIDGE_ACTIVITY_GUARD_OPCODE: u8 = 0xCE;
const ALTERNATE_CONCEPT_CLEAR_OPCODE: u8 = 0xCF;
const TRAVEL_ACTIVITY_GUARD_OPCODE: u8 = 0xD0;
const CONTACT_ACTIVITY_GUARD_OPCODE: u8 = 0xD1;
const PROFILE_REQUEST_OPCODE: u8 = 0xD2;
const MULTIPLY_DIVIDE_OPCODE: u8 = 0xD3;
const MULTIPLY_DIVIDE_SIZE: usize = OPCODE_SIZE + WORD_SIZE + (BYTE_SIZE + WORD_SIZE) * 2;
const SEQUEL_GROWTH_OPCODE: u8 = 0xD6;
const SEQUEL_SETTLEMENT_OPCODE: u8 = 0xD5;
const SEQUEL_SETTLEMENT_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const SEQUEL_GROWTH_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
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
const SHARED_BIT_STATE_SIZE: usize = OPCODE_SIZE + WORD_SIZE + WORD_SIZE;
const DIRECT_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE + WORD_SIZE;
const TRANSFER_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 3;
const BIT_FLAG_SIZE: usize = OPCODE_SIZE + WORD_SIZE + BYTE_SIZE;
const PAIR_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 3;
const RECORD_STATE_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const ABOARD_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const PRESENTATION_QUEUE_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const ACTOR_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const WORLD_STATE_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const TRAVEL_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const ACTIVE_OBJECT_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const OPAQUE_MARKER_RECORD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const RECORD_CLEAR_SIZE: usize = OPCODE_SIZE + WORD_SIZE;
const HOUR_GUARD_SIZE: usize = OPCODE_SIZE + WORD_SIZE * 2;
const DATE_GUARD_SIZE: usize = OPCODE_SIZE + BYTE_SIZE * 3 + WORD_SIZE;
const MINIMUM_SEQUENCE_SLOT_ASSIGNMENT_SIZE: usize = OPCODE_SIZE + BYTE_SIZE + WORD_SIZE;
const ENVIRONMENT_INSTRUCTION_SIZE: usize = OPCODE_SIZE;
const PROFILE_REQUEST_SIZE: usize = OPCODE_SIZE + BYTE_SIZE;
const BITS_PER_BYTE: u8 = u8::BITS as u8;
const PRIMARY_NAVIGATION_OPERAND: u16 = 1;
const SECONDARY_NAVIGATION_OPERAND: u16 = 2;
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
const FIRST_SEQUENCE_SLOT: u8 = 1;
const SEQUENCE_SLOT_COUNT: u8 = 6;
const LAST_SEQUENCE_SLOT: u8 = FIRST_SEQUENCE_SLOT + SEQUENCE_SLOT_COUNT - 1;
const MAXIMUM_SEQUENCE_SLOT_NAME_LENGTH: usize = 15;

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

/// One of the six DESCRIPT sequence-name slots encoded as values one through six.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptSequenceSlot(u8);

impl ScriptSequenceSlot {
    /// Number of sequence names retained by the original presentation state.
    pub const COUNT: usize = SEQUENCE_SLOT_COUNT as usize;

    /// Decode the one-based slot number authored in BloodScript bytecode.
    pub const fn decode(encoded: u8) -> Option<Self> {
        if encoded >= FIRST_SEQUENCE_SLOT && encoded <= LAST_SEQUENCE_SLOT {
            Some(Self(encoded - FIRST_SEQUENCE_SLOT))
        } else {
            None
        }
    }

    /// Return this slot's zero-based index in an owned Rust collection.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Return the one-based value serialized in BloodScript bytecode.
    pub const fn encode(self) -> u8 {
        self.0 + FIRST_SEQUENCE_SLOT
    }
}

/// Owned DESCRIPT sequence-record name assigned to one presentation slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSequenceSlotName(Box<[u8]>);

impl ScriptSequenceSlotName {
    /// Longest name that fits in the original 16-byte field with its trailing NUL.
    pub const MAXIMUM_BYTE_LENGTH: usize = MAXIMUM_SEQUENCE_SLOT_NAME_LENGTH;

    /// Build a bounded name without retaining the original fixed-width field.
    pub fn new(name: impl Into<Box<[u8]>>) -> Option<Self> {
        let name = name.into();
        (name.len() <= Self::MAXIMUM_BYTE_LENGTH && !name.contains(&u8::MIN)).then_some(Self(name))
    }

    /// Return the case-sensitive DESCRIPT directory name exactly as authored.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// One CC assignment of a DESCRIPT sequence name to a presentation slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSequenceSlotAssignment {
    slot: ScriptSequenceSlot,
    name: ScriptSequenceSlotName,
}

impl ScriptSequenceSlotAssignment {
    /// Return the selected sequence-name slot.
    pub const fn slot(&self) -> ScriptSequenceSlot {
        self.slot
    }

    /// Return the assigned DESCRIPT directory name.
    pub fn name(&self) -> &ScriptSequenceSlotName {
        &self.name
    }

    /// Split the assignment into values suitable for owned runtime storage.
    pub fn into_parts(self) -> (ScriptSequenceSlot, ScriptSequenceSlotName) {
        (self.slot, self.name)
    }
}

/// One D2 request to switch the active five-file BloodScript profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProfileRequest {
    one_based_profile_number: i8,
}

impl ScriptProfileRequest {
    /// Return the signed one-based profile number encoded by the authored token.
    pub const fn one_based_profile_number(self) -> i8 {
        self.one_based_profile_number
    }

    /// Return the exact signed zero-based value written by the native handler.
    pub const fn zero_based_profile_index(self) -> i16 {
        self.one_based_profile_number as i16 - 1
    }
}

/// One no-operand instruction controlled by the surrounding game presentation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptEnvironmentInstruction {
    /// Continue only while the bridge renderer is active (CE).
    RequireBridgeActivity,
    /// Clear the alternate concept and its shared resume state (CF).
    ClearAlternateConcept,
    /// Continue only while travel presentation is active (D0).
    RequireTravelActivity,
    /// Continue only while contact presentation is active (D1).
    RequireContactActivity,
}

/// Relation between an authored time literal and the host clock value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTemporalRelation {
    /// The authored literal must be later than the current value (`0xF1`).
    After,
    /// The authored literal must be earlier than the current value (`0xF2`).
    Before,
    /// The authored literal must equal the current value (every other tag).
    Equal,
}

/// One CA signed-hour condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptHourGuard {
    relation: ScriptTemporalRelation,
    hour: i16,
}

impl ScriptHourGuard {
    /// Return the comparison selected by the low byte of the authored tag word.
    pub const fn relation(self) -> ScriptTemporalRelation {
        self.relation
    }

    /// Return the authored signed hour literal.
    pub const fn hour(self) -> i16 {
        self.hour
    }
}

/// One CB month/day condition and its consumed informational year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptDateGuard {
    relation: ScriptTemporalRelation,
    day: i8,
    month: i8,
    encoded_year: u16,
}

impl ScriptDateGuard {
    /// Return the authored month/day comparison.
    pub const fn relation(self) -> ScriptTemporalRelation {
        self.relation
    }

    /// Return the authored signed day byte.
    pub const fn day(self) -> i8 {
        self.day
    }

    /// Return the authored signed month byte.
    pub const fn month(self) -> i8 {
        self.month
    }

    /// Return the year consumed but deliberately not compared by the native handler.
    pub const fn encoded_year(self) -> u16 {
        self.encoded_year
    }
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

/// One shared AE/B0 masked-bit query or mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSharedBitOperation {
    /// State word tested or assigned.
    pub target: ScriptStateWord,
    /// Authored mask applied to the target word.
    pub mask: u16,
    /// Query for absence instead of presence, or clear instead of set.
    pub inverted_or_clear: bool,
}

/// Typed value compared or assigned by the direct-record handler family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRecordValue {
    /// Relationship to one active profile object.
    Object(ScriptObjectId),
    /// Original `0xFFFF` relationship denoting an object aboard the ship.
    Aboard,
    /// Topic interned from the companion dictionary by shipped BC instructions.
    Topic(ScriptWordId),
    /// Proven native word domain for dispatch aliases absent from shipped COD.
    NativeWord(u16),
}

/// One AD/AF/B2/B3/BA/BB/BC direct-record query or assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptDirectRecordOperation {
    /// Typed state field compared or assigned.
    pub target: ScriptStateWord,
    /// Typed relationship, topic, or unshipped native word.
    pub value: ScriptRecordValue,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
    /// Whether assignment also publishes the value for presentation dispatch.
    pub publishes_value: bool,
}

/// One CD object transfer or optionally inverted transfer-record query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTransfer {
    /// Typed action field whose owner is the source object in assignment mode.
    pub source_record: ScriptStateWord,
    /// Object moved between holders.
    pub item: ScriptObjectId,
    /// Destination holder; the built-in player object denotes aboard.
    pub destination: ScriptObjectId,
    /// Whether query-mode triple equality is inverted.
    pub inverted: bool,
}

/// One B7 high-bit-first state-bit query or mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptBitFlagOperation {
    /// Bounded owned byte containing the selected bit.
    pub target: ScriptStateByte,
    /// High-bit-first mask derived from the authored bit index.
    pub mask: u8,
    /// Whether query equality is inverted or assignment clears the bit.
    pub inverted_or_clear: bool,
}

/// One B8/B9/BD adjacent-word comparison or assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordPairOperation {
    /// Bounded pair contained by one owned VAR region.
    pub target: ScriptStateWordPair,
    /// Two authored words compared or assigned in order.
    pub value: [u16; 2],
}

/// Special selector or typed value carried by one C1 state record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRecordStateOperand {
    /// Native special operand 1, mapped to an explicit runtime object.
    PrimaryNavigationObject,
    /// Native special operand 2, mapped to an explicit runtime object.
    SecondaryNavigationObject,
    /// One active profile object.
    Object(ScriptObjectId),
    /// Unshipped value retained in the native record-value domain.
    NativeWord(u16),
}

/// One optionally inverted C1 action-record query or assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordStateOperation {
    /// Bounded three-word action slot.
    pub target: ScriptStateWordTriple,
    /// Typed action value or explicit special navigation selector.
    pub operand: ScriptRecordStateOperand,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C2 aboard-object operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptAboardRecordOperation {
    /// Bounded three-word action slot used by query mode.
    pub target: ScriptStateWordTriple,
    /// Object moved aboard in assignment mode or compared in query mode.
    pub related: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C3 queued-presentation operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptPresentationQueueOperation {
    /// Bounded three-word presentation-action slot.
    pub target: ScriptStateWordTriple,
    /// Object related to the queued presentation.
    pub related: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C4 actor-presentation record operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptActorRecordOperation {
    /// Bounded three-word actor action slot.
    pub target: ScriptStateWordTriple,
    /// Active profile object stored as the actor record's relation.
    pub related: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C5 link to an active world-state object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptWorldStateRecordOperation {
    /// Bounded three-word destination slot.
    pub target: ScriptStateWordTriple,
    /// World-state object stored as the record relation in assignment mode.
    pub related: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C6 travel relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptTravelRecordOperation {
    /// Bounded three-word travel-action slot.
    pub target: ScriptStateWordTriple,
    /// Destination object stored by the travel relation.
    pub destination: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C7 relation to an active object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptActiveObjectRecordOperation {
    /// Bounded three-word destination slot.
    pub target: ScriptStateWordTriple,
    /// Active object stored by the relation.
    pub related: ScriptObjectId,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One optionally inverted C8 opaque-marker operation.
///
/// No shipped profile contains a C8 instruction and the native binary has no
/// C8-specific consumer. The neutral name retains the proven record behavior
/// without assigning an unsupported gameplay meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptOpaqueMarkerRecordOperation {
    /// Bounded three-word marker slot.
    pub target: ScriptStateWordTriple,
    /// Opaque second word compared in query mode and ignored in assignment mode.
    pub comparison_word: u16,
    /// Whether query-mode equality is inverted.
    pub inverted: bool,
}

/// One C9 action-record teardown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptRecordClearOperation {
    /// Bounded three-word action slot cleared by the operation.
    pub target: ScriptStateWordTriple,
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

/// Complete typed semantic domain of one shipped BloodScript COD token.
///
/// The original executable dispatches these families through a native handler
/// table. Grouping the already recovered family-specific values here gives the
/// modern runtime one exhaustive input type without erasing distinctions that
/// matter to the individual handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodedScriptInstruction {
    /// A0-A5 or AA control-flow operation.
    Control(ScriptInstruction),
    /// A6 dialogue or subtitle instruction.
    Text(ScriptText),
    /// A7 optional presentation topic.
    TopicOffer(ScriptTopicOffer),
    /// A8 sequence-resource request.
    SequenceRequest(ScriptSequenceRequest),
    /// A9 mutable procedure gate.
    ProcedureGate(ScriptProcedureGate),
    /// AB procedure enable-state assignment.
    ProcedureActivation(ScriptProcedureActivation),
    /// AE or B0 masked shared-state operation.
    SharedBit(ScriptSharedBitOperation),
    /// B1, B4-B6, or BE-C0 shared-state operation.
    SharedState(ScriptSharedStateOperation),
    /// AD, AF, B2-B3, or BA-BC direct record operation.
    DirectRecord(ScriptDirectRecordOperation),
    /// B7 byte-backed bit flag operation.
    BitFlag(ScriptBitFlagOperation),
    /// B8, B9, or BD adjacent record pair assignment.
    RecordPair(ScriptRecordPairOperation),
    /// C1 navigation action-record operation.
    RecordState(ScriptRecordStateOperation),
    /// C2 aboard-object action-record operation.
    AboardRecord(ScriptAboardRecordOperation),
    /// C3 presentation-queue action-record operation.
    PresentationQueue(ScriptPresentationQueueOperation),
    /// C4 actor-presentation action-record operation.
    ActorRecord(ScriptActorRecordOperation),
    /// C5 world-state action-record operation.
    WorldStateRecord(ScriptWorldStateRecordOperation),
    /// C6 travel action-record operation.
    TravelRecord(ScriptTravelRecordOperation),
    /// C7 active-object action-record operation.
    ActiveObjectRecord(ScriptActiveObjectRecordOperation),
    /// C8 dormant opaque-marker action-record operation.
    OpaqueMarkerRecord(ScriptOpaqueMarkerRecordOperation),
    /// C9 action-record teardown.
    RecordClear(ScriptRecordClearOperation),
    /// CA signed-hour guard.
    HourGuard(ScriptHourGuard),
    /// CB month/day guard.
    DateGuard(ScriptDateGuard),
    /// CC DESCRIPT sequence-slot assignment.
    SequenceSlotAssignment(ScriptSequenceSlotAssignment),
    /// CD object-transfer operation.
    Transfer(ScriptTransfer),
    /// CE-D1 presentation-environment operation.
    Environment(ScriptEnvironmentInstruction),
    /// D2 script-profile replacement request.
    ProfileRequest(ScriptProfileRequest),
    /// Big Bug Bang D3 unsigned multiply/divide assignment, including in query mode.
    MultiplyDivide(ScriptMultiplyDivideOperation),
    /// Big Bug Bang D6 group-filtered actor quantity and growth update.
    SequelGrowth(ScriptSequelGrowthOperation),
    /// Big Bug Bang D5 relocation of matching actors to a nearby free location.
    SequelSettlement(ScriptSequelSettlementOperation),
}

/// Immediate group mask consumed by BLOOD2PG.EXE's D5 handler at 0x7367.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSequelSettlementOperation {
    /// Select source actors and relocating descendants by intersecting group bits.
    pub group_mask: u16,
}

/// Two immediate operands consumed by BLOOD2PG.EXE's D6 handler at 0x728B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSequelGrowthOperation {
    /// Select actors whose group flags intersect this mask.
    pub group_mask: u16,
    /// Signed multiplier used by the quantity-growth arithmetic.
    pub rate: i16,
}

/// Big Bug Bang's native D3 update: target = (target * multiplier) / divisor.
///
/// BLOOD2PG.EXE file 0x7408 reads all operands before writing. The product is
/// 32-bit; division must fault, not wrap or saturate, when the quotient exceeds
/// a word. C0 and C2 operand modes reference VAR words; other modes are immediate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptMultiplyDivideOperation {
    /// Destination in owned flat profile state.
    pub target: ScriptStateWord,
    /// Unsigned multiplication operand, resolved before any write.
    pub multiplier: ScriptStateOperand,
    /// Unsigned division operand, resolved before any write.
    pub divisor: ScriptStateOperand,
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
    /// A CC token names a presentation slot outside the six-entry shipped table.
    InvalidSequenceSlot {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Original one-based slot byte.
        encoded: u8,
    },
    /// A CC token's terminated name does not fit one original sequence-name field.
    InvalidSequenceSlotName {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Name length excluding its encoded NUL terminator.
        byte_length: usize,
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
    /// A transfer operand names no active object in the companion directory.
    InvalidObjectReference {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Unresolved VAR object position.
        encoded_offset: u16,
    },
    /// A bit-flag operand reaches outside all typed VAR regions.
    InvalidStateByte {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Unresolved VAR byte position after applying the bit index.
        encoded_offset: u16,
    },
    /// A pair-record operand is unaligned, truncated, or crosses a VAR owner boundary.
    InvalidStateWordPair {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Unresolved VAR byte position.
        encoded_offset: u16,
    },
    /// A state-record operand is unaligned, truncated, or crosses a VAR owner boundary.
    InvalidStateWordTriple {
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

/// Decode any shipped COD opcode into one exhaustive semantic value.
///
/// The recovered AC handler is a second yield entry used by BAS selector
/// blocks, but AC has no site in the five shipped COD images and remains owned
/// by the BAS decoder. Any malformed or unsupported token remains an explicit
/// error instead of entering the runtime as an opaque operation.
pub fn decode_complete_script_instruction(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
    dictionary: &ScriptDictionary,
) -> Result<DecodedScriptInstruction, ScriptInstructionError> {
    let decoded = match token.opcode().byte() {
        GUARD_BEGIN_OPCODE | GUARD_END_OPCODE | RANDOM_GUARD_OPCODE | CONCEPT_GUARD_OPCODE
        | JUMP_OPCODE | TIMER_STATE_OPCODE | YIELD_OPCODE => {
            DecodedScriptInstruction::Control(decode_script_instruction(token, dictionary)?)
        }
        TEXT_OPCODE => DecodedScriptInstruction::Text(decode_script_text(token, dictionary)?),
        TOPIC_OFFER_OPCODE => {
            DecodedScriptInstruction::TopicOffer(decode_script_topic_offer(token, dictionary)?)
        }
        SEQUENCE_REQUEST_OPCODE => {
            DecodedScriptInstruction::SequenceRequest(decode_script_sequence_request(token)?)
        }
        PROCEDURE_GATE_OPCODE => {
            DecodedScriptInstruction::ProcedureGate(decode_script_procedure_gate(token, directory)?)
        }
        PROCEDURE_ACTIVATION_OPCODE => DecodedScriptInstruction::ProcedureActivation(
            decode_script_procedure_activation(token, directory)?,
        ),
        SHARED_BIT_STATE_A_OPCODE | SHARED_BIT_STATE_B_OPCODE => {
            DecodedScriptInstruction::SharedBit(decode_script_shared_bit_operation(token, state)?)
        }
        SHARED_STATE_A_OPCODE
        | SHARED_STATE_B_OPCODE
        | SHARED_STATE_C_OPCODE
        | SHARED_STATE_D_OPCODE
        | SHARED_STATE_E_OPCODE
        | SHARED_STATE_F_OPCODE
        | SHARED_STATE_G_OPCODE => DecodedScriptInstruction::SharedState(
            decode_script_shared_state_operation(token, state)?,
        ),
        DIRECT_RECORD_A_OPCODE
        | DIRECT_RECORD_B_OPCODE
        | DIRECT_RECORD_C_OPCODE
        | DIRECT_RECORD_D_OPCODE
        | DIRECT_RECORD_E_OPCODE
        | DIRECT_RECORD_F_OPCODE
        | DIRECT_RECORD_TOPIC_OPCODE => DecodedScriptInstruction::DirectRecord(
            decode_script_direct_record_operation(token, state, directory, dictionary)?,
        ),
        BIT_FLAG_OPCODE => {
            DecodedScriptInstruction::BitFlag(decode_script_bit_flag_operation(token, state)?)
        }
        PAIR_RECORD_A_OPCODE | PAIR_RECORD_B_OPCODE | PAIR_RECORD_C_OPCODE => {
            DecodedScriptInstruction::RecordPair(decode_script_record_pair_operation(token, state)?)
        }
        RECORD_STATE_OPCODE => DecodedScriptInstruction::RecordState(
            decode_script_record_state_operation(token, state, directory)?,
        ),
        ABOARD_RECORD_OPCODE => DecodedScriptInstruction::AboardRecord(
            decode_script_aboard_record_operation(token, state, directory)?,
        ),
        PRESENTATION_QUEUE_OPCODE => DecodedScriptInstruction::PresentationQueue(
            decode_script_presentation_queue_operation(token, state, directory)?,
        ),
        ACTOR_RECORD_OPCODE => DecodedScriptInstruction::ActorRecord(
            decode_script_actor_record_operation(token, state, directory)?,
        ),
        WORLD_STATE_RECORD_OPCODE => DecodedScriptInstruction::WorldStateRecord(
            decode_script_world_state_record_operation(token, state, directory)?,
        ),
        TRAVEL_RECORD_OPCODE => DecodedScriptInstruction::TravelRecord(
            decode_script_travel_record_operation(token, state, directory)?,
        ),
        ACTIVE_OBJECT_RECORD_OPCODE => DecodedScriptInstruction::ActiveObjectRecord(
            decode_script_active_object_record_operation(token, state, directory)?,
        ),
        OPAQUE_MARKER_RECORD_OPCODE => DecodedScriptInstruction::OpaqueMarkerRecord(
            decode_script_opaque_marker_record_operation(token, state)?,
        ),
        RECORD_CLEAR_OPCODE => DecodedScriptInstruction::RecordClear(
            decode_script_record_clear_operation(token, state)?,
        ),
        HOUR_GUARD_OPCODE => DecodedScriptInstruction::HourGuard(decode_script_hour_guard(token)?),
        DATE_GUARD_OPCODE => DecodedScriptInstruction::DateGuard(decode_script_date_guard(token)?),
        SEQUENCE_SLOT_ASSIGNMENT_OPCODE => DecodedScriptInstruction::SequenceSlotAssignment(
            decode_script_sequence_slot_assignment(token)?,
        ),
        TRANSFER_OPCODE => {
            DecodedScriptInstruction::Transfer(decode_script_transfer(token, state, directory)?)
        }
        BRIDGE_ACTIVITY_GUARD_OPCODE
        | ALTERNATE_CONCEPT_CLEAR_OPCODE
        | TRAVEL_ACTIVITY_GUARD_OPCODE
        | CONTACT_ACTIVITY_GUARD_OPCODE => {
            DecodedScriptInstruction::Environment(decode_script_environment_instruction(token)?)
        }
        PROFILE_REQUEST_OPCODE => {
            DecodedScriptInstruction::ProfileRequest(decode_script_profile_request(token)?)
        }
        MULTIPLY_DIVIDE_OPCODE if token.dialect() == ScriptDialect::BigBugBang => {
            DecodedScriptInstruction::MultiplyDivide(decode_script_multiply_divide(token, state)?)
        }
        SEQUEL_GROWTH_OPCODE if token.dialect() == ScriptDialect::BigBugBang => {
            DecodedScriptInstruction::SequelGrowth(decode_script_sequel_growth(token)?)
        }
        SEQUEL_SETTLEMENT_OPCODE if token.dialect() == ScriptDialect::BigBugBang => {
            DecodedScriptInstruction::SequelSettlement(decode_script_sequel_settlement(token)?)
        }
        _ => {
            return Err(ScriptInstructionError::UntranslatedOpcode {
                opcode: token.opcode(),
            });
        }
    };
    Ok(decoded)
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

/// Decode one D2 profile request without assuming its operand is playable.
pub fn decode_script_profile_request(
    token: &ScriptToken,
) -> Result<ScriptProfileRequest, ScriptInstructionError> {
    if token.opcode().byte() != PROFILE_REQUEST_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, PROFILE_REQUEST_SIZE)?;
    Ok(ScriptProfileRequest {
        one_based_profile_number: token.encoded_bytes()[OPCODE_SIZE] as i8,
    })
}

/// Decode one CE through D1 environment instruction.
pub fn decode_script_environment_instruction(
    token: &ScriptToken,
) -> Result<ScriptEnvironmentInstruction, ScriptInstructionError> {
    let instruction = match token.opcode().byte() {
        BRIDGE_ACTIVITY_GUARD_OPCODE => ScriptEnvironmentInstruction::RequireBridgeActivity,
        ALTERNATE_CONCEPT_CLEAR_OPCODE => ScriptEnvironmentInstruction::ClearAlternateConcept,
        TRAVEL_ACTIVITY_GUARD_OPCODE => ScriptEnvironmentInstruction::RequireTravelActivity,
        CONTACT_ACTIVITY_GUARD_OPCODE => ScriptEnvironmentInstruction::RequireContactActivity,
        _ => {
            return Err(ScriptInstructionError::UntranslatedOpcode {
                opcode: token.opcode(),
            });
        }
    };
    require_size(token, ENVIRONMENT_INSTRUCTION_SIZE)?;
    Ok(instruction)
}

/// Decode one CA signed-hour guard.
pub fn decode_script_hour_guard(
    token: &ScriptToken,
) -> Result<ScriptHourGuard, ScriptInstructionError> {
    if token.opcode().byte() != HOUR_GUARD_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, HOUR_GUARD_SIZE)?;
    let bytes = token.encoded_bytes();
    Ok(ScriptHourGuard {
        relation: decode_temporal_relation(bytes[OPCODE_SIZE]),
        hour: read_word(bytes, OPCODE_SIZE + WORD_SIZE) as i16,
    })
}

/// Decode one CB signed month/day guard while retaining its ignored year word.
pub fn decode_script_date_guard(
    token: &ScriptToken,
) -> Result<ScriptDateGuard, ScriptInstructionError> {
    if token.opcode().byte() != DATE_GUARD_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, DATE_GUARD_SIZE)?;
    let bytes = token.encoded_bytes();
    Ok(ScriptDateGuard {
        relation: decode_temporal_relation(bytes[OPCODE_SIZE]),
        day: bytes[OPCODE_SIZE + BYTE_SIZE] as i8,
        month: bytes[OPCODE_SIZE + BYTE_SIZE * 2] as i8,
        encoded_year: read_word(bytes, OPCODE_SIZE + BYTE_SIZE * 3),
    })
}

/// Decode one CC bounded DESCRIPT sequence-slot assignment.
pub fn decode_script_sequence_slot_assignment(
    token: &ScriptToken,
) -> Result<ScriptSequenceSlotAssignment, ScriptInstructionError> {
    if token.opcode().byte() != SEQUENCE_SLOT_ASSIGNMENT_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    if bytes.len() < MINIMUM_SEQUENCE_SLOT_ASSIGNMENT_SIZE
        || bytes[bytes.len() - WORD_SIZE..] != [u8::MIN; WORD_SIZE]
        || bytes[OPCODE_SIZE + BYTE_SIZE..bytes.len() - WORD_SIZE].contains(&u8::MIN)
    {
        return Err(ScriptInstructionError::InvalidSequenceSlotName {
            source_offset: token.source_offset(),
            byte_length: bytes
                .len()
                .saturating_sub(OPCODE_SIZE + BYTE_SIZE + WORD_SIZE),
        });
    }
    let encoded_slot = bytes[OPCODE_SIZE];
    let slot = ScriptSequenceSlot::decode(encoded_slot).ok_or(
        ScriptInstructionError::InvalidSequenceSlot {
            source_offset: token.source_offset(),
            encoded: encoded_slot,
        },
    )?;
    let name_bytes = &bytes[OPCODE_SIZE + BYTE_SIZE..bytes.len() - WORD_SIZE];
    let name = ScriptSequenceSlotName::new(Box::<[u8]>::from(name_bytes)).ok_or(
        ScriptInstructionError::InvalidSequenceSlotName {
            source_offset: token.source_offset(),
            byte_length: name_bytes.len(),
        },
    )?;
    Ok(ScriptSequenceSlotAssignment { slot, name })
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

/// Bind the sequel's D3 operands to typed VAR words without evaluating them early.
pub fn decode_script_multiply_divide(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptMultiplyDivideOperation, ScriptInstructionError> {
    if token.dialect() != ScriptDialect::BigBugBang
        || token.opcode().byte() != MULTIPLY_DIVIDE_OPCODE
    {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, MULTIPLY_DIVIDE_SIZE)?;
    let bytes = token.encoded_bytes();
    let operand = |offset| {
        let value = read_word(bytes, offset + BYTE_SIZE);
        match bytes[offset] {
            INDIRECT_STATE_MODE_A | INDIRECT_STATE_MODE_B => {
                resolve_state_word(token, state, value).map(ScriptStateOperand::StateWord)
            }
            _ => Ok(ScriptStateOperand::Immediate(value)),
        }
    };
    Ok(ScriptMultiplyDivideOperation {
        target: resolve_state_word(token, state, read_word(bytes, OPCODE_SIZE))?,
        multiplier: operand(OPCODE_SIZE + WORD_SIZE)?,
        divisor: operand(OPCODE_SIZE + WORD_SIZE + BYTE_SIZE + WORD_SIZE)?,
    })
}

/// Decode the sequel's D6 operands without guessing indirect operand modes.
pub fn decode_script_sequel_growth(
    token: &ScriptToken,
) -> Result<ScriptSequelGrowthOperation, ScriptInstructionError> {
    if token.dialect() != ScriptDialect::BigBugBang || token.opcode().byte() != SEQUEL_GROWTH_OPCODE
    {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, SEQUEL_GROWTH_SIZE)?;
    let bytes = token.encoded_bytes();
    Ok(ScriptSequelGrowthOperation {
        group_mask: read_word(bytes, OPCODE_SIZE),
        rate: read_word(bytes, OPCODE_SIZE + WORD_SIZE) as i16,
    })
}

/// Decode the sequel's D5 group mask without treating it as a state reference.
pub fn decode_script_sequel_settlement(
    token: &ScriptToken,
) -> Result<ScriptSequelSettlementOperation, ScriptInstructionError> {
    if token.dialect() != ScriptDialect::BigBugBang
        || token.opcode().byte() != SEQUEL_SETTLEMENT_OPCODE
    {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, SEQUEL_SETTLEMENT_SIZE)?;
    Ok(ScriptSequelSettlementOperation {
        group_mask: read_word(token.encoded_bytes(), OPCODE_SIZE),
    })
}

/// Decode the shared AE/B0 masked-bit handler family.
pub fn decode_script_shared_bit_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptSharedBitOperation, ScriptInstructionError> {
    if !matches!(
        token.opcode().byte(),
        SHARED_BIT_STATE_A_OPCODE | SHARED_BIT_STATE_B_OPCODE
    ) {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted_or_clear = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted_or_clear);
    let expected_size = SHARED_BIT_STATE_SIZE + prefix_size;
    require_size(token, expected_size)?;
    let target_offset = OPCODE_SIZE + prefix_size;
    let target = resolve_state_word(token, state, read_word(bytes, target_offset))?;
    let mask = read_word(bytes, target_offset + WORD_SIZE);
    Ok(ScriptSharedBitOperation {
        target,
        mask,
        inverted_or_clear,
    })
}

/// Decode the shared direct-record handler into flat typed identities.
pub fn decode_script_direct_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
    dictionary: &ScriptDictionary,
) -> Result<ScriptDirectRecordOperation, ScriptInstructionError> {
    if !is_direct_record_opcode(token.opcode().byte()) {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted);
    require_size(token, DIRECT_RECORD_SIZE + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    let target = resolve_state_word(token, state, read_word(bytes, operand_offset))?;
    let encoded_value = read_word(bytes, operand_offset + WORD_SIZE);
    let publishes_value = token.opcode().byte() == DIRECT_RECORD_TOPIC_OPCODE;
    let value = if publishes_value {
        dictionary
            .resolve_source_offset(encoded_value)
            .map(ScriptRecordValue::Topic)
            .unwrap_or(ScriptRecordValue::NativeWord(encoded_value))
    } else if encoded_value == u16::MAX {
        ScriptRecordValue::Aboard
    } else {
        directory
            .active_objects()
            .find_map(|(object, entry)| (entry.value == encoded_value).then_some(object))
            .map(ScriptRecordValue::Object)
            .unwrap_or(ScriptRecordValue::NativeWord(encoded_value))
    };
    Ok(ScriptDirectRecordOperation {
        target,
        value,
        inverted,
        publishes_value,
    })
}

/// Decode one CD transfer into typed source, item, and destination identities.
pub fn decode_script_transfer(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptTransfer, ScriptInstructionError> {
    if token.opcode().byte() != TRANSFER_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted);
    require_size(token, TRANSFER_SIZE + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    let source_record = resolve_state_word(token, state, read_word(bytes, operand_offset))?;
    let item = resolve_active_object(
        token,
        directory,
        read_word(bytes, operand_offset + WORD_SIZE),
    )?;
    let destination = resolve_active_object(
        token,
        directory,
        read_word(bytes, operand_offset + WORD_SIZE * 2),
    )?;
    Ok(ScriptTransfer {
        source_record,
        item,
        destination,
        inverted,
    })
}

/// Decode one B7 high-bit-first flag into a bounded byte and mask.
pub fn decode_script_bit_flag_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptBitFlagOperation, ScriptInstructionError> {
    if token.opcode().byte() != BIT_FLAG_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted_or_clear = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted_or_clear);
    require_size(token, BIT_FLAG_SIZE + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    let base_offset = read_word(bytes, operand_offset);
    let bit_index = bytes[operand_offset + WORD_SIZE];
    let encoded_offset = base_offset.wrapping_add(u16::from(bit_index / BITS_PER_BYTE));
    let target = state.resolve_byte_source_offset(encoded_offset).ok_or(
        ScriptInstructionError::InvalidStateByte {
            source_offset: token.source_offset(),
            encoded_offset,
        },
    )?;
    let bit_in_byte = bit_index % BITS_PER_BYTE;
    let mask = 1_u8 << (BITS_PER_BYTE - 1 - bit_in_byte);
    Ok(ScriptBitFlagOperation {
        target,
        mask,
        inverted_or_clear,
    })
}

/// Decode the shared B8/B9/BD handler into one bounded adjacent word pair.
pub fn decode_script_record_pair_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptRecordPairOperation, ScriptInstructionError> {
    if !is_pair_record_opcode(token.opcode().byte()) {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, PAIR_RECORD_SIZE)?;
    let bytes = token.encoded_bytes();
    let encoded_offset = read_word(bytes, OPCODE_SIZE);
    let target = state
        .resolve_word_pair_source_offset(encoded_offset)
        .ok_or(ScriptInstructionError::InvalidStateWordPair {
            source_offset: token.source_offset(),
            encoded_offset,
        })?;
    Ok(ScriptRecordPairOperation {
        target,
        value: [
            read_word(bytes, OPCODE_SIZE + WORD_SIZE),
            read_word(bytes, OPCODE_SIZE + WORD_SIZE * 2),
        ],
    })
}

/// Decode one C1 action-record operation into a bounded slot and typed operand.
pub fn decode_script_record_state_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptRecordStateOperation, ScriptInstructionError> {
    if token.opcode().byte() != RECORD_STATE_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted);
    require_size(token, RECORD_STATE_SIZE + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    let encoded_target = read_word(bytes, operand_offset);
    let target = resolve_state_word_triple(token, state, encoded_target)?;
    let encoded_operand = read_word(bytes, operand_offset + WORD_SIZE);
    let operand = match encoded_operand {
        PRIMARY_NAVIGATION_OPERAND => ScriptRecordStateOperand::PrimaryNavigationObject,
        SECONDARY_NAVIGATION_OPERAND => ScriptRecordStateOperand::SecondaryNavigationObject,
        _ => directory
            .active_objects()
            .find_map(|(object, entry)| (entry.value == encoded_operand).then_some(object))
            .map(ScriptRecordStateOperand::Object)
            .unwrap_or(ScriptRecordStateOperand::NativeWord(encoded_operand)),
    };
    Ok(ScriptRecordStateOperation {
        target,
        operand,
        inverted,
    })
}

/// Decode one C2 aboard-object operation into bounded typed identities.
pub fn decode_script_aboard_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptAboardRecordOperation, ScriptInstructionError> {
    let (target, related, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        ABOARD_RECORD_OPCODE,
        ABOARD_RECORD_SIZE,
    )?;
    Ok(ScriptAboardRecordOperation {
        target,
        related,
        inverted,
    })
}

/// Decode one C3 queued presentation into bounded typed identities.
pub fn decode_script_presentation_queue_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptPresentationQueueOperation, ScriptInstructionError> {
    let (target, related, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        PRESENTATION_QUEUE_OPCODE,
        PRESENTATION_QUEUE_SIZE,
    )?;
    Ok(ScriptPresentationQueueOperation {
        target,
        related,
        inverted,
    })
}

/// Decode one C4 actor-presentation operation into bounded typed identities.
pub fn decode_script_actor_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptActorRecordOperation, ScriptInstructionError> {
    let (target, related, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        ACTOR_RECORD_OPCODE,
        ACTOR_RECORD_SIZE,
    )?;
    Ok(ScriptActorRecordOperation {
        target,
        related,
        inverted,
    })
}

/// Decode one C5 world-state link into bounded typed identities.
pub fn decode_script_world_state_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptWorldStateRecordOperation, ScriptInstructionError> {
    let (target, related, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        WORLD_STATE_RECORD_OPCODE,
        WORLD_STATE_RECORD_SIZE,
    )?;
    Ok(ScriptWorldStateRecordOperation {
        target,
        related,
        inverted,
    })
}

/// Decode one C6 travel relation into bounded typed identities.
pub fn decode_script_travel_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptTravelRecordOperation, ScriptInstructionError> {
    let (target, destination, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        TRAVEL_RECORD_OPCODE,
        TRAVEL_RECORD_SIZE,
    )?;
    Ok(ScriptTravelRecordOperation {
        target,
        destination,
        inverted,
    })
}

/// Decode one C7 active-object relation into bounded typed identities.
pub fn decode_script_active_object_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
) -> Result<ScriptActiveObjectRecordOperation, ScriptInstructionError> {
    let (target, related, inverted) = decode_object_record_operands(
        token,
        state,
        directory,
        ACTIVE_OBJECT_RECORD_OPCODE,
        ACTIVE_OBJECT_RECORD_SIZE,
    )?;
    Ok(ScriptActiveObjectRecordOperation {
        target,
        related,
        inverted,
    })
}

/// Decode one dormant C8 opaque-marker operation into bounded state.
pub fn decode_script_opaque_marker_record_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptOpaqueMarkerRecordOperation, ScriptInstructionError> {
    if token.opcode().byte() != OPAQUE_MARKER_RECORD_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted);
    require_size(token, OPAQUE_MARKER_RECORD_SIZE + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    Ok(ScriptOpaqueMarkerRecordOperation {
        target: resolve_state_word_triple(token, state, read_word(bytes, operand_offset))?,
        comparison_word: read_word(bytes, operand_offset + WORD_SIZE),
        inverted,
    })
}

/// Decode one C9 clear into a bounded action slot.
pub fn decode_script_record_clear_operation(
    token: &ScriptToken,
    state: &ScriptState,
) -> Result<ScriptRecordClearOperation, ScriptInstructionError> {
    if token.opcode().byte() != RECORD_CLEAR_OPCODE {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    require_size(token, RECORD_CLEAR_SIZE)?;
    Ok(ScriptRecordClearOperation {
        target: resolve_state_word_triple(
            token,
            state,
            read_word(token.encoded_bytes(), OPCODE_SIZE),
        )?,
    })
}

fn decode_object_record_operands(
    token: &ScriptToken,
    state: &ScriptState,
    directory: &ScriptDirectory,
    expected_opcode: u8,
    base_size: usize,
) -> Result<(ScriptStateWordTriple, ScriptObjectId, bool), ScriptInstructionError> {
    if token.opcode().byte() != expected_opcode {
        return Err(ScriptInstructionError::UntranslatedOpcode {
            opcode: token.opcode(),
        });
    }
    let bytes = token.encoded_bytes();
    let inverted = bytes.get(OPCODE_SIZE) == Some(&INVERTED_CONDITION_PREFIX);
    let prefix_size = usize::from(inverted);
    require_size(token, base_size + prefix_size)?;
    let operand_offset = OPCODE_SIZE + prefix_size;
    let target = resolve_state_word_triple(token, state, read_word(bytes, operand_offset))?;
    let object = resolve_active_object(
        token,
        directory,
        read_word(bytes, operand_offset + WORD_SIZE),
    )?;
    Ok((target, object, inverted))
}

fn resolve_active_object(
    token: &ScriptToken,
    directory: &ScriptDirectory,
    encoded_offset: u16,
) -> Result<ScriptObjectId, ScriptInstructionError> {
    directory
        .active_objects()
        .find_map(|(object, entry)| (entry.value == encoded_offset).then_some(object))
        .ok_or(ScriptInstructionError::InvalidObjectReference {
            source_offset: token.source_offset(),
            encoded_offset,
        })
}

fn resolve_state_word_triple(
    token: &ScriptToken,
    state: &ScriptState,
    encoded_offset: u16,
) -> Result<ScriptStateWordTriple, ScriptInstructionError> {
    state
        .resolve_word_triple_source_offset(encoded_offset)
        .ok_or(ScriptInstructionError::InvalidStateWordTriple {
            source_offset: token.source_offset(),
            encoded_offset,
        })
}

const fn is_direct_record_opcode(opcode: u8) -> bool {
    matches!(
        opcode,
        DIRECT_RECORD_A_OPCODE
            | DIRECT_RECORD_B_OPCODE
            | DIRECT_RECORD_C_OPCODE
            | DIRECT_RECORD_D_OPCODE
            | DIRECT_RECORD_E_OPCODE
            | DIRECT_RECORD_F_OPCODE
            | DIRECT_RECORD_TOPIC_OPCODE
    )
}

const fn is_pair_record_opcode(opcode: u8) -> bool {
    matches!(
        opcode,
        PAIR_RECORD_A_OPCODE | PAIR_RECORD_B_OPCODE | PAIR_RECORD_C_OPCODE
    )
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

const fn decode_temporal_relation(tag: u8) -> ScriptTemporalRelation {
    match tag {
        0xF1 => ScriptTemporalRelation::After,
        0xF2 => ScriptTemporalRelation::Before,
        _ => ScriptTemporalRelation::Equal,
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
    use crate::descript::DescriptRecordKind;
    use crate::descript_database::DescriptDatabase;
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
    const EXPECTED_SHARED_BIT_COUNTS: [usize; PROFILE_COUNT] = [0, 69, 46, 32, 24];
    const EXPECTED_DIRECT_RECORD_COUNTS: [usize; PROFILE_COUNT] = [7, 158, 188, 128, 99];
    const EXPECTED_OBJECT_RECORD_COUNT: usize = 531;
    const EXPECTED_TOPIC_RECORD_COUNT: usize = 49;
    const EXPECTED_TRANSFER_COUNTS: [usize; PROFILE_COUNT] = [0, 18, 14, 10, 4];
    const EXPECTED_BIT_FLAG_COUNTS: [usize; PROFILE_COUNT] = [0, 2, 1, 0, 0];
    const EXPECTED_PAIR_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0, 0, 2, 0, 0];
    const EXPECTED_RECORD_STATE_COUNT: usize = 20;
    const EXPECTED_ABOARD_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0, 2, 0, 0, 0];
    const EXPECTED_PRESENTATION_QUEUE_COUNTS: [usize; PROFILE_COUNT] = [1, 14, 11, 7, 2];
    const EXPECTED_ACTOR_RECORD_COUNTS: [usize; PROFILE_COUNT] = [9, 95, 138, 66, 81];
    const EXPECTED_WORLD_STATE_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0; PROFILE_COUNT];
    const EXPECTED_TRAVEL_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0, 0, 1, 1, 0];
    const EXPECTED_ACTIVE_OBJECT_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0; PROFILE_COUNT];
    const EXPECTED_OPAQUE_MARKER_RECORD_COUNTS: [usize; PROFILE_COUNT] = [0; PROFILE_COUNT];
    const EXPECTED_RECORD_CLEAR_COUNTS: [usize; PROFILE_COUNT] = [10, 102, 119, 81, 52];
    const EXPECTED_PROFILE_REQUESTS: [&[i8]; PROFILE_COUNT] =
        [&[2, 2, 2], &[3, 4, 5, 3], &[4], &[5], &[]];
    const EXPECTED_BRIDGE_ACTIVITY_GUARD_COUNTS: [usize; PROFILE_COUNT] = [8, 38, 36, 21, 10];
    const EXPECTED_ALTERNATE_CONCEPT_CLEAR_COUNTS: [usize; PROFILE_COUNT] = [4, 73, 120, 42, 75];
    const EXPECTED_TRAVEL_ACTIVITY_GUARD_COUNTS: [usize; PROFILE_COUNT] = [0, 50, 89, 27, 58];
    const EXPECTED_CONTACT_ACTIVITY_GUARD_COUNTS: [usize; PROFILE_COUNT] = [1, 15, 16, 19, 14];
    const EXPECTED_HOUR_GUARD_COUNTS: [usize; PROFILE_COUNT] = [0, 50, 30, 0, 0];
    const EXPECTED_DATE_GUARD_COUNTS: [usize; PROFILE_COUNT] = [0, 2, 2, 0, 0];
    const EXPECTED_SEQUENCE_SLOT_ASSIGNMENT_COUNTS: [usize; PROFILE_COUNT] = [0, 7, 12, 11, 6];
    const TEST_STATE_WORD_INDEX: usize = 1;
    const EXPECTED_SHIPPED_BIT_FLAG_MASK: u8 = 32;
    const EXPECTED_SHIPPED_PAIR_OPCODE: u8 = PAIR_RECORD_C_OPCODE;
    const MAXIMUM_SHIPPED_SEQUENCE_BASENAME_LENGTH: usize = 12;

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn sequel_settlement_mask_is_immediate_and_requires_its_dialect() {
        use crate::code::{ScriptTokenDecoder, decode_script_token};
        let bytes = [SEQUEL_SETTLEMENT_OPCODE, 0xC0, 0xC2];
        let token = decode_script_token(
            &bytes,
            ScriptCodeOffset::new(0),
            &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang),
        )
        .unwrap();
        assert_eq!(
            decode_script_sequel_settlement(&token).unwrap(),
            ScriptSequelSettlementOperation { group_mask: 49856 }
        );
        assert!(
            decode_script_token(
                &bytes[..2],
                ScriptCodeOffset::new(0),
                &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang)
            )
            .is_err()
        );
        // A legacy adjacent-data descriptor fitting in this buffer is still
        // not permission to interpret a Commander token as a sequel opcode.
        let mut commander_bytes = [0; u8::MAX as usize + 1];
        commander_bytes[..bytes.len()].copy_from_slice(&bytes);
        let commander = decode_script_token(
            &commander_bytes,
            ScriptCodeOffset::new(0),
            &mut ScriptTokenDecoder::default(),
        )
        .unwrap();
        assert!(matches!(
            decode_script_sequel_settlement(&commander),
            Err(ScriptInstructionError::UntranslatedOpcode { .. })
        ));
    }

    #[test]
    #[ignore = "requires original Big Bug Bang COD images under output/big-bug-bang/disc"]
    fn every_authored_sequel_settlement_operation_decodes() {
        use crate::code::decode_script_code_for_dialect;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/disc");
        let mut counts = Vec::new();
        for profile in 1..=17 {
            let bytes = std::fs::read(root.join(format!("SCRIPT{profile}.COD"))).unwrap();
            let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
            let mut count = 0;
            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == SEQUEL_SETTLEMENT_OPCODE)
            {
                decode_script_sequel_settlement(token).unwrap();
                count += 1;
            }
            counts.push(count);
        }
        assert_eq!(counts, [0, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn sequel_growth_operands_are_immediate_signed_words() {
        use crate::code::{ScriptTokenDecoder, decode_script_token};
        let bytes = [SEQUEL_GROWTH_OPCODE, 0xC0, 0xC2, 0, 128];
        let token = decode_script_token(
            &bytes,
            ScriptCodeOffset::new(0),
            &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang),
        )
        .unwrap();
        assert_eq!(
            decode_script_sequel_growth(&token).unwrap(),
            ScriptSequelGrowthOperation {
                group_mask: 49856,
                rate: i16::MIN,
            }
        );
        assert!(
            decode_script_token(
                &bytes[..4],
                ScriptCodeOffset::new(0),
                &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang)
            )
            .is_err()
        );
        // Commander's adjacent-data descriptor spans 105 bytes at D6; it is
        // not a native D6 operation, even if that legacy framing succeeds.
        let mut commander_bytes = [0; 105];
        commander_bytes[..bytes.len()].copy_from_slice(&bytes);
        let commander = decode_script_token(
            &commander_bytes,
            ScriptCodeOffset::new(0),
            &mut ScriptTokenDecoder::default(),
        )
        .unwrap();
        assert!(matches!(
            decode_script_sequel_growth(&commander),
            Err(ScriptInstructionError::UntranslatedOpcode { .. })
        ));
    }

    #[test]
    #[ignore = "requires original Big Bug Bang COD images under output/big-bug-bang/disc"]
    fn every_authored_sequel_growth_operation_decodes() {
        use crate::code::decode_script_code_for_dialect;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/disc");
        let mut counts = Vec::new();
        for profile in 1..=17 {
            let bytes = std::fs::read(root.join(format!("SCRIPT{profile}.COD"))).unwrap();
            let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang).unwrap();
            let mut count = 0;
            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == SEQUEL_GROWTH_OPCODE)
            {
                decode_script_sequel_growth(token).unwrap();
                count += 1;
            }
            counts.push(count);
        }
        assert_eq!(counts, [0, 39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn multiply_divide_requires_the_sequel_dialect_and_valid_state_words() {
        use crate::code::{ScriptTokenDecoder, decode_script_token};
        use crate::script::decode_script_dictionary;
        let directory = decode_script_directory(&[]).unwrap();
        let state = decode_script_state(&[0; 8], &directory).unwrap();
        let dictionary = decode_script_dictionary(&[]).unwrap();
        let commander = decode_script_token(
            &[MULTIPLY_DIVIDE_OPCODE, 0, 0],
            ScriptCodeOffset::new(0),
            &mut ScriptTokenDecoder::default(),
        )
        .unwrap();
        assert!(matches!(
            decode_complete_script_instruction(&commander, &state, &directory, &dictionary),
            Err(ScriptInstructionError::UntranslatedOpcode { .. })
        ));
        for operand_index in [1, 4, 7] {
            let mut bytes = [
                MULTIPLY_DIVIDE_OPCODE,
                2,
                0,
                INDIRECT_STATE_MODE_A,
                4,
                0,
                INDIRECT_STATE_MODE_B,
                6,
                0,
            ];
            bytes[operand_index] = 8;
            let token = decode_script_token(
                &bytes,
                ScriptCodeOffset::new(0),
                &mut ScriptTokenDecoder::new(ScriptDialect::BigBugBang),
            )
            .unwrap();
            assert!(matches!(
                decode_script_multiply_divide(&token, &state),
                Err(ScriptInstructionError::InvalidStateWord {
                    encoded_offset: 8,
                    ..
                })
            ));
        }
    }

    fn typed_object_record_fixture(
        related_kind: crate::script::ScriptObjectKind,
    ) -> (
        ScriptDirectory,
        ScriptState,
        ScriptObjectId,
        ScriptObjectId,
        u16,
        u16,
    ) {
        let directory =
            decode_script_directory(&std::fs::read(original_asset("SCRIPT1.DEB")).unwrap())
                .unwrap();
        let state = decode_script_state(
            &std::fs::read(original_asset("SCRIPT1.VAR")).unwrap(),
            &directory,
        )
        .unwrap();
        let owner = state
            .objects()
            .iter()
            .find(|object| object.kind == crate::script::ScriptObjectKind::Actor)
            .unwrap();
        let related = state
            .objects()
            .iter()
            .find(|object| object.kind == related_kind)
            .unwrap();
        let owner_id = owner.id;
        let related_id = related.id;
        let target_offset = u16::try_from(
            owner.source_offset() + TEST_STATE_WORD_INDEX * std::mem::size_of::<u16>(),
        )
        .unwrap();
        let related_offset = directory.object(related_id).unwrap().value;
        (
            directory,
            state,
            owner_id,
            related_id,
            target_offset,
            related_offset,
        )
    }

    fn encoded_object_record(
        opcode: u8,
        target_offset: u16,
        related_offset: u16,
        inverted: bool,
    ) -> Vec<u8> {
        let mut bytes = vec![opcode];
        if inverted {
            bytes.push(INVERTED_CONDITION_PREFIX);
        }
        bytes.extend_from_slice(&target_offset.to_le_bytes());
        bytes.extend_from_slice(&related_offset.to_le_bytes());
        bytes.push(CODE_END_MARKER);
        bytes
    }

    fn shipped_opcode_counts(opcode: u8) -> [usize; PROFILE_COUNT] {
        let mut counts = [usize::MIN; PROFILE_COUNT];
        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            counts[profile - 1] = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == opcode)
                .count();
        }
        counts
    }

    #[test]
    fn every_shipped_cod_token_has_one_complete_typed_decoding() {
        let mut shipped_opcodes = BTreeSet::new();
        let mut decoded_token_count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let dictionary = decode_script_dictionary(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap(),
            )
            .unwrap();
            let state = decode_script_state(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap(),
                &directory,
            )
            .unwrap();

            for token in code.tokens() {
                decode_complete_script_instruction(token, &state, &directory, &dictionary)
                    .unwrap_or_else(|error| {
                        panic!(
                            "profile {profile} token {} opcode {:#04X} lacks complete semantics: {error}",
                            token.source_offset().index(),
                            token.opcode().byte()
                        )
                    });
                shipped_opcodes.insert(token.opcode().byte());
                decoded_token_count += 1;
            }
        }

        assert!(decoded_token_count > usize::MIN);
        assert!(!shipped_opcodes.contains(&0xAC));
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
    fn every_shipped_shared_bit_token_resolves_to_a_typed_var_word() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

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
                    SHARED_BIT_STATE_A_OPCODE | SHARED_BIT_STATE_B_OPCODE
                )
            }) {
                decode_script_shared_bit_operation(token, &state).unwrap();
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_SHARED_BIT_COUNTS);
    }

    #[test]
    fn every_shipped_direct_record_token_has_typed_relationship_semantics() {
        let mut counts = [usize::MIN; PROFILE_COUNT];
        let mut object_records = usize::MIN;
        let mut topic_records = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let dictionary_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| is_direct_record_opcode(token.opcode().byte()))
            {
                let operation =
                    decode_script_direct_record_operation(token, &state, &directory, &dictionary)
                        .unwrap();
                assert!(operation.target.object().is_some());
                assert_ne!(operation.target.word_index(), usize::MIN);
                match (token.opcode().byte(), operation.value) {
                    (
                        DIRECT_RECORD_B_OPCODE,
                        ScriptRecordValue::Object(_) | ScriptRecordValue::Aboard,
                    ) => object_records += 1,
                    (DIRECT_RECORD_TOPIC_OPCODE, ScriptRecordValue::Topic(_)) => topic_records += 1,
                    unexpected => panic!("untyped shipped record operation: {unexpected:?}"),
                }
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_DIRECT_RECORD_COUNTS);
        assert_eq!(object_records, EXPECTED_OBJECT_RECORD_COUNT);
        assert_eq!(topic_records, EXPECTED_TOPIC_RECORD_COUNT);
    }

    #[test]
    fn every_shipped_cod_transfer_resolves_to_typed_inventory_relationships() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == TRANSFER_OPCODE)
            {
                let transfer = decode_script_transfer(token, &state, &directory).unwrap();
                let source = transfer.source_record.object().unwrap();
                assert_ne!(transfer.source_record.word_index(), usize::MIN);
                assert!(matches!(
                    state.object(source).unwrap().kind,
                    crate::script::ScriptObjectKind::Player
                        | crate::script::ScriptObjectKind::Actor
                ));
                assert_eq!(
                    state.object(transfer.item).unwrap().kind,
                    crate::script::ScriptObjectKind::InventoryItem
                );
                assert!(matches!(
                    state.object(transfer.destination).unwrap().kind,
                    crate::script::ScriptObjectKind::Player
                        | crate::script::ScriptObjectKind::Actor
                ));
                assert!(!transfer.inverted);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_TRANSFER_COUNTS);
    }

    #[test]
    fn every_shipped_bit_flag_resolves_to_one_typed_object_byte() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == BIT_FLAG_OPCODE)
            {
                let operation = decode_script_bit_flag_operation(token, &state).unwrap();
                assert!(operation.target.object().is_some());
                assert_eq!(operation.mask, EXPECTED_SHIPPED_BIT_FLAG_MASK);
                assert!(!operation.inverted_or_clear);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_BIT_FLAG_COUNTS);
    }

    #[test]
    fn every_shipped_pair_record_resolves_within_one_typed_object() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| is_pair_record_opcode(token.opcode().byte()))
            {
                let operation = decode_script_record_pair_operation(token, &state).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(owner, directory.find_active_object(b"Kraner").unwrap());
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::NavigationEntity
                );
                assert_eq!(token.opcode().byte(), EXPECTED_SHIPPED_PAIR_OPCODE);
                assert_eq!(operation.target.first_word_index(), 12);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_PAIR_RECORD_COUNTS);
    }

    #[test]
    fn every_shipped_c1_record_is_a_typed_navigation_request() {
        let mut count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == RECORD_STATE_OPCODE)
            {
                let operation =
                    decode_script_record_state_operation(token, &state, &directory).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::WorldState
                );
                let ScriptRecordStateOperand::Object(destination) = operation.operand else {
                    panic!("shipped C1 operand is not a typed object");
                };
                assert_eq!(
                    state.object(destination).unwrap().kind,
                    crate::script::ScriptObjectKind::Location
                );
                assert!(!operation.inverted);
                count += 1;
            }
        }

        assert_eq!(count, EXPECTED_RECORD_STATE_COUNT);
    }

    #[test]
    fn every_shipped_c2_record_is_a_typed_aboard_transfer() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == ABOARD_RECORD_OPCODE)
            {
                let operation =
                    decode_script_aboard_record_operation(token, &state, &directory).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::Player
                );
                assert_eq!(
                    state.object(operation.related).unwrap().kind,
                    crate::script::ScriptObjectKind::Actor
                );
                assert_eq!(token.mode_before(), ScriptDecodingMode::Normal);
                assert!(!operation.inverted);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_ABOARD_RECORD_COUNTS);
    }

    #[test]
    fn every_shipped_c3_record_is_a_typed_presentation_queue() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == PRESENTATION_QUEUE_OPCODE)
            {
                let operation =
                    decode_script_presentation_queue_operation(token, &state, &directory).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::Actor
                );
                assert_eq!(
                    state.object(operation.related).unwrap().kind,
                    crate::script::ScriptObjectKind::Player
                );
                assert_eq!(token.mode_before(), ScriptDecodingMode::Normal);
                assert!(!operation.inverted);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_PRESENTATION_QUEUE_COUNTS);
    }

    #[test]
    fn every_shipped_c4_record_is_a_typed_actor_request() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == ACTOR_RECORD_OPCODE)
            {
                let operation =
                    decode_script_actor_record_operation(token, &state, &directory).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::Actor
                );
                assert_eq!(
                    state.object(operation.related).unwrap().kind,
                    crate::script::ScriptObjectKind::Player
                );
                assert_eq!(state.word_triple(operation.target), Some([u16::MIN; 3]));
                assert!(!operation.inverted);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_ACTOR_RECORD_COUNTS);
    }

    #[test]
    fn c5_has_typed_plain_and_inverted_forms_but_no_shipped_sites() {
        let (directory, state, owner, related, target_offset, related_offset) =
            typed_object_record_fixture(crate::script::ScriptObjectKind::WorldState);

        let plain = decode_script_code(&encoded_object_record(
            WORLD_STATE_RECORD_OPCODE,
            target_offset,
            related_offset,
            false,
        ))
        .unwrap();
        let plain =
            decode_script_world_state_record_operation(&plain.tokens()[0], &state, &directory)
                .unwrap();
        assert_eq!(plain.target.object(), Some(owner));
        assert_eq!(plain.related, related);
        assert!(!plain.inverted);

        let mut query_bytes = vec![GUARD_BEGIN_OPCODE, u8::MIN, u8::MIN];
        query_bytes.extend_from_slice(&encoded_object_record(
            WORLD_STATE_RECORD_OPCODE,
            target_offset,
            related_offset,
            true,
        ));
        let inverted = decode_script_code(&query_bytes).unwrap();
        let inverted =
            decode_script_world_state_record_operation(&inverted.tokens()[1], &state, &directory)
                .unwrap();
        assert!(inverted.inverted);

        assert_eq!(
            shipped_opcode_counts(WORLD_STATE_RECORD_OPCODE),
            EXPECTED_WORLD_STATE_RECORD_COUNTS
        );
    }

    #[test]
    fn every_shipped_c6_record_is_a_typed_travel_guard() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == TRAVEL_RECORD_OPCODE)
            {
                let operation =
                    decode_script_travel_record_operation(token, &state, &directory).unwrap();
                let owner = operation.target.object().unwrap();
                assert_eq!(
                    state.object(owner).unwrap().kind,
                    crate::script::ScriptObjectKind::NavigationEntity
                );
                assert_eq!(
                    state.object(operation.destination).unwrap().kind,
                    crate::script::ScriptObjectKind::BlackHole
                );
                assert_eq!(token.mode_before(), ScriptDecodingMode::Query);
                assert_eq!(state.word_triple(operation.target), Some([u16::MIN; 3]));
                assert!(!operation.inverted);
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_TRAVEL_RECORD_COUNTS);
    }

    #[test]
    fn c7_has_typed_plain_and_inverted_forms_but_no_shipped_sites() {
        let (directory, state, owner, related, target_offset, related_offset) =
            typed_object_record_fixture(crate::script::ScriptObjectKind::WorldState);
        let plain = decode_script_code(&encoded_object_record(
            ACTIVE_OBJECT_RECORD_OPCODE,
            target_offset,
            related_offset,
            false,
        ))
        .unwrap();
        let plain =
            decode_script_active_object_record_operation(&plain.tokens()[0], &state, &directory)
                .unwrap();
        assert_eq!(plain.target.object(), Some(owner));
        assert_eq!(plain.related, related);
        assert!(!plain.inverted);

        let mut query_bytes = vec![GUARD_BEGIN_OPCODE, u8::MIN, u8::MIN];
        query_bytes.extend_from_slice(&encoded_object_record(
            ACTIVE_OBJECT_RECORD_OPCODE,
            target_offset,
            related_offset,
            true,
        ));
        let inverted = decode_script_code(&query_bytes).unwrap();
        let inverted =
            decode_script_active_object_record_operation(&inverted.tokens()[1], &state, &directory)
                .unwrap();
        assert!(inverted.inverted);

        assert_eq!(
            shipped_opcode_counts(ACTIVE_OBJECT_RECORD_OPCODE),
            EXPECTED_ACTIVE_OBJECT_RECORD_COUNTS
        );
    }

    #[test]
    fn c8_has_typed_plain_and_inverted_forms_but_no_shipped_sites() {
        const COMPARISON_WORD: u16 = 42_424;

        let (_directory, state, owner, _related, target_offset, _related_offset) =
            typed_object_record_fixture(crate::script::ScriptObjectKind::WorldState);
        let plain = decode_script_code(&encoded_object_record(
            OPAQUE_MARKER_RECORD_OPCODE,
            target_offset,
            COMPARISON_WORD,
            false,
        ))
        .unwrap();
        let plain =
            decode_script_opaque_marker_record_operation(&plain.tokens()[0], &state).unwrap();
        assert_eq!(plain.target.object(), Some(owner));
        assert_eq!(plain.comparison_word, COMPARISON_WORD);
        assert!(!plain.inverted);

        let mut query_bytes = vec![GUARD_BEGIN_OPCODE, u8::MIN, u8::MIN];
        query_bytes.extend_from_slice(&encoded_object_record(
            OPAQUE_MARKER_RECORD_OPCODE,
            target_offset,
            COMPARISON_WORD,
            true,
        ));
        let inverted = decode_script_code(&query_bytes).unwrap();
        let inverted =
            decode_script_opaque_marker_record_operation(&inverted.tokens()[1], &state).unwrap();
        assert!(inverted.inverted);

        assert_eq!(
            shipped_opcode_counts(OPAQUE_MARKER_RECORD_OPCODE),
            EXPECTED_OPAQUE_MARKER_RECORD_COUNTS
        );
    }

    #[test]
    fn every_shipped_cod_c9_clear_resolves_to_a_typed_action_slot() {
        let mut counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code_data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let directory_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let state_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let code = decode_script_code(&code_data).unwrap();
            let directory = decode_script_directory(&directory_data).unwrap();
            let state = decode_script_state(&state_data, &directory).unwrap();

            for token in code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == RECORD_CLEAR_OPCODE)
            {
                let operation = decode_script_record_clear_operation(token, &state).unwrap();
                assert!(operation.target.object().is_some());
                counts[profile - 1] += 1;
            }
        }

        assert_eq!(counts, EXPECTED_RECORD_CLEAR_COUNTS);
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
    fn every_shipped_d2_request_resolves_to_a_playable_profile_number() {
        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let requests = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == PROFILE_REQUEST_OPCODE)
                .map(|token| {
                    decode_script_profile_request(token)
                        .unwrap()
                        .one_based_profile_number()
                })
                .collect::<Vec<_>>();

            assert_eq!(requests, EXPECTED_PROFILE_REQUESTS[profile - 1]);
        }
    }

    #[test]
    fn every_shipped_ce_through_d1_environment_token_has_typed_semantics() {
        let expected = [
            (
                BRIDGE_ACTIVITY_GUARD_OPCODE,
                ScriptEnvironmentInstruction::RequireBridgeActivity,
                EXPECTED_BRIDGE_ACTIVITY_GUARD_COUNTS,
            ),
            (
                ALTERNATE_CONCEPT_CLEAR_OPCODE,
                ScriptEnvironmentInstruction::ClearAlternateConcept,
                EXPECTED_ALTERNATE_CONCEPT_CLEAR_COUNTS,
            ),
            (
                TRAVEL_ACTIVITY_GUARD_OPCODE,
                ScriptEnvironmentInstruction::RequireTravelActivity,
                EXPECTED_TRAVEL_ACTIVITY_GUARD_COUNTS,
            ),
            (
                CONTACT_ACTIVITY_GUARD_OPCODE,
                ScriptEnvironmentInstruction::RequireContactActivity,
                EXPECTED_CONTACT_ACTIVITY_GUARD_COUNTS,
            ),
        ];

        for (opcode, expected_instruction, expected_counts) in expected {
            let mut actual_counts = [usize::MIN; PROFILE_COUNT];
            for profile in 1..=PROFILE_COUNT {
                let code = decode_script_code(
                    &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
                )
                .unwrap();
                for token in code
                    .tokens()
                    .iter()
                    .filter(|token| token.opcode().byte() == opcode)
                {
                    assert_eq!(
                        decode_script_environment_instruction(token).unwrap(),
                        expected_instruction
                    );
                    actual_counts[profile - 1] += 1;
                }
            }
            assert_eq!(actual_counts, expected_counts);
        }
    }

    #[test]
    fn every_shipped_ca_and_cb_token_has_typed_clock_semantics() {
        let mut hour_counts = [usize::MIN; PROFILE_COUNT];
        let mut date_counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            for token in code.tokens() {
                match token.opcode().byte() {
                    HOUR_GUARD_OPCODE => {
                        let guard = decode_script_hour_guard(token).unwrap();
                        assert!((-128..=127).contains(&guard.hour()));
                        hour_counts[profile - 1] += 1;
                    }
                    DATE_GUARD_OPCODE => {
                        let guard = decode_script_date_guard(token).unwrap();
                        assert!((1994..=1995).contains(&guard.encoded_year()));
                        date_counts[profile - 1] += 1;
                    }
                    _ => {}
                }
            }
        }

        assert_eq!(hour_counts, EXPECTED_HOUR_GUARD_COUNTS);
        assert_eq!(date_counts, EXPECTED_DATE_GUARD_COUNTS);
    }

    #[test]
    fn every_shipped_cc_token_names_a_descript_sequence_in_a_bounded_slot() {
        let descript =
            DescriptDatabase::parse(&std::fs::read(original_asset("DESCRIPT.DES")).unwrap())
                .unwrap();
        let sequence_names = descript
            .records()
            .iter()
            .filter(|record| record.kind() == DescriptRecordKind::Sequence)
            .map(|record| record.name())
            .collect::<BTreeSet<_>>();
        let mut assignment_counts = [usize::MIN; PROFILE_COUNT];

        for profile in 1..=PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            for token in code.tokens() {
                if token.opcode().byte() != SEQUENCE_SLOT_ASSIGNMENT_OPCODE {
                    continue;
                }
                let assignment = decode_script_sequence_slot_assignment(token).unwrap();
                assert!(
                    (FIRST_SEQUENCE_SLOT..=LAST_SEQUENCE_SLOT)
                        .contains(&assignment.slot().encode())
                );
                assert!(
                    assignment.name().as_bytes().len()
                        <= ScriptSequenceSlotName::MAXIMUM_BYTE_LENGTH
                );
                assert!(sequence_names.contains(assignment.name().as_bytes()));
                assignment_counts[profile - 1] += 1;
            }
        }

        assert_eq!(assignment_counts, EXPECTED_SEQUENCE_SLOT_ASSIGNMENT_COUNTS);
    }

    #[test]
    fn cc_decoder_rejects_native_adjacent_memory_writes() {
        for encoded_slot in [u8::MIN, 7, 128, 129, u8::MAX] {
            let code = decode_script_code(&[
                SEQUENCE_SLOT_ASSIGNMENT_OPCODE,
                encoded_slot,
                b'x',
                u8::MIN,
                u8::MIN,
                CODE_END_MARKER,
            ])
            .unwrap();
            assert_eq!(
                decode_script_sequence_slot_assignment(&code.tokens()[0]).unwrap_err(),
                ScriptInstructionError::InvalidSequenceSlot {
                    source_offset: ScriptCodeOffset::new(usize::MIN),
                    encoded: encoded_slot,
                }
            );
        }

        let mut overlong = vec![SEQUENCE_SLOT_ASSIGNMENT_OPCODE, FIRST_SEQUENCE_SLOT];
        overlong.extend(std::iter::repeat_n(
            b'x',
            ScriptSequenceSlotName::MAXIMUM_BYTE_LENGTH + 1,
        ));
        overlong.extend_from_slice(&[u8::MIN, u8::MIN, CODE_END_MARKER]);
        let code = decode_script_code(&overlong).unwrap();
        assert_eq!(
            decode_script_sequence_slot_assignment(&code.tokens()[0]).unwrap_err(),
            ScriptInstructionError::InvalidSequenceSlotName {
                source_offset: ScriptCodeOffset::new(usize::MIN),
                byte_length: ScriptSequenceSlotName::MAXIMUM_BYTE_LENGTH + 1,
            }
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
