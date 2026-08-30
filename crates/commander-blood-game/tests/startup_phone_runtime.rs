use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const DISPLAY_ENVIRONMENT_VARIABLES: [&str; 2] = ["DISPLAY", "WAYLAND_DISPLAY"];
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
const SAVE_CANCEL_CLICK: &str = "sclick 100 151";
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
const EMPTY_LABELS: [&str; 0] = [];
const CONTACT_LABELS: [&str; 1] = ["Bob_Morlock"];
const OPTION_LABELS: [&str; 5] = ["TEXT", "MUSIC_OFF", "SAVE", "LOAD", "QUIT"];
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
    let after_answer = &records[phone_index + 1..];
    let active = after_answer
        .iter()
        .find(|record| {
            profile(record) == Some(INITIAL_PROFILE)
                && presentation_flag(record, "active")
                && presentation_flag(record, "defer")
        })
        .expect("phone answer never acquired Izwalito presentation ownership");
    assert_izwalito_inset(active);

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
        assert_ne!(
            presentation_u64(settled, "ui_flags") & NAVIGATION_UI_FLAG,
            u64::MIN
        );
    }
}

#[test]
fn production_runtime_opens_and_closes_the_authored_save_menu() {
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
}

fn run_production_scenario(scenario: &str, trace_name: &str) -> Option<Vec<Value>> {
    let asset_cache = configured_runtime_asset_cache()?;
    if !DISPLAY_ENVIRONMENT_VARIABLES
        .iter()
        .any(|variable| std::env::var_os(variable).is_some())
    {
        return None;
    }

    let root = workspace_root();
    let temporary = TemporaryRoot::create();
    let trace_path = temporary.0.join(trace_name);
    let writable_path = temporary.0.join("writable");
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

fn configured_runtime_asset_cache() -> Option<PathBuf> {
    let path = std::env::var_os(ASSET_CACHE_ENVIRONMENT_VARIABLE).map(PathBuf::from)?;
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
