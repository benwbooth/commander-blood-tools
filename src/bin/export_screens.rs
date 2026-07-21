// Visual-QA export: renders every port screen to a PPM (P6) image so the port's
// rendering can be eyeballed headlessly. Zero-dependency (PPM is a trivial raw format);
// convert to PNG with e.g. `magick out/qa_bridge.ppm out/qa_bridge.png`.
//
// Usage: `cargo run --release --bin export_screens -- <output_dir>`
// Skips gracefully if the game data isn't present.
//
// This is the reproducible form of the 2026-07-21 visual-QA pass that confirmed every
// screen (bridge/TV/cyberspace/cryobox/alien/dialogue/nav + the new choose-a-location
// nav, video-phone, and ending finale) renders faithfully with correct palettes.
use commander_blood_tools::descript::DescriptDb;
use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

/// Write the engine's current 320x200 indexed framebuffer, resolved through its scene
/// palette, as a binary PPM (P6) image.
fn dump(e: &EngineState, dir: &Path, name: &str) {
    let mut buf = Vec::with_capacity(320 * 200 * 3 + 16);
    buf.extend_from_slice(b"P6\n320 200\n255\n");
    for &idx in &e.framebuffer {
        let c = e.scene_palette[idx as usize];
        buf.extend_from_slice(&c);
    }
    let path = dir.join(format!("{name}.ppm"));
    if let Err(err) = std::fs::write(&path, buf) {
        eprintln!("write {}: {err}", path.display());
    } else {
        println!("wrote {}", path.display());
    }
}

fn main() {
    let dir = match std::env::args().nth(1) {
        Some(d) => std::path::PathBuf::from(d),
        None => {
            eprintln!("usage: export_screens <output_dir>");
            std::process::exit(2);
        }
    };
    let _ = std::fs::create_dir_all(&dir);

    let iso = ["output/_tmp_iso", "commander-blood-audio/_tmp_iso"]
        .iter()
        .map(Path::new)
        .find(|p| p.join("DESCRIPT.DES").is_file());
    let assets = ["output/_tmp_dat", "output"]
        .iter()
        .map(Path::new)
        .find(|p| p.join("sq").is_dir());
    let (Some(iso), Some(assets)) = (iso, assets) else {
        println!("SKIP: game data not present");
        return;
    };
    let descript = DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
    let rd = |n: u32, ext: &str| std::fs::read(iso.join(format!("SCRIPT{n}.{ext}")));

    let mut e = EngineState::new();
    e.dialogue_hold_frames = 20;
    let (c, v, d, b) = (
        rd(2, "COD").unwrap(),
        rd(2, "VAR").unwrap(),
        rd(2, "DIC").unwrap(),
        rd(2, "DEB").unwrap(),
    );
    e.load_dialogue_scenes(&c, &v, &d, &b, &descript, assets);
    if let (Ok(carte), Ok(borxx)) = (
        std::fs::read(iso.join("CARTE.SPR")),
        std::fs::read(iso.join("BORXX.SPR")),
    ) {
        e.load_nav_sprites(&carte, &borxx);
    }
    e.load_title(iso);
    e.load_intro(assets, &descript);
    e.load_alien_view(assets, "scrut");
    e.load_tv_channels(assets, "tv");
    e.load_cyberspace(assets);
    e.load_bridge(iso);
    e.load_nav_chart(iso);
    e.load_console_font(iso);
    e.load_cryobox(assets);
    e.load_telephone(iso, assets);
    e.load_ending(assets);

    // Title art.
    e.step(MouseInput::default());
    dump(&e, &dir, "qa_title");
    e.dismiss_title();

    // Fully play out the startup intro (its frames render with precedence), then grab one.
    let mut intro_frames = 0;
    while e.intro_active() && intro_frames < 200_000 {
        e.step(MouseInput::default());
        if intro_frames == 20 {
            dump(&e, &dir, "qa_intro");
        }
        intro_frames += 1;
    }

    // Bridge console (with the 5-item menu).
    e.on_ship = false;
    e.bridge_active = true;
    for _ in 0..3 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_bridge");
    e.bridge_active = false;

    // Comms / Hate TV (two channels).
    e.tv_active = true;
    for _ in 0..8 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_tv0");
    e.switch_tv_channel(1);
    for _ in 0..8 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_tv1");
    e.tv_active = false;

    // Cyberspace tunnel.
    e.cyber_active = true;
    for _ in 0..10 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_cyber");
    e.cyber_active = false;

    // Cryo-chamber (console CRYOBOX).
    e.cryobox_active = true;
    for _ in 0..10 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_cryobox");
    e.cryobox_active = false;

    // Alien-examination (scrutinizer apparatus).
    e.alien_view_active = true;
    e.arm_alien_intro();
    for _ in 0..16 {
        e.step(MouseInput { x: 315, y: 100, buttons: 0 });
    }
    dump(&e, &dir, "qa_alien");
    e.alien_view_active = false;

    // Video-phone (console TELEPHONE): dial screen, then a connected call feed.
    if e.phone_contact_count() > 0 {
        e.phone_active = true;
        for _ in 0..4 {
            e.step(MouseInput::default());
        }
        dump(&e, &dir, "qa_phone_dial");
        e.phone_connect(0);
        for _ in 0..4 {
            e.step(MouseInput::default());
        }
        dump(&e, &dir, "qa_phone_call");
        e.phone_hangup();
        e.phone_active = false;
    }

    // A dialogue scene (subtitle over its talk-HNM background).
    e.on_ship = false;
    for _ in 0..200 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_dialogue");

    // Nav star-map (plain, with the targeted-world label).
    e.on_ship = true;
    for _ in 0..3 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_nav");

    // OPTION 3D-pyramid menu (console OPTION).
    e.option_active = true;
    for _ in 0..8 {
        e.step(MouseInput { x: 220, y: 100, buttons: 0 });
    }
    dump(&e, &dir, "qa_option");
    e.option_active = false;

    // Ending finale (a frame partway in).
    e.on_ship = false;
    e.start_ending();
    for _ in 0..30 {
        e.step(MouseInput::default());
    }
    dump(&e, &dir, "qa_ending");
    e.ending_active = false;

    println!("done — convert with: magick {}/qa_*.ppm out.png", dir.display());
}
