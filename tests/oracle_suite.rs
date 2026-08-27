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

use commander_blood_tools::engine::{
    ENGINE_SCREEN_HEIGHT, ENGINE_SCREEN_WIDTH, EngineState, MouseInput,
};
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
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        e.bridge.frame = 55;
        e.bridge.ring_mouse_x = 320;
        e.bridge.mouse_y = 100;
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
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
        results.push(Scenario {
            name: "engine-console-render",
            mean_abs: mean_abs(&rgb, &live),
            threshold,
        });
    }

    if results.is_empty() {
        eprintln!("oracle suite skipped: no bridge captures present");
        return;
    }

    // Report + assert the whole suite.
    let mut failures = Vec::new();
    eprintln!(
        "--- representative oracle suite ({} scenarios) ---",
        results.len()
    );
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
/// The list widget's left edge is a FORMULA, and two independent captures agree
/// with it at two different values.
///
/// `lx = anchor - (widest + 20)/2 + 10`, with the concept anchor `0xE1` = 225
/// (`mov [0xAC6],0xE1` @`0x89A6`). Nothing is imported from the captures except
/// two MEASUREMENTS — where the text starts, and how wide the widest row is — so
/// this verifies the decode rather than copying a layout out of it:
///
///   * `psychotherapy_topics.ppm`: widest row spans 111px, text starts at x=170.
///   * `honk_talk_menu.ppm`:       widest row spans 105px, text starts at x=173.
///
/// A centring implementation cannot satisfy both: it puts only the widest row at
/// the formula's x and every other row further right (audit-fixes #111). These
/// two files sat in the tree with no test reading them.
#[test]
fn list_widget_left_edge_matches_the_formula_in_two_captures() {
    // (path, expected leading x). The spans are measured, not asserted.
    let cases = [
        (
            "accuracy/captures/bridge/psychotherapy_topics.ppm",
            170usize,
        ),
        ("accuracy/captures/dialogue/honk_talk_menu.ppm", 173usize),
    ];
    let mut checked = 0;
    for (path, expect_x) in cases {
        let Some(px) =
            read_ppm(Path::new(path)).or_else(|| read_ppm(Path::new(&format!("../{path}"))))
        else {
            eprintln!("skipped: no {path}");
            continue;
        };
        let is_grey = |o: usize| {
            let (r, g, b) = (px[o] as i32, px[o + 1] as i32, px[o + 2] as i32);
            (r - 138).abs() < 45
                && (g - 138).abs() < 45
                && (b - 138).abs() < 45
                && (r.max(g).max(b) - r.min(g).min(b)) < 25
        };
        // Per-row extents of the text mask, right half of the screen only.
        let mut lead = usize::MAX;
        let mut widest_span = 0usize;
        for y in 0..200 {
            let xs: Vec<usize> = (150..ENGINE_SCREEN_WIDTH)
                .filter(|&x| is_grey((y * ENGINE_SCREEN_WIDTH + x) * 3))
                .collect();
            if xs.len() < 4 {
                continue; // not a text row
            }
            lead = lead.min(xs[0]);
            widest_span = widest_span.max(xs[xs.len() - 1] - xs[0] + 1);
        }
        assert_ne!(lead, usize::MAX, "{path}: found no text rows");
        assert_eq!(lead, expect_x, "{path}: measured left edge");

        // The mask spans one pixel more than the advance-summed width.
        let widest = widest_span - 1;
        let predicted = 0xE1usize - (widest + 20) / 2 + 10;
        assert_eq!(
            predicted, lead,
            "{path}: formula from anchor 0xE1 and widest {widest} predicts {predicted},              capture shows {lead}"
        );
        checked += 1;
    }
    assert!(checked > 0, "neither capture was readable");
}

/// The CHOICE BOX is centred on `x = 0x64` (`mov [0xAC6],0x64` @`0x86D9`), a
/// different anchor from the concept list's `0xE1` — and `post2_menu_choice.ppm`
/// shows it. That capture sat unread in the tree.
///
/// Only the text row's horizontal CENTRE is taken from the image, so the capture
/// verifies the decoded anchor rather than supplying it. The two anchors are 125px
/// apart, so this cannot pass with the wrong one.
#[test]
fn choice_box_is_centred_on_the_decoded_anchor() {
    let path = "accuracy/captures/dialogue/post2_menu_choice.ppm";
    let Some(px) = read_ppm(Path::new(path)).or_else(|| read_ppm(Path::new(&format!("../{path}"))))
    else {
        eprintln!("skipped: no {path}");
        return;
    };
    let is_grey = |o: usize| {
        let (r, g, b) = (px[o] as i32, px[o + 1] as i32, px[o + 2] as i32);
        (r - 138).abs() < 45
            && (g - 138).abs() < 45
            && (b - 138).abs() < 45
            && (r.max(g).max(b) - r.min(g).min(b)) < 25
    };
    let (mut lo, mut hi) = (usize::MAX, 0usize);
    for y in 0..200 {
        for x in 0..ENGINE_SCREEN_WIDTH {
            if is_grey((y * ENGINE_SCREEN_WIDTH + x) * 3) {
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
    }
    assert_ne!(lo, usize::MAX, "no text found in {path}");
    let centre = (lo + hi) / 2;
    let anchor = EngineState::CHOICE_BOX_CENTER_X;
    assert!(
        centre.abs_diff(anchor) <= 3,
        "text spans x {lo}..{hi} (centre {centre}); the decoded choice-box anchor \
         is {anchor}, and the concept anchor {} is far away",
        EngineState::CHOICE_BOX_ANCHOR_CONCEPT
    );
}

/// The intro montage's console band IS `TB.BIG` frame 90, rows 140..200, pushed
/// through the console-bank remap — byte for byte, all 19200 of them.
///
/// That identification is a headline decode (the band was never separate art),
/// and it was proven ONCE, by hand, with nothing guarding it since:
/// `console_band.idx` sat in the tree unread by any test. Change the frame index
/// or the remap builder and nothing would notice. This is that proof, run every
/// time — the same condition that let the list-menu centring error survive
/// (audit-fixes #111, #112).
#[test]
fn console_band_is_panorama_frame_90_through_the_remap() {
    let Some(iso) = iso_dir() else {
        eprintln!("skipped: no CD data");
        return;
    };
    let harvested = [
        "accuracy/captures/console_band.idx",
        "../accuracy/captures/console_band.idx",
    ]
    .into_iter()
    .find_map(|p| std::fs::read(p).ok());
    let Some(harvested) = harvested else {
        eprintln!("skipped: no console_band.idx");
        return;
    };

    let pan = BridgePanorama::parse(std::fs::read(iso.join("TB.BIG")).unwrap()).unwrap();
    let frame = pan
        .frame_pixels(commander_blood_tools::tbbig::CONSOLE_BAND_FRAME)
        .expect("frame 90 decodes");
    let table = commander_blood_tools::palette::build_console_bank_remap_table(
        &commander_blood_tools::palette::GAME_SCREEN_PALETTE_DAC,
    );

    let top = commander_blood_tools::tbbig::CONSOLE_BAND_TOP;
    let height = commander_blood_tools::tbbig::CONSOLE_BAND_HEIGHT;
    let composed: Vec<u8> = (top..top + height)
        .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| y * ENGINE_SCREEN_WIDTH + x))
        .map(|i| table[frame[i] as usize])
        .collect();

    assert_eq!(composed.len(), 19200, "60 rows of 320");
    assert_eq!(harvested.len(), composed.len(), "harvested band size");
    let differing = composed
        .iter()
        .zip(harvested.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(differing, 0, "{differing} of 19200 bytes differ");
}

/// `nav_screen_opened.ppm` is the VIEWSCREEN CONSOLE: binary static above,
/// console band below. Pointed at the right screen (audit-fixes #115), it is
/// checkable — just not pixel-by-pixel, because the static is generated noise.
///
/// What IS deterministic: the two colours it alternates between, the proportion
/// (`~54% index 224 / ~46% index 239`, per oracle intro_215M), and the row where
/// static stops and the band begins. The capture agrees on all three: 23315 black
/// to 19855 white across rows 0..134 is 54.0%/46.0%.
///
/// A pixel-exact assertion here would be wrong in principle — two runs of the real
/// game do not match each other either.
#[test]
fn viewscreen_static_matches_the_captures_two_colour_distribution() {
    let path = "accuracy/captures/bridge/nav_screen_opened.ppm";
    let Some(live) =
        read_ppm(Path::new(path)).or_else(|| read_ppm(Path::new(&format!("../{path}"))))
    else {
        eprintln!("skipped: no {path}");
        return;
    };

    // The capture's static region: exactly two colours, in per-pixel noise.
    let mut counts = std::collections::HashMap::new();
    for y in 0..135usize {
        for x in 0..ENGINE_SCREEN_WIDTH {
            let o = (y * ENGINE_SCREEN_WIDTH + x) * 3;
            *counts
                .entry([live[o], live[o + 1], live[o + 2]])
                .or_insert(0usize) += 1;
        }
    }
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let total: usize = ranked.iter().map(|(_, n)| *n).sum();
    let (dark, dark_n) = ranked[0];
    let (light, light_n) = ranked[1];
    assert!(
        dark_n + light_n > total * 99 / 100,
        "the static must be TWO colours; top two cover {}/{total}",
        dark_n + light_n
    );
    assert!(
        dark[0] < 40 && light[0] > 200,
        "one dark, one light: {dark:?} {light:?}"
    );

    // The split the port documents from oracle intro_215M: ~54/46.
    let dark_pct = dark_n * 100 / (dark_n + light_n);
    assert!(
        (52..=56).contains(&dark_pct),
        "expected ~54% dark, capture has {dark_pct}%"
    );

    // Per-pixel noise, not blocks: mean run length along a row is under 2.5px.
    let y = 60usize;
    let mut runs = 1usize;
    for x in 1..ENGINE_SCREEN_WIDTH {
        let a = (y * ENGINE_SCREEN_WIDTH + x) * 3;
        let b = (y * ENGINE_SCREEN_WIDTH + x - 1) * 3;
        if live[a..a + 3] != live[b..b + 3] {
            runs += 1;
        }
    }
    let mean_run = ENGINE_SCREEN_WIDTH as f64 / runs as f64;
    assert!(
        mean_run < 2.5,
        "per-pixel noise expected, mean run {mean_run:.2}"
    );

    // And the static STOPS where the console band starts. Not "no white below" --
    // the band has bright content of its own (483px, first attempt asserted zero
    // and failed on it). What separates them is DENSITY: 46% of the static region
    // is white, against 2.5% of the band.
    let band_top = commander_blood_tools::tbbig::CONSOLE_BAND_TOP;
    let white_frac = |y0: usize, y1: usize| -> f64 {
        let n = (y0..y1)
            .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| (y, x)))
            .filter(|&(y, x)| {
                let o = (y * ENGINE_SCREEN_WIDTH + x) * 3;
                live[o] > 200 && live[o + 1] > 200 && live[o + 2] > 200
            })
            .count();
        n as f64 / ((y1 - y0) * ENGINE_SCREEN_WIDTH) as f64
    };
    let above = white_frac(0, 135);
    let below = white_frac(band_top, 200);
    assert!(
        below < above / 5.0,
        "static should not continue past the band top {band_top}: \
         {:.1}% white above vs {:.1}% below",
        above * 100.0,
        below * 100.0
    );
}

/// IDENTIFY a capture: which asset frame is it?
///
/// #114 went wrong because a FILENAME was allowed to stand for evidence — three
/// port states were tried against `nav_screen_opened.ppm` before anyone asked
/// what the image actually was. This answers that mechanically: decode every
/// frame of every HNM under the asset root and report the closest matches by
/// mean absolute difference.
///
/// Run: `cargo test --test oracle_suite -- --ignored --nocapture identify_capture`
/// with `IDENTIFY=path/to/capture.ppm` (defaults to the two still-unread ones).
///
/// Reading the result: a real match lands near ZERO. Across 701 assets the best
/// score for either remaining capture is ~34, which is a negative result, not a
/// weak hit — both are COMPOSITED screens (scene plus overlays), so no single
/// frame equals them. Only full 320x200 frames are compared, and only the first
/// 40 of each file, so a match hiding in a small talk-head band or a long clip
/// would be missed; widen those before concluding a capture has no source.
#[test]
#[ignore]
fn identify_capture() {
    let Some(iso) = iso_dir() else {
        eprintln!("skipped: no CD data");
        return;
    };
    let targets: Vec<String> = match std::env::var("IDENTIFY") {
        Ok(v) => vec![v],
        Err(_) => vec![
            "accuracy/captures/mission_briefing_eye.ppm".into(),
            "accuracy/captures/bridge/script2_first_frame.ppm".into(),
        ],
    };

    // Every .hnm under the asset root, plus the assets dir next to it.
    let mut hnms: Vec<std::path::PathBuf> = Vec::new();
    let mut roots = vec![iso.to_path_buf()];
    if let Some(parent) = iso.parent() {
        roots.push(parent.join("_tmp_assets"));
        roots.push(parent.to_path_buf());
    }
    for root in roots {
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("hnm"))
                    .unwrap_or(false)
                {
                    hnms.push(p);
                }
            }
        }
    }
    hnms.sort();
    hnms.dedup();
    eprintln!("scanning {} HNM asset(s)", hnms.len());

    for target in targets {
        let Some(live) =
            read_ppm(Path::new(&target)).or_else(|| read_ppm(Path::new(&format!("../{target}"))))
        else {
            eprintln!("  no such capture: {target}");
            continue;
        };
        let mut best: Vec<(f64, String, usize)> = Vec::new();
        for path in &hnms {
            let Ok(h) = commander_blood_tools::hnm::HnmFile::open(path) else {
                continue;
            };
            let mut fb = vec![0u8; ENGINE_SCREEN_WIDTH * 200];
            let mut pal = [[0u8; 3]; 256];
            for idx in 0..h.frame_count().min(40) {
                let (w, ht, _) = h.decode_frame(idx, &mut fb, &mut pal);
                if w != ENGINE_SCREEN_WIDTH || ht != 200 {
                    continue;
                }
                let rgb: Vec<u8> = fb.iter().flat_map(|&i| pal[i as usize]).collect();
                best.push((
                    mean_abs(&rgb, &live),
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    idx,
                ));
            }
        }
        best.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        eprintln!("\n{target}:");
        for (d, name, idx) in best.iter().take(5) {
            eprintln!("   mean_abs {d:>7.2}  {name} frame {idx}");
        }
        if best.is_empty() {
            eprintln!("   no 320x200 frames decoded");
        }
    }
}

/// MISNAMED CAPTURE, kept as a measurement harness.
///
/// The star-layer count this prints is now EXPLAINED (audit-fixes #182): of 1000
/// points, 758 project, 64 land inside the viewport, and 33 survive the plot's
/// first-write-wins gate. Sparse by construction, not a wiring failure.
///
/// `nav_screen_opened.ppm` is not the nav screen and not a bridge starfield. Its
/// top 135 rows are two colours — black and white — in per-pixel noise (mean run
/// 1.87px): the binary STATIC of the presentation/boot screen, with the console
/// band below. An earlier version of this test read the dense white as stars and
/// concluded the bridge windows render black; the port's star layer plots 33
/// pixels because 1000 points plot at most ~1000, while the capture has 19855
/// white pixels. No point cloud makes that image (audit-fixes #115).
///
/// Left `#[ignore]` because it still measures something useful — the distance
/// between a chosen port state and a capture, with a star-layer pixel count — but
/// it is NOT evidence about the bridge until pointed at a capture of the bridge.
#[test]
#[ignore]
fn nav_screen_render_distance() {
    let Some(iso) = iso_dir() else {
        eprintln!("skipped: no CD data");
        return;
    };
    let path = "accuracy/captures/bridge/nav_screen_opened.ppm";
    let Some(live) =
        read_ppm(Path::new(path)).or_else(|| read_ppm(Path::new(&format!("../{path}"))))
    else {
        eprintln!("skipped: no {path}");
        return;
    };
    // The capture lives in captures/BRIDGE/ and shows a starfield behind the
    // panorama with the console band below -- that is render_bridge_background,
    // not the on_ship nav view (which draws CHART.FD). Compare the bridge at the
    // pyramid NAV ROOM station: STATION_REST_FRAMES[2] = frame 90.
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.bridge_active = true;
    e.step(MouseInput {
        x: 160,
        y: 100,
        buttons: 0,
        ..Default::default()
    });
    e.bridge.frame = commander_blood_tools::bridge::STATION_REST_FRAMES[2];
    e.step(MouseInput {
        x: 160,
        y: 100,
        buttons: 0,
        ..Default::default()
    });
    let rgb: Vec<u8> = e
        .framebuffer
        .iter()
        .flat_map(|&i| e.scene_palette[i as usize])
        .collect();
    // Is the STAR LAYER itself producing anything at the bridge's parameters?
    {
        use commander_blood_tools::ship3d::*;
        let mut prng = commander_blood_tools::ship3d::BloodPrng::seeded_from_rtc_seconds(0);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: 90 % 180,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin {
            x: 0x8000,
            y: 0x8000,
            z: 0x8000,
        };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: 320,
            top: 0,
            bottom: 200,
        };
        match render_ship_3d_starfield(&mut prng, angles, origin, viewport) {
            None => eprintln!("STAR LAYER: render_ship_3d_starfield returned None"),
            Some(r) => {
                eprintln!(
                    "STAR LAYER: {} non-zero of {}",
                    r.buffer.iter().filter(|&&p| p != 0).count(),
                    r.buffer.len()
                );
                // WHY so few? Count how the 1000 points are lost.
                let pts = randomize_ship_3d_point_cloud(
                    &mut commander_blood_tools::ship3d::BloodPrng::seeded_from_rtc_seconds(0),
                );
                let matrix = build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles).unwrap();
                let (mut projected, mut plotted) = (0, 0);
                for p in &pts {
                    if let Some(pr) = project_ship_3d_point(*p, origin, matrix) {
                        projected += 1;
                        let mut buf = vec![0u8; 320 * 200];
                        if plot_ship_3d_projected_point(&mut buf, viewport, pr).is_some() {
                            plotted += 1;
                        }
                    }
                }
                eprintln!(
                    "  of {} points: {projected} projected, {plotted} inside the viewport",
                    pts.len()
                );
            }
        }
    }
    eprintln!("nav-screen mean_abs = {:.2}", mean_abs(&rgb, &live));
    let nonzero = e.framebuffer.iter().filter(|&&i| i != 0).count();
    eprintln!("port framebuffer non-zero pixels: {nonzero} / 64000");
    // Dump the port's render so it can be inspected next to the capture.
    if let Ok(dir) = std::env::var("NAV_DUMP_DIR") {
        let mut ppm = format!("P6\n{ENGINE_SCREEN_WIDTH} 200\n255\n").into_bytes();
        ppm.extend_from_slice(&rgb);
        let _ = std::fs::write(std::path::Path::new(&dir).join("nav_port.ppm"), ppm);
    }
}

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
        "TALK",
        "EGO",
        "SUPER_EGO",
        "UNDER_EGO",
        "END_OF_MONTH",
        "LIBIDO",
        "WHO",
        "WHERE",
        "WHEN",
        "WHAT",
        "HOW",
        "WHY",
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
    let bounds = |name: &str, f: &dyn Fn(usize, usize) -> bool| {
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
        eprintln!(
            "row {row:2} y{y0:3}: port x={p:4}  live x={l:4}  {}",
            labels[row]
        );
    }
    bounds("port ", &|x, y| {
        e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == 0xE8
    });
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
        "TALK",
        "EGO",
        "SUPER_EGO",
        "UNDER_EGO",
        "END_OF_MONTH",
        "LIBIDO",
        "WHO",
        "WHERE",
        "WHEN",
        "WHAT",
        "HOW",
        "WHY",
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
