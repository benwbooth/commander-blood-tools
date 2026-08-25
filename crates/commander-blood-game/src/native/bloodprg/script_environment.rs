//! BloodScript guards driven by modern presentation activity state.

use commander_blood_formats::instruction::ScriptEnvironmentInstruction;

use super::{ScriptControl, ScriptRuntime, ScriptRuntimeError};

/// Presentation activities tested by the CE, D0, and D1 guard instructions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptEnvironmentActivity {
    /// The bridge renderer currently owns the interface.
    pub bridge_active: bool,
    /// A travel presentation is active.
    pub travel_active: bool,
    /// A contact presentation is active.
    pub contact_active: bool,
}

impl ScriptEnvironmentActivity {
    /// Apply `vm_op_ce_cond_branch` through the typed bridge activity flag.
    pub fn require_bridge(
        self,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        require_activity(self.bridge_active, runtime)
    }

    /// Apply `vm_op_d0_cond_branch` through the typed travel activity flag.
    pub fn require_travel(
        self,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        require_activity(self.travel_active, runtime)
    }

    /// Apply `vm_op_d1_cond_branch` through the typed contact activity flag.
    pub fn require_contact(
        self,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        require_activity(self.contact_active, runtime)
    }

    /// Dispatch one decoded CE through D1 instruction.
    pub fn apply(
        self,
        instruction: ScriptEnvironmentInstruction,
        runtime: &mut ScriptRuntime,
    ) -> Result<ScriptControl, ScriptRuntimeError> {
        match instruction {
            ScriptEnvironmentInstruction::RequireBridgeActivity => self.require_bridge(runtime),
            ScriptEnvironmentInstruction::ClearAlternateConcept => {
                runtime.clear_alternate_resume_state();
                Ok(ScriptControl::Continue)
            }
            ScriptEnvironmentInstruction::RequireTravelActivity => self.require_travel(runtime),
            ScriptEnvironmentInstruction::RequireContactActivity => self.require_contact(runtime),
        }
    }
}

fn require_activity(
    active: bool,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptRuntimeError> {
    if active {
        Ok(ScriptControl::Continue)
    } else {
        runtime.fail_guard()
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::{ScriptCodeOffset, decode_script_code};
    use commander_blood_formats::instruction::decode_script_environment_instruction;
    use commander_blood_formats::script::decode_script_dictionary;
    use serde::Deserialize;

    use super::*;

    const BRIDGE_ACTIVITY_GUARD_OPCODE: u8 = 0xCE;
    const ALTERNATE_CONCEPT_CLEAR_OPCODE: u8 = 0xCF;
    const TRAVEL_ACTIVITY_GUARD_OPCODE: u8 = 0xD0;
    const CONTACT_ACTIVITY_GUARD_OPCODE: u8 = 0xD1;
    const CODE_END_MARKER: u8 = 0xFF;

    #[derive(Deserialize)]
    struct ActivityGuardOracle {
        flag_value: u8,
        branch_taken: bool,
        final_script_offset: usize,
    }

    #[derive(Deserialize)]
    struct AlternateClearOracle {
        resume_state_before: u8,
        resume_value_before: u16,
        resume_state_after: u8,
        resume_value_after: u16,
    }

    fn instruction(opcode: u8) -> ScriptEnvironmentInstruction {
        let code = decode_script_code(&[opcode, CODE_END_MARKER]).unwrap();
        decode_script_environment_instruction(&code.tokens()[0]).unwrap()
    }

    fn verify_activity_guards(
        vectors: &[ActivityGuardOracle],
        opcode: u8,
        activity: impl Fn(bool) -> ScriptEnvironmentActivity,
    ) {
        for vector in vectors {
            let mut runtime = ScriptRuntime::new();
            let failure_target = ScriptCodeOffset::new(vector.final_script_offset);
            runtime.begin_root_guard(failure_target);

            let control = activity(vector.flag_value & 1 != u8::MIN)
                .apply(instruction(opcode), &mut runtime)
                .unwrap();

            assert_eq!(
                matches!(control, ScriptControl::Jump(_)),
                vector.branch_taken
            );
            if vector.branch_taken {
                assert_eq!(control, ScriptControl::Jump(failure_target));
                assert_eq!(runtime.guard_depth(), usize::MIN);
            } else {
                assert_eq!(control, ScriptControl::Continue);
                assert_eq!(runtime.guard_depth(), 1);
            }
        }
    }

    #[test]
    fn bridge_travel_and_contact_guards_match_every_original_vector() {
        let bridge: Vec<ActivityGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6494_natural.json"
        ))
        .unwrap();
        let travel: Vec<ActivityGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64a0_natural.json"
        ))
        .unwrap();
        let contact: Vec<ActivityGuardOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64ac_natural.json"
        ))
        .unwrap();

        verify_activity_guards(&bridge, BRIDGE_ACTIVITY_GUARD_OPCODE, |bridge_active| {
            ScriptEnvironmentActivity {
                bridge_active,
                ..ScriptEnvironmentActivity::default()
            }
        });
        verify_activity_guards(&travel, TRAVEL_ACTIVITY_GUARD_OPCODE, |travel_active| {
            ScriptEnvironmentActivity {
                travel_active,
                ..ScriptEnvironmentActivity::default()
            }
        });
        verify_activity_guards(&contact, CONTACT_ACTIVITY_GUARD_OPCODE, |contact_active| {
            ScriptEnvironmentActivity {
                contact_active,
                ..ScriptEnvironmentActivity::default()
            }
        });
    }

    #[test]
    fn alternate_concept_clear_matches_every_original_vector() {
        let vectors: Vec<AlternateClearOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64c0_natural.json"
        ))
        .unwrap();
        let dictionary = decode_script_dictionary(b"alternate\0").unwrap();
        let alternate = dictionary.resolve_source_offset(u16::MIN).unwrap();

        for vector in vectors {
            let mut runtime = ScriptRuntime::new();
            runtime.set_alternate_concept(Some(alternate));
            if vector.resume_state_before != u8::MIN {
                runtime.arm_resume(ScriptCodeOffset::new(1), vector.resume_value_before);
            }

            let control = ScriptEnvironmentActivity::default()
                .apply(instruction(ALTERNATE_CONCEPT_CLEAR_OPCODE), &mut runtime)
                .unwrap();

            assert_eq!(control, ScriptControl::Continue);
            assert_eq!(runtime.alternate_concept(), None);
            assert_eq!(runtime.resume_state(), None);
            assert_eq!(vector.resume_state_after, u8::MIN);
            assert_eq!(vector.resume_value_after, u16::MIN);
        }
    }
}
