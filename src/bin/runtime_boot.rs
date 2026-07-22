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
        let before = rt.opened_files.len();
        for (i, pt) in spec.split(';').enumerate() {
            let (a, b) = pt.split_once(',').unwrap();
            let (sx, sy): (u16, u16) = (a.trim().parse().unwrap(), b.trim().parse().unwrap());
            rt.set_mouse_pos(sx * 2, sy);
            let _ = rt.run(rt.cpu.steps + 800_000);
            rt.mouse_press(0);
            let _ = rt.run(rt.cpu.steps + 800_000);
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
                let (sx, sy) = match want {
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

        // SUBMENUCAP: click the golden menu's MENU and OPTION rows (decoded box:
        // screen x 177..287, rows top 0x48 pitch 0x12 at frame 45-centred view;
        // at frame 55 the box shifts -8px/frame => right edge 207) and capture
        // what actually opens — ground truth for the submenu/OPTION ports.
        if std::env::var("SUBMENUCAP").is_ok() {
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
                let _ = rt.run(rt.cpu.steps + 10_000_000);
                rt.write_ppm(&out.join(format!("submenu_{name}.ppm"))).unwrap();
                let (fr2, _, st2) = state(&rt);
                println!("after {name} click: frame {fr2} station {st2:#x}");
                rt.inject_key(0x01, 0x1b); // Esc back
                let _ = rt.run(rt.cpu.steps + 6_000_000);
            }
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
            let grid: Vec<(u16, u16)> = [
                (40u16, 100u16), // the BRIDGEPROBE rest state (test ground truth)
                (60, 60), (160, 60), (260, 60),
                (60, 120), (160, 120), (260, 120),
                (60, 170), (160, 170), (260, 170),
            ]
            .to_vec();
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
        // Run to `steps`, then scan all of guest RAM for an ASCII needle — decisive test of
        // whether a given string (e.g. a DESCRIPT credit cue) is resident at that moment.
        let _ = rt.run(steps);
        let pat = needle.as_bytes();
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

    let mut next_shot = shot_every;
    let end = loop {
        match rt.run(next_shot.min(steps)) {
            RunEnd::StepBudget => {
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
