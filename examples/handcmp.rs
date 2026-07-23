//! Hand accuracy comparison: render the PORT's 3D hand at the same grid positions the
//! HANDGRID oracle captured (boot_frames/hg_X_Y.ppm = the real game's hand over the hub),
//! and write side-by-side PPMs + a pixel report. The oracle frames are the ground truth;
//! any deformation/miscolor in the port shows up as a large hand-region diff.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn read_ppm(p: &Path) -> Option<(usize, usize, Vec<u8>)> {
    let d = std::fs::read(p).ok()?;
    let s = std::str::from_utf8(&d[..64]).ok()?;
    let mut it = s.split_ascii_whitespace();
    let magic = it.next()?;
    if magic != "P6" {
        return None;
    }
    let w: usize = it.next()?.parse().ok()?;
    let h: usize = it.next()?.parse().ok()?;
    let _max = it.next()?;
    let hdr = d
        .windows(1)
        .enumerate()
        .filter(|(_, b)| b[0] == b'\n')
        .nth(2)
        .map(|(i, _)| i + 1)?;
    Some((w, h, d[hdr..].to_vec()))
}

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let outdir = Path::new("accuracy/comparisons/hand");
    std::fs::create_dir_all(outdir).unwrap();

    // The port scene: the bridge hub (same TB.BIG panorama the oracle frames show).
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.load_console_font(iso);
    e.on_ship = true;
    e.bridge_active = true;
    // The oracle frames sit at bridge ring frame 45 — align the port's view.
    e.bridge.frame = 45;

    let mut worst: Vec<(f64, String)> = Vec::new();
    for sy in (20..=190).step_by(34) {
        for sx in (40..=280).step_by(40) {
            let gt = Path::new("boot_frames").join(format!("hg_{sx}_{sy}.ppm"));
            let Some((gw, gh, gpix)) = read_ppm(&gt) else {
                continue;
            };
            assert_eq!((gw, gh), (320, 200));
            // Render the port frame with the hand at (sx, sy).
            e.step(MouseInput {
                x: sx as u16,
                y: sy as u16,
                buttons: 0,
                ..Default::default()
            });
            let mut port = vec![0u8; 320 * 200 * 3];
            for (i, &px) in e.framebuffer.iter().enumerate() {
                let c = e.scene_palette[px as usize];
                port[i * 3..i * 3 + 3].copy_from_slice(&c);
            }
            // Diff only the hand's neighborhood (the hand hangs down-left of the tip).
            let (x0, x1) = ((sx as i32 - 90).max(0) as usize, (sx + 20).min(319));
            let (y0, y1) = ((sy as i32 - 15).max(0) as usize, (sy + 90).min(199));
            let mut acc = 0f64;
            let mut n = 0f64;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let i = (y * 320 + x) * 3;
                    for c in 0..3 {
                        acc += (gpix[i + c] as f64 - port[i + c] as f64).abs();
                    }
                    n += 3.0;
                }
            }
            let mean = acc / n;
            worst.push((mean, format!("hg_{sx}_{sy}")));
            // side-by-side
            let mut sbs = vec![0u8; 640 * 200 * 3];
            for y in 0..200 {
                for x in 0..320 {
                    let s = (y * 320 + x) * 3;
                    let d0 = (y * 640 + x) * 3;
                    let d1 = (y * 640 + x + 320) * 3;
                    sbs[d0..d0 + 3].copy_from_slice(&gpix[s..s + 3]);
                    sbs[d1..d1 + 3].copy_from_slice(&port[s..s + 3]);
                }
            }
            let mut out = format!("P6\n640 200\n255\n").into_bytes();
            out.extend_from_slice(&sbs);
            std::fs::write(outdir.join(format!("sbs_{sx}_{sy}.ppm")), out).unwrap();
            let mut pf = format!("P6\n320 200\n255\n").into_bytes();
            pf.extend_from_slice(&port);
            std::fs::write(outdir.join(format!("port_hg_{sx}_{sy}.ppm")), pf).unwrap();
        }
    }
    worst.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    println!("hand-region mean-abs diff (oracle vs port), worst first:");
    for (m, name) in worst.iter().take(8) {
        println!("  {name}: {m:.2}");
    }
    let avg: f64 = worst.iter().map(|w| w.0).sum::<f64>() / worst.len() as f64;
    println!("average over {} positions: {avg:.2}", worst.len());
}
