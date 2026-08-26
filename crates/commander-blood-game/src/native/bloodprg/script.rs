//! Flat runtime state for BloodScript control-flow instructions.

use std::fmt;

use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{ScriptInstruction, ScriptTimerSlot};
use commander_blood_formats::script::ScriptWordId;

use crate::native::random::BloodPrng;

/// Byte count of the timer and reserved-state block stored in original saves.
pub const SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT: usize = 512;

const SCRIPT_TIMER_WORD_BYTE_COUNT: usize = 2;
const SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT: usize =
    ScriptTimerSlot::COUNT * SCRIPT_TIMER_WORD_BYTE_COUNT;
const SCRIPT_TIMER_SAVE_RESERVED_BYTE_COUNT: usize =
    SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT - SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT;

/// Program-counter action produced by one translated BloodScript handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptControl {
    /// Continue with the next framed instruction.
    Continue,
    /// Continue at a typed position in the current COD image.
    Jump(ScriptCodeOffset),
}

/// Resume destination and value retained across a yielded script pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptResumeState {
    /// Typed destination in the current COD image.
    pub target: ScriptCodeOffset,
    /// Instruction position saved after a presentation-producing handler.
    pub saved_cursor: Option<ScriptCodeOffset>,
    /// Value selected by the presentation path before execution resumes.
    pub value: u16,
    /// Semantic phase represented by the native resume-state byte.
    pub phase: ScriptResumePhase,
}

/// Distinct behavior selected by the native resume-state values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptResumePhase {
    /// A6 armed a loop target; ordinary execution can consume it once.
    LoopArmed,
    /// A selector body yielded and later concept guards use its saved choice.
    SelectorResumeActive,
}

/// Invalid typed runtime state detected while applying an instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRuntimeError {
    /// A failing condition has no guard target to consume.
    MissingGuardTarget,
}

impl fmt::Display for ScriptRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptRuntimeError {}

/// Owned state used by the translated BloodScript control-flow handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptRuntime {
    query_mode: bool,
    guard_targets: Vec<ScriptCodeOffset>,
    selected_concept: Option<ScriptWordId>,
    alternate_concept: Option<ScriptWordId>,
    resume: Option<ScriptResumeState>,
    retained_resume_target: Option<ScriptCodeOffset>,
    pending_skip_count: Option<u8>,
    yield_requested: bool,
    timer_words: [u16; ScriptTimerSlot::COUNT],
    timer_save_reserved_bytes: [u8; SCRIPT_TIMER_SAVE_RESERVED_BYTE_COUNT],
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRuntime {
    /// Construct the state established when the original selects a script profile.
    pub const fn new() -> Self {
        Self {
            query_mode: false,
            guard_targets: Vec::new(),
            selected_concept: None,
            alternate_concept: None,
            resume: None,
            retained_resume_target: None,
            pending_skip_count: None,
            yield_requested: false,
            timer_words: [u16::MAX; ScriptTimerSlot::COUNT],
            timer_save_reserved_bytes: [u8::MAX; SCRIPT_TIMER_SAVE_RESERVED_BYTE_COUNT],
        }
    }

    /// Return whether conditional-query handlers are active.
    pub const fn query_mode(&self) -> bool {
        self.query_mode
    }

    /// Return the number of active root and nested guard targets.
    pub fn guard_depth(&self) -> usize {
        self.guard_targets.len()
    }

    /// Return the current resume destination, when one is armed.
    pub const fn resume_target(&self) -> Option<ScriptCodeOffset> {
        match self.resume {
            Some(resume) => Some(resume.target),
            None => None,
        }
    }

    /// Return the complete pending resume state.
    pub const fn resume_state(&self) -> Option<ScriptResumeState> {
        self.resume
    }

    /// Return the cursor saved by the most recent presentation yield.
    pub const fn saved_resume_cursor(&self) -> Option<ScriptCodeOffset> {
        match self.resume {
            Some(resume) => resume.saved_cursor,
            None => None,
        }
    }

    /// Return the number of framed instructions to skip after this handler.
    pub const fn pending_skip_count(&self) -> Option<u8> {
        self.pending_skip_count
    }

    /// Return the alternate concept selected by resumed presentation handling.
    pub const fn alternate_concept(&self) -> Option<ScriptWordId> {
        self.alternate_concept
    }

    /// Return whether execution must stop at the end of the current instruction.
    pub const fn yield_requested(&self) -> bool {
        self.yield_requested
    }

    /// Set the primary concept chosen by the player.
    pub fn set_selected_concept(&mut self, concept: Option<ScriptWordId>) {
        self.selected_concept = concept;
    }

    /// Return the primary concept awaiting selector dispatch.
    pub const fn selected_concept(&self) -> Option<ScriptWordId> {
        self.selected_concept
    }

    /// Set the alternate concept used by resumed menu handling.
    pub fn set_alternate_concept(&mut self, concept: Option<ScriptWordId>) {
        self.alternate_concept = concept;
    }

    /// Clear the alternate concept and the resume state it shares in the native VM.
    pub fn clear_alternate_resume_state(&mut self) {
        self.alternate_concept = None;
        self.resume = None;
    }

    /// Arm a destination used by presentation resume logic.
    pub fn set_resume_target(&mut self, target: Option<ScriptCodeOffset>) {
        self.retained_resume_target = target.or(self.retained_resume_target);
        self.resume = target.map(|target| ScriptResumeState {
            target,
            saved_cursor: None,
            value: u16::MIN,
            phase: ScriptResumePhase::LoopArmed,
        });
    }

    /// Arm a presentation resume destination with its selected value.
    pub fn arm_resume(&mut self, target: ScriptCodeOffset, value: u16) {
        self.retained_resume_target = Some(target);
        self.resume = Some(ScriptResumeState {
            target,
            saved_cursor: None,
            value,
            phase: ScriptResumePhase::LoopArmed,
        });
    }

    /// Retain the native loop destination without arming a resume phase.
    ///
    /// The original keeps this destination in a distinct global after a loop
    /// completes. Save restoration and the frame executor use this method to
    /// represent that persistent authored destination directly.
    pub fn retain_resume_target(&mut self, target: ScriptCodeOffset) {
        self.retained_resume_target = Some(target);
    }

    /// Save a post-handler cursor and advance the semantic resume phase.
    pub(crate) fn save_resume_cursor(&mut self, cursor: ScriptCodeOffset) -> bool {
        if let Some(resume) = &mut self.resume {
            resume.saved_cursor = Some(cursor);
            if resume.phase == ScriptResumePhase::LoopArmed {
                resume.phase = ScriptResumePhase::SelectorResumeActive;
            }
            return true;
        }

        let Some(target) = self.retained_resume_target else {
            return false;
        };
        self.resume = Some(ScriptResumeState {
            target,
            saved_cursor: Some(cursor),
            value: u16::MIN,
            phase: ScriptResumePhase::LoopArmed,
        });
        true
    }

    /// Consume a loop-armed destination after a non-yielding instruction.
    pub(crate) fn take_loop_resume_target(&mut self) -> Option<ScriptCodeOffset> {
        if !matches!(
            self.resume,
            Some(ScriptResumeState {
                phase: ScriptResumePhase::LoopArmed,
                ..
            })
        ) {
            return None;
        }
        self.resume.take().map(|resume| resume.target)
    }

    /// Promote a loop-armed resume to the selector-active phase.
    pub fn activate_selector_resume(&mut self) -> bool {
        let Some(resume) = &mut self.resume else {
            return false;
        };
        resume.phase = ScriptResumePhase::SelectorResumeActive;
        true
    }

    /// Return whether selector guards must use the saved alternate concept.
    pub const fn selector_resume_active(&self) -> bool {
        matches!(
            self.resume,
            Some(ScriptResumeState {
                phase: ScriptResumePhase::SelectorResumeActive,
                ..
            })
        )
    }

    /// Save the selected concept for resumed selector guards.
    pub fn save_resume_concept(&mut self, concept: ScriptWordId, encoded_value: u16) -> bool {
        let Some(resume) = &mut self.resume else {
            return false;
        };
        if resume.phase != ScriptResumePhase::SelectorResumeActive {
            return false;
        }
        resume.value = encoded_value;
        self.alternate_concept = Some(concept);
        true
    }

    /// Consume the primary concept selected during the current presentation pass.
    pub fn take_selected_concept(&mut self) -> Option<ScriptWordId> {
        self.selected_concept.take()
    }

    /// Arm an authored number of framed instructions to skip.
    pub fn arm_skip(&mut self, count: u8) {
        self.pending_skip_count = Some(count);
    }

    /// Consume a pending skip count when its recovered activity bits are set.
    ///
    /// Authored counts use the low four bits, while the original block walker
    /// decremented the complete byte after entering the skip path. Retaining
    /// that distinction keeps malformed or externally restored state explicit
    /// without recreating the native machine representation.
    pub(crate) fn take_actionable_skip_count(&mut self) -> Option<u8> {
        const SKIP_ACTIVITY_MASK: u8 = 0x0F;

        self.pending_skip_count
            .is_some_and(|count| count & SKIP_ACTIVITY_MASK != u8::MIN)
            .then(|| {
                self.pending_skip_count
                    .take()
                    .expect("checked pending skip remains present")
            })
    }

    /// Discard pending token skips after a presentation continuation.
    pub(crate) fn clear_pending_skip_count(&mut self) {
        self.pending_skip_count = None;
    }

    /// Apply `vm_op_aa_yield` using a flat execution-state flag.
    pub fn request_yield(&mut self) {
        self.yield_requested = true;
    }

    /// Apply the distinct `vm_op_ac_yield` entry before BAS selector dispatch.
    pub fn request_selector_yield(&mut self) {
        self.yield_requested = true;
    }

    /// Consume and clear the current execution-pass yield request.
    pub fn take_yield_request(&mut self) -> bool {
        std::mem::take(&mut self.yield_requested)
    }

    /// Read one proven transient timer/state slot.
    pub fn timer(&self, slot: ScriptTimerSlot) -> u16 {
        self.timer_words[slot.index()]
    }

    /// Encode the complete fixed timer block written by the original save path.
    ///
    /// BloodScript can address the first 128 little-endian words. The remaining
    /// 256 bytes have no direct or immediate-base accesses in `BLOODPRG.EXE`,
    /// but the native save and load routines copy them as part of the same
    /// 512-byte region. They therefore remain opaque persistent bytes rather
    /// than becoming invented gameplay fields.
    pub fn encode_timer_save_block(&self) -> [u8; SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT] {
        let mut block = [u8::MIN; SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT];
        for (word, destination) in self.timer_words.iter().zip(
            block[..SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT]
                .chunks_exact_mut(SCRIPT_TIMER_WORD_BYTE_COUNT),
        ) {
            destination.copy_from_slice(&word.to_le_bytes());
        }
        block[SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT..]
            .copy_from_slice(&self.timer_save_reserved_bytes);
        block
    }

    /// Restore the complete fixed timer block read by the original load path.
    pub fn restore_timer_save_block(&mut self, block: &[u8; SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT]) {
        for (source, word) in block[..SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT]
            .chunks_exact(SCRIPT_TIMER_WORD_BYTE_COUNT)
            .zip(&mut self.timer_words)
        {
            *word = u16::from_le_bytes(
                source
                    .try_into()
                    .expect("fixed two-byte serialized timer word"),
            );
        }
        self.timer_save_reserved_bytes
            .copy_from_slice(&block[SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT..]);
    }

    pub(super) fn preserve_timer_save_reserved_bytes(&mut self, previous: &Self) {
        self.timer_save_reserved_bytes = previous.timer_save_reserved_bytes;
    }

    /// Apply any instruction currently represented by the typed control IR.
    pub fn apply_instruction(
        &mut self,
        instruction: &ScriptInstruction,
        random: &mut BloodPrng,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        match *instruction {
            ScriptInstruction::GuardBegin { failure_target } => {
                self.begin_guard(failure_target);
                Ok(ScriptControl::Continue)
            }
            ScriptInstruction::GuardEnd => {
                self.end_guard();
                Ok(ScriptControl::Continue)
            }
            ScriptInstruction::RandomGuard { modulus } => self.random_guard(modulus, random),
            ScriptInstruction::ConceptGuard { expected, inverted } => {
                self.concept_guard(expected, inverted)
            }
            ScriptInstruction::Jump { target } => Ok(self.jump(target)),
            ScriptInstruction::TimerGuard { slot } => self.timer_state(slot, None),
            ScriptInstruction::TimerAssignment { slot, value } => {
                self.timer_state(slot, Some(value))
            }
            ScriptInstruction::Yield => {
                self.request_yield();
                Ok(ScriptControl::Continue)
            }
        }
    }

    /// Enter query mode and push one guard-failure destination.
    pub fn begin_guard(&mut self, failure_target: ScriptCodeOffset) {
        self.query_mode = true;
        self.guard_targets.push(failure_target);
    }

    /// Replace any prior guard stack with one procedure-root failure target.
    pub fn begin_root_guard(&mut self, failure_target: ScriptCodeOffset) {
        self.query_mode = true;
        self.guard_targets.clear();
        self.guard_targets.push(failure_target);
    }

    /// Install a root failure target without changing assignment/query mode.
    ///
    /// Native procedure execution can retain a branch destination while the
    /// low query bit is clear; state-record assignment failures consume that
    /// destination through the same branch helper as failed conditions.
    pub fn arm_root_failure_target(&mut self, failure_target: ScriptCodeOffset) {
        self.guard_targets.clear();
        self.guard_targets.push(failure_target);
    }

    /// Return the current innermost guard target.
    pub fn current_guard_target(&self) -> Option<ScriptCodeOffset> {
        self.guard_targets.last().copied()
    }

    /// Leave query mode and discard only a nested guard target.
    ///
    /// The first target is retained as the current root, matching the native
    /// handler's distinguished two-byte stack-top value.
    pub fn end_guard(&mut self) {
        self.query_mode = false;
        if self.guard_targets.len() > 1 {
            self.guard_targets.pop();
        }
    }

    /// Consume the current guard target after a failed condition.
    pub fn fail_guard(&mut self) -> Result<ScriptControl, ScriptRuntimeError> {
        let target = self
            .guard_targets
            .pop()
            .ok_or(ScriptRuntimeError::MissingGuardTarget)?;
        self.query_mode = false;
        Ok(ScriptControl::Jump(target))
    }

    /// Continue only when the recovered PRNG returns zero for the modulus.
    pub fn random_guard(
        &mut self,
        modulus: u16,
        random: &mut BloodPrng,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        self.apply_random_result(random.next(modulus))
    }

    /// Compare the active concept identity using the original optional inversion.
    pub fn concept_guard(
        &mut self,
        expected: ScriptWordId,
        inverted: bool,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        let selected = self.alternate_concept.or(self.selected_concept);
        let matches = selected == Some(expected);
        if selected.is_some() && matches != inverted {
            Ok(ScriptControl::Continue)
        } else {
            self.fail_guard()
        }
    }

    /// Jump directly and clear the pending resume/alternate-concept state.
    pub fn jump(&mut self, target: ScriptCodeOffset) -> ScriptControl {
        self.alternate_concept = None;
        self.resume = None;
        ScriptControl::Jump(target)
    }

    /// Continue only while one transient timer/state slot is zero.
    pub fn timer_guard(
        &mut self,
        slot: ScriptTimerSlot,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        if self.timer(slot) == u16::MIN {
            Ok(ScriptControl::Continue)
        } else {
            self.fail_guard()
        }
    }

    /// Apply the A5 query or assignment form selected by its optional value.
    pub fn timer_state(
        &mut self,
        slot: ScriptTimerSlot,
        value: Option<u16>,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        if let Some(value) = value {
            self.assign_timer(slot, value);
            Ok(ScriptControl::Continue)
        } else {
            self.timer_guard(slot)
        }
    }

    /// Assign one transient timer/state slot.
    pub fn assign_timer(&mut self, slot: ScriptTimerSlot, value: u16) {
        self.timer_words[slot.index()] = value;
    }

    fn apply_random_result(&mut self, result: u16) -> Result<ScriptControl, ScriptRuntimeError> {
        if result == u16::MIN {
            Ok(ScriptControl::Continue)
        } else {
            self.fail_guard()
        }
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;

    const YIELD_ORACLE_VECTOR_COUNT: usize = 6;

    fn dictionary_words() -> (ScriptWordId, ScriptWordId) {
        let dictionary = decode_script_dictionary(b"alpha\0beta\0").unwrap();
        (
            dictionary.resolve_source_offset(0).unwrap(),
            dictionary.resolve_source_offset(6).unwrap(),
        )
    }

    #[test]
    fn timer_save_block_round_trips_typed_words_and_reserved_bytes() {
        const FIRST_TIMER_VALUE: u16 = 0x1234;
        const LAST_TIMER_VALUE: u16 = 0xFEDC;

        let first = ScriptTimerSlot::decode(u8::MIN).unwrap();
        let last = ScriptTimerSlot::decode((ScriptTimerSlot::COUNT - 1) as u8).unwrap();
        let mut source = [u8::MIN; SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT];
        source[..SCRIPT_TIMER_WORD_BYTE_COUNT].copy_from_slice(&FIRST_TIMER_VALUE.to_le_bytes());
        let last_word_start = SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT - SCRIPT_TIMER_WORD_BYTE_COUNT;
        source[last_word_start..SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT]
            .copy_from_slice(&LAST_TIMER_VALUE.to_le_bytes());
        for (index, byte) in source[SCRIPT_TIMER_WORD_BLOCK_BYTE_COUNT..]
            .iter_mut()
            .enumerate()
        {
            *byte = index as u8;
        }

        let mut runtime = ScriptRuntime::new();
        runtime.restore_timer_save_block(&source);

        assert_eq!(runtime.timer(first), FIRST_TIMER_VALUE);
        assert_eq!(runtime.timer(last), LAST_TIMER_VALUE);
        assert_eq!(runtime.encode_timer_save_block(), source);
    }

    #[test]
    fn fresh_timer_save_block_matches_the_executable_static_region() {
        const BLOODPRG_INITIAL_TIMER_BLOCK_FILE_OFFSET: usize = 0x013EFE;

        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let expected = &executable[BLOODPRG_INITIAL_TIMER_BLOCK_FILE_OFFSET
            ..BLOODPRG_INITIAL_TIMER_BLOCK_FILE_OFFSET + SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT];

        assert_eq!(ScriptRuntime::new().encode_timer_save_block(), expected);
    }

    #[derive(Deserialize)]
    struct BranchOracle {
        target: u16,
    }

    #[derive(Deserialize)]
    struct GuardBeginOracle {
        target: u16,
    }

    #[derive(Deserialize)]
    struct GuardEndOracle {
        initial_top_byte_offset: u16,
        final_top_byte_offset: u16,
    }

    #[derive(Deserialize)]
    struct RandomGuardOracle {
        prng_result: u16,
        branch_taken: bool,
        final_script_offset: u16,
    }

    #[derive(Deserialize)]
    struct ConceptGuardOracle {
        scan_path: bool,
        resume_state: Option<u8>,
        inverted: Option<bool>,
        target: Option<u16>,
        selected_match: Option<u16>,
        branch_taken: bool,
        final_script_offset: u16,
    }

    #[derive(Deserialize)]
    struct JumpOracle {
        target_offset: u16,
    }

    #[derive(Deserialize)]
    struct TimerOracle {
        signed_index: i8,
        query_mode_before: u8,
        state_before: u16,
        operand_word: Option<u16>,
        branch_taken: bool,
        final_script_offset: u16,
        final_state: u16,
    }

    #[derive(Deserialize)]
    struct YieldOracle {
        yield_before: u8,
        yield_after: u8,
    }

    #[test]
    fn guard_stack_retains_its_root_until_a_condition_fails() {
        let root = ScriptCodeOffset::new(100);
        let nested = ScriptCodeOffset::new(200);
        let mut runtime = ScriptRuntime::new();

        runtime.begin_guard(root);
        runtime.begin_guard(nested);
        runtime.end_guard();
        assert_eq!(runtime.guard_depth(), 1);
        assert!(!runtime.query_mode());
        assert_eq!(runtime.fail_guard().unwrap(), ScriptControl::Jump(root));
        assert_eq!(runtime.guard_depth(), 0);
        assert_eq!(
            runtime.fail_guard().unwrap_err(),
            ScriptRuntimeError::MissingGuardTarget
        );
    }

    #[test]
    fn aa_yield_matches_every_original_vector() {
        let vectors: Vec<YieldOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6855_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), YIELD_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut runtime = ScriptRuntime::new();
            runtime.yield_requested = vector.yield_before != u8::MIN;
            runtime.request_yield();
            assert_eq!(runtime.yield_requested(), vector.yield_after != u8::MIN);
            assert!(runtime.take_yield_request());
            assert!(!runtime.yield_requested());
        }
    }

    #[test]
    fn ac_yield_matches_every_original_vector() {
        let vectors: Vec<YieldOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_685c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), YIELD_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut runtime = ScriptRuntime::new();
            runtime.yield_requested = vector.yield_before != u8::MIN;
            runtime.request_selector_yield();
            assert_eq!(runtime.yield_requested(), vector.yield_after != u8::MIN);
        }
    }

    #[test]
    fn concept_guard_matches_both_original_polarities() {
        let (alpha, beta) = dictionary_words();

        let mut ordinary = ScriptRuntime::new();
        ordinary.begin_guard(ScriptCodeOffset::new(10));
        ordinary.set_selected_concept(Some(alpha));
        assert_eq!(
            ordinary.concept_guard(alpha, false).unwrap(),
            ScriptControl::Continue
        );
        assert_eq!(
            ordinary.concept_guard(beta, false).unwrap(),
            ScriptControl::Jump(ScriptCodeOffset::new(10))
        );

        let mut inverted = ScriptRuntime::new();
        inverted.begin_guard(ScriptCodeOffset::new(20));
        inverted.set_selected_concept(Some(alpha));
        assert_eq!(
            inverted.concept_guard(beta, true).unwrap(),
            ScriptControl::Continue
        );
        assert_eq!(
            inverted.concept_guard(alpha, true).unwrap(),
            ScriptControl::Jump(ScriptCodeOffset::new(20))
        );
    }

    #[test]
    fn timer_and_random_guards_use_typed_owned_state() {
        let slot = ScriptTimerSlot::decode(22).unwrap();
        let target = ScriptCodeOffset::new(300);
        let mut runtime = ScriptRuntime::new();
        let mut random = BloodPrng::default();

        runtime.assign_timer(slot, 0);
        assert_eq!(runtime.timer_guard(slot).unwrap(), ScriptControl::Continue);
        runtime.assign_timer(slot, 5);
        runtime.begin_guard(target);
        assert_eq!(
            runtime.timer_guard(slot).unwrap(),
            ScriptControl::Jump(target)
        );

        runtime.begin_guard(target);
        assert_eq!(
            runtime.random_guard(1, &mut random).unwrap(),
            ScriptControl::Continue
        );
        assert_eq!(
            runtime.apply_random_result(1).unwrap(),
            ScriptControl::Jump(target)
        );
        assert_eq!(runtime.guard_depth(), 0);
    }

    #[test]
    fn direct_jump_clears_resume_and_alternate_concept_state() {
        let (alpha, beta) = dictionary_words();
        let target = ScriptCodeOffset::new(400);
        let mut runtime = ScriptRuntime::new();
        runtime.set_alternate_concept(Some(alpha));
        runtime.set_selected_concept(Some(beta));
        runtime.set_resume_target(Some(ScriptCodeOffset::new(500)));

        assert_eq!(runtime.jump(target), ScriptControl::Jump(target));
        assert_eq!(runtime.resume_target(), None);
        runtime.begin_guard(ScriptCodeOffset::new(600));
        assert_eq!(
            runtime.concept_guard(beta, false).unwrap(),
            ScriptControl::Continue
        );
    }

    #[test]
    fn resume_state_uses_semantic_phases_for_selector_dispatch() {
        const SAVED_CONCEPT_ENCODING: u16 = 10;

        let (alpha, _beta) = dictionary_words();
        let target = ScriptCodeOffset::new(700);
        let mut runtime = ScriptRuntime::new();

        assert!(!runtime.activate_selector_resume());
        runtime.arm_resume(target, u16::MIN);
        assert_eq!(
            runtime.resume_state().unwrap().phase,
            ScriptResumePhase::LoopArmed
        );
        assert!(!runtime.save_resume_concept(alpha, SAVED_CONCEPT_ENCODING));

        assert!(runtime.activate_selector_resume());
        assert!(runtime.save_resume_concept(alpha, SAVED_CONCEPT_ENCODING));
        assert_eq!(
            runtime.resume_state(),
            Some(ScriptResumeState {
                target,
                saved_cursor: None,
                value: SAVED_CONCEPT_ENCODING,
                phase: ScriptResumePhase::SelectorResumeActive,
            })
        );
        assert_eq!(runtime.alternate_concept(), Some(alpha));
    }

    #[test]
    fn branch_and_guard_stack_match_the_well_formed_original_vectors() {
        let branches: Vec<BranchOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6462_natural.json"
        ))
        .unwrap();
        for vector in branches {
            let target = ScriptCodeOffset::new(usize::from(vector.target));
            let mut runtime = ScriptRuntime::new();
            runtime.begin_guard(target);
            assert_eq!(runtime.fail_guard().unwrap(), ScriptControl::Jump(target));
            assert!(!runtime.query_mode());
        }

        let pushes: Vec<GuardBeginOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6559_natural.json"
        ))
        .unwrap();
        for vector in pushes {
            let target = ScriptCodeOffset::new(usize::from(vector.target));
            let mut runtime = ScriptRuntime::new();
            runtime.begin_guard(target);
            assert!(runtime.query_mode());
            assert_eq!(runtime.fail_guard().unwrap(), ScriptControl::Jump(target));
        }

        let pops: Vec<GuardEndOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6572_natural.json"
        ))
        .unwrap();
        let mut checked = 0;
        for vector in pops
            .into_iter()
            .filter(|vector| matches!(vector.initial_top_byte_offset, 2 | 4))
        {
            let mut runtime = ScriptRuntime::new();
            for target in 0..usize::from(vector.initial_top_byte_offset / 2) {
                runtime.begin_guard(ScriptCodeOffset::new(target));
            }
            runtime.end_guard();
            assert_eq!(
                runtime.guard_depth(),
                usize::from(vector.final_top_byte_offset / 2)
            );
            assert!(!runtime.query_mode());
            checked += 1;
        }
        assert_eq!(checked, 2);
    }

    #[test]
    fn random_and_concept_decisions_match_original_vectors() {
        let random_vectors: Vec<RandomGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6588_natural.json"
        ))
        .unwrap();
        for vector in random_vectors {
            let target = ScriptCodeOffset::new(usize::from(vector.final_script_offset));
            let mut runtime = ScriptRuntime::new();
            runtime.begin_guard(target);
            let result = runtime.apply_random_result(vector.prng_result).unwrap();
            assert_eq!(
                matches!(result, ScriptControl::Jump(_)),
                vector.branch_taken
            );
            if vector.branch_taken {
                assert_eq!(result, ScriptControl::Jump(target));
            }
        }

        let dictionary_data = vec![u8::MIN; usize::from(u16::MAX) + 1];
        let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
        let concept_vectors: Vec<ConceptGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6596_natural.json"
        ))
        .unwrap();
        for vector in concept_vectors
            .into_iter()
            .filter(|vector| !vector.scan_path)
        {
            let expected = dictionary
                .resolve_source_offset(vector.target.unwrap())
                .unwrap();
            let selected = vector
                .selected_match
                .filter(|value| *value != u16::MIN)
                .map(|value| dictionary.resolve_source_offset(value).unwrap());
            let failure_target = ScriptCodeOffset::new(usize::from(vector.final_script_offset));
            let mut runtime = ScriptRuntime::new();
            runtime.begin_guard(failure_target);
            if vector.resume_state.unwrap() & 2 != 0 {
                runtime.set_alternate_concept(selected);
            } else {
                runtime.set_selected_concept(selected);
            }
            let result = runtime
                .concept_guard(expected, vector.inverted.unwrap())
                .unwrap();
            assert_eq!(
                matches!(result, ScriptControl::Jump(_)),
                vector.branch_taken
            );
            if vector.branch_taken {
                assert_eq!(result, ScriptControl::Jump(failure_target));
            }
        }
    }

    #[test]
    fn jumps_and_timer_slots_match_original_vectors_in_the_flat_domain() {
        let jumps: Vec<JumpOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_65db_natural.json"
        ))
        .unwrap();
        for vector in jumps {
            let target = ScriptCodeOffset::new(usize::from(vector.target_offset));
            let mut runtime = ScriptRuntime::new();
            runtime.set_resume_target(Some(ScriptCodeOffset::new(1)));
            assert_eq!(runtime.jump(target), ScriptControl::Jump(target));
            assert_eq!(runtime.resume_target(), None);
        }

        let timers: Vec<TimerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_65eb_natural.json"
        ))
        .unwrap();
        let mut checked = 0;
        for vector in timers.into_iter().filter(|vector| vector.signed_index >= 0) {
            let slot = ScriptTimerSlot::decode(vector.signed_index as u8).unwrap();
            let mut runtime = ScriptRuntime::new();
            runtime.assign_timer(slot, vector.state_before);
            if vector.query_mode_before & 1 != 0 {
                let target = ScriptCodeOffset::new(usize::from(vector.final_script_offset));
                runtime.begin_guard(target);
                let result = runtime.timer_guard(slot).unwrap();
                assert_eq!(
                    matches!(result, ScriptControl::Jump(_)),
                    vector.branch_taken
                );
            } else {
                runtime.assign_timer(slot, vector.operand_word.unwrap());
            }
            assert_eq!(runtime.timer(slot), vector.final_state);
            checked += 1;
        }
        assert_eq!(checked, 4);
    }
}
