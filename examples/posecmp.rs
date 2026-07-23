//! Pose-sequence VISUAL verification against live captures: the hand atlas
//! (accuracy/captures/bridge/hand/hand_<x>_<y>.bin = the REAL renderer's output,
//! captured live per cursor position/context) is matched against the port's hand
//! rendered with each decoded pose sequence at the same cursor. Reports, per sprite,
//! which selector's sequence best reproduces it and how closely — visual confirmation
//! for the sequences reachable in the captured contexts.

use commander_blood_tools::manu3_hand::{HandMesh, PosePlayer};
use std::path::Path;

struct Sprite {
    cx: i32,
    cy: i32,
    anchor: (i32, i32),
    w: usize,
    h: usize,
    idx: Vec<u8>,
}

fn load_atlas(dir: &Path) -> Vec<Sprite> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some((a, b)) = name
            .strip_prefix("hand2_")
            .or_else(|| name.strip_prefix("hand_"))
            .and_then(|s| s.strip_suffix(".bin"))
            .and_then(|s| s.split_once('_'))
        else {
            continue;
        };
        let (Ok(cx), Ok(cy)) = (a.parse(), b.parse()) else { continue };
        let Ok(d) = std::fs::read(e.path()) else { continue };
        if d.len() < 8 {
            continue;
        }
        let word = |at: usize| i16::from_le_bytes([d[at], d[at + 1]]) as i32;
        let (ax, ay, w, h) = (word(0), word(2), word(4) as usize, word(6) as usize);
        if w == 0 || h == 0 || d.len() < 8 + w * h {
            continue;
        }
        out.push(Sprite { cx, cy, anchor: (ax, ay), w, h, idx: d[8..8 + w * h].to_vec() });
    }
    out
}

fn main() {
    let atlas = load_atlas(Path::new("accuracy/captures/bridge/hand"));
    if atlas.is_empty() {
        println!("no atlas sprites found");
        return;
    }
    println!("{} atlas sprites (live captures)", atlas.len());
    let selectors: &[u16] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10];

    // Pre-render each selector's settled hand once per unique cursor.
    let mut per_sel_hits: std::collections::HashMap<u16, (usize, f64)> = Default::default();
    let mut shown = 0;
    for sp in &atlas {
        let mut best: (f64, u16) = (0.0, u16::MAX);
        for &sel in selectors {
            let mut mesh = HandMesh::load();
            if let Some(mut p) = PosePlayer::new(sel as usize) {
                for _ in 0..600 {
                    if p.done() {
                        break;
                    }
                    mesh.animate(&mut p);
                }
            }
            let mut fb = vec![0u8; 320 * 200];
            mesh.draw(&mut fb, 320, 200, sp.cx, sp.cy);
            // Compare inside the sprite's bbox (screen coords from anchor).
            let (x0, y0) = (sp.cx - sp.anchor.0, sp.cy - sp.anchor.1);
            let mut agree = 0usize;
            let mut total = 0usize;
            for sy in 0..sp.h {
                for sx in 0..sp.w {
                    let (px, py) = (x0 + sx as i32, y0 + sy as i32);
                    if !(0..320).contains(&px) || !(0..200).contains(&py) {
                        continue;
                    }
                    let live = sp.idx[sy * sp.w + sx];
                    let ours = fb[py as usize * 320 + px as usize];
                    // shape agreement: both-on or both-off
                    if (live != 0) == (ours != 0) {
                        agree += 1;
                    }
                    total += 1;
                }
            }
            let score = agree as f64 / total.max(1) as f64;
            if score > best.0 {
                best = (score, sel);
            }
        }
        let e = per_sel_hits.entry(best.1).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += best.0;
        if shown < 12 {
            println!(
                "  sprite @({},{}) {}x{}: best selector {:#x} shape-agreement {:.1}%",
                sp.cx,
                sp.cy,
                sp.w,
                sp.h,
                best.1,
                best.0 * 100.0
            );
            shown += 1;
        }
    }
    println!("\nper-selector wins (count, mean agreement):");
    let mut rows: Vec<_> = per_sel_hits.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (sel, (n, sum)) in rows {
        println!("  selector {sel:#04x}: {n} sprites, mean {:.1}%", sum / n as f64 * 100.0);
    }
}
