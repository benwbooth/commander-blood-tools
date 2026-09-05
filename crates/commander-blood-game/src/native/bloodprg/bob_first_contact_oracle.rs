//! Cross-routine oracle coverage for SCRIPT2 Bob first contact.

use std::convert::Infallible;
use std::path::{Path, PathBuf};

use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::descript_database::{DescriptCommand, DescriptDatabase};
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstruction, ScriptPresentationQueueOperation,
    ScriptRecordClearOperation, ScriptStateOperand, ScriptStateOperator, ScriptText,
    ScriptTextWord, ScriptTimerSlot,
};
use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptState};

use crate::assets::OriginalResourceStore;

use super::record_state::action_slot;
use super::*;

const SCRIPT2_PROFILE_INDEX: u8 = 1;
const PLAYER_STATE_OFFSET: usize = 0x0028;
const BOB_STATE_OFFSET: usize = 0x004A;
const BOB_ACTION_OFFSET: usize = 0x0084;
const BOB_STATE_WORD_OFFSET: u16 = 0x12B4;
const ADIEU_STATE_WORD_OFFSET: u16 = 0x1270;
const FIRST_CONTACT_TIMER_SLOT: u8 = 1;
const FIRST_CONTACT_TIMER_TICKS: u16 = 50;
const DISABLED_TIMER: u16 = u16::MAX;
const POST_ACTOR_CLEAR_DEPTH_STEP: u8 = 6;

const FIRST_CONTACT_LINES: &[BobLineOracle] = &[
    BobLineOracle {
        offset: 0x1C5E,
        selector: 2,
        control: 0x8000,
        text: "HONK! You worthless heap of wires... Are you working?",
        video: Some(b"bobc.hnm"),
    },
    BobLineOracle {
        offset: 0x1C7E,
        selector: -1,
        control: 0x8020,
        text: "Yes sir, Cap'n Bob sir!... Just getting the multiplexers toned up...",
        video: None,
    },
    BobLineOracle {
        offset: 0x1CA2,
        selector: 3,
        control: 0x8008,
        text: "What do you want to know, Commander?",
        video: Some(b"bobd.hnm"),
    },
    BobLineOracle {
        offset: 0x1CC6,
        selector: 6,
        control: 0x8000,
        text: "I feel like a dog I micro-waved by mistake one day... I have to cryonize...",
        video: Some(b"bobg.hnm"),
    },
    BobLineOracle {
        offset: 0x1CF0,
        selector: 3,
        control: 0x8000,
        text: "Ahhhh!!!",
        video: Some(b"bobd.hnm"),
    },
    BobLineOracle {
        offset: 0x1CFC,
        selector: -1,
        control: 0xA008,
        text: "stop",
        video: None,
    },
    BobLineOracle {
        offset: 0x1D14,
        selector: 6,
        control: 0x8048,
        text: "I feel weak, Commander... Let me sleep...",
        video: Some(b"bobg.hnm"),
    },
    BobLineOracle {
        offset: 0x1D46,
        selector: -1,
        control: 0x8020,
        text: "Ah, sleep...",
        video: None,
    },
    BobLineOracle {
        offset: 0x1D54,
        selector: -1,
        control: 0xB008,
        text: "stop",
        video: None,
    },
];

struct BobLineOracle {
    offset: usize,
    selector: i8,
    control: u16,
    text: &'static str,
    video: Option<&'static [u8]>,
}

fn original_data_root() -> Option<PathBuf> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_ORIGINAL_ARCHIVE_ROOT") {
        let root = PathBuf::from(root);
        candidates.push(root.join("resources"));
        candidates.push(root);
    }
    if let Some(root) = std::env::var_os("CBLOOD_ASSET_CACHE") {
        let root = PathBuf::from(root);
        candidates.push(root.join("resources"));
        candidates.push(root);
    }
    candidates.extend([
        workspace_root.join("output/_tmp_iso/resources"),
        workspace_root.join("commander-blood-audio/_tmp_iso/resources"),
        workspace_root.join("accuracy/cblood_install/cblood"),
    ]);
    let root = candidates
        .into_iter()
        .find(|root| root.join("SCRIPT2.COD").is_file());
    assert!(
        root.is_some() || std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none(),
        "CBLOOD_REQUIRE_ACCURACY_TESTS=1 requires original Commander Blood SCRIPT2 resources"
    );
    root
}

fn script2_profile(root: &Path) -> LoadedScriptProfile {
    let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
    let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
    let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
    let store = OriginalResourceStore::new(root.to_path_buf(), None, [], true);
    let mut cache = OriginalResourceCache::new();
    let mut manager = ScriptProfileManager::new(catalog);
    manager
        .select(
            ScriptProfileId::new(SCRIPT2_PROFILE_INDEX).unwrap(),
            &mut cache,
            &store,
            &resources,
        )
        .unwrap();
    manager.current().unwrap().clone()
}

fn text_at(profile: &LoadedScriptProfile, offset: usize) -> &ScriptText {
    let instruction = profile
        .instruction_at(ScriptCodeOffset::new(offset))
        .unwrap_or_else(|| panic!("SCRIPT2 has no instruction at {offset:#06X}"));
    let DecodedScriptInstruction::Text(text) = instruction else {
        panic!("SCRIPT2 instruction at {offset:#06X} is not A6 text")
    };
    text
}

fn plain_text(profile: &LoadedScriptProfile, text: &ScriptText) -> String {
    let mut output = Vec::new();
    for word in text.words.iter() {
        let ScriptTextWord::Dictionary(word) = word else {
            break;
        };
        let bytes = profile.dictionary().word(*word).unwrap();
        if !output.is_empty()
            && !bytes
                .first()
                .is_some_and(|byte| matches!(byte, b',' | b'.' | b'?' | b'!' | b':'))
        {
            output.push(b' ');
        }
        output.extend_from_slice(bytes);
    }
    String::from_utf8(output).unwrap()
}

fn object_and_slot(
    profile: &LoadedScriptProfile,
    name: &[u8],
    expected_offset: usize,
) -> (
    ScriptObjectId,
    commander_blood_formats::script::ScriptStateWordTriple,
) {
    let object = profile.directory().find_active_object(name).unwrap();
    assert_eq!(
        profile.state().object(object).unwrap().source_offset(),
        expected_offset
    );
    let slot = action_slot(profile.state(), object).unwrap();
    (object, slot)
}

fn serialized_slot_offset(state: &ScriptState, object: ScriptObjectId) -> usize {
    let slot = action_slot(state, object).unwrap();
    state.object(object).unwrap().source_offset()
        + slot.first_word_index() * std::mem::size_of::<u16>()
}

#[test]
fn script2_bob_text_media_audio_and_continuation_match_authored_bytes() {
    let Some(root) = original_data_root() else {
        return;
    };
    let profile = script2_profile(&root);
    let (bob, bob_slot) = object_and_slot(&profile, b"Bob_Morlock", BOB_STATE_OFFSET);
    let (player, _) = object_and_slot(&profile, b"blood", PLAYER_STATE_OFFSET);
    assert_eq!(
        serialized_slot_offset(profile.state(), bob),
        BOB_ACTION_OFFSET
    );
    assert_eq!(profile.builtins().player, Some(player));

    let store = OriginalResourceStore::new(root.clone(), None, [], true);
    let descript_name = BloodResourceName::new(b"DESCRIPT.DES").unwrap();
    let database = DescriptDatabase::parse(&store.load(&descript_name).unwrap()).unwrap();
    let bob_descript = database.lookup(b"Bob_Morlock").unwrap();
    let mut sound_bank = None;
    let mut idle_video = None;
    let mut talk_videos = Vec::new();
    for command in bob_descript.commands() {
        match command {
            DescriptCommand::SoundBank(name) => sound_bank = Some(name.as_bytes()),
            DescriptCommand::IdleClip(clip) => idle_video = Some(clip.video().as_bytes()),
            DescriptCommand::TalkClip(clip) => talk_videos.push(clip.video().as_bytes()),
            _ => {}
        }
    }
    assert_eq!(sound_bank, Some(&b"bob.snd"[..]));
    assert_eq!(idle_video, Some(&b"aabob.hnm"[..]));
    assert_eq!(talk_videos.len(), 19);

    for expected in FIRST_CONTACT_LINES {
        let text = text_at(&profile, expected.offset);
        assert_eq!(text.line_record.byte_offset(), BOB_STATE_OFFSET);
        assert_eq!(text.presentation_selector, expected.selector);
        assert_eq!(text.control.bits(), expected.control);
        assert_eq!(plain_text(&profile, text), expected.text);
        assert_eq!(
            text.control.emits_spoken_text(),
            expected.control & 0x20 != u16::MIN
        );
        match expected.video {
            Some(video) => {
                let selector = usize::try_from(expected.selector).unwrap();
                assert_eq!(talk_videos[selector], video);
                assert_eq!(
                    presentation_line_for_text_selector(expected.selector),
                    9 + u16::try_from(selector).unwrap()
                );
            }
            None => {
                assert_eq!(expected.selector, -1);
                assert_eq!(presentation_line_for_text_selector(expected.selector), 8);
            }
        }
    }

    let first = text_at(&profile, 0x1C5E);
    let first_presentation = execute_standalone_text(first, profile.dictionary());
    assert_eq!(first_presentation.0, TextHandlerOutcome::MenuPublished);
    assert!(first_presentation.1.dialogue_chatter_seed_pending);
    assert!(!first_presentation.1.subtitle_voice_trigger);

    let chatter = text_at(&profile, 0x1C7E);
    let chatter_presentation = execute_standalone_text(chatter, profile.dictionary());
    assert_eq!(
        chatter_presentation.0,
        TextHandlerOutcome::SubtitlePublished
    );
    assert!(!chatter_presentation.1.dialogue_chatter_seed_pending);
    assert!(chatter_presentation.1.subtitle_voice_trigger);
    assert_eq!(
        chatter_presentation.1.subtitle_text.as_ref(),
        b"Yes sir, Cap'n Bob sir!... Just \rgetting the multiplexers toned up... \r\r"
    );

    let third = text_at(&profile, 0x1CA2);
    let third_presentation = execute_standalone_text(third, profile.dictionary());
    assert_eq!(third_presentation.0, TextHandlerOutcome::MenuPublished);
    assert!(third_presentation.1.dialogue_chatter_seed_pending);
    assert!(!third_presentation.1.subtitle_voice_trigger);

    assert_timer_assignment(&profile, 0x1CBC, FIRST_CONTACT_TIMER_TICKS);
    assert_guard_target(&profile, 0x1CC0, 0x1D14);
    assert_timer_guard(&profile, 0x1CC3);
    assert_shared_assignment(&profile, 0x1D06, BOB_STATE_WORD_OFFSET, 2);
    assert_timer_assignment(&profile, 0x1D0D, DISABLED_TIMER);
    assert_record_clear(&profile, 0x1D11, bob_slot);

    let choice = text_at(&profile, 0x1D14);
    let choice_word = choice
        .words
        .iter()
        .skip_while(|word| !matches!(word, ScriptTextWord::SectionSeparator))
        .nth(1)
        .and_then(|word| match word {
            ScriptTextWord::Dictionary(word) => profile.dictionary().word(*word),
            ScriptTextWord::SectionSeparator => None,
            ScriptTextWord::StateNumber(_) | ScriptTextWord::InventoryChoices => {
                panic!("Commander fixture cannot contain a sequel number")
            }
        });
    assert_eq!(choice_word, Some(&b"bye_bye"[..]));

    assert_shared_assignment(&profile, 0x1D34, ADIEU_STATE_WORD_OFFSET, 1);
    assert_guard_target(&profile, 0x1D3B, 0x1D73);
    assert_shared_assignment(&profile, 0x1D3E, ADIEU_STATE_WORD_OFFSET, 1);
    assert_shared_assignment(&profile, 0x1D5E, ADIEU_STATE_WORD_OFFSET, 0);
    assert_shared_assignment(&profile, 0x1D65, BOB_STATE_WORD_OFFSET, 2);
    assert_timer_assignment(&profile, 0x1D6C, DISABLED_TIMER);
    assert_record_clear(&profile, 0x1D70, bob_slot);
}

fn execute_standalone_text(
    text: &ScriptText,
    dictionary: &commander_blood_formats::script::ScriptDictionary,
) -> (TextHandlerOutcome, TextPresentationState) {
    let mut presentation = TextPresentationState::default();
    let outcome = handle_text_instruction(
        text,
        &mut TextInstructionState::new(text),
        &mut TextLineState {
            kind: TextLineKind::Presentation,
            already_shown: false,
        },
        dictionary,
        &mut ScriptRuntime::new(),
        &mut presentation,
        TextConditionInputs::default(),
    )
    .unwrap();
    (outcome, presentation)
}

fn assert_timer_assignment(profile: &LoadedScriptProfile, offset: usize, value: u16) {
    assert_eq!(
        profile.instruction_at(ScriptCodeOffset::new(offset)),
        Some(&DecodedScriptInstruction::Control(
            ScriptInstruction::TimerAssignment {
                slot: ScriptTimerSlot::decode(FIRST_CONTACT_TIMER_SLOT).unwrap(),
                value,
            }
        ))
    );
}

fn assert_timer_guard(profile: &LoadedScriptProfile, offset: usize) {
    assert_eq!(
        profile.instruction_at(ScriptCodeOffset::new(offset)),
        Some(&DecodedScriptInstruction::Control(
            ScriptInstruction::TimerGuard {
                slot: ScriptTimerSlot::decode(FIRST_CONTACT_TIMER_SLOT).unwrap(),
            }
        ))
    );
}

fn assert_guard_target(profile: &LoadedScriptProfile, offset: usize, target: usize) {
    assert_eq!(
        profile.instruction_at(ScriptCodeOffset::new(offset)),
        Some(&DecodedScriptInstruction::Control(
            ScriptInstruction::GuardBegin {
                failure_target: ScriptCodeOffset::new(target),
            }
        ))
    );
}

fn assert_shared_assignment(
    profile: &LoadedScriptProfile,
    offset: usize,
    state_offset: u16,
    value: u16,
) {
    let DecodedScriptInstruction::SharedState(operation) = profile
        .instruction_at(ScriptCodeOffset::new(offset))
        .unwrap()
    else {
        panic!("SCRIPT2 instruction at {offset:#06X} is not shared state")
    };
    assert_eq!(
        operation.target,
        profile
            .state()
            .resolve_word_source_offset(state_offset)
            .unwrap()
    );
    assert_eq!(operation.operator, ScriptStateOperator::EqualOrAssign);
    assert_eq!(operation.operand, ScriptStateOperand::Immediate(value));
}

fn assert_record_clear(
    profile: &LoadedScriptProfile,
    offset: usize,
    target: commander_blood_formats::script::ScriptStateWordTriple,
) {
    assert_eq!(
        profile.instruction_at(ScriptCodeOffset::new(offset)),
        Some(&DecodedScriptInstruction::RecordClear(
            ScriptRecordClearOperation { target }
        ))
    );
}

#[derive(Default)]
struct RecordingActionHost {
    calls: Vec<ActionCall>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionCall {
    RadioClip(u16),
    ExecuteObject(ScriptObjectId),
}

impl ScriptActionHost for RecordingActionHost {
    type Error = Infallible;

    fn apply_description(
        &mut self,
        _object: ScriptObjectId,
        _text: &mut TextPresentationState,
    ) -> Result<ScriptActionDescription, Self::Error> {
        Ok(ScriptActionDescription::default())
    }

    fn restart_navigation_music(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn execute_object_code(
        &mut self,
        _state: &ScriptState,
        object: ScriptObjectId,
    ) -> Result<(), Self::Error> {
        self.calls.push(ActionCall::ExecuteObject(object));
        Ok(())
    }

    fn play_radio_clip(&mut self, playback_countdown: u16) -> Result<(), Self::Error> {
        self.calls.push(ActionCall::RadioClip(playback_countdown));
        Ok(())
    }

    fn start_camera_transition(&mut self, _steps: u8) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reset_ship_hud(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Default)]
struct RecordingRadioBackend {
    calls: Vec<&'static str>,
}

impl PresentationLineStepper for RecordingRadioBackend {
    type Error = Infallible;

    fn update_line(
        &mut self,
        _line: &mut PresentationLine,
        _playback: &mut PresentationLinePlayback,
    ) -> Result<PresentationLineOutcome, Self::Error> {
        self.calls.push("line");
        Ok(PresentationLineOutcome::Completed)
    }
}

impl RadioActorBackend for RecordingRadioBackend {
    fn request_radio_hand_animation(&mut self) {
        self.calls.push("hand");
    }

    fn play_radio_completion_clip(&mut self) {
        self.calls.push("clip");
    }

    fn transfer_pending_radio_record(&mut self) {
        self.calls.push("transfer");
    }

    fn reset_presentation_entity(&mut self) {
        self.calls.push("entity");
    }

    fn reload_radio_sound_bank(&mut self) {
        self.calls.push("bank");
    }
}

#[derive(Default)]
struct RecordingScanHost {
    text_resets: Vec<bool>,
    transitions: Vec<ScriptPresentationEntity>,
    dialogue_handoffs: Vec<ScriptObjectId>,
    action_dispatches: usize,
}

impl ScriptPresentationScanHost for RecordingScanHost {
    type Error = Infallible;

    fn dispatch_dialogue_control(
        &mut self,
        context: ScriptDialogueControlDispatchContext<'_>,
    ) -> Result<(), Self::Error> {
        self.dialogue_handoffs.push(context.actor);
        Ok(())
    }

    fn lookup_presentation_description(
        &mut self,
        _related: ScriptObjectId,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn restart_name_area_effect(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn transition_presentation_entity(
        &mut self,
        entity: ScriptPresentationEntity,
    ) -> Result<(), Self::Error> {
        self.transitions.push(entity);
        Ok(())
    }

    fn reset_presentation_text(&mut self, clear_primary_requests: bool) {
        self.text_resets.push(clear_primary_requests);
    }

    fn dispatch_record_action(
        &mut self,
        _context: ScriptRecordActionDispatchContext<'_>,
    ) -> Result<ScriptActionDispatch, Self::Error> {
        self.action_dispatches += 1;
        Ok(ScriptActionDispatch::default())
    }
}

#[test]
fn bob_c3_radio_c4_and_c9_ownership_follows_the_recovered_native_chain() {
    let Some(root) = original_data_root() else {
        return;
    };
    let profile = script2_profile(&root);
    let mut state = profile.state().clone();
    let mut records = profile.record_state().action_records.clone();
    let player = profile.builtins().player.unwrap();
    let arche = profile.builtins().archetype.unwrap();
    let bob = profile
        .directory()
        .find_active_object(b"Bob_Morlock")
        .unwrap();
    let bob_slot = action_slot(&state, bob).unwrap();
    let player_slot = action_slot(&state, player).unwrap();
    assert!(set_object_flag(
        &mut state,
        player,
        ScriptObjectFlag::Active,
        true
    ));
    assert!(set_object_flag(
        &mut state,
        bob,
        ScriptObjectFlag::Active,
        true
    ));
    records.set_record(bob_slot, ScriptActionRecord::Empty);
    records.set_record(player_slot, ScriptActionRecord::Empty);

    let mut runtime = ScriptRuntime::new();
    let queue = apply_presentation_queue_operation(
        ScriptPresentationQueueOperation {
            target: bob_slot,
            related: player,
            inverted: false,
        },
        &state,
        &mut records,
        &mut runtime,
    )
    .unwrap();
    assert_eq!(queue.written_slot, Some(bob_slot));
    assert_eq!(
        records.record(bob_slot),
        ScriptActionRecord::PresentationQueue(player)
    );

    let mut action = ScriptActionState::default();
    let mut presentation = ScriptPresentationScanState {
        name_lookup_enabled: true,
        ..ScriptPresentationScanState::default()
    };
    let mut text = TextPresentationState::default();
    let mut aboard = AboardObjectRoster::default();
    let mut action_host = RecordingActionHost::default();
    let queue_dispatch = dispatch_script_action(
        ScriptActionContext {
            state: &mut state,
            records: &mut records,
            aboard_objects: &mut aboard,
            text: &mut text,
            presentation: &mut presentation,
            action: &mut action,
            runtime: ScriptActionRuntimeState {
                clip_playback_state: u16::MIN,
                voc_playback_enabled: true,
                ..ScriptActionRuntimeState::default()
            },
            owner: bob,
            slot: bob_slot,
            player,
            arche,
            navigation: None,
        },
        ScriptActionRecord::PresentationQueue(player),
        &mut action_host,
    )
    .unwrap();
    assert_eq!(queue_dispatch, ScriptActionDispatch::default());
    assert_eq!(action.pending_presentation_owner, Some(bob));
    assert_eq!(action_host.calls, [ActionCall::RadioClip(2)]);

    let mut radio_state = RadioActorState::new(Some(bob), None, false);
    let mut line = PresentationLine {
        flags: PresentationLineFlags {
            present: false,
            transition_latched: false,
            resource_loaded: false,
            ready: true,
        },
        resource: PresentationResourceId::new(1),
        terminal_frame: 1,
        frame: 1,
        position: [0, 0],
    };
    let mut playback = PresentationLinePlayback::default();
    let mut radio_backend = RecordingRadioBackend::default();
    assert_eq!(
        update_radio_actor(
            true,
            &mut line,
            &mut playback,
            &mut radio_state,
            &mut radio_backend,
        )
        .unwrap(),
        RadioActorOutcome::Completed
    );
    assert_eq!(radio_state.pending_record(), None);
    assert_eq!(radio_state.deferred_record(), Some(&bob));
    assert_eq!(
        radio_state.deferred_action(),
        RadioActorDeferredAction::RadioRecord
    );
    assert_eq!(
        radio_backend.calls,
        ["hand", "line", "clip", "transfer", "entity", "bank"]
    );

    // Runtime owns the adapter from RadioActorState's deferred record into the
    // player's C4 slot. Install that exact post-adapter state to resume the
    // native-only ownership oracle without reaching outside bloodprg.
    records.set_record(player_slot, ScriptActionRecord::ActorPresentation(bob));
    records.set_actionable(player_slot, true);
    let encounter_before = encounter_count(&state, bob);
    let c4_dispatch = dispatch_script_action(
        ScriptActionContext {
            state: &mut state,
            records: &mut records,
            aboard_objects: &mut aboard,
            text: &mut text,
            presentation: &mut presentation,
            action: &mut action,
            runtime: ScriptActionRuntimeState::default(),
            owner: player,
            slot: player_slot,
            player,
            arche,
            navigation: None,
        },
        ScriptActionRecord::ActorPresentation(bob),
        &mut action_host,
    )
    .unwrap();
    assert_eq!(c4_dispatch.disposition, ScriptActionDisposition::Suppress);
    records.set_actionable(player_slot, false);
    assert_eq!(
        encounter_count(&state, bob),
        encounter_before.wrapping_add(1)
    );
    assert_eq!(action.pending_presentation_owner, None);
    assert_eq!(
        records.record(player_slot),
        ScriptActionRecord::ActorPresentation(bob)
    );
    assert_eq!(
        records.record(bob_slot),
        ScriptActionRecord::ActorPresentation(player)
    );
    assert!(!records.is_actionable(bob_slot));
    assert_eq!(
        action_host.calls,
        [ActionCall::RadioClip(2), ActionCall::ExecuteObject(bob)]
    );

    presentation.name_lookup_enabled = false;
    let mut selector = ScriptSelectorState::default();
    let mut scan_host = RecordingScanHost::default();
    let started = scan_script_presentations(
        ScriptPresentationScanContext {
            state: &mut state,
            records: &mut records,
            runtime: &mut runtime,
            selector: &mut selector,
            presentation: &mut presentation,
            player,
            arche,
        },
        &mut scan_host,
    )
    .unwrap();
    assert_eq!(started.presentation_started, Some(bob));
    assert!(presentation.active);
    assert!(presentation.start_locked);
    assert_eq!(scan_host.text_resets, [false]);
    assert_eq!(scan_host.action_dispatches, 0);

    let mut clear_presentation = ScriptRecordClearPresentationState {
        sequence_active: true,
        ship_3d_depth_step: u8::MIN,
    };
    let cleared = apply_record_clear_operation(
        ScriptRecordClearOperation { target: bob_slot },
        &state,
        &mut records,
        &mut clear_presentation,
    )
    .unwrap();
    assert_eq!(cleared.reciprocal_slot, Some(player_slot));
    assert_eq!(records.record(bob_slot), ScriptActionRecord::Empty);
    assert_eq!(records.record(player_slot), ScriptActionRecord::Empty);
    assert!(!clear_presentation.sequence_active);
    assert_eq!(
        clear_presentation.ship_3d_depth_step,
        POST_ACTOR_CLEAR_DEPTH_STEP
    );

    let ended = scan_script_presentations(
        ScriptPresentationScanContext {
            state: &mut state,
            records: &mut records,
            runtime: &mut runtime,
            selector: &mut selector,
            presentation: &mut presentation,
            player,
            arche,
        },
        &mut scan_host,
    )
    .unwrap();
    assert!(ended.presentation_ended);
    assert!(!presentation.active);
    assert_eq!(scan_host.text_resets, [false, true]);
    assert_eq!(
        scan_host.transitions,
        [
            ScriptPresentationEntity::DialogueOverlay,
            ScriptPresentationEntity::NameAreaEffect,
        ]
    );
}

fn encounter_count(state: &ScriptState, actor: ScriptObjectId) -> u16 {
    let offset = script_field_offset(
        ScriptObjectKind::Actor,
        ScriptFieldSelector::ENCOUNTER_COUNT,
    )
    .unwrap();
    let field = state
        .object_word(actor, offset / std::mem::size_of::<u16>())
        .unwrap();
    state.word(field).unwrap()
}

#[derive(Default)]
struct SubtitleRecorder {
    lines: Vec<Box<[u8]>>,
}

impl SubtitleRevealRenderer for SubtitleRecorder {
    fn draw_frame_primitive(&mut self, _draw: SubtitleFrameDraw) {}

    fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
        self.lines.push(Box::from(line.text));
    }
}

#[test]
fn bob_chatter_subtitle_survives_reveal_completion_until_the_hold_owner_releases_it() {
    let Some(root) = original_data_root() else {
        return;
    };
    let profile = script2_profile(&root);
    let chatter = text_at(&profile, 0x1C7E);
    let (outcome, mut presentation) = execute_standalone_text(chatter, profile.dictionary());
    assert_eq!(outcome, TextHandlerOutcome::SubtitlePublished);
    let exact_text = presentation.subtitle_text.clone();
    presentation.subtitle_reveal_cursor = Some(exact_text.len());
    let mut reveal = SubtitleRevealState {
        phase: SubtitleRevealPhase::Text,
        text_speed_step: 6,
        ..SubtitleRevealState::default()
    };
    let mut renderer = SubtitleRecorder::default();

    let completed =
        update_subtitle_reveal(&mut presentation, &mut reveal, &[], &[], &mut renderer).unwrap();
    assert_eq!(
        completed,
        SubtitleRevealOutcome::TextFrame {
            line_count: 3,
            reveal_advanced: false,
            completion_armed: true,
        }
    );
    assert!(presentation.subtitle_display_active);
    assert_eq!(presentation.subtitle_text, exact_text);
    assert!(presentation.dialogue_hold_complete);
    assert_eq!(presentation.dialogue_hold_countdown, 24);
    assert!(!presentation.subtitle_voice_trigger);
    assert_eq!(
        renderer.lines.iter().map(Box::as_ref).collect::<Vec<_>>(),
        [
            &b"Yes sir, Cap'n Bob sir!... Just "[..],
            &b"getting the multiplexers toned up... "[..],
            &b""[..],
        ]
    );

    let held =
        update_subtitle_reveal(&mut presentation, &mut reveal, &[], &[], &mut renderer).unwrap();
    assert_eq!(
        held,
        SubtitleRevealOutcome::TextFrame {
            line_count: 3,
            reveal_advanced: false,
            completion_armed: false,
        }
    );
    assert!(presentation.subtitle_display_active);
    assert_eq!(presentation.subtitle_text, exact_text);
    assert_eq!(renderer.lines.len(), 6);
}
