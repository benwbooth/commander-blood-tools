//! Probe: step the intro like the driver does and report when the console band
//! (indices >= 224 in rows 140..200) is present, per intro clip index.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let db = commander_blood_tools::descript::DescriptDb::parse_file(Path::new(
        "output/_tmp_iso/DESCRIPT.DES",
    ))
    .unwrap();
    let assets = Path::new("output/_tmp_dat");
    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_intro(assets, &db);
    // Mirror run_engine_window's full loader sequence (bisect: one of these was
    // suspected of clobbering the intro band state).
    e.load_tv_programs(&db, assets);
    e.load_tv_channels(assets, "tv");
    e.load_cyberspace(assets);
    e.load_bridge(iso);
    e.load_hand_atlas(Path::new("accuracy/captures/bridge/hand"));
    e.load_nav_chart(iso);
    e.load_console_font(iso);
    e.load_cryobox(assets);
    e.load_telephone(iso, assets);
    e.load_ending(assets);
    let mut tick = 0usize;
    let mut last_idx = usize::MAX;
    while e.intro_active() && tick < 4000 {
        e.step(MouseInput::default());
        let idx = e.intro_index();
        let band = e.framebuffer[320 * 140..]
            .iter()
            .filter(|&&p| p >= 224)
            .count();
        if idx != last_idx || (tick % 400 == 0) {
            println!("tick {tick}: clip {idx} band_px {band}");
            last_idx = idx;
        }
        if tick == 500 {
            let mut ppm = b"P6\n320 200\n255\n".to_vec();
            for &px in e.framebuffer.iter() {
                ppm.extend_from_slice(&e.scene_palette[px as usize]);
            }
            std::fs::write("accuracy/comparisons/hand/introband_500.ppm", ppm).unwrap();
        }
        tick += 1;
    }
    println!("intro ended at tick {tick}");
}
