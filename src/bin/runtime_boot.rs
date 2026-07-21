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

    if let Ok(w) = std::env::var("REVWATCH") {
        // Watch writes to a chosen gs offset (default 0x5e65 = reveal phase) across the credit
        // scene, recording (value, cs, ip) so we can see WHO sets it and to WHAT. gs seg = 0x0e84.
        let off: u32 = u32::from_str_radix(w.trim_start_matches("0x"), 16).unwrap_or(0x5e65);
        // fast-forward to just before the credit scene, then arm the watch
        let _ = rt.run(213_000_000);
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
