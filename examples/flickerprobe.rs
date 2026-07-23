//! Reproduce the 60fps flicker: mimic the main loop on the post-Esc (SCRIPT1 dialogue)
//! screen — one game tick, then fast refreshes — and hash what each present would draw.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn tri_sig(tris: &Option<Vec<[[f32; 5]; 3]>>) -> String {
    match tris {
        None => "NONE".into(),
        Some(t) if t.is_empty() => "EMPTY".into(),
        Some(t) => {
            let mut minx = f32::MAX;
            let mut miny = f32::MAX;
            let mut maxx = f32::MIN;
            let mut maxy = f32::MIN;
            for tri in t {
                for v in tri.iter().copied() {
                    minx = minx.min(v[0]);
                    maxx = maxx.max(v[0]);
                    miny = miny.min(v[1]);
                    maxy = maxy.max(v[1]);
                }
            }
            format!("{} tris bbox=({minx:.1},{miny:.1})..({maxx:.1},{maxy:.1})", t.len())
        }
    }
}

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.load_console_font(iso);
    e.gpu_hand_enabled = true;
    // BRIDGE hub state (the other post-Esc screen).
    e.on_ship = true;
    e.bridge_active = true;

    let fbsig = |e: &EngineState| -> u64 {
        let mut h = 1469598103934665603u64;
        for &b in e.framebuffer.iter().step_by(97) {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
        h
    };
    for cycle in 0..3 {
        e.step(MouseInput { x: 160, y: 100, buttons: 0, ..Default::default() });
        let tick = e.gpu_hand.take();
        println!(
            "tick {cycle}:  {} stars={} key={} fb={:x}",
            tri_sig(&tick),
            e.gpu_stars.as_ref().map_or(0, |s| s.len()),
            e.gpu_bg_colorkey,
            fbsig(&e)
        );
        for f in 0..3 {
            e.refresh_gpu_hand(160, 100);
            let fast = e.gpu_hand.take();
            println!(
                "  fast {f}: {} stars={} key={} fb={:x}",
                tri_sig(&fast),
                e.gpu_stars.as_ref().map_or(0, |s| s.len()),
                e.gpu_bg_colorkey,
                fbsig(&e)
            );
        }
    }
}
