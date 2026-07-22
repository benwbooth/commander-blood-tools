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
                            if marker == "script2" {
                                reached2 = true;
                                rt.save_state(std::path::Path::new("accuracy/script2.state")).unwrap();
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
