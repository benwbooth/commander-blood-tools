use commander_blood_script_compiler::analyze_dialogue;
use serde_json::Value;

fn profile(game: &str, index: usize) -> (String, Value) {
    let directory = match game {
        "cb" => "profiles",
        _ => "big-bug-bang-profiles",
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../re/vm/{directory}/script{index}.blood"));
    let source = std::fs::read_to_string(path).unwrap();
    let graph = analyze_dialogue(&source).unwrap();
    (source, graph)
}

#[test]
fn every_authored_text_statement_has_a_site_in_all_22_profiles() {
    let mut totals = [0, 0];
    for (game_index, (game, count)) in [("cb", 5), ("bbb", 17)].into_iter().enumerate() {
        for index in 1..=count {
            let (source, graph) = profile(game, index);
            let expected = source
                .lines()
                .map(str::trim_start)
                .filter(|line| line.starts_with("say ") || line.starts_with("text_tokens "))
                .count();
            let cod = graph["cod"]["text_sites"].as_array().unwrap();
            let bas = graph["bas"]["text_sites"].as_array().map_or(0, Vec::len);
            assert_eq!(cod.len() + bas, expected, "{game} SCRIPT{index}");
            if game == "bbb" {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                    "../../localization/big-bug-bang/en/script{index}.json"
                ));
                let translation: Value =
                    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
                for key in ["cod_sha256", "dic_sha256"] {
                    assert_eq!(
                        graph["resources"][key], translation[key],
                        "SCRIPT{index} {key}"
                    );
                }
                for site in cod {
                    let key = format!(
                        "bbb.script{index}.cod.{:08x}",
                        site["offset"].as_u64().unwrap()
                    );
                    assert_eq!(
                        site["sections"].as_array().unwrap().len(),
                        translation["messages"][key].as_array().unwrap().len()
                    );
                }
            }
            totals[game_index] += expected;
            assert!(
                graph["cod"]["control_flow"]["unresolved_guard_branches"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let blocks = graph["cod"]["control_flow"]["blocks"].as_array().unwrap();
            for edge in graph["cod"]["control_flow"]["edges"].as_array().unwrap() {
                for endpoint in ["from_block", "to_block"] {
                    assert!(blocks.iter().any(|block| block["start"] == edge[endpoint]));
                }
            }
            for site in cod {
                assert!(blocks.iter().any(|block| block["start"] == site["block"]));
            }
        }
    }
    assert_eq!(totals, [5536, 6921]);
}

#[test]
fn honk_choices_guards_and_deferred_profile_request_are_preserved() {
    let (_, graph) = profile("bbb", 1);
    let sites = graph["cod"]["text_sites"].as_array().unwrap();
    let menu = sites
        .iter()
        .find(|site| {
            site["choice_operands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|word| word["text"] == "JOUER")
        })
        .unwrap();
    assert_eq!(menu["choice_operands"][1]["text"], "EXPLICATIONS");
    assert!(menu["resume_offset"].is_number());
    let instructions = graph["cod"]["instructions"].as_array().unwrap();
    for word in menu["choice_operands"].as_array().unwrap() {
        assert!(
            instructions
                .iter()
                .any(|item| item["procedure"] == menu["procedure"]
                    && item["instruction"]["ConceptGuard"]["word_offset"] == word["offset"])
        );
    }
    assert!(
        graph["cod"]["profile_requests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|request| request["procedure"] == menu["procedure"]
                && request["target"] == "SCRIPT2")
    );
    for kind in ["guard_pass", "guard_failure", "frame_resume", "text_skip"] {
        assert!(
            graph["cod"]["control_flow"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["kind"] == kind)
        );
    }
}

#[test]
fn duplicate_bas_selectors_use_first_match_and_keep_shadowed_nodes() {
    let (_, graph) = profile("cb", 3);
    let edges = graph["bas"]["choice_edges"].as_array().unwrap();
    let duplicates = edges
        .iter()
        .filter(|edge| !edge["shadowed_nodes"].as_array().unwrap().is_empty())
        .collect::<Vec<_>>();
    assert!(!duplicates.is_empty());
    for edge in duplicates {
        assert_eq!(edge["object"], "Izwalito");
        assert_eq!(edge["to_node"], 0x397);
        assert_eq!(edge["shadowed_nodes"], serde_json::json!([0x3c9]));
    }
    let (_, graph) = profile("cb", 1);
    assert_eq!(
        graph["bas"]["choice_edges"][0]["resolution"],
        "no_local_selector_match"
    );
    assert!(graph["bas"]["choice_edges"][0]["to_node"].is_null());
}

#[test]
fn state_numbers_and_inventory_are_symbolic_not_fabricated_text() {
    let (_, graph) = profile("bbb", 2);
    let sites = graph["cod"]["text_sites"].as_array().unwrap();
    let numeric = sites
        .iter()
        .find(|site| site["text"].as_str().unwrap().contains("{VAR:"))
        .unwrap();
    assert_eq!(numeric["dynamic"], true);
    assert!(
        numeric["spoken_operands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|word| word["kind"] == "state_number")
    );
    let (_, graph) = profile("bbb", 17);
    let sites = graph["cod"]["text_sites"].as_array().unwrap();
    assert!(
        sites
            .iter()
            .flat_map(|site| site["choice_operands"].as_array().unwrap())
            .any(|word| word["kind"] == "inventory_choices")
    );
}

#[test]
fn invalid_source_fails_instead_of_producing_a_partial_catalog() {
    assert!(analyze_dialogue("bloodscript 8\nprofile SCRIPT1\nlogic { invalid }").is_err());
}
