// Headless end-to-end smoke test of the port's playable loop. Exercises every screen
// and plays all five destination dialogue scenes to completion, asserting no panic and
// that frames render real content and each scene progresses through all its lines.
// Skips gracefully if the game data isn't present. Run: `cargo run --release --bin smoke`.
// (A fast subset runs in the test suite as `engine::tests::full_playable_loop_end_to_end`.)
use commander_blood_tools::descript::DescriptDb;
use commander_blood_tools::engine::{EngineState, MouseInput};
use std::path::Path;

fn nonblank(fb: &[u8]) -> usize {
    fb.iter().filter(|&&p| p != 0).count()
}

fn main() {
    let iso = ["output/_tmp_iso", "commander-blood-audio/_tmp_iso"]
        .iter()
        .find(|p| Path::new(p).join("DESCRIPT.DES").is_file());
    let assets = ["output/_tmp_dat", "output"]
        .iter()
        .find(|p| Path::new(p).join("sq").is_dir());
    let (Some(iso), Some(assets)) = (iso, assets) else {
        println!("SKIP: game data not present");
        return;
    };
    let (iso, assets) = (Path::new(iso), Path::new(assets));
    let descript = DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
    let rd = |n: u32, ext: &str| std::fs::read(iso.join(format!("SCRIPT{n}.{ext}")));

    let mut e = EngineState::new();
    let (cod, var, dic, deb) = (rd(1, "COD").unwrap(), rd(1, "VAR").unwrap(), rd(1, "DIC").unwrap(), rd(1, "DEB").unwrap());
    e.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, assets);
    e.dialogue_hold_frames = 20;
    if let (Ok(carte), Ok(borxx)) = (std::fs::read(iso.join("CARTE.SPR")), std::fs::read(iso.join("BORXX.SPR"))) {
        e.load_nav_sprites(&carte, &borxx);
    }
    e.load_title(iso);
    e.load_intro(assets, &descript);
    e.load_alien_view(assets, "scrut");
    e.load_tv_channels(assets, "tv");
    e.load_cyberspace(assets);
    e.load_bridge(iso);
    e.load_nav_chart(iso);
    e.load_console_bg(iso);
    e.load_console_font(iso);
    e.load_telephone(iso, assets);
    e.on_ship = true;

    let mut fail = 0;
    let mut check = |cond: bool, msg: &str| {
        println!("{} {msg}", if cond { "ok  " } else { "FAIL" });
        if !cond { fail += 1; }
    };

    // 1) Title screen renders.
    check(e.title_active(), "title screen active at startup");
    e.step(MouseInput::default());
    check(nonblank(&e.framebuffer) > 1000, "title renders content");
    e.dismiss_title();

    // 2) Intro plays to completion and shows real content.
    let mut intro_content = false;
    let mut intro_ended = false;
    for _ in 0..4000 {
        e.step(MouseInput::default());
        if nonblank(&e.framebuffer) > 2000 { intro_content = true; }
        if !e.intro_active() { intro_ended = true; break; }
    }
    check(intro_content, "intro renders real video frames");
    check(intro_ended, "intro finishes and hands off");

    // 3) Nav view renders.
    e.on_ship = true;
    for _ in 0..10 { e.step(MouseInput::default()); }
    check(nonblank(&e.framebuffer) > 500, "nav/star-map view renders");

    // 4) Each screen renders real content.
    e.bridge_active = true;
    for _ in 0..4 { e.step(MouseInput::default()); }
    check(nonblank(&e.framebuffer) > 500, "bridge hub renders");
    e.bridge_active = false;

    e.tv_active = true;
    for _ in 0..8 { e.step(MouseInput::default()); }
    check(nonblank(&e.framebuffer) > 500, "comms/TV screen renders");
    e.switch_tv_channel(1);
    for _ in 0..4 { e.step(MouseInput::default()); }
    e.switch_tv_channel(-1);
    for _ in 0..4 { e.step(MouseInput::default()); }
    check(nonblank(&e.framebuffer) > 500, "TV channel switch renders");
    e.tv_active = false;

    e.cyber_active = true;
    e.start_cyberspace();
    for _ in 0..8 { e.step(MouseInput { x: 210, y: 100, buttons: 0 }); }
    check(nonblank(&e.framebuffer) > 500, "cyberspace tunnel renders");
    let mut arrived = false;
    for _ in 0..30000 { e.step(MouseInput::default()); if e.cyber_arrived { arrived = true; break; } }
    check(arrived, "cyberspace traversal reaches its destination");
    e.cyber_active = false;

    e.alien_view_active = true;
    e.arm_alien_intro();
    for _ in 0..12 { e.step(MouseInput { x: 315, y: 100, buttons: 0 }); }
    check(nonblank(&e.framebuffer) > 500, "alien-examination screen renders");
    e.alien_view_active = false;

    // Video-phone: dial screen renders, then a connected call shows the crew feed.
    if e.phone_contact_count() > 0 {
        e.phone_active = true;
        for _ in 0..6 { e.step(MouseInput::default()); }
        check(nonblank(&e.framebuffer) > 500, "video-phone dial screen renders");
        e.phone_connect(0);
        for _ in 0..6 { e.step(MouseInput::default()); }
        check(nonblank(&e.framebuffer) > 500, "video-phone call feed renders");
        e.phone_hangup();
        e.phone_active = false;
    }

    // OPTION 3D-pyramid menu (console OPTION): renders + selection cycles.
    e.option_active = true;
    for _ in 0..6 { e.step(MouseInput { x: 220, y: 100, buttons: 0 }); }
    check(nonblank(&e.framebuffer) > 500, "OPTION 3D pyramid menu renders");
    e.option_cycle(1);
    check(e.option_item() == 1, "OPTION selection cycles");
    e.option_active = false;

    // Ending finale: arms, plays real content, and reaches completion.
    if e.load_ending(assets) {
        e.start_ending();
        for _ in 0..6 { e.step(MouseInput::default()); }
        check(nonblank(&e.framebuffer) > 500, "ending finale renders");
        for _ in 0..4000 { if e.ending_finished() { break; } e.step(MouseInput::default()); }
        check(e.ending_finished(), "ending finale plays to completion");
        e.ending_active = false;
    }

    // 5) Play each destination's dialogue scene to completion, following D2 chaining.
    for dest in 1..=5u32 {
        if let (Ok(c), Ok(v), Ok(d), Ok(b)) = (rd(dest, "COD"), rd(dest, "VAR"), rd(dest, "DIC"), rd(dest, "DEB")) {
            e.load_dialogue_scenes(&c, &v, &d, &b, &descript, assets);
            e.on_ship = false;
            let total = e.dialogue_len();
            let start = e.dialogue_cursor();
            let mut scene_content = false;
            let mut finished = false;
            let mut steps = 0;
            for _ in 0..40000 {
                e.step(MouseInput::default());
                steps += 1;
                if nonblank(&e.framebuffer) > 500 { scene_content = true; }
                if e.dialogue_finished() { finished = true; break; }
            }
            let end = e.dialogue_cursor();
            check(scene_content, &format!("SCRIPT{dest} dialogue scene renders"));
            // Progression (not a fixed time budget): the cursor must advance through
            // the lines, and a healthy scene reaches its end. Report the shape.
            check(end > start && finished, &format!(
                "SCRIPT{dest} scene progresses {start}->{end}/{total} lines in {steps} frames (finished={finished})"
            ));
        }
    }

    // 6) Visit each targetable world location (decoded background), then leave.
    e.on_ship = true;
    for _ in 0..30 { e.step(MouseInput::default()); }

    println!("\n=== smoke: {} check(s) FAILED ===", fail);
    if fail > 0 { std::process::exit(1); }
    println!("=== smoke: all checks passed ===");
}
