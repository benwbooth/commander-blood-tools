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

    // 1. Panorama decode across all four station sectors, each vs a live
    //    emulator capture at that view: helm/menu console (frame 55), the two
    //    steered edges (15/64), the pyramid-nav room (95), and the Orxx mass
    //    (135). Covers the whole 360° ring, not just the rest frame.
    for (frame, name, cap, threshold) in [
        (55usize, "panorama-console-f55", "console_rest.ppm", 3.0),
        (15, "panorama-steer-f15", "rotate_left.ppm", 5.0),
        (64, "panorama-steer-f64", "rotate_right.ppm", 5.0),
        (95, "panorama-nav-f95", "sector_nav_f95.ppm", 5.0),
        (135, "panorama-orxx-f135", "sector_orxx_f135.ppm", 3.0),
    ] {
        if let Some(live) = capture(cap) {
            results.push(Scenario {
                name,
                mean_abs: mean_abs(&panorama_rgb(&pan, frame), &live),
                threshold,
            });
        }
    }

    // 2. Full engine console render (panorama + hand + menu DAC) vs live.
    if let Some(live) = capture("console_rest.ppm") {
        let mut e = EngineState::new();
        e.load_bridge(iso);
        e.bridge_active = true;
        e.step(MouseInput { x: 160, y: 100, buttons: 0, ..Default::default() });
        e.bridge.frame = 55;
        e.bridge.ring_mouse_x = 320;
        e.bridge.mouse_y = 100;
        e.step(MouseInput { x: 160, y: 100, buttons: 0, ..Default::default() });
        // Render to RGB through the engine's scene palette.
        let rgb: Vec<u8> = e
            .framebuffer
            .iter()
            .flat_map(|&i| e.scene_palette[i as usize])
            .collect();
        // The capture-sprite hand atlas this used to load was DEAD -- it was parsed
        // but never drawn (no HandSprite field was ever read), so both branches of
        // the old threshold measured the same render. The faithful hand is
        // manu3_hand::HandMesh, decoded from manu3.xdb's mesh and cursor law. The
        // panorama-only match is ~2.5; 8.0 is the no-hand tolerance that branch
        // actually exercised.
        let threshold = 8.0;
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

/// Oracle: the psychotherapy CONCEPT MENU text (the square-caps topic list) is
/// rendered faithfully. We feed the port the twelve real concept labels harvested
/// from the live `concept_menu.ppm` capture, render them through the engine's
/// list-menu widget, and compare the resulting glyph mask (framebuffer index
/// 0xE8) against the capture's grey text mask over the eleven glyph-count-verified
/// rows (TALK..HOW). A high intersection-over-union proves the widget's geometry
/// (x=170, first row y=34, 11px pitch), the PROPORTIONAL advance (glyph width +
/// 2px), and the glyph shapes all match the original — the whole word "LIBIDO"
/// (with two 1px-wide 'I's) only lands correctly if the advance is proportional.
/// Diagnostic for the concept-menu divergence: prints where each mask actually
/// sits, so a failing IoU says WHICH WAY it is wrong instead of just how much.
/// Run with `cargo test --test oracle_suite -- --ignored concept_menu_masks`.
#[test]
#[ignore]
fn concept_menu_mask_bounds() {
    let Some(cap) = capture("concept_menu.ppm") else {
        eprintln!("skipped: no concept_menu.ppm");
        return;
    };
    let labels: Vec<String> = [
        "TALK", "EGO", "SUPER_EGO", "UNDER_EGO", "END_OF_MONTH", "LIBIDO", "WHO", "WHERE", "WHEN",
        "WHAT", "HOW", "WHY",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let mut e = EngineState::new();
    e.draw_list_menu(&labels, None);

    let is_grey = |r: u8, g: u8, b: u8| {
        let (r, g, b) = (r as i32, g as i32, b as i32);
        (r - 138).abs() < 45
            && (g - 138).abs() < 45
            && (b - 138).abs() < 45
            && (r.max(g).max(b) - r.min(g).min(b)) < 25
    };
    let mut bounds = |name: &str, f: &dyn Fn(usize, usize) -> bool| {
        let (mut x0, mut x1, mut y0, mut y1, mut n) = (9999usize, 0usize, 9999usize, 0usize, 0u32);
        for y in 0..200 {
            for x in 0..ENGINE_SCREEN_WIDTH {
                if f(x, y) {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                    n += 1;
                }
            }
        }
        eprintln!("{name}: x {x0}..{x1}  y {y0}..{y1}  pixels {n}");
    };
    // Per-ROW leading x: identical bounding boxes with poor overlap means the rows
    // are placed differently inside the same band -- centred vs flush-left.
    for row in 0..12 {
        let y0 = 34 + row * 11;
        let lead = |f: &dyn Fn(usize, usize) -> bool| {
            (0..ENGINE_SCREEN_WIDTH)
                .find(|&x| (y0..y0 + 9).any(|y| y < 200 && f(x, y)))
                .map(|x| x as i32)
                .unwrap_or(-1)
        };
        let p = lead(&|x, y| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == 0xE8);
        let l = lead(&|x, y| {
            let o = (y * ENGINE_SCREEN_WIDTH + x) * 3;
            is_grey(cap[o], cap[o + 1], cap[o + 2])
        });
        eprintln!("row {row:2} y{y0:3}: port x={p:4}  live x={l:4}  {}", labels[row]);
    }
    bounds("port ", &|x, y| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == 0xE8);
    bounds("live ", &|x, y| {
        let o = (y * ENGINE_SCREEN_WIDTH + x) * 3;
        is_grey(cap[o], cap[o + 1], cap[o + 2])
    });
}

#[test]
fn concept_menu_text_matches_live_game_capture() {
    let Some(cap) = capture("concept_menu.ppm") else {
        eprintln!("concept-menu oracle skipped: no concept_menu.ppm");
        return;
    };
    // The real concept list (12 rows: TALK..WHY). The menu is row-count-CENTERED,
    // so all 12 must be rendered for the band to top at y=34 (choice_box_top_y(12));
    // the compare region below covers rows 0..=10, glyph-verified against the capture
    // (the 12th row WHY at y~155 and the trailing indented "44" are outside it).
    let labels: Vec<String> = [
        "TALK", "EGO", "SUPER_EGO", "UNDER_EGO", "END_OF_MONTH", "LIBIDO", "WHO", "WHERE", "WHEN",
        "WHAT", "HOW", "WHY",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut e = EngineState::new();
    e.draw_list_menu(&labels, None);

    // The list region: right column, first eleven rows (y 32..153).
    let (x0, x1, y0, y1) = (168usize, 305usize, 32usize, 153usize);
    let is_grey = |r: u8, g: u8, b: u8| {
        let (r, g, b) = (r as i32, g as i32, b as i32);
        (r - 138).abs() < 45
            && (g - 138).abs() < 45
            && (b - 138).abs() < 45
            && (r.max(g).max(b) - r.min(g).min(b)) < 25
    };
    let (mut inter, mut uni) = (0u32, 0u32);
    for y in y0..y1 {
        for x in x0..x1 {
            let port = e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == 0xE8;
            let o = (y * ENGINE_SCREEN_WIDTH + x) * 3;
            let live = is_grey(cap[o], cap[o + 1], cap[o + 2]);
            if port && live {
                inter += 1;
            }
            if port || live {
                uni += 1;
            }
        }
    }
    let iou = inter as f64 / uni as f64;
    eprintln!("concept-menu text IoU = {iou:.3} (inter={inter}, union={uni})");
    // Observed 1.000 (pixel-exact). A tight gate so a geometry/advance regression
    // (e.g. reverting the proportional advance, which misaligns LIBIDO) fails here.
    assert!(
        iou > 0.90,
        "concept-menu text mask must overlap the live capture (IoU {iou:.3} <= 0.90)"
    );
}
