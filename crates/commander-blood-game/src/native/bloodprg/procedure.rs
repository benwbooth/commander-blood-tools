//! Flat mutable state for BloodScript procedure gates and activation writes.

use std::fmt;

use commander_blood_formats::instruction::{ScriptProcedureActivation, ScriptProcedureGate};
use commander_blood_formats::script::{ScriptDirectory, ScriptProcedureId};

use super::{ScriptControl, ScriptRuntime};

/// Byte count of one target/value record in an original save patch stream.
pub const SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT: usize = 3;
const LEGACY_PROCEDURE_PATCH_VALUE_OFFSET: usize = 2;
const ENABLED_FLAG_MASK: u8 = 1;

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
    /// A serialized save-game patch ends partway through its three-byte record.
    InvalidPatchStreamLength {
        /// Number of bytes in the malformed stream.
        byte_length: usize,
    },
    /// A serialized save-game patch names no procedure in the active profile.
    UnknownPatchTarget {
        /// One-based COD position stored by the original save format.
        encoded_target: u16,
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

/// Serialize the active profile's procedure gates in the original save-game format.
///
/// This translates `vm_patch_stream_build` at BLOODPRG file offset `0x001D94`.
/// The historical three-byte records contain a one-based COD position followed
/// by the mutable A9 byte. The modern runtime retains that position only as a
/// stable file-format identifier and stores the live value as typed Boolean
/// procedure state; it never patches executable bytes.
pub fn build_procedure_patch_stream(
    directory: &ScriptDirectory,
    procedures: &ScriptProcedureStates,
) -> Result<Vec<u8>, ScriptProcedureStateError> {
    let mut stream = Vec::with_capacity(
        directory.procedures().count() * SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT,
    );
    for (procedure, entry) in directory.procedures() {
        let enabled = procedures.is_enabled(procedure)?;
        stream.extend_from_slice(&entry.value.to_le_bytes());
        stream.push(u8::from(enabled));
    }
    Ok(stream)
}

/// Restore typed procedure gates from the original save-game patch format.
///
/// This translates `vm_patch_stream_apply` at BLOODPRG file offset `0x001D74`.
/// Every record is validated before state changes, so a truncated or foreign
/// save cannot partially corrupt the active profile. Ordered duplicate records
/// retain the native last-write-wins behavior.
pub fn apply_procedure_patch_stream(
    stream: &[u8],
    directory: &ScriptDirectory,
    procedures: &mut ScriptProcedureStates,
) -> Result<Option<ScriptProcedureId>, ScriptProcedureStateError> {
    if !stream
        .len()
        .is_multiple_of(SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT)
    {
        return Err(ScriptProcedureStateError::InvalidPatchStreamLength {
            byte_length: stream.len(),
        });
    }

    let mut updates = Vec::with_capacity(stream.len() / SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT);
    for record in stream.chunks_exact(SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT) {
        let encoded_target = u16::from_le_bytes(
            record[..LEGACY_PROCEDURE_PATCH_VALUE_OFFSET]
                .try_into()
                .expect("validated three-byte procedure patch record"),
        );
        let procedure = directory
            .resolve_procedure_activation_target(encoded_target)
            .ok_or(ScriptProcedureStateError::UnknownPatchTarget { encoded_target })?;
        procedures.is_enabled(procedure)?;
        updates.push((
            procedure,
            record[LEGACY_PROCEDURE_PATCH_VALUE_OFFSET] & ENABLED_FLAG_MASK != u8::MIN,
        ));
    }

    let mut final_procedure = None;
    for (procedure, enabled) in updates {
        procedures.set_enabled(procedure, enabled)?;
        final_procedure = Some(procedure);
    }
    Ok(final_procedure)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::decode_script_procedure_gate;
    use commander_blood_formats::script::decode_script_directory;
    use serde::Deserialize;

    use super::*;

    const PROCEDURE_GATE_OPCODE: u8 = 0xA9;
    const PROCEDURE_GATE_VECTOR_COUNT: usize = 9;
    const PROCEDURE_ACTIVATION_VECTOR_COUNT: usize = 10;
    const INITIAL_GUARD_TARGET: usize = 23_205;
    const PATCH_BUILD_VECTOR_COUNT: usize = 4;
    const PATCH_APPLY_VECTOR_COUNT: usize = 4;
    const SCRIPT_PROFILE_COUNT: usize = 5;
    const PROCEDURE_DIRECTORY_KIND: u16 = 2;

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

    #[derive(Deserialize)]
    struct PatchDirectoryEntryOracle {
        object_offset: u16,
        entry_kind: u16,
    }

    #[derive(Deserialize)]
    struct PatchRecordOracle {
        target_offset: u16,
        value: u8,
    }

    #[derive(Deserialize)]
    struct PatchBuildOracle {
        name: String,
        directory_entries: Vec<PatchDirectoryEntryOracle>,
        emitted_records: Vec<PatchRecordOracle>,
    }

    #[derive(Deserialize)]
    struct PatchApplyOracle {
        name: String,
        records: Vec<PatchRecordOracle>,
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

    fn synthetic_directory(entries: &[(u16, u16)]) -> ScriptDirectory {
        const DIRECTORY_NAME_BYTES: usize = 16;
        const DIRECTORY_ENTRY_BYTES: usize = 20;

        let mut encoded = Vec::with_capacity(entries.len() * DIRECTORY_ENTRY_BYTES);
        for (index, (value, kind)) in entries.iter().copied().enumerate() {
            let mut name = [u8::MIN; DIRECTORY_NAME_BYTES];
            name[0] = b'p';
            name[1..3].copy_from_slice(&(index as u16).to_le_bytes());
            encoded.extend_from_slice(&name);
            encoded.extend_from_slice(&value.to_le_bytes());
            encoded.extend_from_slice(&kind.to_le_bytes());
        }
        decode_script_directory(&encoded).unwrap()
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

    #[test]
    fn patch_stream_build_matches_every_original_directory_vector() {
        let vectors: Vec<PatchBuildOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1d94_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PATCH_BUILD_VECTOR_COUNT);

        for vector in vectors {
            let entries = vector
                .directory_entries
                .iter()
                .map(|entry| (entry.object_offset, entry.entry_kind))
                .collect::<Vec<_>>();
            let directory = synthetic_directory(&entries);
            let procedures = ScriptProcedureStates {
                enabled: vector
                    .emitted_records
                    .iter()
                    .map(|record| record.value & ENABLED_FLAG_MASK != u8::MIN)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            };
            let expected = vector
                .emitted_records
                .iter()
                .flat_map(|record| {
                    let [low, high] = record.target_offset.to_le_bytes();
                    [
                        low,
                        high,
                        u8::from(record.value & ENABLED_FLAG_MASK != u8::MIN),
                    ]
                })
                .collect::<Vec<_>>();

            assert_eq!(
                build_procedure_patch_stream(&directory, &procedures).unwrap(),
                expected,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn patch_stream_apply_matches_every_original_ordering_vector() {
        let vectors: Vec<PatchApplyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1d74_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PATCH_APPLY_VECTOR_COUNT);

        for vector in vectors {
            let unique_targets = vector
                .records
                .iter()
                .map(|record| record.target_offset)
                .collect::<BTreeSet<_>>();
            let entries = unique_targets
                .iter()
                .copied()
                .map(|target| (target, PROCEDURE_DIRECTORY_KIND))
                .collect::<Vec<_>>();
            let directory = synthetic_directory(&entries);
            let mut procedures = ScriptProcedureStates {
                enabled: vec![false; entries.len()].into_boxed_slice(),
            };
            let stream = vector
                .records
                .iter()
                .flat_map(|record| {
                    let [low, high] = record.target_offset.to_le_bytes();
                    [low, high, record.value]
                })
                .collect::<Vec<_>>();

            let final_procedure =
                apply_procedure_patch_stream(&stream, &directory, &mut procedures).unwrap();

            for (procedure, entry) in directory.procedures() {
                let expected = vector
                    .records
                    .iter()
                    .rev()
                    .find(|record| record.target_offset == entry.value)
                    .is_some_and(|record| record.value & ENABLED_FLAG_MASK != u8::MIN);
                assert_eq!(
                    procedures.is_enabled(procedure).unwrap(),
                    expected,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                final_procedure.map(ScriptProcedureId::index),
                vector.records.last().map(|record| {
                    unique_targets
                        .iter()
                        .position(|target| *target == record.target_offset)
                        .unwrap()
                }),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn every_shipped_profile_patch_stream_round_trips_typed_procedure_state() {
        for profile in 1..=SCRIPT_PROFILE_COUNT {
            let code = decode_script_code(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap(),
            )
            .unwrap();
            let directory = decode_script_directory(
                &std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap(),
            )
            .unwrap();
            let gates = code
                .tokens()
                .iter()
                .filter(|token| token.opcode().byte() == PROCEDURE_GATE_OPCODE)
                .map(|token| decode_script_procedure_gate(token, &directory).unwrap())
                .collect::<Vec<_>>();
            let expected = ScriptProcedureStates::from_gates(&gates).unwrap();
            let stream = build_procedure_patch_stream(&directory, &expected).unwrap();
            let original_code = code.encode();
            for (record, (_procedure, entry)) in stream
                .chunks_exact(SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT)
                .zip(directory.procedures())
            {
                assert_eq!(
                    record[LEGACY_PROCEDURE_PATCH_VALUE_OFFSET],
                    original_code[usize::from(entry.value)],
                    "SCRIPT{profile} procedure {}",
                    String::from_utf8_lossy(entry.name())
                );
            }
            let mut restored = ScriptProcedureStates {
                enabled: vec![false; expected.len()].into_boxed_slice(),
            };

            apply_procedure_patch_stream(&stream, &directory, &mut restored).unwrap();

            assert_eq!(restored, expected, "SCRIPT{profile}");
        }
    }

    #[test]
    fn malformed_patch_streams_are_rejected_before_state_changes() {
        let directory = synthetic_directory(&[(16, PROCEDURE_DIRECTORY_KIND)]);
        let initial = ScriptProcedureStates {
            enabled: vec![true].into_boxed_slice(),
        };

        for malformed in [&[16, 0][..], &[17, 0, 0][..]] {
            let mut procedures = initial.clone();
            assert!(apply_procedure_patch_stream(malformed, &directory, &mut procedures).is_err());
            assert_eq!(procedures, initial);
        }
    }
}
