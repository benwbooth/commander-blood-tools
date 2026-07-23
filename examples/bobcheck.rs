//! Render the port's BOB_MORLOCK contact screen for comparison with vs_005..007.

use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let mut e = EngineState::new();
    e.load_bridge(iso);
    e.bob_contact_active = true;
    e.set_speech_dialogue(vec![(
        "HONK! You worthless heap of wires... Are  \nyou working?".into(),
        None,
    )]);
    e.load_bob_contact(iso, Path::new("output/_tmp_dat"));
    e.step(MouseInput { x: 120, y: 130, ..Default::default() });
    let mut ppm = b"P6\n320 200\n255\n".to_vec();
    for &px in e.framebuffer.iter() {
        ppm.extend_from_slice(&e.scene_palette[px as usize]);
    }
    std::fs::write("accuracy/comparisons/hand/bob_port.ppm", ppm).unwrap();
    println!("wrote accuracy/comparisons/hand/bob_port.ppm");
}
