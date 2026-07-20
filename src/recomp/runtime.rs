//! The DOS/BIOS/hardware layer around [`Cpu`] + [`Machine`] — boots the REAL BLOODPRG.EXE.
//!
//! This is path B's runtime (M2 of the roadmap in re/REVERSE.md): an MZ loader, a PSP, the
//! int 21h DOS services the game uses (file I/O rooted at host dirs mapped as C:/D:), EMS
//! (int 67h) backed by memory above 1 MB, BIOS video/keyboard/timer services, and the VGA DAC
//! port protocol. The interpreter exits ([`Exit`]) are the ONLY seam: `int`/`in`/`out`/`hlt`
//! arrive here, everything else is the original game code executing over [`Machine`].
//!
//! Interrupt model: ALL interrupts dispatch through the guest IVT (so game hooks compose
//! naturally). Every vector initially points at a one-byte `hlt` stub in a reserved BIOS
//! segment; when execution lands there the runtime services the interrupt natively and performs
//! the `iret`. A game hook that chains to the original vector therefore reaches the native
//! service exactly like real DOS.

use super::interp::{Cpu, Exit};
use super::machine::Machine;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Segment of the 256 one-byte interrupt stubs (`hlt` at `STUB_SEG:v*4`).
const STUB_SEG: u16 = 0xf000;
/// Dedicated stub segment for int 67h so the EMS device name can live at offset 0x0A
/// (the standard EMS presence check reads `seg(IVT[67h]):000A` for "EMMXXXX0").
const EMS_STUB_SEG: u16 = 0xf200;
/// EMS physical page frame (4 × 16 KB pages at E000:0000).
const EMS_FRAME_SEG: u16 = 0xe000;
/// EMS logical page store: above the 1 MB real-mode space inside Machine memory.
const EMS_STORE: usize = 0x10_0000;
const EMS_PAGE: usize = 0x4000;
const EMS_MAX_PAGES: usize = (super::machine::MEM_SIZE - EMS_STORE) / EMS_PAGE;

const PSP_SEG: u16 = 0x0192;
const ENV_SEG: u16 = 0x0180;
const MEM_TOP_SEG: u16 = 0xa000;

/// How a runtime run ended.
#[derive(Debug)]
pub enum RunEnd {
    /// int 21h AH=4Ch with the exit code.
    Exited(u8),
    /// The step budget given to `run` was consumed (normal for incremental boots).
    StepBudget,
    /// Something the runtime can't service yet — the message carries full context to fix it.
    Fatal(String),
}

struct HostFile {
    f: std::fs::File,
}

/// DOS 8.3 wildcard match: both sides are padded to 11-char name+ext masks, `*` expands to `?`s
/// for the rest of its component, `?` matches any char including a blank.
fn dos_wildcard_match(pattern: &str, name: &str) -> bool {
    fn mask(s: &str) -> [u8; 11] {
        let mut out = [b' '; 11];
        let (n, e) = match s.find('.') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => (s, ""),
        };
        let (name8, ext3) = out.split_at_mut(8);
        for (dst, src) in [(name8, n), (ext3, e)] {
            let mut i = 0;
            for c in src.bytes() {
                if c == b'*' {
                    while i < dst.len() {
                        dst[i] = b'?';
                        i += 1;
                    }
                    break;
                }
                if i >= dst.len() {
                    break;
                }
                dst[i] = c;
                i += 1;
            }
        }
        out
    }
    let p = mask(pattern);
    let n = mask(name);
    p.iter().zip(n.iter()).all(|(pc, nc)| *pc == b'?' || pc == nc)
}

pub struct Runtime {
    pub m: Machine,
    pub cpu: Cpu,
    drive_roots: [Option<PathBuf>; 26], // index 2 = C:, 3 = D:
    cur_drive: u8,                      // 0-based (2 = C:)
    cwd: [String; 26],                  // per-drive, "\\"-separated, no leading slash
    files: Vec<Option<HostFile>>,
    dta: (u16, u16),
    ticks: u32,
    next_tick_at: u64,
    pub steps_per_tick: u64,
    pit_divisor: u32,
    // EMS: handle -> logical pages (indices into the store)
    ems_handles: Vec<Option<Vec<u32>>>,
    ems_next_page: u32,
    ems_map: [Option<u32>; 4],
    // VGA
    pub vga_mode: u8,
    pub dac: [u8; 768],
    dac_widx: usize,
    dac_ridx: usize,
    seq_idx: u8,
    seq: [u8; 8],
    gc_idx: u8,
    gc: [u8; 16],
    crtc_idx: u8,
    crtc: [u8; 32],
    cmos_idx: u8,
    /// Conventional-memory accounting: the program block ends here after int 21h/4Ah; 48h
    /// allocations bump from `alloc_next` toward MEM_TOP_SEG.
    prog_end: u16,
    alloc_next: u16,
    kbd_queue: VecDeque<(u8, u8)>, // (scancode, ascii) pending hardware events
    bios_keys: VecDeque<(u8, u8)>, // decoded buffer served by int 16h
    /// FindFirst/FindNext state per DTA address: (matches, next index).
    searches: HashMap<(u16, u16), (Vec<(String, u32, u8)>, usize)>,
    /// (vector, AH) -> count, for the boot log.
    pub int_log: HashMap<(u8, u8), u64>,
    pub trace_ints: bool,
    console: String,
    exit_code: Option<u8>,
}

impl Runtime {
    pub fn new(c_root: PathBuf, d_root: PathBuf) -> Self {
        let mut drive_roots: [Option<PathBuf>; 26] = Default::default();
        drive_roots[2] = Some(c_root);
        drive_roots[3] = Some(d_root);
        let mut rt = Self {
            m: Machine::new(),
            cpu: Cpu::new(0, 0),
            drive_roots,
            cur_drive: 3,
            cwd: std::array::from_fn(|_| String::new()),
            files: (0..5).map(|_| None).collect(), // 0..4 reserved (stdio devices)
            dta: (PSP_SEG, 0x80),
            ticks: 0,
            next_tick_at: 0,
            steps_per_tick: 450_000, // ~18.2 Hz at a modelled ~8 MIPS; PIT reprogramming scales it
            pit_divisor: 0x1_0000,
            ems_handles: vec![Some(vec![])], // handle 0 reserved (the OS handle)
            ems_next_page: 0,
            ems_map: [None; 4],
            prog_end: MEM_TOP_SEG,
            alloc_next: MEM_TOP_SEG,
            vga_mode: 3,
            dac: [0; 768],
            dac_widx: 0,
            dac_ridx: 0,
            seq_idx: 0,
            seq: [0x0e; 8], // chain-4 defaults so read-modify-write leaves mode 13h intact
            gc_idx: 0,
            gc: [0; 16],
            crtc_idx: 0,
            crtc: [0; 32],
            cmos_idx: 0,
            kbd_queue: VecDeque::new(),
            bios_keys: VecDeque::new(),
            searches: HashMap::new(),
            int_log: HashMap::new(),
            trace_ints: false,
            console: String::new(),
            exit_code: None,
        };
        rt.m.vga = Some(Box::default());
        rt.init_bios();
        rt
    }

    fn init_bios(&mut self) {
        // IVT: every vector -> hlt stub at STUB_SEG:v*4 (int 67h gets its own segment).
        for v in 0u32..256 {
            self.m.write16(0, v * 4, (v * 4) as u16);
            self.m.write16(0, v * 4 + 2, STUB_SEG);
            self.m.write8(STUB_SEG, v * 4, 0xf4);
        }
        self.m.write16(0, 0x67 * 4, 0);
        self.m.write16(0, 0x67 * 4 + 2, EMS_STUB_SEG);
        self.m.write8(EMS_STUB_SEG, 0, 0xf4);
        for (i, b) in b"EMMXXXX0".iter().enumerate() {
            self.m.write8(EMS_STUB_SEG, 0x0a + i as u32, *b);
        }
        // BIOS data area.
        self.m.write16(0x40, 0x10, 0x0021); // equipment: 80x25 color
        self.m.write16(0x40, 0x13, 640); // conventional KB
        self.m.write8(0x40, 0x49, 3); // video mode
        self.m.write16(0x40, 0x4a, 80); // columns
        self.m.write16(0x40, 0x63, 0x3d4); // CRTC base
        self.m.write32(0x40, 0x6c, 0); // tick counter
        self.m.write16(0x40, 0x1a, 0x1e); // kbd buffer head
        self.m.write16(0x40, 0x1c, 0x1e); // kbd buffer tail
        self.m.write8(0x40, 0x84, 24); // rows-1
    }

    /// Build PSP + environment and load the MZ executable, exactly like DOS EXEC.
    pub fn load_exe(&mut self, exe: &[u8], cmd_tail: &str, program_path: &str) -> Result<(), String> {
        if exe.len() < 28 || &exe[0..2] != b"MZ" {
            return Err("not an MZ executable".into());
        }
        let word = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]);
        let cblp = word(2) as usize;
        let cp = word(4) as usize;
        let crlc = word(6) as usize;
        let cparhdr = word(8) as usize;
        let h_ss = word(14);
        let h_sp = word(16);
        let h_ip = word(20);
        let h_cs = word(22);
        let lfarlc = word(24) as usize;
        let total = cp * 512 - if cblp != 0 { 512 - cblp } else { 0 };
        let img = &exe[cparhdr * 16..total];
        let img_seg = PSP_SEG + 0x10;
        let base = (img_seg as usize) * 16;
        self.m.mem[base..base + img.len()].copy_from_slice(img);
        for i in 0..crlc {
            let off = word(lfarlc + i * 4) as u32;
            let seg = word(lfarlc + i * 4 + 2);
            let s = img_seg.wrapping_add(seg);
            let v = self.m.read16(s, off);
            self.m.write16(s, off, v.wrapping_add(img_seg));
        }

        // Environment block: minimal, then word 1 + the full program path (DOS 3+).
        let mut env: Vec<u8> = Vec::new();
        env.extend_from_slice(b"COMSPEC=C:\\COMMAND.COM\0PATH=D:\\\0\0");
        env.extend_from_slice(&[1, 0]);
        env.extend_from_slice(program_path.as_bytes());
        env.push(0);
        let eb = (ENV_SEG as usize) * 16;
        self.m.mem[eb..eb + env.len()].copy_from_slice(&env);

        // PSP.
        let p = PSP_SEG;
        self.m.write16(p, 0x00, 0x20cd); // int 20h
        self.m.write16(p, 0x02, MEM_TOP_SEG); // first segment beyond the allocation
        self.m.write16(p, 0x2c, ENV_SEG);
        self.m.write16(p, 0x32, 20); // handle count
        self.m.write16(p, 0x34, 0x18);
        self.m.write16(p, 0x36, p);
        for i in 0..20u32 {
            self.m
                .write8(p, 0x18 + i, if i < 5 { i as u8 } else { 0xff });
        }
        self.m.write8(p, 0x50, 0xcd); // int 21h / retf stub
        self.m.write8(p, 0x51, 0x21);
        self.m.write8(p, 0x52, 0xcb);
        let tail = cmd_tail.as_bytes();
        self.m.write8(p, 0x80, tail.len() as u8);
        for (i, b) in tail.iter().enumerate() {
            self.m.write8(p, 0x81 + i as u32, *b);
        }
        self.m.write8(p, 0x81 + tail.len() as u32, 0x0d);

        self.m.regs.ds = p;
        self.m.regs.es = p;
        self.m.regs.ss = img_seg.wrapping_add(h_ss);
        self.m.regs.set_sp(h_sp);
        self.m.regs.set_ax(0);
        self.cpu.cs = img_seg.wrapping_add(h_cs);
        self.cpu.ip = h_ip;
        self.m.regs.cs = self.cpu.cs;
        // Whole-program run: depth-0 ret exits are an oracle-replay feature, disable them.
        self.cpu.depth = 1 << 30;
        Ok(())
    }

    // ---------------- host path mapping ----------------

    fn read_asciiz(&self, seg: u16, off: u16) -> String {
        let mut s = String::new();
        for i in 0..256u32 {
            let b = self.m.read8(seg, off as u32 + i);
            if b == 0 {
                break;
            }
            s.push(b as char);
        }
        s
    }

    /// Resolve a DOS path to a host path. `create`: final component may not exist yet.
    fn resolve(&self, dos_path: &str, create: bool) -> Result<PathBuf, u16> {
        let mut p = dos_path.replace('/', "\\");
        let mut drive = self.cur_drive;
        let bytes = p.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            drive = (bytes[0] as char).to_ascii_uppercase() as u8 - b'A';
            p = p[2..].to_string();
        }
        let root = self
            .drive_roots
            .get(drive as usize)
            .and_then(|r| r.as_ref())
            .ok_or(3u16)?;
        let rel = if let Some(stripped) = p.strip_prefix('\\') {
            stripped.to_string()
        } else if self.cwd[drive as usize].is_empty() {
            p
        } else {
            format!("{}\\{}", self.cwd[drive as usize], p)
        };
        let mut host = root.clone();
        let comps: Vec<&str> = rel.split('\\').filter(|c| !c.is_empty() && *c != ".").collect();
        for (i, comp) in comps.iter().enumerate() {
            if *comp == ".." {
                host.pop();
                continue;
            }
            // case-insensitive lookup
            let found = std::fs::read_dir(&host)
                .ok()
                .and_then(|rd| {
                    rd.filter_map(|e| e.ok())
                        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))
                })
                .map(|e| e.path());
            match found {
                Some(f) => host = f,
                None => {
                    if create && i == comps.len() - 1 {
                        host.push(comp);
                        return Ok(host);
                    }
                    return Err(if i == comps.len() - 1 { 2 } else { 3 });
                }
            }
        }
        Ok(host)
    }

    fn alloc_handle(&mut self, hf: HostFile) -> u16 {
        for (i, slot) in self.files.iter_mut().enumerate().skip(5) {
            if slot.is_none() {
                *slot = Some(hf);
                return i as u16;
            }
        }
        self.files.push(Some(hf));
        (self.files.len() - 1) as u16
    }

    // ---------------- the run loop ----------------

    pub fn run(&mut self, max_steps: u64) -> RunEnd {
        loop {
            if let Some(c) = self.exit_code {
                return RunEnd::Exited(c);
            }
            if self.cpu.steps >= max_steps {
                return RunEnd::StepBudget;
            }
            // timer IRQ0
            if self.cpu.steps >= self.next_tick_at {
                if self.cpu.iflag {
                    self.next_tick_at = self.cpu.steps + self.steps_per_tick;
                    self.cpu.deliver_int(&mut self.m, 8);
                } else {
                    self.next_tick_at = self.cpu.steps + 1000; // retry once IF is set
                }
            }
            let budget = (max_steps - self.cpu.steps).min(4096);
            match self.cpu.run(&mut self.m, budget) {
                Exit::StepLimit => {}
                Exit::Int { vector } => self.cpu.deliver_int(&mut self.m, vector),
                Exit::Hlt => {
                    let (cs, ip) = (self.cpu.cs, self.cpu.ip);
                    let v = if cs == STUB_SEG && ip >= 1 {
                        ((ip - 1) / 4) as u8
                    } else if cs == EMS_STUB_SEG {
                        0x67
                    } else {
                        return RunEnd::Fatal(format!("hlt outside stubs at {cs:04x}:{ip:04x}"));
                    };
                    if let Err(e) = self.native_int(v) {
                        return RunEnd::Fatal(e);
                    }
                }
                Exit::In { port, size } => {
                    let val = self.port_in(port, size);
                    match size {
                        1 => self.m.regs.set_al(val as u8),
                        2 => self.m.regs.set_ax(val as u16),
                        _ => self.m.regs.eax = val,
                    }
                }
                Exit::Out { port, size, value } => self.port_out(port, size, value),
                Exit::Unimplemented { cs, ip, byte, what } => {
                    let ctx: Vec<String> = (0..8)
                        .map(|i| format!("{:02x}", self.m.read8(cs, ip.wrapping_sub(1).wrapping_add(i) as u32)))
                        .collect();
                    return RunEnd::Fatal(format!(
                        "unimplemented {what} (op {byte:#04x}) at {cs:04x}:{ip:04x} bytes [{}]",
                        ctx.join(" ")
                    ));
                }
                Exit::Ret | Exit::Retf => return RunEnd::Fatal("depth-0 ret in runtime".into()),
            }
        }
    }

    // ---------------- port I/O ----------------

    fn port_in(&mut self, port: u16, _size: u8) -> u32 {
        match port {
            0x3c9 => {
                let v = self.dac[self.dac_ridx % 768];
                self.dac_ridx = (self.dac_ridx + 1) % 768;
                v as u32
            }
            0x3c6 => 0xff,
            0x3da => {
                // vsync bit 3 + display-enable bit 0, derived from the step clock so polls make
                // progress. One "frame" ~ steps_per_tick*18/70; vsync asserted for a slice of it.
                let frame = (self.steps_per_tick * 18 / 70).max(1000);
                let ph = self.cpu.steps % frame;
                let vsync = ph < frame / 12;
                let hblank = vsync || (self.cpu.steps % 97) < 20;
                ((vsync as u32) << 3) | hblank as u32
            }
            0x60 => self.kbd_queue.front().map(|(s, _)| *s as u32).unwrap_or(0),
            0x61 => 0x20,
            0x71 => match self.cmos_idx {
                0x00 => 0x27, // RTC seconds (fixed: deterministic PRNG seed)
                0x02 => 0x30,
                0x04 => 0x12,
                _ => 0,
            },
            0x3c5 => self.seq[(self.seq_idx & 7) as usize] as u32,
            0x3c4 => self.seq_idx as u32,
            0x3cf => self.gc[(self.gc_idx & 15) as usize] as u32,
            0x3ce => self.gc_idx as u32,
            0x3d5 => self.crtc[(self.crtc_idx & 31) as usize] as u32,
            0x3d4 => self.crtc_idx as u32,
            0x42 => (self.cpu.steps >> 2) as u32 & 0xff, // PIT ch2: a moving count
            0x40 => (self.cpu.steps >> 1) as u32 & 0xff,
            0x201 => 0xff, // joystick: none
            _ => {
                if self.trace_ints {
                    eprintln!("in port {port:#x}");
                }
                0xff
            }
        }
    }

    fn port_out(&mut self, port: u16, _size: u8, value: u32) {
        let v = value as u8;
        match port {
            0x3c8 => {
                self.dac_widx = (v as usize) * 3;
            }
            0x3c9 => {
                self.dac[self.dac_widx % 768] = v & 0x3f;
                self.dac_widx = (self.dac_widx + 1) % 768;
            }
            0x3c7 => self.dac_ridx = (v as usize) * 3,
            0x70 => self.cmos_idx = v & 0x7f,
            0x40 => {
                // PIT channel 0 divisor (two writes, lo then hi)
                self.pit_divisor = (self.pit_divisor >> 8 | ((v as u32) << 8)) & 0xffff;
                let d = if self.pit_divisor == 0 { 0x1_0000 } else { self.pit_divisor };
                // ~8 modelled instructions per PIT count (1.19 MHz vs ~8 MIPS -> ~7).
                self.steps_per_tick = (d as u64 * 7).max(2000);
            }
            0x3c4 => self.seq_idx = v,
            0x3c5 => {
                self.seq[(self.seq_idx & 7) as usize] = v;
                if let Some(vga) = self.m.vga.as_deref_mut() {
                    match self.seq_idx & 7 {
                        2 => vga.map_mask = v & 0x0f,
                        4 => vga.chain4 = v & 8 != 0,
                        _ => {}
                    }
                }
            }
            0x3ce => self.gc_idx = v,
            0x3cf => {
                self.gc[(self.gc_idx & 15) as usize] = v;
                if let Some(vga) = self.m.vga.as_deref_mut() {
                    if self.gc_idx & 15 == 4 {
                        vga.read_map = v & 3;
                    }
                }
                if self.gc_idx & 15 == 5 && v & 3 != 0 && self.trace_ints {
                    eprintln!("VGA write mode {} set (latches not modelled)", v & 3);
                }
            }
            0x3d4 => self.crtc_idx = v,
            0x3d5 => self.crtc[(self.crtc_idx & 31) as usize] = v,
            0x20 | 0x21 | 0xa0 | 0xa1 | 0x43 | 0x41 | 0x42 | 0x61 | 0x3c2 | 0x3c0 => {}
            _ => {
                if self.trace_ints {
                    eprintln!("out port {port:#x} = {value:#x}");
                }
            }
        }
    }

    // ---------------- native interrupt services ----------------

    fn log_int(&mut self, v: u8, ah: u8) {
        let n = self.int_log.entry((v, ah)).or_insert(0);
        *n += 1;
        if self.trace_ints && *n == 1 {
            eprintln!(
                "int {v:02x} AH={ah:02x} (first; ax={:04x} dx={:04x} ds={:04x})",
                self.m.regs.ax(),
                self.m.regs.dx(),
                self.m.regs.ds
            );
        }
    }

    fn native_int(&mut self, v: u8) -> Result<(), String> {
        let ah = self.m.regs.ah();
        self.log_int(v, ah);
        match v {
            0x08 => {
                self.ticks = self.ticks.wrapping_add(1);
                self.m.write32(0x40, 0x6c, self.ticks);
                self.cpu.emulate_iret(&mut self.m);
                // BIOS chains int 1Ch; only bother when the game hooked it.
                let t = (self.m.read16(0, 0x1c * 4), self.m.read16(0, 0x1c * 4 + 2));
                if t != (0x70, STUB_SEG) {
                    self.cpu.deliver_int(&mut self.m, 0x1c);
                }
                Ok(())
            }
            0x09 | 0x0b | 0x0c | 0x0d | 0x0e | 0x0f | 0x70 | 0x1c | 0x23 | 0x24 => {
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x10 => self.int10(),
            0x11 => {
                self.m.regs.set_ax(0x0021);
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x12 => {
                self.m.regs.set_ax(640);
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x15 => {
                // extended services: report nothing present
                self.cpu.patch_frame_cf(&mut self.m, true);
                self.m.regs.set_ah(0x86);
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x16 => self.int16(),
            0x1a => {
                match ah {
                    0 => {
                        let t = self.ticks;
                        self.m.regs.set_cx((t >> 16) as u16);
                        self.m.regs.set_dx(t as u16);
                        self.m.regs.set_al(0);
                    }
                    2 => {
                        // RTC time in BCD: fixed 12:34:27 (deterministic)
                        self.m.regs.set_ch(0x12);
                        self.m.regs.set_cl(0x34);
                        self.m.regs.set_dh(0x27);
                        self.m.regs.set_dl(0);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    _ => {}
                }
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x21 => self.int21(),
            0x2f => {
                match self.m.regs.ax() {
                    0x1500 => {
                        // MSCDEX: present, 1 drive, first = D (index 3)
                        self.m.regs.set_bx(1);
                        self.m.regs.set_cx(3);
                    }
                    0x150b => {
                        // drive check: DX drive supported?
                        self.m.regs.set_ax(0x15ff);
                        self.m.regs.set_bx(0xadad);
                    }
                    0x4300 => self.m.regs.set_al(0), // no XMS (game runs with EMS)
                    _ => self.m.regs.set_al(0),
                }
                self.cpu.emulate_iret(&mut self.m);
                Ok(())
            }
            0x33 => self.int33(),
            0x67 => self.int67(),
            _ => Err(format!(
                "unhandled int {v:02x} AH={ah:02x} AX={:04x} at return {:04x}:{:04x}",
                self.m.regs.ax(),
                self.m.read16(self.m.regs.ss, self.m.regs.sp() as u32 + 2),
                self.m.read16(self.m.regs.ss, self.m.regs.sp() as u32),
            )),
        }
    }

    fn int10(&mut self) -> Result<(), String> {
        let ah = self.m.regs.ah();
        match ah {
            0x00 => {
                let mode = self.m.regs.al() & 0x7f;
                self.vga_mode = mode;
                self.m.write8(0x40, 0x49, mode);
                if mode == 0x13 {
                    if let Some(vga) = self.m.vga.as_deref_mut() {
                        vga.planes.fill(0);
                        vga.chain4 = true;
                        vga.map_mask = 0x0f;
                    }
                    // BIOS loads the default VGA palette on mode set; the game overwrites it
                    // via the DAC, but a sane grey ramp beats black-on-black for early frames.
                    for i in 0..256 {
                        let g = (i as u8) >> 2;
                        self.dac[i * 3] = g;
                        self.dac[i * 3 + 1] = g;
                        self.dac[i * 3 + 2] = g;
                    }
                } else {
                    self.m.mem[0xb8000..0xc0000].fill(0);
                }
            }
            0x01 | 0x02 | 0x03 | 0x05 | 0x06 | 0x07 => {}
            0x08 => self.m.regs.set_ax(0x0720),
            0x0e => {
                let c = self.m.regs.al() as char;
                self.console.push(c);
            }
            0x0f => {
                let mode = self.vga_mode;
                self.m.regs.set_al(mode);
                self.m.regs.set_ah(80);
                self.m.regs.set_bh(0);
            }
            0x10 => match self.m.regs.al() {
                0x10 => {
                    let i = (self.m.regs.bx() as usize % 256) * 3;
                    self.dac[i] = self.m.regs.dh() & 0x3f;
                    self.dac[i + 1] = self.m.regs.ch() & 0x3f;
                    self.dac[i + 2] = self.m.regs.cl() & 0x3f;
                }
                0x12 => {
                    let start = self.m.regs.bx() as usize;
                    let count = self.m.regs.cx() as usize;
                    let es = self.m.regs.es;
                    let dx = self.m.regs.dx() as u32;
                    for i in 0..count * 3 {
                        let v = self.m.read8(es, dx + i as u32) & 0x3f;
                        self.dac[(start * 3 + i) % 768] = v;
                    }
                }
                _ => {}
            },
            0x11 | 0x12 => {}
            0x1a => {
                self.m.regs.set_al(0x1a);
                self.m.regs.set_bl(8); // VGA color
            }
            _ => {}
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    fn int16(&mut self) -> Result<(), String> {
        let ah = self.m.regs.ah();
        match ah {
            0x00 | 0x10 => {
                if let Some((sc, asc)) = self.bios_keys.pop_front() {
                    self.m.regs.set_ah(sc);
                    self.m.regs.set_al(asc);
                } else {
                    // Blocking read with nothing buffered: rewind the frame's return IP onto the
                    // `int 16h` itself so it retries — timer IRQs keep firing in between, which
                    // is exactly the semantics of a BIOS blocking key wait.
                    let sp = self.m.regs.sp() as u32;
                    let ret_ip = self.m.read16(self.m.regs.ss, sp);
                    self.m.write16(self.m.regs.ss, sp, ret_ip.wrapping_sub(2));
                }
            }
            0x01 | 0x11 => {
                if let Some((sc, asc)) = self.bios_keys.front() {
                    self.m.regs.set_ah(*sc);
                    self.m.regs.set_al(*asc);
                    self.cpu.patch_frame_zf(&mut self.m, false);
                } else {
                    self.cpu.patch_frame_zf(&mut self.m, true);
                }
            }
            0x02 => self.m.regs.set_al(0),
            0x03 | 0x05 => {}
            _ => {}
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    fn int33(&mut self) -> Result<(), String> {
        match self.m.regs.ax() {
            0x0000 => {
                self.m.regs.set_ax(0xffff); // mouse present
                self.m.regs.set_bx(2);
            }
            0x0003 => {
                self.m.regs.set_bx(0);
                self.m.regs.set_cx(320);
                self.m.regs.set_dx(100);
            }
            0x000b => {
                self.m.regs.set_cx(0);
                self.m.regs.set_dx(0);
            }
            _ => {} // show/hide/ranges/handlers: accept silently (M3 wires real input)
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    fn int67(&mut self) -> Result<(), String> {
        let ah = self.m.regs.ah();
        match ah {
            0x40 => self.m.regs.set_ah(0), // status OK
            0x41 => {
                self.m.regs.set_bx(EMS_FRAME_SEG);
                self.m.regs.set_ah(0);
            }
            0x42 => {
                let used: u32 = self.ems_next_page;
                self.m.regs.set_bx((EMS_MAX_PAGES as u32 - used) as u16);
                self.m.regs.set_dx(EMS_MAX_PAGES as u16);
                self.m.regs.set_ah(0);
            }
            0x43 => {
                let want = self.m.regs.bx() as u32;
                if self.ems_next_page + want > EMS_MAX_PAGES as u32 {
                    self.m.regs.set_ah(0x88); // not enough pages
                } else {
                    let pages: Vec<u32> =
                        (self.ems_next_page..self.ems_next_page + want).collect();
                    self.ems_next_page += want;
                    self.ems_handles.push(Some(pages));
                    self.m.regs.set_dx((self.ems_handles.len() - 1) as u16);
                    self.m.regs.set_ah(0);
                }
            }
            0x44 => {
                let phys = self.m.regs.al() as usize;
                let logical = self.m.regs.bx();
                let handle = self.m.regs.dx() as usize;
                if phys >= 4 {
                    self.m.regs.set_ah(0x8b);
                } else if logical == 0xffff {
                    self.ems_unmap(phys);
                    self.m.regs.set_ah(0);
                } else {
                    match self.ems_handles.get(handle).and_then(|h| h.as_ref()) {
                        Some(pages) => match pages.get(logical as usize) {
                            Some(&store_page) => {
                                self.ems_unmap(phys);
                                let src = EMS_STORE + store_page as usize * EMS_PAGE;
                                let dst = (EMS_FRAME_SEG as usize) * 16 + phys * EMS_PAGE;
                                self.m.mem.copy_within(src..src + EMS_PAGE, dst);
                                self.ems_map[phys] = Some(store_page);
                                self.m.regs.set_ah(0);
                            }
                            None => self.m.regs.set_ah(0x8a),
                        },
                        None => self.m.regs.set_ah(0x83),
                    }
                }
            }
            0x45 => {
                let handle = self.m.regs.dx() as usize;
                if handle < self.ems_handles.len() {
                    self.ems_handles[handle] = None; // pages leak in the bump store: fine for one run
                }
                self.m.regs.set_ah(0);
            }
            0x46 => {
                self.m.regs.set_al(0x40); // EMS 4.0
                self.m.regs.set_ah(0);
            }
            0x47 | 0x48 => self.m.regs.set_ah(0), // save/restore page map: mappings persist
            0x4b => {
                self.m.regs.set_bx(self.ems_handles.iter().flatten().count() as u16);
                self.m.regs.set_ah(0);
            }
            0x4c => {
                let handle = self.m.regs.dx() as usize;
                match self.ems_handles.get(handle).and_then(|h| h.as_ref()) {
                    Some(pages) => {
                        self.m.regs.set_bx(pages.len() as u16);
                        self.m.regs.set_ah(0);
                    }
                    None => self.m.regs.set_ah(0x83),
                }
            }
            _ => {
                return Err(format!("EMS int 67 AH={ah:02x} not implemented"));
            }
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    /// Write a mapped physical EMS page back to its logical store before remapping.
    fn ems_unmap(&mut self, phys: usize) {
        if let Some(store_page) = self.ems_map[phys].take() {
            let dst = EMS_STORE + store_page as usize * EMS_PAGE;
            let src = (EMS_FRAME_SEG as usize) * 16 + phys * EMS_PAGE;
            self.m.mem.copy_within(src..src + EMS_PAGE, dst);
        }
    }

    fn int21(&mut self) -> Result<(), String> {
        let ah = self.m.regs.ah();
        match ah {
            0x02 => {
                let c = self.m.regs.dl() as char;
                self.console.push(c);
            }
            0x06 => {
                let d = self.m.regs.dl();
                if d != 0xff {
                    self.console.push(d as char);
                } else {
                    self.m.regs.set_al(0);
                    self.cpu.patch_frame_zf(&mut self.m, true);
                }
            }
            0x09 => {
                let (ds, dx) = (self.m.regs.ds, self.m.regs.dx());
                let mut s = String::new();
                for i in 0..512u32 {
                    let b = self.m.read8(ds, dx as u32 + i);
                    if b == b'$' {
                        break;
                    }
                    s.push(b as char);
                }
                self.console.push_str(&s);
            }
            0x0e => self.m.regs.set_al(26), // set drive (DL) -> number of drives
            0x19 => self.m.regs.set_al(self.cur_drive),
            0x1a => self.dta = (self.m.regs.ds, self.m.regs.dx()),
            0x25 => {
                let (v, ds, dx) = (self.m.regs.al() as u32, self.m.regs.ds, self.m.regs.dx());
                self.m.write16(0, v * 4, dx);
                self.m.write16(0, v * 4 + 2, ds);
            }
            0x2a => {
                // fixed date: 1995-07-20, a Thursday
                self.m.regs.set_cx(1995);
                self.m.regs.set_dh(7);
                self.m.regs.set_dl(20);
                self.m.regs.set_al(4);
            }
            0x2c => {
                let t = self.ticks as u64 * 10 / 182; // seconds*10 from ticks
                self.m.regs.set_ch(((t / 36000) % 24) as u8);
                self.m.regs.set_cl(((t / 600) % 60) as u8);
                self.m.regs.set_dh(((t / 10) % 60) as u8);
                self.m.regs.set_dl((t % 10) as u8 * 10);
            }
            0x2f => {
                let (s, o) = self.dta;
                self.m.regs.es = s;
                self.m.regs.set_bx(o);
            }
            0x30 => {
                self.m.regs.set_al(5);
                self.m.regs.set_ah(0);
                self.m.regs.set_bx(0);
                self.m.regs.set_cx(0);
            }
            0x33 => match self.m.regs.al() {
                0 => self.m.regs.set_dl(0),
                _ => {}
            },
            0x34 => {
                // InDOS flag pointer
                self.m.regs.es = 0x50;
                self.m.regs.set_bx(0);
            }
            0x35 => {
                let v = self.m.regs.al() as u32;
                let off = self.m.read16(0, v * 4);
                let seg = self.m.read16(0, v * 4 + 2);
                self.m.regs.es = seg;
                self.m.regs.set_bx(off);
            }
            0x36 => {
                // free disk space: plenty
                self.m.regs.set_ax(64); // sectors per cluster
                self.m.regs.set_bx(0x4000);
                self.m.regs.set_cx(512);
                self.m.regs.set_dx(0x8000);
            }
            0x38 => {
                // country info: fill a plausible US table
                let (ds, dx) = (self.m.regs.ds, self.m.regs.dx() as u32);
                for i in 0..34 {
                    self.m.write8(ds, dx + i, 0);
                }
                self.m.write8(ds, dx + 2, b'$');
                self.m.regs.set_ax(1);
                self.cpu.patch_frame_cf(&mut self.m, false);
            }
            0x4e => {
                // FindFirst: pattern in DS:DX, attrs in CX; results via the DTA
                let pattern = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                let want_dirs = self.m.regs.cx() & 0x10 != 0;
                let (dir_part, file_part) = match pattern.rfind(['\\', ':']) {
                    Some(i) => (pattern[..=i].to_string(), pattern[i + 1..].to_string()),
                    None => (String::new(), pattern.clone()),
                };
                let dir = if dir_part.is_empty() {
                    self.resolve(".", false)
                } else {
                    self.resolve(&dir_part, false)
                };
                let mut matches: Vec<(String, u32, u8)> = vec![];
                if let Ok(d) = dir {
                    if let Ok(rd) = std::fs::read_dir(&d) {
                        for e in rd.filter_map(|e| e.ok()) {
                            let name = e.file_name().to_string_lossy().to_uppercase();
                            let md = match e.metadata() {
                                Ok(md) => md,
                                Err(_) => continue,
                            };
                            if md.is_dir() && !want_dirs {
                                continue;
                            }
                            if dos_wildcard_match(&file_part.to_uppercase(), &name) {
                                let attr = if md.is_dir() { 0x10 } else { 0x20 };
                                matches.push((name, md.len() as u32, attr));
                            }
                        }
                    }
                }
                matches.sort();
                self.searches.insert(self.dta, (matches, 0));
                self.find_next_into_dta();
            }
            0x4f => self.find_next_into_dta(),
            0x39 | 0x3a => {
                // mkdir / rmdir
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                match self.resolve(&path, ah == 0x39) {
                    Ok(p) => {
                        let res = if ah == 0x39 {
                            std::fs::create_dir(&p)
                        } else {
                            std::fs::remove_dir(&p)
                        };
                        match res {
                            Ok(_) => self.cpu.patch_frame_cf(&mut self.m, false),
                            Err(_) => {
                                self.m.regs.set_ax(5);
                                self.cpu.patch_frame_cf(&mut self.m, true);
                            }
                        }
                    }
                    Err(e) => {
                        self.m.regs.set_ax(e);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x3b => {
                // chdir
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                match self.resolve(&path, false) {
                    Ok(p) if p.is_dir() => {
                        let mut drive = self.cur_drive;
                        let b = path.as_bytes();
                        let rest = if b.len() >= 2 && b[1] == b':' {
                            drive = (b[0] as char).to_ascii_uppercase() as u8 - b'A';
                            &path[2..]
                        } else {
                            &path[..]
                        };
                        self.cwd[drive as usize] =
                            rest.trim_start_matches('\\').trim_end_matches('\\').to_uppercase();
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    _ => {
                        self.m.regs.set_ax(3);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x3c => {
                // create/truncate
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                match self.resolve(&path, true) {
                    Ok(p) => match std::fs::File::create(&p) {
                        Ok(f) => {
                            let h = self.alloc_handle(HostFile { f });
                            self.m.regs.set_ax(h);
                            self.cpu.patch_frame_cf(&mut self.m, false);
                        }
                        Err(_) => {
                            self.m.regs.set_ax(5);
                            self.cpu.patch_frame_cf(&mut self.m, true);
                        }
                    },
                    Err(e) => {
                        self.m.regs.set_ax(e);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x3d => {
                // open
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                let write = self.m.regs.al() & 3 != 0;
                match self.resolve(&path, false) {
                    Ok(p) => {
                        let res = std::fs::OpenOptions::new()
                            .read(true)
                            .write(write)
                            .open(&p);
                        match res {
                            Ok(f) => {
                                let h = self.alloc_handle(HostFile { f });
                                self.m.regs.set_ax(h);
                                self.cpu.patch_frame_cf(&mut self.m, false);
                            }
                            Err(_) => {
                                self.m.regs.set_ax(2);
                                self.cpu.patch_frame_cf(&mut self.m, true);
                            }
                        }
                    }
                    Err(e) => {
                        self.m.regs.set_ax(e);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x3e => {
                let h = self.m.regs.bx() as usize;
                if h >= 5 && h < self.files.len() {
                    self.files[h] = None;
                }
                self.cpu.patch_frame_cf(&mut self.m, false);
            }
            0x3f => {
                // read
                let h = self.m.regs.bx() as usize;
                let count = self.m.regs.cx() as usize;
                let (ds, dx) = (self.m.regs.ds, self.m.regs.dx());
                match self.files.get_mut(h).and_then(|f| f.as_mut()) {
                    Some(hf) => {
                        let mut buf = vec![0u8; count];
                        let n = hf.f.read(&mut buf).unwrap_or(0);
                        let base = Machine::lin(ds, dx as u32);
                        self.m.mem[base..base + n].copy_from_slice(&buf[..n]);
                        self.m.regs.set_ax(n as u16);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    None => {
                        if h < 5 {
                            self.m.regs.set_ax(0); // stdio read: EOF
                            self.cpu.patch_frame_cf(&mut self.m, false);
                        } else {
                            self.m.regs.set_ax(6);
                            self.cpu.patch_frame_cf(&mut self.m, true);
                        }
                    }
                }
            }
            0x40 => {
                // write
                let h = self.m.regs.bx() as usize;
                let count = self.m.regs.cx() as usize;
                let (ds, dx) = (self.m.regs.ds, self.m.regs.dx());
                if h < 5 {
                    let mut s = String::new();
                    for i in 0..count {
                        s.push(self.m.read8(ds, dx as u32 + i as u32) as char);
                    }
                    self.console.push_str(&s);
                    self.m.regs.set_ax(count as u16);
                    self.cpu.patch_frame_cf(&mut self.m, false);
                } else {
                    match self.files.get_mut(h).and_then(|f| f.as_mut()) {
                        Some(hf) => {
                            let base = Machine::lin(ds, dx as u32);
                            let n = hf.f.write(&self.m.mem[base..base + count]).unwrap_or(0);
                            self.m.regs.set_ax(n as u16);
                            self.cpu.patch_frame_cf(&mut self.m, false);
                        }
                        None => {
                            self.m.regs.set_ax(6);
                            self.cpu.patch_frame_cf(&mut self.m, true);
                        }
                    }
                }
            }
            0x41 => {
                // unlink
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                match self.resolve(&path, false) {
                    Ok(p) => {
                        let _ = std::fs::remove_file(p);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    Err(e) => {
                        self.m.regs.set_ax(e);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x42 => {
                // lseek
                let h = self.m.regs.bx() as usize;
                let off = ((self.m.regs.cx() as u64) << 16 | self.m.regs.dx() as u64) as i64;
                let whence = self.m.regs.al();
                match self.files.get_mut(h).and_then(|f| f.as_mut()) {
                    Some(hf) => {
                        let pos = match whence {
                            0 => hf.f.seek(SeekFrom::Start(off as u64)),
                            1 => hf.f.seek(SeekFrom::Current(off)),
                            _ => hf.f.seek(SeekFrom::End(off)),
                        }
                        .unwrap_or(0);
                        self.m.regs.set_dx((pos >> 16) as u16);
                        self.m.regs.set_ax(pos as u16);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    None => {
                        self.m.regs.set_ax(6);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x43 => {
                // attributes
                let path = self.read_asciiz(self.m.regs.ds, self.m.regs.dx());
                match self.resolve(&path, false) {
                    Ok(p) => {
                        let attr = if p.is_dir() { 0x10 } else { 0x20 };
                        self.m.regs.set_cx(attr);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    Err(e) => {
                        self.m.regs.set_ax(e);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x44 => {
                // ioctl
                let h = self.m.regs.bx() as usize;
                match self.m.regs.al() {
                    0 => {
                        let dx = if h < 5 { 0x80d3 } else { 0x0002 };
                        self.m.regs.set_dx(dx);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    1 => self.cpu.patch_frame_cf(&mut self.m, false),
                    7 => {
                        self.m.regs.set_al(0xff);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    8 => {
                        // removable? C fixed (1), others say fixed too
                        self.m.regs.set_ax(1);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    0x0e => {
                        self.m.regs.set_al(0);
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                    _ => {
                        self.m.regs.set_ax(1);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    }
                }
            }
            0x47 => {
                // getcwd of DL (0=default)
                let d = if self.m.regs.dl() == 0 { self.cur_drive } else { self.m.regs.dl() - 1 };
                let cwd = self.cwd[d as usize].clone();
                let (ds, si) = (self.m.regs.ds, self.m.regs.si() as u32);
                for (i, b) in cwd.bytes().enumerate() {
                    self.m.write8(ds, si + i as u32, b);
                }
                self.m.write8(ds, si + cwd.len() as u32, 0);
                self.cpu.patch_frame_cf(&mut self.m, false);
            }
            0x48 => {
                let want = self.m.regs.bx();
                let free = MEM_TOP_SEG - self.alloc_next;
                if want > free {
                    self.m.regs.set_ax(8);
                    self.m.regs.set_bx(free);
                    self.cpu.patch_frame_cf(&mut self.m, true);
                } else {
                    self.m.regs.set_ax(self.alloc_next);
                    self.alloc_next += want;
                    self.cpu.patch_frame_cf(&mut self.m, false);
                }
            }
            0x49 => self.cpu.patch_frame_cf(&mut self.m, false), // free: bump store, no reuse
            0x4a => {
                // resize block ES to BX paras; the program block resize frees the arena above
                let es = self.m.regs.es;
                let want = self.m.regs.bx();
                if es == PSP_SEG {
                    let end = PSP_SEG.saturating_add(want);
                    if end > MEM_TOP_SEG {
                        self.m.regs.set_ax(8);
                        self.m.regs.set_bx(MEM_TOP_SEG - PSP_SEG);
                        self.cpu.patch_frame_cf(&mut self.m, true);
                    } else {
                        self.prog_end = end;
                        self.alloc_next = end;
                        self.cpu.patch_frame_cf(&mut self.m, false);
                    }
                } else {
                    self.cpu.patch_frame_cf(&mut self.m, false);
                }
            }
            0x4c => {
                self.exit_code = Some(self.m.regs.al());
            }
            0x50 => {} // set PSP: ignore
            0x51 | 0x62 => self.m.regs.set_bx(PSP_SEG),
            0x58 => {
                self.m.regs.set_al(0);
                self.cpu.patch_frame_cf(&mut self.m, false);
            }
            _ => {
                return Err(format!(
                    "int 21 AH={ah:02x} AX={:04x} DX={:04x} DS={:04x} not implemented",
                    self.m.regs.ax(),
                    self.m.regs.dx(),
                    self.m.regs.ds
                ));
            }
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    /// Emit the next FindFirst/FindNext match into the DTA, or error 18 (no more files).
    fn find_next_into_dta(&mut self) {
        let dta = self.dta;
        let next = self.searches.get_mut(&dta).and_then(|(list, pos)| {
            let item = list.get(*pos).cloned();
            *pos += 1;
            item
        });
        match next {
            Some((name, size, attr)) => {
                let (s, o) = dta;
                self.m.write8(s, o as u32 + 0x15, attr);
                self.m.write16(s, o as u32 + 0x16, 0x6000); // 12:00:00
                self.m.write16(s, o as u32 + 0x18, 0x1ef4); // 1995-07-20
                self.m.write32(s, o as u32 + 0x1a, size);
                for (i, b) in name.bytes().chain(std::iter::once(0)).enumerate().take(13) {
                    self.m.write8(s, o as u32 + 0x1e + i as u32, b);
                }
                self.cpu.patch_frame_cf(&mut self.m, false);
            }
            None => {
                self.m.regs.set_ax(18);
                self.cpu.patch_frame_cf(&mut self.m, true);
            }
        }
    }

    // ---------------- introspection ----------------

    /// 320x200 RGB screenshot through the DAC (6-bit scaled like the decoders). Composites the
    /// VGA planes exactly as the CRT would: chain-4 (stock 13h) or unchained Mode-X addressing
    /// with the CRTC start address + row offset (page flipping honoured).
    pub fn screenshot_rgb(&self) -> Vec<u8> {
        let vga = self.m.vga.as_deref().expect("runtime always has vga");
        let start = ((self.crtc[0x0c] as usize) << 8) | self.crtc[0x0d] as usize;
        let stride = {
            let s = self.crtc[0x13] as usize * 2;
            if s == 0 { 80 } else { s }
        };
        let mut out = Vec::with_capacity(320 * 200 * 3);
        for y in 0..200 {
            for x in 0..320 {
                let px = if vga.chain4 {
                    let i = y * 320 + x;
                    vga.planes[(i & 3) * 0x10000 + (i >> 2)] as usize
                } else {
                    let cell = (start + y * stride + (x >> 2)) & 0xffff;
                    vga.planes[(x & 3) * 0x10000 + cell] as usize
                };
                for c in 0..3 {
                    let v = self.dac[px * 3 + c];
                    out.push((v << 2) | (v >> 4));
                }
            }
        }
        out
    }

    pub fn write_ppm(&self, path: &Path) -> std::io::Result<()> {
        let mut data = b"P6\n320 200\n255\n".to_vec();
        data.extend_from_slice(&self.screenshot_rgb());
        std::fs::write(path, data)
    }

    /// Text-mode screen contents (mode 3) for early-boot error messages.
    pub fn text_screen(&self) -> String {
        let mut s = String::new();
        for row in 0..25 {
            let mut line = String::new();
            for col in 0..80 {
                let ch = self.m.mem[0xb8000 + (row * 80 + col) * 2];
                line.push(if (0x20..0x7f).contains(&ch) { ch as char } else { ' ' });
            }
            let t = line.trim_end();
            if !t.is_empty() {
                s.push_str(t);
                s.push('\n');
            }
        }
        s
    }

    pub fn console_output(&self) -> &str {
        &self.console
    }

    /// Push a key event (scancode, ascii) — M3 wires real input to this.
    pub fn key_event(&mut self, scancode: u8, ascii: u8) {
        self.kbd_queue.push_back((scancode, ascii));
        self.bios_keys.push_back((scancode, ascii));
    }
}
