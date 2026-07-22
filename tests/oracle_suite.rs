//! Representative oracle suite (Definition of Done #8): compares the port's
//! rendering/decoding of the game's major screens against real captures taken
//! from the original `BLOODPRG.EXE` running inside the in-repo emulator
//! (`runtime_boot` diagnostics; captures under `accuracy/captures/`). Each
//! scenario asserts a measured mean-absolute-difference threshold, so a
//! rendering/decoding regression on any covered screen fails the suite.
//!
//! Scenarios that need the CD data (`output/_tmp_iso/TB.BIG`) or a capture that
//! is absent are skipped, not failed — the suite runs wherever the assets are
//! present and is a no-op otherwise.

use commander_blood_tools::engine::{EngineState, MouseInput, ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH};
use commander_blood_tools::tbbig::BridgePanorama;
use std::path::Path;

/// Read a fixed 320x200 P6 PPM's RGB body.
fn read_ppm(path: &Path) -> Option<Vec<u8>> {
    let raw = std::fs::read(path).ok()?;
    let at = raw.windows(4).position(|w| w == b"255\n")? + 4;
    let body = &raw[at..];
    (body.len() == ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3).then(|| body.to_vec())
}

fn iso_dir() -> Option<&'static Path> {
    ["output/_tmp_iso", "../output/_tmp_iso"]
        .into_iter()
        .map(Path::new)
        .find(|p| p.join("TB.BIG").exists())
}

fn capture(name: &str) -> Option<Vec<u8>> {
    for base in ["accuracy/captures/bridge", "../accuracy/captures/bridge"] {
        if let Some(px) = read_ppm(&Path::new(base).join(name)) {
            return Some(px);
        }
    }
    None
}

/// Mean absolute difference between two 320x200 RGB buffers.
fn mean_abs(a: &[u8], b: &[u8]) -> f64 {
    let total: u64 = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64)
        .sum();
    total as f64 / a.len() as f64
}

/// Render a decoded panorama frame to RGB via the game palette.
fn panorama_rgb(pan: &BridgePanorama, frame: usize) -> Vec<u8> {
    let dac = &commander_blood_tools::palette::GAME_SCREEN_PALETTE_DAC;
    let expand = |v: u8| (v << 2) | (v >> 4);
    pan.frame_pixels(frame)
        .unwrap()
        .iter()
        .flat_map(|&i| (0..3).map(move |c| expand(dac[i as usize * 3 + c])))
        .collect()
}

/// One oracle scenario result.
struct Scenario {
    name: &'static str,
    mean_abs: f64,
    threshold: f64,
}

impl Scenario {
    fn passed(&self) -> bool {
        self.mean_abs < self.threshold
    }
}

/// Run every available scenario and assert the whole suite passes.
#[test]
fn representative_oracle_suite() {
    let Some(iso) = iso_dir() else {
        eprintln!("oracle suite skipped: no CD data (output/_tmp_iso/TB.BIG)");
        return;
    };
    let pan = BridgePanorama::parse(std::fs::read(iso.join("TB.BIG")).unwrap()).unwrap();
    let mut results: Vec<Scenario> = Vec::new();

    // 1. Panorama decode: console rest frame (55), and the two steered views
    //    (15 = left edge, 64 = right edge), each vs its live emulator capture.
    for (frame, cap, threshold) in [(55usize, "console_rest.ppm", 3.0), (15, "rotate_left.ppm", 5.0), (64, "rotate_right.ppm", 5.0)] {
        if let Some(live) = capture(cap) {
            results.push(Scenario {
                name: match frame { 55 => "panorama-console-f55", 15 => "panorama-steer-f15", _ => "panorama-steer-f64" },
                mean_abs: mean_abs(&panorama_rgb(&pan, frame), &live),
                threshold,
            });
        }
    }

    // 2. Full engine console render (panorama + hand atlas + menu DAC) vs live.
    if let Some(live) = capture("console_rest.ppm") {
        let mut e = EngineState::new();
        e.load_bridge(iso);
        for dir in ["accuracy/captures/bridge/hand", "../accuracy/captures/bridge/hand"] {
            e.load_hand_atlas(Path::new(dir));
            if e.hand_atlas_len() > 0 {
                break;
            }
        }
        e.bridge_active = true;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        e.bridge.frame = 55;
        e.bridge.ring_mouse_x = 320;
        e.bridge.mouse_y = 100;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        // Render to RGB through the engine's scene palette.
        let rgb: Vec<u8> = e
            .framebuffer
            .iter()
            .flat_map(|&i| e.scene_palette[i as usize])
            .collect();
        let threshold = if e.hand_atlas_len() == 0 { 4.0 } else { 1.0 };
        results.push(Scenario { name: "engine-console-render", mean_abs: mean_abs(&rgb, &live), threshold });
    }

    if results.is_empty() {
        eprintln!("oracle suite skipped: no bridge captures present");
        return;
    }

    // Report + assert the whole suite.
    let mut failures = Vec::new();
    eprintln!("--- representative oracle suite ({} scenarios) ---", results.len());
    for s in &results {
        eprintln!(
            "  {:<26} mean_abs={:>6.2}  (< {:.1})  {}",
            s.name,
            s.mean_abs,
            s.threshold,
            if s.passed() { "PASS" } else { "FAIL" }
        );
        if !s.passed() {
            failures.push(s.name);
        }
    }
    assert!(failures.is_empty(), "oracle scenarios failed: {failures:?}");
}
