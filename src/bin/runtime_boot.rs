//! Boot the real BLOODPRG.EXE inside the path-B runtime (recomp::runtime) and capture frames.
//!
//! Usage: runtime_boot [--steps N] [--shot-every N] [--out DIR] [--trace]
//! Environment mirrors the DOSBox-X oracle: C: = accuracy/cdrive (writable, game saves in
//! C:\cblood\), D: = output/_tmp_iso (the CD), CWD = D:\, launch args from BLOOD.BAT.

use commander_blood_tools::recomp::runtime::{RunEnd, Runtime};
use std::path::PathBuf;

fn main() {
    let mut steps: u64 = 400_000_000;
    let mut shot_every: u64 = 10_000_000;
    let mut out = PathBuf::from("boot_frames");
    let mut trace = false;
    let mut lockstep: Option<(u64, u64, PathBuf)> = None;
    let mut resume: Option<PathBuf> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--lockstep" => {
                let skip = args[i + 1].parse().unwrap();
                let window = args[i + 2].parse().unwrap();
                let path = PathBuf::from(&args[i + 3]);
                lockstep = Some((skip, window, path));
                i += 3;
            }
            "--steps" => {
                i += 1;
                steps = args[i].parse().unwrap();
            }
            "--shot-every" => {
                i += 1;
                shot_every = args[i].parse().unwrap();
            }
            "--out" => {
                i += 1;
                out = PathBuf::from(&args[i]);
            }
            "--trace" => trace = true,
            "--resume" => {
                i += 1;
                resume = Some(PathBuf::from(&args[i]));
            }
            a => {
                eprintln!("unknown arg {a}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let c_root = PathBuf::from("accuracy/cdrive");
    let d_root = PathBuf::from("output/_tmp_iso");
    std::fs::create_dir_all(c_root.join("cblood")).unwrap(); // BLOOD.BAT does `md cblood`
    std::fs::create_dir_all(&out).unwrap();
    let exe = std::fs::read(d_root.join("BLOODPRG.EXE")).expect("D:\\BLOODPRG.EXE");

    let mut rt = Runtime::new(c_root, d_root);
    rt.trace_ints = trace;
    rt.load_exe(&exe, " AMR S162227 EMS WRIC:\\cblood\\", "D:\\BLOODPRG.EXE")
        .unwrap();
    if let Some(state) = resume {
        rt.load_state(&state).expect("load savestate");
        eprintln!("resumed from {} @ {} steps", state.display(), rt.cpu.steps);
    }

    if let Some((skip, window, path)) = lockstep {
        eprintln!("lockstep: skip={skip} window={window} -> {}", path.display());
        rt.lockstep_capture(skip, window, &path).unwrap();
        eprintln!("lockstep capture done ({} steps reached)", rt.cpu.steps);
        return;
    }

    if let Ok(w) = std::env::var("READSTR") {
        // Run to `steps`, then print the ASCII string at gs:<off> (default 0xe18 subtitle buffer).
        let off: u32 = u32::from_str_radix(w.trim_start_matches("0x"), 16).unwrap_or(0xe18);
        let _ = rt.run(steps);
        let g = 0x0e84u16;
        let mut s = String::new();
        for i in 0..80 {
            let b = rt.m.read8(g, off + i);
            if b == 0 {
                break;
            }
            s.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        println!("@{} gs:{off:#06x} = {s:?}", rt.cpu.steps);
        return;
    }

    if std::env::var("INTROTRACE").is_ok() {
        // FAITHFULNESS ORACLE for the boot/intro: boot the real game with NO input injection (so
        // the intro plays naturally) and merge the file-opens with the SB audio-playback starts
        // into one step-stamped timeline — the ground truth the port's early sequence is diffed
        // against ("what asset loads / audio starts, and when"). Frames every 1M steps for the eye.
        let limit: u64 = std::env::var("STEPS").ok().and_then(|s| s.parse().ok()).unwrap_or(40_000_000);
        let mut next_shot = 0u64;
        // DISMISS_AT=<steps>: inject one Enter there (dismiss the title so the
        // tutorial's scripted sequence plays hands-off — the event-order oracle).
        let dismiss: Option<u64> = std::env::var("DISMISS_AT").ok().and_then(|v| v.parse().ok());
        let mut dismissed = false;
        while rt.cpu.steps < limit {
            let mut target = next_shot.min(limit);
            if let (Some(d), false) = (dismiss, dismissed) {
                if rt.cpu.steps < d {
                    target = target.min(d);
                } else {
                    // Click the EYE-ORB in the bottom band (screen ~(160,170)) — the
                    // attract frames all show it; it is the "commander arrives" target.
                    // NO Esc (Esc cancels the narration we want to trace).
                    for _ in 0..3 {
                        rt.set_mouse_pos(160, 170);
                        rt.mouse_press(0);
                        let _ = rt.run(rt.cpu.steps + 300_000);
                        rt.mouse_release(0);
                        let _ = rt.run(rt.cpu.steps + 2_000_000);
                    }
                    dismissed = true;
                    eprintln!("(gate clicked @ {})", rt.cpu.steps);
                }
            }
            let _ = rt.run(target);
            if rt.cpu.steps >= next_shot {
                let m = rt.cpu.steps / 1_000_000;
                let _ = rt.write_ppm(&out.join(format!("intro_{m:03}M.ppm")));
                next_shot += 1_000_000;
            }
        }
        enum Ev { Open(String), Audio(u32, u32) }
        let mut tl: Vec<(u64, Ev)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (step, path) in &rt.opened_files {
            if seen.insert(path.clone()) { tl.push((*step, Ev::Open(path.clone()))); }
        }
        for (step, len, rate) in &rt.sb_play_log {
            tl.push((*step, Ev::Audio(*len, *rate)));
        }
        tl.sort_by_key(|(s, _)| *s);
        println!("--- REAL-GAME BOOT/INTRO TIMELINE (step: event), to {limit} steps ---");
        for (step, ev) in &tl {
            match ev {
                Ev::Open(p) => println!("  @{step:>10}  OPEN   {p}"),
                Ev::Audio(len, rate) => println!("  @{step:>10}  AUDIO  play {len}B @ {rate}Hz  <== sound starts"),
            }
        }
        println!("total audio-play events: {}", rt.sb_play_log.len());
        return;
    }

    if std::env::var("SKIPPROBE").is_ok() {
        // Inject input periodically from early on and capture frames, to find the EARLIEST
        // step at which interactive gameplay (bridge/menu) appears — i.e. can we skip the
        // long intro? Captures boot_skip/skip_<M>.ppm every 15M steps to 300M.
        let mut next_shot = 15_000_000u64;
        let mut next_input = 5_000_000u64;
        let limit = 90_000_000u64;
        while rt.cpu.steps < limit {
            let target = next_shot.min(next_input).min(limit);
            let _ = rt.run(target);
            if rt.cpu.steps >= next_input {
                // press any-key + click to try to dismiss/skip the current intro screen
                rt.inject_key(0x01, 0x1b); // Esc
                rt.inject_key(0x1c, 0x0d); // Enter
                rt.inject_key(0x39, 0x20); // Space
                rt.set_mouse_pos(320, 100);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 500_000);
                rt.mouse_release(0);
                next_input += 5_000_000;
            }
            if rt.cpu.steps >= next_shot {
                let m = rt.cpu.steps / 1_000_000;
                rt.write_ppm(&out.join(format!("skip_{m:05}M.ppm"))).unwrap();
                next_shot += 15_000_000;
            }
        }
        // Dump the nav runtime region at this interactive state for offline analysis.
        let g = 0x0e84u16;
        let bytes: Vec<u8> = (0..8448u32).map(|i| rt.m.read8(g, 0x2f00 + i)).collect();
        std::fs::write(out.join("skip_navstate.bin"), &bytes).unwrap();
        // Which files did the game open? (deduped, in order) — identifies the screen's assets.
        println!("--- opened files (deduped) ---");
        let mut seen = std::collections::HashSet::new();
        for (step, path) in &rt.opened_files {
            if seen.insert(path.clone()) {
                println!("  @{:>10} {path}", step);
            }
        }
        println!("SKIPPROBE done -> {}/skip_*.ppm + skip_navstate.bin @ {} steps", out.display(), rt.cpu.steps);
        return;
    }

    if let Ok(spec) = std::env::var("CLICKAT") {
        // Skip to the console, then click a sequence of "sx,sy;sx,sy;..." positions (with
        // Esc between), capturing a frame + opened-files after each. Finds which button
        // opens a given screen (e.g. the map/chart).
        let mut next_input = 5_000_000u64;
        while rt.cpu.steps < 45_000_000 {
            let _ = rt.run(next_input);
            rt.inject_key(0x01, 0x1b);
            rt.inject_key(0x1c, 0x0d);
            rt.set_mouse_pos(320, 100);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            next_input += 5_000_000;
        }
        // Optional idle settle before the click sequence (IDLE=<steps>): lets any
        // running presentation finish so clicks dispatch from the IDLE console.
        if let Ok(idle) = std::env::var("IDLE") {
            let extra: u64 = idle.parse().unwrap_or(40_000_000);
            let _ = rt.run(rt.cpu.steps + extra);
        }
        let before = rt.opened_files.len();
        for (i, pt) in spec.split(';').enumerate() {
            let (a, b) = pt.split_once(',').unwrap();
            let (sx, sy): (u16, u16) = (a.trim().parse().unwrap(), b.trim().parse().unwrap());
            // The TUTORIAL mode's cadence (which achieved dispatch): move, 150K, press,
            // 150K, release, then a longer settle for any screen build.
            rt.set_mouse_pos(sx * 2, sy);
            let _ = rt.run(rt.cpu.steps + 150_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 150_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 5_000_000);
            rt.write_ppm(&out.join(format!("click_{i:02}_{sx}_{sy}.ppm"))).unwrap();
            println!("click ({sx},{sy}): opened since start:");
            for (_, p) in rt.opened_files.iter().skip(before) {
                println!("    {p}");
            }
        }
        println!("CLICKAT done -> {}/click_*.ppm", out.display());
        return;
    }

    if std::env::var("EXPLORE").is_ok() {
        // Skip to interactive, then systematically click the console menu rows + corners and
        // press keys over a long run, capturing a frame + the FULL opened-files map. Surfaces
        // which assets many screens load, to guide bulk porting.
        let mut next_input = 5_000_000u64;
        while rt.cpu.steps < 45_000_000 {
            let _ = rt.run(next_input);
            rt.inject_key(0x01, 0x1b);
            rt.inject_key(0x1c, 0x0d);
            rt.set_mouse_pos(320, 100);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            next_input += 5_000_000;
        }
        // Poke each console menu row (and back out) repeatedly, capturing frames.
        let rows = [65u16, 82, 98, 114, 130];
        let mut shot = 0;
        for pass in 0..3 {
            for &sy in &rows {
                rt.set_mouse_pos(160 * 2, sy);
                let _ = rt.run(rt.cpu.steps + 800_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 800_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 4_000_000);
                rt.write_ppm(&out.join(format!("exp_{shot:02}_p{pass}_y{sy}.ppm"))).unwrap();
                shot += 1;
                rt.inject_key(0x01, 0x1b); // Esc back
                let _ = rt.run(rt.cpu.steps + 2_000_000);
            }
        }
        println!("--- ALL opened files (deduped, in order) ---");
        let mut seen = std::collections::HashSet::new();
        for (step, path) in &rt.opened_files {
            if seen.insert(path.to_lowercase()) {
                println!("  @{:>10} {path}", step);
            }
        }
        println!("EXPLORE done -> {}/exp_*.ppm ({} steps)", out.display(), rt.cpu.steps);
        return;
    }

    if std::env::var("MENUMAP").is_ok() {
        // Fast-skip to the ship-console menu, then click each menu-item row and capture the
        // resulting screen, to map the menu -> screen structure.
        if std::env::var("VERTINIT").is_ok() {
            // manu3 data segment lands at 0x17A3; vertex buffers at data:0xE000+.
            rt.m.trace_range = Some(0x17a30 + 0xE000..0x17a30 + 0xF800);
        }
        let mut next_input = 5_000_000u64;
        while rt.cpu.steps < 50_000_000 {
            let _ = rt.run(next_input);
            rt.inject_key(0x01, 0x1b);
            rt.inject_key(0x1c, 0x0d);
            rt.set_mouse_pos(320, 100);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 500_000);
            rt.mouse_release(0);
            next_input += 5_000_000;
        }
        rt.write_ppm(&out.join("menu_00_base.ppm")).unwrap();
        // Menu items are stacked in the console box (screen x~165, y 60..130). Click each,
        // then a corner (to close/return) between probes.
        let rows = [60u16, 78, 95, 112, 128];
        for (i, &sy) in rows.iter().enumerate() {
            rt.set_mouse_pos(165 * 2, sy);
            let _ = rt.run(rt.cpu.steps + 1_000_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 1_000_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 6_000_000);
            rt.write_ppm(&out.join(format!("menu_{:02}_row{sy}.ppm", i + 1))).unwrap();
            // try Esc to back out to the menu before the next probe
            rt.inject_key(0x01, 0x1b);
            let _ = rt.run(rt.cpu.steps + 3_000_000);
        }
        println!("MENUMAP done -> {}/menu_*.ppm", out.display());
        return;
    }

    if std::env::var("INPUTPROBE").is_ok() {
        // Reach the bridge state, snapshot, inject mouse motion + clicks + keys, run on,
        // and report whether the frame / nav data changed (i.e. is it interactive?).
        let reach: u64 = std::env::var("REACH").ok().and_then(|s| s.parse().ok()).unwrap_or(500) * 1_000_000;
        let _ = rt.run(reach);
        let g = 0x0e84u16;
        let snap_frame = rt.m.mem.clone();
        let nav_before: Vec<u8> = (0..88).map(|i| rt.m.read8(g, 0x4f09 + i)).collect();
        rt.write_ppm(&out.join("probe_before.ppm")).unwrap();
        eprintln!("reached {reach} steps; injecting input");
        // Sweep the mouse across the screen and click at several spots; also press a few keys.
        let spots = [(160u16, 100u16), (80, 60), (240, 60), (160, 150), (60, 180), (260, 180)];
        for (i, &(sx, sy)) in spots.iter().enumerate() {
            rt.set_mouse_pos(sx * 2, sy);
            let _ = rt.run(rt.cpu.steps + 2_000_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 1_000_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 2_000_000);
            if i % 2 == 0 {
                rt.inject_key(0x1c, 0x0d); // Enter
                rt.inject_key(0x39, 0x20); // Space
            }
            let _ = rt.run(rt.cpu.steps + 2_000_000);
        }
        let _ = rt.run(rt.cpu.steps + 20_000_000);
        rt.write_ppm(&out.join("probe_after.ppm")).unwrap();
        let nav_after: Vec<u8> = (0..88).map(|i| rt.m.read8(g, 0x4f09 + i)).collect();
        let mem_changed = rt.m.mem.iter().zip(snap_frame.iter()).filter(|(a, b)| a != b).count();
        let nav_changed = nav_before.iter().zip(nav_after.iter()).filter(|(a, b)| a != b).count();
        println!("INPUTPROBE: {} RAM bytes changed since snapshot; {}/88 nav-anchor bytes changed", mem_changed, nav_changed);
        println!("  (frames: probe_before.ppm vs probe_after.ppm in {})", out.display());
        return;
    }

    if std::env::var("TUTORIAL").is_ok() {
        // Fast-skip to the ship console, then click the SCRIPT1 tutorial's indicated
        // target (the centre orb the pointing hand points to) and a spread of console
        // points, watching the subtitle + opened_files, to complete the tutorial and reach
        // SCRIPT2 gameplay — which unblocks OPTION + the interactive systems.
        let mut next_input = 5_000_000u64;
        while rt.cpu.steps < 45_000_000 {
            let _ = rt.run(next_input);
            rt.inject_key(0x01, 0x1b);
            rt.inject_key(0x1c, 0x0d);
            rt.set_mouse_pos(320, 100);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            next_input += 5_000_000;
        }
        // CORRECTLY calibrated from the gridded "Cap'n Bob is waiting" console frame:
        // orange orb (the hand's target, click-to-advance) at (125,118); Cap'n Bob's
        // portrait at (65,110); golden menu at x~230 rows HONK y90 / TELEPHONE y105 /
        // CRYOBOX y120 / MENU y135 / OPTION y150. The tutorial "Click quick" wants the orb,
        // so click it RAPIDLY (short gaps), interleaving the menu buttons.
        // The tutorial teaches each console button in turn ("CLICK QUICK ON 'CRYOBOX'"),
        // so cycle the orb (125,118) + all 5 menu buttons (x~230: HONK y88, TELEPHONE y103,
        // CRYOBOX y118, MENU y133, OPTION y148) + the submenu-option area (175,115, where a
        // {BOB_MORLOCK,CANCEL}-style sub-choice appears) — whatever the current step wants
        // gets clicked, and any sub-choice is dismissed so the tutorial keeps advancing.
        // Cycle the orb (125,118) + all 5 menu buttons (x~230: HONK y88, TELEPHONE y103,
        // CRYOBOX y118, MENU y133, OPTION y148) + the sub-choice area (110,88=BOB_MORLOCK,
        // 115,102=CANCEL) so whatever the current tutorial step wants gets clicked. This
        // advances the SCRIPT1 tutorial dialogue but never triggers a scene transition to
        // SCRIPT2 (the credit-divergence scene-coordinator bug — see re/REVERSE.md).
        let targets = [
            (125u16, 118u16), (230, 88), (230, 103), (230, 118), (230, 133), (230, 148),
            (110, 88), (115, 102),
        ];
        let baseline = rt.opened_files.len();
        let mut reached2 = false;
        for round in 0..250 {
            let (sx, sy) = targets[round % targets.len()];
            rt.set_mouse_pos(sx * 2, sy);
            let _ = rt.run(rt.cpu.steps + 150_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 150_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 1_000_000);
            // A NEW asset beyond the console baseline = a scene transition (tutorial done /
            // a location loaded). Report the moment it happens.
            if rt.opened_files.len() > baseline {
                let newest: Vec<String> = rt.opened_files[baseline..]
                    .iter().map(|(_, p)| p.clone()).collect();
                let has2 = newest.iter().any(|p| p.to_lowercase().contains("script2"));
                println!("round {round:2} NEW files since console: {newest:?} script2={has2}");
                if has2 || round % 10 == 0 {
                    rt.write_ppm(&out.join(format!("tut_r{round:03}.ppm"))).unwrap();
                }
                if has2 {
                    reached2 = true;
                    break;
                }
            } else if round % 20 == 0 {
                println!("round {round:3} click({sx},{sy}) files={} (no new scene yet)", rt.opened_files.len());
                rt.write_ppm(&out.join(format!("tut_r{round:03}.ppm"))).unwrap();
            }
        }
        println!("TUTORIAL done, reached_script2={reached2} @ {} steps", rt.cpu.steps);
        // Locate the REAL tutorial-subtitle buffer: scan RAM for the on-screen text so a
        // future run can read tutorial STATE (gs:0xe18 held stale attract text).
        for needle in ["waiting", "Bob", "Click quick", "found"] {
            let pat = needle.as_bytes();
            let mem = &rt.m.mem;
            let gs = 0x0e84u32 * 16;
            let mut hits = 0;
            for i in 0..mem.len().saturating_sub(pat.len()) {
                if &mem[i..i + pat.len()] == pat {
                    let rel = if (i as u32) >= gs && (i as u32) < gs + 0x10000 {
                        format!("gs:{:#06x}", i as u32 - gs)
                    } else {
                        format!("linear:{i:#08x}")
                    };
                    println!("  SUBSCAN {needle:?} @ {rel}");
                    hits += 1;
                    if hits >= 4 {
                        break;
                    }
                }
            }
        }
        for (step, path) in &rt.opened_files {
            println!("  @{step:>10} {path}");
        }
        return;
    }

    if std::env::var("BRIDGEPROBE").is_ok() {
        // Verify the TB.BIG bridge-panorama model against the LIVE game: reach the
        // interactive console, then read the decoded bridge state words each probe —
        //   gs:0x2795 = current TB.BIG panorama frame index (0..179, feeds ship-3D yaw)
        //   gs:0x0a2a = bridge view angle (0..0x5a0, 8 units per panorama frame)
        //   gs:0x278b = station/view state byte (cmp 8 gates the console one-shot draw)
        // while injecting candidate rotation inputs (mouse at screen edges, arrow keys),
        // capturing a frame after each so the port's TB.BIG rendering can be pixel-diffed.
        let g = 0x0e84u16;
        let state = |rt: &Runtime| {
            let fr = rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
            let ang = rt.m.read8(g, 0x0a2a) as u16 | ((rt.m.read8(g, 0x0a2b) as u16) << 8);
            let st = rt.m.read8(g, 0x278b);
            (fr, ang, st)
        };
        let mut next_input = 5_000_000u64;
        while rt.cpu.steps < 50_000_000 {
            let _ = rt.run(next_input);
            rt.inject_key(0x01, 0x1b);
            rt.inject_key(0x1c, 0x0d);
            rt.set_mouse_pos(320, 100);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            next_input += 5_000_000;
        }
        let (fr, ang, st) = state(&rt);
        println!("console reached: tb_frame={fr} angle={ang:#x} station={st:#x} @ {} steps", rt.cpu.steps);
        rt.write_ppm(&out.join("bridge_00_console.ppm")).unwrap();
        // TEXTBAND: provoke tutorial text, then dump the top-band palette indices
        // (rows 0..24) + histogram — pins the subtitle glyphs' actual indices and
        // row offsets for the OCR.
        if std::env::var("TEXTBAND").is_ok() {
            // Click HONK to provoke a line.
            let (fr, _, _) = state(&rt);
            let delta = fr as i32 - 45;
            let x = 0x11f - delta * 8 - 0x37;
            let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4 + 8;
            let ring = (x + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, y as u16);
            let _ = rt.run(rt.cpu.steps + 700_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 8_000_000);
            let idx = rt.screen_indices();
            rt.write_ppm(&out.join("textband.ppm")).unwrap();
            std::fs::write(out.join("textband_indices.bin"), &idx).unwrap();
            // Dump the LIVE subtitle font (glyphs at gs:0x71AA, ascii->glyph map
            // at gs:0x70FA) so the OCR matches exactly what the game draws.
            let map: Vec<u8> = (0..256u32).map(|i| rt.m.read8(g, 0x70fa + i)).collect();
            let glyphs: Vec<u8> = (0..2048u32).map(|i| rt.m.read8(g, 0x71aa + i)).collect();
            std::fs::write(out.join("live_font_map.bin"), &map).unwrap();
            std::fs::write(out.join("live_font_glyphs.bin"), &glyphs).unwrap();
            let mut hist = std::collections::HashMap::new();
            for yy in 0..24usize {
                for xx in 0..320usize {
                    *hist.entry(idx[yy * 320 + xx]).or_insert(0u32) += 1;
                }
            }
            let mut top: Vec<_> = hist.into_iter().collect();
            top.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            println!("top-band histogram: {:?}", &top[..top.len().min(12)]);
            // Print rows 0..24 as glyph-mask ASCII for the two most text-like
            // indices (sparse ones).
            for &(v, n) in top.iter().filter(|&&(v, n)| v != 0 && n < 3000).take(3) {
                println!("mask for index {v:#04x} ({n} px):");
                for yy in 0..20usize {
                    let row: String = (0..120)
                        .map(|xx| if idx[yy * 320 + xx] == v { '#' } else { '.' })
                        .collect();
                    println!("  {row}");
                }
            }
            println!("TEXTBAND done");
            return;
        }

        if std::env::var("REVEALTRACE").is_ok() {
            // Sample DURING tutorial progress: alternate a single advance-click with
            // dense 100k-step sampling windows, so line reveals land inside sampling.
            let mut last = (0usize, 0usize, 0usize);
            for round in 0..1400usize {
                if round % 120 == 60 {
                    // one advance click at the orb region (ring-corrected)
                    let fr = rt.m.read8(g, 0x2795) as u16
                        | ((rt.m.read8(g, 0x2796) as u16) << 8);
                    let ring = ((125i32 + fr as i32 * 8 - 160).rem_euclid(1440)) as u16;
                    rt.set_mouse_pos(ring, 118);
                    let _ = rt.run(rt.cpu.steps + 200_000);
                    rt.mouse_press(0);
                    let _ = rt.run(rt.cpu.steps + 200_000);
                    rt.mouse_release(0);
                }
                let _ = rt.run(rt.cpu.steps + 100_000);
                let idx = rt.screen_indices();
                let mut revealing = 0usize;
                let mut settled = 0usize;
                let mut other_text = 0usize;
                for y in 0..30usize {
                    for x in 0..320usize {
                        match idx[y * 320 + x] {
                            0xFD..=0xFF => revealing += 1,
                            0xE0 => settled += 1,
                            0xEE | 0xEF => other_text += 1,
                            _ => {}
                        }
                    }
                }
                if (revealing, settled, other_text) != last {
                    println!(
                        "round {round:5} steps {:>12}: revealing {revealing:5} settled {settled:5} other {other_text:4}",
                        rt.cpu.steps
                    );
                    last = (revealing, settled, other_text);
                }
            }
            println!("REVEALTRACE done");
            return;
        }
        if std::env::var("REVEALTRACE_OLD").is_ok() {
            // SUBTITLE-CADENCE ground truth: boot to the tutorial and log, every
            // 100k steps, the subtitle band's pixel classes — revealing glyphs
            // (indices 0xFD..0xFF, the green console font) vs settled text (0xE0)
            // — plus the audio-chatter write count. The log yields the reveal
            // rate (chars/frame), the reveal->settle transition, and the honk
            // cadence, all measured from the real game.
            let mut last = (0usize, 0usize);
            let mut printed = 0usize;
            for round in 0..6000usize {
                let _ = rt.run(rt.cpu.steps + 100_000);
                let idx = rt.screen_indices();
                let mut revealing = 0usize;
                let mut settled = 0usize;
                for y in 0..30usize {
                    for x in 0..320usize {
                        match idx[y * 320 + x] {
                            0xFD..=0xFF => revealing += 1,
                            0xE0 => settled += 1,
                            _ => {}
                        }
                    }
                }
                if (revealing, settled) != last {
                    println!(
                        "round {round:5} steps {:>12}: revealing {revealing:5} settled {settled:5}",
                        rt.cpu.steps
                    );
                    last = (revealing, settled);
                    printed += 1;
                    if printed > 400 {
                        break;
                    }
                }
            }
            println!("REVEALTRACE done");
            return;
        }

        // TUTORIAL4: screen-OCR instruction follower. The game's subtitle glyphs
        // write reserved indices >= 0xFD; OCR the top rows of screen_indices()
        // against the game's own font bitmaps to read the live line, then click
        // the named target. Deterministic — no buffers or heuristics.
        if std::env::var("TUTORIAL4").is_ok() {
            // OCR the live subtitle rows with the game's OWN in-memory font
            // (glyphs gs:0x71AA, ascii map gs:0x70FA — monospace 8px advance;
            // calibrated offline against textband dumps: rows 8/18, text
            // indices 0xE0 (settled) + 0xFD..0xFF (revealing)).
            let read_font = |rt: &Runtime| -> Vec<(char, [u8; 8])> {
                let mut out = Vec::new();
                for code in 32u8..127 {
                    let gi = rt.m.read8(g, 0x70fa + code as u32);
                    if gi == 0xFF { continue; }
                    let mut rows = [0u8; 8];
                    for (i, r) in rows.iter_mut().enumerate() {
                        *r = rt.m.read8(g, 0x71aa + gi as u32 * 8 + i as u32);
                    }
                    let lit: u32 = rows.iter().map(|r| r.count_ones()).sum();
                    if lit >= 3 {
                        out.push((code as char, rows));
                    }
                }
                out.sort_by_key(|(_, rows)| {
                    std::cmp::Reverse(rows.iter().map(|r| r.count_ones()).sum::<u32>())
                });
                out
            };
            let ocr = |idx: &[u8], font: &[(char, [u8; 8])]| -> String {
                let lit = |px: u8| px == 0xE0 || px == 0xEE || px == 0xEF || px >= 0xFD;
                let mut text = String::new();
                // Dynamic alignment: subtitle rows differ per screen (console
                // 8/18; scene close-ups draw 3 lines from the very top). Find
                // aligned rows by trying each and keeping non-empty reads.
                let mut row0 = 0usize;
                while row0 < 40 {
                    let mut line = String::new();
                    let mut blanks = 0usize;
                    let mut x = 0usize;
                    while x < 313 {
                        let mut got = None;
                        for (ch, rows) in font {
                            let mut ok = true;
                            'cell: for gy in 0..8usize {
                                for gx in 0..8usize {
                                    let on = (rows[gy] >> (7 - gx)) & 1 == 1;
                                    let px = idx
                                        .get((row0 + gy) * 320 + x + gx)
                                        .copied()
                                        .unwrap_or(0);
                                    if on != lit(px) {
                                        ok = false;
                                        break 'cell;
                                    }
                                }
                            }
                            if ok {
                                got = Some(*ch);
                                break;
                            }
                        }
                        if let Some(ch) = got {
                            if blanks >= 8 && !line.is_empty() {
                                line.push(' ');
                            }
                            line.push(ch);
                            blanks = 0;
                            x += 8;
                        } else {
                            blanks += 1;
                            x += 1;
                        }
                    }
                    if line.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 3 {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&line);
                        row0 += 9; // next line
                    } else {
                        row0 += 1;
                    }
                }
                text
            };
            let baseline = rt.opened_files.len();
            let mut last = String::new();
            let mut silent = 0usize;
            let mut reached2 = false;
            for round in 0..1200 {
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                let font = read_font(&rt);
                // Only trust a stable (fully revealed) line.
                let line_a = ocr(&rt.screen_indices(), &font);
                let _ = rt.run(rt.cpu.steps + 400_000);
                let line_b = ocr(&rt.screen_indices(), &font);
                let mut line = if line_a == line_b { line_b } else { String::new() };
                // Second pass: scene subtitles (letterboxed close-ups) use the
                // THIN variable-width GAME_FONT at index 0xEF — the port's own
                // decoded font tables read them directly.
                if line.is_empty() {
                    use commander_blood_tools::font::{game_font_advance, game_font_glyph};
                    let idx = rt.screen_indices();
                    let lit = |px: u8| px == 0xEE || px == 0xEF || px == 0xE8;
                    let mut text = String::new();
                    let mut row0 = 6usize;
                    while row0 < 170 {
                        let mut l = String::new();
                        let mut blanks = 0usize;
                        let mut x = 0usize;
                        while x < 312 {
                            let mut got = None;
                            for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'\",.!?-".chars() {
                                let Some(gl) = game_font_glyph(ch) else { continue };
                                let adv = game_font_advance(ch);
                                let total: u32 = gl.rows.iter().map(|r| r.count_ones()).sum();
                                if total < 4 { continue; }
                                let mut ok = true;
                                'c: for gy in 0..8usize {
                                    for gx in 0..adv.min(8) {
                                        let on = (gl.rows[gy] >> (7 - gx)) & 1 == 1;
                                        let px = idx
                                            .get((row0 + gy) * 320 + x + gx)
                                            .copied()
                                            .unwrap_or(0);
                                        if on != lit(px) { ok = false; break 'c; }
                                    }
                                }
                                if ok { got = Some((ch, adv)); break; }
                            }
                            if let Some((ch, adv)) = got {
                                if blanks >= 5 && !l.is_empty() { l.push(' '); }
                                l.push(ch);
                                blanks = 0;
                                x += adv.max(2);
                            } else {
                                blanks += 1;
                                x += 1;
                            }
                        }
                        if l.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 3 {
                            if !text.is_empty() { text.push(' '); }
                            text.push_str(&l);
                            row0 += 8;
                        } else {
                            row0 += 1;
                        }
                    }
                    line = text;
                }
                if line != last && !line.is_empty() {
                    println!("round {round}: OCR {line:?}");
                    last = line.clone();
                    silent = 0;
                } else {
                    silent += 1;
                    if silent == 40 {
                        // Long silence: capture the state and start working the
                        // menu items in order with long dwells (the tutorial may
                        // be waiting for a function to actually be USED).
                        rt.write_ppm(&out.join(format!("silent_{round}.ppm"))).unwrap();
                        std::fs::write(
                            out.join(format!("silent_{round}_indices.bin")),
                            rt.screen_indices(),
                        )
                        .unwrap();
                        println!("round {round}: silent 40 rounds (frame {fr}) — captured");
                        silent = 0;
                    }
                }
                // NUMKEY: answer the SCRIPT2 exercise by pressing the digit key
                // matching the prompted number word.
                if std::env::var("NUMKEY").is_ok() {
                    let words: [(&str, u8, u8); 9] = [
                        ("ONE", 0x02, b'1'), ("TWO", 0x03, b'2'), ("THREE", 0x04, b'3'),
                        ("FOUR", 0x05, b'4'), ("F1VE", 0x06, b'5'), ("S1X", 0x07, b'6'),
                        ("SEVEN", 0x08, b'7'), ("E1GHT", 0x09, b'8'), ("N1NE", 0x0a, b'9'),
                    ];
                    if let Some(&(w, sc, ch)) = words.iter().find(|(w, _, _)| line == *w) {
                        println!("round {round}: prompt {w:?} -> pressing key");
                        rt.inject_key(sc, ch);
                        let _ = rt.run(rt.cpu.steps + 3_000_000);
                        continue;
                    }
                }
                // NUMANSWER: click the topic-list row matching the prompted word.
                // The list (TALK/ONE..NINE, blue square-capitals) runs down the
                // console's right at x~168.., rows from y~35 at ~13px pitch.
                // HOOKSNAP: when HONK says "CLICK ON ... OVER THERE", capture a
                // frame series (the direction may highlight/animate the target),
                // then follow: click the eye-orb.
                if std::env::var("HOOKSNAP").is_ok() && (line.contains("CL1CK ON") || line.contains("OVER THERE")) {
                    println!("round {round}: HOOK {line:?}");
                    for shot in 0..6 {
                        rt.write_ppm(&out.join(format!("hook_{round}_{shot}.ppm"))).unwrap();
                        let _ = rt.run(rt.cpu.steps + 3_000_000);
                    }
                    // Also: click the orb to open the concept menu, then dump its
                    // indices (the full topic list = many square-caps letters).
                    {
                        let mut orb = (129i32, 117i32);
                        for rec in 0..4u32 {
                            let base = 0x2a1b + rec * 0x18;
                            let w16 = |o: u32| {
                                rt.m.read8(g, base + o) as u16
                                    | ((rt.m.read8(g, base + o + 1) as u16) << 8)
                            };
                            if w16(0xc) != 0xffff {
                                orb = (w16(0xc) as i32 + w16(0x10) as i32 / 2,
                                       w16(0xe) as i32 + w16(0x12) as i32 / 2);
                                break;
                            }
                        }
                        let ring = (orb.0 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                        rt.set_mouse_pos(ring, orb.1 as u16);
                        let _ = rt.run(rt.cpu.steps + 700_000);
                        rt.mouse_press(0);
                        let _ = rt.run(rt.cpu.steps + 400_000);
                        rt.mouse_release(0);
                        let _ = rt.run(rt.cpu.steps + 8_000_000);
                        std::fs::write(out.join("concept_menu_indices.bin"), rt.screen_indices()).unwrap();
                        rt.write_ppm(&out.join("concept_menu.ppm")).unwrap();
                        println!("concept menu indices dumped");
                    }
                    // Follow the direction: click the current frame's orb box.
                    let mut orb = (160i32, 120i32);
                    for rec in 0..4u32 {
                        let base = 0x2a1b + rec * 0x18;
                        let w16 = |o: u32| {
                            rt.m.read8(g, base + o) as u16
                                | ((rt.m.read8(g, base + o + 1) as u16) << 8)
                        };
                        if w16(0xc) != 0xffff {
                            orb = (
                                w16(0xc) as i32 + w16(0x10) as i32 / 2,
                                w16(0xe) as i32 + w16(0x12) as i32 / 2,
                            );
                            break;
                        }
                    }
                    let ring = (orb.0 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                    rt.set_mouse_pos(ring, orb.1 as u16);
                    let _ = rt.run(rt.cpu.steps + 700_000);
                    rt.mouse_press(0);
                    let _ = rt.run(rt.cpu.steps + 400_000);
                    rt.mouse_release(0);
                    let _ = rt.run(rt.cpu.steps + 10_000_000);
                    rt.write_ppm(&out.join(format!("hook_{round}_after_orb.ppm"))).unwrap();
                    println!("round {round}: followed to orb {orb:?} — captured");
                    continue;
                }
                // TOPICTOUR: click each consultation topic once (TALK,1..8),
                // transcribing what each yields, then try rotating the bridge.
                if std::env::var("TOPICTOUR").is_ok() {
                    let topic = (round / 60) % 9; // ~60 rounds per topic
                    if round % 60 == 0 {
                        let (sx, sy) = (190i32, 35 + 13 * topic as i32);
                        let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                        println!("round {round}: TOUR clicking topic row {topic}");
                        rt.set_mouse_pos(ring, sy as u16);
                        let _ = rt.run(rt.cpu.steps + 700_000);
                        rt.mouse_press(0);
                        let _ = rt.run(rt.cpu.steps + 400_000);
                        rt.mouse_release(0);
                        let _ = rt.run(rt.cpu.steps + 2_000_000);
                        continue;
                    }
                }
                if std::env::var("NUMANSWER").is_ok() {
                    // The top-left word is the ECHO of what was clicked (clicking
                    // EIGHT echoes "EIGHT"); "NINE... GOOD WORK" in the transcript
                    // means NINE is the correct topic — always answer row 9.
                    let words = ["TALK", "ONE", "TWO", "THREE", "FOUR", "F1VE", "S1X", "SEVEN", "E1GHT", "N1NE"];
                    if words.iter().any(|w| line == *w) {
                        let row = 9usize; // NINE
                        let (sx, sy) = (190i32, 35 + 13 * row as i32);
                        let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                        println!("round {round}: echo {line:?} -> answering NINE at y{sy}");
                        rt.set_mouse_pos(ring, sy as u16);
                        let _ = rt.run(rt.cpu.steps + 700_000);
                        rt.mouse_press(0);
                        let _ = rt.run(rt.cpu.steps + 400_000);
                        rt.mouse_release(0);
                        let _ = rt.run(rt.cpu.steps + 2_000_000);
                        continue;
                    }
                }
                // NUMSERIES: at the first number prompt, capture a 16-frame series
                // (the numbers display/animation) then stop.
                if std::env::var("NUMSERIES").is_ok() {
                    let numbers = ["ONE", "TWO", "THREE", "FOUR", "F1VE", "S1X", "SEVEN", "E1GHT", "N1NE"];
                    if numbers.iter().any(|n| line == *n) {
                        println!("round {round}: prompt {line:?} — capturing series");
                        for shot in 0..16 {
                            rt.write_ppm(&out.join(format!("series_{shot:02}.ppm"))).unwrap();
                            std::fs::write(
                                out.join(format!("series_{shot:02}_indices.bin")),
                                rt.screen_indices(),
                            )
                            .unwrap();
                            let _ = rt.run(rt.cpu.steps + 2_000_000);
                        }
                        break;
                    }
                }
                // NUMSNAP: capture the screen the moment a number prompt shows.
                if std::env::var("NUMSNAP").is_ok() {
                    let numbers = ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "E1GHT", "EIGHT", "N1NE", "NINE"];
                    if numbers.iter().any(|n| line == *n) {
                        rt.write_ppm(&out.join(format!("numprompt_{round}.ppm"))).unwrap();
                        std::fs::write(
                            out.join(format!("numprompt_{round}_indices.bin")),
                            rt.screen_indices(),
                        )
                        .unwrap();
                        println!("round {round}: number prompt {line:?} captured");
                        if round > 40 { break; }
                    }
                }
                let names = ["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"];
                let want = names.iter().position(|n| line.contains(n));
                if let Some(row) = want {
                    // Obey the instruction, then WATCH what opens: capture the
                    // resulting screen (ground truth for that console function),
                    // and Esc back to the console before continuing.
                    if (40..=60).contains(&fr) {
                        let x = 0x11f - delta * 8 - 0x37;
                        let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4
                            + row as i32 * (0x12 - delta.unsigned_abs() as i32 / 8) + 8;
                        let ring = (x + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                        rt.set_mouse_pos(ring, y as u16);
                        let _ = rt.run(rt.cpu.steps + 700_000);
                        rt.mouse_press(0);
                        let _ = rt.run(rt.cpu.steps + 400_000);
                        rt.mouse_release(0);
                        // Long dwell: let the function open and play (some steps
                        // animate for seconds), capturing along the way.
                        for shot in 0..3 {
                            let _ = rt.run(rt.cpu.steps + 15_000_000);
                            rt.write_ppm(&out.join(format!(
                                "obeyed_{}_{round}_{shot}.ppm",
                                names[row]
                            )))
                            .unwrap();
                        }
                        println!("round {round}: obeyed {} -> captured x3", names[row]);
                        // Return to the console if we left it.
                        rt.inject_key(0x01, 0x1b);
                        let _ = rt.run(rt.cpu.steps + 6_000_000);
                        continue;
                    }
                }
                let effective: Option<usize> = { let t = round % 8; (t < 5).then_some(t) };
                let (sx, sy) = match effective {
                    Some(row) if (40..=60).contains(&fr) => {
                        let x = 0x11f - delta * 8 - 0x37;
                        let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4
                            + row as i32 * (0x12 - delta.unsigned_abs() as i32 / 8) + 8;
                        (x, y)
                    }
                    _ => if round % 2 == 0 { (85, 96) } else { (125, 118) },
                };
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                if rt.opened_files.len() > baseline {
                    let newest: Vec<String> = rt.opened_files[baseline..]
                        .iter().map(|(_, p)| p.clone()).collect();
                    // Milestone: any NEW asset class (scripts, worlds, rooms) =
                    // story progress — capture + savestate + log.
                    for marker in ["script2", "script3", "script4", "script5", ".ext", ".fd"] {
                        if newest.iter().any(|p| p.to_lowercase().contains(marker)) {
                            let tag = marker.trim_start_matches('.');
                            println!("round {round}: MILESTONE {marker} (files {newest:?})");
                            rt.write_ppm(&out.join(format!("milestone_{tag}_{round}.ppm"))).unwrap();
                            rt.save_state(std::path::Path::new(&format!(
                                "accuracy/milestone_{tag}.state"
                            )))
                            .unwrap();
                            if marker == "script2" && !reached2 {
                                reached2 = true;
                                // Run forward past the console-rebuild transition so
                                // the savestate is a CLEAN interactive SCRIPT2 state
                                // (the raw load moment is mid-rebuild — glowing-empty
                                // menu — and doesn't resume into a drivable hub).
                                // Advance the dialogue a few times to reach the hub.
                                for _ in 0..8 {
                                    let (fr2, _, _) = state(&rt);
                                    rt.set_mouse_pos((160i32 + fr2 as i32 * 8 - 160).rem_euclid(1440) as u16, 100);
                                    rt.mouse_press(0);
                                    let _ = rt.run(rt.cpu.steps + 300_000);
                                    rt.mouse_release(0);
                                    let _ = rt.run(rt.cpu.steps + 8_000_000);
                                }
                                rt.write_ppm(&out.join("script2_stable.ppm")).unwrap();
                                rt.save_state(std::path::Path::new("accuracy/script2.state")).unwrap();
                                println!("clean SCRIPT2 savestate written (post-transition)");
                            }
                        }
                    }
                    // Keep walking (don't break) — deeper milestones follow.
                }
            }
            println!("TUTORIAL4 done, reached_script2={reached2} @ {} steps", rt.cpu.steps);
            return;
        }

        // TEXTBAND: provoke tutorial text, then dump the top-band palette indices
        // (rows 0..24) + histogram — pins the subtitle glyphs' actual indices and
        // row offsets for the OCR.
        if std::env::var("TEXTBAND").is_ok() {
            // Click HONK to provoke a line.
            let (fr, _, _) = state(&rt);
            let delta = fr as i32 - 45;
            let x = 0x11f - delta * 8 - 0x37;
            let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4 + 8;
            let ring = (x + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, y as u16);
            let _ = rt.run(rt.cpu.steps + 700_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 8_000_000);
            let idx = rt.screen_indices();
            rt.write_ppm(&out.join("textband.ppm")).unwrap();
            std::fs::write(out.join("textband_indices.bin"), &idx).unwrap();
            // Dump the LIVE subtitle font (glyphs at gs:0x71AA, ascii->glyph map
            // at gs:0x70FA) so the OCR matches exactly what the game draws.
            let map: Vec<u8> = (0..256u32).map(|i| rt.m.read8(g, 0x70fa + i)).collect();
            let glyphs: Vec<u8> = (0..2048u32).map(|i| rt.m.read8(g, 0x71aa + i)).collect();
            std::fs::write(out.join("live_font_map.bin"), &map).unwrap();
            std::fs::write(out.join("live_font_glyphs.bin"), &glyphs).unwrap();
            let mut hist = std::collections::HashMap::new();
            for yy in 0..24usize {
                for xx in 0..320usize {
                    *hist.entry(idx[yy * 320 + xx]).or_insert(0u32) += 1;
                }
            }
            let mut top: Vec<_> = hist.into_iter().collect();
            top.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            println!("top-band histogram: {:?}", &top[..top.len().min(12)]);
            // Print rows 0..24 as glyph-mask ASCII for the two most text-like
            // indices (sparse ones).
            for &(v, n) in top.iter().filter(|&&(v, n)| v != 0 && n < 3000).take(3) {
                println!("mask for index {v:#04x} ({n} px):");
                for yy in 0..20usize {
                    let row: String = (0..120)
                        .map(|xx| if idx[yy * 320 + xx] == v { '#' } else { '.' })
                        .collect();
                    println!("  {row}");
                }
            }
            println!("TEXTBAND done");
            return;
        }

        if std::env::var("REVEALTRACE").is_ok() {
            // SUBTITLE-CADENCE ground truth: boot to the tutorial and log, every
            // 100k steps, the subtitle band's pixel classes — revealing glyphs
            // (indices 0xFD..0xFF, the green console font) vs settled text (0xE0)
            // — plus the audio-chatter write count. The log yields the reveal
            // rate (chars/frame), the reveal->settle transition, and the honk
            // cadence, all measured from the real game.
            let mut last = (0usize, 0usize);
            let mut printed = 0usize;
            for round in 0..6000usize {
                let _ = rt.run(rt.cpu.steps + 100_000);
                let idx = rt.screen_indices();
                let mut revealing = 0usize;
                let mut settled = 0usize;
                for y in 0..30usize {
                    for x in 0..320usize {
                        match idx[y * 320 + x] {
                            0xFD..=0xFF => revealing += 1,
                            0xE0 => settled += 1,
                            _ => {}
                        }
                    }
                }
                if (revealing, settled) != last {
                    println!(
                        "round {round:5} steps {:>12}: revealing {revealing:5} settled {settled:5}",
                        rt.cpu.steps
                    );
                    last = (revealing, settled);
                    printed += 1;
                    if printed > 400 {
                        break;
                    }
                }
            }
            println!("REVEALTRACE done");
            return;
        }

        // TUTORIAL4: screen-OCR instruction follower. The game's subtitle glyphs
        // write reserved indices >= 0xFD; OCR the top rows of screen_indices()
        // against the game's own font bitmaps to read the live line, then click
        // the named target. Deterministic — no buffers or heuristics.
        if std::env::var("TUTORIAL4").is_ok() {
            use commander_blood_tools::font::{game_font_advance, game_font_glyph};
            let ocr = |idx: &[u8]| -> String {
                let mut text = String::new();
                for row0 in [8usize, 18] {
                    let mut line = String::new();
                    let mut x = 0usize;
                    let mut blanks = 0usize;
                    while x < 314 {
                        // Try to match a glyph at (x, row0): all lit pixels of the
                        // glyph must be >= 0xFD on screen, and its column must
                        // contain at least one lit pixel.
                        let mut matched = None;
                        // Bigger glyphs first so punctuation can't subset-match
                        // inside letters; require STRICT cell equality (on==lit).
                        let mut candidates: Vec<(char, usize)> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'\",.!?-"
                            .chars()
                            .filter_map(|ch| {
                                let g = game_font_glyph(ch)?;
                                let lit: usize = g
                                    .rows
                                    .iter()
                                    .map(|r| r.count_ones() as usize)
                                    .sum();
                                Some((ch, lit))
                            })
                            .collect();
                        candidates.sort_by_key(|&(_, lit)| std::cmp::Reverse(lit));
                        for (ch, lit_total) in candidates {
                            let Some(g) = game_font_glyph(ch) else { continue };
                            let mut ok = lit_total >= 5;
                            'cell: for gy in 0..8usize {
                                for gx in 0..g.advance.min(8) {
                                    let on = (g.rows[gy] >> (7 - gx)) & 1 == 1;
                                    let px = idx
                                        .get((row0 + gy) * 320 + x + gx)
                                        .copied()
                                        .unwrap_or(0);
                                    if on != (px == 0xE0 || px >= 0xFD) {
                                        ok = false;
                                        break 'cell;
                                    }
                                }
                            }
                            if ok {
                                matched = Some((ch, game_font_advance(ch)));
                                break;
                            }
                        }
                        if let Some((ch, adv)) = matched {
                            if blanks > 3 && !line.is_empty() { line.push(' '); }
                            line.push(ch);
                            blanks = 0;
                            x += adv.max(2);
                        } else {
                            blanks += 1;
                            x += 1;
                        }
                    }
                    if !line.is_empty() {
                        if !text.is_empty() { text.push(' '); }
                        text.push_str(&line);
                    }
                }
                text
            };
            let baseline = rt.opened_files.len();
            let mut last = String::new();
            let mut reached2 = false;
            for round in 0..500 {
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                // OCR twice with a settle gap; only trust a stable, fully-revealed line.
                let line_a = ocr(&rt.screen_indices());
                let _ = rt.run(rt.cpu.steps + 400_000);
                let line_b = ocr(&rt.screen_indices());
                let line = if line_a == line_b { line_b } else { String::new() };
                if line != last && !line.is_empty() {
                    println!("round {round}: OCR {line:?}");
                    last = line.clone();
                }
                let names = ["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"];
                let want = names.iter().position(|n| line.contains(n));
                let effective = want.or_else(|| { let t = round % 8; (t < 5).then_some(t) });
                let (sx, sy) = match effective {
                    Some(row) if (40..=60).contains(&fr) => {
                        let x = 0x11f - delta * 8 - 0x37;
                        let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4
                            + row as i32 * (0x12 - delta.unsigned_abs() as i32 / 8) + 8;
                        (x, y)
                    }
                    _ => if round % 2 == 0 { (85, 96) } else { (125, 118) },
                };
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                if rt.opened_files.len() > baseline {
                    let newest: Vec<String> = rt.opened_files[baseline..]
                        .iter().map(|(_, p)| p.clone()).collect();
                    if newest.iter().any(|p| p.to_lowercase().contains("script2")) {
                        println!("round {round}: SCRIPT2 reached!");
                        reached2 = true;
                        rt.write_ppm(&out.join("tut4_script2.ppm")).unwrap();
                        break;
                    }
                }
            }
            println!("TUTORIAL4 done, reached_script2={reached2} @ {} steps", rt.cpu.steps);
            return;
        }

        // VMWATCH: per-round dump of candidate VM line-id/state words while blind
        // clicking (as TUTORIAL2) — offline correlation against the port's decoded
        // SCRIPT1 lines identifies which word is the active-line id.
        if std::env::var("VMWATCH").is_ok() {
            for round in 0..60 {
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                let target = round % 8;
                let (sx, sy) = if target >= 6 {
                    (85, if target == 6 { 96 } else { 109 })
                } else if target < 5 && (40..=60).contains(&fr) {
                    let x = 0x11f - delta * 8 - 0x37;
                    let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4
                        + target as i32 * (0x12 - delta.unsigned_abs() as i32 / 8) + 8;
                    (x, y)
                } else {
                    (125, 118)
                };
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                let w = |off: u32| -> u16 {
                    rt.m.read8(g, off) as u16 | ((rt.m.read8(g, off + 1) as u16) << 8)
                };
                println!(
                    "r{round:02} fr={fr:3} 1fab={:04x} 6788={:04x} 67aa={:04x} 6780={:04x} 671c={:04x} 6724={:04x}:{:04x} 6752={:04x} 2a19={:04x} e18={:04x}",
                    w(0x1fab), w(0x6788), w(0x67aa), w(0x6780), w(0x671c),
                    w(0x6726), w(0x6724), w(0x6752), w(0x2a19), w(0xe18)
                );
            }
            println!("VMWATCH done");
            return;
        }

        // SUBFIND: locate the LIVE tutorial-subtitle buffer — run a while at the
        // console, then scan RAM for words known to be on screen (from captures)
        // and print their addresses (gs-relative when inside the data segment).
        if std::env::var("SUBFIND").is_ok() {
            let _ = rt.run(rt.cpu.steps + 20_000_000);
            rt.write_ppm(&out.join("subfind_screen.ppm")).unwrap();
            let gs_lin = 0x0e84usize * 16;
            for needle in ["waiting", "WAITING", "Cap'n", "CAP'N", "Click", "CLICK", "quick", "QUICK", "course", "COURSE"] {
                let pat = needle.as_bytes();
                let mem = &rt.m.mem[..0x100000.min(rt.m.mem.len())];
                let mut shown = 0;
                let mut pos = 0;
                while let Some(i) = mem[pos..].windows(pat.len()).position(|w| w == pat) {
                    let at = pos + i;
                    let rel = if at >= gs_lin && at < gs_lin + 0x10000 {
                        format!("gs:{:#06x}", at - gs_lin)
                    } else {
                        format!("lin:{at:#07x}")
                    };
                    let ctx: String = mem[at.saturating_sub(8)..(at + 40).min(mem.len())]
                        .iter()
                        .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                        .collect();
                    println!("SUBFIND {needle:?} @ {rel}: {ctx:?}");
                    shown += 1;
                    pos = at + 1;
                    if shown >= 3 { break; }
                }
            }
            println!("SUBFIND done");
            return;
        }

        // TUTORIAL3: instruction-FOLLOWING tutorial driver — scans RAM each round
        // for the live tutorial text ("...HONK/TELEPHONE/CRYOBOX/MENU/OPTION...")
        // and clicks the named golden-menu row; choice boxes get their first row;
        // otherwise the orb. Prints each new instruction seen.
        if std::env::var("TUTORIAL3").is_ok() {
            let baseline = rt.opened_files.len();
            let mut last_instr = String::new();
            let mut reached2 = false;
            for round in 0..500 {
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                // Find the most recent menu keyword in RAM near the subtitle areas.
                let names = ["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"];
                let mut want: Option<usize> = None;
                {
                    let mem = &rt.m.mem[..0x100000.min(rt.m.mem.len())];
                    // Look for "ON "<NAME>" or "ON '<NAME>" (the tutorial phrasing).
                    for (row, name) in names.iter().enumerate() {
                        for pat in [format!("ON \"{name}"), format!("ON '{name}"), format!("on \"{name}")] {
                            if let Some(pos) = mem
                                .windows(pat.len())
                                .position(|w| w == pat.as_bytes())
                            {
                                let ctx: String = mem[pos.saturating_sub(24)..(pos + 24).min(mem.len())]
                                    .iter()
                                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                                    .collect();
                                if ctx != last_instr {
                                    println!("round {round}: instruction {ctx:?}");
                                    last_instr = ctx;
                                }
                                want = Some(row);
                                break;
                            }
                        }
                        if want.is_some() { break; }
                    }
                }
                // Directed row when the subtitle names one; else cycle all targets
                // (menu rows / orb / choice rows) so dialogue keeps being provoked.
                let effective: Option<usize> = want.or_else(|| {
                    let t = round % 8;
                    (t < 5).then_some(t)
                });
                let (sx, sy) = match effective {
                    Some(row) if (40..=60).contains(&fr) => {
                        let x = 0x11f - delta * 8 - 0x37;
                        let y = 0x48
                            + delta.unsigned_abs() as i32 * 5 / 4
                            + row as i32 * (0x12 - delta.unsigned_abs() as i32 / 8)
                            + 8;
                        (x, y)
                    }
                    _ => {
                        // Alternate: choice-box first row, then the orb.
                        if round % 2 == 0 { (85, 96) } else {
                            let mut orb = (160, 120);
                            for rec in 0..4u32 {
                                let base = 0x2a1b + rec * 0x18;
                                let w16 = |o: u32| {
                                    rt.m.read8(g, base + o) as u16
                                        | ((rt.m.read8(g, base + o + 1) as u16) << 8)
                                };
                                if w16(0xc) != 0xffff {
                                    orb = (
                                        w16(0xc) as i32 + w16(0x10) as i32 / 2,
                                        w16(0xe) as i32 + w16(0x12) as i32 / 2,
                                    );
                                    break;
                                }
                            }
                            orb
                        }
                    }
                };
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                if rt.opened_files.len() > baseline {
                    let newest: Vec<String> = rt.opened_files[baseline..]
                        .iter()
                        .map(|(_, p)| p.clone())
                        .collect();
                    if newest.iter().any(|p| p.to_lowercase().contains("script2")) {
                        println!("round {round}: SCRIPT2 reached! new files {newest:?}");
                        reached2 = true;
                        rt.write_ppm(&out.join("tut3_script2.ppm")).unwrap();
                        break;
                    }
                }
                if round % 50 == 0 {
                    rt.write_ppm(&out.join(format!("tut3_r{round:03}.ppm"))).unwrap();
                }
            }
            println!("TUTORIAL3 done, reached_script2={reached2} @ {} steps", rt.cpu.steps);
            return;
        }

        // TUTORIAL2: drive the SCRIPT1 tutorial with DECODED-geometry clicks (the
        // old TUTORIAL used pre-panorama guessed coordinates). Each round clicks
        // the next target: the eye-orb (live box from the station table) or a
        // golden-menu row (decoded box math at the live frame), in ring space.
        // Watches opened_files for script2.* = tutorial complete.
        if std::env::var("TUTORIAL2").is_ok() {
            let baseline = rt.opened_files.len();
            let mut reached2 = false;
            for round in 0..400 {
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                // Targets: 5 menu rows (only valid frames 40..60) + the orb + the
                // LEFT CHOICE BOX rows (e.g. {BOB_MORLOCK, CANCEL} — appears over
                // the window at ~x45..130, rows ~y95/y108; clicking the first row
                // answers the tutorial's prompt).
                let target = round % 8;
                let (sx, sy) = if target >= 6 {
                    (85, if target == 6 { 96 } else { 109 })
                } else if target < 5 && (40..=60).contains(&fr) {
                    let x = 0x11f - delta * 8 - 0x37; // box centre-ish
                    let y = 0x48
                        + delta.unsigned_abs() as i32 * 5 / 4
                        + target as i32 * (0x12 - delta.unsigned_abs() as i32 / 8)
                        + 8;
                    (x, y)
                } else {
                    // The eye-orb: current frame's box from the station table
                    // (gs:0x2A1B, the record with a valid box at +0xC..+0x13).
                    let mut orb = None;
                    for rec in 0..4u32 {
                        let base = 0x2a1b + rec * 0x18;
                        let w16 = |o: u32| {
                            rt.m.read8(g, base + o) as u16
                                | ((rt.m.read8(g, base + o + 1) as u16) << 8)
                        };
                        let (x, y, w, h) = (w16(0xc), w16(0xe), w16(0x10), w16(0x12));
                        if x != 0xffff {
                            orb = Some((x as i32 + w as i32 / 2, y as i32 + h as i32 / 2));
                            break;
                        }
                    }
                    orb.unwrap_or((160, 120))
                };
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                if rt.opened_files.len() > baseline {
                    let newest: Vec<String> = rt.opened_files[baseline..]
                        .iter()
                        .map(|(_, p)| p.clone())
                        .collect();
                    let has2 = newest.iter().any(|p| p.to_lowercase().contains("script2"));
                    println!("round {round}: NEW files {newest:?} script2={has2}");
                    if has2 {
                        reached2 = true;
                        rt.write_ppm(&out.join("tut2_script2.ppm")).unwrap();
                        break;
                    }
                }
                if round % 40 == 0 {
                    let (fr2, _, _) = state(&rt);
                    println!("round {round}: frame {fr2}, files {}", rt.opened_files.len());
                    rt.write_ppm(&out.join(format!("tut2_r{round:03}.ppm"))).unwrap();
                }
            }
            println!("TUTORIAL2 done, reached_script2={reached2} @ {} steps", rt.cpu.steps);
            return;
        }

        // CHOICEDRIVE: from the CLEAN savestate, click the LEFT choice box rows
        // (the conversation's topic selector, where CANCEL shows) + watch gs:0x6780
        // (the D2 profile request — nonzero = the `what` destination offer fired).
        if std::env::var("CHOICEDRIVE").is_ok() {
            let d2 = |rt: &Runtime| rt.m.read8(g, 0x6780) as u16 | ((rt.m.read8(g, 0x6781) as u16) << 8);
            let anchors = |rt: &Runtime| (0..33u32).any(|i| {
                let v=(rt.m.read8(g,0x4f09+i*2) as u16|((rt.m.read8(g,0x4f09+i*2+1) as u16)<<8)) as i16;
                v!=0 && v.abs()<8000 && v.abs()!=900 && v.abs()!=10200 && v.abs()!=12100
            });
            println!("start: D2={:#06x}", d2(&rt));
            let baseline = rt.opened_files.len();
            // Systematically click each LEFT choice-box row (x ~85, y from 88 step 13),
            // then re-open (orb) between tries. 40 passes.
            for pass in 0..40u32 {
                let row = pass % 8;
                let (fr,_,_) = state(&rt);
                let sy = 88 + 13*row as u16;
                let ring = (85i32 + fr as i32*8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _=rt.run(rt.cpu.steps+600_000); rt.mouse_press(0);
                let _=rt.run(rt.cpu.steps+300_000); rt.mouse_release(0);
                let _=rt.run(rt.cpu.steps+6_000_000);
                let d = d2(&rt); let a = anchors(&rt);
                let nf = rt.opened_files.len() - baseline;
                if d != 0xffff || a || nf > 0 {
                    println!("pass {pass} row {row}: D2={d:#06x} anchors={a} newfiles={nf}");
                    rt.write_ppm(&out.join(format!("choice_p{pass}.ppm"))).unwrap();
                }
                if d != 0xffff || a {
                    println!("DESTINATION OFFER FIRED at pass {pass}!");
                    rt.save_state(std::path::Path::new("accuracy/world_loaded.state")).unwrap();
                    break;
                }
                // Try the orb to (re)open a menu between rows.
                if pass % 8 == 7 {
                    let oring = (125i32 + fr as i32*8 - 160).rem_euclid(1440) as u16;
                    rt.set_mouse_pos(oring, 118);
                    let _=rt.run(rt.cpu.steps+500_000); rt.mouse_press(0);
                    let _=rt.run(rt.cpu.steps+300_000); rt.mouse_release(0);
                    let _=rt.run(rt.cpu.steps+5_000_000);
                }
            }
            println!("CHOICEDRIVE done, final D2={:#06x}", d2(&rt));
            return;
        }

        // HUBSCAN: capture the SCRIPT2-start state + probe whether a topic menu /
        // "click on anything" prompt is showing (OCR the list region), then try
        // opening the concept menu (orb) and clicking the WHAT topic -> the
        // `what` destination chooser -> world load.
        if std::env::var("HUBSCAN").is_ok() {
            rt.write_ppm(&out.join("hub_state.ppm")).unwrap();
            std::fs::write(out.join("hub_indices.bin"), rt.screen_indices()).unwrap();
            let baseline = rt.opened_files.len();
            // Try the concept-menu WHAT row directly (row 9 of the psychotherapy
            // layout: x~190, y 35+11*9=134) — in case the hub menu is showing.
            let anchors_live = |rt: &Runtime| (0..33u32).any(|i| {
                let v=(rt.m.read8(g,0x4f09+i*2) as u16|((rt.m.read8(g,0x4f09+i*2+1) as u16)<<8)) as i16;
                v!=0 && v.abs()<8000 && v.abs()!=900 && v.abs()!=10200 && v.abs()!=12100
            });
            for (name, mx, my) in [("orb-then-what", 125u16, 118u16), ("what-row9", 190, 134),
                                   ("what-row9b", 190, 145), ("menu-what", 232, 148)] {
                let (fr,_,_) = state(&rt);
                // If orb variant: click orb first to open the menu.
                if name.starts_with("orb") {
                    let ring=(125i32+fr as i32*8-160).rem_euclid(1440) as u16;
                    rt.set_mouse_pos(ring,118);
                    let _=rt.run(rt.cpu.steps+600_000); rt.mouse_press(0);
                    let _=rt.run(rt.cpu.steps+300_000); rt.mouse_release(0);
                    let _=rt.run(rt.cpu.steps+4_000_000);
                }
                let ring=(mx as i32+fr as i32*8-160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring,my);
                let _=rt.run(rt.cpu.steps+600_000); rt.mouse_press(0);
                let _=rt.run(rt.cpu.steps+300_000); rt.mouse_release(0);
                let _=rt.run(rt.cpu.steps+12_000_000);
                let nf:Vec<String>=rt.opened_files[baseline..].iter().map(|(_,p)|p.clone()).collect();
                let anch=anchors_live(&rt);
                println!("{name}: anchors={anch} files={nf:?}");
                rt.write_ppm(&out.join(format!("hub_{name}.ppm"))).unwrap();
                if anch || nf.iter().any(|f| f.to_lowercase().ends_with(".ext")) {
                    println!("WORLD LOADED via {name}!");
                    rt.save_state(std::path::Path::new("accuracy/world_loaded.state")).unwrap();
                    break;
                }
                rt.inject_key(0x01,0x1b); let _=rt.run(rt.cpu.steps+3_000_000);
            }
            println!("HUBSCAN done");
            return;
        }

        // SCRIPT2FWD: play SCRIPT2 forward by click-to-advance (NOT re-entering the
        // topic hub) — advance the linear story past the consultation toward the
        // free-choice nav, watching DS:0x4F09 anchors + new .ext/script loads.
        if std::env::var("SCRIPT2FWD").is_ok() {
            let anchors_live = |rt: &Runtime| (0..33u32).any(|i| {
                let v = (rt.m.read8(g, 0x4f09 + i*2) as u16 | ((rt.m.read8(g, 0x4f09+i*2+1) as u16)<<8)) as i16;
                v != 0 && v.abs() < 8000 && v.abs() != 900 && v.abs() != 10200 && v.abs() != 12100
            });
            let baseline = rt.opened_files.len();
            let mut last_files = 0usize;
            for round in 0..1500u32 {
                // Advance: a center click (dismiss/next line), occasionally the orb.
                let (fr,_,_) = state(&rt);
                let (sx, sy) = if round % 5 == 0 { (125u16, 118u16) } else { (160, 100) };
                let ring = (sx as i32 + fr as i32*8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_press(0); let _ = rt.run(rt.cpu.steps + 250_000); rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_200_000);
                let nfiles = rt.opened_files.len();
                if nfiles > last_files {
                    let newest: Vec<String> = rt.opened_files[baseline.max(last_files)..].iter().map(|(_,p)|p.clone()).collect();
                    let has_world = newest.iter().any(|f| f.to_lowercase().ends_with(".ext"));
                    let has_scr = newest.iter().any(|f| f.to_lowercase().contains("script3")||f.to_lowercase().contains("script4")||f.to_lowercase().contains("script5"));
                    println!("round {round}: NEW {newest:?} world={has_world} script345={has_scr}");
                    if has_world || has_scr || anchors_live(&rt) {
                        println!("REACHED destination/world state at round {round}!");
                        rt.write_ppm(&out.join(format!("s2fwd_reached_{round}.ppm"))).unwrap();
                        rt.save_state(std::path::Path::new("accuracy/world_loaded.state")).unwrap();
                        break;
                    }
                    last_files = nfiles;
                }
                if round % 150 == 0 {
                    println!("round {round}: frame {fr}, anchors={} files={}", anchors_live(&rt), nfiles);
                    rt.write_ppm(&out.join(format!("s2fwd_r{round}.ppm"))).unwrap();
                }
            }
            println!("SCRIPT2FWD done");
            return;
        }

        // PHONEWALK: from SCRIPT2 open the TELEPHONE (golden menu row 1) and call
        // each crew contact (choice box), watching for a destination grant
        // (nav anchors populate / a new location .ext opens) — the story beat
        // HONK flags ("IF THE PHONE RINGS...").
        if std::env::var("PHONEWALK").is_ok() {
            let anchors_live = |rt: &Runtime| (0..33u32).any(|i| {
                let v = (rt.m.read8(g, 0x4f09 + i*2) as u16 | ((rt.m.read8(g, 0x4f09+i*2+1) as u16)<<8)) as i16;
                v != 0 && v.abs() < 8000 && v.abs() != 900 && v.abs() != 10200 && v.abs() != 12100
            });
            let baseline = rt.opened_files.len();
            // Click TELEPHONE (golden menu row 1 at the console frame).
            let (fr,_,_) = state(&rt);
            let delta = fr as i32 - 45;
            let mx = 0x11f - delta*8 - 0x37;
            let my = 0x48 + delta.unsigned_abs() as i32*5/4 + 1*(0x12 - delta.unsigned_abs() as i32/8) + 8;
            let ring = (mx + fr as i32*8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, my as u16);
            let _ = rt.run(rt.cpu.steps + 700_000);
            rt.mouse_press(0); let _ = rt.run(rt.cpu.steps + 400_000); rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 10_000_000);
            rt.write_ppm(&out.join("phone_dial.ppm")).unwrap();
            let newest: Vec<String> = rt.opened_files[baseline..].iter().map(|(_,p)|p.clone()).collect();
            println!("after TELEPHONE: new files {newest:?}");
            // Call each contact row in the choice box (left, y88+13*i).
            for row in 0..8u16 {
                let (fr,_,_) = state(&rt);
                let sy = 88 + 6 + 13*row;
                let ring = (90i32 + fr as i32*8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0); let _ = rt.run(rt.cpu.steps + 400_000); rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 15_000_000);
                rt.write_ppm(&out.join(format!("phone_call_{row}.ppm"))).unwrap();
                let anchors = anchors_live(&rt);
                let nf: Vec<String> = rt.opened_files[baseline..].iter().map(|(_,p)|p.clone()).collect();
                println!("call row {row}: anchors={anchors} files={:?}", &nf[nf.len().saturating_sub(3)..]);
                if anchors || nf.iter().any(|f| f.to_lowercase().contains(".ext")) {
                    println!("GRANT at row {row}!");
                    rt.save_state(std::path::Path::new("accuracy/granted.state")).unwrap();
                    break;
                }
                rt.inject_key(0x01, 0x1b); // Esc back
                let _ = rt.run(rt.cpu.steps + 4_000_000);
            }
            println!("PHONEWALK done");
            return;
        }

        // GRANTWALK: from SCRIPT2, interact exhaustively (topics, orb, phone
        // rows, menu) and watch DS:0x4F09 nav anchors for POPULATION (a granted
        // destination). On non-empty anchors: save a milestone state + capture.
        if std::env::var("GRANTWALK").is_ok() {
            let anchors_nonempty = |rt: &Runtime| {
                // Empty = the default trig-table-overlap pattern; treat all-equal
                // or zero as empty. Non-empty = varied small coordinate values.
                let vals: Vec<i16> = (0..33u32)
                    .map(|i| {
                        let lo = rt.m.read8(g, 0x4f09 + i * 2) as u16;
                        let hi = rt.m.read8(g, 0x4f09 + i * 2 + 1) as u16;
                        (lo | (hi << 8)) as i16
                    })
                    .collect();
                // Heuristic: a populated anchor set has values in a plausible
                // world-coordinate range and not the 900/10200/12100 default cycle.
                let defaulty = vals.windows(3).any(|w| w == [10200, 12100, 900]);
                !defaulty && vals.iter().any(|&v| v.abs() > 0 && v.abs() < 8000)
            };
            let targets: [(u16, u16); 10] = [
                (190, 45), (190, 56), (190, 67), (190, 78), (190, 89), // topic rows
                (125, 118), // orb
                (230, 88), (230, 103), (230, 118), (230, 133), // menu rows
            ];
            let mut milestones = 0;
            for round in 0..600u32 {
                let (fr, _, _) = state(&rt);
                let (sx, sy) = targets[(round as usize) % targets.len()];
                let ring = (sx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _ = rt.run(rt.cpu.steps + 500_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 1_500_000);
                if anchors_nonempty(&rt) {
                    println!("round {round}: NAV ANCHORS POPULATED!");
                    rt.write_ppm(&out.join(format!("granted_{round}.ppm"))).unwrap();
                    rt.save_state(std::path::Path::new("accuracy/granted.state")).unwrap();
                    milestones += 1;
                    if milestones >= 2 { break; }
                }
                if round % 60 == 0 {
                    println!("round {round}: frame {fr}, anchors {}",
                        if anchors_nonempty(&rt) { "populated" } else { "empty" });
                    rt.write_ppm(&out.join(format!("grant_r{round}.ppm"))).unwrap();
                }
            }
            println!("GRANTWALK done, milestones={milestones}");
            return;
        }

        // GLYPHSRC: watch 0xE8 writes into the CHUNKY composition buffer (seg
        // 0x266c) during a MENU-submenu open — the writer is the box-text
        // drawer; its ds:si reveals the glyph source (font table / strokes).
        if std::env::var("GLYPHSRC").is_ok() {
            // Arm the watch on the gs:0x175 stream region FIRST, THEN open the
            // MENU submenu — so we catch the builder as it bakes the box.
            let gsbuf = 0x0e84usize * 16;
            rt.m.watch = Some((0xE8, gsbuf + 0x100..gsbuf + 0x3000));
            rt.m.watch_hits.clear();
            let (fr, _, _) = state(&rt);
            let delta = fr as i32 - 45;
            let x = 0x11f - delta * 8 - 0x37;
            let y = 0x48 + delta.unsigned_abs() as i32 * 5 / 4
                + 3 * (0x12 - delta.unsigned_abs() as i32 / 8) + 8;
            let ring = (x + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, y as u16);
            let _ = rt.run(rt.cpu.steps + 700_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 6_000_000);
            let mut seen = std::collections::HashSet::new();
            for &(cs, ip, ds, si, addr) in rt.m.watch_hits.iter() {
                if seen.insert((cs, ip)) {
                    println!("stream builder {cs:04x}:{ip:04x} -> gs:{:#06x} (ds:si={ds:04x}:{si:04x})", addr - gsbuf);
                }
            }
            rt.m.watch = None;
            println!("GLYPHSRC done");
            return;
        }

        // PINTRACE: trace every write to [0x2793] while attempting rotation from
        // the fresh savestate (no clicks) — identifies the actual pin.
        if std::env::var("PINTRACE").is_ok() {
            rt.m.trace_range = Some(0xE840 + 0x2793..0xE840 + 0x2795);
            rt.m.range_hits.clear();
            let (fr0, _, _) = state(&rt);
            let target = ((fr0 as u32 * 8 + 280) % 1440) as u16;
            rt.set_mouse_pos(target, 100);
            let _ = rt.run(rt.cpu.steps + 12_000_000);
            let (fr1, _, _) = state(&rt);
            println!("rotation attempt: frame {fr0} -> {fr1}");
            let mut seen = std::collections::HashSet::new();
            for &(addr, v, cs, ip) in rt.m.range_hits.iter() {
                if seen.insert((cs, ip, v)) {
                    println!("[0x2793] write {v:#04x} (byte {}) from {cs:04x}:{ip:04x}", addr - 0xE840 - 0x2793);
                }
                if seen.len() > 20 { break; }
            }
            rt.m.trace_range = None;
            println!("PINTRACE done");
            return;
        }

        // ADIEUWALK: read the live topic list (square-caps OCR is host-side: we
        // just reuse TUTORIAL4's loop), click TALK first (back to the hub), then
        // walk topics watching [0x2793] bit2 — find the exit that releases the
        // script-owned engagement legitimately.
        if std::env::var("ADIEUWALK").is_ok() {
            let flags = |rt: &Runtime| rt.m.read8(g, 0x2793) as u16 | ((rt.m.read8(g, 0x2794) as u16) << 8);
            println!("start: [0x2793]={:#06x}", flags(&rt));
            // Topic rows at x~175/pitch 11 from y45 (list geometry) + the hub rows
            // (TALK first). Click each row, then check the engagement bit.
            for row in 0..12u16 {
                let (fr, _, _) = state(&rt);
                let sx = 190i32;
                let sy = 45 + 11 * row as i32;
                let ring = (sx + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 700_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 400_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 6_000_000);
                let f = flags(&rt);
                println!("row {row}: [0x2793]={f:#06x}");
                if f & 4 == 0 {
                    println!("RELEASED at row {row}!");
                    rt.write_ppm(&out.join(format!("released_row{row}.ppm"))).unwrap();
                    break;
                }
            }
            println!("ADIEUWALK done");
            return;
        }

        // TRAVELPROBE: leave HONK's consultation (Esc / TALK), then attempt the
        // rotation to the nav sector and the orb click — the travel exit.
        if std::env::var("TRAVELPROBE").is_ok() {
            // First: advance/close the consultation — Esc, then a few advancing clicks.
            rt.inject_key(0x01, 0x1b);
            let _ = rt.run(rt.cpu.steps + 6_000_000);
            for _ in 0..6 {
                rt.set_mouse_pos((160 + 45 * 8 - 160) as u16, 100);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 500_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 2_500_000);
            }
            // Try right-click (back) then check the engagement flags directly.
            rt.mouse_press(1);
            let _ = rt.run(rt.cpu.steps + 500_000);
            rt.mouse_release(1);
            let _ = rt.run(rt.cpu.steps + 5_000_000);
            let flags = rt.m.read8(g, 0x2793) as u16 | ((rt.m.read8(g, 0x2794) as u16) << 8);
            let item = rt.m.read8(g, 0x2a19) as u16 | ((rt.m.read8(g, 0x2a1a) as u16) << 8);
            println!("after right-click: [0x2793]={flags:#06x} [0x2A19]={item:#06x}");
            if std::env::var("UNPIN").is_ok() {
                // Diagnostic: clear the menu engagement and see if rotation frees.
                rt.m.write8(g, 0x2a19, 0);
                rt.m.write8(g, 0x2a1a, 0);
                let f = flags & !0x000C;
                rt.m.write8(g, 0x2793, f as u8);
                rt.m.write8(g, 0x2794, (f >> 8) as u8);
                println!("UNPIN: cleared [0x2A19] and [0x2793] bits 2/3");
            }
            // Rotation sweep toward the pyramid sector.
            for stop in 0..8 {
                let (fr, _, _) = state(&rt);
                let target = ((fr as u32 * 8 + 280) % 1440) as u16;
                rt.set_mouse_pos(target, 100);
                let _ = rt.run(rt.cpu.steps + 10_000_000);
                let (fr2, _, _) = state(&rt);
                println!("travel stop {stop}: frame {fr2}");
                rt.write_ppm(&out.join(format!("travel_{stop}_f{fr2}.ppm"))).unwrap();
                if (72..=107).contains(&fr2) {
                    let _ = rt.run(rt.cpu.steps + 8_000_000);
                    rt.write_ppm(&out.join("travel_nav_sector.ppm")).unwrap();
                    // Click the orb at the nav sector.
                    for rec in 0..4u32 {
                        let base = 0x2a1b + rec * 0x18;
                        let w16 = |o: u32| {
                            rt.m.read8(g, base + o) as u16
                                | ((rt.m.read8(g, base + o + 1) as u16) << 8)
                        };
                        if w16(0xc) != 0xffff {
                            let (ox, oy) = (
                                w16(0xc) as i32 + w16(0x10) as i32 / 2,
                                w16(0xe) as i32 + w16(0x12) as i32 / 2,
                            );
                            let ring = (ox + fr2 as i32 * 8 - 160).rem_euclid(1440) as u16;
                            rt.set_mouse_pos(ring, oy as u16);
                            let _ = rt.run(rt.cpu.steps + 700_000);
                            rt.mouse_press(0);
                            let _ = rt.run(rt.cpu.steps + 400_000);
                            rt.mouse_release(0);
                            let _ = rt.run(rt.cpu.steps + 12_000_000);
                            rt.write_ppm(&out.join("travel_after_orb.ppm")).unwrap();
                            println!("nav orb clicked at ({ox},{oy})");
                            // Interact with the opened NAV SCREEN: click pyramids
                            // (candidate destinations) + the centre orb, capturing.
                            let targets: [(u16, u16, &str); 6] = [
                                (60, 165, "pyr_left"), (120, 155, "pyr_midl"),
                                (200, 155, "pyr_midr"), (260, 165, "pyr_right"),
                                (160, 100, "viewscreen"), (160, 160, "orb2"),
                            ];
                            for (px, py, name) in targets {
                                rt.set_mouse_pos(px * 2, py);
                                let _ = rt.run(rt.cpu.steps + 1_000_000);
                                rt.mouse_press(0);
                                let _ = rt.run(rt.cpu.steps + 500_000);
                                rt.mouse_release(0);
                                let _ = rt.run(rt.cpu.steps + 8_000_000);
                                rt.write_ppm(&out.join(format!("navscr_{name}.ppm"))).unwrap();
                                println!("nav screen: clicked {name}");
                            }
                            break;
                        }
                    }
                    break;
                }
            }
            println!("TRAVELPROBE done");
            return;
        }

        // NAVPROBE: post-tutorial nav sector — rotate to the pyramid room and
        // capture + OCR: are real destinations offered now? (The core
        // choose-a-location gameplay loop's ground truth.)
        if std::env::var("NAVPROBE").is_ok() {
            for stop in 0..6 {
                let (fr, _, _) = state(&rt);
                let target = ((fr as u32 * 8 + 280) % 1440) as u16;
                rt.set_mouse_pos(target, 100);
                let _ = rt.run(rt.cpu.steps + 10_000_000);
                let (fr2, _, _) = state(&rt);
                rt.write_ppm(&out.join(format!("nav_{stop}_f{fr2}.ppm"))).unwrap();
                std::fs::write(
                    out.join(format!("nav_{stop}_f{fr2}_indices.bin")),
                    rt.screen_indices(),
                )
                .unwrap();
                println!("nav stop {stop}: frame {fr2}");
                if (72..=107).contains(&fr2) {
                    // In the pyramid sector: linger, click the orb, capture.
                    let _ = rt.run(rt.cpu.steps + 10_000_000);
                    rt.write_ppm(&out.join(format!("nav_sector_f{fr2}.ppm"))).unwrap();
                    break;
                }
            }
            println!("NAVPROBE done");
            return;
        }

        // FONTFIND: search live RAM for the choice-box glyph bitmaps (not present
        // in any file — derived or copied at runtime).
        if std::env::var("FONTFIND").is_ok() {
            let pats: [(&str, [u8; 8]); 3] = [
                ("C", [0xfe,0x80,0x80,0x80,0x80,0x80,0x80,0xff]),
                ("A", [0x7f,0x81,0x81,0x81,0x81,0xbf,0x81,0x81]),
                ("N", [0xfe,0x81,0x81,0x81,0x81,0x81,0x81,0x81]),
            ];
            for (name, pat) in pats {
                let mut found = 0;
                for i in 0..rt.m.mem.len().saturating_sub(8) {
                    if rt.m.mem[i..i + 8] == pat {
                        println!("FONTFIND {name} at linear {i:#07x}");
                        found += 1;
                        if found >= 4 { break; }
                    }
                }
                if found == 0 { println!("FONTFIND {name}: not in RAM"); }
            }
            return;
        }

        // SUBMENUCAP: click the golden menu's MENU and OPTION rows (decoded box:
        // screen x 177..287, rows top 0x48 pitch 0x12 at frame 45-centred view;
        // at frame 55 the box shifts -8px/frame => right edge 207) and capture
        // what actually opens — ground truth for the submenu/OPTION ports.
        if std::env::var("SUBMENUCAP").is_ok() {
            // Watch who BUILDS the choice-box RLE stream at gs:0x0175 (the
            // panorama unpacker composites the box from there).
            rt.m.trace_range = Some(0xE840 + 0x100..0xE840 + 0x2000);
            rt.m.range_hits.clear();
            rt.m.watch = Some((0xE8, 0..0x100000));
            rt.m.watch_hits.clear();
            for (row, name) in [(3u16, "menu"), (4, "option")] {
                // Row centre at the CURRENT frame (55): delta=10 => box x 97..207,
                // top = 0x48 + 12 = 84, pitch 17.
                let (fr, _, _) = state(&rt);
                let delta = fr as i32 - 45;
                let x = (0x11f - delta * 8 - 0x37) as u16; // box centre-ish
                let y = (0x48 + delta.unsigned_abs() as i32 * 5 / 4
                    + (row as i32) * (0x12 - delta.unsigned_abs() as i32 / 8)
                    + 8) as u16;
                let ring = (x as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                println!("clicking row {row} at screen ({x},{y}) ring {ring} (frame {fr})");
                rt.set_mouse_pos(ring, y);
                let _ = rt.run(rt.cpu.steps + 2_000_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 1_000_000);
                rt.mouse_release(0);
                for stage in 0..4 {
                    let _ = rt.run(rt.cpu.steps + 8_000_000);
                    rt.write_ppm(&out.join(format!("submenu_{name}_{stage}.ppm"))).unwrap();
                    std::fs::write(
                        out.join(format!("submenu_{name}_{stage}_indices.bin")),
                        rt.screen_indices(),
                    )
                    .unwrap();
                    // OCR the choice-box region: thin GAME_FONT knocked out at
                    // index 0xE8 (the measured box spec).
                    use commander_blood_tools::font::{game_font_advance, game_font_glyph};
                    let idx = rt.screen_indices();
                    let mut lit_font: Vec<(char, [u8; 8], usize)> = Vec::new();
                    for code in 32u8..127 {
                        if let Some(gl) = game_font_glyph(code as char) {
                            if gl.rows.iter().map(|r| r.count_ones()).sum::<u32>() >= 4 {
                                lit_font.push((code as char, gl.rows, game_font_advance(code as char)));
                            }
                        }
                    }
                    lit_font.sort_by_key(|(_, rows, _)| {
                        std::cmp::Reverse(rows.iter().map(|r| r.count_ones()).sum::<u32>())
                    });
                    let lit = |px: u8| px == 0xE8 || px == 0xEF;
                    let mut row0 = 60usize;
                    while row0 < 170 {
                        let mut line = String::new();
                        let mut blanks = 0usize;
                        let mut x = 0usize;
                        while x < 312 {
                            let mut got = None;
                            for (ch, rows, adv) in &lit_font {
                                let mut ok = true;
                                'c: for gy in 0..8usize {
                                    for gx in 0..adv.min(&8usize).clone() {
                                        let on = (rows[gy] >> (7 - gx)) & 1 == 1;
                                        let px = idx
                                            .get((row0 + gy) * 320 + x + gx)
                                            .copied()
                                            .unwrap_or(0);
                                        if on != lit(px) { ok = false; break 'c; }
                                    }
                                }
                                if ok { got = Some((*ch, *adv)); break; }
                            }
                            if let Some((ch, adv)) = got {
                                if blanks >= 8 && !line.is_empty() { line.push(' '); }
                                line.push(ch);
                                blanks = 0;
                                x += adv.max(2);
                            } else {
                                blanks += 1;
                                x += 1;
                            }
                        }
                        if line.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 3 {
                            println!("{name} stage {stage} y{row0}: {line:?}");
                            row0 += 9;
                        } else {
                            row0 += 1;
                        }
                    }
                }
                let (fr2, _, st2) = state(&rt);
                println!("after {name} click: frame {fr2} station {st2:#x}");
                rt.inject_key(0x01, 0x1b); // Esc back
                let _ = rt.run(rt.cpu.steps + 6_000_000);
            }
            for &(cs, ip, ds, si, addr) in rt.m.watch_hits.iter() {
                // Only surface writes into the composition/back-buffer/VRAM ranges.
                if (0x266C0..0x366C0).contains(&addr) || addr >= 0xA0000 {
                    println!("0xE8 pixel writer {cs:04x}:{ip:04x} -> {addr:#07x} (ds:si={ds:04x}:{si:04x})");
                }
            }
            rt.m.watch = None;
            let mut seen = std::collections::HashSet::new();
            for &(addr, v, cs, ip) in rt.m.range_hits.iter() {
                if seen.insert((cs, ip)) {
                    println!("stream builder {cs:04x}:{ip:04x} -> gs:{:#06x} = {v:#04x}", addr - 0xE840);
                }
                if seen.len() > 15 { break; }
            }
            rt.m.trace_range = None;
            println!("SUBMENUCAP done");
            return;
        }
        // HANDATLAS: capture the pointing-hand sprite from the LIVE renderer at a
        // grid of cursor positions on the console view (frame 55). Each capture is
        // diffed against the decoded panorama frame; the largest connected diff
        // blob = the hand (stars are sparse single pixels). Output: one .bin per
        // position {x_anchor,y_anchor,w,h,u16s then w*h indices}, consumed by the
        // engine as REAL-CAPTURE interim art while the mesh-renderer port lands.
        if std::env::var("HANDATLAS").is_ok() {
            let pano = std::fs::read("output/_tmp_iso/TB.BIG")
                .ok()
                .and_then(commander_blood_tools::tbbig::BridgePanorama::parse)
                .expect("TB.BIG");
            let bg = pano.frame_pixels(55).unwrap();
            // Dense grid: every ~40 px so the port can pick a near sprite for any
            // cursor position (smooth tracking). Plus the (40,100) rest state.
            let mut grid: Vec<(u16, u16)> = vec![(40, 100)];
            for gy in (50u16..=180).step_by(30) {
                for gx in (40u16..=280).step_by(40) {
                    grid.push((gx, gy));
                }
            }
            for (gx, gy) in grid {
                // Park the hardware cursor so the game's ring-anchored cursor sits
                // at screen (gx,gy) for view frame 55: ring = gx + 55*8 - 160.
                let ring = (gx as i32 + 55 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, gy);
                let _ = rt.run(rt.cpu.steps + 6_000_000);
                let (fr, _, _) = state(&rt);
                let live = rt.screen_indices();
                println!("capture at frame {fr}");
                // Diff vs the decoded panorama; collect blob pixels.
                let mut diff: Vec<usize> = (0..64000)
                    .filter(|&i| live[i] != bg[i] && live[i] != 0)
                    .collect();
                // Drop pixels with no diff neighbour (isolated stars).
                let set: std::collections::HashSet<usize> = diff.iter().copied().collect();
                diff.retain(|&i| {
                    let (x, y) = (i % 320, i / 320);
                    [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        (0..320).contains(&nx)
                            && (0..200).contains(&ny)
                            && set.contains(&((ny * 320 + nx) as usize))
                    })
                });
                if diff.len() < 50 {
                    println!("atlas ({gx},{gy}): no hand blob ({} px)", diff.len());
                    continue;
                }
                // Keep only the connected component nearest the cursor: the hand is
                // drawn at the cursor; the tutorial subtitle band / portrait are
                // separate far-away components.
                {
                    let set: std::collections::HashSet<usize> = diff.iter().copied().collect();
                    let mut seen: std::collections::HashSet<usize> = Default::default();
                    let mut best: Vec<usize> = Vec::new();
                    let mut best_d = i64::MAX;
                    for &start in &diff {
                        if seen.contains(&start) { continue; }
                        let mut comp = vec![start];
                        let mut stack = vec![start];
                        seen.insert(start);
                        while let Some(i) = stack.pop() {
                            let (x, y) = ((i % 320) as i32, (i / 320) as i32);
                            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                                let (nx, ny) = (x + dx, y + dy);
                                if !(0..320).contains(&nx) || !(0..200).contains(&ny) { continue; }
                                let n = (ny * 320 + nx) as usize;
                                if set.contains(&n) && seen.insert(n) {
                                    comp.push(n);
                                    stack.push(n);
                                }
                            }
                        }
                        if comp.len() < 200 { continue; } // stars / speckle
                        let d = comp
                            .iter()
                            .map(|&i| {
                                let (x, y) = ((i % 320) as i64, (i / 320) as i64);
                                (x - gx as i64).pow(2) + (y - gy as i64).pow(2)
                            })
                            .min()
                            .unwrap_or(i64::MAX);
                        if d < best_d {
                            best_d = d;
                            best = comp;
                        }
                    }
                    if best.is_empty() {
                        println!("atlas ({gx},{gy}): no component near cursor");
                        continue;
                    }
                    diff = best;
                }
                let (mut x0, mut y0, mut x1, mut y1) = (320usize, 200usize, 0usize, 0usize);
                for &i in &diff {
                    let (x, y) = (i % 320, i / 320);
                    x0 = x0.min(x); x1 = x1.max(x); y0 = y0.min(y); y1 = y1.max(y);
                }
                let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
                let mut sprite = vec![0u8; w * h]; // 0 = transparent
                for &i in &diff {
                    let (x, y) = (i % 320, i / 320);
                    sprite[(y - y0) * w + (x - x0)] = live[i];
                }
                let mut blob = Vec::new();
                for v in [gx as i32 - x0 as i32, gy as i32 - y0 as i32, w as i32, h as i32] {
                    blob.extend_from_slice(&(v as i16).to_le_bytes());
                }
                blob.extend_from_slice(&sprite);
                std::fs::write(out.join(format!("hand_{gx}_{gy}.bin")), &blob).unwrap();
                println!("atlas ({gx},{gy}): blob {}px bbox {w}x{h} at ({x0},{y0})", diff.len());
            }
            println!("HANDATLAS done -> {}/hand_*.bin", out.display());
            return;
        }

        // Dump the live 6-record station table (gs:0x2A1B, 0x18 stride) so the port
        // can mirror the real records: +0 flags, +0xA = 2*rest-frame seek target,
        // +0xC..0x13 = current bbox {w,h,x,y}.
        let dump_table = |rt: &Runtime, tag: &str| {
            for rec in 0..6u32 {
                let base = 0x2a1b + rec * 0x18;
                let words: Vec<String> = (0..12)
                    .map(|w| {
                        let lo = rt.m.read8(g, base + w * 2) as u16;
                        let hi = rt.m.read8(g, base + w * 2 + 1) as u16;
                        format!("{:04x}", lo | (hi << 8))
                    })
                    .collect();
                println!("station[{rec}] {tag}: {}", words.join(" "));
            }
        };
        dump_table(&rt, "console");
        // Candidate rotation inputs; after each, run a while and report the state words.
        // Find the pointing-hand drawer: watch a chunky back-buffer pixel inside
        // the hand (screen ~(85,130) at the console — the hand hovers there) and
        // report which code writes it + what ds:si (pixel source) it reads.
        {
            // 1. Locate the hand: park the cursor at two spots, snapshot the visible
            //    frame at each, and diff — the moved pixels are the hand sprite.
            rt.set_mouse_pos(160, 60);
            let _ = rt.run(rt.cpu.steps + 6_000_000);
            let a = rt.screen_indices();
            rt.set_mouse_pos(160, 160);
            let _ = rt.run(rt.cpu.steps + 6_000_000);
            let b = rt.screen_indices();
            let diff: Vec<usize> = (0..a.len().min(b.len())).filter(|&i| a[i] != b[i]).collect();
            let (mut x0, mut y0, mut x1, mut y1) = (320usize, 200usize, 0usize, 0usize);
            for &i in &diff {
                let (x, y) = (i % 320, i / 320);
                x0 = x0.min(x); x1 = x1.max(x); y0 = y0.min(y); y1 = y1.max(y);
            }
            println!("hand diff: {} px, bbox x{x0}..{x1} y{y0}..{y1}", diff.len());
            let mut hist = std::collections::HashMap::new();
            for &i in &diff { *hist.entry(b[i]).or_insert(0u32) += 1; }
            let mut top: Vec<_> = hist.into_iter().collect();
            top.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            println!("hand palette indices (top): {:?}", &top[..top.len().min(10)]);
            // 2. Watch writes of the hand's dominant colour (246) into the BACK
            //    BUFFER only (filters out same-valued data stores): the writers
            //    are the true rasterizer sites, ds:si their pixel source.
            let bb_off = rt.m.read8(g, 0x5229) as usize | ((rt.m.read8(g, 0x522a) as usize) << 8);
            let bb_seg = rt.m.read8(g, 0x522b) as usize | ((rt.m.read8(g, 0x522c) as usize) << 8);
            let bb = bb_seg * 16 + bb_off;
            println!("back buffer at linear {bb:#x}");
            rt.m.watch = Some((246, 0..0x100000));
            rt.m.watch_hits.clear();
            let _ = rt.run(rt.cpu.steps + 4_000_000);
            for &(cs, ip, ds, si, addr) in rt.m.watch_hits.iter() {
                println!("hand colour write from {cs:04x}:{ip:04x} -> linear {addr:#07x} (ds:si={ds:04x}:{si:04x})");
            }
            rt.m.watch = None;
            // 3. Identify the writer segment's contents (dynamic overlay?): dump it
            //    plus the source data segments for offline matching.
            if let Some(&(cs, _, ds, _, _)) = rt.m.watch_hits.first() {
                for (seg, len, tag) in [(cs, 0x4000usize, "code"), (ds, 0x4000, "data")] {
                    let base = (seg as usize) * 16;
                    std::fs::write(
                        out.join(format!("hand_{tag}_{seg:04x}.bin")),
                        &rt.m.mem[base..(base + len).min(rt.m.mem.len())],
                    )
                    .unwrap();
                }
                println!("hand code/data segments dumped");
            }
        }
        // Vertex-buffer INIT trace: who seeds the runtime vertex records past the
        // image (data:0xE000+)? trace_range during the whole skip-to-console run
        // was armed in main before boot when VERTINIT is set (see below), so just
        // report here.
        if std::env::var("VERTINIT").is_ok() {
            let mut seen = std::collections::HashSet::new();
            for &(addr, v, cs, ip) in rt.m.range_hits.iter() {
                if seen.insert((cs, ip)) {
                    println!("vert init write: {cs:04x}:{ip:04x} -> {addr:#07x} = {v:#04x}");
                }
                if seen.len() > 20 { break; }
            }
            println!("vert init: {} total hits", rt.m.range_hits.len());
        }
        // Dump manu3's live table heads + the face/vertex tables they point to
        // (the bank relocation fills these; statics in the file are stale).
        {
            // manu3's data segment is cs:[0x136A], patched at load time.
            let m3cs = 0x166cu16;
            let m3 = rt.m.read8(m3cs, 0x136a) as u16 | ((rt.m.read8(m3cs, 0x136b) as u16) << 8);
            println!("manu3 data segment = {m3:04x}");
            macro_rules! rw {
                ($off:expr) => {
                    (rt.m.read8(m3, $off) as u16 | ((rt.m.read8(m3, $off + 1) as u16) << 8))
                };
            }
            println!("manu3 data:[0]={:#06x} [2]={:#06x} [4]={:#06x} [6]={:#06x} [8]={:#06x}",
                rw!(0), rw!(2), rw!(4), rw!(6), rw!(8));
            // Dump the vertex-buffer segment (fs:[2]) so the runtime vertex records
            // (face-table targets) can be decoded offline.
            {
                let vseg = rw!(2);
                let base = (vseg as usize) * 16;
                let dump: Vec<u8> = rt.m.mem[base..(base + 0x10000).min(rt.m.mem.len())].to_vec();
                std::fs::write(out.join(format!("manu3_vertseg_{vseg:04x}.bin")), &dump).unwrap();
            }
            // Catch the REAL vertex-record addresses: read-watch the vertex segment
            // for a slice of console time; reads cluster at the live records.
            let vseg_base = rw!(2) as usize * 16;
            {
                let vseg = vseg_base;
                rt.m.read_watch = Some(vseg..vseg + 0x10000);
                rt.m.read_hits.borrow_mut().clear();
                let _ = rt.run(rt.cpu.steps + 600_000);
                rt.m.read_watch = None;
                let hits = rt.m.read_hits.borrow();
                let mut addrs: Vec<usize> = hits.iter().map(|h| h.0 - vseg).collect();
                addrs.sort_unstable();
                addrs.dedup();
                println!("vertex-seg reads: {} sites, offsets {:x?}", addrs.len(),
                    &addrs[..addrs.len().min(24)]);
            }
            let (faces, nfaces) = (rw!(0x2300), rw!(0x2304));
            let (recs, nrecs) = (rw!(0x22fa), rw!(0x22fe));
            println!("manu3 live: faces@{faces:#06x} n={nfaces:#x} pose-recs@{recs:#06x} n={nrecs:#x} root={:#06x} list2={:#06x}", rw!(0x2248), rw!(0x224a));
            let mut dump = Vec::new();
            for i in 0..0x6000u32 {
                dump.push(rt.m.read8(m3, faces as u32 + i));
            }
            std::fs::write(out.join("manu3_face_table.bin"), &dump).unwrap();
        }
        // Coverage-map manu3's segment for one second of console time: which code
        // runs per frame (entry points by hit count) — the decompile worklist.
        {
            rt.m.coverage_seg = Some(0x166c);
            rt.m.coverage = vec![0u32; 65536];
            let _ = rt.run(rt.cpu.steps + 8_000_000);
            rt.m.coverage_seg = None;
            // Report basic-block heads: covered ip whose predecessor byte is uncovered.
            let cov = std::mem::take(&mut rt.m.coverage);
            let mut blocks: Vec<(u32, usize)> = Vec::new();
            for ip in 1..65536usize {
                if cov[ip] > 0 && cov[ip - 1] == 0 {
                    blocks.push((cov[ip], ip));
                }
            }
            blocks.sort_unstable_by(|a, b| b.cmp(a));
            for (hits, ip) in blocks.iter().take(30) {
                println!("manu3 hot entry {ip:#06x} ({hits} hits)");
            }
            println!("manu3 covered bytes: {}", cov.iter().filter(|&&c| c > 0).count());
            // Contiguous covered spans (allowing <=8-byte gaps for instruction bodies):
            // the decompile worklist, written as "start end max_hits" lines.
            let mut spans = String::new();
            let mut start: Option<usize> = None;
            let mut gap = 0usize;
            let mut peak = 0u32;
            for ip in 0..65536usize {
                if cov[ip] > 0 {
                    if start.is_none() { start = Some(ip); peak = 0; }
                    peak = peak.max(cov[ip]);
                    gap = 0;
                } else if let Some(s0) = start {
                    gap += 1;
                    if gap > 8 {
                        spans.push_str(&format!("{:#06x} {:#06x} {}\n", s0, ip - gap, peak));
                        start = None;
                    }
                }
            }
            std::fs::write(out.join("manu3_coverage_spans.txt"), spans).unwrap();
        }
        // Who READS the manu3 "3DB0" mesh bank (file 0x3642.. at seg 0x166C)?
        {
            let bank = 0x166cusize * 16 + 0x3642;
            rt.m.read_watch = Some(bank..bank + 0x60);
            let _ = rt.run(rt.cpu.steps + 4_000_000);
            for &(addr, cs, ip) in rt.m.read_hits.borrow().iter() {
                println!("bank read: {cs:04x}:{ip:04x} <- linear {addr:#07x} (manu3+{:#x})", addr - 0x166c0);
            }
            rt.m.read_watch = None;
        }
        // Dump the bridge overlay entity records (gs:0x6212 + id*32, ids 0x10..0x20
        // — page_flip commits 0x15..0x1F) to find the pointing-hand's pixel source.
        for id in 0x10u32..0x20 {
            let base = 0x6212 + id * 32;
            let words: Vec<String> = (0..16)
                .map(|w| {
                    let lo = rt.m.read8(g, base + w * 2) as u16;
                    let hi = rt.m.read8(g, base + w * 2 + 1) as u16;
                    format!("{:04x}", lo | (hi << 8))
                })
                .collect();
            println!("entity[{id:#04x}] {}", words.join(" "));
        }
        // Rotate around the FULL panorama ring by repeatedly parking the cursor at
        // ring positions ahead of the view (the steering law chases it), capturing
        // each stop — live references of every bridge sector (nav pyramids, Orxx).
        for stop in 1..=12u32 {
            // Aim the cursor 120 ring px ahead of the current view centre.
            let (fr, _, _) = state(&rt);
            let target = ((fr as u32 * 8 + 280) % 1440) as u16;
            rt.set_mouse_pos(target, 100);
            let _ = rt.run(rt.cpu.steps + 10_000_000);
            let (fr, ang, st) = state(&rt);
            println!("rotate stop {stop}: tb_frame={fr} angle={ang:#x} station={st:#x}");
            rt.write_ppm(&out.join(format!("rotate_{stop:02}_f{fr}.ppm"))).unwrap();
        }
        println!("BRIDGEPROBE done -> {}/bridge_*.ppm + rotate_*.ppm", out.display());
        return;
    }

    if let Ok(spec) = std::env::var("LINDUMP") {
        // Dump LIVE bytes at a LINEAR address after resuming the hub state —
        // for code sites whose file bytes are relocation-patched. Spec "<linhex>:<len>".
        let parts: Vec<&str> = spec.split(':').collect();
        let lin = usize::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap();
        let len: usize = parts[1].parse().unwrap();
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let bytes = &rt.m.mem[lin..lin + len];
        println!("LINDUMP {lin:#x}+{len}:");
        for (i, ch) in bytes.chunks(16).enumerate() {
            let hexs: Vec<String> = ch.iter().map(|b| format!("{b:02x}")).collect();
            println!("  {:#07x}: {}", lin + i * 16, hexs.join(" "));
        }
        return;
    }
    if let Ok(spec) = std::env::var("MEMDUMP") {
        // Dump N bytes at gs:<off> to a file after running to `steps`. Spec: "<offhex>:<len>:<path>".
        let parts: Vec<&str> = spec.split(':').collect();
        let off = u32::from_str_radix(parts[0].trim_start_matches("0x"), 16).unwrap();
        let len: u32 = parts[1].parse().unwrap();
        let path = parts[2];
        let _ = rt.run(steps);
        let g = 0x0e84u16;
        let bytes: Vec<u8> = (0..len).map(|i| rt.m.read8(g, off + i)).collect();
        std::fs::write(path, &bytes).unwrap();
        println!("MEMDUMP gs:{off:#06x} {len} bytes -> {path} @ {} steps", rt.cpu.steps);
        return;
    }

    if let Ok(needle) = std::env::var("MEMFIND") {
        // Run to `steps`, then scan all of guest RAM for the needle — decisive test of
        // whether a given pattern is resident. ASCII by default; a `hex:` prefix (e.g.
        // `hex:a301007a26`) searches for raw bytes (used to locate the loaded SCRIPT*.BAS
        // by its 0xA3 concept-menu table bytes).
        let _ = rt.run(steps);
        let hexbuf: Vec<u8>;
        let pat: &[u8] = if let Some(hx) = needle.strip_prefix("hex:") {
            hexbuf = (0..hx.len() / 2)
                .map(|i| u8::from_str_radix(&hx[i * 2..i * 2 + 2], 16).unwrap_or(0))
                .collect();
            &hexbuf
        } else {
            needle.as_bytes()
        };
        let mem = &rt.m.mem;
        let mut hits = 0;
        for i in 0..mem.len().saturating_sub(pat.len()) {
            if &mem[i..i + pat.len()] == pat {
                let gs = 0x0e84u32 * 16;
                let rel = if (i as u32) >= gs { format!("gs:{:#06x}", i as u32 - gs) } else { String::new() };
                println!("  found {needle:?} at linear {i:#08x} {rel}");
                hits += 1;
                if hits >= 8 { println!("  (more; stopping at 8)"); break; }
            }
        }
        println!("MEMFIND {needle:?}: {hits} hit(s) at {} steps", rt.cpu.steps);
        return;
    }

    // BASSTEP: trace the BAS conversation VM's program counter (si) at the dispatch
    // (067c:0309, the `lodsb` reading each opcode) while triggering a topic — reveals
    // the exact opcode-walk structure (how a menu's topics/blocks are traversed), the
    // last piece needed to write the clean-Rust BasConversationVm executor.
    if std::env::var("BASSTEP").is_ok() {
        let g = 0x0e84u16;
        rt.cpu.si_trace_at = Some((0x067c, 0x0309));
        rt.cpu.si_trace_log.clear();
        let fr = rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        let ring = (200i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
        rt.set_mouse_pos(ring, 75); // FEAR (row 1 of the fear/anger menu)
        let _ = rt.run(rt.cpu.steps + 400_000);
        rt.mouse_press(0);
        let _ = rt.run(rt.cpu.steps + 250_000);
        rt.mouse_release(0);
        let _ = rt.run(rt.cpu.steps + 3_000_000);
        let log = std::mem::take(&mut rt.cpu.si_trace_log);
        rt.cpu.si_trace_at = None;
        println!("BASSTEP: {} dispatch steps (si @ 067c:0309 = BAS offset, op = byte there)", log.len());
        for (i, &(si, op)) in log.iter().enumerate().take(64) {
            let m = if op == 0xa3 { " <MENU>" } else if op == 0xa6 { " <TEXT>" } else { "" };
            println!("  [{i:2}] si={si:#06x} op={op:#04x}{m}");
        }
        return;
    }

    // SELWATCH: watch writes to the topic-selection word gs:0x6762 while clicking a
    // topic, logging the value + writer cs:ip — finds the INPUT HANDLER that turns a
    // topic click into a selection, the entry to the record-update/branching logic.
    if std::env::var("SELWATCH").is_ok() {
        let g = 0x0e84u16;
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        rt.m.watch_addr = Some(0xe84usize * 16 + 0x6762);
        rt.m.addr_hits.clear();
        let fr = frame(&rt);
        let ring = (200i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
        rt.set_mouse_pos(ring, 75); // FEAR row
        let _ = rt.run(rt.cpu.steps + 400_000);
        rt.mouse_press(0);
        let _ = rt.run(rt.cpu.steps + 250_000);
        rt.mouse_release(0);
        let _ = rt.run(rt.cpu.steps + 3_000_000);
        rt.m.watch_addr = None;
        println!("SELWATCH: {} writes to gs:0x6762 (selection)", rt.m.addr_hits.len());
        let mut seen = std::collections::HashSet::new();
        let mut input_cs = None;
        for &(v, cs, ip) in rt.m.addr_hits.iter() {
            if seen.insert((cs, ip)) {
                println!("  gs:0x6762 = {v:#04x} written by {cs:04x}:{ip:04x}");
                if cs != 0x067c {
                    input_cs = Some(cs);
                }
            }
        }
        if let Some(cs) = input_cs {
            let base = (cs as usize) * 16;
            let dump = rt.m.mem[base..(base + 0x2000).min(rt.m.mem.len())].to_vec();
            std::fs::write(out.join(format!("input_code_{cs:04x}.bin")), &dump).unwrap();
            println!("SELWATCH: dumped input-handler segment {cs:04x} -> input_code_{cs:04x}.bin");
        }
        return;
    }

    // MENUWATCH: watch writes to the current-menu word gs:0x6772 while driving a
    // conversation, logging each menu change + the code (cs:ip) that made it — reveals
    // the push/pop routines and every menu transition empirically (the ground truth
    // for the clean-port conversation VM's navigation).
    if std::env::var("MENUWATCH").is_ok() {
        let g = 0x0e84u16;
        let cur = |rt: &Runtime| rt.m.read8(g, 0x6772) as u16 | ((rt.m.read8(g, 0x6773) as u16) << 8);
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        println!("MENUWATCH start menu = {:#06x}", cur(&rt));
        rt.m.watch_addr = Some(0xe84usize * 16 + 0x6772);
        rt.m.addr_hits.clear();
        let mut distinct = std::collections::BTreeSet::new();
        distinct.insert(cur(&rt));
        let npass: u32 = std::env::var("MW_PASSES").ok().and_then(|s| s.parse().ok()).unwrap_or(12);
        for pass in 0..npass {
            let fr = frame(&rt);
            // rotate through the topic rows + the orb, to provoke navigation.
            let (mx, my) = if pass % 4 == 3 { (125u16, 118u16) } else { (200, 61 + 11 * (pass % 7) as u16 + 3) };
            let ring = (mx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, my);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 250_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 2_000_000);
            distinct.insert(cur(&rt));
        }
        rt.m.watch_addr = None;
        println!("MENUWATCH: distinct menus reached: {:x?}", distinct);
        println!("MENUWATCH: {} writes to gs:0x6772", rt.m.addr_hits.len());
        let mut seen = std::collections::HashSet::new();
        for &(v, cs, ip) in rt.m.addr_hits.iter() {
            if seen.insert((v, cs, ip)) {
                println!("  gs:0x6772 low={v:#04x} written by {cs:04x}:{ip:04x}");
            }
        }
        println!("MENUWATCH end menu = {:#06x}", cur(&rt));
        return;
    }

    // MENUTREE: empirically map a concept menu's topic→sub-menu branch targets by
    // clicking each topic (reloading the savestate to isolate each) and reading the
    // resulting current-menu offset gs:0x6772. Gives the ground-truth navigation the
    // clean-port conversation VM must reproduce, without a full static VM decode.
    if std::env::var("CONVDRIVER").is_ok() {
        // OCR-driven conversation player: from the hub savestate, read the live
        // subtitle/menu text with the game's own font; click bye_bye/goodbye topics
        // when visible, else advance via the orb — until the presentation frees.
        let g = 0x0e84u16;
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
            let read_font = |rt: &Runtime| -> Vec<(char, [u8; 8])> {
                let mut out = Vec::new();
                for code in 32u8..127 {
                    let gi = rt.m.read8(g, 0x70fa + code as u32);
                    if gi == 0xFF { continue; }
                    let mut rows = [0u8; 8];
                    for (i, r) in rows.iter_mut().enumerate() {
                        *r = rt.m.read8(g, 0x71aa + gi as u32 * 8 + i as u32);
                    }
                    let lit: u32 = rows.iter().map(|r| r.count_ones()).sum();
                    if lit >= 3 {
                        out.push((code as char, rows));
                    }
                }
                out.sort_by_key(|(_, rows)| {
                    std::cmp::Reverse(rows.iter().map(|r| r.count_ones()).sum::<u32>())
                });
                out
            };
            let ocr = |idx: &[u8], font: &[(char, [u8; 8])]| -> String {
                let lit = |px: u8| px == 0xE0 || px == 0xEE || px == 0xEF || px >= 0xFD;
                let mut text = String::new();
                // Dynamic alignment: subtitle rows differ per screen (console
                // 8/18; scene close-ups draw 3 lines from the very top). Find
                // aligned rows by trying each and keeping non-empty reads.
                let mut row0 = 0usize;
                while row0 < 40 {
                    let mut line = String::new();
                    let mut blanks = 0usize;
                    let mut x = 0usize;
                    while x < 313 {
                        let mut got = None;
                        for (ch, rows) in font {
                            let mut ok = true;
                            'cell: for gy in 0..8usize {
                                for gx in 0..8usize {
                                    let on = (rows[gy] >> (7 - gx)) & 1 == 1;
                                    let px = idx
                                        .get((row0 + gy) * 320 + x + gx)
                                        .copied()
                                        .unwrap_or(0);
                                    if on != lit(px) {
                                        ok = false;
                                        break 'cell;
                                    }
                                }
                            }
                            if ok {
                                got = Some(*ch);
                                break;
                            }
                        }
                        if let Some(ch) = got {
                            if blanks >= 8 && !line.is_empty() {
                                line.push(' ');
                            }
                            line.push(ch);
                            blanks = 0;
                            x += 8;
                        } else {
                            blanks += 1;
                            x += 1;
                        }
                    }
                    if line.chars().filter(|c| c.is_ascii_alphanumeric()).count() >= 3 {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&line);
                        row0 += 9; // next line
                    } else {
                        row0 += 1;
                    }
                }
                text
            };
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        let presenting = |rt: &Runtime| rt.m.read8(g, 0x2793) & 4 != 0;
        let font = read_font(&rt);
        let mut clicked_rows: Vec<usize> = Vec::new();
        for round in 0..120 {
            if !presenting(&rt) {
                println!("CONVDRIVER: presentation FREED at round {round}");
                rt.save_state(std::path::Path::new("accuracy/hub_idle.state")).unwrap();
                println!("hub_idle.state saved");
                return;
            }
            // OCR the screen text.
            let idx = rt.screen_indices();
            let text = ocr(&idx, &font);
            let lower = text.to_lowercase();
            if round % 10 == 0 {
                println!("round {round}: {}", &text.chars().take(70).collect::<String>());
            }
            // Menu visible? Try the goodbye rows (bye/adieu variants).
            let fr = frame(&rt) as i32;
            let click = |rt: &mut Runtime, sx: i32, sy: u16| {
                let ring = (sx + fr * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 3_000_000);
            };
            if lower.contains("bye") || lower.contains("adieu") || lower.contains("later") {
                // Click each menu row once, prioritizing unclicked ones.
                for row in 0..9 {
                    if !clicked_rows.contains(&row) {
                        clicked_rows.push(row);
                        click(&mut rt, 200, (61 + 11 * row + 3) as u16);
                        break;
                    }
                }
            } else {
                click(&mut rt, 125, 118); // advance
            }
            let _ = rt.run(rt.cpu.steps + 3_000_000);
        }
        // Round-2 probe: the golden-menu rows (station records), watching EVERY
        // observable (presentation bit, FSM state, text, opened files).
        for (name, y) in [("HONK", 88u16), ("TELEPHONE", 103), ("CRYOBOX", 118), ("MENU", 133), ("OPTION", 148)] {
            rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
            let fr = frame(&rt) as i32;
            let ring = (230 + fr * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, y);
            let _ = rt.run(rt.cpu.steps + 500_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 300_000);
            rt.mouse_release(0);
            let before_files = rt.opened_files.len();
            let _ = rt.run(rt.cpu.steps + 15_000_000);
            let idx = rt.screen_indices();
            let text = ocr(&idx, &font);
            let fsm = rt.m.read8(g, 0x6788);
            println!(
                "{name}: pres={} fsm={} files+{} text='{}'",
                presenting(&rt),
                fsm,
                rt.opened_files.len() - before_files,
                text.chars().take(50).collect::<String>()
            );
        }
        println!("CONVDRIVER round-2 complete");
        return;
    }
    if std::env::var("PLAYTO").is_ok() {
        // Play forward from the hub toward a LOCATION VISIT using the decoded grammar:
        // advance the presentation via the orb until idle, steer to the nav sector,
        // open the destination box, choose row 0, then run the travel — dumping frames
        // and saving a location savestate at the end (unlocks the entity-stepper watch).
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let g = 0x0e84u16;
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        let presenting = |rt: &Runtime| rt.m.read8(g, 0x2793) & 4 != 0;
        let click = |rt: &mut Runtime, sx: i32, sy: u16| {
            let fr = frame(rt) as i32;
            let ring = (sx + fr * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, sy);
            let _ = rt.run(rt.cpu.steps + 300_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 300_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 4_000_000);
        };
        // 1) find the conversation EXIT: try each concept-menu row (x~200, y=61+11i,
        // MENUTREE geometry) — the row that clears [0x2793]&4 is the goodbye topic.
        let mut exited = false;
        for row in 0..9 {
            rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
            let my = 61 + 11 * row + 3;
            click(&mut rt, 200, my as u16);
            let _ = rt.run(rt.cpu.steps + 10_000_000);
            if !presenting(&rt) {
                println!("EXIT topic = row {row} (y{my}) — presentation freed");
                exited = true;
                break;
            }
        }
        if !exited {
            println!("no exit row freed the presentation; advancing anyway");
            // fall back: advance clicks
            for _ in 0..30 {
                if !presenting(&rt) {
                    break;
                }
                click(&mut rt, 125, 118);
            }
        }
        rt.write_ppm(&out.join("pt_idle.ppm")).unwrap();
        // 2) steer to the nav sector (park past it; trail lands ~95).
        for _ in 0..6 {
            rt.set_mouse_pos(880, 100);
            let _ = rt.run(rt.cpu.steps + 3_000_000);
        }
        println!("steered to frame {}", frame(&rt));
        rt.write_ppm(&out.join("pt_nav.ppm")).unwrap();
        // 3) open the destination box via the nav orb, then choose row 0.
        click(&mut rt, 105, 148);
        rt.write_ppm(&out.join("pt_box.ppm")).unwrap();
        click(&mut rt, 75, 97);
        // 4) run the travel/arrival, dumping along the way.
        for i in 0..8 {
            let _ = rt.run(rt.cpu.steps + 10_000_000);
            rt.write_ppm(&out.join(format!("pt_travel_{i}.ppm"))).unwrap();
        }
        rt.save_state(std::path::Path::new("accuracy/location_visit.state")).unwrap();
        println!("PLAYTO done @ {} steps; location_visit.state saved", rt.cpu.steps);
        // Opened files reveal what loaded (travel film? location assets?).
        let mut seen = std::collections::HashSet::new();
        for (step, path) in &rt.opened_files {
            if seen.insert(path.clone()) && *step > 2_000_000_000 {
                println!("  @{step} {path}");
            }
        }
        return;
    }
    if std::env::var("REGIONDUMP").is_ok() {
        // Dump the LIVE ui-region table (32 x 32B descending from ds:0x65F2) at the hub.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let g = 0x0e84u16;
        if std::env::var("UNPIN").is_ok() {
            let v = rt.m.read8(g, 0x2793);
            rt.m.write8(g, 0x2793, v & !4);
            let _ = rt.run(rt.cpu.steps + 8_000_000);
        }
        for rid in (0..=0x1Fu32).rev() {
            let base = 0x65F2 - (0x1F - rid) * 0x20;
            let flag = rt.m.read8(g, base);
            let rd = |off: u32| {
                (rt.m.read8(g, base + off) as u16 | (rt.m.read8(g, base + off + 1) as u16) << 8)
                    as i16
            };
            let (x, y, w, h) = (rd(8), rd(10), rd(12), rd(14));
            if flag != 0 || x != 0 || w != 0 {
                println!(
                    "region {rid:#04x}: flag={flag:#04x} rect=({x},{y} {w}x{h}) screen_x~{}",
                    x as i32 - 45 * 8 + 160
                );
            }
        }
        return;
    }
    if std::env::var("CALLERWATCH").is_ok() {
        // Find WHO far-calls the manu3 overlay: resume the hub, single-step until CS
        // enters the manu3 code segment (0x166C), then read the far return address
        // (caller CS:IP) off the stack.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let gseg = 0x0e84u16;
        let fr = rt.m.read8(gseg, 0x2795) as u16 | ((rt.m.read8(gseg, 0x2796) as u16) << 8);
        let ring = (230i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
        rt.set_mouse_pos(ring, 103);
        let mut found = 0;
        for _ in 0..30_000_000u64 {
            let _ = rt.run(rt.cpu.steps + 1);
            if rt.cpu.cs == 0x166C {
                let ss = rt.m.regs.ss;
                let sp = rt.m.regs.esp as u16;
                let rip = rt.m.read8(ss, sp as u32) as u16
                    | (rt.m.read8(ss, sp as u32 + 1) as u16) << 8;
                let rcs = rt.m.read8(ss, sp as u32 + 2) as u16
                    | (rt.m.read8(ss, sp as u32 + 3) as u16) << 8;
                println!(
                    "manu3 entered at ip={:#06x} from caller {:#06x}:{:#06x} (steps {})",
                    rt.cpu.ip, rcs, rip, rt.cpu.steps
                );
                found += 1;
                if found >= 4 {
                    break;
                }
                let _ = rt.run(rt.cpu.steps + 200_000); // skip past this call
            }
        }
        if found == 0 {
            println!("CALLERWATCH: manu3 never entered in the window");
        }
        return;
    }
    if std::env::var("XDBDUMP").is_ok() {
        // Dump the RESIDENT manu3.xdb segment from the savestate: find the load segment by
        // scanning memory for the file's head bytes, then dump the relocated DATA segment
        // (init-filled: real face/vertex tables) for offline mesh extraction.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        // Replicate the probe conditions that visibly render the hand: set a ring-space
        // mouse position and run enough frames (the hand draw follows mouse activity).
        let gseg = 0x0e84u16;
        let fr = rt.m.read8(gseg, 0x2795) as u16 | ((rt.m.read8(gseg, 0x2796) as u16) << 8);
        let ring = (230i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
        rt.set_mouse_pos(ring, 103);
        let _ = rt.run(rt.cpu.steps + 12_000_000);
        let head = std::fs::read("output/_tmp_dat/manu3.xdb").unwrap();
        let sig = &head[0..16];
        let mut found = None;
        for seg in (0x1000u32..0xA000).step_by(1) {
            let mut ok = true;
            for (i, &b) in sig.iter().enumerate() {
                if rt.m.read8(seg as u16, i as u32) != b {
                    ok = false;
                    break;
                }
            }
            if ok {
                found = Some(seg as u16);
                break;
            }
        }
        let Some(seg) = found else {
            println!("XDBDUMP: manu3 signature not found in memory");
            return;
        };
        let delta = 0x137u16; // cs:[0x1368]
        let ds = seg + delta;
        println!("XDBDUMP: manu3 code seg {seg:#06x}, data seg {ds:#06x}");
        let mut data = Vec::with_capacity(0x10000);
        for off in 0..0x10000u32 {
            data.push(rt.m.read8(ds, off));
        }
        std::fs::write(out.join("manu3_ds.bin"), &data).unwrap();
        // Also dump the DERIVED segments (ds:[2]/[4]/[6]) — the vertex pools/work areas
        // may live there (the render code switches ds to fs:[2] mid-pipeline).
        for cell in [2u32, 4, 6] {
            let seg = rt.m.read8(ds, cell) as u16 | ((rt.m.read8(ds, cell + 1) as u16) << 8);
            let mut sd = Vec::with_capacity(0x10000);
            for off in 0..0x10000u32 {
                sd.push(rt.m.read8(seg, off));
            }
            std::fs::write(out.join(format!("manu3_seg{cell}_{seg:04x}.bin")), &sd).unwrap();
            println!("wrote seg[{cell}] = {seg:#06x}");
        }
        println!("wrote manu3_ds.bin (64KB live data segment)");
        return;
    }
    if std::env::var("REVEALDUMP").is_ok() {
        // Subtitle-reveal cadence ground truth: resume the hub, CANCEL the running
        // presentation, then click the HONK row — its actor presentation starts a
        // FRESH line ('What do you want Commander ?'), which we capture EVERY frame
        // (PPM + raw indices) so the port can match reveal rate, glyph colors, and
        // settle style. SB playback state is logged per shot (the honk chatter).
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let g = 0x0e84u16;
        // Ring x from the CURRENT frame at click time — the view can rotate after
        // CANCEL, so a load-time frame maps clicks to the wrong screen x.
        let frame_now = |rt: &Runtime| {
            rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8)
        };
        let click = |rt: &mut Runtime, sx: i32, sy: u16| {
            let ring =
                ((sx + frame_now(rt) as i32 * 8 - 160).rem_euclid(1440)) as u16;
            rt.set_mouse_pos(ring, sy);
            let _ = rt.run(rt.cpu.steps + 400_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 300_000);
            rt.mouse_release(0);
        };
        click(&mut rt, 100, 98); // CANCEL the arrival presentation (the real gate)
        let _ = rt.run(rt.cpu.steps + 30_000_000); // full teardown (the scenario's wait 20)
        // The HONK console row -> fresh presentation. NO settle after: capture starts
        // at the click so the reveal is caught mid-flight.
        click(&mut rt, 230, 88);
        for i in 0..160 {
            let _ = rt.run(rt.cpu.steps + 150_000);
            rt.write_ppm(&out.join(format!("rv_{i:03}.ppm"))).unwrap();
            std::fs::write(out.join(format!("rv_{i:03}.idx")), rt.screen_indices()).unwrap();
        }
        for &(step, len, rate) in &rt.sb_play_log {
            println!("sb start: step {step} len {len} rate {rate}");
        }
        std::fs::write(out.join("rv_dac.bin"), rt.dac).unwrap();
        println!("REVEALDUMP done");
        return;
    }
    if let Ok(scenario) = std::env::var("VERIFYSCRIPT") {
        // DUAL-RUN DIFFERENTIAL, oracle side: execute a scenario file against the
        // REAL game (resume the hub state, then per line: an action + settle), and
        // write one settled frame (PPM + raw indices) per step into boot_frames/vs_*.
        // The port runs the same scenario via `verify_port`; tools/verify_compare.py
        // scores every step. Scenario line format (TSV):
        //   move <x> <y> | click <x> <y> | key <scancode> | wait <frames>
        // Coordinates are SCREEN coords; the ring correction is applied here.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        // Optional write watch during the scenario (WRITEWATCHLIN=<linear hex>):
        // reports every write to the address with the writer's cs:ip — the story
        // event tracer (e.g. the scr record slot at block+0x1276).
        let mut rec_watch: Option<u32> = None;
        if let Ok(w) = std::env::var("WRITEWATCHLIN") {
            if let Some(recspec) = w.strip_prefix("rec:") {
                rec_watch =
                    Some(u32::from_str_radix(recspec.trim_start_matches("0x"), 16).unwrap());
                eprintln!("record write-watch armed at block+{:#x}", rec_watch.unwrap());
            } else {
                let lin = usize::from_str_radix(w.trim_start_matches("0x"), 16).unwrap();
                rt.m.watch_addr = Some(lin);
                eprintln!("write-watch armed at linear {lin:#x}");
            }
        }
        let g = 0x0e84u16;
        let w16 = |rt: &Runtime, a: u32| {
            rt.m.read8(g, a) as u32 | ((rt.m.read8(g, a + 1) as u32) << 8)
        };
        // RECDUMP=<off>,<off>,... : print the record block pointer and the named
        // record slots' values at scenario start and end (the story-state probe).
        let rec_dump = |rt: &Runtime, tag: &str| {
            if let Ok(spec) = std::env::var("RECDUMP") {
                let (boff, bseg) = (w16(rt, 0x6724), w16(rt, 0x6726));
                let base = bseg as usize * 16 + boff as usize;
                let mut line = format!("RECDUMP {tag}: block {bseg:04x}:{boff:04x}");
                for tok in spec.split(',') {
                    let off = u32::from_str_radix(tok.trim().trim_start_matches("0x"), 16)
                        .unwrap();
                    let lin = base + off as usize;
                    let v = rt.m.mem[lin] as u16 | ((rt.m.mem[lin + 1] as u16) << 8);
                    line += &format!(" [{off:#06x}]={v}");
                }
                println!("{line}");
            }
        };
        rec_dump(&rt, "start");
        // CHARDUMP: print the 0x60-byte character-slot block (gs:0x6CDE) — the
        // SETCHAR bindings (which crew/overlay occupies each slot).
        if std::env::var("CHARDUMP").is_ok() {
            let bytes: Vec<u8> = (0..0x60u32).map(|i| rt.m.read8(g, 0x6cde + i)).collect();
            for (i, ch) in bytes.chunks(16).enumerate() {
                let hex: String = ch.iter().map(|b| format!("{b:02x} ")).collect();
                let asc: String = ch
                    .iter()
                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                    .collect();
                println!("CHARDUMP +{:02x}: {hex} {asc}", i * 16);
            }
        }
        let frame = |rt: &Runtime| {
            rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8)
        };
        let text = std::fs::read_to_string(&scenario).expect("scenario file");
        let settle = 1_500_000u64;
        let mut step = 0usize;
        for line in text.lines() {
            let toks: Vec<&str> = line.split_whitespace().collect();
            if toks.is_empty() || toks[0].starts_with('#') {
                continue;
            }
            match toks[0] {
                "move" => {
                    let (sx, sy): (i32, u16) = (toks[1].parse().unwrap(), toks[2].parse().unwrap());
                    let ring = (sx + frame(&rt) as i32 * 8 - 160).rem_euclid(1440) as u16;
                    rt.set_mouse_pos(ring, sy);
                }
                "click" => {
                    let (sx, sy): (i32, u16) = (toks[1].parse().unwrap(), toks[2].parse().unwrap());
                    let ring = (sx + frame(&rt) as i32 * 8 - 160).rem_euclid(1440) as u16;
                    rt.set_mouse_pos(ring, sy);
                    let _ = rt.run(rt.cpu.steps + 400_000);
                    rt.mouse_press(0);
                    let _ = rt.run(rt.cpu.steps + 300_000);
                    rt.mouse_release(0);
                }
                // sclick <x> <y>: RAW screen coordinates (no ring conversion) — for
                // overlay UIs (the save-slot box) that read the mouse in screen space.
                // DOS-virtual mouse x is 0..639: screen column sx = virtual sx*2.
                "sclick" => {
                    let (sx, sy): (u16, u16) = (toks[1].parse().unwrap(), toks[2].parse().unwrap());
                    rt.set_mouse_pos(sx * 2, sy);
                    let _ = rt.run(rt.cpu.steps + 400_000);
                    rt.mouse_press(0);
                    let _ = rt.run(rt.cpu.steps + 300_000);
                    rt.mouse_release(0);
                }
                "key" => {
                    // key <scancode> [ascii] — the ASCII byte reaches BIOS-buffer
                    // consumers (the save-slot name entry polls int16 with ASCII).
                    let sc: u8 = toks[1].parse().unwrap();
                    let ascii: u8 = toks.get(2).map(|t| t.parse().unwrap()).unwrap_or(0);
                    rt.inject_key(sc, ascii);
                    let _ = rt.run(rt.cpu.steps + 200_000);
                    rt.inject_key(sc | 0x80, 0);
                }
                "wait" => {
                    let frames: u64 = toks[1].parse().unwrap();
                    let _ = rt.run(rt.cpu.steps + frames * 1_850_000);
                }
                // park <edge-x> <target-frame>: hold the cursor at screen edge x
                // (the player's rotate gesture) until the panorama reaches the
                // target frame (DS:0x2795) — closed-loop sector navigation.
                "park" => {
                    let (ex, target): (u16, u16) =
                        (toks[1].parse().unwrap(), toks[2].parse().unwrap());
                    for _ in 0..600 {
                        let d = (frame(&rt) as i32 - target as i32).rem_euclid(180);
                        if d.min(180 - d) <= 2 {
                            break;
                        }
                        let ring =
                            (ex as i32 + frame(&rt) as i32 * 8 - 160).rem_euclid(1440) as u16;
                        rt.set_mouse_pos(ring, 100);
                        let _ = rt.run(rt.cpu.steps + 1_000_000);
                    }
                    eprintln!("park: frame now {}", frame(&rt));
                }
                _ => {}
            }
            let _ = rt.run(rt.cpu.steps + settle);
            // SAYDUMP: print the subtitle display buffer (gs:0xE18, the 0x7612
            // string-sink target) as text each step — reads dialogue that the
            // frame captures only catch mid-reveal.
            if std::env::var("SAYDUMP").is_ok() {
                let bytes: Vec<u8> = (0..160u32).map(|i| rt.m.read8(g, 0xe18 + i)).collect();
                let text: String = bytes
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { ' ' })
                    .collect();
                if !text.trim().is_empty() {
                    println!("SAY step {step}: {}", text.trim());
                }
            }
            if let Some(off) = rec_watch {
                let (boff, bseg) = (w16(&rt, 0x6724), w16(&rt, 0x6726));
                if bseg > 0 {
                    let lin = bseg as usize * 16 + boff as usize + off as usize;
                    if rt.m.watch_addr != Some(lin) {
                        eprintln!(
                            "record-watch re-armed: block {bseg:04x}:{boff:04x} -> lin {lin:#x} @ step {step}"
                        );
                        rt.m.watch_addr = Some(lin);
                    }
                }
            }
            rt.write_ppm(&out.join(format!("vs_{step:03}.ppm"))).unwrap();
            std::fs::write(out.join(format!("vs_{step:03}.idx")), rt.screen_indices()).unwrap();
            step += 1;
        }
        rec_dump(&rt, "end");
        std::fs::write(out.join("vs_dac.bin"), rt.dac).unwrap();
        println!("VERIFYSCRIPT done: {step} steps");
        if !rt.m.addr_hits.is_empty() {
            println!("  write-watch hits:");
            let mut seen = std::collections::HashSet::new();
            for &(v, cs, ip) in &rt.m.addr_hits {
                if seen.insert((v, cs, ip)) {
                    println!("    ={v:#04x} at {cs:04x}:{ip:04x}");
                }
            }
        }
        println!("  bios_keys pending: {}, [0xB15]={:#04x}, [0x2738]={:#04x}, [0x272E]={}",
            rt.bios_keys.len(),
            rt.m.mem[0xE840 + 0xB15],
            rt.m.mem[0xE840 + 0x2738],
            rt.m.mem[0xE840 + 0x272E] as u16 | ((rt.m.mem[0xE840 + 0x272F] as u16) << 8));
        for (st, f) in rt.opened_files.iter().rev().take(8) {
            println!("  opened @{st}: {f}");
        }
        if std::env::var("SAVESAMPLE").is_ok() {
            rt.ip_sample = Some(Default::default());
            let _ = rt.run(rt.cpu.steps + 4_000_000);
            if let Some(h) = rt.ip_sample.take() {
                let mut v: Vec<_> = h.into_iter().collect();
                v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
                println!("  save-UI wait hot ips:");
                for ((cs, ip), n) in v.into_iter().take(12) {
                    println!("    {cs:04x}:{ip:04x} x{n}");
                }
            }
        }
        return;
    }
    if std::env::var("FRAMERATE").is_ok() {
        // Measure the REAL main-loop frame rate: count subtitle-pump gate reads /
        // frame-counter increments per emulated second at the hub. The frame counter
        // [0x0A40] (countdown) and the per-frame [0x27E0] gate change once per main
        // loop pass; sample a known per-frame cell across exact PIT time.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let g = 0x0e84u16;
        // ticks: PIT reprogrammed to 0x1746 (5958) -> 200.27 Hz. steps_per_tick from
        // the runtime diag ~= 39946. Run exactly 1000 ticks (~5 s) and count changes
        // of the frame counter byte [0x0A40].
        let _ = g;
        let steps_per_tick = 39946u64;
        // Count VGA page flips (CRTC start-address writes) = presented frames.
        let mut changes = 0u64;
        let mut last = (rt.crtc[0x0c], rt.crtc[0x0d]);
        for _ in 0..1000u64 {
            let _ = rt.run(rt.cpu.steps + steps_per_tick / 4);
            let v = (rt.crtc[0x0c], rt.crtc[0x0d]);
            if v != last {
                changes += 1;
                last = v;
            }
        }
        println!(
            "page flips in ~{:.2}s: {changes} -> {:.1} fps",
            1000.0 * (steps_per_tick as f64 / 4.0) / (39946.0 * 200.27),
            changes as f64 / (1000.0 * (steps_per_tick as f64 / 4.0) / (39946.0 * 200.27))
        );
        return;
    }
    if std::env::var("TEXDUMP").is_ok() {
        // Re-bank the manu3 hand texture from the CURRENT live state (the original
        // manu3_ds.bin dump shows noise in rows 40..63 — was it mid-load?).
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let _ = rt.run(rt.cpu.steps + 2_000_000);
        // manu3 ds runtime segment = 0x17A3 (labels); texture at ds:0x6400. The seam
        // faces' folded rows ((v>>8)+(v&0xFF), span-setup 0xE89..0xED0) reach row 245,
        // so bank 246 rows (the fill's addressable window from the texture base).
        let seg = 0x17A3u16;
        const ROWS: u32 = 246;
        let mut out_bytes = Vec::with_capacity((ROWS * 256) as usize);
        for off in 0x6400u32..(0x6400 + ROWS * 256) {
            out_bytes.push(rt.m.read8(seg, off));
        }
        std::fs::write(out.join("hand_tex_live.bin"), &out_bytes).unwrap();
        println!("TEXDUMP done ({} bytes)", out_bytes.len());
        return;
    }
    if let Ok(spec) = std::env::var("SEAMFS") {
        // Capture FS at the manu3 span-setup (default 0x120B, the uv/segment writer
        // the hub path uses) — then read the REAL fs parameter block (fs:[2] vertex
        // seg, fs:[4] TEXTURE seg, fs:[6] data seg) and dump 64 rows from the true
        // texture base.
        let ip = u16::from_str_radix(spec.trim_start_matches("0x"), 16).unwrap_or(0x120B);
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        rt.m.capture_ip = Some((0x166C, ip));
        rt.m.captured = None;
        rt.m.captured_fs = None;
        let _ = rt.run(rt.cpu.steps + 6_000_000);
        match rt.m.captured_fs {
            None => println!("SEAMFS: 166C:{ip:04x} never hit"),
            Some(fs) => {
                let w = |off: u32| {
                    rt.m.read8(fs, off) as u16 | ((rt.m.read8(fs, off + 1) as u16) << 8)
                };
                let (w0, vseg, tex, dseg) = (w(0), w(2), w(4), w(6));
                println!("SEAMFS: fs={fs:04x} [0]={w0:04x} vertexseg={vseg:04x} TEXseg={tex:04x} dataseg={dseg:04x}");
                let dump: Vec<u8> =
                    (0..64u32 * 256).map(|i| rt.m.read8(tex, i)).collect();
                std::fs::write(out.join("seamfs_tex.bin"), &dump).unwrap();
                println!("dumped 64 rows from {tex:04x}:0 -> seamfs_tex.bin");
            }
        }
        return;
    }
    if std::env::var("FSBLOCK").is_ok() {
        // Find the manu3 fill's fs parameter block: three consecutive words
        // {vertex seg, TEXTURE seg, data seg} at fs:[2]/[4]/[6]. Scan live memory
        // for candidate blocks whose [6] word equals the known data segment 0x17A3
        // and report the neighbours; then dump 64 rows from each texture-seg candidate.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let _ = rt.run(rt.cpu.steps + 2_000_000);
        let mut cands: Vec<(usize, u16, u16)> = Vec::new();
        for a in (0..0x9F000usize).step_by(2) {
            let w = rt.m.mem[a] as u16 | ((rt.m.mem[a + 1] as u16) << 8);
            if w == 0x17A3 && a >= 4 {
                let tex = rt.m.mem[a - 2] as u16 | ((rt.m.mem[a - 1] as u16) << 8);
                let vseg = rt.m.mem[a - 4] as u16 | ((rt.m.mem[a - 3] as u16) << 8);
                // Plausible segments only (below 640K, nonzero).
                if tex > 0x100 && tex < 0x9F00 && vseg > 0x100 && vseg < 0x9F00 {
                    cands.push((a - 6, vseg, tex));
                }
            }
        }
        println!("fs-block candidates (base, fs:[2] vertexseg, fs:[4] texseg):");
        for &(base, vseg, tex) in cands.iter().take(20) {
            println!("  fs={:04x} vseg={vseg:04x} tex={tex:04x}", base / 16);
            let lin = (tex as usize) * 16;
            let sample: Vec<u8> = (0..16).map(|i| rt.m.mem[lin + 45 * 256 + 64 + i]).collect();
            println!("    row45 sample: {sample:02x?}");
        }
        if let Some(&(_, _, tex)) = cands.first() {
            let lin = (tex as usize) * 16;
            let dump: Vec<u8> = (0..64 * 256).map(|i| rt.m.mem[lin + i]).collect();
            std::fs::write(out.join("fsblock_tex.bin"), &dump).unwrap();
            println!("dumped 64 rows from tex seg {tex:04x} -> fsblock_tex.bin");
        }
        return;
    }
    if let Ok(spec) = std::env::var("SEAMWATCH") {
        // Seam-face texture decode: watch writes to manu3's per-face fill slots
        // during live hub hand frames. Spec = hex ds-offset (default 623 = the HIGH
        // byte of [0x622], the texture-page selector the span setup shifts into the
        // segment). Logs every unique (value, cs:ip) writer.
        let off = u32::from_str_radix(spec.trim_start_matches("0x"), 16).unwrap_or(0x623);
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let _ = rt.run(rt.cpu.steps + 1_000_000);
        let seg: usize = std::env::var("SEAMSEG")
            .ok()
            .and_then(|s| usize::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0x17A3);
        rt.m.watch_addr = Some(seg * 16 + off as usize);
        let _ = rt.run(rt.cpu.steps + 8_000_000);
        let mut seen = std::collections::HashMap::new();
        for &(v, cs, ip) in &rt.m.addr_hits {
            *seen.entry((v, cs, ip)).or_insert(0u32) += 1;
        }
        let mut rows: Vec<_> = seen.into_iter().collect();
        rows.sort_by_key(|&((v, _, _), n)| (std::cmp::Reverse(n), v as u32));
        println!("writes to manu3 ds:{off:#x} ({} total):", rt.m.addr_hits.len());
        for ((v, cs, ip), n) in rows.into_iter().take(40) {
            println!("  ={v:#04x} x{n} at {cs:04x}:{ip:04x}");
        }
        // Also: does the manu3 overlay's 0xE80..0xF30 span-setup region even EXECUTE
        // at the hub? Sample cs:ip for a couple of frames and report overlay hits.
        rt.ip_sample = Some(Default::default());
        let _ = rt.run(rt.cpu.steps + 4_000_000);
        if let Some(h) = rt.ip_sample.take() {
            let mut manu: Vec<_> = h
                .iter()
                .filter(|&(&(cs, _), _)| cs == 0x166C)
                .map(|(&(_, ip), &n)| (ip, n))
                .collect();
            manu.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            println!("manu3 overlay (cs=166C) hot ips: {:?}", &manu[..manu.len().min(20)]);
            let mut segs: Vec<_> = {
                let mut m = std::collections::HashMap::new();
                for (&(cs, _), &n) in h.iter() {
                    *m.entry(cs).or_insert(0u64) += n;
                }
                let mut v: Vec<_> = m.into_iter().collect();
                v.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
                v
            };
            segs.truncate(8);
            println!("hot segments: {segs:04x?}");
        }
        return;
    }
    if std::env::var("SELECTORWATCH").is_ok() {
        // POSE-SELECTOR ground truth: resume the hub, perform scripted interactions
        // (idle, move, menu hover, orb hover, click, steer to the edge), sampling the
        // manu3 call arg frame at ds:0xAB4 ({cursor dword, selector word}) — logs which
        // selector the REAL game passes in each interaction context.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let g = 0x0e84u16;
        let rd16 = |rt: &Runtime, off: u32| {
            rt.m.read8(g, off) as u16 | ((rt.m.read8(g, off + 1) as u16) << 8)
        };
        let frame = |rt: &Runtime| rd16(rt, 0x2795);
        let fr = frame(&rt);
        let ring = |sx: i32| ((sx + fr as i32 * 8 - 160).rem_euclid(1440)) as u16;
        let scenarios: [(&str, u16, u16, bool); 6] = [
            ("idle centre", ring(160), 100, false),
            ("orb hover", ring(125), 118, false),
            ("orb click", ring(125), 118, true),
            ("menu hover (HONK row)", ring(230), 88, false),
            ("menu click (HONK row)", ring(230), 88, true),
            ("edge steer (right)", ring(316), 100, false),
        ];
        for (name, mx, my, click) in scenarios {
            rt.set_mouse_pos(mx, my);
            let mut seen: Vec<(u16, u32)> = Vec::new();
            for _ in 0..40 {
                let _ = rt.run(rt.cpu.steps + 100_000);
                let sel = rd16(&rt, 0xAB8);
                let cur = rd16(&rt, 0xAB4);
                let _ = cur;
                if let Some(e) = seen.iter_mut().find(|e| e.0 == sel) {
                    e.1 += 1;
                } else {
                    seen.push((sel, 1));
                }
            }
            if click {
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(0);
                for _ in 0..40 {
                    let _ = rt.run(rt.cpu.steps + 100_000);
                    let sel = rd16(&rt, 0xAB8);
                    if let Some(e) = seen.iter_mut().find(|e| e.0 == sel) {
                        e.1 += 1;
                    } else {
                        seen.push((sel, 1));
                    }
                }
            }
            println!("{name}: selectors {seen:?} (frame {})", frame(&rt));
        }
        println!("SELECTORWATCH done");
        return;
    }
    if let Ok(spec) = std::env::var("BOOTIDX") {
        // Cold-boot to a step target and dump RAW VGA INDICES + DAC (+ ppm) — the
        // ground truth for the boot-dialogue screen (character porthole + pyramid
        // deck + eye-orb + subtitle). Spec: comma-separated M-step targets.
        let mut targets: Vec<u64> = spec
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        targets.sort_unstable();
        for m in targets {
            let _ = rt.run(m * 1_000_000);
            std::fs::write(out.join(format!("bd_{m:05}M.idx")), rt.screen_indices()).unwrap();
            std::fs::write(out.join(format!("bd_{m:05}M.dac")), rt.dac).unwrap();
            rt.write_ppm(&out.join(format!("bd_{m:05}M.ppm"))).unwrap();
            println!("BOOTIDX {m}M done");
        }
        return;
    }
    if std::env::var("INDEXDUMP").is_ok() {
        // Dump the hub screen as RAW VGA INDICES + the DAC — lets the port compare
        // content (indices) and palette state (DAC) separately.
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let _ = rt.run(rt.cpu.steps + 2_000_000);
        std::fs::write(out.join("hub_indices.bin"), rt.screen_indices()).unwrap();
        std::fs::write(out.join("hub_dac.bin"), rt.dac).unwrap();
        let g = 0x0e84u16;
        let fr = rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        println!("INDEXDUMP done (ring frame {fr})");
        return;
    }
    if std::env::var("HANDGRID").is_ok() {
        // Dense hand-pose capture: resume the hub state, keep the view parked (all grid
        // points sit inside the steering dead zone), and dump a frame with the REAL 3D hand
        // at each grid position. Offline: modal-background diff -> sprite atlas.
        let g = 0x0e84u16;
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        rt.load_state(std::path::Path::new("accuracy/script2.state")).unwrap();
        let fr = frame(&rt);
        for sy in (20..=190).step_by(34) {
            for sx in (40..=280).step_by(40) {
                let ring = (sx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy as u16);
                let _ = rt.run(rt.cpu.steps + 2_000_000);
                rt.write_ppm(&out.join(format!("hg_{sx}_{sy}.ppm"))).unwrap();
            }
        }
        println!("HANDGRID done (frame {fr})");
        return;
    }
    if let Ok(spec) = std::env::var("RESUMEPROBE") {
        // Resume the clean interactive SCRIPT2 state and click screen positions with
        // RING-space mouse x (ring = screen_x + frame*8 - 160, the console's mouse model —
        // the reason plain CLICKAT never dispatched). Spec: "sx,sy;sx,sy;..." with a long
        // dwell + capture after each.
        let g = 0x0e84u16;
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        let statepath = std::path::Path::new("accuracy/script2.state");
        rt.load_state(statepath).unwrap();
        let before = rt.opened_files.len();
        for (i, pt) in spec.split(';').enumerate() {
            // "e0,0" = press Escape; "b<sx>,<sy>" = RIGHT-click at screen coords.
            if pt.starts_with('e') {
                rt.inject_key(0x01, 0x1b);
                let _ = rt.run(rt.cpu.steps + 3_000_000);
                rt.write_ppm(&out.join(format!("rp_{i:02}_esc.ppm"))).unwrap();
                println!("rp {i} ESC (0x2793={:#04x}, frame {})", rt.m.read8(g, 0x2793), frame(&rt));
                continue;
            }
            if let Some(rest) = pt.strip_prefix('b') {
                let (a, b) = rest.split_once(',').unwrap();
                let (sx, sy): (u16, u16) = (a.trim().parse().unwrap(), b.trim().parse().unwrap());
                let fr = frame(&rt);
                let ring = (sx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, sy);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_press(1);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(1);
                let _ = rt.run(rt.cpu.steps + 4_000_000);
                rt.write_ppm(&out.join(format!("rp_{i:02}_rclick.ppm"))).unwrap();
                println!("rp {i} RCLICK ({sx},{sy}) (0x2793={:#04x}, frame {})", rt.m.read8(g, 0x2793), frame(&rt));
                continue;
            }
            // "c0,0" = clear the menu-engaged flag [0x2793]&4 (diagnostic disengage).
            if let Some(rest) = pt.strip_prefix('c') {
                let _ = rest;
                let v = rt.m.read8(g, 0x2793);
                rt.m.write8(g, 0x2793, v & !4);
                let _ = rt.run(rt.cpu.steps + 2_000_000);
                rt.write_ppm(&out.join(format!("rp_{i:02}_clear.ppm"))).unwrap();
                println!("rp {i} cleared menu-engaged (0x2793={:#04x})", rt.m.read8(g, 0x2793));
                continue;
            }
            // "m<sx>,<sy>" = move/hover only; "r<ringx>,<sy>" = park at ABSOLUTE ring x
            // (steers the view there); "<sx>,<sy>" = click at screen coords.
            let (mode, pt) = match (pt.strip_prefix('m'), pt.strip_prefix('r')) {
                (Some(rest), _) => (1u8, rest),
                (_, Some(rest)) => (2u8, rest),
                _ => (0, pt),
            };
            let mv = mode != 0;
            let (a, b) = pt.split_once(',').unwrap();
            let (sx, sy): (u16, u16) = (a.trim().parse().unwrap(), b.trim().parse().unwrap());
            let fr = frame(&rt);
            let ring = if mode == 2 {
                sx % 1440
            } else {
                (sx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16
            };
            rt.set_mouse_pos(ring, sy);
            let _ = rt.run(rt.cpu.steps + 500_000);
            if !mv {
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(0);
            }
            let _ = rt.run(rt.cpu.steps + 12_000_000);
            rt.write_ppm(&out.join(format!("rp_{i:02}_{sx}_{sy}.ppm"))).unwrap();
            println!("rp {i} ({sx},{sy}) ring {ring} frame {fr}: files:");
            for (_, p) in rt.opened_files.iter().skip(before) {
                println!("    {p}");
            }
        }
        println!("RESUMEPROBE done");
        return;
    }
    if std::env::var("MENUTREE").is_ok() {
        let g = 0x0e84u16;
        let cur_menu = |rt: &Runtime| rt.m.read8(g, 0x6772) as u16 | ((rt.m.read8(g, 0x6773) as u16) << 8);
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        let statepath = std::path::Path::new("accuracy/milestone_script2.state");
        // The current menu's topic rows (measured from the fear/anger menu capture):
        // x=175.., first row top y=61, 11px pitch.
        let topics = ["talk", "fear", "weakness", "complain", "anger", "break", "cry"];
        rt.load_state(statepath).unwrap();
        let base = cur_menu(&rt);
        println!("MENUTREE base menu = {base:#06x} ({} topics)", topics.len());
        for (i, name) in topics.iter().enumerate() {
            rt.load_state(statepath).unwrap();
            let fr = frame(&rt);
            let my = 61 + 11 * i as u16 + 3;
            let ring = (200i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
            rt.set_mouse_pos(ring, my);
            let _ = rt.run(rt.cpu.steps + 500_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 300_000);
            rt.mouse_release(0);
            let _ = rt.run(rt.cpu.steps + 4_000_000);
            let after = cur_menu(&rt);
            let tag = if after != base { "NAVIGATED" } else { "(stayed)" };
            println!("  topic {i} '{name}' (y{my}): {base:#06x} -> {after:#06x} {tag}");
            rt.write_ppm(&out.join(format!("menutree_{i}_{name}.ppm"))).unwrap();
        }
        // Then pop to the parent (talk) and map THAT menu (the top-level, which
        // has the real topic→sub-menu navigation), writing its screen for geometry.
        rt.load_state(statepath).unwrap();
        let fr = frame(&rt);
        let ring = (200i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
        rt.set_mouse_pos(ring, 64);
        let _ = rt.run(rt.cpu.steps + 500_000);
        rt.mouse_press(0);
        let _ = rt.run(rt.cpu.steps + 300_000);
        rt.mouse_release(0);
        let _ = rt.run(rt.cpu.steps + 4_000_000);
        println!("MENUTREE after talk: menu = {:#06x}", cur_menu(&rt));
        rt.write_ppm(&out.join("menutree_parent.ppm")).unwrap();
        rt.save_state(std::path::Path::new("accuracy/menu_toplevel.state")).unwrap();
        return;
    }

    // BASWATCH: from a resumed SCRIPT2 state, read-watch the loaded SCRIPT2.BAS
    // region and report which BAS offsets the console menu handler READS while
    // a concept menu is displayed/redrawn — decodes the COD→BAS menu-selection
    // linkage (which BAS menu the conversation state picks). Locates BAS by
    // searching for the psychotherapy menu bytes (a3 01 00 7a 26 1a 44 24).
    if std::env::var("BASWATCH").is_ok() {
        let sig: [u8; 8] = [0xa3, 0x01, 0x00, 0x7a, 0x26, 0x1a, 0x44, 0x24];
        let menu_lin = (0..rt.m.mem.len().saturating_sub(8)).find(|&i| rt.m.mem[i..i + 8] == sig);
        let Some(menu_lin) = menu_lin else {
            println!("BASWATCH: SCRIPT2.BAS not resident (resume milestone_script2.state)");
            return;
        };
        let bas_base = menu_lin - 0xc27; // menu is at BAS file offset 0xc27
        let bas_size = 0x5825usize; // SCRIPT2.BAS is 22565 bytes
        println!("BASWATCH: SCRIPT2.BAS @ linear {bas_base:#08x}..{:#08x} (psy menu @ {menu_lin:#08x})", bas_base + bas_size);
        rt.write_ppm(&out.join("baswatch_00.ppm")).unwrap();
        // Watch reads across the BAS while driving the console (clicks that would
        // (re)open a topic list): the menu handler reads the selected menu table.
        rt.m.read_watch = Some(bas_base..bas_base + bas_size);
        rt.m.read_hits.borrow_mut().clear();
        let g = 0x0e84u16;
        let frame = |rt: &Runtime| rt.m.read8(g, 0x2795) as u16 | ((rt.m.read8(g, 0x2796) as u16) << 8);
        for pass in 0..6u32 {
            let fr = frame(&rt);
            // orb (open concept menu) then a topic-row click, over the console.
            for (mx, my) in [(125u16, 118u16), (190, 45 + 11 * (pass % 12) as u16)] {
                let ring = (mx as i32 + fr as i32 * 8 - 160).rem_euclid(1440) as u16;
                rt.set_mouse_pos(ring, my);
                let _ = rt.run(rt.cpu.steps + 500_000);
                rt.mouse_press(0);
                let _ = rt.run(rt.cpu.steps + 300_000);
                rt.mouse_release(0);
                let _ = rt.run(rt.cpu.steps + 2_000_000);
            }
        }
        rt.m.read_watch = None;
        let hits = rt.m.read_hits.borrow();
        // Report distinct BAS offsets read, and whether any is a 0xA3 menu head.
        let mut offs: Vec<usize> = hits.iter().map(|h| h.0 - bas_base).collect();
        offs.sort_unstable();
        offs.dedup();
        println!("BASWATCH: {} BAS reads, {} distinct offsets", hits.len(), offs.len());
        let menu_heads: Vec<String> = offs
            .iter()
            .filter(|&&o| o < bas_size && rt.m.mem[bas_base + o] == 0xa3)
            .map(|&o| format!("{o:#06x}"))
            .collect();
        println!("BASWATCH: distinct offsets (first 40): {:x?}", &offs[..offs.len().min(40)]);
        println!("BASWATCH: 0xA3 menu-head offsets READ: {menu_heads:?}");
        // The READER code (cs:ip) of each menu-head read = the menu-selection/draw
        // routine; the offset it reads is the per-state selected menu.
        let mut reader_cs = None;
        for &(addr, cs, ip) in hits.iter() {
            let o = addr - bas_base;
            if o < bas_size && rt.m.mem[bas_base + o] == 0xa3 {
                println!("  menu-head @BAS+{o:#06x} read by {cs:04x}:{ip:04x}");
                reader_cs = Some(cs);
            }
        }
        // Dump the menu-selection/draw code segment so its offset-selection can be
        // disassembled offline (dis_xdb.py) — the last step of the linkage decode.
        if let Some(cs) = reader_cs {
            let base = (cs as usize) * 16;
            let dump = rt.m.mem[base..(base + 0x2000).min(rt.m.mem.len())].to_vec();
            std::fs::write(out.join(format!("menu_code_{cs:04x}.bin")), &dump).unwrap();
            println!("BASWATCH: dumped menu-selection code segment {cs:04x} -> menu_code_{cs:04x}.bin");
        }
        rt.write_ppm(&out.join("baswatch_end.ppm")).unwrap();
        return;
    }

    if let Ok(iv) = std::env::var("STATEDUMP") {
        // Print the credit-scene state machine every <iv> million steps: the two text buffers
        // and the gate/phase flags that select static "WAIT COMMANDER" vs clean "CRYO..." credit.
        let iv_m: u64 = iv.parse().unwrap_or(20);
        let g = 0x0e84u16;
        let readstr = |rt: &Runtime, off: u32| -> String {
            let mut s = String::new();
            for i in 0..40 {
                let b = rt.m.read8(g, off + i);
                if b == 0 { break; }
                s.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
            }
            s
        };
        let mut mark = iv_m * 1_000_000;
        loop {
            match rt.run(mark) {
                RunEnd::StepBudget => {}
                other => { println!("ended: {other:?} at {}", rt.cpu.steps); break; }
            }
            let m = rt.cpu.steps / 1_000_000;
            println!(
                "@{m:>3}M 5e64={:#06x} 5e65={:#04x} 5e58={:#06x} 6780={:#06x} ba0={:#06x} 27e2={:#06x} | buf(e18)={:?} src(190)={:?}",
                rt.m.read16(g, 0x5e64), rt.m.read8(g, 0x5e65), rt.m.read16(g, 0x5e58),
                rt.m.read16(g, 0x6780), rt.m.read16(g, 0x0ba0), rt.m.read16(g, 0x27e2),
                readstr(&rt, 0x0e18), readstr(&rt, 0x0190),
            );
            mark += iv_m * 1_000_000;
            if rt.cpu.steps >= steps { break; }
        }
        return;
    }

    if let Ok(w) = std::env::var("EXECWATCHLIN") {
        // Watch execution by LINEAR address (cs*16+ip), resolving the segment-relocation
        // ambiguity a (cs,ip) watch has. Spec = comma-separated FILE offsets (hex); each
        // maps to linear = 0x1a20 + (file - 0x600) (image loaded at para 0x1a2, header 0x600).
        for tok in w.split(',') {
            let file = u32::from_str_radix(tok.trim().trim_start_matches("0x"), 16).unwrap();
            let lin = 0x1a20 + file.saturating_sub(0x600);
            rt.cpu.exec_watch_linear.push(lin);
            eprintln!("watch file {file:#07x} -> linear {lin:#07x}");
        }
        let _ = rt.run(steps);
        println!("linear exec-watch results ({} of {} hit) at {} steps:",
            rt.cpu.exec_hits_linear.len(), rt.cpu.exec_watch_linear.len(), rt.cpu.steps);
        for &(lin, first, count) in &rt.cpu.exec_hits_linear {
            let file = lin - 0x1a20 + 0x600;
            println!("  linear {lin:#07x} (file {file:#07x}): first@{first} count={count}");
        }
        for &lin in &rt.cpu.exec_watch_linear {
            if !rt.cpu.exec_hits_linear.iter().any(|h| h.0 == lin) {
                let file = lin - 0x1a20 + 0x600;
                println!("  linear {lin:#07x} (file {file:#07x}): NEVER EXECUTED");
            }
        }
        return;
    }

    if let Ok(w) = std::env::var("EXECWATCH") {
        // Watch execution of specific cs:ip entry points across the whole run, e.g.
        // EXECWATCH=08c0:0432,????:9432. Reports first-hit step and hit count for each.
        // Converts file offsets too: a bare hex >0x1000 is treated as a FILE offset and
        // mapped to loaded cs:ip via seg = (file>>4)+0x1a2, ip = file&0xf ... but callers
        // pass explicit cs:ip so we keep it simple.
        for tok in w.split(',') {
            let tok = tok.trim();
            if let Some((c, i)) = tok.split_once(':') {
                let cs = u16::from_str_radix(c.trim_start_matches("0x"), 16).unwrap();
                let ip = u16::from_str_radix(i.trim_start_matches("0x"), 16).unwrap();
                rt.cpu.exec_watch.push((cs, ip));
                eprintln!("watch {cs:04x}:{ip:04x} (file ~{:#07x})", (cs as u32 - 0x1a2) * 16 + ip as u32);
            }
        }
        let _ = rt.run(steps);
        println!("exec watch results ({} of {} entries hit):", rt.cpu.exec_hits.len(), rt.cpu.exec_watch.len());
        for (cs, ip, first, count) in &rt.cpu.exec_hits {
            println!("  {cs:04x}:{ip:04x} (file ~{:#07x}): first@{first} count={count}", (*cs as u32 - 0x1a2) * 16 + *ip as u32);
        }
        for (cs, ip) in &rt.cpu.exec_watch {
            if !rt.cpu.exec_hits.iter().any(|h| h.0 == *cs && h.1 == *ip) {
                println!("  {cs:04x}:{ip:04x} (file ~{:#07x}): NEVER EXECUTED", (*cs as u32 - 0x1a2) * 16 + *ip as u32);
            }
        }
        eprintln!("execwatch done at {} steps", rt.cpu.steps);
        return;
    }

    if let Ok(w) = std::env::var("REVWATCH") {
        // Watch writes to a chosen gs offset (default 0x5e65 = reveal phase) across the credit
        // scene, recording (value, cs, ip) so we can see WHO sets it and to WHAT. gs seg = 0x0e84.
        let off: u32 = u32::from_str_radix(w.trim_start_matches("0x"), 16).unwrap_or(0x5e65);
        // fast-forward to just before the credit scene, then arm the watch (REVFROM=<Msteps>)
        let from_m: u64 = std::env::var("REVFROM").ok().and_then(|s| s.parse().ok()).unwrap_or(213);
        let _ = rt.run(from_m * 1_000_000);
        rt.m.watch_addr = Some(0x0e84 * 16 + off as usize);
        let _ = rt.run(steps);
        println!("writes to gs:{off:#06x} (value, cs:ip), {} hits:", rt.m.addr_hits.len());
        let mut seen = std::collections::HashSet::new();
        for (v, cs, ip) in &rt.m.addr_hits {
            if seen.insert((*v, *cs, *ip)) {
                let fseg = (*cs as i32 - 0x1a2) as u32;
                println!("  ={v:#04x} at {cs:04x}:{ip:04x}  (file ~{:#07x})", fseg * 16 + *ip as u32);
            }
        }
        eprintln!("revwatch done at {} steps", rt.cpu.steps);
        return;
    }

    let mut rec_watch: Option<u32> = None;
    if let Ok(w) = std::env::var("BOOTWRITEWATCH") {
        if let Some(recspec) = w.strip_prefix("rec:") {
            // POINTER-RELATIVE record watch: re-resolve the object block ([0x6724]
            // far ptr) each shot interval and re-arm block+offset — survives the
            // per-profile block relocation.
            rec_watch =
                Some(u32::from_str_radix(recspec.trim_start_matches("0x"), 16).unwrap());
            eprintln!("boot record-watch armed at block+{:#x}", rec_watch.unwrap());
        } else {
            let lin = usize::from_str_radix(w.trim_start_matches("0x"), 16).unwrap();
            rt.m.watch_addr = Some(lin);
            eprintln!("boot write-watch armed at linear {lin:#x}");
        }
    }
    let mut next_shot = shot_every;
    let end = loop {
        match rt.run(next_shot.min(steps)) {
            RunEnd::StepBudget => {
                if let Some(off) = rec_watch {
                    let g = 0x0e84u16;
                    let w16 = |rt: &Runtime, a: u32| {
                        rt.m.read8(g, a) as u32 | ((rt.m.read8(g, a + 1) as u32) << 8)
                    };
                    let (blk_off, blk_seg) = (w16(&rt, 0x6724), w16(&rt, 0x6726));
                    if blk_seg > 0 {
                        let lin = blk_seg as usize * 16 + blk_off as usize + off as usize;
                        if rt.m.watch_addr != Some(lin) {
                            eprintln!(
                                "record-watch re-armed: block {blk_seg:04x}:{blk_off:04x} -> lin {lin:#x} @ step {}",
                                rt.cpu.steps
                            );
                            rt.m.watch_addr = Some(lin);
                        }
                    }
                }
                let mstep = rt.cpu.steps / 1_000_000;
                rt.write_ppm(&out.join(format!("boot_{mstep:05}M.ppm"))).unwrap();
                if rt.cpu.steps >= steps {
                    break RunEnd::StepBudget;
                }
                next_shot += shot_every;
            }
            other => break other,
        }
    };

    if !rt.m.addr_hits.is_empty() {
        println!("boot write-watch hits:");
        let mut seen = std::collections::HashSet::new();
        for &(v, cs, ip) in &rt.m.addr_hits {
            if seen.insert((v, cs, ip)) {
                println!("  ={v:#04x} at {cs:04x}:{ip:04x}");
            }
        }
    }
    let mstep = rt.cpu.steps / 1_000_000;
    rt.write_ppm(&out.join(format!("final_{mstep:05}M.ppm"))).unwrap();
    println!("=== end: {end:?} after {} steps (mode {:#04x}) ===", rt.cpu.steps, rt.vga_mode);
    let [ins, outs, ints, hlts, chunks] = rt.exit_counts;
    println!("exits: in={ins} out={outs} int={ints} hlt={hlts} chunks={chunks}");
    println!("{}", rt.debug_state());
    {
        let ss = rt.m.regs.ss;
        let sp = rt.m.regs.sp() as u32;
        let w: Vec<String> = (0..6).map(|i| format!("{:04x}", rt.m.read16(ss, sp + i * 2))).collect();
        println!("stack top: {}", w.join(" "));
    }
    std::fs::write(out.join("driver.bin"), &rt.m.mem[0x765e0..0x765e0 + 0x1000]).unwrap();
    // dump the code around the final cs:ip for offline disassembly
    let base = (rt.cpu.cs as usize) * 16 + rt.cpu.ip as usize;
    let lo = base.saturating_sub(64);
    std::fs::write(out.join("spin_code.bin"), &rt.m.mem[lo..base + 192]).unwrap();
    eprintln!("spin code dumped ({:#x}..{:#x}, ip at +64)", lo, base + 192);
    let txt = rt.text_screen();
    if !txt.is_empty() {
        println!("--- text screen ---\n{txt}");
    }
    if !rt.console_output().is_empty() {
        println!("--- console ---\n{}", rt.console_output());
    }
    let mut log: Vec<_> = rt.int_log.iter().collect();
    log.sort();
    println!("--- int usage (vector, AH) -> count ---");
    for ((v, ah), n) in log {
        println!("int {v:02x}/{ah:02x}: {n}");
    }
}
