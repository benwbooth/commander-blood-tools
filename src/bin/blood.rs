//! Commander Blood — playable, faithful: the ORIGINAL BLOODPRG.EXE running in the path-B
//! runtime, presented in an X11 window with real keyboard + mouse input.
//!
//! Usage:
//!   blood                         # windowed, real-time pacing (~18.2 ticks/s)
//!   blood --turbo                 # windowed, unpaced
//!   blood --script FILE --out DIR # headless: drive with a scripted input timeline
//!
//! Script lines (headless mode): `wait <ticks>`, `key <scan> [ascii]`, `move <x> <y>`,
//! `press <x> <y>`, `release <x> <y>`, `click <x> <y>` (press+release), `shot <name>`.
//! Coordinates are native 320x200; the runtime converts to DOS-virtual mouse coords.

use commander_blood_tools::recomp::runtime::{RunEnd, Runtime, STEPS_PER_SECOND};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_micros(54_925); // 18.2065 Hz PIT

fn make_runtime() -> Runtime {
    let c_root = PathBuf::from("accuracy/cdrive");
    let d_root = PathBuf::from("output/_tmp_iso");
    std::fs::create_dir_all(c_root.join("cblood")).unwrap();
    let exe = std::fs::read(d_root.join("BLOODPRG.EXE")).expect("D:\\BLOODPRG.EXE");
    let mut rt = Runtime::new(c_root, d_root);
    rt.load_exe(&exe, " AMR S162227 EMS WRIC:\\cblood\\", "D:\\BLOODPRG.EXE")
        .unwrap();
    rt
}

/// One BIOS-tick (1/18.2 s) of emulated time, regardless of the guest's PIT rate.
/// Returns false when the guest exited/faulted.
fn run_tick(rt: &mut Runtime) -> Result<bool, String> {
    let target = rt.cpu.steps + STEPS_PER_SECOND * 55 / 1000;
    match rt.run(target) {
        RunEnd::StepBudget => Ok(true),
        RunEnd::Exited(c) => {
            eprintln!("game exited with code {c}");
            Ok(false)
        }
        RunEnd::Fatal(e) => Err(e),
    }
}

fn scancode_ascii(scan: u8) -> u8 {
    // US layout, unshifted — enough for menu/gameplay keys.
    const MAP: &[(u8, u8)] = &[
        (0x01, 27),
        (0x1c, b'\r'),
        (0x39, b' '),
        (0x0e, 8),
        (0x0f, b'\t'),
        (0x02, b'1'),
        (0x03, b'2'),
        (0x04, b'3'),
        (0x05, b'4'),
        (0x06, b'5'),
        (0x07, b'6'),
        (0x08, b'7'),
        (0x09, b'8'),
        (0x0a, b'9'),
        (0x0b, b'0'),
        (0x10, b'q'),
        (0x11, b'w'),
        (0x12, b'e'),
        (0x13, b'r'),
        (0x14, b't'),
        (0x15, b'y'),
        (0x16, b'u'),
        (0x17, b'i'),
        (0x18, b'o'),
        (0x19, b'p'),
        (0x1e, b'a'),
        (0x1f, b's'),
        (0x20, b'd'),
        (0x21, b'f'),
        (0x22, b'g'),
        (0x23, b'h'),
        (0x24, b'j'),
        (0x25, b'k'),
        (0x26, b'l'),
        (0x2c, b'z'),
        (0x2d, b'x'),
        (0x2e, b'c'),
        (0x2f, b'v'),
        (0x30, b'b'),
        (0x31, b'n'),
        (0x32, b'm'),
    ];
    MAP.iter().find(|(s, _)| *s == scan).map(|(_, a)| *a).unwrap_or(0)
}

fn run_script(script_path: &str, out_dir: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut rt = make_runtime();
    let script = std::fs::read_to_string(script_path).map_err(|e| e.to_string())?;
    let (mut mx, mut my, mut mb) = (160u16, 100u16, 0u16);
    for (ln, line) in script.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let w: Vec<&str> = line.split_whitespace().collect();
        let arg = |i: usize| -> u16 { w.get(i).and_then(|s| s.parse().ok()).unwrap_or(0) };
        match w[0] {
            "wait" => {
                for _ in 0..arg(1) {
                    if !run_tick(&mut rt).map_err(|e| format!("line {}: {e}", ln + 1))? {
                        return Ok(());
                    }
                }
            }
            "key" => {
                let scan = arg(1) as u8;
                let ascii = if w.len() > 2 { arg(2) as u8 } else { scancode_ascii(scan) };
                rt.key_event(scan, ascii);
            }
            "move" | "press" | "release" | "click" => {
                mx = arg(1).min(319);
                my = arg(2).min(199);
                match w[0] {
                    "press" => mb = 1,
                    "release" => mb = 0,
                    _ => {}
                }
                rt.mouse_event(mx * 2, my, mb);
                if w[0] == "click" {
                    for _ in 0..2 {
                        if !run_tick(&mut rt)? {
                            return Ok(());
                        }
                    }
                    rt.mouse_event(mx * 2, my, 1);
                    for _ in 0..3 {
                        if !run_tick(&mut rt)? {
                            return Ok(());
                        }
                    }
                    rt.mouse_event(mx * 2, my, 0);
                }
            }
            "shot" => {
                let name = w.get(1).unwrap_or(&"shot");
                rt.write_ppm(&out_dir.join(format!("{name}.ppm"))).map_err(|e| e.to_string())?;
                eprintln!("shot {name} at tick {}", rt.ticks());
            }
            "font" => {
                let ds = rt.m.regs.ds;
                let ss = rt.m.regs.ss;
                let dump = |seg: u16, off: u32, n: u32, rt: &Runtime| -> String {
                    (0..n).map(|i| format!("{:02x}", rt.m.read8(seg, off + i))).collect::<Vec<_>>().join(" ")
                };
                eprintln!("ds={ds:04x} ss={ss:04x}");
                eprintln!("DS:6fa8 (ascii->glyph map, 'A'=0x41): {}", dump(ds, 0x6fa8 + 0x41, 8, &rt));
                eprintln!("DS:7028 (glyphs)   : {}", dump(ds, 0x7028, 20, &rt));
                eprintln!("SS:7028 (glyphs)   : {}", dump(ss, 0x7028, 20, &rt));
                let gs = rt.m.regs.gs;
                let fb_off = rt.m.read16(gs, 0x5219);
                let fb_seg = rt.m.read16(gs, 0x521b);
                eprintln!("gs={gs:04x} framebuffer ptr gs:5219 = {fb_seg:04x}:{fb_off:04x}");
                eprintln!("DS:70fa (subtitle map @'A'): {}", dump(ds, 0x70fa + 0x41, 8, &rt));
                eprintln!("DS:71aa (subtitle glyphs)  : {}", dump(ds, 0x71aa, 24, &rt));
                // during the subtitle blit DS==gs (chars read from the gs state buffer via DS:si)
                eprintln!("gs:70fa (map @'A'): {}", dump(gs, 0x70fa + 0x41, 8, &rt));
                eprintln!("gs:71aa (glyphs)  : {}", dump(gs, 0x71aa, 24, &rt));
                let gidx = rt.m.read8(gs, 0x70fa + 0x57) as u32;
                eprintln!("(gs) 'W'->glyph {gidx:#x}, bitmap: {}", dump(gs, 0x71aa + gidx*8, 8, &rt));
            }
            "watchaddr" => {
                let off = usize::from_str_radix(w.get(1).unwrap_or(&"67bc"), 16).unwrap_or(0x67bc);
                let lin = 0x0e84usize * 16 + off;
                rt.m.watch_addr = Some(lin);
                rt.m.addr_hits.clear();
                eprintln!("watching writes to 0e84:{off:04x} (lin {lin:#x})");
            }
            "watchlin" => {
                // watchlin <hexlinear>: all writes to a raw linear address
                let lin = usize::from_str_radix(w.get(1).unwrap_or(&"2e4c0"), 16).unwrap_or(0x2e4c0);
                rt.m.watch_addr = Some(lin);
                rt.m.addr_hits.clear();
                eprintln!("watching all writes to lin {lin:#x}");
            }
            "watchaddrdump" => {
                eprintln!("writes ({}):", rt.m.addr_hits.len());
                for (v, cs, ip) in &rt.m.addr_hits {
                    let rel = cs.wrapping_sub(0x1a2);
                    let file = 0x600 + (rel as usize) * 16 + *ip as usize;
                    eprintln!("  =0x{v:02x} @ {cs:04x}:{ip:04x} (seg 0x{rel:x}, file {file:#07x})");
                }
                rt.m.watch_addr = None;
            }
            "trace" => {
                let gs = rt.m.regs.gs;
                eprintln!("  crtc_start={:#06x} 5219={:04x}:{:04x} 521d={:04x}:{:04x}",
                    ((rt.crtc_reg(0x0c) as u16) << 8) | rt.crtc_reg(0x0d) as u16,
                    rt.m.read16(gs,0x521b), rt.m.read16(gs,0x5219),
                    rt.m.read16(gs,0x521f), rt.m.read16(gs,0x521d));
                eprintln!("t{:>4} 5e58={:04x} 5e65={:04x} b31={:04x} b37={:04x} 67bc={:02x} 67bb={:02x} 5e64={:02x} 27e2={:04x} 679a={:04x}",
                    rt.ticks(),
                    rt.m.read16(gs,0x5e58), rt.m.read16(gs,0x5e65), rt.m.read16(gs,0xb31), rt.m.read16(gs,0xb37),
                    rt.m.read8(gs,0x67bc), rt.m.read8(gs,0x67bb), rt.m.read8(gs,0x5e64), rt.m.read16(gs,0x27e2), rt.m.read16(gs,0x679a));
            }
            "trapset" => {
                // arm exec counters for the glyph blitter entry + reveal draw glyph-call site
                rt.m.trap_ips.clear();
                rt.m.trap_ips.insert((0x043b, 0x06a0), 0); // glyph blitter 0x299:0x6a0 (file 0x3630)
                rt.m.trap_ips.insert((0x08c0, 0x1d0e), 0); // lcall glyph blitter (0x94ee), seg 0x71e->0x8c0
                rt.m.trap_ips.insert((0x08c0, 0x1c15), 0); // reveal draw entry (0x93f5)
                rt.m.trap_ips.insert((0x08c0, 0x1c4a), 0); // reveal draw 0x942a-ish region (0x93f8+)
                rt.m.trap_ips.insert((0x043b, 0x0f91), 0); // chunky->planar scene blit (per frame)
                rt.m.trap_ips.insert((0x0cbd, 0x0679), 0); // 0xbe00 audio subtitle present (seg 0xb1b->0xcbd)
                rt.m.trap_ips.insert((0x043b, 0x0722), 0); // glyph pixel write 1 (file 0x36b2 = ip 0x722)
                rt.m.trap_ips.insert((0x043b, 0x072a), 0); // glyph pixel write 2 (file 0x36ba = ip 0x72a)
                rt.m.trap_ips.insert((0x043b, 0x06e2), 0); // char loop head (0x3672)
                rt.m.trap_ips.insert((0x043b, 0x0718), 0); // column loop (0x36a8)
                rt.m.trap_ips.insert((0x043b, 0x071b), 0); // row loop body (0x36ab)
                rt.m.trap_ips.insert((0x043b, 0x074d), 0); // char loop dec (0x36dd)
                rt.m.trap_ips.insert((0x043b, 0x0721), 0); // js skip (0x36e1)
                rt.m.trap_ips.insert((0x0cbd, 0x0610), 0); // subtitle-present routine ENTRY (0xbdc0)
                rt.m.trap_ips.insert((0x0cbd, 0x0617), 0); // after test 0xade (0xbdc7)
                rt.m.trap_ips.insert((0x0cbd, 0x0621), 0); // after test 0xba3 (0xbdd1)
                rt.m.trap_ips.insert((0x0cbd, 0x05e0), 0); // file-chunk read routine 0xbd90 (0xb1b:0x5e0)
                rt.m.trap_ips.insert((0x0cbd, 0x05ff), 0); // the int21 read (0xbdaf)
                eprintln!("traps armed");
            }
            "capglyph" => {
                rt.m.capture_ip = Some((0x043b, 0x06a0)); // glyph blitter entry
                rt.m.capture_ip2 = Some((0x043b, 0x0722)); // pixel write (es:di target)
                rt.m.captured = None;
                rt.m.captured2.clear();
                eprintln!("capture armed at glyph blitter");
            }
            "capdump" => {
                match rt.m.captured {
                    Some((ss,ds,es,si,bp,bx)) => {
                        eprintln!("at glyph blit: ss={ss:04x} ds={ds:04x} es={es:04x} si={si:04x} bp={bp:04x} bx={bx:04x}");
                        let fss = (0..8).map(|i| format!("{:02x}", rt.m.read8(ss, 0x71aa+i))).collect::<Vec<_>>().join(" ");
                        let fgs = (0..8).map(|i| format!("{:02x}", rt.m.read8(0x0e84, 0x71aa+i))).collect::<Vec<_>>().join(" ");
                        eprintln!("  SS:71aa (glyph src) = {fss}");
                        eprintln!("  gs:71aa (valid font)= {fgs}");
                    }
                    None => eprintln!("glyph blitter not captured"),
                }
                eprintln!("glyph pixel targets (es:di): {:04x?}", &rt.m.captured2[..rt.m.captured2.len().min(12)]);
            }
            "trapdump" => {
                let mut rows: Vec<_> = rt.m.trap_ips.iter().collect();
                rows.sort();
                for ((cs,ip),n) in rows { eprintln!("  {cs:04x}:{ip:04x} executed {n} times"); }
            }
            "tracevga" => {
                // trace VGA page-1 (a000:8000) subtitle-band writes during the next draw
                rt.m.trace_range = Some(0xa9000..0xaab00);
                rt.m.range_hits.clear();
                eprintln!("tracing VGA page-1 subtitle-band writes");
            }
            "tracechunky" => {
                // trace all writes to the chunky-buffer subtitle band during the next draw
                let base = 0x266cusize * 16;
                rt.m.trace_range = Some(base + 0x7c00..base + 0x8800);
                rt.m.range_hits.clear();
                eprintln!("tracing chunky subtitle-band writes");
            }
            "tracedump" => {
                let hits = rt.m.range_hits.clone();
                eprintln!("range writes: {}", hits.len());
                // per-code-address summary: count + value histogram
                use std::collections::HashMap;
                let mut by_code: HashMap<(u16,u16), (usize, HashMap<u8,usize>)> = HashMap::new();
                for (_a, v, cs, ip) in &hits {
                    let e = by_code.entry((*cs,*ip)).or_default();
                    e.0 += 1; *e.1.entry(*v).or_default() += 1;
                }
                let mut rows: Vec<_> = by_code.into_iter().collect();
                rows.sort_by_key(|(_,v)| std::cmp::Reverse(v.0));
                for ((cs,ip),(n,vals)) in rows.into_iter().take(12) {
                    let rel = cs.wrapping_sub(0x1a2);
                    let file = 0x600 + (rel as usize)*16 + ip as usize;
                    let mut vv: Vec<_> = vals.into_iter().collect(); vv.sort_by_key(|(_,c)| std::cmp::Reverse(*c));
                    let top: Vec<String> = vv.iter().take(4).map(|(val,c)| format!("0x{val:02x}x{c}")).collect();
                    eprintln!("  {cs:04x}:{ip:04x} (file {file:#07x}) n={n} vals=[{}]", top.join(" "));
                }
                rt.m.trace_range = None;
            }
            "fbptr" => {
                let gs = rt.m.regs.gs;
                let r32 = |off: u32| ((rt.m.read16(gs, off+2) as u32) << 16) | rt.m.read16(gs, off) as u32;
                eprintln!("gs:5219 (cur FB) = {:04x}:{:04x}", rt.m.read16(gs,0x521b), rt.m.read16(gs,0x5219));
                eprintln!("gs:521d (alt FB) = {:04x}:{:04x}", rt.m.read16(gs,0x521f), rt.m.read16(gs,0x521d));
                eprintln!("gs:5221 (disp)   = {:04x}:{:04x}", rt.m.read16(gs,0x5223), rt.m.read16(gs,0x5221));
                let _ = r32;
            }
            "remap" => {
                let gs = rt.m.regs.gs;
                let d = |off: u32, n: u32| (0..n).map(|i| format!("{:02x}", rt.m.read8(gs, off + i))).collect::<Vec<_>>().join(" ");
                eprintln!("remap 5f11[0..32]: {}", d(0x5f11, 32));
                eprintln!("remap 5f11[0xf0..]: {}", d(0x5f11 + 0xf0, 16));
                eprintln!("cmd-table 5e6f: {}", d(0x5e6f, 16));
                eprintln!("cmd-table 5eaf: {}", d(0x5eaf, 16));
            }
            "src190" => {
                let gs = rt.m.regs.gs;
                let mut t = String::new();
                for i in 0..48u32 {
                    let b = rt.m.read8(gs, 0x190 + i);
                    t.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
                }
                eprintln!("gs:0190 source text: \"{t}\"");
            }
            "buf" => {
                let gs = rt.m.regs.gs;
                let mut txt = String::new();
                for i in 0..48u32 {
                    let b = rt.m.read8(gs, 0x0e18 + i);
                    txt.push_str(&format!("{:02x}", b));
                    txt.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
                    txt.push(' ');
                }
                eprintln!("buffer gs:0E18: {txt}");
                eprintln!("reveal_ptr={:04x} (offset {})", rt.m.read16(gs, 0x5e58), rt.m.read16(gs, 0x5e58).wrapping_sub(0x0e18));
                eprintln!("gates: 67bb={:02x} 67bc={:02x} b35={:04x} cfb={:02x}",
                    rt.m.read8(gs, 0x67bb), rt.m.read8(gs, 0x67bc),
                    rt.m.read16(gs, 0xb35), rt.m.read8(gs, 0xcfb));
                eprintln!("draw-mode: 5e65(state)={:04x} 5b56(remap flag)={:02x} 5e64={:02x} 27e2={:04x} 679a={:04x}",
                    rt.m.read16(gs, 0x5e65), rt.m.read8(gs, 0x5b56),
                    rt.m.read8(gs, 0x5e64), rt.m.read16(gs, 0x27e2), rt.m.read16(gs, 0x679a));
            }
            "presflags" => {
                let gs = rt.m.regs.gs;
                eprintln!("67b0={:02x} 67bc={:02x} 679a={:04x} (needs 679a==0x67b0 or 67b0&1) 6724fp={:04x}:{:04x} 674a={:04x}:{:04x} 6728={:04x}:{:04x}",
                    rt.m.read8(gs,0x67b0), rt.m.read8(gs,0x67bc), rt.m.read16(gs,0x679a),
                    rt.m.read16(gs,0x6726), rt.m.read16(gs,0x6724),
                    rt.m.read16(gs,0x674c), rt.m.read16(gs,0x674a),
                    rt.m.read16(gs,0x672a), rt.m.read16(gs,0x6728));
            }
            "revsample" => {
                let count = if w.len() > 1 { arg(1) } else { 20 };
                let step = if w.len() > 2 { arg(2) } else { 20 };
                for _ in 0..count {
                    let gs = rt.m.regs.gs;
                    let txt: String = (0..12u32).map(|i| {
                        let b = rt.m.read8(gs, 0x0e18 + i);
                        if (0x20..0x7f).contains(&b) { b as char } else { '.' }
                    }).collect();
                    eprintln!("t{:>5} phase={} pos={:04x} 67ac={:02x} ade={:02x} ba3={:02x} ba0={:02x} ae2={:02x} 67bc={:02x} 5e64={:02x} 27e2={:02x} txt='{}'",
                        rt.ticks(),
                        rt.m.read16(gs,0x5e65), rt.m.read16(gs,0x5e58),
                        rt.m.read8(gs,0x67ac), rt.m.read8(gs,0xade), rt.m.read8(gs,0xba3),
                        rt.m.read8(gs,0xba0), rt.m.read8(gs,0xae2), rt.m.read8(gs,0x67bc),
                        rt.m.read8(gs,0x5e64), rt.m.read8(gs,0x27e2), txt);
                    for _ in 0..step {
                        if !run_tick(&mut rt).map_err(|e| format!("line {}: {e}", ln + 1))? { return Ok(()); }
                    }
                }
            }
            "memfind" => {
                let needle = w.get(1).unwrap_or(&"CRYO").as_bytes();
                let mut hits = 0;
                let mem = &rt.m.mem;
                for i in 0..mem.len().saturating_sub(needle.len()) {
                    if &mem[i..i+needle.len()] == needle {
                        let seg = (i >> 4) as u32;
                        eprintln!("  found '{}' at lin {:#07x} (~{:04x}:{:04x})",
                            String::from_utf8_lossy(needle), i, seg, (i as u32)&0xf);
                        hits += 1;
                        if hits >= 8 { break; }
                    }
                }
                eprintln!("memfind '{}': {} hits", String::from_utf8_lossy(needle), hits);
            }
            "rdw" => {
                let off = u32::from_str_radix(w.get(1).unwrap_or(&"6eb0"), 16).unwrap_or(0);
                let n = w.get(2).and_then(|s| s.parse().ok()).unwrap_or(8u32);
                let gs = rt.m.regs.gs;
                let vals: Vec<String> = (0..n).map(|i| format!("{:04x}", rt.m.read16(gs, off + i*2))).collect();
                eprintln!("gs:{off:04x} words: {}", vals.join(" "));
                // C4 opcode = idx 0x24, handler at table+0x24*2
                let c4 = rt.m.read16(gs, 0x6eb0 + 0x24*2);
                let c4_cs = rt.m.regs.cs; // handler is in the VM code seg; report file via 0x4da base
                let c4_file = 0x600 + 0x4dausize*16 + c4 as usize;
                eprintln!("  C4 opcode handler ptr = {c4:04x} (VM-seg offset; ~file {c4_file:#07x}) cs~{c4_cs:04x}");
            }
            "vmtrace" => {
                // record al at cs:ip (default 067c:0274 = VM opcode; or pass cs ip to trace elsewhere)
                let cs = w.get(1).and_then(|s| u16::from_str_radix(s,16).ok()).unwrap_or(0x067c);
                let ip = w.get(2).and_then(|s| u16::from_str_radix(s,16).ok()).unwrap_or(0x0274);
                rt.m.vm_trace_ip = Some((cs, ip));
                rt.m.vm_ops.clear();
                eprintln!("al trace armed at {cs:04x}:{ip:04x}");
            }
            "vmdump" => {
                let ops = &rt.m.vm_ops;
                eprintln!("VM opcodes ({}): {}", ops.len(),
                    ops.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
                // opcodes are 0xA0-based; note key ones: C4=present, D2=schedule
                let key: Vec<String> = ops.iter().filter(|b| **b >= 0xc0).map(|b| format!("{b:02x}")).collect();
                eprintln!("  >=0xC0 opcodes seen: {}", key.join(" "));
            }
            "resname" => {
                let fs = rt.m.regs.fs;
                for ids in w.iter().skip(1) {
                    if let Ok(id) = ids.parse::<u32>() {
                        let mut name = String::new();
                        for i in 0..14u32 {
                            let b = rt.m.read8(fs, 0x0c04 + id*16 + i);
                            if b == 0 { break; }
                            name.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
                        }
                        // resource_handle_resolve (0x5320): bx=id<<3; seg=fs:[bx], flags=fs:[bx+2]; loaded iff flags&3
                        let hseg = rt.m.read16(fs, id*8);
                        let hflags = rt.m.read16(fs, id*8 + 2);
                        let hsize = rt.m.read16(fs, id*8 + 4) as u32 | ((rt.m.read16(fs, id*8+6) as u32) << 16);
                        eprintln!("  id {id}: \"{name}\"  handle[seg={hseg:04x} flags={hflags:04x} loaded={} size={hsize}]",
                            hflags & 3 != 0);
                    }
                }
            }
            "rdstr" => {
                let off = u32::from_str_radix(w.get(1).unwrap_or(&"d2d"), 16).unwrap_or(0);
                let gs = rt.m.regs.gs;
                let mut t = String::new();
                for i in 0..40u32 {
                    let b = rt.m.read8(gs, off + i);
                    t.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
                }
                eprintln!("gs:{off:04x} = \"{t}\"  [0xc49 handle={:04x}]", rt.m.read16(gs, 0xc49));
            }
            "trapadd" => {
                let cs = u16::from_str_radix(w.get(1).unwrap_or(&"0cbd"), 16).unwrap_or(0);
                let ip = u16::from_str_radix(w.get(2).unwrap_or(&"0610"), 16).unwrap_or(0);
                rt.m.trap_ips.insert((cs, ip), 0);
                eprintln!("trap added {cs:04x}:{ip:04x}");
            }
            "trapclear" => { rt.m.trap_ips.clear(); eprintln!("traps cleared"); }
            "capret" => {
                // capret <cs_hex> <ip_hex>: capture the return address (dispatch site) when (cs,ip) is first hit
                let cs = u16::from_str_radix(w.get(1).unwrap_or(&"0cbd"), 16).unwrap_or(0x0cbd);
                let ip = u16::from_str_radix(w.get(2).unwrap_or(&"0610"), 16).unwrap_or(0x0610);
                rt.m.capture_ip = Some((cs, ip));
                rt.m.captured = None;
                rt.m.capture_ret = None;
                eprintln!("capret armed for {cs:04x}:{ip:04x}");
            }
            "capretdump" => {
                if let Some((sp, w0, w1, w2)) = rt.m.capture_ret {
                    // interpret [ss:sp] as near-return ip in the same cs, and far (ip,cs)
                    let cs = rt.m.capture_ip.map(|(c,_)| c).unwrap_or(0);
                    let near_file = 0x600 + (cs.wrapping_sub(0x1a2) as usize)*16 + w0 as usize;
                    let far_cs = w1;
                    let far_file = 0x600 + (far_cs.wrapping_sub(0x1a2) as usize)*16 + w0 as usize;
                    eprintln!("capret sp={sp:04x} stack=[{w0:04x} {w1:04x} {w2:04x}]");
                    eprintln!("  if NEAR call: caller ret {cs:04x}:{w0:04x} (file {near_file:#07x})");
                    eprintln!("  if FAR  call: caller ret {far_cs:04x}:{w0:04x} (file {far_file:#07x})");
                    if let Some((ss,ds,es,si,bp,bx)) = rt.m.captured {
                        eprintln!("  regs at entry: ss={ss:04x} ds={ds:04x} es={es:04x} si={si:04x} bp={bp:04x} bx={bx:04x}");
                    }
                    if let Some((pcs,pip)) = rt.m.captured_prev {
                        let pfile = 0x600 + (pcs.wrapping_sub(0x1a2) as usize)*16 + pip as usize;
                        eprintln!("  PREV instr (jump/call source): {pcs:04x}:{pip:04x} (file {pfile:#07x})");
                    }
                } else {
                    eprintln!("capret: not hit");
                }
            }
            "capsegdump" => {
                if let Some(seg) = &rt.m.captured_seg {
                    eprintln!("captured ds:0..64 = {}", seg.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" "));
                } else { eprintln!("capseg: not hit"); }
            }
            "ipstart" => { rt.ip_sample = Some(Default::default()); eprintln!("ip sampling on"); }
            "ipdump" => {
                if let Some(h) = rt.ip_sample.take() {
                    let mut v: Vec<_> = h.into_iter().collect();
                    v.sort_by_key(|(_,c)| std::cmp::Reverse(*c));
                    let total: u64 = v.iter().map(|(_,c)| *c).sum();
                    eprintln!("ip histogram (top hot spots, total {total} samples):");
                    for ((cs,ip),c) in v.into_iter().take(14) {
                        let rel = cs.wrapping_sub(0x1a2);
                        let file = 0x600 + (rel as usize)*16 + ip as usize;
                        eprintln!("  {cs:04x}:{ip:04x} (seg 0x{rel:x} file {file:#07x}): {c} ({:.0}%)", 100.0*c as f64/total as f64);
                    }
                }
            }
            "resid" => {
                let gs = rt.m.regs.gs;
                let fs = rt.m.regs.fs;
                let id = rt.m.read16(gs, 0xc3b);
                let mut name = String::new();
                for i in 0..14u32 {
                    let b = rt.m.read8(fs, 0x0c04 + (id as u32)*16 + i);
                    if b == 0 { break; }
                    name.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
                }
                eprintln!("resource [0xc3b]={id} fs={fs:04x} name-table entry = \"{name}\"");
                // also dump a few nearby resource ids' names
                for rid in [id.wrapping_sub(1), id, id.wrapping_add(1)] {
                    let mut n=String::new();
                    for i in 0..14u32 { let b=rt.m.read8(fs,0x0c04+(rid as u32)*16+i); if b==0{break} n.push(if (0x20..0x7f).contains(&b){b as char}else{'.'}); }
                    eprintln!("  id {rid}: \"{n}\"");
                }
            }
            "peek" => {
                // peek gs-relative words: reveal pointer 0x5E58, timer 0xB31, text-speed 0xACA
                let gs = rt.m.regs.gs;
                eprintln!(
                    "tick {} gs={:04x} reveal_ptr(5E58)={:04x} timer(B31)={:04x} speed(ACA)={:04x} buf0(0E18)={:02x}{:02x}{:02x}{:02x}",
                    rt.ticks(), gs,
                    rt.m.read16(gs, 0x5e58), rt.m.read16(gs, 0xb31), rt.m.read16(gs, 0xaca),
                    rt.m.read8(gs, 0x0e18), rt.m.read8(gs, 0x0e19), rt.m.read8(gs, 0x0e1a), rt.m.read8(gs, 0x0e1b),
                );
            }
            "watchef" => {
                let start = 0xa0000 + 0x8000 + 100 * 80;
                rt.m.watch = Some((0xef, start..start + 30 * 80 + 0x30000));
                rt.m.watch_hits.clear();
                eprintln!("watch armed for 0xEF writes to VRAM subtitle band");
            }
            "watchval" => {
                // watchval <hex>: watch that value written anywhere in VGA page-1 band
                let v = u8::from_str_radix(w.get(1).unwrap_or(&"fd"), 16).unwrap_or(0xfd);
                let start = 0xa0000 + 0x8000 + 60 * 80;
                rt.m.watch = Some((v, start..start + 80 * 80 + 0x30000));
                rt.m.watch_hits.clear();
                eprintln!("watch armed for 0x{v:02x} writes to VGA page-1");
            }
            "watchchunky" => {
                // 0xEF writes into the chunky back-buffer (seg 0x266c) subtitle band
                let base = 0x266cusize * 16;
                rt.m.watch = Some((0xef, base + 0x7c00..base + 0x8600));
                rt.m.watch_hits.clear();
                eprintln!("watch armed for 0xEF writes to chunky buffer band");
            }
            "watchdump" => {
                let mut hits = rt.m.watch_hits.clone();
                hits.sort();
                eprintln!("0xEF writers ({}):", hits.len());
                for (cs, ip, ds, si, _addr) in &hits {
                    let rel = cs.wrapping_sub(0x1a2);
                    let file = 0x600 + (rel as usize) * 16 + *ip as usize;
                    eprintln!("  cs:ip={cs:04x}:{ip:04x} (seg 0x{rel:x}, file ~{file:#07x}) src ds:si={ds:04x}:{si:04x}");
                }
                // dump the chunky source around the first hit's ds:si
                if let Some((_, _, ds, si, _)) = hits.first() {
                    let base = si.saturating_sub(4);
                    let mut chunk = Vec::new();
                    for i in 0..320u32 { chunk.push(rt.m.read8(*ds, base as u32 + i)); }
                    std::fs::write(out_dir.join("chunky_src.bin"), &chunk).unwrap();
                    eprintln!("dumped 320B chunky source at {ds:04x}:{base:04x}");
                }
                rt.m.watch = None;
            }
            "scanpages" => {
                let v = rt.m.vga.as_deref().unwrap();
                for (name, base) in [("page0", 0usize), ("page1", 0x8000usize)] {
                    let mut cnt = std::collections::HashMap::<u8, usize>::new();
                    for plane in 0..4 {
                        for o in base..base + 0x8000 {
                            *cnt.entry(v.planes[plane * 0x10000 + o]).or_default() += 1;
                        }
                    }
                    let fdfe = cnt.get(&0xfd).copied().unwrap_or(0) + cnt.get(&0xfe).copied().unwrap_or(0) + cnt.get(&0xff).copied().unwrap_or(0);
                    let ef = cnt.get(&0xef).copied().unwrap_or(0);
                    eprintln!("{name}: 0xFD/FE/FF pixels={fdfe}  0xEF pixels={ef}");
                }
            }
            "dumpvram" => {
                // dump planes at the subtitle rows for offline glyph reconstruction
                let v = rt.m.vga.as_deref().unwrap();
                let start = 0x8000usize;
                let mut out = Vec::new();
                for y in 95..130usize {
                    for plane in 0..4usize {
                        for xb in 0..80usize {
                            out.push(v.planes[plane * 0x10000 + start + y * 80 + xb]);
                        }
                    }
                }
                std::fs::write(out_dir.join("vram_sub.bin"), &out).unwrap();
                eprintln!("dumped {} bytes of subtitle VRAM (35 rows x 4 planes x 80)", out.len());
            }
            "forcesub" => {
                rt.force_sub = true;
                eprintln!("force_sub enabled (27e2=2 each frame)");
            }
            "vga" => {
                let v = rt.m.vga.as_deref().unwrap();
                eprintln!(
                    "VGA: chain4={} map_mask={:#x} read_map={} write_mode={} bit_mask={:#x} set_reset={:#x} enable_sr={:#x} logic_op={} rotate={}",
                    v.chain4, v.map_mask, v.read_map, v.write_mode, v.bit_mask, v.set_reset, v.enable_sr, v.logic_op, v.rotate,
                );
            }
            other => return Err(format!("line {}: unknown command {other}", ln + 1)),
        }
    }
    rt.write_ppm(&out_dir.join("end.ppm")).map_err(|e| e.to_string())?;
    if !rt.sb_pcm.is_empty() {
        write_wav(&out_dir.join("end.wav"), &rt.sb_pcm, rt.sb_pcm_rate).map_err(|e| e.to_string())?;
        eprintln!("wav: {} bytes at {} Hz", rt.sb_pcm.len(), rt.sb_pcm_rate);
    }
    Ok(())
}

/// Minimal 8-bit unsigned mono WAV writer for the SB PCM tap.
fn write_wav(path: &std::path::Path, pcm: &[u8], rate: u32) -> std::io::Result<()> {
    let mut d = Vec::with_capacity(44 + pcm.len());
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes()); // PCM
    d.extend_from_slice(&1u16.to_le_bytes()); // mono
    d.extend_from_slice(&rate.to_le_bytes());
    d.extend_from_slice(&rate.to_le_bytes()); // byte rate (8-bit mono)
    d.extend_from_slice(&1u16.to_le_bytes()); // block align
    d.extend_from_slice(&8u16.to_le_bytes()); // bits
    d.extend_from_slice(b"data");
    d.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    d.extend_from_slice(pcm);
    std::fs::write(path, d)
}

/// Live audio: a ring buffer the emulation fills from the SB PCM tap and a cpal stream
/// drains with naive resampling. Absent an output device the game just runs silent.
struct AudioOut {
    _stream: cpal::Stream,
    ring: Arc<Mutex<VecDeque<u8>>>,
    src_rate: Arc<AtomicU32>,
}

impl AudioOut {
    fn start() -> Option<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let dev_rate = config.sample_rate().0.max(1) as u64;
        let channels = config.channels() as usize;
        let ring: Arc<Mutex<VecDeque<u8>>> = Arc::default();
        let src_rate = Arc::new(AtomicU32::new(11111));
        let (r2, sr) = (Arc::clone(&ring), Arc::clone(&src_rate));
        let mut frac: u64 = 0;
        let mut cur: f32 = 0.0;
        let stream = device
            .build_output_stream(
                &config.config(),
                move |out: &mut [f32], _| {
                    let mut q = r2.lock().unwrap();
                    let step = ((sr.load(Ordering::Relaxed) as u64) << 16) / dev_rate;
                    for frame in out.chunks_mut(channels) {
                        frac += step;
                        while frac >= 1 << 16 {
                            frac -= 1 << 16;
                            match q.pop_front() {
                                Some(b) => cur = (b as f32 - 128.0) / 128.0,
                                None => cur *= 0.995, // underrun: decay to silence, no click
                            }
                        }
                        for s in frame.iter_mut() {
                            *s = cur;
                        }
                    }
                },
                |_| {},
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Self {
            _stream: stream,
            ring,
            src_rate,
        })
    }
}

fn run_window(turbo: bool) -> anyhow::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        AtomEnum, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, ImageFormat, PropMode,
        WindowClass,
    };
    use x11rb::wrapper::ConnectionExt as _;

    let mut rt = make_runtime();
    let (src_w, src_h) = (320usize, 200usize);
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let (mut win_w, mut win_h) = (960u16, 600u16);
    let win = conn.generate_id()?;
    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        win_w,
        win_h,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new().event_mask(
            EventMask::EXPOSURE
                | EventMask::POINTER_MOTION
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::KEY_PRESS
                | EventMask::STRUCTURE_NOTIFY,
        ),
    )?;
    conn.change_property8(
        PropMode::REPLACE,
        win,
        u32::from(AtomEnum::WM_NAME),
        u32::from(AtomEnum::STRING),
        b"Commander Blood",
    )?;
    conn.map_window(win)?;
    let gc = conn.generate_id()?;
    conn.create_gc(gc, win, &CreateGCAux::new())?;
    conn.flush()?;

    let audio = AudioOut::start();
    let mut pcm_consumed = 0usize;
    let max_req = 262_144usize;
    let (mut mx, mut my, mut mb) = (160u16, 100u16, 0u16);
    let mut next_frame = Instant::now();
    loop {
        let scale = ((win_w as usize / src_w).min(win_h as usize / src_h)).max(1);
        let (dst_w, dst_h) = (src_w * scale, src_h * scale);
        let off_x = (win_w as usize).saturating_sub(dst_w) / 2;
        let off_y = (win_h as usize).saturating_sub(dst_h) / 2;
        let to_src = |ex: i16, ey: i16| -> (u16, u16) {
            let x = ((ex as isize - off_x as isize).clamp(0, dst_w as isize - 1) as usize / scale)
                .min(src_w - 1) as u16;
            let y = ((ey as isize - off_y as isize).clamp(0, dst_h as isize - 1) as usize / scale)
                .min(src_h - 1) as u16;
            (x, y)
        };
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::MotionNotify(m) => {
                    (mx, my) = to_src(m.event_x, m.event_y);
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::ButtonPress(b) if b.detail == 1 || b.detail == 3 => {
                    mb |= if b.detail == 1 { 1 } else { 2 };
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::ButtonRelease(b) if b.detail == 1 || b.detail == 3 => {
                    mb &= if b.detail == 1 { !1 } else { !2 };
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::KeyPress(k) => {
                    // X evdev keycodes are PC scancodes + 8 for the classic set.
                    let scan = k.detail.saturating_sub(8);
                    rt.key_event(scan, scancode_ascii(scan));
                }
                Event::ConfigureNotify(c) => {
                    if c.width != win_w || c.height != win_h {
                        win_w = c.width;
                        win_h = c.height;
                    }
                }
                _ => {}
            }
        }

        match run_tick(&mut rt) {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(e) => anyhow::bail!("runtime fault: {e}"),
        }

        // Feed freshly streamed SB PCM to the audio ring (cap ~2 s to bound turbo runaway).
        if let Some(a) = &audio {
            if rt.sb_pcm.len() > pcm_consumed {
                let mut q = a.ring.lock().unwrap();
                q.extend(&rt.sb_pcm[pcm_consumed..]);
                let cap = (rt.sb_pcm_rate as usize) * 2;
                while q.len() > cap {
                    q.pop_front();
                }
                a.src_rate.store(rt.sb_pcm_rate, Ordering::Relaxed);
            }
        }
        pcm_consumed = rt.sb_pcm.len();

        // Present: scale the 320x200 frame into a BGRX image, letterboxed.
        let rgb = rt.screenshot_rgb();
        let mut image = vec![0u8; win_w as usize * win_h as usize * 4];
        for y in 0..dst_h.min(win_h as usize) {
            let sy = y / scale;
            for x in 0..dst_w.min(win_w as usize) {
                let sx = x / scale;
                let si = (sy * src_w + sx) * 3;
                let di = ((y + off_y) * win_w as usize + x + off_x) * 4;
                image[di] = rgb[si + 2];
                image[di + 1] = rgb[si + 1];
                image[di + 2] = rgb[si];
            }
        }
        let stride = win_w as usize * 4;
        let rows_per_chunk = (max_req / stride).max(1);
        let mut row = 0usize;
        while row < win_h as usize {
            let n = rows_per_chunk.min(win_h as usize - row);
            conn.put_image(
                ImageFormat::Z_PIXMAP,
                win,
                gc,
                win_w,
                n as u16,
                0,
                row as i16,
                0,
                screen.root_depth,
                &image[row * stride..(row + n) * stride],
            )?;
            row += n;
        }
        conn.flush()?;

        if !turbo {
            next_frame += TICK;
            let now = Instant::now();
            if next_frame > now {
                std::thread::sleep(next_frame - now);
            } else {
                next_frame = now;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut script: Option<String> = None;
    let mut out = PathBuf::from("blood_shots");
    let mut turbo = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--script" => {
                i += 1;
                script = Some(args[i].clone());
            }
            "--out" => {
                i += 1;
                out = PathBuf::from(&args[i]);
            }
            "--turbo" => turbo = true,
            a => anyhow::bail!("unknown arg {a}"),
        }
        i += 1;
    }
    match script {
        Some(s) => run_script(&s, &out).map_err(|e| anyhow::anyhow!(e)),
        None => run_window(turbo),
    }
}
