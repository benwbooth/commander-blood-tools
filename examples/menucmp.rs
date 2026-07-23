//! Menu accuracy audit: the oracle hub background (median of the HANDGRID frames) shows
//! the REAL console menu (gold box: HONK/TELEPHONE/CRYOBOX/MENU/OPTION), the CANCEL
//! label, and the orb. Render the port's hub with its console box open, align the ring
//! frame, and pixel-diff. Writes a side-by-side and reports the per-region errors.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn oracle_bg() -> Vec<u8> {
    let mut frames: Vec<Vec<u8>> = Vec::new();
    for sy in (20..=190).step_by(34) {
        for sx in (40..=280).step_by(40) {
            if let Ok(d) = std::fs::read(format!("boot_frames/hg_{sx}_{sy}.ppm")) {
                let hdr =
                    d.iter().enumerate().filter(|&(_, &b)| b == b'\n').nth(2).unwrap().0 + 1;
                frames.push(d[hdr..].to_vec());
            }
        }
    }
    let n = frames.len();
    let mut bg = vec![0u8; 320 * 200 * 3];
    let mut buf = vec![0u8; n];
    for i in 0..bg.len() {
        for (j, f) in frames.iter().enumerate() {
            buf[j] = f[i];
        }
        buf.sort_unstable();
        bg[i] = buf[n / 2];
    }
    bg
}

fn main() {
    let bg = oracle_bg();
    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.load_console_font(iso);
    e.on_ship = true;
    e.bridge_active = true;
    e.console_box = vec![
        "HONK".into(),
        "TELEPHONE".into(),
        "CRYOBOX".into(),
        "MENU".into(),
        "OPTION".into(),
    ];

    // Align the ring: scan for the best frame with the menu drawn, hand parked.
    let mut best = (f64::MAX, 0u16);
    for frame in 0..180u16 {
        e.bridge.frame = frame;
        e.step(MouseInput { x: 315, y: 5, buttons: 0, ..Default::default() });
        e.bridge.frame = frame;
        let (mut acc, mut cnt) = (0f64, 0f64);
        for y in 0..200usize {
            for x in 0..320usize {
                if x >= 235 && y <= 115 {
                    continue; // parked hand region
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
    println!("best frame {} mean_abs {:.2} (menu open)", best.1, best.0);

    e.bridge.frame = best.1;
    e.step(MouseInput { x: 315, y: 5, buttons: 0, ..Default::default() });
    e.bridge.frame = best.1;
    // Region errors: the menu box (oracle: x~170..305, y~55..165), left half.
    let regions = [
        ("menu box", 170usize, 55usize, 305usize, 165usize),
        ("left half", 0, 0, 160, 200),
        ("full", 0, 0, 320, 200),
    ];
    for (name, x0, y0, x1, y1) in regions {
        let (mut acc, mut cnt) = (0f64, 0f64);
        for y in y0..y1 {
            for x in x0..x1 {
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
        println!("  {name}: mean_abs {:.2}", acc / cnt);
    }
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
    std::fs::write("accuracy/comparisons/hand/menu_sbs.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/menu_sbs.ppm");
}
