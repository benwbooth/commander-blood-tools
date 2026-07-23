//! Render the manu3 hand in each pose selector via the SOFTWARE path (draw) at
//! cursor (160,100) — a contact sheet to spot contorted/flipped poses.

use commander_blood_tools::manu3_hand::HandMesh;
use commander_blood_tools::palette::game_screen_palette;

fn main() {
    let pal = game_screen_palette();
    let poses: Vec<u16> = vec![1, 2, 3, 4, 6, 0x10];
    let (tw, th) = (320usize, 200usize);
    let cols = 3usize;
    let rows = poses.len().div_ceil(cols);
    let mut sheet = vec![0u8; tw * cols * th * rows * 3];
    for (pi, &sel) in poses.iter().enumerate() {
        let mut mesh = HandMesh::load();
        mesh.set_pose(sel);
        // settle the tween
        for _ in 0..30 {
            mesh.tick_pose();
        }
        let mut fb = vec![0u8; tw * th];
        mesh.draw(&mut fb, tw, th, 160, 100);
        // ALSO overlay the triangles() bounding box as a sanity check that the
        // GPU geometry matches: mark tri vertices with palette index 255.
        for t in mesh.triangles(160, 100) {
            for v in t {
                let (x, y) = (v[0] as i32, v[1] as i32);
                if (0..tw as i32).contains(&x) && (0..th as i32).contains(&y) {
                    let i = y as usize * tw + x as usize;
                    if fb[i] == 0 {
                        fb[i] = 254;
                    }
                }
            }
        }
        let (gx, gy) = (pi % cols, pi / cols);
        for y in 0..th {
            for x in 0..tw {
                let c = pal[fb[y * tw + x] as usize];
                let di = ((gy * th + y) * tw * cols + gx * tw + x) * 3;
                sheet[di..di + 3].copy_from_slice(&c);
            }
        }
        // label: print tip info
        let pts = mesh.debug_project(160, 100);
        let tip = pts[34];
        println!("pose {sel:#x}: tip=({:.0},{:.0}) verts_on_screen={}", tip.0, tip.1,
            pts.iter().filter(|p| p.0 >= 0.0 && p.0 < 320.0 && p.1 >= 0.0 && p.1 < 200.0).count());
    }
    let mut out = format!("P6\n{} {}\n255\n", tw * cols, th * rows).into_bytes();
    out.extend_from_slice(&sheet);
    std::fs::write("accuracy/comparisons/hand/posecheck.ppm", out).unwrap();
    println!("wrote accuracy/comparisons/hand/posecheck.ppm");
}
