use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use commander_blood_formats::archive::BloodResourceName;
use commander_blood_game::native::bloodprg::{
    OriginalSaveGame, OriginalSaveSlotDirectory, ScriptProfileId,
    original_save_state_block_byte_count,
};
use commander_blood_game::runtime::{OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime};
use serde_json::Value;

const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";
const DISPLAY_ENVIRONMENT_VARIABLES: [&str; 2] = ["DISPLAY", "WAYLAND_DISPLAY"];
const INTRO_ESCAPE_KEY: &str = "key 1";
const OPENING_VIDEO: &str = "sq\\mind.HNM";
const FIRST_STARTUP_VIDEO: &str = "SQ\\cliptoot.hnm";
const PHONE_CLICK: &str = "click 125 118";
const GAME_CHOICE_CLICK: &str = "sclick 200 105";
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
const LOAD_FIRST_SLOT_CLICK: &str = "sclick 100 40";
const OPTIONS_CANCEL_CLICK: &str = "sclick 100 125";
const TEXT_OPTION_CLICK: &str = "sclick 100 68";
const SEEDED_LOAD_PROFILE: u8 = 2;
const SCRIPT2_PROFILE: u8 = 1;
const SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET: u16 = 0x12C2;
const SCRIPT2_PTERRA_UNLOCKED: u16 = 1;
const SCRIPT2_PTERRA_MARKER: [u64; 2] = [201, 93];
const PTERRA_NAME: &str = "Pterra";
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
const STREAMED_DIALOGUE_EVENT_KIND: &str = "streamed_dialogue";
const VOICE_REACTION_EVENT_KIND: &str = "voice_reaction";
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

static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

struct TemporaryRoot(PathBuf);

impl TemporaryRoot {
    fn create() -> Self {
        let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "commander-blood-startup-phone-runtime-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TemporaryRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
    let Some(records) = run_production_scenario(
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
}

fn run_production_scenario(scenario: &str, trace_name: &str) -> Option<Vec<Value>> {
    run_production_scenario_with_setup(scenario, trace_name, |_, _| Ok(()))
}

fn run_production_scenario_with_setup(
    scenario: &str,
    trace_name: &str,
    setup: impl FnOnce(&Path, &Path) -> anyhow::Result<()>,
) -> Option<Vec<Value>> {
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
    let temporary = TemporaryRoot::create();
    let trace_path = temporary.0.join(trace_name);
    let writable_path = temporary.0.join("writable");
    setup(&asset_cache, &writable_path).unwrap();
    let scenario_path = root.join(scenario);
    let output = Command::new(env!("CARGO_BIN_EXE_commander-blood"))
        .arg("--write-data")
        .arg(&writable_path)
        .arg("--scenario")
        .arg(&scenario_path)
        .arg("--trace")
        .arg(&trace_path)
        .arg("--oracle-packed-second")
        .arg(DOS_ORACLE_PACKED_SECOND.to_string())
        .env(ASSET_CACHE_ENVIRONMENT_VARIABLE, asset_cache)
        .env("SDL_AUDIODRIVER", "dummy")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "production scenario {} failed:\nstdout:\n{}\nstderr:\n{}",
        scenario_path.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(load_trace(&trace_path))
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
