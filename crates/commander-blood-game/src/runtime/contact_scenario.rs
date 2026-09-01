//! Typed contact-procedure preparation for deterministic production scenarios.

use anyhow::{Context, Result, bail};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstruction, ScriptRecordValue, ScriptStateOperand,
    ScriptStateOperator,
};
use commander_blood_formats::script::{ScriptObjectId, ScriptProcedureId};
use serde::Deserialize;

use crate::native::bloodprg::{LoadedScriptProfile, ScriptObjectFlag, set_object_flag};

use super::OriginalGameRuntime;

const CONTACT_MANIFEST_JSON: &str =
    include_str!("../../../../re/vm/contact-manifest/contact-manifest.json");
const SCRIPT_NAME_PREFIX: &str = "SCRIPT";
const FIRST_SCRIPT_NUMBER: u8 = 1;
const PROCEDURE_ENTRY_BIAS: usize = 1;

#[derive(Deserialize)]
struct ContactManifest {
    procedures: Vec<ContactScenario>,
}

#[derive(Deserialize)]
struct ContactScenario {
    script: String,
    procedure: String,
    procedure_offset: usize,
    contact_object_offset: usize,
    entry_tokens: Vec<ContactEntryToken>,
    presentations: Vec<ContactPresentation>,
}

#[derive(Deserialize)]
struct ContactEntryToken {
    offset: usize,
}

#[derive(Deserialize)]
struct ContactPresentation {
    object_offset: usize,
    related_record_offset: usize,
}

/// Prepare one binary-derived D1 procedure immediately before its real UI click.
pub(super) fn prepare_contact_for_scenario(
    runtime: &mut OriginalGameRuntime,
    procedure_offset: usize,
) -> Result<()> {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON)
        .context("decoding the binary-derived contact manifest")?;
    let profile_id = runtime
        .current_profile()
        .context("contact preparation requires a loaded BloodScript profile")?
        .id();
    let script = format!(
        "{SCRIPT_NAME_PREFIX}{}",
        profile_id.value() + FIRST_SCRIPT_NUMBER
    );
    let matches = manifest
        .procedures
        .iter()
        .filter(|scenario| {
            scenario.script == script && scenario.procedure_offset == procedure_offset
        })
        .collect::<Vec<_>>();
    let [scenario] = matches.as_slice() else {
        bail!(
            "contact procedure {script}@{procedure_offset:04x} resolved to {} manifest rows",
            matches.len()
        );
    };
    let profile = runtime
        .current_profile_mut()
        .context("loaded BloodScript profile disappeared during contact preparation")?;

    let selected = select_contact_procedure(&manifest, scenario, profile)?;
    activate_contact_objects(scenario, profile)?;
    configure_entry_predicates(scenario, profile)?;
    let synchronized = profile
        .synchronized_state()
        .context("synchronizing contact record state before predicate writes")?;
    let mut synchronized = synchronized;
    apply_entry_state_predicates(scenario, profile, &mut synchronized)?;
    profile
        .replace_state(synchronized)
        .context("rebuilding typed records after contact predicate writes")?;
    if !profile.procedures().is_enabled(selected)? {
        bail!("selected contact procedure became disabled during preparation");
    }
    Ok(())
}

fn select_contact_procedure(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    profile: &mut LoadedScriptProfile,
) -> Result<ScriptProcedureId> {
    let procedure_ids = profile
        .directory()
        .procedures()
        .map(|(procedure, entry)| (procedure, usize::from(entry.value)))
        .collect::<Vec<_>>();
    let contact_offsets = manifest
        .procedures
        .iter()
        .filter(|candidate| candidate.script == scenario.script)
        .map(|candidate| candidate.procedure_offset + PROCEDURE_ENTRY_BIAS)
        .collect::<Vec<_>>();
    let selected_entry = scenario.procedure_offset + PROCEDURE_ENTRY_BIAS;
    let selected = procedure_ids
        .iter()
        .find_map(|(procedure, entry)| (*entry == selected_entry).then_some(*procedure))
        .with_context(|| {
            format!(
                "{}:{} has no DEB procedure at COD entry {selected_entry}",
                scenario.script, scenario.procedure
            )
        })?;
    let selected_name = profile
        .directory()
        .procedure(selected)
        .context("selected contact procedure has no directory entry")?
        .name();
    if !selected_name.eq_ignore_ascii_case(scenario.procedure.as_bytes()) {
        bail!(
            "{}:{} resolved to DEB procedure {:?}",
            scenario.script,
            scenario.procedure,
            String::from_utf8_lossy(selected_name)
        );
    }
    for (procedure, entry) in procedure_ids {
        if contact_offsets.contains(&entry) {
            profile
                .procedures_mut()
                .set_enabled(procedure, procedure == selected)?;
        }
    }
    Ok(selected)
}

fn activate_contact_objects(
    scenario: &ContactScenario,
    profile: &mut LoadedScriptProfile,
) -> Result<()> {
    let active_offsets = std::iter::once(scenario.contact_object_offset).chain(
        scenario.presentations.iter().flat_map(|presentation| {
            [
                presentation.object_offset,
                presentation.related_record_offset,
            ]
        }),
    );
    let active_objects = active_offsets
        .map(|offset| object_at_source_offset(profile, offset))
        .collect::<Result<Vec<_>>>()?;
    for object in active_objects {
        if !set_object_flag(profile.state_mut(), object, ScriptObjectFlag::Active, true) {
            bail!("failed to activate contact object {object:?}");
        }
    }
    Ok(())
}

fn configure_entry_predicates(
    scenario: &ContactScenario,
    profile: &mut LoadedScriptProfile,
) -> Result<()> {
    for entry in &scenario.entry_tokens {
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(entry.offset))
            .with_context(|| format!("contact entry has no instruction at {:04x}", entry.offset))?
            .clone();
        match instruction {
            DecodedScriptInstruction::ActorRecord(_)
            | DecodedScriptInstruction::Control(ScriptInstruction::GuardBegin { .. }) => {}
            DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard { slot }) => {
                profile.runtime_mut().assign_timer(slot, u16::MIN);
            }
            DecodedScriptInstruction::SharedState(_) | DecodedScriptInstruction::SharedBit(_) => {}
            DecodedScriptInstruction::DirectRecord(operation) => {
                let current = profile
                    .record_state()
                    .record_fields
                    .value(operation.target)
                    .context("decoded direct-record contact target is unbound")?;
                let value = if operation.inverted && current == operation.value {
                    unequal_record_value(profile, operation.value)?
                } else if operation.inverted {
                    current
                } else {
                    operation.value
                };
                profile
                    .execution_parts()
                    .record_state
                    .record_fields
                    .set_value(operation.target, value);
            }
            instruction => bail!(
                "{}:{} has unsupported entry predicate at {:04x}: {instruction:?}",
                scenario.script,
                scenario.procedure,
                entry.offset
            ),
        }
    }
    Ok(())
}

fn apply_entry_state_predicates(
    scenario: &ContactScenario,
    profile: &LoadedScriptProfile,
    state: &mut commander_blood_formats::script::ScriptState,
) -> Result<()> {
    for entry in &scenario.entry_tokens {
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(entry.offset))
            .context("validated contact entry instruction disappeared")?;
        match instruction {
            DecodedScriptInstruction::SharedState(operation) => {
                if operation.operator != ScriptStateOperator::EqualOrAssign {
                    bail!("contact entry shared-state predicate is not equality");
                }
                let value = match operation.operand {
                    ScriptStateOperand::Immediate(value) => value,
                    ScriptStateOperand::StateWord(source) => state
                        .word(source)
                        .context("contact shared-state source is outside VAR")?,
                };
                if !state.set_word(operation.target, value) {
                    bail!("contact shared-state target is outside VAR");
                }
            }
            DecodedScriptInstruction::SharedBit(operation) => {
                let current = state
                    .word(operation.target)
                    .context("contact shared-bit target is outside VAR")?;
                let value = if operation.inverted_or_clear {
                    current & !operation.mask
                } else {
                    current | operation.mask
                };
                if !state.set_word(operation.target, value) {
                    bail!("contact shared-bit target is outside VAR");
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn unequal_record_value(
    profile: &LoadedScriptProfile,
    value: ScriptRecordValue,
) -> Result<ScriptRecordValue> {
    Ok(match value {
        ScriptRecordValue::Aboard => ScriptRecordValue::Object(
            profile
                .builtins()
                .archetype
                .context("profile has no Arche object for a non-aboard value")?,
        ),
        ScriptRecordValue::Object(object) => {
            let builtins = profile.builtins();
            ScriptRecordValue::Object(
                builtins
                    .archetype
                    .filter(|candidate| *candidate != object)
                    .or(builtins.player.filter(|candidate| *candidate != object))
                    .context("profile has no distinct relation object")?,
            )
        }
        ScriptRecordValue::Topic(word) => ScriptRecordValue::Topic(
            profile
                .dictionary()
                .words()
                .find_map(|(candidate, _)| (candidate != word).then_some(candidate))
                .context("profile dictionary has no alternate topic")?,
        ),
        ScriptRecordValue::NativeWord(_) => ScriptRecordValue::Aboard,
    })
}

fn object_at_source_offset(
    profile: &LoadedScriptProfile,
    source_offset: usize,
) -> Result<ScriptObjectId> {
    profile
        .state()
        .objects()
        .iter()
        .find_map(|object| (object.source_offset() == source_offset).then_some(object.id))
        .with_context(|| format!("profile has no object at VAR offset {source_offset:04x}"))
}
