//! Drive the record-driven nav chart + info panel exactly as the app does and
//! dump the resulting frame, so the wiring is checked against a real render
//! rather than only against unit fixtures.
use commander_blood_tools::engine::{EngineState, NavChartObject};
use commander_blood_tools::vm::VmMachine;
use std::path::Path;

fn status_headers() -> commander_blood_tools::vm::StatusHeaders {
    // The game's own strings, read from the image (DS:0x12E/0x137/0x13E/0x14B).
    let b = commander_blood_tools::bloodprg::BloodPrg::parse_file("re/bin/BLOODPRG.EXE")
        .expect("BLOODPRG.EXE");
    let h = b.location_status_headers();
    commander_blood_tools::vm::StatusHeaders {
        planet: h[0].clone(),
        ship: h[1].clone(),
        black_hole: h[2].clone(),
        life_support: h[3].clone(),
    }
}

fn main() {
    let iso = Path::new("output/_tmp_iso");
    let mut m = VmMachine::new();
    m.load_var(&std::fs::read(iso.join("SCRIPT5.VAR")).unwrap());
    m.load_deb_objects(&std::fs::read(iso.join("SCRIPT5.DEB")).unwrap());
    let context = m.nav_chart_arche_context();
    let objects: Vec<NavChartObject> = m
        .build_nav_chart_list()
        .into_iter()
        .map(|object| {
            let name = m.object_inline_name(object);
            NavChartObject {
                object,
                kind: m.rec_read_pub(object),
                marker: m.nav_chart_marker(object, context),
                art_id: commander_blood_tools::levels::world_art_resource_id(&name),
                name,
            }
        })
        .collect();
    println!("chart objects: {objects:?}");

    let mut e = EngineState::new();
    e.load_nav_chart(iso);
    e.on_ship = true;
    e.set_nav_chart_objects(objects.clone());
    e.render_ship_view();

    let target = objects[0].marker;
    let opened = e.nav_chart_click(target.0 + 1, target.1 + 1, 0, |o| m.location_panel_rows(o, &status_headers()));
    println!("click at {target:?} -> opened {opened}, state {:?}", e.location_panel.state);
    for i in 0..12 {
        e.render_ship_view();
        if i == 3 || i == 11 {
            let ppm_path = format!("accuracy/comparisons/navpanel_{i}.ppm");
            let mut ppm = b"P6\n320 200\n255\n".to_vec();
            for &px in e.framebuffer.iter() {
                ppm.extend_from_slice(&e.scene_palette[px as usize]);
            }
            let _ = std::fs::create_dir_all("accuracy/comparisons");
            std::fs::write(&ppm_path, ppm).unwrap();
            println!("frame {i}: state {:?} -> {ppm_path}", e.location_panel.state);
        }
    }
    println!("panel rows: {:?}", m.location_panel_rows(objects[0].object, &status_headers()));
}
