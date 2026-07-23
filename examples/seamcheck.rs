//! Zoomed seam check: render the port hand at (160,88) — the oracle_hand_160_88
//! capture's cursor — and write a 3x side-by-side crop of the hand region so the
//! seam-face material (fold decode) can be compared against the oracle.

use commander_blood_tools::manu3_hand::HandMesh;
use commander_blood_tools::palette::game_screen_palette;

fn main() {
    let mesh = HandMesh::load();
    let pal = game_screen_palette();
    let mut fb = vec![0u8; 320 * 200];
    mesh.draw(&mut fb, 320, 200, 160, 88);
    // oracle capture: 640x400 png already exists; here write the port side.
    let (x0, y0, cw, ch) = (100usize, 60usize, 140usize, 130usize);
    let z = 3usize;
    let mut out = format!("P6\n{} {}\n255\n", cw * z, ch * z).into_bytes();
    for y in 0..ch * z {
        for x in 0..cw * z {
            let c = pal[fb[(y0 + y / z) * 320 + (x0 + x / z)] as usize];
            out.extend_from_slice(&c);
        }
    }
    std::fs::write("accuracy/comparisons/hand/seam_port.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/seam_port.ppm (crop {cw}x{ch} @({x0},{y0}) x{z})");
}
