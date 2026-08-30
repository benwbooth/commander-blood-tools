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
const DOS_ORACLE_PACKED_SECOND: u8 = 39;

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
    let Some(asset_cache) = configured_runtime_asset_cache() else {
        return;
    };
    if !DISPLAY_ENVIRONMENT_VARIABLES
        .iter()
        .any(|variable| std::env::var_os(variable).is_some())
    {
        return;
    }

    let root = workspace_root();
    let temporary = TemporaryRoot::create();
    let trace_path = temporary.0.join("startup-phone.jsonl");
    let writable_path = temporary.0.join("writable");
    let scenario_path = root.join("accuracy/scenarios/startup_phone_complete.tsv");
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
        "production startup-phone scenario failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let records = load_trace(&trace_path);
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
