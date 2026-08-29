use std::convert::Infallible;
use std::ops::Range;

use commander_blood_formats::bas::ScriptBasInstruction;
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstruction, ScriptRecordValue, ScriptStateOperand,
    ScriptStateOperator, ScriptTextWord, ScriptTimerSlot,
};
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::script::{ScriptObjectId, ScriptProcedureId};
use commander_blood_game::native::bloodprg::{
    GameLifecycleState, GameSceneLink, GameTimerContext, GameTimerState, IndexedGamePalette,
    LoadedScriptProfile, OriginalSaveGame, PresentationPresentPolicy, PresentationRequestFlags,
    PresentationResourceId, PresentationSceneDescriptor, PresentationSceneDispatchContext,
    PresentationSceneDispatchHost, PresentationSceneDispatchOutcome,
    PresentationSceneDispatchState, PresentationSceneQueueService, PresentationSceneSource,
    ResourceLoadStatus, SCRIPT_PROFILE_RESOURCE_COUNT, SHIP_HUD_PALETTE_COLOR_COUNT,
    ScriptActionRecord, ScriptClock, ScriptEnvironmentActivity, ScriptFieldSelector,
    ScriptObjectFlag, ScriptProfileId, ScriptProfileResourceKind,
    ScriptRecordStateNavigationContext, SequenceRequestContext, advance_game_timer_tick,
    dispatch_presentation_scene, object_has_flag, presentation_line_for_text_selector,
    script_field_offset, set_object_flag, update_game_presentation_ownership,
};
use commander_blood_game::runtime::{
    OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime, RuntimeScriptSystem,
    initialize_and_restore_original_save_game,
};
use serde::Deserialize;

const CONTACT_MANIFEST_JSON: &str =
    include_str!("../../../re/vm/contact-manifest/contact-manifest.json");
const EXPECTED_CONTACT_PROCEDURE_COUNT: usize = 65;
const FIRST_SCRIPT_NUMBER: u8 = 1;
const SCRIPT_NAME_PREFIX: &str = "SCRIPT";
const PROFILE_RESOURCE_NAME_PREFIX: &str = "script";
const AUTHENTIC_SAVE_FILENAMES: &[&str] = &["GAME1.SAV", "game1.sav"];
const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const PROCEDURE_ENTRY_BIAS: usize = 1;
const OBJECT_FLAGS_WORD_INDEX: usize = 1;
const CONTACT_COUNTDOWN_TIMER_INDEX: u8 = 1;
const MAXIMUM_ENTRY_FRAMES: usize = 32;
const MAXIMUM_CONTACT_COMPLETION_FRAMES: usize = 256;
const MAXIMUM_TIMER_TICKS_PER_SCRIPT_COUNTDOWN: usize = 256;
const EXIT_DIALOGUE_TOPIC: &[u8] = b"bye_bye";
const SCRUTER_JO_PROCEDURE: &str = "scrujo";
const SCRUTER_JO_OVERLAY_VOICE_SELECTOR: u8 = 20;
const SCRUTER_JO_POST_OVERLAY_VOICE_SELECTOR: u8 = 21;
const PRIMARY_TEXT_REQUEST_PENDING: u8 = 1;
const UNCLAMPED_PRESENTATION_LINE_COUNT: usize = 8;
const PRESENTATION_DESCRIPTOR_TERMINATOR_COUNT: usize = 1;
const PROFILE_RESOURCE_IDENTITIES: &[(ScriptProfileResourceKind, &str)] = &[
    (ScriptProfileResourceKind::Code, "cod"),
    (ScriptProfileResourceKind::Dialogue, "bas"),
    (ScriptProfileResourceKind::State, "var"),
    (ScriptProfileResourceKind::Dictionary, "dic"),
    (ScriptProfileResourceKind::Directory, "deb"),
];
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
fn every_recovered_contact_completes_one_authored_path() {
    let manifest: ContactManifest = serde_json::from_str(CONTACT_MANIFEST_JSON).unwrap();
    let Some(paths) = original_data_paths() else {
        return;
    };

    for scenario in &manifest.procedures {
        let data =
            OriginalGameData::load_with_writable_root(paths.clone(), std::env::temp_dir()).unwrap();
        let mut scripts = RuntimeScriptSystem::new(&data, ORACLE_CLOCK);
        let mut runtime = OriginalGameRuntime::new(data);
        let mut timer = GameTimerState::default();
        timer.start();
        scripts
            .load_profile(&mut runtime, profile_id(&scenario.script))
            .unwrap();
        scripts.execute_frame(&mut runtime, true).unwrap();
        let selected_procedure = configure_contact_entry(&manifest, scenario, &mut runtime);
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

        let mut next_expected_index = usize::MIN;
        let mut observed_indices = Vec::new();
        let mut observed_bas_offsets = Vec::new();
        let mut selected_topics: Vec<String> = Vec::new();
        let mut bas_count_at_topic_selection = None;
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
            advance_one_script_countdown(&mut timer, &mut runtime);
            let snapshot = contact_snapshot(&scripts, &runtime);
            let selector_topics = runtime
                .current_profile()
                .unwrap()
                .selector_state()
                .pending_presentation_words()
                .to_vec();
            let published_topics = presentation_dictionary_words(&scripts);
            if snapshot.subtitle.is_empty()
                && !selector_topics.is_empty()
                && ((published_topics == selector_topics)
                    || (snapshot.word_offsets.is_empty()
                        && bas_count_at_topic_selection
                            .is_some_and(|before| observed_bas_offsets.len() > before)))
            {
                let profile = runtime.current_profile().unwrap();
                let topic_names = selector_topics
                    .iter()
                    .map(|word| profile.dictionary().word(*word).unwrap())
                    .collect::<Vec<_>>();
                if let Some(previous_bas_count) = bas_count_at_topic_selection {
                    if selected_topics.last().is_some_and(|topic| {
                        topic.as_bytes().eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC)
                    }) {
                        assert!(
                            !observed_bas_offsets.is_empty(),
                            "{}:{} reached the exit topic without first presenting BAS dialogue",
                            scenario.script,
                            scenario.procedure
                        );
                        completed = true;
                        break;
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
                let selected_index = topic_names
                    .iter()
                    .position(|name| !name.eq_ignore_ascii_case(EXIT_DIALOGUE_TOPIC))
                    .unwrap_or(usize::MIN);
                let selected = selector_topics[selected_index];
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
            let presentation_pending = snapshot.selected_line.is_some()
                || !snapshot.word_offsets.is_empty()
                || !snapshot.subtitle.is_empty();
            if presentation_pending {
                let expected_index = scenario
                    .texts
                    .iter()
                    .enumerate()
                    .skip(next_expected_index)
                    .find_map(|(index, expected)| {
                        contact_text_matches(expected, &snapshot).then_some(index)
                    });
                if let Some(expected_index) = expected_index {
                    let expected = &scenario.texts[expected_index];
                    observed_indices.push(expected_index);
                    next_expected_index = expected_index + 1;

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
                        scripts
                            .complete_word_choice(&mut runtime, choice_words[usize::MIN])
                            .unwrap();
                        word_choice_completed_this_frame = true;
                    }
                } else if let Some(source_offset) =
                    matching_bas_text_offset(runtime.current_profile().unwrap(), &snapshot)
                {
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
                    if bas_text_arms_selector_resume(
                        runtime.current_profile().unwrap(),
                        source_offset,
                    ) {
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
                        "{}:{} emitted unknown contact text {:?} / {:?} / {:?}; next expected index {}, observed COD {:?}, observed BAS {:?}, actor flags {:?}",
                        scenario.script,
                        scenario.procedure,
                        snapshot.selected_line,
                        snapshot.word_offsets,
                        snapshot.subtitle,
                        next_expected_index,
                        observed_indices,
                        observed_bas_offsets,
                        actor_flags
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
        assert_contact_host_handoff(scenario, &observed_indices);
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
        Err(_) => None,
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
        return actual.word_offsets.is_empty()
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
                navigation_link_pending: false,
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
