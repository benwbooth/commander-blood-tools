use std::path::{Path, PathBuf};

const FORBIDDEN_HEURISTIC_DEPENDENCIES: [&str; 3] =
    ["commander_blood_tools", "EngineState", "recomp::"];
const FORBIDDEN_LEGACY_MEMORY_MODEL_MARKERS: [&str; 68] = [
    "FarPtr",
    "FarPointer",
    "NearPtr",
    "NearPointer",
    "HugePtr",
    "HugePointer",
    "SegmentedMemory",
    "SegmentAddress",
    "SegmentOffset",
    "SegmentRegister",
    "SegmentSelector",
    "SegmentPointer",
    "RealModeAddress",
    "RealModePointer",
    "DosAddress",
    "ConventionalMemory",
    "far_ptr",
    "far_pointer",
    "near_ptr",
    "near_pointer",
    "huge_ptr",
    "huge_pointer",
    "read_16_far",
    "write_16_far",
    "paragraph_address",
    "real_mode_address",
    "real_mode_pointer",
    "segment_to_linear",
    "segment_register",
    "segment_offset",
    "segment_selector",
    "segment_pointer",
    "selector_offset",
    "linear_address",
    "DosPointer",
    "dos_pointer",
    "dos_address",
    "PhysicalAddress",
    "ParagraphAddress",
    "SegPtr",
    "SegPointer",
    "DosMemory",
    "RealModeMemory",
    "EmulatedMemory",
    "__far",
    "__near",
    "__huge",
    "MK_FP",
    "FP_SEG",
    "FP_OFF",
    "dos_memory",
    "real_mode_memory",
    "segment_base",
    "memory_paragraph",
    "AddressTranslation",
    "address_translation",
    "AddressSpace",
    "address_space",
    "CpuRegisters",
    "cpu_registers",
    "CpuRegisterState",
    "cpu_register_state",
    "RegisterFile",
    "register_file",
    "RegisterState",
    "register_state",
    "MemoryBus",
    "memory_bus",
];
const FORBIDDEN_RUNTIME_CAPTURE_MARKERS: [&str; 4] = [
    "accuracy/manu3",
    "manu3_ds.bin",
    "manu3_seg2_1b76.bin",
    "manu3_seg4_1c94.bin",
];

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

#[test]
fn modern_game_does_not_recreate_segmented_memory() {
    let game_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_sources(&game_root.join("src"), &mut sources);

    for source_path in sources {
        let source = std::fs::read_to_string(&source_path).unwrap();
        for forbidden in FORBIDDEN_LEGACY_MEMORY_MODEL_MARKERS {
            assert!(
                !source.contains(forbidden),
                "{} contains legacy memory-model marker {forbidden}",
                source_path.display()
            );
        }
    }
}

#[test]
fn modern_game_does_not_ship_runtime_memory_captures() {
    let game_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_sources(&game_root.join("src"), &mut sources);

    for source_path in sources {
        let source = std::fs::read_to_string(&source_path).unwrap();
        for forbidden in FORBIDDEN_RUNTIME_CAPTURE_MARKERS {
            assert!(
                !source.contains(forbidden),
                "{} references runtime capture {forbidden}",
                source_path.display()
            );
        }
    }
}

#[test]
fn production_runtime_opens_only_the_imported_loose_asset_store() {
    let game_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let runtime = std::fs::read_to_string(game_root.join("src/runtime.rs")).unwrap();
    let lifecycle =
        std::fs::read_to_string(game_root.join("src/runtime/game_lifecycle.rs")).unwrap();
    let importer = std::fs::read_to_string(game_root.join("src/asset_import.rs")).unwrap();
    let media_importer = std::fs::read_to_string(game_root.join("src/media_import.rs")).unwrap();
    let services = std::fs::read_to_string(game_root.join("src/runtime/services.rs")).unwrap();

    assert!(!runtime.contains("BloodArchive::decode"));
    assert!(!runtime.contains("std::fs::read(paths.archive"));
    assert!(!lifecycle.contains("archive_entries()"));
    assert!(importer.contains("BloodArchive::decode"));
    assert!(importer.contains("RESOURCE_DIRECTORY_NAME"));
    assert!(media_importer.contains("VocPcm::decode"));
    assert!(media_importer.contains("SndBank::decode"));
    assert!(services.contains(".normalized_media()"));
    assert!(!services.contains("VocPcm::decode"));
}
