//! Bank the port-side PLANET world screens (arrow 6's port reference set): visit
//! each primary world, render its rooms with entities + the candidate box, dump PPMs.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let assets = Path::new("output/_tmp_dat");
    std::fs::create_dir_all("accuracy/comparisons/planets").unwrap();
    for world in ["corpo", "pterra", "venusia", "ekatomb", "kortex", "cyber"] {
        let mut e = EngineState::new();
        e.load_bridge(iso);
        if !e.visit_world(world, assets) {
            println!("{world}: no world data");
            continue;
        }
        if let Ok(ext) = std::fs::read(iso.join(format!("{}.EXT", world.to_uppercase()))) {
            e.set_world_ext(&ext);
        }
        e.step(MouseInput { x: 160, y: 100, ..Default::default() });
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for &px in e.framebuffer.iter() {
            ppm.extend_from_slice(&e.scene_palette[px as usize]);
        }
        std::fs::write(format!("accuracy/comparisons/planets/{world}.ppm"), ppm).unwrap();
        println!("{world}: banked");
    }
}
