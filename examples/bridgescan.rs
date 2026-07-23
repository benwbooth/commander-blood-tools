//! Bridge view-mapping audit: render the port's bridge background at every ring frame
//! and diff each against the oracle's hand-free background (boot_frames median,
//! written by accuracy/hand_isolate.py as hg_bg.png -> re-exported as PPM here from
//! the oracle frames directly). Reports the best-matching frame and its error — if the
//! best error is high, the unpack itself is wrong, not just the index.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    // Rebuild the oracle background (median) from the raw grid frames, skipping the
    // hand region around each frame's cursor: simpler — median across frames per pixel.
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for sy in (20..=190).step_by(34) {
        for sx in (40..=280).step_by(40) {
            let p = format!("boot_frames/hg_{sx}_{sy}.ppm");
            if let Ok(d) = std::fs::read(&p) {
                let hdr = d.iter().enumerate().filter(|&(_, &b)| b == b'\n').nth(2).unwrap().0 + 1;
                frames.push(d[hdr..].to_vec());
            }
        }
    }
    let n = frames.len();
    let mut bg = vec![0u8; 320 * 200 * 3];
    let mut buf = vec![0u8; n];
    for i in 0..320 * 200 * 3 {
        for (j, f) in frames.iter().enumerate() {
            buf[j] = f[i];
        }
        buf.sort_unstable();
        bg[i] = buf[n / 2];
    }

    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.load_console_font(iso);
    e.on_ship = true;
    e.bridge_active = true;

    let mut best = (f64::MAX, 0u16);
    for frame in 0..180u16 {
        e.bridge.frame = frame;
        // park the mouse far corner so the hand sits mostly off-frame
        e.step(MouseInput { x: 315, y: 5, buttons: 0, ..Default::default() });
        e.bridge.frame = frame;
        let mut acc = 0f64;
        // compare rows 0..200, columns excluding the right edge (hand at 315,5 covers
        // the top-right; exclude x>=240 y<=110 region)
        let mut cnt = 0f64;
        for y in 0..200usize {
            for x in 0..320usize {
                if x >= 235 && y <= 115 {
                    continue;
                }
                let i = y * 320 + x;
                let c = e.scene_palette[e.framebuffer[i] as usize];
                for ch in 0..3 {
                    acc += (c[ch] as f64 - bg[i * 3 + ch] as f64).abs();
                    cnt += 1.0;
                }
            }
        }
        let mean = acc / cnt;
        if mean < best.0 {
            best = (mean, frame);
        }
    }
    println!("best port frame vs oracle ring-45 bg: frame {} mean_abs {:.2}", best.1, best.0);

    // dump the best frame side-by-side for visual inspection
    e.bridge.frame = best.1;
    e.step(MouseInput { x: 315, y: 5, buttons: 0, ..Default::default() });
    e.bridge.frame = best.1;
    let mut sbs = vec![0u8; 640 * 200 * 3];
    for y in 0..200 {
        for x in 0..320 {
            let i = y * 320 + x;
            let d0 = (y * 640 + x) * 3;
            let d1 = (y * 640 + x + 320) * 3;
            sbs[d0..d0 + 3].copy_from_slice(&bg[i * 3..i * 3 + 3]);
            let c = e.scene_palette[e.framebuffer[i] as usize];
            sbs[d1..d1 + 3].copy_from_slice(&c);
        }
    }
    let mut out = b"P6\n640 200\n255\n".to_vec();
    out.extend_from_slice(&sbs);
    std::fs::write("accuracy/comparisons/hand/bridge_best.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/bridge_best.ppm");
}
