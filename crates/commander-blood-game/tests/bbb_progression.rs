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

fn replay_templand(finish: bool) {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = std::env::var_os("BBB_ASSET_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("output/big-bug-bang/imported-assets"));
    let save = std::env::var_os("BBB_PROGRESSED_SAVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace.join("output/big-bug-bang/daddy-tempest-save-load-01/writable")
        });
    let name = if finish {
        "bbb-templand-finish"
    } else {
        "bbb-templand-dialogue"
    };
    let artifacts = scenario_artifacts::ScenarioArtifacts::create(&workspace, name).unwrap();
    let writable = artifacts.0.join("writable");
    fs::create_dir(&writable).unwrap();
    for name in ["BLOOD.SAV", "GAME1.SAV"] {
        fs::copy(save.join(name), writable.join(name)).unwrap();
    }
    let scenario = workspace.join(if finish {
        "accuracy/scenarios/bbb_load_daddy_templand_finish.tsv"
    } else {
        "accuracy/scenarios/bbb_load_daddy_templand_dialogue.tsv"
    });
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
    let mut saw_interlude = false;
    let mut saw_continuation = false;
    let mut saw_choices = false;
    let mut saw_visible_choices = false;
    let mut saw_selection = false;
    let mut saw_finish_clip = false;
    let mut saw_finish_continuation = false;
    let mut finish_clip_completed = false;
    let mut returned_to_navigation = false;
    let mut missing_choice_frames = 0;
    for line in BufReader::new(File::open(&frames).unwrap()).lines() {
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
        returned_to_navigation = saw_selection
            && subtitle == "PLANET: Tempest LIFE FORMS: Daddy_Gluxx"
            && semantic["vm"]["resource_profile"] == 2
            && semantic["video"]["active_resource"].is_null()
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Closed"
            && semantic["presentation"]["screen_active"] == false
            && semantic["presentation"]["ship_scene"]["dispatch_blocked"] == false;
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
            "replay did not end at unblocked Tempest navigation"
        );
    }
}
