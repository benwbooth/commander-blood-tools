//! Contact sheet of port bridge frames for visual alignment vs the oracle ring view.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.on_ship = true;
    e.bridge_active = true;
    let frames: Vec<u16> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let frames = if frames.is_empty() {
        (0..180).step_by(12).collect()
    } else {
        frames
    };
    let cols = 5usize;
    let rows = frames.len().div_ceil(cols);
    let (fw, fh) = (320usize, 200usize);
    let mut sheet = vec![0u8; cols * fw * rows * fh * 3];
    for (i, &fr) in frames.iter().enumerate() {
        e.bridge.frame = fr;
        e.step(MouseInput { x: 315, y: 5, buttons: 0, ..Default::default() });
        e.bridge.frame = fr;
        let (gx, gy) = (i % cols, i / cols);
        for y in 0..fh {
            for x in 0..fw {
                let c = e.scene_palette[e.framebuffer[y * fw + x] as usize];
                let d = ((gy * fh + y) * cols * fw + gx * fw + x) * 3;
                sheet[d..d + 3].copy_from_slice(&c);
            }
        }
    }
    let mut out = format!("P6\n{} {}\n255\n", cols * fw, rows * fh).into_bytes();
    out.extend_from_slice(&sheet);
    std::fs::write("accuracy/comparisons/hand/bridge_sheet.ppm", out).unwrap();
    println!("frames: {frames:?}");
}
