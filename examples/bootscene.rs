//! Headless SCRIPT1 boot-presentation render: drive the tutorial through the engine
//! (console-band mode, as the windowed driver does) and dump frames whenever the
//! subtitle changes — for side-by-side comparison with the interpreter's BOOTIDX
//! captures (bd_218M / bd_290M).

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let assets = Path::new("output/_tmp_dat");
    let db = commander_blood_tools::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES"))
        .unwrap();
    let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT1.{ext}"))).unwrap();
    let mut e = EngineState::new();
    e.load_bridge(iso); // fonts (bold console) + panorama, as the windowed driver has
    e.load_dialogue_scenes(&rd("COD"), &rd("VAR"), &rd("DIC"), &rd("DEB"), &db, assets);
    e.load_console_font(iso);
    e.dialogue_hold_frames = 20;
    e.on_ship = false;
    e.set_console_band_dialogue(true);
    let mut last = String::new();
    let mut shot = 0usize;
    for tick in 0..6000 {
        e.step(MouseInput { x: 300, y: 190, ..Default::default() });
        let cur = e.current_subtitle().unwrap_or("").to_string();
        if cur != last && !cur.is_empty() && tick > 0 {
            // capture mid-reveal (a few chars in) — enough for layout comparison
            for _ in 0..10 {
                e.step(MouseInput { x: 300, y: 190, ..Default::default() });
            }
            let mut ppm = b"P6\n320 200\n255\n".to_vec();
            for &px in e.framebuffer.iter() {
                ppm.extend_from_slice(&e.scene_palette[px as usize]);
            }
            std::fs::write(format!("accuracy/comparisons/hand/ps_{shot:02}.ppm"), ppm)
                .unwrap();
            println!("shot {shot}: {:.60}", cur.replace('\n', " / "));
            shot += 1;
            last = e.current_subtitle().unwrap_or("").to_string();
            if shot >= 8 {
                break;
            }
        } else if cur != last {
            last = cur;
        }
        if e.dialogue_finished() {
            break;
        }
    }
    println!("done ({shot} shots)");
}
