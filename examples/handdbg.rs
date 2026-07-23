//! Debug: projected point cloud stats at a neutral cursor — where does vertex 34 land
//! vs the rest, per node?

use commander_blood_tools::manu3_hand::HandMesh;

fn main() {
    let m = HandMesh::load();
    let (cx, cy) = (160, 100);
    let pts = m.debug_project(cx, cy);
    let counts = [14usize, 7, 8, 6, 7, 6, 6, 7, 5, 5, 8, 6, 6, 5, 4, 8];
    let mut vi = 0;
    for (ni, &c) in counts.iter().enumerate() {
        let slice = &pts[vi..vi + c];
        let ys: Vec<i32> = slice.iter().map(|p| p.1 as i32).collect();
        let xs: Vec<i32> = slice.iter().map(|p| p.0 as i32).collect();
        println!(
            "node {ni:2}: x {:?}..{:?} y {:?}..{:?}",
            xs.iter().min(),
            xs.iter().max(),
            ys.iter().min(),
            ys.iter().max()
        );
        vi += c;
    }
    println!("tip pt[34] = {:?}", pts[34]);
}
