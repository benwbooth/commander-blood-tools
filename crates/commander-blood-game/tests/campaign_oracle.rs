use std::collections::BTreeSet;
use std::convert::Infallible;
use std::mem::size_of;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bas::ScriptBasInstruction;
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstruction, ScriptRecordValue, ScriptStateOperand,
    ScriptStateOperator, ScriptTextWord, ScriptTimerSlot,
};
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::script::{
    ScriptObjectId, ScriptProcedureId, ScriptStateByte, ScriptStateWord, ScriptStateWordPair,
};
use commander_blood_game::native::bloodprg::{
    GameLifecycleState, GameSceneLink, GameTimerContext, GameTimerState, IndexedGamePalette,
    LoadedScriptProfile, OriginalSaveGame, OriginalSaveSlotDirectory, PresentationPresentPolicy,
    PresentationRequestFlags, PresentationResourceId, PresentationSceneDescriptor,
    PresentationSceneDispatchContext, PresentationSceneDispatchHost,
    PresentationSceneDispatchOutcome, PresentationSceneDispatchState,
    PresentationSceneQueueService, PresentationSceneSource, ResourceLoadStatus,
    SCRIPT_PROFILE_RESOURCE_COUNT, SHIP_HUD_PALETTE_COLOR_COUNT, ScriptActionRecord, ScriptClock,
    ScriptEnvironmentActivity, ScriptFieldSelector, ScriptObjectFlag, ScriptProfileId,
    ScriptProfileResourceKind, ScriptRecordStateNavigationContext, SequenceRequestContext,
    advance_game_timer_tick, dispatch_presentation_scene, object_has_flag,
    presentation_line_for_text_selector, script_field_offset, set_object_flag,
    update_game_presentation_ownership,
};
use commander_blood_game::native::random::BloodPrng;
use commander_blood_game::runtime::{
    OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime, RuntimePresentationBackground,
    RuntimePresentationCatalog, RuntimeScriptSystem, initialize_and_restore_original_save_game,
};
use serde::Deserialize;
use serde_json::Value;

const CONTACT_MANIFEST_JSON: &str =
    include_str!("../../../re/vm/contact-manifest/contact-manifest.json");
const EXPECTED_CONTACT_PROCEDURE_COUNT: usize = 65;
const EXPECTED_CONTACT_TEXT_COUNT: usize = 661;
const EXPECTED_RENDERED_CONTACT_TEXT_COUNT: usize = 658;
const EXPECTED_REACHABLE_CONTACT_TEXT_COUNT: usize = 655;
const EXPECTED_CONTACT_CHOICE_EDGE_COUNT: usize = 24;
const EXPECTED_REACHABLE_CONTACT_CHOICE_EDGE_COUNT: usize = 23;
const FIRST_SCRIPT_NUMBER: u8 = 1;
const SCRIPT_NAME_PREFIX: &str = "SCRIPT";
const PROFILE_RESOURCE_NAME_PREFIX: &str = "script";
const AUTHENTIC_SAVE_FILENAMES: &[&str] = &["GAME1.SAV", "game1.sav"];
const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";
const DISPLAY_ENVIRONMENT_VARIABLES: [&str; 2] = ["DISPLAY", "WAYLAND_DISPLAY"];
const PRODUCTION_CONTACT_SCENARIO: &str = "accuracy/scenarios/production_seeded_contact.tsv";
const PRODUCTION_CONTACT_TRACE: &str = "production-seeded-contact.jsonl";
const PRODUCTION_CONTACT_SCRIPT: &str = "SCRIPT2";
const PRODUCTION_CONTACT_PROCEDURE: &str = "Cryomorn2";
const PRODUCTION_CONTACT_PROCEDURE_OFFSET: usize = 0x6a7a;
const PRODUCTION_CONTACT_NAME: &str = "Morning_Oil";
const PRODUCTION_CONTACT_SELECTION_ACTION: &str = "contact 100 89 0x6a7a";
const DOS_ORACLE_PACKED_SECOND: u8 = 39;
const ORIGINAL_SAVE_DIRECTORY_RESOURCE: &[u8] = b"BLOOD.SAV";
const PRODUCTION_CONTACT_DOS_SUBTITLES: [&str; 4] = [
    "Ahhh... Morning Oil, veteran of the Great Croolis War, reporting for duty. I greet you and say thank you also...",
    "You gave me back my life... Ah! I feel quite lubrified, I must say! Recharged batteries too, eh?",
    "My gratitude knows no bounds, noble ones. And I just love your ship. Not a model I'm familiar with...",
    "Ahh, if only you'd seen the SPIDER4000 I piloted... Such invigorating combat... BABOOOM... VRRRRRRR CHAKA CHAKA CHAKA...",
];
const PROCEDURE_ENTRY_BIAS: usize = 1;
const OBJECT_FLAGS_WORD_INDEX: usize = 1;
const CONTACT_COUNTDOWN_TIMER_INDEX: u8 = 1;
const MAXIMUM_ENTRY_FRAMES: usize = 32;
const MAXIMUM_CONTACT_COMPLETION_FRAMES: usize = 256;
const MAXIMUM_TIMER_TICKS_PER_SCRIPT_COUNTDOWN: usize = 256;
const REACHABLE_RANDOM_WARMUP_LIMIT: u8 = 128;
const CLOCK_SECONDS_PER_MINUTE: u8 = 60;
const EXIT_DIALOGUE_TOPIC: &[u8] = b"bye_bye";
const SCRUTER_JO_PROCEDURE: &str = "scrujo";
const SCRUTER_JO_OVERLAY_VOICE_SELECTOR: u8 = 20;
const SCRUTER_JO_POST_OVERLAY_VOICE_SELECTOR: u8 = 21;
const PRIMARY_TEXT_REQUEST_PENDING: u8 = 1;
const DYNAMIC_PRESENTATION_PLACEHOLDER: &[u8] = b"xxxxxxxxxxxx";
const UNCLAMPED_PRESENTATION_LINE_COUNT: usize = 8;
const PRESENTATION_DESCRIPTOR_TERMINATOR_COUNT: usize = 1;
const SCRIPT2_BOBA3_UNREACHABLE_TIMER_SLOT: u8 = 1;
const SCRIPT2_BOBA3_UNREACHABLE_TEXT_INDICES: &[usize] = &[6, 7];
const SCRIPT4_BOBA1_UNREACHABLE_TEXT_INDICES: &[usize] = &[5];
const PROFILE_RESOURCE_IDENTITIES: &[(ScriptProfileResourceKind, &str)] = &[
    (ScriptProfileResourceKind::Code, "cod"),
    (ScriptProfileResourceKind::Dialogue, "bas"),
    (ScriptProfileResourceKind::State, "var"),
    (ScriptProfileResourceKind::Dictionary, "dic"),
    (ScriptProfileResourceKind::Directory, "deb"),
];

struct ProvenUnreachableContactTexts {
    script: &'static str,
    procedure: &'static str,
    reason: ProvenUnreachableReason,
    text_indices: &'static [usize],
}

#[derive(Clone, Copy)]
enum ProvenUnreachableReason {
    TimerReassignedBeforeGuard { slot: u8 },
    BasExitEndsPresentation,
}

const PROVEN_UNREACHABLE_CONTACT_TEXTS: &[ProvenUnreachableContactTexts] = &[
    ProvenUnreachableContactTexts {
        script: "SCRIPT2",
        procedure: "boba3",
        reason: ProvenUnreachableReason::TimerReassignedBeforeGuard {
            slot: SCRIPT2_BOBA3_UNREACHABLE_TIMER_SLOT,
        },
        text_indices: SCRIPT2_BOBA3_UNREACHABLE_TEXT_INDICES,
    },
    ProvenUnreachableContactTexts {
        script: "SCRIPT4",
        procedure: "boba1",
        reason: ProvenUnreachableReason::BasExitEndsPresentation,
        text_indices: SCRIPT4_BOBA1_UNREACHABLE_TEXT_INDICES,
    },
];
const ORACLE_CLOCK: ScriptClock = ScriptClock {
    hour: 12,
    day: 2,
    month: 1,
};
static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

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
    procedure_end: usize,
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
    opcode_offset: usize,
    actor_object_offset: usize,
    voice_selector: u8,
    word_offsets: Vec<u16>,
    subtitle: String,
    choices: Vec<String>,
}

#[derive(Debug)]
struct ContactEntrySnapshot {
    selected_line: Option<i8>,
    word_offsets: Vec<u16>,
    subtitle: String,
}

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn create() -> Self {
        let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "commander-blood-contact-campaign-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "preserving failed production contact campaign at {}",
                self.0.display()
            );
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ContactStateVariant {
    Default,
    SharedWord {
        target: ScriptStateWord,
        value: u16,
    },
    SharedByte {
        target: ScriptStateByte,
        value: u8,
    },
    RecordValue {
        target: ScriptStateWord,
        value: ScriptRecordValue,
    },
    RecordPair {
        target: ScriptStateWordPair,
        value: [u16; 2],
    },
    Timer {
        slot: ScriptTimerSlot,
        value: u16,
    },
    Random(BloodPrng),
    WaitForTimer {
        slot: ScriptTimerSlot,
    },
    Combined(Vec<ContactStateVariant>),
}

#[test]
fn contact_manifest_declares_every_recovered_contact_entry() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();

    assert_eq!(manifest.procedure_count, EXPECTED_CONTACT_PROCEDURE_COUNT);
    assert_eq!(manifest.procedures.len(), EXPECTED_CONTACT_PROCEDURE_COUNT);
    assert_eq!(
        manifest
            .procedures
            .iter()
            .map(|scenario| scenario.texts.len())
            .sum::<usize>(),
        EXPECTED_CONTACT_TEXT_COUNT
    );
    assert_eq!(
        manifest
            .procedures
            .iter()
            .flat_map(|scenario| &scenario.texts)
            .filter(|text| !text.subtitle.is_empty() || !text.choices.is_empty())
            .count(),
        EXPECTED_RENDERED_CONTACT_TEXT_COUNT
    );
    assert_eq!(
        manifest
            .procedures
            .iter()
            .flat_map(|scenario| &scenario.texts)
            .map(|text| text.choices.len())
            .sum::<usize>(),
        EXPECTED_CONTACT_CHOICE_EDGE_COUNT
    );
    assert_eq!(
        manifest
            .procedures
            .iter()
            .map(|scenario| {
                scenario
                    .texts
                    .iter()
                    .enumerate()
                    .filter(|(index, text)| {
                        (!text.subtitle.is_empty() || !text.choices.is_empty())
                            && !proven_unreachable_contact_texts(scenario)
                                .is_some_and(|entry| entry.text_indices.contains(index))
                    })
                    .count()
            })
            .sum::<usize>(),
        EXPECTED_REACHABLE_CONTACT_TEXT_COUNT
    );
    assert_eq!(
        manifest
            .procedures
            .iter()
            .map(|scenario| {
                let unreachable = proven_unreachable_contact_texts(scenario)
                    .map(|entry| entry.text_indices)
                    .unwrap_or_default();
                scenario
                    .texts
                    .iter()
                    .enumerate()
                    .filter(|(index, _text)| !unreachable.contains(index))
                    .map(|(_index, text)| text.choices.len())
                    .sum::<usize>()
            })
            .sum::<usize>(),
        EXPECTED_REACHABLE_CONTACT_CHOICE_EDGE_COUNT
    );
    assert!(
        manifest
            .procedures
            .iter()
            .all(|scenario| scenario.texts.iter().any(|text| !text.subtitle.is_empty()))
    );
}

#[test]
fn declared_unreachable_contact_texts_remain_dominated_by_timer_reassignment() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let Some(paths) = original_data_paths() else {
        return;
    };

    for declared in PROVEN_UNREACHABLE_CONTACT_TEXTS {
        let ProvenUnreachableReason::TimerReassignedBeforeGuard { slot: timer_slot } =
            declared.reason
        else {
            continue;
        };
        let scenario = manifest
            .procedures
            .iter()
            .find(|scenario| {
                declared.script.eq_ignore_ascii_case(&scenario.script)
                    && declared.procedure.eq_ignore_ascii_case(&scenario.procedure)
            })
            .expect("every unreachable-contact declaration names a recovered procedure");
        let data =
            OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut scripts = RuntimeScriptSystem::new(runtime.data(), ORACLE_CLOCK);
        scripts
            .load_profile(&mut runtime, profile_id(&scenario.script))
            .unwrap();
        let profile = runtime.current_profile().unwrap();
        let slot = ScriptTimerSlot::decode(timer_slot).unwrap();
        let procedure = profile
            .code()
            .tokens()
            .iter()
            .zip(profile.instructions())
            .filter(|(token, _instruction)| {
                let offset = token.source_offset().index();
                offset > scenario.procedure_offset && offset < scenario.procedure_end
            })
            .collect::<Vec<_>>();
        let guard_index = procedure
            .iter()
            .enumerate()
            .filter_map(|(index, (_token, instruction))| {
                matches!(
                    instruction,
                    DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard {
                        slot: candidate
                    }) if *candidate == slot
                )
                .then_some(index)
            })
            .next()
            .expect("declared unreachable branch retains its timer guard");
        let assignments = procedure
            .iter()
            .take(guard_index)
            .enumerate()
            .filter_map(|(index, (_token, instruction))| {
                matches!(
                    instruction,
                    DecodedScriptInstruction::Control(ScriptInstruction::TimerAssignment {
                        slot: candidate,
                        ..
                    }) if *candidate == slot
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        assert_eq!(assignments.len(), 1);
        let assignment_index = assignments[usize::MIN];
        let DecodedScriptInstruction::Text(preceding_text) = procedure[assignment_index - 1].1
        else {
            panic!("timer reassignment is no longer immediately preceded by A6 text")
        };
        assert_eq!(preceding_text.control.rejection_skip_count(), None);

        assert!(matches!(
            procedure[guard_index - 1].1,
            DecodedScriptInstruction::Control(ScriptInstruction::GuardBegin { .. })
        ));
        let guard_offset = procedure[guard_index].0.source_offset().index();
        for text_index in declared.text_indices {
            assert!(scenario.texts[*text_index].opcode_offset > guard_offset);
        }
    }
}

#[test]
fn declared_unreachable_history_signoffs_remain_preempted_by_bas_teardown() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let Some(paths) = original_data_paths() else {
        return;
    };

    for declared in PROVEN_UNREACHABLE_CONTACT_TEXTS {
        if !matches!(
            declared.reason,
            ProvenUnreachableReason::BasExitEndsPresentation
        ) {
            continue;
        }
        let scenario = manifest
            .procedures
            .iter()
            .find(|scenario| {
                declared.script.eq_ignore_ascii_case(&scenario.script)
                    && declared.procedure.eq_ignore_ascii_case(&scenario.procedure)
            })
            .expect("every BAS-teardown declaration names a recovered procedure");
        let data =
            OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut scripts = RuntimeScriptSystem::new(runtime.data(), ORACLE_CLOCK);
        scripts
            .load_profile(&mut runtime, profile_id(&scenario.script))
            .unwrap();
        let profile = runtime.current_profile().unwrap();
        let exit_word = profile
            .dictionary()
            .words()
            .find_map(|(word, bytes)| {
                bytes
                    .eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC)
                    .then_some(word)
            })
            .expect("profile dictionary retains the exit concept");

        for text_index in declared.text_indices {
            let instruction = profile
                .instruction_at(ScriptCodeOffset::new(
                    scenario.texts[*text_index].opcode_offset,
                ))
                .expect("declared unreachable COD text remains decoded");
            let DecodedScriptInstruction::Text(text) = instruction else {
                panic!("declared unreachable COD offset is no longer A6 text")
            };
            assert!(text.control.uses_history_condition());
            assert!(text.words.iter().any(
                |word| matches!(word, ScriptTextWord::Dictionary(candidate) if *candidate == exit_word)
            ));
        }

        let bas_exit_teardown = profile.dialogue().tokens().windows(2).any(|tokens| {
            let ScriptBasInstruction::Text(text) = tokens[usize::MIN].instruction() else {
                return false;
            };
            text.line_record.byte_offset() == scenario.contact_object_offset
                && text.words.iter().any(
                    |word| matches!(word, ScriptTextWord::Dictionary(candidate) if *candidate == exit_word)
                )
                && matches!(
                    tokens[1].instruction(),
                    ScriptBasInstruction::RecordClear(_)
                )
        });
        assert!(
            bas_exit_teardown,
            "{}:{} no longer has a BAS exit line followed by presentation teardown",
            scenario.script, scenario.procedure
        );
    }
}

#[test]
fn every_profile_handoff_reloads_the_exact_authored_companion_set() {
    let Some(paths) = original_data_paths() else {
        return;
    };
    let mut transition_count = usize::MIN;

    for source in ScriptProfileId::all() {
        for target in ScriptProfileId::all() {
            let data =
                OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir())
                    .unwrap();
            let mut scripts = RuntimeScriptSystem::new(&data, ORACLE_CLOCK);
            let mut runtime = OriginalGameRuntime::new(data);

            let source_outcome = scripts.load_profile(&mut runtime, source).unwrap();
            assert!(source_outcome.profile_changed);
            assert_eq!(source_outcome.released_resources, usize::MIN);
            assert_eq!(
                source_outcome.resource_statuses,
                [ResourceLoadStatus::LoadedNow; SCRIPT_PROFILE_RESOURCE_COUNT]
            );
            assert_profile_companions(&runtime, source);
            scripts.execute_frame(&mut runtime, true).unwrap();

            let target_outcome = scripts.load_profile(&mut runtime, target).unwrap();
            let profile_changed = source != target;
            assert_eq!(target_outcome.profile_changed, profile_changed);
            assert_eq!(
                target_outcome.released_resources,
                if profile_changed {
                    SCRIPT_PROFILE_RESOURCE_COUNT
                } else {
                    usize::MIN
                },
                "profile {} -> {} released the wrong companion count",
                source.value(),
                target.value()
            );
            assert_eq!(
                target_outcome.resource_statuses,
                [if profile_changed {
                    ResourceLoadStatus::LoadedNow
                } else {
                    ResourceLoadStatus::AlreadyLoaded
                }; SCRIPT_PROFILE_RESOURCE_COUNT],
                "profile {} -> {} retained the wrong companion resources",
                source.value(),
                target.value()
            );
            assert_profile_companions(&runtime, target);
            let outcome = scripts.execute_frame(&mut runtime, true).unwrap();
            assert!(
                outcome.next_instruction.is_some(),
                "profile {} -> {} initialization terminated the VM",
                source.value(),
                target.value()
            );
            transition_count += 1;
        }
    }

    assert_eq!(
        transition_count,
        ScriptProfileId::all().count() * ScriptProfileId::all().count()
    );
}

#[test]
fn authentic_save_restores_through_the_production_flat_transaction() {
    let Some(paths) = original_data_paths() else {
        return;
    };
    let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
    let save_bytes = AUTHENTIC_SAVE_FILENAMES
        .iter()
        .find_map(|name| data.load_named_resource(name.as_bytes()).ok())
        .expect("complete original resource store has no authentic GAME1.SAV");
    let saved_profile = OriginalSaveGame::decode_profile(&save_bytes).unwrap();
    let mut scripts = RuntimeScriptSystem::new(&data, ORACLE_CLOCK);
    let mut runtime = OriginalGameRuntime::new(data);
    scripts.load_profile(&mut runtime, saved_profile).unwrap();
    let mut lifecycle = GameLifecycleState::default();

    initialize_and_restore_original_save_game(
        &mut scripts,
        &mut runtime,
        &mut lifecycle,
        &save_bytes,
    )
    .unwrap();

    assert_eq!(runtime.current_profile().unwrap().id(), saved_profile);
    assert_eq!(lifecycle.pending_profile, None);
    assert!(lifecycle.vm_execution_enabled);
    let recaptured = OriginalSaveGame::capture(runtime.current_profile().unwrap()).unwrap();
    assert_eq!(recaptured.encode(), save_bytes.as_ref());

    let outcome = scripts
        .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
        .unwrap();
    assert!(
        outcome.next_instruction.is_some(),
        "authentic save terminated on its first resumed VM frame"
    );
}

#[test]
fn every_recovered_contact_completes_every_authored_choice_path() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let Some(paths) = original_data_paths() else {
        return;
    };
    let mut uncovered = Vec::new();
    let mut uncovered_choices = Vec::new();

    for scenario in &manifest.procedures {
        let proven_unreachable = proven_unreachable_contact_texts(scenario)
            .map(|entry| entry.text_indices)
            .unwrap_or_default();
        let choice_nodes = scenario
            .texts
            .iter()
            .enumerate()
            .filter(|(index, text)| !proven_unreachable.contains(index) && !text.choices.is_empty())
            .collect::<Vec<_>>();
        assert!(
            choice_nodes.len() <= 1,
            "{}:{} requires multi-choice path enumeration",
            scenario.script,
            scenario.procedure
        );
        let choice_paths = choice_nodes.first().map_or_else(
            || vec![None],
            |(text_index, text)| {
                (usize::MIN..text.choices.len())
                    .map(|choice_index| Some((*text_index, choice_index)))
                    .collect()
            },
        );
        let state_variants = contact_state_variants(&paths, scenario);
        let mut observed_indices = BTreeSet::new();
        let mut observed_choice_edges = BTreeSet::new();
        let expected_choice_edges =
            choice_nodes
                .first()
                .map_or_else(BTreeSet::new, |(text_index, text)| {
                    (usize::MIN..text.choices.len())
                        .map(|choice_index| (*text_index, choice_index))
                        .collect()
                });
        run_contact_variants(
            &manifest,
            scenario,
            &paths,
            &choice_paths,
            &state_variants,
            &mut observed_indices,
            &mut observed_choice_edges,
        );
        let mut missing_indices = missing_rendered_contact_indices(scenario, &observed_indices);
        if !missing_indices.is_empty() || observed_choice_edges != expected_choice_edges {
            let pairwise_variants = combined_contact_state_variants(&state_variants, 2);
            run_contact_variants(
                &manifest,
                scenario,
                &paths,
                &choice_paths,
                &pairwise_variants,
                &mut observed_indices,
                &mut observed_choice_edges,
            );
            missing_indices = missing_rendered_contact_indices(scenario, &observed_indices);
        }
        if !missing_indices.is_empty() || observed_choice_edges != expected_choice_edges {
            let triple_variants = combined_contact_state_variants(&state_variants, 3);
            run_contact_variants(
                &manifest,
                scenario,
                &paths,
                &choice_paths,
                &triple_variants,
                &mut observed_indices,
                &mut observed_choice_edges,
            );
            missing_indices = missing_rendered_contact_indices(scenario, &observed_indices);
        }
        if !missing_indices.is_empty() {
            uncovered.push((
                scenario.script.clone(),
                scenario.procedure.clone(),
                missing_indices,
                observed_indices,
            ));
        }
        if observed_choice_edges != expected_choice_edges {
            uncovered_choices.push((
                scenario.script.clone(),
                scenario.procedure.clone(),
                expected_choice_edges
                    .difference(&observed_choice_edges)
                    .copied()
                    .collect::<Vec<_>>(),
            ));
        }
    }
    assert!(
        uncovered.is_empty() && uncovered_choices.is_empty(),
        "recovered contact paths left rendered COD texts uncovered: {uncovered:#?}; choice edges: {uncovered_choices:#?}"
    );
}

#[test]
fn production_runtime_reaches_the_binary_derived_morning_oil_contact() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let scenario = manifest
        .procedures
        .iter()
        .find(|scenario| {
            scenario.script == PRODUCTION_CONTACT_SCRIPT
                && scenario.procedure == PRODUCTION_CONTACT_PROCEDURE
                && scenario.procedure_offset == PRODUCTION_CONTACT_PROCEDURE_OFFSET
        })
        .expect("the contact census lost the selected production campaign");
    let Some(asset_cache) = configured_process_asset_cache() else {
        return;
    };
    if !DISPLAY_ENVIRONMENT_VARIABLES
        .iter()
        .any(|variable| std::env::var_os(variable).is_some())
    {
        assert!(
            !accuracy_tests_are_required(),
            "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires DISPLAY or WAYLAND_DISPLAY"
        );
        return;
    }

    let temporary = TemporaryRoot::create();
    let writable = temporary.0.join("writable");
    seed_manifest_contact_save(&manifest, scenario, &asset_cache, &writable);
    let trace_path = temporary.0.join(PRODUCTION_CONTACT_TRACE);
    let root = workspace_root();
    let output = Command::new(env!("CARGO_BIN_EXE_commander-blood"))
        .arg("--write-data")
        .arg(&writable)
        .arg("--scenario")
        .arg(root.join(PRODUCTION_CONTACT_SCENARIO))
        .arg("--trace")
        .arg(&trace_path)
        .arg("--oracle-packed-second")
        .arg(DOS_ORACLE_PACKED_SECOND.to_string())
        .env(ASSET_CACHE_ENVIRONMENT_VARIABLE, &asset_cache)
        .env("SDL_AUDIODRIVER", "dummy")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "production contact campaign failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let records = load_process_trace(&trace_path);

    let contacts = records
        .iter()
        .find(|record| record["semantic"]["bridge_console"]["selected"] == "contacts")
        .expect("the seeded production campaign never opened CONTACTS");
    assert_eq!(
        contacts["semantic"]["bridge_console"]["choice_labels"],
        serde_json::json!([PRODUCTION_CONTACT_NAME]),
        "the original-format save did not expose only the selected aboard contact"
    );
    let selection_index = records
        .iter()
        .position(|record| record["action"] == PRODUCTION_CONTACT_SELECTION_ACTION)
        .expect("the production trace omitted the contact selection click");
    let contact_records = &records[selection_index..];
    assert!(contact_records.iter().any(|record| {
        record["semantic"]["descript"]["active_object"]["name"] == PRODUCTION_CONTACT_NAME
    }));

    let mut next_record = usize::MIN;
    for dos_subtitle in PRODUCTION_CONTACT_DOS_SUBTITLES {
        let expected = scenario
            .texts
            .iter()
            .find(|text| normalize_text(text.subtitle.as_bytes()) == dos_subtitle)
            .unwrap_or_else(|| {
                panic!(
                    "the binary-derived contact manifest does not contain DOS checkpoint {dos_subtitle:?}"
                )
            });
        let relative_index = contact_records[next_record..]
            .iter()
            .position(|record| trace_rendered_contact_text(record).as_deref() == Some(dos_subtitle))
            .unwrap_or_else(|| {
                panic!(
                    "production contact omitted rendered DOS text {dos_subtitle:?}; observed {:?}",
                    contact_records
                        .iter()
                        .filter_map(trace_rendered_contact_text)
                        .collect::<Vec<_>>()
                )
            });
        let record_index = next_record + relative_index;
        let record = &contact_records[record_index];
        assert_eq!(
            record["semantic"]["descript"]["active_object"]["name"], PRODUCTION_CONTACT_NAME,
            "the rendered dialogue was published under the wrong DESCRIPT owner"
        );
        if !matches!(expected.voice_selector, u8::MIN | u8::MAX) {
            let line_records = &contact_records[next_record..=record_index];
            let expected_line = u64::from(expected.voice_selector) + 9;
            assert!(
                line_records.iter().any(|record| {
                    record["semantic"]["vm"]["active_line"].as_u64() == Some(expected_line)
                        && record["semantic"]["video"]["active_resource"].is_string()
                }),
                "voice selector {} did not reach authored presentation line {expected_line}",
                expected.voice_selector
            );
            let prior_audio_event_count = contact_records
                .get(next_record.saturating_sub(1))
                .map_or(usize::MIN, streamed_dialogue_event_count);
            assert!(
                streamed_dialogue_event_count(record) > prior_audio_event_count,
                "voice selector {} produced no new deterministic streamed-dialogue event",
                expected.voice_selector
            );
            assert_eq!(
                record["semantic"]["audio"]["streamed_sound_bank"],
                record["semantic"]["descript"]["sound_bank"],
                "dialogue audio did not use the active actor's DESCRIPT bank"
            );
        }
        next_record = record_index + 1;
    }
}

fn seed_manifest_contact_save(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    asset_cache: &Path,
    writable: &Path,
) {
    std::fs::create_dir_all(writable).unwrap();
    let paths = OriginalGameDataPaths::from_root(asset_cache).unwrap();
    let data = OriginalGameData::load_with_writable_root(paths, writable).unwrap();
    let directory_name = BloodResourceName::new(ORIGINAL_SAVE_DIRECTORY_RESOURCE).unwrap();
    let directory =
        OriginalSaveSlotDirectory::decode(&data.resource_store().load(&directory_name).unwrap())
            .unwrap();
    std::fs::write(writable.join("BLOOD.SAV"), directory.encode()).unwrap();

    let mut runtime = OriginalGameRuntime::new(data);
    runtime.load_profile(profile_id(&scenario.script)).unwrap();
    let selected_procedure = configure_contact_entry(manifest, scenario, &mut runtime);
    let profile = runtime.current_profile_mut().unwrap();
    let selected = object_at_source_offset(profile, scenario.contact_object_offset);
    let fallback = profile
        .builtins()
        .archetype
        .or(profile.builtins().player)
        .expect("the selected profile has no non-aboard relation object");
    let contact_objects = manifest
        .procedures
        .iter()
        .filter(|candidate| candidate.script == scenario.script)
        .map(|candidate| object_at_source_offset(profile, candidate.contact_object_offset))
        .collect::<BTreeSet<_>>();
    for object in contact_objects {
        let kind = profile.state().object(object).unwrap().kind;
        let field_offset = script_field_offset(kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
            .expect("a contact object has no holder/location field");
        let field = profile
            .state()
            .object_word(object, field_offset / size_of::<u16>())
            .expect("a contact holder/location field is outside VAR");
        profile
            .execution_parts()
            .record_state
            .record_fields
            .set_value(
                field,
                if object == selected {
                    ScriptRecordValue::Aboard
                } else {
                    ScriptRecordValue::Object(fallback)
                },
            );
    }
    let selected_kind = profile.state().object(selected).unwrap().kind;
    let action_offset = script_field_offset(selected_kind, ScriptFieldSelector::ACTION)
        .expect("the selected contact has no action field");
    let action_slot = profile
        .state()
        .object_word_triple(selected, action_offset / size_of::<u16>())
        .expect("the selected contact action field is outside VAR");
    profile
        .execution_parts()
        .record_state
        .action_records
        .set_record(action_slot, ScriptActionRecord::Empty);
    profile
        .procedures_mut()
        .set_enabled(selected_procedure, false)
        .unwrap();
    assert_eq!(
        profile.directory().object(selected).unwrap().name(),
        PRODUCTION_CONTACT_NAME.as_bytes()
    );
    let save = OriginalSaveGame::capture(profile).unwrap();
    std::fs::write(writable.join("GAME1.SAV"), save.encode()).unwrap();
}

fn configured_process_asset_cache() -> Option<PathBuf> {
    let Some(path) = std::env::var_os(ASSET_CACHE_ENVIRONMENT_VARIABLE).map(PathBuf::from) else {
        assert!(
            !accuracy_tests_are_required(),
            "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires {ASSET_CACHE_ENVIRONMENT_VARIABLE}"
        );
        return None;
    };
    assert!(
        path.is_dir(),
        "configured Commander Blood asset cache does not exist: {}",
        path.display()
    );
    Some(path)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_process_trace(path: &Path) -> Vec<Value> {
    let source = std::fs::read_to_string(path).unwrap();
    let records = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect::<Vec<_>>();
    assert!(
        !records.is_empty(),
        "production runtime wrote an empty trace"
    );
    records
}

fn trace_rendered_contact_text(record: &Value) -> Option<String> {
    let subtitle = record["semantic"]["subtitle"].as_str()?;
    if !subtitle.is_empty() && trace_raster_matches(&record["semantic"]["subtitle_raster"]) {
        return Some(normalize_text(subtitle.as_bytes()));
    }

    let menu = &record["semantic"]["presentation"]["inline_menu"];
    let words = menu["words"].as_array()?;
    let revealed_words = menu["revealed_words"].as_array()?;
    if words.is_empty()
        || words != revealed_words
        || !trace_raster_matches(&record["semantic"]["inline_menu_raster"])
    {
        return None;
    }
    let words = words
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()?;
    let mut text = String::new();
    for (index, word) in words.iter().enumerate() {
        text.push_str(word);
        if words.get(index + 1).is_some_and(|next| {
            !next
                .as_bytes()
                .first()
                .is_some_and(|byte| matches!(byte, b',' | b'.' | b'?' | b'!' | b':'))
        }) {
            text.push(' ');
        }
    }
    Some(normalize_text(text.as_bytes()))
}

fn trace_raster_matches(raster: &Value) -> bool {
    let Some(expected) = raster["expected_pixel_count"].as_u64() else {
        return false;
    };
    expected != u64::MIN
        && raster["matching_pixel_count"].as_u64() == Some(expected)
        && raster["mismatch_samples"]
            .as_array()
            .is_some_and(Vec::is_empty)
}

fn streamed_dialogue_event_count(record: &Value) -> usize {
    record["semantic"]["audio"]["events"]
        .as_array()
        .map_or(usize::MIN, |events| {
            events
                .iter()
                .filter(|event| event["kind"] == "streamed_dialogue")
                .count()
        })
}

fn run_contact_variants(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    paths: &OriginalGameDataPaths,
    choice_paths: &[Option<(usize, usize)>],
    state_variants: &[ContactStateVariant],
    observed_indices: &mut BTreeSet<usize>,
    observed_choice_edges: &mut BTreeSet<(usize, usize)>,
) {
    for selected_choice in choice_paths.iter().copied() {
        for state_variant in state_variants {
            let (path_indices, selected_choice_observed) =
                run_contact_path(manifest, scenario, paths, selected_choice, state_variant);
            if let Some(edge) = selected_choice.filter(|_| selected_choice_observed) {
                observed_choice_edges.insert(edge);
            }
            assert_contact_host_handoff(scenario, &path_indices);
            observed_indices.extend(path_indices);
        }
    }
}

fn missing_rendered_contact_indices(
    scenario: &ContactScenario,
    observed_indices: &BTreeSet<usize>,
) -> Vec<usize> {
    let proven_unreachable = proven_unreachable_contact_texts(scenario)
        .map(|entry| entry.text_indices)
        .unwrap_or_default();
    scenario
        .texts
        .iter()
        .enumerate()
        .filter_map(|(expected_index, expected)| {
            if proven_unreachable.contains(&expected_index)
                || (expected.subtitle.is_empty() && expected.choices.is_empty())
            {
                return None;
            }
            let covered = observed_indices.iter().any(|observed_index| {
                contact_texts_are_semantically_equal(expected, &scenario.texts[*observed_index])
            });
            (!covered).then_some(expected_index)
        })
        .collect()
}

fn contact_texts_are_semantically_equal(left: &ContactText, right: &ContactText) -> bool {
    left.actor_object_offset == right.actor_object_offset
        && left.voice_selector == right.voice_selector
        && left.word_offsets == right.word_offsets
        && normalize_text(left.subtitle.as_bytes()) == normalize_text(right.subtitle.as_bytes())
        && left.choices == right.choices
}

fn proven_unreachable_contact_texts(
    scenario: &ContactScenario,
) -> Option<&'static ProvenUnreachableContactTexts> {
    PROVEN_UNREACHABLE_CONTACT_TEXTS.iter().find(|entry| {
        entry.script.eq_ignore_ascii_case(&scenario.script)
            && entry.procedure.eq_ignore_ascii_case(&scenario.procedure)
    })
}

fn contact_state_variants(
    paths: &OriginalGameDataPaths,
    scenario: &ContactScenario,
) -> Vec<ContactStateVariant> {
    let data =
        OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
    let mut runtime = OriginalGameRuntime::new(data);
    let mut scripts = RuntimeScriptSystem::new(runtime.data(), ORACLE_CLOCK);
    scripts
        .load_profile(&mut runtime, profile_id(&scenario.script))
        .unwrap();
    let profile = runtime.current_profile().unwrap();
    let gate = profile
        .instruction_at(ScriptCodeOffset::new(scenario.procedure_offset))
        .expect("contact procedure gate remains decoded");
    let DecodedScriptInstruction::ProcedureGate(gate) = gate else {
        panic!(
            "{}:{} does not begin with a decoded procedure gate",
            scenario.script, scenario.procedure
        );
    };
    let procedure_end = gate.failure_target.index();
    let entry_offsets = scenario
        .entry_tokens
        .iter()
        .map(|entry| entry.offset)
        .collect::<BTreeSet<_>>();
    let mut protected_words = BTreeSet::new();
    let mut protected_bytes = BTreeSet::new();
    let mut protected_timers = BTreeSet::new();
    for (token, instruction) in profile.code().tokens().iter().zip(profile.instructions()) {
        if !entry_offsets.contains(&token.source_offset().index()) {
            continue;
        }
        match instruction {
            DecodedScriptInstruction::Control(ScriptInstruction::GuardBegin { .. }) => break,
            DecodedScriptInstruction::SharedState(operation) => {
                protected_words.insert(operation.target);
            }
            DecodedScriptInstruction::SharedBit(operation) => {
                protected_words.insert(operation.target);
            }
            DecodedScriptInstruction::DirectRecord(operation) => {
                protected_words.insert(operation.target);
            }
            DecodedScriptInstruction::BitFlag(operation) => {
                protected_bytes.insert(operation.target);
            }
            DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard { slot }) => {
                protected_timers.insert(*slot);
            }
            _ => {}
        }
    }

    let mut variants = vec![ContactStateVariant::Default];
    let mut random_gate_count = usize::MIN;
    let mut timer_guard_slots = BTreeSet::new();
    for (token, instruction) in profile.code().tokens().iter().zip(profile.instructions()) {
        let offset = token.source_offset().index();
        if offset <= scenario.procedure_offset || offset >= procedure_end {
            continue;
        }
        match instruction {
            DecodedScriptInstruction::Text(text) if text.control.uses_random_gate() => {
                random_gate_count += 1;
            }
            DecodedScriptInstruction::Control(ScriptInstruction::RandomGuard { .. }) => {
                random_gate_count += 1;
            }
            DecodedScriptInstruction::SharedState(operation)
                if !protected_words.contains(&operation.target) =>
            {
                let operand = match operation.operand {
                    ScriptStateOperand::Immediate(value) => value,
                    ScriptStateOperand::StateWord(source) => profile
                        .state()
                        .word(source)
                        .expect("decoded shared-state source remains bound"),
                };
                for value in comparison_boundary_values(operation.operator, operand) {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::SharedWord {
                            target: operation.target,
                            value,
                        },
                    );
                }
            }
            DecodedScriptInstruction::SharedBit(operation)
                if !protected_words.contains(&operation.target) =>
            {
                let current = profile.state().word(operation.target).unwrap();
                for value in [current | operation.mask, current & !operation.mask] {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::SharedWord {
                            target: operation.target,
                            value,
                        },
                    );
                }
            }
            DecodedScriptInstruction::DirectRecord(operation)
                if !protected_words.contains(&operation.target) =>
            {
                for value in [
                    operation.value,
                    unequal_record_value(profile, operation.value),
                ] {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::RecordValue {
                            target: operation.target,
                            value,
                        },
                    );
                }
            }
            DecodedScriptInstruction::BitFlag(operation)
                if !protected_bytes.contains(&operation.target) =>
            {
                let current = profile.state().byte(operation.target).unwrap();
                for value in [current | operation.mask, current & !operation.mask] {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::SharedByte {
                            target: operation.target,
                            value,
                        },
                    );
                }
            }
            DecodedScriptInstruction::RecordPair(operation) => {
                for value in [
                    operation.value,
                    [operation.value[0] ^ 1, operation.value[1]],
                ] {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::RecordPair {
                            target: operation.target,
                            value,
                        },
                    );
                }
            }
            DecodedScriptInstruction::Control(ScriptInstruction::TimerGuard { slot })
                if !protected_timers.contains(slot) =>
            {
                timer_guard_slots.insert(*slot);
                for value in [u16::MIN, 1] {
                    push_contact_variant(
                        &mut variants,
                        ContactStateVariant::Timer { value, slot: *slot },
                    );
                }
            }
            _ => {}
        }
    }
    for random in reachable_random_states_covering_each_call(random_gate_count) {
        push_contact_variant(&mut variants, ContactStateVariant::Random(random));
    }
    let unreachable_timer_slot =
        proven_unreachable_contact_texts(scenario).and_then(|entry| match entry.reason {
            ProvenUnreachableReason::TimerReassignedBeforeGuard { slot } => {
                ScriptTimerSlot::decode(slot)
            }
            ProvenUnreachableReason::BasExitEndsPresentation => None,
        });
    for slot in timer_guard_slots {
        if Some(slot) == unreachable_timer_slot {
            continue;
        }
        push_contact_variant(&mut variants, ContactStateVariant::WaitForTimer { slot });
    }
    variants
}

fn reachable_random_states_covering_each_call(call_count: usize) -> Vec<BloodPrng> {
    let mut uncovered = (usize::MIN..call_count).collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    for seconds in u8::MIN..CLOCK_SECONDS_PER_MINUTE {
        for warmup in u8::MIN..REACHABLE_RANDOM_WARMUP_LIMIT {
            let mut candidate = BloodPrng::default();
            candidate.seed_from_clock_register(seconds);
            for _ in u8::MIN..warmup {
                candidate.next(u16::MIN);
            }
            let mut probe = candidate;
            let covered = (usize::MIN..call_count)
                .filter(|_index| probe.next(5) == u16::MIN)
                .filter(|index| uncovered.contains(index))
                .collect::<Vec<_>>();
            if covered.is_empty() {
                continue;
            }
            selected.push(candidate);
            for index in covered {
                uncovered.remove(&index);
            }
            if uncovered.is_empty() {
                return selected;
            }
        }
    }
    assert!(
        uncovered.is_empty(),
        "reachable native PRNG states do not cover random calls {uncovered:?}"
    );
    selected
}

fn comparison_boundary_values(operator: ScriptStateOperator, operand: u16) -> Vec<u16> {
    let operand = operand as i16;
    let mut values = Vec::new();
    let mut push = |value: i16| {
        let value = value as u16;
        if !values.contains(&value) {
            values.push(value);
        }
    };
    match operator {
        ScriptStateOperator::NotEqual => {
            push(operand);
            push(operand.wrapping_add(1));
        }
        ScriptStateOperator::LessThan => {
            push(operand);
            if operand != i16::MIN {
                push(operand - 1);
            }
        }
        ScriptStateOperator::GreaterThan => {
            push(operand);
            if operand != i16::MAX {
                push(operand + 1);
            }
        }
        ScriptStateOperator::LessThanOrEqual => {
            push(operand);
            if operand != i16::MAX {
                push(operand + 1);
            }
        }
        ScriptStateOperator::GreaterThanOrEqual => {
            push(operand);
            if operand != i16::MIN {
                push(operand - 1);
            }
        }
        ScriptStateOperator::EqualOrAssign => {
            push(operand);
            push(operand.wrapping_add(1));
        }
        ScriptStateOperator::Add
        | ScriptStateOperator::Subtract
        | ScriptStateOperator::PreserveOrFail(_) => {}
    }
    values
}

fn push_contact_variant(variants: &mut Vec<ContactStateVariant>, variant: ContactStateVariant) {
    if !variants.contains(&variant) {
        variants.push(variant);
    }
}

fn combined_contact_state_variants(
    variants: &[ContactStateVariant],
    combination_size: usize,
) -> Vec<ContactStateVariant> {
    let atoms = variants
        .iter()
        .filter(|variant| !matches!(variant, ContactStateVariant::Default))
        .cloned()
        .collect::<Vec<_>>();
    let mut combinations = Vec::new();
    collect_contact_state_combinations(
        &atoms,
        combination_size,
        usize::MIN,
        &mut Vec::new(),
        &mut combinations,
    );
    combinations
}

fn collect_contact_state_combinations(
    atoms: &[ContactStateVariant],
    remaining: usize,
    first_index: usize,
    selected: &mut Vec<ContactStateVariant>,
    combinations: &mut Vec<ContactStateVariant>,
) {
    if remaining == usize::MIN {
        combinations.push(ContactStateVariant::Combined(selected.clone()));
        return;
    }
    for index in first_index..atoms.len() {
        if selected
            .iter()
            .any(|existing| contact_state_variants_conflict(existing, &atoms[index]))
        {
            continue;
        }
        selected.push(atoms[index].clone());
        collect_contact_state_combinations(atoms, remaining - 1, index + 1, selected, combinations);
        selected.pop();
    }
}

fn contact_state_variants_conflict(
    left: &ContactStateVariant,
    right: &ContactStateVariant,
) -> bool {
    match (left, right) {
        (
            ContactStateVariant::SharedWord { target: left, .. }
            | ContactStateVariant::RecordValue { target: left, .. },
            ContactStateVariant::SharedWord { target: right, .. }
            | ContactStateVariant::RecordValue { target: right, .. },
        ) => left == right,
        (
            ContactStateVariant::SharedByte { target: left, .. },
            ContactStateVariant::SharedByte { target: right, .. },
        ) => left == right,
        (
            ContactStateVariant::RecordPair { target: left, .. },
            ContactStateVariant::RecordPair { target: right, .. },
        ) => left == right,
        (
            ContactStateVariant::Timer { slot: left, .. },
            ContactStateVariant::Timer { slot: right, .. },
        ) => left == right,
        (ContactStateVariant::Random(_), ContactStateVariant::Random(_)) => true,
        (
            ContactStateVariant::WaitForTimer { slot: left },
            ContactStateVariant::WaitForTimer { slot: right },
        ) => left == right,
        (
            ContactStateVariant::WaitForTimer { slot: left },
            ContactStateVariant::Timer { slot: right, .. },
        )
        | (
            ContactStateVariant::Timer { slot: left, .. },
            ContactStateVariant::WaitForTimer { slot: right },
        ) => left == right,
        _ => false,
    }
}

fn apply_contact_state_variant(
    runtime: &mut OriginalGameRuntime,
    scripts: &mut RuntimeScriptSystem,
    variant: &ContactStateVariant,
) {
    match variant {
        ContactStateVariant::Default => {}
        ContactStateVariant::SharedWord { target, value } => {
            let profile = runtime.current_profile_mut().unwrap();
            let mut state = profile.synchronized_state().unwrap();
            assert!(state.set_word(*target, *value));
            profile.replace_state(state).unwrap();
            assert_eq!(profile.state().word(*target), Some(*value));
        }
        ContactStateVariant::SharedByte { target, value } => {
            let profile = runtime.current_profile_mut().unwrap();
            let mut state = profile.synchronized_state().unwrap();
            assert!(state.set_byte(*target, *value));
            profile.replace_state(state).unwrap();
            assert_eq!(profile.state().byte(*target), Some(*value));
        }
        ContactStateVariant::RecordValue { target, value } => {
            let profile = runtime.current_profile_mut().unwrap();
            profile
                .execution_parts()
                .record_state
                .record_fields
                .set_value(*target, *value);
            let state = profile.synchronized_state().unwrap();
            profile.replace_state(state).unwrap();
            assert_eq!(
                profile.record_state().record_fields.value(*target),
                Some(*value)
            );
        }
        ContactStateVariant::RecordPair { target, value } => {
            let profile = runtime.current_profile_mut().unwrap();
            let mut state = profile.synchronized_state().unwrap();
            assert!(state.set_word_pair(*target, *value));
            profile.replace_state(state).unwrap();
            assert_eq!(profile.state().word_pair(*target), Some(*value));
        }
        ContactStateVariant::Timer { slot, value } => {
            runtime
                .current_profile_mut()
                .unwrap()
                .runtime_mut()
                .assign_timer(*slot, *value);
            assert_eq!(
                runtime.current_profile().unwrap().runtime().timer(*slot),
                *value
            );
        }
        ContactStateVariant::Random(random) => scripts.import_random_state(*random),
        ContactStateVariant::WaitForTimer { .. } => {}
        ContactStateVariant::Combined(variants) => {
            for variant in variants {
                apply_contact_state_variant(runtime, scripts, variant);
            }
        }
    }
}

fn contact_variant_wait_timer(variant: &ContactStateVariant) -> Option<ScriptTimerSlot> {
    match variant {
        ContactStateVariant::WaitForTimer { slot } => Some(*slot),
        ContactStateVariant::Combined(variants) => {
            variants.iter().find_map(contact_variant_wait_timer)
        }
        _ => None,
    }
}

fn run_contact_path(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    paths: &OriginalGameDataPaths,
    selected_choice: Option<(usize, usize)>,
    state_variant: &ContactStateVariant,
) -> (Vec<usize>, bool) {
    let data =
        OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
    let mut presentation_catalog = RuntimePresentationCatalog::new(data.presentation_catalog());
    let mut scripts = RuntimeScriptSystem::new(&data, ORACLE_CLOCK);
    let mut runtime = OriginalGameRuntime::new(data);
    let mut timer = GameTimerState::default();
    timer.start();
    scripts
        .load_profile(&mut runtime, profile_id(&scenario.script))
        .unwrap();
    scripts.execute_frame(&mut runtime, true).unwrap();
    let selected_procedure = configure_contact_entry(manifest, scenario, &mut runtime);
    configure_script_context(&mut scripts, &runtime, scenario);
    runtime
        .current_profile_mut()
        .unwrap()
        .procedures_mut()
        .set_enabled(selected_procedure, false)
        .unwrap();
    scripts.execute_frame(&mut runtime, true).unwrap();
    runtime
        .current_profile_mut()
        .unwrap()
        .procedures_mut()
        .set_enabled(selected_procedure, true)
        .unwrap();
    apply_contact_state_variant(&mut runtime, &mut scripts, state_variant);

    let mut observed_indices: Vec<usize> = Vec::new();
    let mut observed_bas_offsets = Vec::new();
    let mut selected_topics: Vec<String> = Vec::new();
    let mut bas_count_at_topic_selection = None;
    let mut selected_choice_observed = selected_choice.is_none();
    let wait_timer_slot = contact_variant_wait_timer(state_variant);
    let mut waiting_choice_index = None;
    let mut completed = false;
    for _ in usize::MIN..MAXIMUM_CONTACT_COMPLETION_FRAMES {
        let mut presentation_completed_this_frame = false;
        let mut word_choice_completed_this_frame = false;
        let outcome = scripts
            .execute_frame(&mut runtime, true)
            .unwrap_or_else(|error| {
                panic!(
                    "{}:{} failed while completing contact: {error:?}",
                    scenario.script, scenario.procedure
                )
            });
        presentation_catalog
            .apply_descript_assets(scripts.backend().assets())
            .unwrap_or_else(|error| {
                panic!(
                    "{}:{} failed to publish persistent DESCRIPT media: {error:#}",
                    scenario.script, scenario.procedure
                )
            });
        advance_one_script_countdown(&mut timer, &mut runtime);
        let snapshot = contact_snapshot(&scripts, &runtime);
        let selector_topics = runtime
            .current_profile()
            .unwrap()
            .selector_state()
            .pending_presentation_words()
            .to_vec();
        let published_topics = presentation_dictionary_words(&scripts);
        if let Some(slot) = wait_timer_slot.filter(|_slot| {
            snapshot.selected_line.is_none()
                && snapshot.subtitle.is_empty()
                && (!snapshot.word_offsets.is_empty() || !selector_topics.is_empty())
        }) {
            advance_script_countdown_to_zero(&mut timer, &mut runtime, slot);
            continue;
        }
        if snapshot.subtitle.is_empty()
            && !selector_topics.is_empty()
            && ((published_topics == selector_topics)
                || (snapshot.word_offsets.is_empty()
                    && !scripts.presentation_scan_state().start_locked)
                || (snapshot.word_offsets.is_empty()
                    && bas_count_at_topic_selection
                        .is_some_and(|before| observed_bas_offsets.len() > before)))
        {
            if let Some(slot) = wait_timer_slot {
                advance_script_countdown_to_zero(&mut timer, &mut runtime, slot);
                continue;
            }
            if !matches!(state_variant, ContactStateVariant::Default) {
                assert!(
                    !observed_indices.is_empty(),
                    "{}:{} entered BAS selection before its fuzzed COD contact emitted text",
                    scenario.script,
                    scenario.procedure
                );
                completed = true;
                break;
            }
            let profile = runtime.current_profile().unwrap();
            let topic_names = selector_topics
                .iter()
                .map(|word| profile.dictionary().word(*word).unwrap())
                .collect::<Vec<_>>();
            if let Some(previous_bas_count) = bas_count_at_topic_selection {
                if selected_topics
                    .last()
                    .is_some_and(|topic| topic.as_bytes().eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC))
                {
                    assert!(
                        !observed_bas_offsets.is_empty(),
                        "{}:{} reached the exit topic without first presenting BAS dialogue",
                        scenario.script,
                        scenario.procedure
                    );
                    if selected_choice.is_none() || selected_choice_observed {
                        completed = true;
                        break;
                    }
                    continue;
                }
                assert!(
                    observed_bas_offsets.len() > previous_bas_count,
                    "{}:{} returned to selector {:?} without presenting BAS dialogue after topic {:?}; snapshot {:?}, selector state {:?}, presentation {:?}",
                    scenario.script,
                    scenario.procedure,
                    topic_names
                        .iter()
                        .map(|name| String::from_utf8_lossy(name).into_owned())
                        .collect::<Vec<_>>(),
                    selected_topics.last(),
                    snapshot,
                    profile.selector_state(),
                    scripts.presentation_scan_state()
                );
                if let Some(exit_index) = topic_names
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC))
                {
                    let selected = selector_topics[exit_index];
                    selected_topics.push(
                        String::from_utf8_lossy(profile.dictionary().word(selected).unwrap())
                            .into_owned(),
                    );
                    bas_count_at_topic_selection = Some(observed_bas_offsets.len());
                    scripts
                        .complete_word_choice(&mut runtime, selected)
                        .unwrap();
                    continue;
                }
                completed = true;
                break;
            }
            let selected_index = if selected_choice.is_some() {
                topic_names
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC))
                    .expect("a COD exit choice must be reachable from the BAS selector")
            } else {
                topic_names
                    .iter()
                    .position(|name| !name.eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC))
                    .unwrap_or(usize::MIN)
            };
            let selected = selector_topics[selected_index];
            selected_topics.push(
                String::from_utf8_lossy(profile.dictionary().word(selected).unwrap()).into_owned(),
            );
            bas_count_at_topic_selection = Some(observed_bas_offsets.len());
            scripts
                .complete_word_choice(&mut runtime, selected)
                .unwrap();
            continue;
        }
        let presentation_pending = snapshot.selected_line.is_some()
            || !snapshot.word_offsets.is_empty()
            || !snapshot.subtitle.is_empty();
        if presentation_pending {
            let consumed_presentation_tail = snapshot.word_offsets.is_empty()
                && snapshot.subtitle.is_empty()
                && observed_indices.iter().any(|index| {
                    snapshot.selected_line == Some(scenario.texts[*index].voice_selector as i8)
                });
            if consumed_presentation_tail {
                complete_contact_text(&mut scripts);
                continue;
            }
            let expected_index = scenario
                .texts
                .iter()
                .enumerate()
                .filter(|(index, _expected)| !observed_indices.contains(index))
                .find_map(|(index, expected)| {
                    contact_text_matches(expected, &snapshot).then_some(index)
                })
                .or_else(|| {
                    waiting_choice_index
                        .filter(|index| contact_text_matches(&scenario.texts[*index], &snapshot))
                });
            if let Some(expected_index) = expected_index {
                let expected = &scenario.texts[expected_index];
                assert_contact_media_selection(
                    scenario,
                    expected.actor_object_offset,
                    &snapshot,
                    &presentation_catalog,
                    &scripts,
                    &runtime,
                );
                if !observed_indices.contains(&expected_index) {
                    observed_indices.push(expected_index);
                }

                if !expected.choices.is_empty() {
                    let profile = runtime.current_profile().unwrap();
                    let choice_words = if snapshot.word_offsets.contains(&u16::MAX) {
                        contact_choice_words(&scripts, &runtime)
                    } else {
                        selector_topics.clone()
                    };
                    let actual_choices = choice_words
                        .iter()
                        .map(|word| {
                            String::from_utf8_lossy(profile.dictionary().word(*word).unwrap())
                                .into_owned()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        actual_choices.len(),
                        expected.choices.len(),
                        "{}:{} exposed the wrong choice count at text {}",
                        scenario.script,
                        scenario.procedure,
                        expected_index
                    );
                    assert!(
                        actual_choices
                            .iter()
                            .zip(&expected.choices)
                            .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected)),
                        "{}:{} exposed choices {:?}, expected {:?}",
                        scenario.script,
                        scenario.procedure,
                        actual_choices,
                        expected.choices
                    );
                    if wait_timer_slot.is_some() {
                        waiting_choice_index = Some(expected_index);
                        word_choice_completed_this_frame = true;
                    } else {
                        let choice_index = selected_choice
                            .filter(|(text_index, _choice_index)| *text_index == expected_index)
                            .map_or(usize::MIN, |(_text_index, choice_index)| choice_index);
                        assert!(
                            choice_index < choice_words.len(),
                            "{}:{} choice {} is outside text {}",
                            scenario.script,
                            scenario.procedure,
                            choice_index,
                            expected_index
                        );
                        selected_choice_observed |=
                            selected_choice == Some((expected_index, choice_index));
                        scripts
                            .complete_word_choice(&mut runtime, choice_words[choice_index])
                            .unwrap();
                        word_choice_completed_this_frame = true;
                    }
                } else {
                    waiting_choice_index = None;
                }
            } else if let Some(source_offset) =
                matching_bas_text_offset(runtime.current_profile().unwrap(), &snapshot)
            {
                assert_contact_media_selection(
                    scenario,
                    scenario.texts[usize::MIN].actor_object_offset,
                    &snapshot,
                    &presentation_catalog,
                    &scripts,
                    &runtime,
                );
                let repeated_terminal_response = observed_bas_offsets.last()
                    == Some(&source_offset)
                    && bas_count_at_topic_selection
                        .is_some_and(|before| observed_bas_offsets.len() > before);
                if repeated_terminal_response {
                    completed = true;
                    break;
                }
                if observed_bas_offsets.last() != Some(&source_offset) {
                    observed_bas_offsets.push(source_offset);
                }
                if bas_text_arms_selector_resume(runtime.current_profile().unwrap(), source_offset)
                {
                    let choice_words = contact_choice_words(&scripts, &runtime);
                    let selected = choice_words[usize::MIN];
                    selected_topics.push(
                        String::from_utf8_lossy(
                            runtime
                                .current_profile()
                                .unwrap()
                                .dictionary()
                                .word(selected)
                                .unwrap(),
                        )
                        .into_owned(),
                    );
                    scripts
                        .complete_word_choice(&mut runtime, selected)
                        .unwrap();
                    word_choice_completed_this_frame = true;
                }
            } else {
                let actor = object_at_source_offset(
                    runtime.current_profile().unwrap(),
                    scenario.texts[usize::MIN].actor_object_offset,
                );
                let actor_flags = runtime
                    .current_profile()
                    .unwrap()
                    .state()
                    .object_word(actor, OBJECT_FLAGS_WORD_INDEX)
                    .and_then(|field| runtime.current_profile().unwrap().state().word(field));
                panic!(
                    "{}:{} emitted unknown contact text {:?} / {:?} / {:?}; observed COD {:?}, observed BAS {:?}, actor flags {:?}, variant {:?}",
                    scenario.script,
                    scenario.procedure,
                    snapshot.selected_line,
                    snapshot.word_offsets,
                    snapshot.subtitle,
                    observed_indices,
                    observed_bas_offsets,
                    actor_flags,
                    state_variant
                )
            }
            if !word_choice_completed_this_frame {
                complete_contact_text(&mut scripts);
            }
            presentation_completed_this_frame = true;
        }
        let selected_procedure_enabled = runtime
            .current_profile()
            .unwrap()
            .procedures()
            .is_enabled(selected_procedure)
            .unwrap();
        let one_shot_procedure_completed = !selected_procedure_enabled;
        let persistent_dialogue_completed =
            !presentation_completed_this_frame && !scripts.presentation_scan_state().active;
        if !observed_indices.is_empty()
            && (one_shot_procedure_completed || persistent_dialogue_completed)
        {
            completed = true;
            break;
        }
        assert!(
            outcome.next_instruction.is_some(),
            "{}:{} terminated before presentation teardown; observed {:?}, selected topics {:?}",
            scenario.script,
            scenario.procedure,
            observed_indices,
            selected_topics
        );
    }

    assert!(
        completed,
        "{}:{} did not complete an authored path within {} frames; observed COD {:?}, observed BAS {:?}, selected topics {:?}, BAS count at selection {:?}, contact timer {}, current text {:?}, raw text {:?}, presentation {:?}, scan {:?}, pending selector words {:?}",
        scenario.script,
        scenario.procedure,
        MAXIMUM_CONTACT_COMPLETION_FRAMES,
        observed_indices,
        observed_bas_offsets,
        selected_topics,
        bas_count_at_topic_selection,
        runtime
            .current_profile()
            .unwrap()
            .runtime()
            .timer(ScriptTimerSlot::decode(CONTACT_COUNTDOWN_TIMER_INDEX).unwrap()),
        contact_snapshot(&scripts, &runtime),
        scripts.text_presentation(),
        scripts.presentation_scan_state(),
        scripts.last_presentation_outcome(),
        runtime
            .current_profile()
            .unwrap()
            .selector_state()
            .pending_presentation_words()
            .iter()
            .map(|word| String::from_utf8_lossy(
                runtime
                    .current_profile()
                    .unwrap()
                    .dictionary()
                    .word(*word)
                    .unwrap()
            )
            .into_owned())
            .collect::<Vec<_>>()
    );
    (observed_indices, selected_choice_observed)
}

fn assert_contact_media_selection(
    scenario: &ContactScenario,
    actor_source_offset: usize,
    snapshot: &ContactEntrySnapshot,
    catalog: &RuntimePresentationCatalog,
    scripts: &RuntimeScriptSystem,
    runtime: &OriginalGameRuntime,
) {
    let Some(selector) = snapshot.selected_line else {
        return;
    };
    if selector < 0 {
        return;
    }
    let actor = object_at_source_offset(runtime.current_profile().unwrap(), actor_source_offset);
    assert_eq!(
        scripts.backend().active_description_object(),
        Some(actor),
        "{}:{} selected dialogue for an actor that does not own the active DESCRIPT record",
        scenario.script,
        scenario.procedure
    );

    let line = PresentationResourceId::new(presentation_line_for_text_selector(selector));
    let resource = catalog.resource_name(line).unwrap_or_else(|| {
        panic!(
            "{}:{} selector {selector} has no persistent presentation resource",
            scenario.script, scenario.procedure
        )
    });
    assert!(
        !resource
            .as_bytes()
            .ends_with(DYNAMIC_PRESENTATION_PLACEHOLDER),
        "{}:{} selector {selector} retained the executable's unresolved dynamic placeholder",
        scenario.script,
        scenario.procedure
    );
    runtime
        .data()
        .resource_store()
        .load(resource)
        .unwrap_or_else(|error| {
            panic!(
                "{}:{} selector {selector} selected missing media {}: {error:#}",
                scenario.script,
                scenario.procedure,
                String::from_utf8_lossy(resource.as_bytes())
            )
        });

    if let Some(RuntimePresentationBackground::Cached(slot)) = catalog.background(line) {
        let Some(background) = scripts.backend().backgrounds().get(slot) else {
            // Character records select an inherited scene slot. This isolated
            // contact harness has no preceding location presentation.
            return;
        };
        let background_name = BloodResourceName::new(background.source_name()).unwrap();
        runtime
            .data()
            .resource_store()
            .load(&background_name)
            .unwrap_or_else(|error| {
                panic!(
                    "{}:{} selector {selector} selected missing background {}: {error:#}",
                    scenario.script,
                    scenario.procedure,
                    String::from_utf8_lossy(background.source_name())
                )
            });
    }
}

fn assert_contact_host_handoff(scenario: &ContactScenario, observed_indices: &[usize]) {
    if !scenario
        .procedure
        .eq_ignore_ascii_case(SCRUTER_JO_PROCEDURE)
    {
        return;
    }

    let overlay_index = scenario
        .texts
        .iter()
        .position(|text| text.voice_selector == SCRUTER_JO_OVERLAY_VOICE_SELECTOR)
        .expect("every Scruter Jo contact must declare the AMER overlay selector");
    let post_overlay_index = scenario
        .texts
        .iter()
        .position(|text| text.voice_selector == SCRUTER_JO_POST_OVERLAY_VOICE_SELECTOR)
        .expect("every Scruter Jo contact must declare its post-overlay response");
    assert!(
        observed_indices.contains(&overlay_index),
        "{}:{} never reached its authored AMER overlay selector; observed {:?}",
        scenario.script,
        scenario.procedure,
        observed_indices
    );
    assert!(
        observed_indices.contains(&post_overlay_index),
        "{}:{} never resumed at its authored post-AMER response; observed {:?}",
        scenario.script,
        scenario.procedure,
        observed_indices
    );
    assert!(overlay_index < post_overlay_index);

    let mut lifecycle = GameLifecycleState::default();
    lifecycle.presentation.active = true;
    lifecycle.presentation.scene_gate_active = true;
    lifecycle.presentation.text_menu_pending = true;
    lifecycle.presentation.text_selector = Some(SCRUTER_JO_OVERLAY_VOICE_SELECTOR as i8);
    lifecycle.presentation.request_flags =
        PresentationRequestFlags::decode(PRIMARY_TEXT_REQUEST_PENDING);
    let mut scene_link = GameSceneLink::Initial;
    update_game_presentation_ownership(&mut lifecycle, &mut scene_link);
    let expected_line =
        presentation_line_for_text_selector(SCRUTER_JO_OVERLAY_VOICE_SELECTOR as i8);
    assert_eq!(lifecycle.presentation.active_line, Some(expected_line));

    let record = scenario.contact_object_offset;
    let scenes = vec![
        PresentationSceneDescriptor { image: None };
        usize::from(expected_line) + PRESENTATION_DESCRIPTOR_TERMINATOR_COUNT
    ];
    let mut scene_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    let mut presentation_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; SHIP_HUD_PALETTE_COLOR_COUNT];
    let unclamped_line_ids = [u8::MIN; UNCLAMPED_PRESENTATION_LINE_COUNT];
    let mut context = PresentationSceneDispatchContext {
        scenes: &scenes,
        active_record_related: Some(&record),
        scruter_jo_record: Some(&record),
        unclamped_line_ids: &unclamped_line_ids,
        shared_cache_available: false,
        scene_palette: &mut scene_palette,
        presentation_palette: &mut presentation_palette,
    };
    let mut state = PresentationSceneDispatchState {
        presentation: commander_blood_game::native::bloodprg::PresentationUpdateState {
            active_line: lifecycle.presentation.active_line,
            ..commander_blood_game::native::bloodprg::PresentationUpdateState::default()
        },
        scene_gate: true,
        ..PresentationSceneDispatchState::default()
    };
    let mut host = CompletedPresentationHost;

    assert!(matches!(
        dispatch_presentation_scene(&mut state, &mut context, &mut host).unwrap(),
        PresentationSceneDispatchOutcome::SequenceStarted { .. }
    ));
    assert!(state.alien_overlay_armed);
    assert_eq!(
        dispatch_presentation_scene(&mut state, &mut context, &mut host).unwrap(),
        PresentationSceneDispatchOutcome::PresentationFinished
    );
    assert!(state.temporary_sound_trigger);
}

struct CompletedPresentationHost;

impl PresentationSceneDispatchHost<()> for CompletedPresentationHost {
    type Error = Infallible;

    fn load_scene_image(
        &mut self,
        _image: &(),
        _scene_palette: &mut IndexedGamePalette,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear_back_buffer_band(
        &mut self,
        _rows: Range<usize>,
        _color: u8,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn load_presentation_sequence(
        &mut self,
        _resource: PresentationResourceId,
        _source: PresentationSceneSource,
        _policy: PresentationPresentPolicy,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn build_black_remap(
        &mut self,
        _blend_percent: u8,
        _target: [u8; RGB_COMPONENT_COUNT],
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn service_presentation_queue(
        &mut self,
        _policy: PresentationPresentPolicy,
    ) -> Result<PresentationSceneQueueService, Self::Error> {
        Ok(PresentationSceneQueueService::default())
    }

    fn presentation_source_open_or_draining(&mut self) -> bool {
        false
    }

    fn clear_display_band(&mut self, _rows: Range<usize>, _color: u8) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[test]
fn every_recovered_contact_enters_the_expected_rust_presentation() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let Some(paths) = original_data_paths() else {
        return;
    };

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
            let current = contact_snapshot(&scripts, &runtime);
            let matches_contact = scenario
                .texts
                .iter()
                .any(|expected| contact_text_matches(expected, &current));
            if matches_contact {
                snapshot = Some(current);
                break;
            }
            if outcome.presentation_yields != 0 {
                clear_contact_presentation(&mut scripts);
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
            .find(|text| contact_text_matches(text, &snapshot))
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

fn original_data_paths() -> Option<OriginalGameDataPaths> {
    match OriginalGameDataPaths::discover(None) {
        Ok(paths) => Some(paths),
        Err(error) if std::env::var_os(ASSET_CACHE_ENVIRONMENT_VARIABLE).is_some() => {
            panic!("configured Commander Blood asset cache is invalid: {error:#}")
        }
        Err(error) if accuracy_tests_are_required() => {
            panic!(
                "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires original Commander Blood data: {error:#}"
            )
        }
        Err(_) => None,
    }
}

fn accuracy_tests_are_required() -> bool {
    std::env::var_os(REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE).is_some()
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

fn assert_profile_companions(runtime: &OriginalGameRuntime, expected: ScriptProfileId) {
    let profile = runtime
        .current_profile()
        .expect("profile loader must retain the selected profile");
    assert_eq!(profile.id(), expected);
    assert_profile_resource_names(runtime, profile);
}

fn assert_profile_resource_names(runtime: &OriginalGameRuntime, profile: &LoadedScriptProfile) {
    let profile_number = profile.id().value() + FIRST_SCRIPT_NUMBER;
    let catalog = runtime.data().resource_catalog();
    for &(kind, extension) in PROFILE_RESOURCE_IDENTITIES {
        let resource = profile.resources().resource(kind);
        let actual = catalog.name(resource).unwrap_or_else(|| {
            panic!(
                "profile resource {} is absent from the catalog",
                resource.value()
            )
        });
        let expected = format!("{PROFILE_RESOURCE_NAME_PREFIX}{profile_number}.{extension}");
        assert_eq!(
            actual.as_bytes(),
            expected.as_bytes(),
            "profile {} {:?} resource {} has the wrong authored identity",
            profile.id().value(),
            kind,
            resource.value()
        );
    }
}

fn configure_contact_entry(
    manifest: &ContactManifest,
    scenario: &ContactScenario,
    runtime: &mut OriginalGameRuntime,
) -> ScriptProcedureId {
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
    selected
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
        ScriptRecordValue::Topic(word) => ScriptRecordValue::Topic(
            profile
                .dictionary()
                .words()
                .find_map(|(candidate, _bytes)| (candidate != word).then_some(candidate))
                .expect("shipped dictionaries contain more than one topic"),
        ),
        ScriptRecordValue::NativeWord(_) => ScriptRecordValue::Aboard,
    }
}

fn configure_script_context(
    scripts: &mut RuntimeScriptSystem,
    runtime: &OriginalGameRuntime,
    scenario: &ContactScenario,
) {
    scripts.presentation_scan_state_mut().name_lookup_enabled = true;
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

fn contact_snapshot(
    scripts: &RuntimeScriptSystem,
    runtime: &OriginalGameRuntime,
) -> ContactEntrySnapshot {
    let text = scripts.text_presentation();
    let profile = runtime.current_profile().unwrap();
    let word_offsets = text
        .menu_words
        .iter()
        .map(|word| match word {
            ScriptTextWord::Dictionary(word) => profile.dictionary().source_offset(*word).unwrap(),
            ScriptTextWord::SectionSeparator => u16::MAX,
        })
        .collect::<Vec<_>>();
    ContactEntrySnapshot {
        selected_line: text.selected_line,
        word_offsets,
        subtitle: if text.subtitle_display_active {
            normalize_text(&text.subtitle_text)
        } else {
            String::new()
        },
    }
}

fn contact_text_matches(expected: &ContactText, actual: &ContactEntrySnapshot) -> bool {
    if actual.selected_line != Some(expected.voice_selector as i8) {
        return false;
    }
    if !actual.subtitle.is_empty() {
        if normalize_text(expected.subtitle.as_bytes()) != actual.subtitle {
            return false;
        }
        return expected.choices.is_empty()
            || actual.word_offsets.is_empty()
            || post_separator_offsets(&expected.word_offsets) == actual.word_offsets;
    }
    expected.word_offsets == actual.word_offsets
}

fn matching_bas_text_offset(
    profile: &LoadedScriptProfile,
    actual: &ContactEntrySnapshot,
) -> Option<usize> {
    profile.dialogue().tokens().iter().find_map(|token| {
        let ScriptBasInstruction::Text(text) = token.instruction() else {
            return None;
        };
        if actual.selected_line != Some(text.presentation_selector) {
            return None;
        }
        let matches = if !actual.subtitle.is_empty() {
            normalized_script_words(profile, &text.words) == actual.subtitle
                && (actual.word_offsets.is_empty()
                    || post_separator_script_offsets(profile, &text.words) == actual.word_offsets)
        } else {
            text.words
                .iter()
                .map(|word| match word {
                    ScriptTextWord::Dictionary(word) => {
                        profile.dictionary().source_offset(*word).unwrap()
                    }
                    ScriptTextWord::SectionSeparator => u16::MAX,
                })
                .eq(actual.word_offsets.iter().copied())
        };
        matches.then_some(token.source_offset().index())
    })
}

fn bas_text_arms_selector_resume(profile: &LoadedScriptProfile, source_offset: usize) -> bool {
    profile.dialogue().tokens().iter().any(|token| {
        token.source_offset().index() == source_offset
            && matches!(
                token.instruction(),
                ScriptBasInstruction::Text(text) if text.control.arms_resume()
            )
    })
}

fn post_separator_offsets(offsets: &[u16]) -> &[u16] {
    offsets
        .iter()
        .position(|offset| *offset == u16::MAX)
        .map_or(&[], |separator| &offsets[separator + 1..])
}

fn post_separator_script_offsets(
    profile: &LoadedScriptProfile,
    words: &[ScriptTextWord],
) -> Vec<u16> {
    words
        .iter()
        .skip_while(|word| !matches!(word, ScriptTextWord::SectionSeparator))
        .skip(1)
        .filter_map(|word| match word {
            ScriptTextWord::Dictionary(word) => profile.dictionary().source_offset(*word),
            ScriptTextWord::SectionSeparator => None,
        })
        .collect()
}

fn normalized_script_words(profile: &LoadedScriptProfile, words: &[ScriptTextWord]) -> String {
    let spoken_words = words
        .iter()
        .take_while(|word| matches!(word, ScriptTextWord::Dictionary(_)))
        .filter_map(|word| match word {
            ScriptTextWord::Dictionary(word) => Some(*word),
            ScriptTextWord::SectionSeparator => None,
        })
        .collect::<Vec<_>>();
    let mut text = Vec::new();
    for (index, word) in spoken_words.iter().copied().enumerate() {
        text.extend_from_slice(profile.dictionary().word(word).unwrap());
        let next = spoken_words
            .get(index + 1)
            .and_then(|word| profile.dictionary().word(*word).unwrap().first())
            .copied();
        if !next.is_some_and(is_attached_subtitle_punctuation) {
            text.push(b' ');
        }
    }
    normalize_text(&text)
}

const fn is_attached_subtitle_punctuation(byte: u8) -> bool {
    matches!(byte, b',' | b'.' | b'?' | b'!' | b':')
}

fn contact_choice_words(
    scripts: &RuntimeScriptSystem,
    runtime: &OriginalGameRuntime,
) -> Vec<commander_blood_formats::script::ScriptWordId> {
    let profile = runtime.current_profile().unwrap();
    let words = &scripts.text_presentation().menu_words;
    let separator = words
        .iter()
        .position(|word| matches!(word, ScriptTextWord::SectionSeparator))
        .expect("manifest-declared contact choice has no section separator");
    words[separator + 1..]
        .iter()
        .map(|word| match word {
            ScriptTextWord::Dictionary(word) => {
                profile
                    .dictionary()
                    .word(*word)
                    .expect("contact choice dictionary word remains valid");
                *word
            }
            ScriptTextWord::SectionSeparator => {
                panic!("contact choice list contains a second section separator")
            }
        })
        .collect()
}

fn presentation_dictionary_words(
    scripts: &RuntimeScriptSystem,
) -> Vec<commander_blood_formats::script::ScriptWordId> {
    scripts
        .text_presentation()
        .menu_words
        .iter()
        .filter_map(|word| match word {
            ScriptTextWord::Dictionary(word) => Some(*word),
            ScriptTextWord::SectionSeparator => None,
        })
        .collect()
}

fn clear_contact_presentation(scripts: &mut RuntimeScriptSystem) {
    *scripts.text_presentation_mut() = Default::default();
    *scripts.presentation_scan_state_mut() = Default::default();
}

fn complete_contact_text(scripts: &mut RuntimeScriptSystem) {
    *scripts.text_presentation_mut() = Default::default();
    let presentation = scripts.presentation_scan_state_mut();
    presentation.start_locked = false;
    presentation.word_choice_active = false;
    presentation.hold_ready = false;
    presentation.dialogue_hold_complete = false;
}

fn advance_one_script_countdown(timer: &mut GameTimerState, runtime: &mut OriginalGameRuntime) {
    let previous_countdown_count = timer.mouse_motion_idle_counter;
    let profile = runtime.current_profile_mut().unwrap();
    for _ in usize::MIN..MAXIMUM_TIMER_TICKS_PER_SCRIPT_COUNTDOWN {
        advance_game_timer_tick(
            timer,
            profile.runtime_mut(),
            GameTimerContext {
                paused: false,
                pending_record_link: false,
            },
        );
        if timer.mouse_motion_idle_counter != previous_countdown_count {
            return;
        }
    }
    panic!(
        "native timer cadence did not reach a script countdown within {} ticks",
        MAXIMUM_TIMER_TICKS_PER_SCRIPT_COUNTDOWN
    );
}

fn advance_script_countdown_to_zero(
    timer: &mut GameTimerState,
    runtime: &mut OriginalGameRuntime,
    slot: ScriptTimerSlot,
) {
    let countdown = runtime.current_profile().unwrap().runtime().timer(slot);
    assert!(
        usize::from(countdown) <= MAXIMUM_CONTACT_COMPLETION_FRAMES,
        "contact timer {} contains non-countdown value {}",
        slot.index(),
        countdown
    );
    for _ in u16::MIN..countdown {
        advance_one_script_countdown(timer, runtime);
    }
    assert_eq!(
        runtime.current_profile().unwrap().runtime().timer(slot),
        u16::MIN
    );
}
