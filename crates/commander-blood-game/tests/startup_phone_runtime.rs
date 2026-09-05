use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "support/scenario_artifacts.rs"]
mod scenario_artifacts;
#[path = "support/scenario_process.rs"]
mod scenario_process;

use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::instruction::ScriptRecordValue;
use commander_blood_formats::lbm::RGB_COMPONENT_COUNT;
use commander_blood_game::native::bloodprg::{
    OriginalSaveGame, OriginalSaveSlotDirectory, PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT,
    ScriptFieldSelector, ScriptProfileId, original_save_state_block_byte_count,
    script_field_offset,
};
use commander_blood_game::runtime::{OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime};
use serde_json::Value;

const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";
const DISPLAY_ENVIRONMENT_VARIABLES: [&str; 2] = ["DISPLAY", "WAYLAND_DISPLAY"];
const SDL_AUDIO_DISK_OUTPUT_FILE_ENVIRONMENT_VARIABLE: &str = "SDL_AUDIO_DISK_OUTPUT_FILE";
const SDL_AUDIO_DISK_TIMESCALE_ENVIRONMENT_VARIABLE: &str = "SDL_AUDIO_DISK_TIMESCALE";
const SDL_AUDIO_DISK_TIMESCALE: &str = "10";
const SDL_AUDIO_F32_SAMPLE_BYTE_COUNT: usize = size_of::<f32>();
const INTRO_ESCAPE_KEY: &str = "key 1";
const OPENING_VIDEO: &str = "sq\\mind.HNM";
const FIRST_STARTUP_VIDEO: &str = "SQ\\cliptoot.hnm";
const PHONE_CLICK: &str = "click 125 118";
const GAME_CHOICE_CLICK: &str = "sclick 200 105";
const EXPLANATIONS_CHOICE_CLICK: &str = "sclick 200 94";
const CONTACTS_CLICK: &str = "click 230 124";
const BOB_CONTACT_CLICK: &str = "sclick 100 89";
const INITIAL_PROFILE: u64 = 0;
const POST_CALL_PROFILE: u64 = 1;
const ANSWER_HAND_SELECTOR: u64 = 4;
const CHOICE_HAND_SELECTOR: u64 = 7;
const NEUTRAL_HAND_SELECTOR: u64 = 1;
const PORTRAIT_RESOURCE: u64 = 7;
const PORTRAIT_POSITION: [u64; 2] = [16, 74];
const PORTRAIT_EXTENT: [u64; 2] = [104, 80];
const ACTIVE_ENTITY_FLAG: u64 = 1;
const UI_ENABLED_FLAG: u64 = 1;
const MODAL_UI_FLAG: u64 = 1 << 2;
const NAVIGATION_UI_FLAG: u64 = 1 << 3;
const DOS_ORACLE_PACKED_SECOND: u8 = 39;
const HONK_CLICK: &str = "click 230 88";
const SAVE_OPTION_CLICK: &str = "sclick 100 95";
const LOAD_OPTION_CLICK: &str = "sclick 100 106";
const SAVE_CANCEL_CLICK: &str = "sclick 100 151";
const MOVE_PREVIOUS_KEY: &str = "key 72";
const MOVE_NEXT_KEY: &str = "key 80";
const ACCEPT_KEY: &str = "key 28";
const TEXT_A_KEY: &str = "key 30 97";
const PAUSE_KEY: &str = "key 25 112";
const ASCII_CARRIAGE_RETURN: u64 = 13;
const ASCII_LOWERCASE_A: u64 = 97;
const ASCII_LOWERCASE_P: u64 = 112;
const LOAD_FIRST_SLOT_CLICK: &str = "sclick 100 40";
const PTERRA_TELEPORT: &str = "teleport Pterra";
const SHIP_DESTINATION_CLICK: &str = "click 216 72";
const PTERRA_TARGET_CLICK: &str = "click 80 88";
const PTERRA_LOCATION_VIDEO: &str = "PL\\pterra10.hnm";
const PTERRA_SCRUTER_APPROACH_VIDEO: &str = "PE\\scr02.hnm";
const PTERRA_SCRUTER_VIDEO: &str = "PE\\scr20.hnm";
const PTERRA_IDENTITY_CHOICES: [&str; 8] = [
    "robyx", "code", "ulikan", "69", "exxos", "electret", "666", "9",
];
const PTERRA_DOS_MARKER_LOW_COLOR_FNV: &str = "336b81698be4fca5";
const PTERRA_DOS_SETTLED_LOW_COLOR_FNV: &str = "602f7fd2f3730b6e";
const PTERRA_LOCATION_ENTRY_METRIC: u64 = 257;
const HEX_DIGITS_PER_BYTE: usize = 2;
const HEX_RADIX: u32 = 16;
const FNV1A64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;
const HYPERJUMP_LEVER_CLICK: &str = "click 250 125";
const HYPERJUMP_ACTOR_SLOT: usize = 5;
const NAV_ACTOR_ACTIVE_FLAG: u64 = 1;
const HYPERJUMP_IDLE_RESOURCE: u64 = 18;
const HYPERJUMP_VIDEO: &str = "SQ\\hyper_00.hnm";
const HYPERSPACE_ACTIVE_LINE: u64 = 6;
const HYPERJUMP_LEVER_POSITION: [i64; 2] = [250, 125];
const OPTIONS_CANCEL_CLICK: &str = "sclick 100 125";
const TEXT_OPTION_CLICK: &str = "sclick 100 68";
const SEEDED_LOAD_PROFILE: u8 = 2;
const SCRIPT2_PROFILE: u8 = 1;
const SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET: u16 = 0x12C2;
const SCRIPT2_PTERRA_UNLOCKED: u16 = 1;
const SCRIPT2_PTERRA_MARKER: [u64; 2] = [201, 93];
const PTERRA_NAME: &str = "Pterra";
const SCRUTER_JO_NAME: &str = "Scruter_Jo";
const SCRUTER_JO_CONTACT_CLICK: &str = "sclick 100 95";
const SCRUTER_JO_SOUND_BANK: &str = "scrut.snd";
const SCRUTER_JO_SPRITE: &str = "scruter.spr";
const SCRUTER_JO_IDLE_VIDEO: &str = "scr20.hnm";
const SCRUTER_JO_POST_OVERLAY_VIDEO: &str = "PE\\scr21.hnm";
const SCRUTER_JO_RECOVERY_VIDEO: &str = "PE\\scr22.hnm";
const SCRUTER_JO_FIRST_CONTACT_SUBTITLE: &str = "I've reprogrammed him";
const NEXT_ALIEN_OVERLAY_AFTER_AMER: &str = "Croolis";
const NEXT_ALIEN_OVERLAY_AFTER_SCRUT: &str = "Amer";
const COMPLETED_ALIEN_OVERLAY_COUNT: u64 = 1;
const FIRST_ALIEN_INVOCATION_SEQUENCE: u64 = 1;
const AUTHENTIC_ALIEN_OVERLAY_SEQUENCE: [(&str, &str); 3] = [
    ("Amer", "AMER.XDB"),
    ("Croolis", "CROOLIS.XDB"),
    ("Scrut", "SCRUT.XDB"),
];
const AUTHENTIC_GAME1_SAVE: &str = "accuracy/cblood_install/cblood/GAME1.SAV";
const HONK_WORD_CHOICES: [&str; 9] = [
    "bye_bye",
    "optimization",
    "consultation",
    "explanations",
    "calm_down",
    "play",
    "win",
    "lose",
    "help",
];
const IZWALITO_GREETING_WORDS: [&str; 11] = [
    "You", "found", "the", "right", "button", ".", "So", "far", "so", "good", "...",
];
const IZWALITO_CHOICE_PROMPT_WORDS: [&str; 7] =
    ["Click", "quick,", "Cap'n", "Bob", "is", "waiting", "..."];
const IZWALITO_EXPLANATIONS_FINAL_WORDS: [&str; 7] =
    ["Click", "quick", "on", "\"HONK\"", ".", "He", "has"];
const STREAMED_DIALOGUE_EVENT_KIND: &str = "streamed_dialogue";
const VOICE_REACTION_EVENT_KIND: &str = "voice_reaction";
const STREAM_MIXED_AUDIO_ROUTE: &str = "stream_mixed";
const STREAM_UNAVAILABLE_AUDIO_ROUTE: &str = "stream_unavailable";
const RADIO_RING_CLIP_INDEX: u64 = 6;
const RADIO_COMPLETION_CLIP_INDEX: u64 = 2;
const MINIMUM_REPEATED_RING_COUNT: usize = 2;
const RADIO_SOUND_BANK: &str = "radio.snd";
const RADIO_TERMINAL_FRAME: u64 = 11;
const IZWALITO_DESCRIPT_SOUND_BANK: &str = "izwal.snd";
const IZWALITO_SPRITE: &str = "izwalito.spr";
const IZWALITO_IDLE_VIDEO: &str = "aaisw.hnm";
const IZWALITO_FIRST_TALK_VIDEO: &str = "iswa1.hnm";
const IZWALITO_LAST_TALK_VIDEO: &str = "iswx.hnm";
const IZWALITO_TALK_CLIP_COUNT: usize = 15;
const BOB_NAME: &str = "Bob_Morlock";
const BOB_SOUND_BANK: &str = "bob.snd";
const BOB_SPRITE: &str = "bob.spr";
const BOB_THAW_VIDEO: &str = "sq\\cryogel.hnm";
const BOB_IDLE_VIDEO: &str = "PE\\aabob.hnm";
const BOB_FIRST_TALK_VIDEO: &str = "PE\\bobc.hnm";
const BOB_SECOND_TALK_VIDEO: &str = "PE\\bobd.hnm";
const BOB_FIRST_TALK_WORDS: [&str; 12] = [
    "HONK",
    "!",
    "You",
    "worthless",
    "heap",
    "of",
    "wires",
    "...",
    "Are",
    "you",
    "working",
    "?",
];
const BOB_SECOND_TALK_WORDS: [&str; 9] = [
    "What",
    "do",
    "you",
    "want",
    "to",
    "know",
    ",",
    "Commander",
    "?",
];
const BOB_TEXT_ONLY_CHATTER_PREFIX: &str = "Yes sir, Cap'n Bob";
const BOB_TEXT_ONLY_CHATTER_BYTES: &[u8] =
    b"Yes sir, Cap'n Bob sir!... Just \rgetting the multiplexers toned up... \r\r";
const BOB_GOODBYE_CLICK: &str = "sclick 225 58";
const BOB_POINTER_LEFT_MOVE: &str = "motion -2560 0";
const BOB_POINTER_RIGHT_MOVE: &str = "motion 1280 0";
const RESIDENT_LAST_CLIP_INDEX: u64 = 16;
const BOB_FIRST_CONTACT_PROMPT: [&str; 9] = [
    "What",
    "do",
    "you",
    "want",
    "to",
    "know",
    ",",
    "Commander",
    "?",
];
const BOB_FIRST_CONTACT_CHOICES: [&str; 8] = [
    "bye_bye",
    "black_hole",
    "Big_Bang",
    "Bob_Morlock",
    "Kanary",
    "mission",
    "Corpo",
    "Good_ol_Bob",
];
const EMPTY_LABELS: [&str; 0] = [];
const CONTACT_LABELS: [&str; 1] = ["Bob_Morlock"];
const OPTION_LABELS: [&str; 6] = ["TEXT", "MUSIC_OFF", "SAVE", "LOAD", "QUIT", "CANCEL"];
const NO_RECORDS: [u64; 0] = [];
const CONTACT_RECORDS: [u64; 1] = [3];

struct BridgeConsoleProbe {
    scenario: &'static str,
    trace_name: &'static str,
    click: &'static str,
    selected: Option<&'static str>,
    labels: &'static [&'static str],
    records: &'static [u64],
    panel_target_y: u64,
    presentation_target: Option<&'static str>,
}

const BRIDGE_CONSOLE_PROBES: [BridgeConsoleProbe; 4] = [
    BridgeConsoleProbe {
        scenario: "accuracy/scenarios/probe_console_navigation.tsv",
        trace_name: "console-navigation.jsonl",
        click: "click 230 106",
        selected: Some("navigation"),
        labels: &EMPTY_LABELS,
        records: &NO_RECORDS,
        panel_target_y: 98,
        presentation_target: None,
    },
    BridgeConsoleProbe {
        scenario: "accuracy/scenarios/probe_console_contacts.tsv",
        trace_name: "console-contacts.jsonl",
        click: "click 230 124",
        selected: Some("contacts"),
        labels: &CONTACT_LABELS,
        records: &CONTACT_RECORDS,
        panel_target_y: 116,
        presentation_target: None,
    },
    BridgeConsoleProbe {
        scenario: "accuracy/scenarios/probe_console_radio.tsv",
        trace_name: "console-radio.jsonl",
        click: "click 230 142",
        selected: None,
        labels: &EMPTY_LABELS,
        records: &NO_RECORDS,
        panel_target_y: 134,
        presentation_target: Some("menu"),
    },
    BridgeConsoleProbe {
        scenario: "accuracy/scenarios/probe_console_options.tsv",
        trace_name: "console-options.jsonl",
        click: "click 230 160",
        selected: Some("options"),
        labels: &OPTION_LABELS,
        records: &NO_RECORDS,
        panel_target_y: 152,
        presentation_target: None,
    },
];

#[test]
fn production_runtime_retains_the_intro_caption_until_its_blank_cue() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_intro_caption.tsv",
        "production-intro-caption.jsonl",
    ) else {
        return;
    };
    let clip_frames: Vec<_> = records
        .iter()
        .filter(|record| record["semantic"]["video"]["active_resource"] == FIRST_STARTUP_VIDEO)
        .collect();
    let title_frames: Vec<_> = clip_frames
        .iter()
        .copied()
        .filter(|record| {
            record["semantic"]["video"]["queue_metrics"]["sequence_index"]
                .as_u64()
                .is_some_and(|frame| (32..100).contains(&frame))
        })
        .collect();
    assert!(
        title_frames.len() >= 6,
        "not enough title checkpoints: {}",
        title_frames.len()
    );
    let caption = &title_frames[0]["semantic"]["sequence_caption"];
    assert!(caption["opaque_pixels"].as_u64().unwrap() > 0);
    for frame in title_frames {
        let semantic = &frame["semantic"];
        assert_eq!(
            &semantic["sequence_caption"], caption,
            "caption blinked or changed color"
        );
        assert_eq!(
            semantic["rgb_ui"]["rgba_hash"], caption["rgba_hash"],
            "the retained caption was not copied to the presented RGB UI"
        );
    }
    let blank = clip_frames
        .iter()
        .find(|record| {
            record["semantic"]["video"]["queue_metrics"]["sequence_index"]
                .as_u64()
                .is_some_and(|frame| frame >= 102)
        })
        .expect("CLIPTOOT never reached the authored blank cue");
    assert_eq!(blank["semantic"]["sequence_caption"]["opaque_pixels"], 0);
    let cancelled = records
        .iter()
        .rev()
        .find(|record| record["action"] == INTRO_ESCAPE_KEY)
        .expect("missing caption cancellation checkpoint");
    assert_eq!(
        cancelled["semantic"]["sequence_caption"]["opaque_pixels"],
        0
    );
}

#[test]
fn production_runtime_escape_cancels_the_opening_and_enters_script1() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_intro_escape.tsv",
        "production-intro-escape.jsonl",
    ) else {
        return;
    };

    let escape_index = records
        .iter()
        .position(|record| record["action"] == INTRO_ESCAPE_KEY)
        .expect("runtime trace omitted the authored Escape key");
    let opening = records[..escape_index]
        .iter()
        .rev()
        .find(|record| record["semantic"]["video"]["active_resource"] == OPENING_VIDEO)
        .expect("opening presentation never activated MIND.HNM before Escape");
    assert!(profile(opening).is_none());

    let cancelled = &records[escape_index];
    assert_eq!(profile(cancelled), Some(INITIAL_PROFILE));
    assert!(cancelled["semantic"]["video"]["active_resource"].is_null());
    assert!(!presentation_flag(cancelled, "active"));
    assert_ne!(
        presentation_u64(cancelled, "ui_flags") & UI_ENABLED_FLAG,
        u64::MIN,
        "Escape did not return input ownership to the recovered startup script"
    );

    let first_startup_presentation = records[escape_index + 1..]
        .iter()
        .find(|record| record["semantic"]["video"]["active_resource"] == FIRST_STARTUP_VIDEO)
        .expect("Escape did not advance SCRIPT1 into its first authored presentation");
    assert_eq!(profile(first_startup_presentation), Some(INITIAL_PROFILE));
    assert_eq!(
        presentation(first_startup_presentation)["screen_active"],
        true
    );
    assert_eq!(first_startup_presentation["liveness"], "progress");
}

#[test]
fn production_runtime_completes_the_authored_startup_phone_call() {
    let Some((records, sdl_audio_output)) = run_production_scenario_with_sdl_audio_capture(
        "accuracy/scenarios/startup_phone_complete.tsv",
        "startup-phone.jsonl",
    ) else {
        return;
    };
    let phone_index = records
        .iter()
        .position(|record| record["action"] == PHONE_CLICK)
        .expect("runtime trace omitted the authored phone click");
    let answer = &records[phone_index];
    assert_eq!(hand_selector(answer), ANSWER_HAND_SELECTOR);
    assert_ne!(
        presentation_u64(answer, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "answering the phone did not acquire the native console modal bit"
    );

    let before_answer = &records[phone_index - 1];
    assert_eq!(
        presentation(before_answer)["pending_presentation_owner"]["name"],
        "Izwalito"
    );
    assert!(presentation(before_answer)["active_actor_presentation"].is_null());
    let before_answer_audio = audio_events(before_answer);
    let ring_count = before_answer_audio
        .iter()
        .filter(|event| {
            event["kind"].as_str() == Some(VOICE_REACTION_EVENT_KIND)
                && event["index"].as_u64() == Some(RADIO_RING_CLIP_INDEX)
        })
        .count();
    assert!(
        ring_count >= MINIMUM_REPEATED_RING_COUNT,
        "C3 did not repeat radio clip 6 while the phone remained unanswered"
    );
    assert_eq!(
        audio_events(answer),
        before_answer_audio,
        "answering the orb emitted completion audio before its animation finished"
    );
    assert_eq!(
        presentation(answer)["pending_presentation_owner"]["name"],
        "Izwalito"
    );
    assert!(presentation(answer)["active_actor_presentation"].is_null());
    assert_eq!(presentation_u64(answer, "radio_slot/frame"), 4);
    assert_eq!(
        presentation_u64(answer, "radio_slot/terminal_frame"),
        RADIO_TERMINAL_FRAME
    );
    assert_eq!(presentation(answer)["radio_slot"]["ready"], true);
    assert_eq!(presentation(answer)["radio_slot"]["loaded"], true);
    assert_eq!(
        answer["semantic"]["audio"]["streamed_sound_bank_loads"],
        serde_json::json!([])
    );
    let after_answer = &records[phone_index + 1..];
    let active = after_answer
        .iter()
        .find(|record| {
            profile(record) == Some(INITIAL_PROFILE)
                && presentation_flag(record, "active")
                && presentation_flag(record, "defer")
        })
        .expect("phone answer never acquired Izwalito presentation ownership");
    assert!(presentation(active)["pending_presentation_owner"].is_null());
    assert_eq!(
        presentation(active)["active_actor_presentation"]["name"],
        "Izwalito"
    );
    assert_eq!(hand_selector(active), NEUTRAL_HAND_SELECTOR);
    assert_eq!(
        presentation_u64(active, "radio_slot/frame"),
        RADIO_TERMINAL_FRAME
    );
    assert_eq!(presentation(active)["radio_slot"]["ready"], false);
    assert_eq!(presentation(active)["radio_slot"]["loaded"], false);
    assert_izwalito_inset(active);
    assert_eq!(
        presentation(active)["inline_menu"]["words"],
        serde_json::json!(IZWALITO_GREETING_WORDS)
    );
    assert!(active["semantic"]["video"]["active_resource"].is_null());
    let active_descript = descript(active);
    assert_eq!(active_descript["active_object"]["name"], "Izwalito");
    assert_eq!(active_descript["application"]["record_kind"], "Character");
    assert_eq!(active_descript["character_sprite"], IZWALITO_SPRITE);
    assert_eq!(active_descript["sound_bank"], IZWALITO_DESCRIPT_SOUND_BANK);
    assert_eq!(active_descript["idle_clip"]["video"], IZWALITO_IDLE_VIDEO);
    assert_eq!(active_descript["idle_clip"]["loaded"], false);
    assert_eq!(
        active_descript["talk_clips"]
            .as_array()
            .expect("Izwalito DESCRIPT talk clips are not an array")
            .len(),
        IZWALITO_TALK_CLIP_COUNT
    );
    assert_eq!(
        active_descript["talk_clips"][usize::MIN]["video"],
        IZWALITO_FIRST_TALK_VIDEO
    );
    assert_eq!(
        active_descript["talk_clips"][IZWALITO_TALK_CLIP_COUNT - 1]["video"],
        IZWALITO_LAST_TALK_VIDEO
    );
    assert_eq!(active_descript["backgrounds"], serde_json::json!([]));
    assert_eq!(
        active["semantic"]["audio"]["streamed_sound_bank"],
        RADIO_SOUND_BANK
    );
    assert_eq!(
        active["semantic"]["audio"]["streamed_sound_bank_loads"],
        serde_json::json!([RADIO_SOUND_BANK])
    );
    let post_answer_audio = &audio_events(active)[before_answer_audio.len()..];
    assert_eq!(
        post_answer_audio.first().unwrap()["kind"],
        VOICE_REACTION_EVENT_KIND
    );
    assert_eq!(
        post_answer_audio.first().unwrap()["index"],
        RADIO_COMPLETION_CLIP_INDEX
    );
    assert_eq!(
        post_answer_audio.first().unwrap()["route"],
        STREAM_MIXED_AUDIO_ROUTE,
        "the radio completion boing was requested but not mixed into the live audio stream"
    );
    assert!(
        post_answer_audio.first().unwrap()["mixed_sample_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "the radio completion boing selected clip 2 but mixed zero audible samples"
    );
    assert!(
        post_answer_audio
            .iter()
            .all(|event| event["route"] != STREAM_UNAVAILABLE_AUDIO_ROUTE),
        "the startup phone call selected audio without an available playback route"
    );
    assert_eq!(
        post_answer_audio
            .iter()
            .filter(|event| {
                event["kind"].as_str() == Some(VOICE_REACTION_EVENT_KIND)
                    && event["index"].as_u64() == Some(RADIO_COMPLETION_CLIP_INDEX)
            })
            .count(),
        1,
        "radio actor completion clip did not fire exactly once"
    );
    assert_audible_sdl_callback_output(&sdl_audio_output);
    let greeting_audio = &post_answer_audio[1..];
    assert!(!greeting_audio.is_empty());
    assert!(
        greeting_audio
            .iter()
            .all(|event| { event["kind"].as_str() == Some(STREAMED_DIALOGUE_EVENT_KIND) })
    );

    let active_actor_hashes = after_answer
        .iter()
        .filter(|record| presentation_flag(record, "active"))
        .filter_map(bridge_actor_hash)
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        active_actor_hashes.len() >= 2,
        "Izwalito inset did not animate across distinct actor frames"
    );

    let waiting = after_answer
        .iter()
        .find(|record| {
            presentation_flag(record, "waiting_for_input")
                && presentation_flag(record, "word_choice_active")
        })
        .expect("startup call never reached its authored word-choice gate");
    assert_eq!(
        waiting["semantic"]["presentation"]["rendered_word_choices"],
        serde_json::json!(["explanations", "game"])
    );
    assert_eq!(
        presentation(waiting)["inline_menu"]["words"],
        serde_json::json!(IZWALITO_CHOICE_PROMPT_WORDS)
    );
    assert!(
        after_answer
            .iter()
            .filter(|record| presentation_flag(record, "active"))
            .all(|record| record["semantic"]["video"]["active_resource"].is_null())
    );

    let choice = after_answer
        .iter()
        .find(|record| record["action"] == GAME_CHOICE_CLICK)
        .expect("runtime trace omitted the authored GAME choice");
    assert_eq!(hand_selector(choice), CHOICE_HAND_SELECTOR);
    assert_ne!(
        presentation_u64(choice, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "closing the authored word choice released the modal bit before bridge cleanup"
    );

    let teardown = after_answer
        .iter()
        .find(|record| {
            profile(record) == Some(POST_CALL_PROFILE)
                && !presentation_flag(record, "active")
                && !presentation_flag(record, "defer")
        })
        .expect("startup call did not release ownership and enter SCRIPT2");
    assert_eq!(hand_selector(teardown), NEUTRAL_HAND_SELECTOR);
    assert_ne!(
        presentation_u64(teardown, "ui_flags") & UI_ENABLED_FLAG,
        u64::MIN
    );
    assert_eq!(
        presentation_u64(teardown, "portrait_entity/flags") & ACTIVE_ENTITY_FLAG,
        u64::MIN
    );
    assert_eq!(
        bridge_actor_hash(teardown),
        bridge_actor_hash(before_answer)
    );
}

#[test]
fn production_runtime_completes_izwalito_explanations_and_releases_the_phone() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_izwalito_explanations.tsv",
        "production-izwalito-explanations.jsonl",
    ) else {
        return;
    };

    let phone_index = records
        .iter()
        .position(|record| record["action"] == PHONE_CLICK)
        .expect("runtime trace omitted the authored phone click");
    let choice_index = records
        .iter()
        .position(|record| record["action"] == EXPLANATIONS_CHOICE_CLICK)
        .expect("runtime trace omitted the authored EXPLANATIONS click");
    let choice = &records[choice_index];
    assert_eq!(profile(choice), Some(INITIAL_PROFILE));
    assert_eq!(hand_selector(choice), CHOICE_HAND_SELECTOR);
    assert!(presentation_flag(choice, "active"));

    let explanations = &records[choice_index + 1..];
    let final_instruction = explanations
        .iter()
        .find(|record| {
            let words = presentation(record)["inline_menu"]["words"]
                .as_array()
                .expect("Izwalito inline menu words are not an array");
            words.starts_with(
                &IZWALITO_EXPLANATIONS_FINAL_WORDS
                    .iter()
                    .map(|word| serde_json::json!(word))
                    .collect::<Vec<_>>(),
            )
        })
        .expect("Izwalito's authored HONK instruction never appeared");
    assert_eq!(profile(final_instruction), Some(INITIAL_PROFILE));
    assert_eq!(
        presentation(final_instruction)["active_actor_presentation"]["name"],
        "Izwalito"
    );

    let teardown = explanations
        .iter()
        .find(|record| {
            profile(record) == Some(INITIAL_PROFILE)
                && !presentation_flag(record, "active")
                && presentation(record)["active_actor_presentation"].is_null()
        })
        .expect("Izwalito's EXPLANATIONS branch never released presentation ownership");
    assert_eq!(hand_selector(teardown), NEUTRAL_HAND_SELECTOR);
    assert_ne!(
        presentation_u64(teardown, "ui_flags") & UI_ENABLED_FLAG,
        u64::MIN
    );
    assert_eq!(
        presentation_u64(teardown, "portrait_entity/flags") & ACTIVE_ENTITY_FLAG,
        u64::MIN
    );
    assert_eq!(
        bridge_actor_hash(teardown),
        bridge_actor_hash(&records[phone_index - 1])
    );
}

#[test]
fn production_runtime_dispatches_honk_after_the_startup_phone_call() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/startup_phone_honk.tsv",
        "startup-phone-honk.jsonl",
    ) else {
        return;
    };

    let honk_index = records
        .iter()
        .position(|record| record["action"] == HONK_CLICK)
        .expect("runtime trace omitted the authored HONK click");
    let activated = &records[honk_index];
    assert_eq!(profile(activated), Some(POST_CALL_PROFILE));
    assert!(presentation_flag(activated, "active"));
    assert_ne!(
        presentation_u64(activated, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "HONK presentation did not retain native UI bit 2"
    );
    assert_eq!(
        presentation_u64(activated, "ui_flags") & NAVIGATION_UI_FLAG,
        u64::MIN,
        "bridge console left native seek bit 3 latched after dispatch"
    );

    let waiting = records[honk_index..]
        .iter()
        .find(|record| {
            presentation_flag(record, "waiting_for_input")
                && presentation_flag(record, "word_choice_active")
        })
        .expect("HONK presentation never reached its authored word-choice gate");
    assert_eq!(
        waiting["semantic"]["presentation"]["rendered_word_choices"],
        serde_json::json!(HONK_WORD_CHOICES)
    );
}

#[test]
fn production_runtime_reaches_bob_first_contact_with_complete_audio_and_media() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_bob_first_contact.tsv",
        "production-bob-first-contact.jsonl",
    ) else {
        return;
    };

    let contacts_index = records
        .iter()
        .position(|record| record["action"] == CONTACTS_CLICK)
        .expect("runtime trace omitted the authored CONTACTS click");
    let contacts = records[contacts_index..]
        .iter()
        .find(|record| console(record)["panel_phase"] == "interactive")
        .expect("CONTACTS never reached its interactive choice list");
    assert_eq!(
        console(contacts)["choice_labels"],
        serde_json::json!([BOB_NAME])
    );

    let bob_index = records
        .iter()
        .position(|record| record["action"] == BOB_CONTACT_CLICK)
        .expect("runtime trace omitted the authored Bob Morlock click");
    let thaw = &records[bob_index];
    assert_eq!(thaw["semantic"]["video"]["active_resource"], BOB_THAW_VIDEO);
    assert_eq!(descript(thaw)["active_object"]["name"], BOB_NAME);
    assert_eq!(descript(thaw)["sound_bank"], BOB_SOUND_BANK);
    assert_eq!(descript(thaw)["character_sprite"], BOB_SPRITE);
    assert_eq!(descript(thaw)["idle_clip"]["video"], "aabob.hnm");

    let bob_records = &records[bob_index + 1..];
    let first_talk = bob_records
        .iter()
        .find(|record| record["semantic"]["video"]["active_resource"] == BOB_FIRST_TALK_VIDEO)
        .expect("Bob's first authored talk clip never played");
    assert_eq!(
        first_talk["semantic"]["video"]["manu3_layer_allowed"], true,
        "Bob's embedded phone video incorrectly occluded the interactive MANU3 hand"
    );
    assert_eq!(
        presentation(first_talk)["active_actor_presentation"]["name"],
        BOB_NAME
    );
    assert!(presentation_flag(first_talk, "active"));
    assert!(
        first_talk["semantic"]["video"]["content_region"]["front"]["nonblack_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "Bob's decoded first-talk frame resolved entirely through black palette entries"
    );
    assert_eq!(
        presentation(first_talk)["inline_menu"]["words"],
        serde_json::json!(BOB_FIRST_TALK_WORDS),
        "Bob's bobc.hnm line lost or corrupted its authored SCRIPT2 dialogue words"
    );
    assert!(
        presentation(first_talk)["inline_menu"]["reveal_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "Bob's bobc.hnm line played without revealing any authored dialogue text"
    );
    assert_inline_menu_raster(first_talk, "Bob's bobc.hnm line");

    assert!(
        bob_records
            .iter()
            .any(|record| { record["semantic"]["video"]["active_resource"] == BOB_IDLE_VIDEO })
    );
    let second_talk = bob_records
        .iter()
        .find(|record| record["semantic"]["video"]["active_resource"] == BOB_SECOND_TALK_VIDEO)
        .expect("Bob's second authored talk clip never played");
    assert_eq!(
        second_talk["semantic"]["video"]["manu3_layer_allowed"], true,
        "Bob's embedded phone video incorrectly occluded the interactive MANU3 hand"
    );
    assert_eq!(
        presentation(second_talk)["inline_menu"]["words"],
        serde_json::json!(BOB_SECOND_TALK_WORDS),
        "Bob's bobd.hnm line lost or corrupted its authored SCRIPT2 dialogue words"
    );
    assert!(
        presentation(second_talk)["inline_menu"]["reveal_count"]
            .as_u64()
            .is_some_and(|count| count > 0),
        "Bob's bobd.hnm line played without revealing any authored dialogue text"
    );
    assert_inline_menu_raster(second_talk, "Bob's bobd.hnm line");

    let waiting = bob_records
        .iter()
        .find(|record| {
            presentation_flag(record, "waiting_for_input")
                && presentation_flag(record, "word_choice_active")
        })
        .expect("Bob's first contact never reached its authored topic chooser");
    assert_eq!(
        presentation(waiting)["rendered_word_choices"],
        serde_json::json!(BOB_FIRST_CONTACT_CHOICES)
    );
    assert_eq!(
        presentation(waiting)["inline_menu"]["words"],
        serde_json::json!(BOB_FIRST_CONTACT_PROMPT),
        "Bob's selector choices replaced his active authored dialogue"
    );
    assert_eq!(
        waiting["semantic"]["video"]["active_resource"],
        BOB_IDLE_VIDEO
    );
    assert_eq!(
        waiting["semantic"]["audio"]["streamed_sound_bank"],
        BOB_SOUND_BANK
    );
    assert!(bob_records.iter().any(|record| {
        audio_events(record).iter().any(|event| {
            event["kind"].as_str() == Some(VOICE_REACTION_EVENT_KIND)
                && event["index"].as_u64() == Some(RESIDENT_LAST_CLIP_INDEX)
        })
    }));

    let honk_chatter = bob_records
        .iter()
        .find(|record| {
            presentation_u64(record, "text_display_active") != u64::MIN
                && record["semantic"]["subtitle"]
                    .as_str()
                    .is_some_and(|subtitle| subtitle.starts_with(BOB_TEXT_ONLY_CHATTER_PREFIX))
        })
        .expect("Bob's opening Honk response produced chatter without its authored subtitle");
    assert_eq!(
        honk_chatter["semantic"]["video"]["active_resource"], BOB_IDLE_VIDEO,
        "Honk's text-only response did not return Bob to his authored idle clip"
    );
    assert_eq!(
        honk_chatter["semantic"]["subtitle_bytes"],
        serde_json::json!(BOB_TEXT_ONLY_CHATTER_BYTES),
        "Bob's SCRIPT2 chatter did not preserve the exact A6 dictionary bytes and native CR wrapping"
    );
    let subtitle_raster = &honk_chatter["semantic"]["subtitle_raster"];
    let expected_subtitle_pixels = subtitle_raster["expected_pixel_count"]
        .as_u64()
        .expect("Bob's Honk response omitted its recovered subtitle raster");
    assert!(
        expected_subtitle_pixels > 0,
        "Bob's Honk response produced no visible recovered subtitle glyph pixels"
    );
    assert_eq!(
        subtitle_raster["matching_pixel_count"].as_u64(),
        Some(expected_subtitle_pixels),
        "Bob's Honk response did not preserve the recovered subtitle glyphs in the live framebuffer"
    );

    let goodbye = bob_records
        .iter()
        .find(|record| record["action"] == BOB_GOODBYE_CLICK)
        .expect("runtime trace omitted Bob's authored goodbye choice");
    assert_eq!(
        goodbye["semantic"]["input"]["bridge_steering"]["before"][0],
        225
    );

    let goodbye_settled = bob_records
        .iter()
        .find(|record| record["action"] == BOB_POINTER_LEFT_MOVE)
        .expect("Bob's goodbye never returned control to the bridge pointer");
    assert!(!presentation_flag(goodbye_settled, "word_choice_active"));
    assert_eq!(
        presentation(goodbye_settled)["rendered_word_choices"],
        serde_json::json!([])
    );
    let left_steering = &goodbye_settled["semantic"]["input"]["bridge_steering"];
    let left_delta = left_steering["horizontal_delta"]
        .as_i64()
        .expect("left-move trace omitted its horizontal delta");
    assert!(
        left_delta < 0 && left_steering["view_changed"] == true,
        "Bob's goodbye did not release leftward bridge steering: {left_steering}; lock: {}; console: {}",
        goodbye_settled["semantic"]["input"]["pointer_lock"],
        goodbye_settled["semantic"]["bridge_console"],
    );
    let pointer_right = bob_records
        .iter()
        .find(|record| record["action"] == BOB_POINTER_RIGHT_MOVE)
        .expect("Bob's goodbye scenario omitted the rightward pointer move");
    let right_steering = &pointer_right["semantic"]["input"]["bridge_steering"];
    let right_delta = right_steering["horizontal_delta"]
        .as_i64()
        .expect("right-move trace omitted its horizontal delta");
    assert!(
        right_delta > 0 && right_steering["view_changed"] == true,
        "Bob's goodbye did not release rightward bridge steering: {right_steering}"
    );
}

#[test]
fn production_runtime_reaches_each_authored_bridge_console_handler() {
    for probe in &BRIDGE_CONSOLE_PROBES {
        let Some(records) = run_production_scenario(probe.scenario, probe.trace_name) else {
            return;
        };
        let click_index = records
            .iter()
            .position(|record| record["action"] == probe.click)
            .unwrap_or_else(|| panic!("runtime trace omitted {}", probe.click));
        let activated = &records[click_index];
        let settled = records
            .last()
            .expect("bridge console probe wrote no settled trace record");
        assert_eq!(profile(activated), Some(POST_CALL_PROFILE));
        assert_eq!(
            console_u64(activated, "panel_target_y"),
            probe.panel_target_y
        );

        if let Some(presentation_target) = probe.presentation_target {
            assert_eq!(console(activated)["selected"], Value::Null);
            assert!(presentation_flag(activated, "active"));
            assert!(presentation_flag(activated, "defer"));
            assert_eq!(
                presentation(activated)["active_actor_presentation"]["name"],
                presentation_target
            );
            assert_eq!(
                presentation(settled)["active_actor_presentation"]["name"],
                presentation_target
            );
            continue;
        }

        assert_eq!(
            console(activated)["selected"].as_str(),
            probe.selected,
            "{} activated the wrong top-level handler",
            probe.scenario
        );
        assert_eq!(console(activated)["panel_phase"], "transitioning");
        assert_eq!(console(settled)["selected"].as_str(), probe.selected);
        assert_eq!(console(settled)["panel_phase"], "interactive");
        assert_eq!(console(settled)["interface_active"], true);
        assert_eq!(console(settled)["interface_busy"], false);
        assert_eq!(
            console(settled)["choice_labels"],
            serde_json::json!(probe.labels)
        );
        assert_eq!(
            console(settled)["choice_records"],
            serde_json::json!(probe.records)
        );
        assert_eq!(
            presentation_u64(settled, "ui_flags") & NAVIGATION_UI_FLAG,
            u64::MIN
        );
    }
}

#[test]
fn production_runtime_opens_and_closes_the_authored_save_and_load_menus() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_save_menu.tsv",
        "production-save-menu.jsonl",
    ) else {
        return;
    };

    let save_index = records
        .iter()
        .position(|record| record["action"] == SAVE_OPTION_CLICK)
        .expect("runtime trace omitted the authored SAVE option click");
    let save_records = &records[save_index..];
    let active = save_records
        .iter()
        .find(|record| save_load(record)["save_requested"] == true)
        .expect("SAVE option never reached the production save/load owner");
    assert_eq!(save_load(active)["active"], true);
    assert_ne!(
        presentation_u64(active, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "save menu did not acquire the shared modal UI bit"
    );

    let interactive = save_records
        .iter()
        .find(|record| {
            save_load(record)["save_requested"] == true && save_load(record)["phase"] == "ready"
        })
        .expect("save menu never completed its recovered opening transition");
    assert_eq!(save_load(interactive)["selected_slot"], 0);
    assert_eq!(save_load(interactive)["active_slot"], 0);

    let move_next = save_records
        .iter()
        .find(|record| record["action"] == MOVE_NEXT_KEY)
        .expect("save menu never dispatched the recovered move-next key");
    assert_eq!(save_load(move_next)["selected_slot"], 1);
    assert_eq!(save_load(move_next)["active_slot"], 1);

    let move_previous = save_records
        .iter()
        .find(|record| record["action"] == MOVE_PREVIOUS_KEY)
        .expect("save menu never dispatched the recovered move-previous key");
    assert_eq!(save_load(move_previous)["selected_slot"], 0);
    assert_eq!(save_load(move_previous)["active_slot"], 0);

    let text = save_records
        .iter()
        .find(|record| record["action"] == TEXT_A_KEY)
        .expect("save menu never dispatched the recovered text-byte key");
    assert_eq!(input(text)["text_byte"], ASCII_LOWERCASE_A);

    let save_pause_key = save_records
        .iter()
        .find(|record| record["action"] == PAUSE_KEY)
        .expect("save menu never dispatched the recovered P key");
    assert_eq!(input(save_pause_key)["text_byte"], ASCII_LOWERCASE_P);
    assert_eq!(input(save_pause_key)["paused"], false);

    let cancel_index = records
        .iter()
        .position(|record| record["action"] == SAVE_CANCEL_CLICK)
        .expect("runtime trace omitted the authored save CANCEL click");
    let closed = records[cancel_index..]
        .iter()
        .find(|record| save_load(record)["active"] == false)
        .expect("save CANCEL row did not release the production save/load owner");
    assert_eq!(save_load(closed)["save_requested"], false);
    assert_eq!(save_load(closed)["load_requested"], false);
    assert_eq!(
        presentation_u64(closed, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "save CANCEL row left the shared modal UI bit latched"
    );

    let pause_records = records
        .iter()
        .filter(|record| record["action"] == PAUSE_KEY)
        .collect::<Vec<_>>();
    assert_eq!(pause_records.len(), 3);
    assert_eq!(input(pause_records[0])["paused"], false);
    assert_eq!(input(pause_records[1])["paused"], true);
    assert_eq!(input(pause_records[2])["paused"], false);
    assert_eq!(input(pause_records[1])["text_byte"], ASCII_LOWERCASE_P);
    assert_eq!(input(pause_records[2])["text_byte"], ASCII_LOWERCASE_P);

    let load_index = records
        .iter()
        .position(|record| record["action"] == LOAD_OPTION_CLICK)
        .expect("runtime trace omitted the authored LOAD option click");
    let load_records = &records[load_index..];
    let active = load_records
        .iter()
        .find(|record| save_load(record)["load_requested"] == true)
        .expect("LOAD option never reached the production save/load owner");
    assert_eq!(save_load(active)["active"], true);
    assert_ne!(
        presentation_u64(active, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "load menu did not acquire the shared modal UI bit"
    );

    let interactive = load_records
        .iter()
        .find(|record| {
            save_load(record)["load_requested"] == true && save_load(record)["phase"] == "ready"
        })
        .expect("load menu never completed its recovered opening transition");
    assert_eq!(save_load(interactive)["selected_slot"], 0);

    let accept = load_records
        .iter()
        .find(|record| record["action"] == ACCEPT_KEY)
        .expect("load menu never dispatched the recovered Enter key");
    assert_eq!(input(accept)["text_byte"], ASCII_CARRIAGE_RETURN);
    assert_eq!(save_load(accept)["active"], true);

    let closed = load_records
        .iter()
        .skip_while(|record| record["action"] != SAVE_CANCEL_CLICK)
        .find(|record| save_load(record)["active"] == false)
        .expect("load CANCEL row did not release the production save/load owner");
    assert_eq!(save_load(closed)["save_requested"], false);
    assert_eq!(save_load(closed)["load_requested"], false);
    assert_eq!(
        presentation_u64(closed, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "load CANCEL row left the shared modal UI bit latched"
    );
}

#[test]
fn production_runtime_cancels_options_and_text_speed_without_side_effects() {
    let Some(records) = run_production_scenario(
        "accuracy/scenarios/production_options_cancel.tsv",
        "production-options-cancel.jsonl",
    ) else {
        return;
    };

    let cancel_indices = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| (record["action"] == OPTIONS_CANCEL_CLICK).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(cancel_indices.len(), 2);

    let options_closed = records[cancel_indices[0]..]
        .iter()
        .find(|record| console(record)["selected"].is_null())
        .expect("Options CANCEL did not close the bridge submenu");
    assert_eq!(console(options_closed)["text_options_active"], false);
    assert_eq!(
        presentation_u64(options_closed, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "Options CANCEL left the modal UI bit latched"
    );

    let text_index = records
        .iter()
        .position(|record| record["action"] == TEXT_OPTION_CLICK)
        .expect("runtime trace omitted the authored TEXT option click");
    let text_open = records[text_index..]
        .iter()
        .find(|record| console(record)["text_options_active"] == true)
        .expect("TEXT option never opened the text-speed chooser");
    let dialogue_delay =
        text_open["semantic"]["presentation"]["text_state"]["dialogue_word_delay"].clone();

    let text_closed = records[cancel_indices[1]..]
        .iter()
        .find(|record| console(record)["text_options_active"] == false)
        .expect("Text Speed CANCEL did not close the chooser");
    assert_eq!(console(text_closed)["selected"], Value::Null);
    assert_eq!(
        presentation_u64(text_closed, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "Text Speed CANCEL left the modal UI bit latched"
    );
    assert_eq!(
        text_closed["semantic"]["presentation"]["text_state"]["dialogue_word_delay"],
        dialogue_delay,
        "Text Speed CANCEL changed dialogue timing"
    );
}

#[test]
fn production_runtime_restores_an_exact_seeded_save_through_the_authored_load_path() {
    let Some(records) = run_production_scenario_with_setup(
        "accuracy/scenarios/production_load_seeded_profile.tsv",
        "production-load-seeded-profile.jsonl",
        |asset_cache, writable_path| {
            seed_original_save(asset_cache, writable_path, SEEDED_LOAD_PROFILE)
        },
    ) else {
        return;
    };

    let load_index = records
        .iter()
        .position(|record| record["action"] == LOAD_OPTION_CLICK)
        .expect("runtime trace omitted the authored LOAD option click");
    let interactive = records[load_index..]
        .iter()
        .find(|record| {
            save_load(record)["load_requested"] == true && save_load(record)["phase"] == "ready"
        })
        .expect("seeded load menu never completed its opening transition");
    assert_eq!(save_load(interactive)["selected_slot"], 0);

    let slot_index = records
        .iter()
        .position(|record| record["action"] == LOAD_FIRST_SLOT_CLICK)
        .expect("runtime trace omitted the authored first save-slot click");
    let restored = records[slot_index..]
        .iter()
        .find(|record| profile(record) == Some(u64::from(SEEDED_LOAD_PROFILE)))
        .expect("seeded save never selected its encoded BloodScript profile");
    assert_eq!(save_load(restored)["active"], false);
    assert_eq!(save_load(restored)["save_requested"], false);
    assert_eq!(save_load(restored)["load_requested"], false);
    assert_eq!(
        presentation_u64(restored, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "successful load left the shared modal UI bit latched"
    );
}

#[test]
fn production_runtime_vm_unlocks_pterra_and_opens_the_authored_navigation_chart() {
    let Some(records) = run_production_scenario_with_setup(
        "accuracy/scenarios/production_load_pterra_navigation.tsv",
        "production-load-pterra-navigation.jsonl",
        seed_authentic_pterra_unlock_save,
    ) else {
        return;
    };

    let slot_index = records
        .iter()
        .position(|record| record["action"] == LOAD_FIRST_SLOT_CLICK)
        .expect("runtime trace omitted the authored first save-slot click");
    let restored = records[slot_index..]
        .iter()
        .find(|record| profile(record) == Some(u64::from(SCRIPT2_PROFILE)))
        .expect("authentic GAME1.SAV never restored SCRIPT2");
    assert_eq!(save_load(restored)["active"], false);

    let unlocked = records[slot_index..]
        .iter()
        .find(|record| record["semantic"]["script2"]["pterra_in_play"] == true)
        .expect("SCRIPT2 proc init never marked Pterra as known");
    assert_eq!(unlocked["semantic"]["script2"]["globals_a0"], 1);
    assert_eq!(unlocked["semantic"]["script2"]["init_enabled"], false);

    let chart = records[slot_index..]
        .iter()
        .find(|record| record["semantic"]["navigation"]["chart"]["active"] == true)
        .expect("the source-proven bridge navigation station never opened the chart");
    assert_ne!(
        chart["semantic"]["navigation"]["chart"]["chart_object_count"], 0,
        "the opened navigation chart retained no known destinations"
    );
    let pterra_record = unlocked["semantic"]["script2"]["pterra_record"]
        .as_u64()
        .expect("SCRIPT2 did not expose Pterra's typed object identity");
    let pterra_chart_object = chart["semantic"]["navigation"]["chart"]["objects"]
        .as_array()
        .expect("navigation chart objects are not an array")
        .iter()
        .find(|object| object["record"].as_u64() == Some(pterra_record))
        .expect("the opened navigation chart omitted the unlocked Pterra object");
    assert_eq!(pterra_chart_object["name"], PTERRA_NAME);
    assert_eq!(
        pterra_chart_object["marker"],
        serde_json::json!(SCRIPT2_PTERRA_MARKER)
    );

    let pterra_panel = records[slot_index..]
        .iter()
        .find(|record| {
            record["semantic"]["navigation"]["chart"]["selected_location"].as_u64()
                == Some(pterra_record)
        })
        .expect("clicking Pterra's recovered marker never opened its location panel");
    assert_eq!(
        pterra_panel["semantic"]["navigation"]["chart"]["location_panel_active"],
        true
    );

    let lever_click_index = records
        .iter()
        .position(|record| record["action"] == HYPERJUMP_LEVER_CLICK)
        .expect("runtime trace omitted the authored hyperjump-lever click");
    let lever_ready = records[..lever_click_index]
        .iter()
        .rev()
        .find(|record| {
            actor_slot(record, HYPERJUMP_ACTOR_SLOT)["flags"]
                .as_u64()
                .is_some_and(|flags| flags & NAV_ACTOR_ACTIVE_FLAG != u64::MIN)
        })
        .expect("Pterra's location panel never activated the authored hyperjump lever");
    let lever_slot = actor_slot(lever_ready, HYPERJUMP_ACTOR_SLOT);
    assert_eq!(lever_slot["resource"], HYPERJUMP_IDLE_RESOURCE);
    assert!(
        traced_hit_region_contains(&lever_slot["hit_region"], HYPERJUMP_LEVER_POSITION),
        "the scripted lever click is outside its executable-authored hit rectangle: {lever_slot}"
    );

    let travel_records = &records[lever_click_index..];
    let approach = travel_records
        .iter()
        .find(|record| record["semantic"]["navigation"]["camera"]["approach_active"] == true)
        .unwrap_or_else(|| {
            panic!(
                "pulling the hyperjump lever never started the recovered camera approach: {:?}",
                travel_records
                    .iter()
                    .map(|record| serde_json::json!({
                        "action": record["action"],
                        "input": record["semantic"]["input"],
                        "chart": record["semantic"]["navigation"]["chart"],
                        "camera": record["semantic"]["navigation"]["camera"],
                        "bridge_frame": record["semantic"]["presentation"]["bridge_frame"],
                        "bridge_mode": record["semantic"]["presentation"]["bridge_presentation_mode"],
                        "deferred": record["semantic"]["presentation"]["hyperjump_deferred_record_pending"],
                        "lever": actor_slot(record, HYPERJUMP_ACTOR_SLOT),
                    }))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        approach["semantic"]["presentation"]["hyperjump_deferred_record_pending"], false,
        "the C1 transfer left a duplicate native navigation link"
    );

    let hyperspace = travel_records
        .iter()
        .find(|record| record["semantic"]["video"]["active_resource"] == HYPERJUMP_VIDEO)
        .expect("the camera approach never selected the executable-authored hyperspace stream");
    assert_eq!(
        hyperspace["semantic"]["vm"]["active_line"],
        HYPERSPACE_ACTIVE_LINE
    );
    assert_eq!(
        hyperspace["semantic"]["video"]["manu3_layer_allowed"], false,
        "the independent hand layer was enabled over hyperspace"
    );

    let returned = travel_records
        .iter()
        .rev()
        .find(|record| {
            record["semantic"]["navigation"]["camera"]["approach_active"] == false
                && record["semantic"]["presentation"]["hyperjump_deferred_record_pending"] == false
        })
        .expect("the recovered camera approach never completed its return easing");
    assert_eq!(
        returned["semantic"]["navigation"]["camera"]["camera_view_active"],
        false
    );
    assert_eq!(returned["semantic"]["navigation"]["chart"]["active"], false);
    assert_eq!(
        presentation_u64(returned, "ui_flags") & MODAL_UI_FLAG,
        u64::MIN,
        "completed camera approach retained the native modal UI bit"
    );
}

#[test]
fn production_runtime_enters_pterra_ship_navigation_through_the_recovered_camera_path() {
    let Some(records) = run_production_scenario_with_setup(
        "accuracy/scenarios/production_load_pterra_ship_navigation.tsv",
        "production-load-pterra-ship-navigation.jsonl",
        seed_authentic_pterra_unlock_save,
    ) else {
        return;
    };

    let slot_index = records
        .iter()
        .position(|record| record["action"] == LOAD_FIRST_SLOT_CLICK)
        .expect("runtime trace omitted the authored first save-slot click");
    let unlocked = records[slot_index..]
        .iter()
        .find(|record| record["semantic"]["script2"]["pterra_in_play"] == true)
        .expect("SCRIPT2 proc init never marked Pterra as known");
    let pterra_record = unlocked["semantic"]["script2"]["pterra_record"]
        .as_u64()
        .expect("SCRIPT2 did not expose Pterra's typed object identity");
    let teleport_index = records
        .iter()
        .position(|record| record["action"] == PTERRA_TELEPORT)
        .expect("runtime trace omitted the typed Pterra teleport");
    assert_eq!(
        records[teleport_index]["semantic"]["navigation"]["camera"]["target"]["record"],
        pterra_record,
        "the typed teleport did not publish Pterra as Arche's navigation target"
    );
    let destination_ready = records[teleport_index..]
        .iter()
        .find(|record| {
            record["semantic"]["navigation"]["camera"]["status_region"]["active"] == true
                && record["semantic"]["navigation"]["camera"]["camera_view_active"] == false
        })
        .expect("Pterra's recovered ship-view artwork never exposed its destination region");
    assert_ne!(
        destination_ready["semantic"]["navigation"]["camera"]["status_region"]["extent"],
        serde_json::json!([0, 0]),
        "Pterra's recovered destination region has no extent"
    );

    let selected_index = records
        .iter()
        .position(|record| record["action"] == SHIP_DESTINATION_CLICK)
        .expect("runtime trace omitted the ship destination-region click");
    let ship_records = &records[selected_index..];
    let ship_hud = ship_records
        .iter()
        .find(|record| {
            record["semantic"]["navigation"]["ship_mode"] == "Active"
                && record["semantic"]["navigation"]["ship_hud"]["coordinator"]["initialized"]
                    == true
        })
        .unwrap_or_else(|| {
            panic!(
                "the destination region never entered the authored ship HUD: {:?}",
                records[teleport_index..]
                    .iter()
                    .map(|record| {
                        serde_json::json!({
                            "action": record["action"],
                            "input": record["semantic"]["input"],
                            "bridge_frame": record["semantic"]["presentation"]["bridge_frame"],
                            "navigation": record["semantic"]["navigation"],
                        })
                    })
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        ship_hud["semantic"]["navigation"]["ship_hud"]["coordinator"]["presentable_targets"]
            .as_array()
            .expect("ship HUD presentable targets are not an array")
            .iter()
            .any(|record| record.as_u64() == Some(pterra_record)),
        "the production ship HUD omitted Pterra from its recovered target list"
    );
    assert_ne!(presentation_u64(ship_hud, "ship_flags"), u64::MIN);

    let selector = ship_records
        .iter()
        .filter_map(|record| {
            record["semantic"]["navigation"]["ship_target_selector"]
                .as_object()
                .map(|_| &record["semantic"]["navigation"]["ship_target_selector"])
        })
        .find(|selector| {
            selector["rows"]
                .as_array()
                .is_some_and(|rows| !rows.is_empty())
        })
        .expect("the Pterra ship HUD never rendered its target selector rows");
    let rows = selector["rows"]
        .as_array()
        .expect("ship target selector rows are not an array");
    let mut previous_default_color = Value::Null;
    let default_color_timeline = records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            let color = record["semantic"]["video"]["console_palette"][8].clone();
            if color == previous_default_color {
                return None;
            }
            previous_default_color = color.clone();
            Some(serde_json::json!({
                "index": index,
                "action": record["action"],
                "color": color,
                "active_resource": record["semantic"]["video"]["active_resource"],
                "navigation": record["semantic"]["navigation"],
            }))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        3,
        "the Pterra target selector must contain PTERRA, ARK, and CANCEL"
    );
    assert!(
        rows.iter().any(|row| row["kind"] == "Item(1)"),
        "the Pterra target selector omitted ARK"
    );
    assert!(
        rows.iter().any(|row| row["kind"] == "Cancel"),
        "the Pterra target selector omitted CANCEL"
    );
    for row in rows {
        assert_ne!(
            row["rgb"],
            serde_json::json!([0, 0, 0]),
            "the Pterra target selector mapped {:?} to black: {selector}; palette timeline: {default_color_timeline:?}",
            row["kind"]
        );
        assert_ne!(
            row["matching_rect_pixels"].as_u64().unwrap_or_default(),
            u64::MIN,
            "the Pterra target selector did not rasterize {:?}: {selector}",
            row["kind"]
        );
    }

    let pterra_click_index = records
        .iter()
        .position(|record| record["action"] == PTERRA_TARGET_CLICK)
        .expect("runtime trace omitted the rendered Pterra target click");
    let pterra_queued = records[pterra_click_index..]
        .iter()
        .find(|record| record["semantic"]["navigation"]["target"]["name"] == PTERRA_NAME)
        .expect("clicking Pterra never published the authored ship navigation target");
    assert_eq!(
        pterra_queued["semantic"]["navigation"]["ship_mode"],
        "Active"
    );

    let pterra_video_records = records[pterra_click_index..]
        .iter()
        .filter(|record| {
            matches!(
                record["semantic"]["video"]["active_resource"].as_str(),
                Some(PTERRA_LOCATION_VIDEO | PTERRA_SCRUTER_APPROACH_VIDEO | PTERRA_SCRUTER_VIDEO)
            )
        })
        .collect::<Vec<_>>();
    assert!(
        !pterra_video_records.is_empty(),
        "Pterra selection never entered either authored planet-video stream"
    );
    for resource in [
        PTERRA_LOCATION_VIDEO,
        PTERRA_SCRUTER_APPROACH_VIDEO,
        PTERRA_SCRUTER_VIDEO,
    ] {
        let resource_records = pterra_video_records
            .iter()
            .filter(|record| {
                record["semantic"]["video"]["active_resource"].as_str() == Some(resource)
            })
            .collect::<Vec<_>>();
        assert!(
            !resource_records.is_empty(),
            "Pterra travel never presented authored stream {resource}"
        );
        let flat_game_color_hash = resource_records[0]["semantic"]["video"]["palette_hash"]
            .as_str()
            .expect("an active Pterra stream omitted the flat game color hash");
        for record in resource_records {
            assert_eq!(
                record["semantic"]["video"]["palette_hash"], flat_game_color_hash,
                "HNM-local color records escaped into flat game colors during {resource}: {record}"
            );
        }
    }
    let low_color_byte_count = PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT * RGB_COMPONENT_COUNT;
    let pterra_oracle_frame = pterra_video_records
        .iter()
        .find(|record| {
            if record["semantic"]["video"]["active_resource"].as_str()
                != Some(PTERRA_LOCATION_VIDEO)
                || record["semantic"]["video"]["queue_metrics"]["entry_metric"].as_u64()
                    != Some(PTERRA_LOCATION_ENTRY_METRIC)
            {
                return false;
            }
            let Some(encoded) = record["semantic"]["video"]["display_color_bytes"].as_str() else {
                return false;
            };
            let colors = decode_hexadecimal_bytes(encoded);
            matches!(
                fnv1a64(&colors[..low_color_byte_count]).as_str(),
                PTERRA_DOS_MARKER_LOW_COLOR_FNV | PTERRA_DOS_SETTLED_LOW_COLOR_FNV
            )
        })
        .expect("Pterra travel omitted both captured DOS low-color states");
    assert_eq!(
        pterra_oracle_frame["semantic"]["video"]["manu3_layer_allowed"], false,
        "the independent MANU3 layer covered a DOS-matched Pterra frame"
    );

    let retained_bridge_palette = pterra_queued["semantic"]["video"]["bridge_palette_hash"]
        .as_str()
        .expect("Pterra selection did not expose the retained bridge palette hash");
    for record in &pterra_video_records {
        assert_eq!(
            record["semantic"]["video"]["display_frame_owned"], true,
            "an active Pterra stream did not own its displayed frame: {record}"
        );
        assert_eq!(
            record["semantic"]["video"]["manu3_layer_allowed"], false,
            "the independent wgpu MANU3 layer was enabled over a Pterra video: {record}"
        );
        assert_eq!(
            record["semantic"]["video"]["manu3_submitted_triangle_count"], 0,
            "the renderer submitted MANU3 geometry over a Pterra video: {record}"
        );
        assert_eq!(
            record["semantic"]["video"]["palette_transition"]["surface"], "PresentationFrame",
            "a Pterra fade targeted the game surface instead of its true-color video page: {record}"
        );
        assert_eq!(
            record["semantic"]["video"]["bridge_palette_hash"], retained_bridge_palette,
            "an HNM-local palette escaped into the retained RGBA bridge surface: {record}"
        );
        assert!(
            record["semantic"]["video"]["display_rgba_hash"].is_string(),
            "an active Pterra HNM page was retained as indexed palette state instead of true-color RGBA: {record}"
        );
    }
    for record in &records[pterra_click_index..] {
        assert_eq!(
            record["semantic"]["video"]["bridge_palette_hash"], retained_bridge_palette,
            "Pterra playback changed the retained RGBA bridge palette after a stream boundary: {record}"
        );
    }
    // SCRIPT2 proc pter waits for this choice; DESCRIPT assigns scr20 as
    // Scruter Jo's idle stream. The C lifecycle restarts line 8 while waiting,
    // so a closed video source is not a valid completion condition here.
    let identity_choices = records[pterra_click_index..]
        .iter()
        .filter(|record| {
            record["semantic"]["presentation"]["waiting_for_input"] == true
                && record["semantic"]["presentation"]["rendered_word_choices"]
                    == serde_json::json!(PTERRA_IDENTITY_CHOICES)
        })
        .collect::<Vec<_>>();
    assert!(
        identity_choices.len() >= 2,
        "Pterra never reached a stable identity-code choice"
    );
    let post_video_choice = identity_choices.last().unwrap();
    assert_eq!(
        post_video_choice["semantic"]["video"]["active_resource"], PTERRA_SCRUTER_VIDEO,
        "the identity choice did not play Scruter Jo's authored idle video"
    );
    assert_eq!(
        post_video_choice["semantic"]["presentation"]["inline_menu"]["revealed_words"],
        serde_json::json!(["You", "give", "identity", "code", ":"]),
    );
    assert_inline_menu_raster(post_video_choice, "Pterra identity-code prompt");
    let idle_frame_hashes = identity_choices
        .iter()
        .map(|record| {
            record["semantic"]["video"]["display_rgba_hash"]
                .as_str()
                .expect("the identity-code screen omitted its true-color image hash")
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        idle_frame_hashes.len() > 1,
        "Scruter Jo's idle video froze during the choice"
    );
    assert_eq!(
        post_video_choice["semantic"]["video"]["palette_transition"]["surface"],
        "PresentationFrame",
        "the post-video Pterra page redirected its fade into game colors"
    );
    assert_eq!(
        post_video_choice["semantic"]["video"]["manu3_layer_allowed"], false,
        "the post-video Pterra choice page exposed the independent hand layer"
    );
    assert_eq!(
        post_video_choice["semantic"]["video"]["manu3_submitted_triangle_count"], 0,
        "the renderer submitted MANU3 geometry over the post-video Pterra choice page"
    );
    assert!(
        post_video_choice["semantic"]["video"]["display_rgba_hash"].is_string(),
        "the post-video Pterra choice page lost its retained true-color surface"
    );
    assert!(
        records[pterra_click_index..].iter().any(|record| {
            record["semantic"]["presentation"]["active_actor_presentation"]["name"]
                == SCRUTER_JO_NAME
        }),
        "the shared C4 deferred record never entered the authored Scruter Jo travel sequence"
    );
}

#[test]
fn production_runtime_runs_scruter_jo_alien_overlay_and_restores_the_bridge() {
    let Some(records) = run_production_scenario_with_setup(
        "accuracy/scenarios/production_load_script2_contacts.tsv",
        "production-load-script2-contacts.jsonl",
        seed_script2_scruter_jo_aboard_save,
    ) else {
        return;
    };

    let contact_click = records
        .iter()
        .position(|record| record["action"] == CONTACTS_CLICK)
        .expect("runtime trace omitted the authored CONTACTS click");
    let contacts = records[contact_click..]
        .iter()
        .find(|record| record["semantic"]["bridge_console"]["selected"] == "contacts")
        .expect("SCRIPT2 CONTACTS never opened");
    let labels = contacts["semantic"]["bridge_console"]["choice_labels"]
        .as_array()
        .expect("SCRIPT2 contact labels are not an array");
    assert_eq!(
        labels,
        serde_json::json!([BOB_NAME, SCRUTER_JO_NAME])
            .as_array()
            .expect("literal contact labels are an array"),
        "typed aboard state did not produce the authored SCRIPT2 contact order"
    );

    let scruter_index = records
        .iter()
        .position(|record| record["action"] == SCRUTER_JO_CONTACT_CLICK)
        .expect("runtime trace omitted the authored Scruter Jo click");
    let scruter_records = &records[scruter_index..];
    let scruter_owner = scruter_records
        .iter()
        .find(|record| descript(record)["active_object"]["name"] == SCRUTER_JO_NAME)
        .expect("Scruter Jo never acquired production DESCRIPT ownership");
    assert_eq!(descript(scruter_owner)["sound_bank"], SCRUTER_JO_SOUND_BANK);
    assert_eq!(
        descript(scruter_owner)["character_sprite"],
        SCRUTER_JO_SPRITE
    );
    assert_eq!(
        descript(scruter_owner)["idle_clip"]["video"],
        SCRUTER_JO_IDLE_VIDEO
    );

    assert!(scruter_records.iter().any(|record| {
        record["semantic"]["subtitle"]
            .as_str()
            .is_some_and(|subtitle| subtitle.contains(SCRUTER_JO_FIRST_CONTACT_SUBTITLE))
    }));

    let overlay_return = scruter_records
        .iter()
        .find(|record| {
            record["semantic"]["alien_overlay"]["next_overlay"] == NEXT_ALIEN_OVERLAY_AFTER_AMER
        })
        .expect("the production AMER overlay never completed its round-robin handoff");
    assert_eq!(overlay_return["semantic"]["alien_overlay"]["armed"], false);
    assert_eq!(
        overlay_return["semantic"]["alien_overlay"]["trigger_pending"],
        false
    );
    assert_eq!(
        overlay_return["semantic"]["alien_overlay"]["palette_dirty"],
        true
    );
    assert_eq!(
        overlay_return["semantic"]["alien_overlay"]["plane_band_enabled"],
        true
    );

    assert!(scruter_records.iter().any(|record| {
        record["semantic"]["video"]["active_resource"] == SCRUTER_JO_POST_OVERLAY_VIDEO
    }));
    assert!(scruter_records.iter().any(|record| {
        record["semantic"]["video"]["active_resource"] == SCRUTER_JO_RECOVERY_VIDEO
    }));
    assert_eq!(
        overlay_return["semantic"]["audio"]["streamed_sound_bank"],
        SCRUTER_JO_SOUND_BANK
    );

    let completed_round_robin = scruter_records
        .iter()
        .find(|record| {
            let counts = &record["semantic"]["alien_overlay"]["completed_overlays"];
            ["Amer", "Croolis", "Scrut"]
                .iter()
                .all(|overlay| counts[*overlay].as_u64() == Some(COMPLETED_ALIEN_OVERLAY_COUNT))
        })
        .expect("the production process did not complete the three-XDB round robin");
    assert_eq!(
        completed_round_robin["semantic"]["alien_overlay"]["next_overlay"],
        NEXT_ALIEN_OVERLAY_AFTER_SCRUT
    );
    let invocations = completed_round_robin["semantic"]["alien_overlay"]["invocations"]
        .as_array()
        .expect("the completed alien round robin omitted per-XDB invocation evidence");
    assert_eq!(
        invocations.len(),
        AUTHENTIC_ALIEN_OVERLAY_SEQUENCE.len(),
        "the production process did not retain one live invocation for each shipped XDB"
    );
    for (index, (invocation, (expected_overlay, expected_resource))) in invocations
        .iter()
        .zip(AUTHENTIC_ALIEN_OVERLAY_SEQUENCE)
        .enumerate()
    {
        assert_eq!(invocation["overlay"], expected_overlay);
        assert_eq!(invocation["resource"], expected_resource);
        assert_eq!(
            invocation["sequence"].as_u64(),
            Some(FIRST_ALIEN_INVOCATION_SEQUENCE + index as u64),
            "the production XDB round robin ran out of recovered order: {invocations:?}"
        );
        let input_frames = invocation["input_frames"]
            .as_u64()
            .expect("a live alien invocation omitted its input frame count");
        let presented_frames = invocation["presented_frames"]
            .as_u64()
            .expect("a live alien invocation omitted its presented frame count");
        let paced_frames = invocation["paced_frames"]
            .as_u64()
            .expect("a live alien invocation omitted its pacing frame count");
        assert_ne!(
            input_frames,
            u64::MIN,
            "{expected_overlay} consumed no input"
        );
        assert_eq!(
            presented_frames, input_frames,
            "{expected_overlay} dropped a recovered XDB frame"
        );
        assert_eq!(
            paced_frames, input_frames,
            "{expected_overlay} bypassed recovered frame pacing"
        );
        invocation["sound_callbacks"]
            .as_u64()
            .unwrap_or_else(|| panic!("{expected_overlay} omitted its sound callback count"));
        assert_eq!(invocation["resources_released"], true);
        assert_eq!(invocation["coordinator_restored"], true);
        let restoration = &invocation["restoration"];
        for required_call in [
            "alien_sound_bank_loaded",
            "cd_audio_started",
            "cd_audio_stopped",
            "bridge_sound_bank_loaded",
            "sound_header_restored",
            "manu3_reloaded",
            "transition_row_cleared",
        ] {
            assert_eq!(
                restoration[required_call], true,
                "{expected_overlay} skipped mandatory coordinator call {required_call}"
            );
        }
        match invocation["graphics_tail"].as_str() {
            Some("Sequence") => {
                assert_eq!(restoration["sequence_back_buffer_restored"], true);
            }
            Some("SceneImage") => {
                assert_eq!(restoration["bridge_back_buffer_initialized"], true);
                assert_eq!(restoration["scene_image_reloaded"], true);
            }
            tail => panic!("{expected_overlay} returned through unknown graphics tail {tail:?}"),
        }
    }

    for completed_count in 1..=AUTHENTIC_ALIEN_OVERLAY_SEQUENCE.len() {
        let restored = scruter_records
            .iter()
            .find(|record| {
                record["semantic"]["alien_overlay"]["invocations"]
                    .as_array()
                    .is_some_and(|invocations| invocations.len() == completed_count)
            })
            .unwrap_or_else(|| {
                panic!("no trace record followed alien invocation {completed_count}")
            });
        assert_eq!(restored["semantic"]["alien_overlay"]["armed"], false);
        assert_eq!(
            restored["semantic"]["alien_overlay"]["trigger_pending"],
            false
        );
        assert_eq!(restored["semantic"]["alien_overlay"]["palette_dirty"], true);
        assert_eq!(
            restored["semantic"]["alien_overlay"]["plane_band_enabled"],
            true
        );
        assert_eq!(
            restored["semantic"]["audio"]["streamed_sound_bank"], SCRUTER_JO_SOUND_BANK,
            "alien invocation {completed_count} did not restore Scruter Jo's audio owner"
        );
    }
}

fn run_production_scenario(scenario: &str, trace_name: &str) -> Option<Vec<Value>> {
    run_production_scenario_with_setup(scenario, trace_name, |_, _| Ok(()))
}

fn run_production_scenario_with_sdl_audio_capture(
    scenario: &str,
    trace_name: &str,
) -> Option<(Vec<Value>, Box<[u8]>)> {
    run_production_scenario_internal(scenario, trace_name, |_, _| Ok(()), true).map(|output| {
        (
            output.records,
            output
                .sdl_audio_output
                .expect("SDL audio capture run omitted its disk output"),
        )
    })
}

fn run_production_scenario_with_setup(
    scenario: &str,
    trace_name: &str,
    setup: impl FnOnce(&Path, &Path) -> anyhow::Result<()>,
) -> Option<Vec<Value>> {
    run_production_scenario_internal(scenario, trace_name, setup, false)
        .map(|output| output.records)
}

struct ProductionScenarioOutput {
    records: Vec<Value>,
    sdl_audio_output: Option<Box<[u8]>>,
}

fn run_production_scenario_internal(
    scenario: &str,
    trace_name: &str,
    setup: impl FnOnce(&Path, &Path) -> anyhow::Result<()>,
    capture_sdl_audio: bool,
) -> Option<ProductionScenarioOutput> {
    let asset_cache = configured_runtime_asset_cache()?;
    if !DISPLAY_ENVIRONMENT_VARIABLES
        .iter()
        .any(|variable| std::env::var_os(variable).is_some())
    {
        assert!(
            !accuracy_tests_are_required(),
            "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires DISPLAY or WAYLAND_DISPLAY"
        );
        return None;
    }

    let root = workspace_root();
    let temporary = scenario_artifacts::ScenarioArtifacts::create(&root, trace_name).unwrap();
    let trace_path = temporary.0.join(trace_name);
    let sdl_audio_output_path = temporary.0.join("sdl-audio.raw");
    let writable_path = temporary.0.join("writable");
    setup(&asset_cache, &writable_path).unwrap();
    let scenario_path = root.join(scenario);
    let mut command = Command::new(env!("CARGO_BIN_EXE_commander-blood"));
    command
        .arg("--write-data")
        .arg(&writable_path)
        .arg("--scenario")
        .arg(&scenario_path)
        .arg("--trace")
        .arg(&trace_path)
        .arg("--oracle-packed-second")
        .arg(DOS_ORACLE_PACKED_SECOND.to_string())
        .env(ASSET_CACHE_ENVIRONMENT_VARIABLE, &asset_cache);
    if capture_sdl_audio {
        command
            .env("SDL_AUDIODRIVER", "disk")
            .env(
                SDL_AUDIO_DISK_OUTPUT_FILE_ENVIRONMENT_VARIABLE,
                &sdl_audio_output_path,
            )
            .env(
                SDL_AUDIO_DISK_TIMESCALE_ENVIRONMENT_VARIABLE,
                SDL_AUDIO_DISK_TIMESCALE,
            );
    } else {
        command.env("SDL_AUDIODRIVER", "dummy");
    }
    let timeout = scenario_artifacts::timeout().unwrap();
    temporary
        .record_inputs(&command, &scenario_path, &asset_cache, &writable_path, timeout)
        .unwrap();
    let output = scenario_process::run(&mut command, &temporary.0, timeout).unwrap();
    std::fs::write(
        temporary.0.join("process-result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "status": output.status.to_string(),
            "timed_out": output.timed_out,
        })).unwrap(),
    ).unwrap();
    assert!(
        output.status.success() && !output.timed_out,
        "production scenario {} failed ({}, timed out: {}); artifacts: {}",
        scenario_path.display(),
        output.status,
        output.timed_out,
        temporary.0.display(),
    );
    let sdl_audio_output = capture_sdl_audio.then(|| {
        std::fs::read(&sdl_audio_output_path).unwrap_or_else(|error| {
            panic!(
                "reading SDL callback output {}: {error}",
                sdl_audio_output_path.display()
            )
        })
    });
    Some(ProductionScenarioOutput {
        records: load_trace(&trace_path),
        sdl_audio_output: sdl_audio_output.map(Vec::into_boxed_slice),
    })
}

fn assert_audible_sdl_callback_output(encoded: &[u8]) {
    assert!(!encoded.is_empty(), "SDL callback wrote no process audio");
    let samples = encoded.chunks_exact(SDL_AUDIO_F32_SAMPLE_BYTE_COUNT);
    assert!(
        samples.remainder().is_empty(),
        "SDL callback output is not aligned to f32 samples"
    );
    let mut audible_sample_count = usize::MIN;
    for encoded_sample in samples {
        let sample = f32::from_le_bytes(encoded_sample.try_into().unwrap());
        assert!(
            sample.is_finite(),
            "SDL callback emitted a non-finite sample"
        );
        assert!(
            (-1.0..=1.0).contains(&sample),
            "SDL callback sample {sample} exceeds normalized f32 range"
        );
        audible_sample_count += usize::from(sample != 0.0);
    }
    assert_ne!(
        audible_sample_count,
        usize::MIN,
        "the phone process selected and mixed TB.SND clip 2, but SDL received only silence"
    );
}

fn seed_original_save(asset_cache: &Path, writable_path: &Path, profile: u8) -> anyhow::Result<()> {
    std::fs::create_dir_all(writable_path)?;
    let paths = OriginalGameDataPaths::from_root(asset_cache)?;
    let data = OriginalGameData::load_with_writable_root(paths, writable_path)?;
    let directory_name = BloodResourceName::new(b"BLOOD.SAV")?;
    let directory =
        OriginalSaveSlotDirectory::decode(&data.resource_store().load(&directory_name)?)?;
    std::fs::write(writable_path.join("BLOOD.SAV"), directory.encode())?;

    let mut runtime = OriginalGameRuntime::new(data);
    let profile = ScriptProfileId::new(profile)
        .ok_or_else(|| anyhow::anyhow!("invalid seeded BloodScript profile {profile}"))?;
    runtime.load_profile(profile)?;
    let save = OriginalSaveGame::capture(
        runtime
            .current_profile()
            .expect("seeded profile load retained no profile"),
    )?;
    std::fs::write(writable_path.join("GAME1.SAV"), save.encode())?;
    Ok(())
}

fn seed_authentic_pterra_unlock_save(
    asset_cache: &Path,
    writable_path: &Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(writable_path)?;
    let paths = OriginalGameDataPaths::from_root(asset_cache)?;
    let data = OriginalGameData::load_with_writable_root(paths, writable_path)?;
    let directory_name = BloodResourceName::new(b"BLOOD.SAV")?;
    let directory =
        OriginalSaveSlotDirectory::decode(&data.resource_store().load(&directory_name)?)?;
    std::fs::write(writable_path.join("BLOOD.SAV"), directory.encode())?;

    let mut runtime = OriginalGameRuntime::new(data);
    let script2 = ScriptProfileId::new(SCRIPT2_PROFILE).expect("SCRIPT2 profile is valid");
    runtime.load_profile(script2)?;
    let state_block_byte_count = original_save_state_block_byte_count(
        runtime
            .current_profile()
            .expect("SCRIPT2 load retained no profile"),
    )?;
    let authentic_save = std::fs::read(workspace_root().join(AUTHENTIC_GAME1_SAVE))?;
    let authentic_save = OriginalSaveGame::decode(&authentic_save, state_block_byte_count)?;
    authentic_save.restore_into(
        runtime
            .current_profile_mut()
            .expect("SCRIPT2 load retained no mutable profile"),
    )?;

    let profile = runtime
        .current_profile_mut()
        .expect("authentic save restore retained no profile");
    let unlock = profile
        .state()
        .resolve_word_source_offset(SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET)
        .ok_or_else(|| anyhow::anyhow!("SCRIPT2 globals.A0 has no decoded state word"))?;
    if !profile
        .state_mut()
        .set_word(unlock, SCRIPT2_PTERRA_UNLOCKED)
    {
        anyhow::bail!("failed to seed SCRIPT2 globals.A0");
    }
    let save = OriginalSaveGame::capture(profile)?;
    std::fs::write(writable_path.join("GAME1.SAV"), save.encode())?;
    Ok(())
}

fn seed_script2_scruter_jo_aboard_save(
    asset_cache: &Path,
    writable_path: &Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(writable_path)?;
    let paths = OriginalGameDataPaths::from_root(asset_cache)?;
    let data = OriginalGameData::load_with_writable_root(paths, writable_path)?;
    let directory_name = BloodResourceName::new(b"BLOOD.SAV")?;
    let directory =
        OriginalSaveSlotDirectory::decode(&data.resource_store().load(&directory_name)?)?;
    std::fs::write(writable_path.join("BLOOD.SAV"), directory.encode())?;

    let mut runtime = OriginalGameRuntime::new(data);
    let script2 = ScriptProfileId::new(SCRIPT2_PROFILE).expect("SCRIPT2 profile is valid");
    runtime.load_profile(script2)?;
    let profile = runtime
        .current_profile_mut()
        .expect("SCRIPT2 load retained no mutable profile");
    let scruter_jo = profile
        .builtins()
        .scruter_jo
        .ok_or_else(|| anyhow::anyhow!("SCRIPT2 has no Scruter Jo object"))?;
    let scruter_kind = profile
        .state()
        .object(scruter_jo)
        .ok_or_else(|| anyhow::anyhow!("SCRIPT2 Scruter Jo object is missing from VAR"))?
        .kind;
    let holder_offset = script_field_offset(scruter_kind, ScriptFieldSelector::HOLDER_OR_LOCATION)
        .ok_or_else(|| anyhow::anyhow!("Scruter Jo has no holder/location field"))?;
    let holder = profile
        .state()
        .object_word(scruter_jo, holder_offset / size_of::<u16>())
        .ok_or_else(|| anyhow::anyhow!("Scruter Jo holder/location field is outside VAR"))?;
    profile
        .execution_parts()
        .record_state
        .record_fields
        .set_value(holder, ScriptRecordValue::Aboard);

    let save = OriginalSaveGame::capture(profile)?;
    std::fs::write(writable_path.join("GAME1.SAV"), save.encode())?;
    Ok(())
}

fn configured_runtime_asset_cache() -> Option<PathBuf> {
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

fn accuracy_tests_are_required() -> bool {
    std::env::var_os(REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE).is_some()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_trace(path: &Path) -> Vec<Value> {
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

fn decode_hexadecimal_bytes(encoded: &str) -> Vec<u8> {
    assert!(
        encoded.len().is_multiple_of(HEX_DIGITS_PER_BYTE),
        "hexadecimal trace value has an incomplete byte"
    );
    encoded
        .as_bytes()
        .chunks_exact(HEX_DIGITS_PER_BYTE)
        .map(|byte| {
            let byte = std::str::from_utf8(byte).expect("hexadecimal trace bytes are ASCII");
            u8::from_str_radix(byte, HEX_RADIX).expect("trace value contains invalid hexadecimal")
        })
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = FNV1A64_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV1A64_PRIME);
    }
    format!("{hash:016x}")
}

fn profile(record: &Value) -> Option<u64> {
    record["semantic"]["vm"]["resource_profile"].as_u64()
}

fn presentation(record: &Value) -> &Value {
    &record["semantic"]["presentation"]
}

fn console(record: &Value) -> &Value {
    &record["semantic"]["bridge_console"]
}

fn save_load(record: &Value) -> &Value {
    &record["semantic"]["save_load"]
}

fn input(record: &Value) -> &Value {
    &record["semantic"]["input"]
}

fn actor_slot(record: &Value, index: usize) -> &Value {
    &presentation(record)["bridge_actor_slots"][index]
}

fn traced_hit_region_contains(region: &Value, point: [i64; 2]) -> bool {
    let origin = region["origin"]
        .as_array()
        .expect("traced hit region has no origin");
    let extent = region["extent"]
        .as_array()
        .expect("traced hit region has no extent");
    (0..2).all(|axis| {
        let origin = origin[axis].as_i64().unwrap();
        let extent = extent[axis].as_i64().unwrap();
        point[axis] >= origin && point[axis] <= origin + extent
    })
}

fn console_u64(record: &Value, field: &str) -> u64 {
    console(record)[field].as_u64().unwrap()
}

fn presentation_flag(record: &Value, field: &str) -> bool {
    let value = &presentation(record)[field];
    value
        .as_bool()
        .unwrap_or_else(|| value.as_u64().is_some_and(|value| value != u64::MIN))
}

fn presentation_u64(record: &Value, path: &str) -> u64 {
    path.split('/')
        .fold(presentation(record), |value, field| &value[field])
        .as_u64()
        .unwrap()
}

fn assert_inline_menu_raster(record: &Value, label: &str) {
    let audit = &record["semantic"]["inline_menu_raster"];
    let expected = audit["expected_pixel_count"]
        .as_u64()
        .unwrap_or_else(|| panic!("{label} omitted its recovered inline-dialogue raster"));
    assert!(expected > 0, "{label} produced no visible glyph pixels");
    assert_eq!(
        audit["matching_pixel_count"].as_u64(),
        Some(expected),
        "{label} did not preserve its recovered dialogue glyphs in the live framebuffer"
    );
}

fn hand_selector(record: &Value) -> u64 {
    presentation_u64(record, "manu3_current")
}

fn audio_events(record: &Value) -> &[Value] {
    record["semantic"]["audio"]["events"]
        .as_array()
        .expect("runtime audio trace is not an event array")
}

fn descript(record: &Value) -> &Value {
    &record["semantic"]["descript"]
}

fn bridge_actor_hash(record: &Value) -> Option<&str> {
    record["semantic"]["video"]["bridge_layers"]["actor_sprite_hash"].as_str()
}

fn assert_izwalito_inset(record: &Value) {
    let portrait = &presentation(record)["portrait_entity"];
    assert_eq!(portrait["source"]["kind"], "cached");
    assert_eq!(portrait["source"]["resource"], PORTRAIT_RESOURCE);
    assert_eq!(
        portrait["draw_position"],
        serde_json::json!(PORTRAIT_POSITION)
    );
    assert_eq!(portrait["extent"], serde_json::json!(PORTRAIT_EXTENT));
    assert_ne!(
        portrait["flags"].as_u64().unwrap() & ACTIVE_ENTITY_FLAG,
        u64::MIN
    );
    assert!(record["semantic"]["vm"]["active_line"].is_null());
}
