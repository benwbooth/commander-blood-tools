//! Flat runtime state for BloodScript control-flow instructions.

use std::fmt;

use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{ScriptInstruction, ScriptTimerSlot};
use commander_blood_formats::script::ScriptWordId;

use crate::native::random::BloodPrng;

/// Program-counter action produced by one translated BloodScript handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptControl {
    /// Continue with the next framed instruction.
    Continue,
    /// Continue at a typed position in the current COD image.
    Jump(ScriptCodeOffset),
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
    resume_target: Option<ScriptCodeOffset>,
    timer_words: [u16; ScriptTimerSlot::COUNT],
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
            resume_target: None,
            timer_words: [u16::MAX; ScriptTimerSlot::COUNT],
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
        self.resume_target
    }

    /// Set the primary concept chosen by the player.
    pub fn set_selected_concept(&mut self, concept: Option<ScriptWordId>) {
        self.selected_concept = concept;
    }

    /// Set the alternate concept used by resumed menu handling.
    pub fn set_alternate_concept(&mut self, concept: Option<ScriptWordId>) {
        self.alternate_concept = concept;
    }

    /// Arm a destination used by presentation resume logic.
    pub fn set_resume_target(&mut self, target: Option<ScriptCodeOffset>) {
        self.resume_target = target;
    }

    /// Read one proven transient timer/state slot.
    pub fn timer(&self, slot: ScriptTimerSlot) -> u16 {
        self.timer_words[slot.index()]
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
        }
    }

    /// Enter query mode and push one guard-failure destination.
    pub fn begin_guard(&mut self, failure_target: ScriptCodeOffset) {
        self.query_mode = true;
        self.guard_targets.push(failure_target);
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
        self.resume_target = None;
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

    fn dictionary_words() -> (ScriptWordId, ScriptWordId) {
        let dictionary = decode_script_dictionary(b"alpha\0beta\0").unwrap();
        (
            dictionary.resolve_source_offset(0).unwrap(),
            dictionary.resolve_source_offset(6).unwrap(),
        )
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
