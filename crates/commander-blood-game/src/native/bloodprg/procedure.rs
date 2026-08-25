//! Flat mutable state for BloodScript procedure gates and activation writes.

use std::fmt;

use commander_blood_formats::instruction::{ScriptProcedureActivation, ScriptProcedureGate};
use commander_blood_formats::script::ScriptProcedureId;

use super::{ScriptControl, ScriptRuntime};

/// Invalid procedure-state construction or access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptProcedureStateError {
    /// Two A9 entry gates resolve to the same procedure.
    DuplicateGate {
        /// Repeated procedure identity.
        procedure: ScriptProcedureId,
    },
    /// No A9 gate initializes one position in the procedure table.
    MissingGate {
        /// Missing zero-based procedure index.
        procedure_index: usize,
    },
    /// Runtime state does not contain a requested procedure.
    UnknownProcedure {
        /// Out-of-range procedure identity.
        procedure: ScriptProcedureId,
    },
}

impl fmt::Display for ScriptProcedureStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptProcedureStateError {}

/// Owned Boolean enabled state replacing mutable A9 bytes in the COD image.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptProcedureStates {
    enabled: Box<[bool]>,
}

impl ScriptProcedureStates {
    /// Initialize every procedure from its typed A9 entry gate.
    pub fn from_gates(gates: &[ScriptProcedureGate]) -> Result<Self, ScriptProcedureStateError> {
        let procedure_count = gates
            .iter()
            .map(|gate| gate.procedure.index())
            .max()
            .map_or(usize::MIN, |maximum| maximum + 1);
        let mut enabled = vec![false; procedure_count];
        let mut initialized = vec![false; procedure_count];

        for gate in gates {
            let index = gate.procedure.index();
            if initialized[index] {
                return Err(ScriptProcedureStateError::DuplicateGate {
                    procedure: gate.procedure,
                });
            }
            enabled[index] = gate.initially_enabled;
            initialized[index] = true;
        }
        if let Some(procedure_index) = initialized.iter().position(|initialized| !initialized) {
            return Err(ScriptProcedureStateError::MissingGate { procedure_index });
        }

        Ok(Self {
            enabled: enabled.into_boxed_slice(),
        })
    }

    /// Return the number of procedures in the profile.
    pub const fn len(&self) -> usize {
        self.enabled.len()
    }

    /// Return whether the profile declares no procedures.
    pub const fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }

    /// Read one procedure's current enabled state.
    pub fn is_enabled(
        &self,
        procedure: ScriptProcedureId,
    ) -> Result<bool, ScriptProcedureStateError> {
        self.enabled
            .get(procedure.index())
            .copied()
            .ok_or(ScriptProcedureStateError::UnknownProcedure { procedure })
    }

    /// Assign one procedure's current enabled state.
    pub fn set_enabled(
        &mut self,
        procedure: ScriptProcedureId,
        enabled: bool,
    ) -> Result<(), ScriptProcedureStateError> {
        let state = self
            .enabled
            .get_mut(procedure.index())
            .ok_or(ScriptProcedureStateError::UnknownProcedure { procedure })?;
        *state = enabled;
        Ok(())
    }
}

/// Apply `vm_op_a9_cond_jump` through typed procedure and control-flow state.
pub fn evaluate_procedure_gate(
    gate: ScriptProcedureGate,
    procedures: &ScriptProcedureStates,
    runtime: &mut ScriptRuntime,
) -> Result<ScriptControl, ScriptProcedureStateError> {
    if procedures.is_enabled(gate.procedure)? {
        runtime.begin_root_guard(gate.failure_target);
        Ok(ScriptControl::Continue)
    } else {
        Ok(ScriptControl::Jump(gate.failure_target))
    }
}

/// Apply `vm_op_ab_poke_byte` as a typed procedure-enabled assignment.
pub fn apply_procedure_activation(
    activation: ScriptProcedureActivation,
    procedures: &mut ScriptProcedureStates,
) -> Result<(), ScriptProcedureStateError> {
    procedures.set_enabled(activation.procedure, activation.enabled)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::decode_script_procedure_gate;
    use commander_blood_formats::script::decode_script_directory;
    use serde::Deserialize;

    use super::*;

    const PROCEDURE_GATE_OPCODE: u8 = 0xA9;
    const PROCEDURE_GATE_VECTOR_COUNT: usize = 9;
    const PROCEDURE_ACTIVATION_VECTOR_COUNT: usize = 10;
    const ENABLED_FLAG_MASK: u8 = 1;
    const INITIAL_GUARD_TARGET: usize = 23_205;

    #[derive(Deserialize)]
    struct ProcedureGateOracle {
        flags_byte: u8,
        odd_path: bool,
        target: u16,
        query_after: u8,
        root_after: u16,
    }

    #[derive(Deserialize)]
    struct ProcedureActivationOracle {
        value: u8,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn first_shipped_gate() -> ScriptProcedureGate {
        let code_data = std::fs::read(original_asset("SCRIPT1.COD")).unwrap();
        let directory_data = std::fs::read(original_asset("SCRIPT1.DEB")).unwrap();
        let code = decode_script_code(&code_data).unwrap();
        let directory = decode_script_directory(&directory_data).unwrap();
        let token = code
            .tokens()
            .iter()
            .find(|token| token.opcode().byte() == PROCEDURE_GATE_OPCODE)
            .unwrap();
        let gate = decode_script_procedure_gate(token, &directory).unwrap();
        assert_eq!(gate.procedure.index(), usize::MIN);
        gate
    }

    #[test]
    fn procedure_gates_match_every_original_a9_vector() {
        let vectors: Vec<ProcedureGateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_6830_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PROCEDURE_GATE_VECTOR_COUNT);

        for vector in vectors {
            let gate = ScriptProcedureGate {
                initially_enabled: vector.flags_byte & ENABLED_FLAG_MASK != u8::MIN,
                failure_target: commander_blood_formats::code::ScriptCodeOffset::new(usize::from(
                    vector.target,
                )),
                ..first_shipped_gate()
            };
            assert_eq!(gate.initially_enabled, vector.odd_path);
            let procedures = ScriptProcedureStates::from_gates(&[gate]).unwrap();
            let mut runtime = ScriptRuntime::new();
            runtime.begin_root_guard(commander_blood_formats::code::ScriptCodeOffset::new(
                INITIAL_GUARD_TARGET,
            ));

            let control = evaluate_procedure_gate(gate, &procedures, &mut runtime).unwrap();

            assert_eq!(runtime.query_mode(), vector.query_after != u8::MIN);
            assert_eq!(
                runtime.current_guard_target(),
                Some(commander_blood_formats::code::ScriptCodeOffset::new(
                    usize::from(vector.root_after)
                ))
            );
            assert_eq!(
                control,
                if vector.odd_path {
                    ScriptControl::Continue
                } else {
                    ScriptControl::Jump(gate.failure_target)
                }
            );
        }
    }

    #[test]
    fn procedure_activation_matches_every_original_ab_value_in_the_flat_domain() {
        let vectors: Vec<ProcedureActivationOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_684c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PROCEDURE_ACTIVATION_VECTOR_COUNT);
        let gate = first_shipped_gate();

        for vector in vectors {
            let expected = vector.value & ENABLED_FLAG_MASK != u8::MIN;
            let mut procedures = ScriptProcedureStates::from_gates(&[ScriptProcedureGate {
                initially_enabled: !expected,
                ..gate
            }])
            .unwrap();
            apply_procedure_activation(
                ScriptProcedureActivation {
                    procedure: gate.procedure,
                    enabled: expected,
                },
                &mut procedures,
            )
            .unwrap();
            assert_eq!(procedures.is_enabled(gate.procedure).unwrap(), expected);
        }
    }
}
