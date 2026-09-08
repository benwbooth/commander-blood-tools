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

#[test]
#[ignore = "requires BBB assets, a progressed Daddy save, and a graphical display"]
fn honk_inventory_pass_retains_six_items_and_releases_presentation() {
    let frames = replay_bbb(
        "bbb-honk-inventory",
        "accuracy/scenarios/bbb_load_honk_inventory.tsv",
    );
    assert_honk_inventory_trace(&frames);
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a retained inventory trace"]
fn validate_recorded_honk_inventory_pass() {
    let frames =
        PathBuf::from(std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"));
    assert_honk_inventory_trace(&frames);
}

#[test]
#[ignore = "requires BBB assets, a progressed Daddy save, and a graphical display"]
fn honk_inventory_survives_save_and_fresh_process_load() {
    let frames = replay_bbb(
        "bbb-honk-save",
        "accuracy/scenarios/bbb_honk_inventory_save_followup.tsv",
    );
    assert_honk_inventory_trace(&frames);
    let save = frames.parent().unwrap().join("writable");
    let loaded = replay_bbb_from_save(
        "bbb-honk-load",
        "accuracy/scenarios/bbb_load_honk_checkpoint.tsv",
        &save,
    );
    assert_honk_checkpoint_trace(&loaded);
    for name in ["BLOOD.SAV", "GAME1.SAV"] {
        assert_eq!(
            fs::read(save.join(name)).unwrap(),
            fs::read(loaded.parent().unwrap().join("writable").join(name)).unwrap(),
            "loading and contacting Honk must not rewrite {name}"
        );
    }
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a fresh-process Honk checkpoint load"]
fn validate_recorded_honk_checkpoint_load() {
    let frames =
        PathBuf::from(std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"));
    assert_honk_checkpoint_trace(&frames);
}

#[test]
#[ignore = "requires BBB assets, an earned Honk inventory save, and a graphical display"]
fn daddy_offers_earned_inventory_after_loading() {
    let frames = replay_inventory_checkpoint(
        "bbb-daddy-inventory",
        "accuracy/scenarios/bbb_honk_checkpoint_templand.tsv",
    );
    assert_daddy_inventory_offer(&frames);
}

fn replay_inventory_checkpoint(name: &str, scenario: &str) -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let save = std::env::var_os("BBB_INVENTORY_SAVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace.join("output/big-bug-bang/honk-save-followup-jcsWFXhd/writable")
        });
    replay_bbb_from_save(name, scenario, &save)
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a retained Daddy inventory offer"]
fn validate_recorded_daddy_inventory_offer() {
    let frames =
        PathBuf::from(std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"));
    assert_daddy_inventory_offer(&frames);
}

#[test]
#[ignore = "requires BBB assets, an earned Honk inventory save, and a graphical display"]
fn writing_gift_survives_save_and_fresh_process_load() {
    let frames = replay_inventory_checkpoint(
        "bbb-daddy-writing",
        "accuracy/scenarios/bbb_daddy_give_writing.tsv",
    );
    assert_daddy_writing_gift(&frames);
    let save = frames.parent().unwrap().join("writable");
    let loaded = replay_bbb_from_save(
        "bbb-writing-load",
        "accuracy/scenarios/bbb_load_writing_checkpoint.tsv",
        &save,
    );
    assert_writing_checkpoint_load(&loaded);
    for name in ["BLOOD.SAV", "GAME1.SAV"] {
        assert_eq!(
            fs::read(save.join(name)).unwrap(),
            fs::read(loaded.parent().unwrap().join("writable").join(name)).unwrap()
        );
    }
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a retained writing gift trace"]
fn validate_recorded_daddy_writing_gift() {
    assert_daddy_writing_gift(&PathBuf::from(
        std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"),
    ));
}

#[test]
#[ignore = "requires BBB_PROGRESSION_TRACE pointing to a fresh writing checkpoint load"]
fn validate_recorded_writing_checkpoint_load() {
    assert_writing_checkpoint_load(&PathBuf::from(
        std::env::var_os("BBB_PROGRESSION_TRACE").expect("BBB_PROGRESSION_TRACE"),
    ));
}

fn has_writing_gift(semantic: &serde_json::Value) -> bool {
    let Some(objects) = semantic["persistent"]["object_locations"].as_array() else {
        return false;
    };
    objects.iter().any(|object| {
        object["name"] == "ecriture"
            && object["kind"] == "InventoryItem"
            && object["holder_raw"] == 0xC24
            && object["target_name"] == "Daddy_Gluxx"
    }) && ["technologie", "guitare", "parfum", "energie", "vaisseau"]
        .iter()
        .all(|name| {
            objects.iter().any(|object| {
                object["name"] == *name
                    && object["kind"] == "InventoryItem"
                    && object["holder_raw"] == 65535
            })
        })
}

fn main_profile_unblocked(semantic: &serde_json::Value) -> bool {
    semantic["vm"]["resource_profile"] == 1
        && semantic["vm"]["execution_enabled"] == 1
        && semantic["presentation"]["active_actor_presentation"].is_null()
        && semantic["presentation"]["active"] == 0
        && semantic["presentation"]["screen_active"] == false
        && semantic["presentation"]["retained_word_choice"]["phase"] == "Closed"
        && semantic["presentation"]["ship_scene"]["dispatch_blocked"] == false
}

fn assert_daddy_writing_gift(frames: &std::path::Path) {
    assert_daddy_inventory_offer(frames);
    let mut acknowledged = false;
    let mut returned = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        acknowledged |= semantic["vm"]["resource_profile"] == 2
            && has_writing_gift(semantic)
            && semantic["subtitle"]
                == "WRITING has made them intelligent. We can finally talk to them...";
        returned = acknowledged && has_writing_gift(semantic) && main_profile_unblocked(semantic);
    }
    assert!(
        acknowledged,
        "writing transfer and intelligence acknowledgement were not observed together"
    );
    assert!(
        returned,
        "writing gift did not persist through an unblocked return"
    );
}

fn assert_writing_checkpoint_load(frames: &std::path::Path) {
    let mut saw_startup = false;
    let mut loaded = false;
    let mut ready = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        if !saw_startup && semantic["vm"]["resource_profile"].is_null() {
            continue;
        }
        if !loaded && semantic["vm"]["resource_profile"] == 0 {
            saw_startup = true;
            continue;
        }
        assert!(saw_startup, "fresh startup was not observed");
        assert_eq!(
            semantic["vm"]["resource_profile"], 1,
            "wrong writing checkpoint profile"
        );
        assert!(
            has_writing_gift(semantic),
            "saved writing gift was not restored immediately or lost later"
        );
        loaded = true;
        ready = main_profile_unblocked(semantic);
    }
    assert!(loaded && ready, "writing checkpoint never became ready");
}

fn assert_daddy_inventory_offer(frames: &std::path::Path) {
    let mut loaded_inventory = false;
    let mut travel_enabled = false;
    let mut saw_interlude = false;
    let mut saw_continue_choice = false;
    let mut offered = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        let owns_all = [
            "technologie",
            "guitare",
            "parfum",
            "energie",
            "ecriture",
            "vaisseau",
        ]
        .iter()
        .all(|name| {
            semantic["persistent"]["object_locations"]
                .as_array()
                .is_some_and(|objects| {
                    objects.iter().any(|object| {
                        object["kind"] == "InventoryItem"
                            && object["name"] == *name
                            && object["holder_raw"] == 65535
                    })
                })
        });
        loaded_inventory |= semantic["vm"]["resource_profile"] == 1 && owns_all;
        travel_enabled |= loaded_inventory && semantic["vm"]["sequel_travel_enabled"] == true;
        if semantic["vm"]["resource_profile"] != 2 {
            continue;
        }
        saw_interlude |=
            travel_enabled && semantic["video"]["active_resource"] == "SQ\\venus06.hnm";
        saw_continue_choice |= saw_interlude
            && semantic["presentation"]["rendered_word_choices"]
                == serde_json::json!(["finish", "no_hurry"]);
        if saw_continue_choice
            && semantic["subtitle"] == "GIVE:"
            && semantic["presentation"]["active_actor_presentation"]["name"] == "Daddy_Gluxx"
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Selecting"
            && semantic["presentation"]["rendered_word_choices"]
                == serde_json::json!([
                    "technology",
                    "guitar",
                    "perfume",
                    "energy",
                    "writing",
                    "ship"
                ])
        {
            let rows = semantic["presentation"]["retained_word_choice"]["rows"]
                .as_array()
                .unwrap();
            let visible = rows.len() == 7
                && rows[6]["kind"] == "Cancel"
                && rows
                    .iter()
                    .all(|row| row["matching_text_pixels"].as_u64().is_some_and(|n| n > 0));
            offered |= owns_all && visible;
        }
    }
    assert!(
        loaded_inventory,
        "earned inventory checkpoint was not loaded"
    );
    assert!(
        travel_enabled,
        "Travel was not enabled before visiting Daddy"
    );
    assert!(
        saw_interlude && saw_continue_choice,
        "Daddy continuation route was not traversed"
    );
    assert!(
        offered,
        "Daddy never offered all six earned items with visible labels and Cancel"
    );
}

fn assert_honk_checkpoint_trace(frames: &std::path::Path) {
    let mut saw_startup = false;
    let mut loaded = false;
    let mut responded = false;
    let mut released = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        if !saw_startup && semantic["vm"]["resource_profile"].is_null() {
            continue;
        }
        if !loaded && semantic["vm"]["resource_profile"] == 0 {
            saw_startup = true;
            continue;
        }
        assert!(saw_startup, "fresh-process startup was not observed");
        assert_eq!(
            semantic["vm"]["resource_profile"], 1,
            "wrong checkpoint profile"
        );
        if !loaded {
            assert_ne!(
                semantic["subtitle"], "I'm searching...",
                "stale phone response at load"
            );
        }
        // Check the first loaded frame, before Honk can grant any more objects.
        for name in [
            "ecriture",
            "vaisseau",
            "technologie",
            "guitare",
            "energie",
            "parfum",
        ] {
            assert!(
                semantic["persistent"]["object_locations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|object| object["name"] == name
                        && object["kind"] == "InventoryItem"
                        && object["holder_raw"] == 65535),
                "loaded checkpoint lost {name}"
            );
        }
        loaded = true;
        if semantic["subtitle"] == "I'm searching..." {
            assert_eq!(semantic["audio"]["streamed_sound_bank"], "radio.snd");
            responded = true;
        }
        released = semantic["vm"]["execution_enabled"] == 1
            && semantic["presentation"]["active_actor_presentation"].is_null()
            && semantic["presentation"]["active"] == 0
            && semantic["presentation"]["screen_active"] == false
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Closed"
            && semantic["presentation"]["ship_scene"]["dispatch_blocked"] == false;
    }
    assert!(loaded, "checkpoint never loaded");
    assert!(responded, "Honk did not respond after loading");
    assert!(
        released,
        "loaded checkpoint ended with blocked presentation"
    );
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
    let save = std::env::var_os("BBB_PROGRESSED_SAVE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace.join("output/big-bug-bang/daddy-tempest-save-load-01/writable")
        });
    replay_bbb_from_save(name, scenario, &save)
}

fn replay_bbb_from_save(name: &str, scenario: &str, save: &std::path::Path) -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = std::env::var_os("BBB_ASSET_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("output/big-bug-bang/imported-assets"));
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

fn assert_honk_inventory_trace(frames: &std::path::Path) {
    use std::collections::BTreeSet;
    assert_honk_trace(frames);
    let mut baseline = None;
    let mut final_inventory = BTreeSet::new();
    let mut saw_honk = false;
    let mut returned = false;
    for line in BufReader::new(File::open(frames).unwrap()).lines() {
        let frame: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let semantic = &frame["semantic"];
        if semantic["vm"]["resource_profile"] != 1 {
            returned = false;
            continue;
        }
        let inventory: BTreeSet<String> = semantic["persistent"]["object_locations"]
            .as_array()
            .expect("persistent object locations")
            .iter()
            .filter(|object| object["kind"] == "InventoryItem" && object["holder_raw"] == 65535)
            .map(|object| object["name"].as_str().expect("inventory name").to_owned())
            .collect();
        baseline.get_or_insert_with(|| inventory.clone());
        final_inventory = inventory;
        saw_honk |= semantic["presentation"]["active_actor_presentation"]["name"] == "Honk";
        returned = saw_honk
            && final_inventory
                .difference(baseline.as_ref().unwrap())
                .count()
                == 6
            && semantic["vm"]["execution_enabled"] == 1
            && semantic["presentation"]["active_actor_presentation"].is_null()
            && semantic["presentation"]["active"] == 0
            && semantic["presentation"]["screen_active"] == false
            && semantic["presentation"]["retained_word_choice"]["phase"] == "Closed"
            && semantic["presentation"]["ship_scene"]["dispatch_blocked"] == false;
    }
    let baseline = baseline.expect("main profile inventory baseline");
    for name in [
        "ecriture",
        "vaisseau",
        "technologie",
        "guitare",
        "energie",
        "parfum",
    ] {
        assert!(!baseline.contains(name), "fixture already owns {name}");
        assert!(final_inventory.contains(name), "Honk did not retain {name}");
    }
    assert_eq!(
        final_inventory.difference(&baseline).count(),
        6,
        "hold-search inventory delta differs from the six-item fixture"
    );
    assert!(returned, "Honk did not return to an unblocked main profile");
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
