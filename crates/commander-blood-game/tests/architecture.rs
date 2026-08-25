use std::path::{Path, PathBuf};

const FORBIDDEN_HEURISTIC_DEPENDENCIES: [&str; 3] =
    ["commander_blood_tools", "EngineState", "recomp::"];

fn rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, output);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            output.push(path);
        }
    }
}

#[test]
fn modern_game_does_not_depend_on_the_retired_heuristic_runtime() {
    let game_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(game_root.join("Cargo.toml")).unwrap();
    assert!(!manifest.contains("commander-blood-tools"));

    let mut sources = Vec::new();
    rust_sources(&game_root.join("src"), &mut sources);
    for source_path in sources {
        let source = std::fs::read_to_string(&source_path).unwrap();
        for forbidden in FORBIDDEN_HEURISTIC_DEPENDENCIES {
            assert!(
                !source.contains(forbidden),
                "{} references retired heuristic dependency {forbidden}",
                source_path.display()
            );
        }
    }
}
