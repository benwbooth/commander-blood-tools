//! Typed presentation-procedure preparation for deterministic production scenarios.

use anyhow::{Context, Result, bail, ensure};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptEnvironmentInstruction, ScriptInstruction, ScriptRecordValue,
    ScriptStateOperand, ScriptStateOperator,
};
use commander_blood_formats::script::{ScriptObjectId, ScriptProcedureId, ScriptStateWord};
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

pub(super) fn validate_contact_chapter(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
) -> Result<()> {
    if profile.code().dialect() == commander_blood_formats::code::ScriptDialect::BigBugBang {
        return authored_actor_scenario(
            profile,
            procedure_offset,
            target,
            Some(ScriptEnvironmentInstruction::RequireContactActivity),
        )
        .map(|_| ());
    }
    ensure!(
        profile.code().dialect() == commander_blood_formats::code::ScriptDialect::CommanderBlood,
        "the contact preparation manifest covers Commander Blood only"
    );
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON)?;
    let scenario = find_scenario(&manifest, profile.id(), procedure_offset)?;
    ensure!(
        profile.directory().find_active_object(target.as_bytes())
            == Some(object_at_source_offset(
                profile,
                scenario.contact_object_offset
            )?),
        "chapter target differs from the contact procedure's authored actor"
    );
    Ok(())
}

/// Recover only the outer, authored actor guard. Body conditions are not setup.
fn authored_actor_scenario(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    activity: Option<ScriptEnvironmentInstruction>,
) -> Result<ContactScenario> {
    let actor = profile
        .directory()
        .find_active_object(target.as_bytes())
        .context("chapter actor is not an authored object")?;
    let actor_record = profile
        .state()
        .object(actor)
        .context("chapter actor has no state")?;
    let action_offset = crate::native::bloodprg::script_field_offset(
        actor_record.kind,
        crate::native::bloodprg::ScriptFieldSelector::ACTION,
    )
    .context("chapter actor has no action field")?;
    let action = profile
        .state()
        .object_word_triple(actor, action_offset / 2)
        .context("chapter actor has no action slot")?;
    let Some(DecodedScriptInstruction::ProcedureGate(gate)) =
        profile.instruction_at(ScriptCodeOffset::new(procedure_offset))
    else {
        bail!("chapter procedure is not an authored procedure gate");
    };
    let mut activity_guard = activity.is_none();
    let mut presentation = false;
    let mut closed = false;
    let mut entry_tokens = Vec::new();
    for token in profile.code().tokens().iter().filter(|token| {
        token.source_offset().index() > procedure_offset
            && token.source_offset() < gate.failure_target
    }) {
        let offset = token.source_offset().index();
        match profile.instruction_at(token.source_offset()).unwrap() {
            DecodedScriptInstruction::Control(ScriptInstruction::GuardEnd) => {
                closed = true;
                break;
            }
            DecodedScriptInstruction::Environment(guard) if Some(*guard) == activity => {
                ensure!(!activity_guard, "duplicate chapter activity guard");
                activity_guard = true;
            }
            DecodedScriptInstruction::ActorRecord(operation) => {
                ensure!(
                    !presentation
                        && !operation.inverted
                        && operation.target == action
                        && Some(operation.related) == profile.builtins().player,
                    "procedure does not select the chapter actor"
                );
                presentation = true;
            }
            DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard { .. })
            | DecodedScriptInstruction::DirectRecord(_)
            | DecodedScriptInstruction::SharedBit(_) => {
                entry_tokens.push(ContactEntryToken { offset })
            }
            DecodedScriptInstruction::SharedState(operation) => {
                ensure!(
                    operation.operator == ScriptStateOperator::EqualOrAssign,
                    "chapter setup currently requires equality state predicates"
                );
                entry_tokens.push(ContactEntryToken { offset });
            }
            other => bail!("unsupported chapter entry predicate at {offset:#x}: {other:?}"),
        }
    }
    ensure!(
        closed && activity_guard && presentation,
        "procedure is not a complete {activity:?} actor guard"
    );
    let name = profile
        .directory()
        .procedure(gate.procedure)
        .context("chapter procedure has no name")?;
    Ok(ContactScenario {
        script: format!("SCRIPT{}", profile.id().value() + 1),
        procedure: String::from_utf8_lossy(name.name()).into_owned(),
        procedure_offset,
        contact_object_offset: actor_record.source_offset(),
        entry_tokens,
        presentations: Vec::new(),
    })
}

pub(super) fn validate_travel_chapter(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
) -> Result<()> {
    authored_actor_scenario(
        profile,
        procedure_offset,
        target,
        Some(ScriptEnvironmentInstruction::RequireTravelActivity),
    )
    .map(|_| ())
}

pub(super) fn prepare_travel_for_chapter(
    profile: &mut LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
) -> Result<()> {
    prepare_authored_actor_chapter(
        profile,
        procedure_offset,
        target,
        ScriptEnvironmentInstruction::RequireTravelActivity,
    )
}

pub(super) fn travel_supporting_procedures(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    supporting: &[usize],
) -> Result<Vec<ScriptProcedureId>> {
    let mut seen = std::collections::BTreeSet::from([procedure_offset]);
    supporting
        .iter()
        .map(|&offset| {
            ensure!(seen.insert(offset), "duplicate travel procedure");
            let scenario = authored_actor_scenario(
                profile,
                offset,
                target,
                Some(ScriptEnvironmentInstruction::RequireTravelActivity),
            )?;
            ensure!(
                scenario.entry_tokens.is_empty(),
                "supporting travel procedures must not add unprepared entry predicates"
            );
            match profile
                .instruction_at(ScriptCodeOffset::new(offset))
                .unwrap()
            {
                DecodedScriptInstruction::ProcedureGate(gate) => Ok(gate.procedure),
                _ => unreachable!("authored_actor_scenario validated the gate"),
            }
        })
        .collect()
}

pub(super) fn travel_actor_destination(
    profile: &LoadedScriptProfile,
    target: &str,
    planet: &str,
    destination: &str,
) -> Result<(ScriptStateWord, ScriptObjectId)> {
    use commander_blood_formats::script::{ScriptObjectKind, ScriptStateObjectReference};
    let object = |name: &str| -> Result<ScriptObjectId> {
        profile
            .directory()
            .find_active_object(name.as_bytes())
            .with_context(|| format!("unknown chapter object {name}"))
    };
    let actor = object(target)?;
    let planet = object(planet)?;
    let destination = object(destination)?;
    ensure!(
        profile.state().object(actor).unwrap().kind == ScriptObjectKind::Actor,
        "staged travel target must be an actor"
    );
    ensure!(
        profile.state().object(planet).unwrap().kind == ScriptObjectKind::CelestialBody,
        "staged travel planet must be a planet"
    );
    ensure!(
        profile.state().object(destination).unwrap().kind == ScriptObjectKind::Location,
        "staged travel destination must be a location"
    );
    let relation = |id| -> Result<ScriptStateWord> {
        let kind = profile.state().object(id).unwrap().kind;
        let offset = crate::native::bloodprg::script_field_offset(
            kind,
            crate::native::bloodprg::ScriptFieldSelector::HOLDER_OR_LOCATION,
        )
        .context("chapter object has no location field")?;
        profile
            .state()
            .object_word(id, offset / 2)
            .context("chapter location field is unbound")
    };
    ensure!(
        profile.state().object_reference(relation(destination)?)
            == Some(ScriptStateObjectReference::Object(planet)),
        "staged travel destination does not belong to the selected planet"
    );
    Ok((relation(actor)?, destination))
}

pub(super) fn stage_travel_actor(
    profile: &mut LoadedScriptProfile,
    target: &str,
    planet: &str,
    destination: &str,
) -> Result<()> {
    let (location, destination) = travel_actor_destination(profile, target, planet, destination)?;
    profile
        .execution_parts()
        .record_state
        .record_fields
        .set_value(location, ScriptRecordValue::Object(destination));
    let actor = profile
        .directory()
        .find_active_object(target.as_bytes())
        .unwrap();
    ensure!(
        set_object_flag(
            profile.state_mut(),
            actor,
            ScriptObjectFlag::LocationPanelDetails,
            true
        ),
        "cannot make the staged actor visible to native BBB navigation"
    );
    let state = profile.synchronized_state()?;
    profile.replace_state(state)?;
    Ok(())
}

pub(super) fn validate_aboard_inventory(
    profile: &LoadedScriptProfile,
    offsets: &[u16],
) -> Result<Vec<ScriptStateWord>> {
    let mut seen = std::collections::BTreeSet::new();
    offsets
        .iter()
        .map(|offset| {
            ensure!(seen.insert(offset), "duplicate staged inventory item");
            let object = profile
                .state()
                .objects()
                .iter()
                .find(|object| object.source_offset() == usize::from(*offset))
                .with_context(|| format!("unknown chapter inventory item {offset:#x}"))?
                .id;
            ensure!(
                profile.state().object(object).unwrap().kind
                    == commander_blood_formats::script::ScriptObjectKind::InventoryItem,
                "chapter inventory object is not an inventory item"
            );
            let offset = crate::native::bloodprg::script_field_offset(
                commander_blood_formats::script::ScriptObjectKind::InventoryItem,
                crate::native::bloodprg::ScriptFieldSelector::HOLDER_OR_LOCATION,
            )
            .unwrap();
            profile
                .state()
                .object_word(object, offset / 2)
                .context("chapter inventory holder is unbound")
        })
        .collect()
}

pub(super) fn stage_aboard_inventory(
    profile: &mut LoadedScriptProfile,
    offsets: &[u16],
) -> Result<()> {
    let fields = validate_aboard_inventory(profile, offsets)?;
    for holder in fields {
        profile
            .execution_parts()
            .record_state
            .record_fields
            .set_value(holder, ScriptRecordValue::Aboard);
    }
    let state = profile.synchronized_state()?;
    // Recover the native aboard roster from its serialized sentinel relationships.
    profile.replace_state(state)?;
    Ok(())
}

/// Derive a starting value from a complete authored evolution guard, never a body assignment.
pub(super) fn travel_actor_evolution_guard(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    supporting: &[usize],
    guard_offset: usize,
) -> Result<(ScriptStateWord, u16)> {
    use crate::native::bloodprg::{ScriptControl, ScriptRuntime, apply_shared_state_operation};

    validate_travel_chapter(profile, procedure_offset, target)?;
    travel_supporting_procedures(profile, procedure_offset, target, supporting)?;
    let gate = std::iter::once(procedure_offset)
        .chain(supporting.iter().copied())
        .find_map(
            |offset| match profile.instruction_at(ScriptCodeOffset::new(offset)) {
                Some(DecodedScriptInstruction::ProcedureGate(gate))
                    if offset < guard_offset && guard_offset < gate.failure_target.index() =>
                {
                    Some(gate)
                }
                _ => None,
            },
        )
        .context("evolution guard is outside the enabled travel procedures")?;
    let Some(DecodedScriptInstruction::Control(ScriptInstruction::GuardBegin { failure_target })) =
        profile.instruction_at(ScriptCodeOffset::new(guard_offset))
    else {
        bail!("evolution setup must reference an authored guard start");
    };
    ensure!(
        failure_target.index() > guard_offset && *failure_target <= gate.failure_target,
        "evolution guard escapes its procedure"
    );
    let actor = profile
        .directory()
        .find_active_object(target.as_bytes())
        .unwrap();
    let kind = profile.state().object(actor).unwrap().kind;
    ensure!(
        kind == commander_blood_formats::script::ScriptObjectKind::Actor,
        "evolution setup requires an actor"
    );
    let field_offset = crate::native::bloodprg::script_field_offset(
        kind,
        crate::native::bloodprg::ScriptFieldSelector::EVOLUTION,
    )
    .context("travel actor has no evolution field")?;
    let field = profile
        .state()
        .object_word(actor, field_offset / 2)
        .context("actor evolution field is unbound")?;
    let mut predicates = Vec::new();
    let mut closed = false;
    for token in profile.code().tokens().iter().filter(|token| {
        guard_offset < token.source_offset().index() && token.source_offset() < *failure_target
    }) {
        match profile.instruction_at(token.source_offset()) {
            Some(DecodedScriptInstruction::Control(ScriptInstruction::GuardEnd)) => {
                closed = true;
                break;
            }
            Some(DecodedScriptInstruction::SharedState(operation)) => {
                ensure!(
                    operation.target == field
                        && matches!(operation.operand, ScriptStateOperand::Immediate(_))
                        && matches!(
                            operation.operator,
                            ScriptStateOperator::NotEqual
                                | ScriptStateOperator::LessThan
                                | ScriptStateOperator::GreaterThan
                                | ScriptStateOperator::LessThanOrEqual
                                | ScriptStateOperator::GreaterThanOrEqual
                                | ScriptStateOperator::EqualOrAssign
                        ),
                    "evolution guard must compare only this actor's evolution with immediate values"
                );
                predicates.push(*operation);
            }
            _ => bail!("evolution guard contains an unsupported predicate"),
        }
    }
    ensure!(
        closed && !predicates.is_empty(),
        "evolution guard is empty or incomplete"
    );
    let mut state = profile.state().clone();
    let current = state.word(field).context("actor evolution is missing")?;
    // Use the production signed query semantics, preserving a valid initial value.
    for candidate in std::iter::once(current).chain(u16::MIN..=u16::MAX) {
        ensure!(
            state.set_word(field, candidate),
            "actor evolution field disappeared"
        );
        let mut runtime = ScriptRuntime::default();
        runtime.begin_guard(*failure_target);
        let mut passes = true;
        for predicate in &predicates {
            if apply_shared_state_operation(*predicate, &mut state, &mut runtime)?
                != ScriptControl::Continue
            {
                passes = false;
                break;
            }
        }
        if passes {
            return Ok((field, candidate));
        }
    }
    bail!("evolution guard has no satisfying 16-bit value")
}

pub(super) fn prepare_contact_for_chapter(
    runtime: &mut OriginalGameRuntime,
    procedure_offset: usize,
    target: &str,
) -> Result<()> {
    if runtime.data().game() == crate::game::GameVariant::CommanderBlood {
        return prepare_contact_for_scenario(runtime, procedure_offset);
    }
    prepare_authored_actor_chapter(
        runtime
            .current_profile_mut()
            .context("contact chapter has no profile")?,
        procedure_offset,
        target,
        ScriptEnvironmentInstruction::RequireContactActivity,
    )
}

/// Select a visit through an authored, single-predicate encounter guard.
pub(super) fn contact_encounter_guard(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    guard_offset: usize,
) -> Result<(ScriptStateWord, u16)> {
    validate_contact_chapter(profile, procedure_offset, target)?;
    actor_encounter_guard(profile, procedure_offset, target, guard_offset)
}

/// Select an authored visit in an enabled actor-only story procedure during travel.
pub(super) fn travel_actor_encounter_guard(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    guard_offset: usize,
) -> Result<(ScriptStateWord, u16)> {
    validate_travel_chapter(profile, procedure_offset, target)?;
    let (story_offset, procedure) = profile
        .code()
        .tokens()
        .iter()
        .find_map(
            |token| match profile.instruction_at(token.source_offset()) {
                Some(DecodedScriptInstruction::ProcedureGate(gate))
                    if token.source_offset().index() < guard_offset
                        && guard_offset < gate.failure_target.index() =>
                {
                    Some((token.source_offset().index(), gate.procedure))
                }
                _ => None,
            },
        )
        .context("travel encounter guard has no owning story procedure")?;
    let story = authored_actor_scenario(profile, story_offset, target, None)?;
    ensure!(
        story.entry_tokens.is_empty() && profile.procedures().is_enabled(procedure)?,
        "travel encounter guard requires an enabled actor-only story procedure"
    );
    actor_encounter_guard(profile, story_offset, target, guard_offset)
}

fn actor_encounter_guard(
    profile: &LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    guard_offset: usize,
) -> Result<(ScriptStateWord, u16)> {
    let Some(DecodedScriptInstruction::ProcedureGate(gate)) =
        profile.instruction_at(ScriptCodeOffset::new(procedure_offset))
    else {
        bail!("actor encounter setup has no procedure gate");
    };
    let tokens = profile.code().tokens();
    let index = tokens
        .iter()
        .position(|token| token.source_offset().index() == guard_offset)
        .context("actor encounter setup has no source instruction")?;
    ensure!(
        index > 0
            && index + 1 < tokens.len()
            && tokens[index - 1].source_offset().index() > procedure_offset
            && tokens[index + 1].source_offset() < gate.failure_target,
        "actor encounter guard is outside the selected procedure"
    );
    ensure!(
        matches!(
            profile.instruction_at(tokens[index - 1].source_offset()),
            Some(DecodedScriptInstruction::Control(
                ScriptInstruction::GuardBegin { .. }
            ))
        ) && matches!(
            profile.instruction_at(tokens[index + 1].source_offset()),
            Some(DecodedScriptInstruction::Control(
                ScriptInstruction::GuardEnd
            ))
        ),
        "actor encounter setup requires a single authored guard predicate"
    );
    let Some(DecodedScriptInstruction::SharedState(operation)) =
        profile.instruction_at(ScriptCodeOffset::new(guard_offset))
    else {
        bail!("actor encounter guard is not a state predicate");
    };
    let ScriptStateOperand::Immediate(value) = operation.operand else {
        bail!("actor encounter guard must use an immediate count");
    };
    ensure!(
        operation.operator == ScriptStateOperator::EqualOrAssign && value > 0,
        "actor encounter guard must equal a positive visit count"
    );
    let actor = profile
        .directory()
        .find_active_object(target.as_bytes())
        .unwrap();
    let kind = profile.state().object(actor).unwrap().kind;
    let offset = crate::native::bloodprg::script_field_offset(
        kind,
        crate::native::bloodprg::ScriptFieldSelector::ENCOUNTER_COUNT,
    )
    .context("chapter actor has no encounter counter")?;
    let counter = profile
        .state()
        .object_word(actor, offset / 2)
        .context("chapter actor encounter counter is unbound")?;
    ensure!(
        operation.target == counter,
        "actor encounter predicate does not address the chapter actor's counter"
    );
    Ok((counter, value))
}

fn prepare_authored_actor_chapter(
    profile: &mut LoadedScriptProfile,
    procedure_offset: usize,
    target: &str,
    activity: ScriptEnvironmentInstruction,
) -> Result<()> {
    let scenario = authored_actor_scenario(profile, procedure_offset, target, Some(activity))?;
    let selected = match profile
        .instruction_at(ScriptCodeOffset::new(procedure_offset))
        .unwrap()
    {
        DecodedScriptInstruction::ProcedureGate(gate) => gate.procedure,
        _ => unreachable!("authored_actor_scenario validated the gate"),
    };
    let mut actor_procedures = Vec::new();
    let mut gate = None;
    for instruction in profile.instructions() {
        match instruction {
            DecodedScriptInstruction::ProcedureGate(value) => gate = Some(value.procedure),
            DecodedScriptInstruction::Control(ScriptInstruction::GuardEnd) => gate = None,
            DecodedScriptInstruction::Environment(guard) if *guard == activity => {
                if let Some(procedure) = gate {
                    actor_procedures.push(procedure);
                }
            }
            _ => {}
        }
    }
    for procedure in actor_procedures {
        profile
            .procedures_mut()
            .set_enabled(procedure, procedure == selected)?;
    }
    activate_contact_objects(&scenario, profile)?;
    configure_entry_predicates(&scenario, profile)?;
    let mut synchronized = profile.synchronized_state()?;
    apply_entry_state_predicates(&scenario, profile, &mut synchronized)?;
    let actor = object_at_source_offset(profile, scenario.contact_object_offset)?;
    let kind = synchronized.object(actor).unwrap().kind;
    if activity == ScriptEnvironmentInstruction::RequireTravelActivity
        && let Some(offset) = crate::native::bloodprg::script_field_offset(
            kind,
            crate::native::bloodprg::ScriptFieldSelector::ENCOUNTER_COUNT,
        )
    {
        let counter = synchronized
            .object_word(actor, offset / 2)
            .context("travel actor has no encounter counter")?;
        if scenario.entry_tokens.iter().any(|entry| matches!(
            profile.instruction_at(ScriptCodeOffset::new(entry.offset)),
            Some(DecodedScriptInstruction::SharedState(operation)) if operation.target == counter
        )) {
            // C4 increments this word before the actor's COD guard is evaluated.
            let before_entry = synchronized.word(counter).unwrap().wrapping_sub(1);
            ensure!(synchronized.set_word(counter, before_entry), "travel encounter counter disappeared");
        }
    }
    profile.replace_state(synchronized)?;
    ensure!(
        profile.procedures().is_enabled(selected)?,
        "chapter setup disabled its selected procedure"
    );
    Ok(())
}

fn find_scenario(
    manifest: &ContactManifest,
    profile_id: crate::native::bloodprg::ScriptProfileId,
    procedure_offset: usize,
) -> Result<&ContactScenario> {
    let script = format!(
        "{SCRIPT_NAME_PREFIX}{}",
        profile_id.value() + FIRST_SCRIPT_NUMBER
    );
    let mut matches = manifest.procedures.iter().filter(|scenario| {
        scenario.script == script && scenario.procedure_offset == procedure_offset
    });
    let scenario = matches
        .next()
        .with_context(|| format!("no contact manifest row for {script}@{procedure_offset:04x}"))?;
    ensure!(matches.next().is_none(), "ambiguous contact manifest row");
    Ok(scenario)
}

/// Prepare one binary-derived D1 procedure immediately before its real UI click.
pub(super) fn prepare_contact_for_scenario(
    runtime: &mut OriginalGameRuntime,
    procedure_offset: usize,
) -> Result<()> {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON)
        .context("decoding the binary-derived contact manifest")?;
    ensure!(
        runtime.data().game() == crate::game::GameVariant::CommanderBlood,
        "the contact preparation manifest covers Commander Blood only"
    );
    let profile_id = runtime
        .current_profile()
        .context("contact preparation requires a loaded BloodScript profile")?
        .id();
    let scenario = find_scenario(&manifest, profile_id, procedure_offset)?;
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
