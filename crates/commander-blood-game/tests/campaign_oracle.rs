use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstruction, ScriptRecordValue, ScriptStateOperand,
    ScriptStateOperator, ScriptTextWord,
};
use commander_blood_formats::script::ScriptObjectId;
use commander_blood_game::native::bloodprg::{
    ScriptActionRecord, ScriptClock, ScriptEnvironmentActivity, ScriptFieldSelector,
    ScriptObjectFlag, ScriptProfileId, ScriptRecordStateNavigationContext, SequenceRequestContext,
    object_has_flag, script_field_offset, set_object_flag,
};
use commander_blood_game::runtime::{
    OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime, RuntimeScriptSystem,
};
use serde::Deserialize;

const CONTACT_MANIFEST_JSON: &str =
    include_str!("../../../re/vm/contact-manifest/contact-manifest.json");
const EXPECTED_CONTACT_PROCEDURE_COUNT: usize = 65;
const FIRST_SCRIPT_NUMBER: u8 = 1;
const SCRIPT_NAME_PREFIX: &str = "SCRIPT";
const PROCEDURE_ENTRY_BIAS: usize = 1;
const MAXIMUM_ENTRY_FRAMES: usize = 32;
const ORACLE_CLOCK: ScriptClock = ScriptClock {
    hour: 12,
    day: 2,
    month: 1,
};

#[derive(Debug, Deserialize)]
struct ContactManifest {
    procedure_count: usize,
    procedures: Vec<ContactScenario>,
}

#[derive(Debug, Deserialize)]
struct ContactScenario {
    script: String,
    procedure: String,
    procedure_offset: usize,
    contact_object_offset: usize,
    entry_tokens: Vec<ContactEntryToken>,
    presentations: Vec<ContactPresentation>,
    texts: Vec<ContactText>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct ContactEntryToken {
    offset: usize,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct ContactPresentation {
    predicate_offset: usize,
    object_offset: usize,
    related_record_offset: usize,
}

#[derive(Debug, Deserialize)]
struct ContactText {
    voice_selector: u8,
    word_offsets: Vec<u16>,
    subtitle: String,
}

#[derive(Debug)]
struct ContactEntrySnapshot {
    selected_line: Option<i8>,
    word_offsets: Vec<u16>,
    subtitle: String,
}

#[test]
fn contact_manifest_declares_every_recovered_contact_entry() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();

    assert_eq!(manifest.procedure_count, EXPECTED_CONTACT_PROCEDURE_COUNT);
    assert_eq!(manifest.procedures.len(), EXPECTED_CONTACT_PROCEDURE_COUNT);
    assert!(
        manifest
            .procedures
            .iter()
            .all(|scenario| scenario.texts.iter().any(|text| !text.subtitle.is_empty()))
    );
}

#[test]
#[ignore = "requires original Commander Blood data"]
fn every_recovered_contact_enters_the_expected_rust_presentation() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let paths = OriginalGameDataPaths::discover(None).unwrap();

    for scenario in &manifest.procedures {
        let data =
            OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, ORACLE_CLOCK);
        let mut runtime = OriginalGameRuntime::new(data);
        let profile_id = profile_id(&scenario.script);
        scripts.load_profile(&mut runtime, profile_id).unwrap();
        scripts
            .execute_frame(&mut runtime, true)
            .unwrap_or_else(|error| {
                panic!(
                    "{}:{} profile initialization failed: {error:?}",
                    scenario.script, scenario.procedure
                )
            });
        configure_contact_entry(&manifest, scenario, &mut runtime);
        configure_script_context(&mut scripts, &runtime, scenario);

        let mut snapshot = None;
        let mut frame_outcomes = Vec::with_capacity(MAXIMUM_ENTRY_FRAMES);
        for _ in 0..MAXIMUM_ENTRY_FRAMES {
            let outcome = scripts
                .execute_frame(&mut runtime, true)
                .unwrap_or_else(|error| {
                    panic!(
                        "{}:{} at {} failed: {error:?}",
                        scenario.script, scenario.procedure, scenario.procedure_offset
                    )
                });
            frame_outcomes.push(outcome);
            let text = scripts.text_presentation();
            let profile = runtime.current_profile().unwrap();
            let word_offsets = text
                .menu_words
                .iter()
                .map(|word| match word {
                    ScriptTextWord::Dictionary(word) => {
                        profile.dictionary().source_offset(*word).unwrap()
                    }
                    ScriptTextWord::SectionSeparator => u16::MAX,
                })
                .collect::<Vec<_>>();
            let subtitle = normalize_text(&text.subtitle_text);
            let matches_contact = scenario.texts.iter().any(|expected| {
                if word_offsets.is_empty() {
                    normalize_text(expected.subtitle.as_bytes()) == subtitle
                } else {
                    expected.word_offsets == word_offsets
                }
            });
            if matches_contact {
                snapshot = Some(ContactEntrySnapshot {
                    selected_line: text.selected_line,
                    word_offsets,
                    subtitle,
                });
                break;
            }
            if outcome.presentation_yields != 0 {
                *scripts.text_presentation_mut() = Default::default();
                *scripts.presentation_scan_state_mut() = Default::default();
            }
            if frame_outcomes.len() == 1 && outcome.presentation_yields == 0 {
                assert_entry_predicates(runtime.current_profile().unwrap(), scenario);
            }
            assert!(
                outcome.next_instruction.is_some(),
                "{}:{} terminated before entering its contact",
                scenario.script,
                scenario.procedure
            );
        }

        let snapshot = snapshot.unwrap_or_else(|| {
            panic!(
                "{}:{} at {} did not enter a presentation within {} frames: {:?}",
                scenario.script,
                scenario.procedure,
                scenario.procedure_offset,
                MAXIMUM_ENTRY_FRAMES,
                frame_outcomes,
            )
        });
        let expected = scenario
            .texts
            .iter()
            .find(|text| {
                if snapshot.word_offsets.is_empty() {
                    normalize_text(text.subtitle.as_bytes()) == snapshot.subtitle
                } else {
                    text.word_offsets == snapshot.word_offsets
                }
            })
            .unwrap_or_else(|| {
                panic!(
                    "{}:{} at {} emitted unexpected text words {:?} / {:?}; frames: {:?}",
                    scenario.script,
                    scenario.procedure,
                    scenario.procedure_offset,
                    snapshot.word_offsets,
                    snapshot.subtitle,
                    frame_outcomes
                )
            });
        assert_eq!(
            snapshot.selected_line,
            Some(expected.voice_selector as i8),
            "{}:{} selected the wrong voice for {:?}",
            scenario.script,
            scenario.procedure,
            expected.subtitle
        );
    }
}

fn profile_id(script: &str) -> ScriptProfileId {
    let profile_number = script
        .strip_prefix(SCRIPT_NAME_PREFIX)
        .unwrap_or_else(|| panic!("invalid script profile name {script:?}"))
        .parse::<u8>()
        .unwrap_or_else(|error| panic!("invalid script profile number in {script:?}: {error}"));
    ScriptProfileId::new(profile_number - FIRST_SCRIPT_NUMBER)
        .unwrap_or_else(|| panic!("script profile {script:?} is outside the shipped profile set"))
}

fn configure_contact_entry(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    runtime: &mut OriginalGameRuntime,
) {
    let selected_profile = profile_id(&scenario.script);
    let profile = runtime.current_profile_mut().unwrap();
    let procedure_ids = profile
        .directory()
        .procedures()
        .map(|(procedure, entry)| (procedure, usize::from(entry.value)))
        .collect::<Vec<_>>();
    let contact_offsets = manifest
        .procedures
        .iter()
        .filter(|candidate| profile_id(&candidate.script) == selected_profile)
        .map(|candidate| candidate.procedure_offset + PROCEDURE_ENTRY_BIAS)
        .collect::<Vec<_>>();
    let selected_entry = scenario.procedure_offset + PROCEDURE_ENTRY_BIAS;
    let selected = procedure_ids
        .iter()
        .find_map(|(procedure, entry)| (*entry == selected_entry).then_some(*procedure))
        .unwrap_or_else(|| {
            panic!(
                "{}:{} has no DEB procedure at COD entry {}",
                scenario.script, scenario.procedure, selected_entry
            )
        });
    let selected_name = profile.directory().procedure(selected).unwrap().name();
    assert!(
        selected_name.eq_ignore_ascii_case(scenario.procedure.as_bytes()),
        "{}:{} resolved COD entry {} to DEB procedure {:?}",
        scenario.script,
        scenario.procedure,
        selected_entry,
        String::from_utf8_lossy(selected_name)
    );

    for (procedure, entry) in procedure_ids {
        if contact_offsets.contains(&entry) {
            profile
                .procedures_mut()
                .set_enabled(procedure, procedure == selected)
                .unwrap();
        }
    }
    assert!(profile.procedures().is_enabled(selected).unwrap());

    let active_offsets = std::iter::once(scenario.contact_object_offset)
        .chain(scenario.presentations.iter().flat_map(|presentation| {
            [
                presentation.object_offset,
                presentation.related_record_offset,
            ]
        }))
        .collect::<Vec<_>>();
    let active_objects = profile
        .state()
        .objects()
        .iter()
        .filter_map(|object| {
            active_offsets
                .contains(&object.source_offset())
                .then_some(object.id)
        })
        .collect::<Vec<_>>();
    for object in active_objects {
        assert!(set_object_flag(
            profile.state_mut(),
            object,
            ScriptObjectFlag::Active,
            true
        ));
    }

    configure_entry_predicates(profile, scenario);

    if let Some(presentation) = scenario.presentations.first() {
        let owner = object_at_source_offset(profile, presentation.object_offset);
        let related = object_at_source_offset(profile, presentation.related_record_offset);
        let owner_kind = profile.state().object(owner).unwrap().kind;
        let action_offset = script_field_offset(owner_kind, ScriptFieldSelector::ACTION).unwrap();
        let action_slot = profile
            .state()
            .object_word_triple(owner, action_offset / std::mem::size_of::<u16>())
            .unwrap();
        profile
            .execution_parts()
            .record_state
            .action_records
            .set_record(action_slot, ScriptActionRecord::ActorPresentation(related));
        assert_eq!(
            profile.record_state().action_records.record(action_slot),
            ScriptActionRecord::ActorPresentation(related)
        );
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(presentation.predicate_offset))
            .unwrap();
        let DecodedScriptInstruction::ActorRecord(operation) = instruction else {
            panic!(
                "{}:{} contact predicate is not a C4 actor record: {instruction:?}",
                scenario.script, scenario.procedure
            );
        };
        assert_eq!(operation.target, action_slot);
        assert_eq!(operation.related, related);
        assert_eq!(
            object_has_flag(profile.state(), owner, ScriptObjectFlag::Active),
            Some(true)
        );
        assert_eq!(
            object_has_flag(profile.state(), related, ScriptObjectFlag::Active),
            Some(true)
        );
    } else {
        let owner = object_at_source_offset(profile, scenario.contact_object_offset);
        let related = profile
            .builtins()
            .player
            .expect("every shipped profile binds the player object");
        assert!(set_object_flag(
            profile.state_mut(),
            related,
            ScriptObjectFlag::Active,
            true
        ));
        let owner_kind = profile.state().object(owner).unwrap().kind;
        let action_offset = script_field_offset(owner_kind, ScriptFieldSelector::ACTION).unwrap();
        let action_slot = profile
            .state()
            .object_word_triple(owner, action_offset / std::mem::size_of::<u16>())
            .unwrap();
        profile
            .execution_parts()
            .record_state
            .action_records
            .set_record(action_slot, ScriptActionRecord::ActorPresentation(related));
    }
    let mut synchronized = profile.synchronized_state().unwrap();
    apply_entry_state_predicates(profile, scenario, &mut synchronized);
    profile.replace_state(synchronized).unwrap();
    assert_entry_predicates(profile, scenario);
}

fn configure_entry_predicates(
    profile: &mut commander_blood_game::native::bloodprg::LoadedScriptProfile,
    scenario: &ContactScenario,
) {
    for entry in &scenario.entry_tokens {
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(entry.offset))
            .unwrap_or_else(|| {
                panic!(
                    "{}:{} has no decoded entry predicate at {}",
                    scenario.script, scenario.procedure, entry.offset
                )
            })
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
                    .expect("decoded direct-record target remains bound");
                let value = if operation.inverted && current == operation.value {
                    unequal_record_value(profile, operation.value)
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
            instruction => panic!(
                "{}:{} has unsupported entry predicate at {}: {instruction:?}",
                scenario.script, scenario.procedure, entry.offset
            ),
        }
    }
}

fn apply_entry_state_predicates(
    profile: &commander_blood_game::native::bloodprg::LoadedScriptProfile,
    scenario: &ContactScenario,
    state: &mut commander_blood_formats::script::ScriptState,
) {
    for entry in &scenario.entry_tokens {
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(entry.offset))
            .expect("contact entry instruction was validated")
            .clone();
        match instruction {
            DecodedScriptInstruction::SharedState(operation) => {
                let value = match operation.operand {
                    ScriptStateOperand::Immediate(value) => value,
                    ScriptStateOperand::StateWord(source) => state
                        .word(source)
                        .expect("decoded shared-state source remains valid"),
                };
                assert!(state.set_word(operation.target, value));
            }
            DecodedScriptInstruction::SharedBit(operation) => {
                let value = state
                    .word(operation.target)
                    .expect("decoded shared-bit target remains valid");
                let value = if operation.inverted_or_clear {
                    value & !operation.mask
                } else {
                    value | operation.mask
                };
                assert!(state.set_word(operation.target, value));
            }
            _ => {}
        }
    }
}

fn assert_entry_predicates(
    profile: &commander_blood_game::native::bloodprg::LoadedScriptProfile,
    scenario: &ContactScenario,
) {
    for entry in &scenario.entry_tokens {
        let instruction = profile
            .instruction_at(ScriptCodeOffset::new(entry.offset))
            .expect("contact entry instruction was validated");
        let passed = match instruction {
            DecodedScriptInstruction::ActorRecord(operation) => {
                let owner = operation.target.object().unwrap();
                object_has_flag(profile.state(), owner, ScriptObjectFlag::Active) == Some(true)
                    && profile
                        .record_state()
                        .action_records
                        .record(operation.target)
                        == ScriptActionRecord::ActorPresentation(operation.related)
            }
            DecodedScriptInstruction::Control(ScriptInstruction::GuardBegin { .. }) => true,
            DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard { slot }) => {
                profile.runtime().timer(*slot) == u16::MIN
            }
            DecodedScriptInstruction::SharedState(operation) => {
                let left = profile.state().word(operation.target).unwrap();
                let right = match operation.operand {
                    ScriptStateOperand::Immediate(value) => value,
                    ScriptStateOperand::StateWord(source) => profile.state().word(source).unwrap(),
                };
                operation.operator == ScriptStateOperator::EqualOrAssign && left == right
            }
            DecodedScriptInstruction::SharedBit(operation) => {
                let matched = profile.state().word(operation.target).unwrap() & operation.mask != 0;
                matched != operation.inverted_or_clear
            }
            DecodedScriptInstruction::DirectRecord(operation) => {
                let matched = profile.record_state().record_fields.value(operation.target)
                    == Some(operation.value);
                matched != operation.inverted
            }
            _ => false,
        };
        assert!(
            passed,
            "{}:{} predicate at {} is not satisfied after typed setup: {instruction:?}",
            scenario.script, scenario.procedure, entry.offset
        );
    }
}

fn unequal_record_value(
    profile: &commander_blood_game::native::bloodprg::LoadedScriptProfile,
    value: ScriptRecordValue,
) -> ScriptRecordValue {
    match value {
        ScriptRecordValue::Aboard => ScriptRecordValue::Object(
            profile
                .builtins()
                .archetype
                .expect("every shipped profile binds Arche"),
        ),
        ScriptRecordValue::Object(object) => {
            let builtins = profile.builtins();
            ScriptRecordValue::Object(
                builtins
                    .archetype
                    .filter(|candidate| *candidate != object)
                    .or(builtins.player.filter(|candidate| *candidate != object))
                    .expect("shipped profiles provide a distinct relation object"),
            )
        }
        ScriptRecordValue::Topic(_) => ScriptRecordValue::NativeWord(u16::MIN),
        ScriptRecordValue::NativeWord(word) => {
            ScriptRecordValue::NativeWord(if word == u16::MIN { u16::MAX } else { u16::MIN })
        }
    }
}

fn configure_script_context(
    scripts: &mut RuntimeScriptSystem,
    runtime: &OriginalGameRuntime,
    scenario: &ContactScenario,
) {
    scripts
        .backend_mut()
        .set_environment_activity(ScriptEnvironmentActivity {
            bridge_active: false,
            travel_active: false,
            contact_active: true,
        });
    scripts
        .backend_mut()
        .set_sequence_context(SequenceRequestContext {
            ship_active: false,
            scene_gate_active: true,
        });
    let profile = runtime.current_profile().unwrap();
    let builtins = profile.builtins();
    let contact = object_at_source_offset(profile, scenario.contact_object_offset);
    scripts
        .backend_mut()
        .set_navigation_context(builtins.archetype.map(|arche| {
            ScriptRecordStateNavigationContext {
                primary_object: contact,
                secondary_object: contact,
                arche,
            }
        }));
}

fn object_at_source_offset(
    profile: &commander_blood_game::native::bloodprg::LoadedScriptProfile,
    source_offset: usize,
) -> ScriptObjectId {
    profile
        .state()
        .objects()
        .iter()
        .find_map(|object| (object.source_offset() == source_offset).then_some(object.id))
        .unwrap_or_else(|| panic!("profile has no object at VAR offset {source_offset}"))
}

fn normalize_text(text: &[u8]) -> String {
    String::from_utf8_lossy(text)
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
