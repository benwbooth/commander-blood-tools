use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

#[path = "support/scenario_artifacts.rs"]
mod scenario_artifacts;
#[path = "support/scenario_process.rs"]
mod scenario_process;

#[test]
#[ignore = "requires BBB assets, a progressed Daddy save, and a graphical display"]
fn templand_interlude_returns_to_dialogue_and_choices() {
    replay_templand(false);
}

#[test]
#[ignore = "requires BBB assets, a progressed Daddy save, and a graphical display"]
fn templand_finish_exits_after_authored_clip() {
    replay_templand(true);
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a retained live continuation trace"]
fn validate_recorded_templand_continuation() {
    let frames =
        PathBuf::from(std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"));
    assert_templand_trace(&frames, false);
}

#[test]
#[ignore = "requires BBB assets, a progressed Daddy save, and a graphical display"]
fn honk_phone_loads_radio_bank_and_passes_cryobox_dialogue() {
    let frames = replay_bbb(
        "bbb-honk-radio",
        "accuracy/scenarios/bbb_load_daddy_f7_phone.tsv",
    );
    assert_honk_trace(&frames);
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a retained Honk trace"]
fn validate_recorded_honk_radio_continuation() {
    let frames =
        PathBuf::from(std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"));
    assert_honk_trace(&frames);
}

fn replay_templand(finish: bool) {
    let frames = replay_bbb(
        if finish {
            "bbb-templand-finish"
        } else {
            "bbb-templand-dialogue"
        },
        if finish {
            "accuracy/scenarios/bbb_load_daddy_templand_finish.tsv"
        } else {
            "accuracy/scenarios/bbb_load_daddy_templand_dialogue.tsv"
        },
    );
    assert_templand_trace(&frames, finish);
}

fn replay_bbb(name: &str, scenario: &str) -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = std::env::var_os("BBB_ASSET_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("output/big-bug-bang/imported-assets"));
    let save = std::env::var_os("BBB_PROGRESSED_SAVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace.join("output/big-bug-bang/daddy-tempest-save-load-01/writable")
        });
    let artifacts = scenario_artifacts::ScenarioArtifacts::create(&workspace, name).unwrap();
    let writable = artifacts.0.join("writable");
    fs::create_dir(&writable).unwrap();
    for name in ["BLOOD.SAV", "GAME1.SAV"] {
        fs::copy(save.join(name), writable.join(name)).unwrap();
    }
    let scenario = workspace.join(scenario);
    let frames = artifacts.0.join("frames.jsonl");
    let mut command = Command::new(env!("CARGO_BIN_EXE_commander-blood"));
    command
        .args(["--data"])
        .arg(&assets)
        .arg("--write-data")
        .arg(&writable)
        .arg("--scenario")
        .arg(&scenario)
        .arg("--trace")
        .arg(artifacts.0.join("trace.jsonl"))
        .arg("--live-trace")
        .arg(&frames)
        .env("SDL_AUDIODRIVER", "dummy");
    let timeout = scenario_artifacts::timeout().unwrap();
    artifacts
        .record_inputs(&command, &scenario, &assets, &writable, timeout)
        .unwrap();
    let outcome = scenario_process::run(&mut command, &artifacts.0, timeout).unwrap();
    assert!(
        outcome.status.success() && !outcome.timed_out,
        "BBB replay failed; artifacts: {}",
        artifacts.0.display()
    );
    frames
}

fn assert_honk_trace(frames: &std::path::Path) {
    let mut loaded_save = false;
    let mut returned = false;
    let mut honk_with_bank = false;
    let mut reached_cryobox = false;
    let mut streamed_at_cryobox = None;
    let mut streamed_after_cryobox = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        loaded_save |= semantic["vm"]["resource_profile"] == 2;
        returned |= loaded_save && semantic["vm"]["resource_profile"] == 1;
        let honk = semantic["presentation"]["active_actor_presentation"]["name"] == "Honk";
        if returned && honk {
            assert_eq!(
                semantic["audio"]["streamed_sound_bank"], "radio.snd",
                "Honk must load his authored radio bank before dispatch"
            );
            honk_with_bank = true;
        }
        let streamed = semantic["audio"]["events"].as_array().map_or(0, |events| {
            events
                .iter()
                .filter(|event| event["kind"] == "streamed_dialogue")
                .count()
        });
        reached_cryobox |=
            honk_with_bank && semantic["subtitle"] == "I'll put them in the cryobox for you...";
        if reached_cryobox {
            let baseline = *streamed_at_cryobox.get_or_insert(streamed);
            streamed_after_cryobox |= streamed > baseline;
        }
    }
    assert!(
        loaded_save && returned,
        "save load and F7 return were not observed"
    );
    assert!(honk_with_bank, "Honk never acquired his streamed bank");
    assert!(
        reached_cryobox,
        "the previously crashing line was not reached"
    );
    assert!(
        streamed_after_cryobox,
        "no streamed dialogue followed the formerly crashing line"
    );
}

fn assert_templand_trace(frames: &std::path::Path, finish: bool) {
    let mut saw_interlude = false;
    let mut saw_continuation = false;
    let mut saw_choices = false;
    let mut saw_visible_choices = false;
    let mut saw_selection = false;
    let mut saw_finish_clip = false;
    let mut saw_finish_continuation = false;
    let mut finish_clip_completed = false;
    let mut returned_to_navigation = false;
    let mut ended_in_main_profile = false;
    let mut missing_choice_frames = 0;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        saw_interlude |= semantic["video"]["active_resource"] == "SQ\\venus06.hnm";
        let subtitle = semantic["subtitle"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        saw_continuation |= saw_interlude && subtitle.starts_with("With that boiled-shank face");
        let expected_choices = semantic["presentation"]["rendered_word_choices"]
            == serde_json::json!(["finish", "no_hurry"]);
        saw_choices |= saw_continuation && expected_choices;
        if saw_continuation
            && expected_choices
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Selecting"
        {
            let visible = semantic["presentation"]["retained_word_choice"]["rows"]
                .as_array()
                .is_some_and(|rows| {
                    rows.len() == 2
                        && rows.iter().all(|row| {
                            row["matching_text_pixels"]
                                .as_u64()
                                .is_some_and(|pixels| pixels > 0)
                        })
                });
            saw_visible_choices |= visible;
            missing_choice_frames += usize::from(!visible);
        }
        saw_selection |= saw_visible_choices
            && subtitle.starts_with(if finish {
                "it's over for you"
            } else {
                "let's continue, then"
            });
        saw_finish_clip |= saw_selection && semantic["video"]["active_resource"] == "SQ\\fin.hnm";
        saw_finish_continuation |= saw_finish_clip && subtitle.starts_with("Still, no racism");
        finish_clip_completed = saw_finish_clip && semantic["video"]["active_resource"].is_null();
        returned_to_navigation |= saw_selection
            && semantic["navigation"]["camera"]["target"]["name"] == "Tempest"
            && semantic["navigation"]["camera"]["target"]["record"] == 72
            && semantic["presentation"]["active_actor_presentation"].is_null()
            && semantic["presentation"]["active"] == 0
            && (finish || semantic["vm"]["resource_profile"] == 1)
            && semantic["video"]["active_resource"].is_null()
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Closed"
            && semantic["presentation"]["screen_active"] == false
            && semantic["presentation"]["ship_scene"]["dispatch_blocked"] == false;
        ended_in_main_profile = semantic["vm"]["resource_profile"] == 1;
    }
    assert!(saw_interlude, "Templand interlude was never reached");
    assert!(
        saw_continuation,
        "venus06 completion did not release the next authored A6 line"
    );
    assert!(
        saw_choices,
        "renewed Templand conversation never reached its next choice"
    );
    assert!(
        saw_visible_choices,
        "choice labels never reached the RGB UI layer"
    );
    assert_eq!(
        missing_choice_frames, 0,
        "choice labels disappeared while selecting"
    );
    assert!(saw_selection, "selection never resumed the authored branch");
    if finish {
        assert!(
            saw_finish_clip,
            "FINISH never played its authored fin.hnm clip"
        );
        assert!(
            finish_clip_completed,
            "game exited before fin.hnm completed"
        );
        assert!(
            !saw_finish_continuation,
            "terminal fin.hnm resumed the script"
        );
        assert!(
            !returned_to_navigation,
            "terminal fin.hnm returned to navigation"
        );
    } else {
        assert!(
            returned_to_navigation,
            "replay never returned to unblocked Tempest navigation in SCRIPT2"
        );
        assert!(
            ended_in_main_profile,
            "replay left SCRIPT2 after the conversation return"
        );
    }
}
