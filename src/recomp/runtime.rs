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

/// Modelled CPU speed: interpreter steps per emulated second. Wall-clock pacing and the PIT
/// divisor -> IRQ0 cadence both derive from this (1 step ~= 1 instruction on a ~8 MIPS 386/33).
pub const STEPS_PER_SECOND: u64 = 8_000_000;

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
    pit_lo: u8,
    pit_phase: u8,
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
    /// 8259 PIC interrupt masks (port 0x21 master, 0xA1 slave). A set bit masks that IRQ line.
    /// Default: IRQ0(timer)/IRQ1(kbd)/IRQ2(cascade)/IRQ6 unmasked on the master (0xB8), slave all masked.
    pic_mask0: u8,
    pic_mask1: u8,
    /// Conventional-memory accounting: the program block ends here after int 21h/4Ah; 48h
    /// allocations bump from `alloc_next` toward MEM_TOP_SEG.
    prog_end: u16,
    alloc_next: u16,
    kbd_queue: VecDeque<(u8, u8)>, // (scancode, ascii) pending hardware events
    bios_keys: VecDeque<(u8, u8)>, // decoded buffer served by int 16h
    kbd_irq_pending: u32,          // int 9 deliveries owed (when the game hooked int 9)
    // int 33h mouse state. Coordinates are DOS-virtual: x 0..639 (2px granularity in mode 13h),
    // y 0..199.
    mouse_x: u16,
    mouse_y: u16,
    mouse_buttons: u16,
    mouse_presses: [(u16, u16, u16); 2],  // per button: count, last x, last y
    mouse_releases: [(u16, u16, u16); 2],
    mouse_handler: Option<(u16, u16, u16)>, // event mask, seg, off
    mouse_pending: u16,                     // accumulated event mask awaiting delivery
    mouse_saved: Option<(super::machine::Regs, u16, u16, bool, u16)>, // ctx during callback
    mouse_shown: i16,
    /// FindFirst/FindNext state per DTA address: (matches, next index).
    searches: HashMap<(u16, u16), (Vec<(String, u32, u8)>, usize)>,
    /// (vector, AH) -> count, for the boot log.
    pub int_log: HashMap<(u8, u8), u64>,
    /// Every file the game opens, with the step it happened — identifies which assets a
    /// given screen/scene loads (drive the emulator there, then inspect this).
    pub opened_files: Vec<(u64, String)>,
    pub trace_ints: bool,
    pub ip_sample: Option<std::collections::HashMap<(u16,u16), u64>>,
    pub force_sub: bool,
    pub trace_glyph: bool,
    pub glyph_log: Vec<(u8, u32, u8)>, // (plane_mask, offset, value) for subtitle-row writes
    console: String,
    exit_code: Option<u8>,
    /// exit-path counters for performance triage: [in, out, int, hlt, chunks]
    pub exit_counts: [u64; 5],
    // 8237 DMA (channels 0-3): [base_addr, base_count, cur_addr, cur_count] + page, per channel.
    dma_flipflop: bool,
    dma_addr: [u16; 4],
    dma_count: [u16; 4],
    dma_cur_count: [u16; 4],
    dma_page: [u8; 4],
    dma_mode: [u8; 4],
    dma_tc: u8, // terminal-count status bits
    // SoundBlaster DSP at base 0x220 (game config S162227: SB16, IRQ 2, DMA 1).
    sb_reset_state: u8,
    sb_out: VecDeque<u8>,      // bytes readable at 0x22A
    sb_cmd: Option<(u8, Vec<u8>, usize)>, // in-progress command: (cmd, args, needed)
    sb_time_constant: u8,
    sb_rate_hz: u32,
    /// Active playback: (dma channel, start step, total samples, auto_init, samples_done)
    sb_play: Option<(usize, u64, u32, bool)>,
    /// Captured PCM (what the game streamed) — the M4 audio-out tap.
    pub sb_pcm: Vec<u8>,
    pub sb_pcm_rate: u32,
    sb_irq_pending: bool,
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
            pit_lo: 0,
            pit_phase: 0,
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
            pic_mask0: 0xb8,
            pic_mask1: 0xff,
            kbd_queue: VecDeque::new(),
            bios_keys: VecDeque::new(),
            kbd_irq_pending: 0,
            mouse_x: 320,
            mouse_y: 100,
            mouse_buttons: 0,
            mouse_presses: [(0, 0, 0); 2],
            mouse_releases: [(0, 0, 0); 2],
            mouse_handler: None,
            mouse_pending: 0,
            mouse_saved: None,
            mouse_shown: -1,
            searches: HashMap::new(),
            int_log: HashMap::new(),
            opened_files: Vec::new(),
            trace_ints: false,
            ip_sample: None,
            force_sub: false, // persistence experiment; OFF — it made a spurious attract subtitle persistent (DOSBox shows none)
            trace_glyph: false,
            glyph_log: Vec::new(),
            console: String::new(),
            exit_code: None,
            exit_counts: [0; 5],
            dma_flipflop: false,
            dma_addr: [0; 4],
            dma_count: [0; 4],
            dma_cur_count: [0; 4],
            dma_page: [0; 4],
            dma_mode: [0; 4],
            dma_tc: 0,
            sb_reset_state: 0,
            sb_out: VecDeque::new(),
            sb_cmd: None,
            sb_time_constant: 0xa6, // ~11 kHz default
            sb_rate_hz: 11111,
            sb_play: None,
            sb_pcm: Vec::new(),
            sb_pcm_rate: 11111,
            sb_irq_pending: false,
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
        // Mouse-callback return trampoline (far-called by the guest handler's retf path).
        self.m.write8(STUB_SEG, 0x420, 0xf4);
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

    /// Inject a keystroke: buffered for `int 16h` (BIOS read) and queued as a hardware
    /// scancode for the `int 9` IRQ path (when the game hooked it). Drives the game's
    /// menus/prompts from a headless driver.
    pub fn inject_key(&mut self, scancode: u8, ascii: u8) {
        self.bios_keys.push_back((scancode, ascii));
        self.kbd_queue.push_back((scancode, ascii));
        self.kbd_irq_pending += 1;
    }

    /// Move the virtual mouse (DOS-virtual coords: x 0..639, y 0..199 — screen column
    /// `sx` is `sx*2`). Flags a move event for the game's mouse callback if registered.
    pub fn set_mouse_pos(&mut self, x: u16, y: u16) {
        self.mouse_x = x;
        self.mouse_y = y;
        if let Some((mask, _, _)) = self.mouse_handler {
            self.mouse_pending |= mask & 0x01;
        }
    }

    /// Press a mouse button (0 = left, 1 = right) at the current position: bumps the
    /// int33 ax=5 press counter and flags the press event.
    pub fn mouse_press(&mut self, button: u16) {
        let b = (button as usize).min(1);
        self.mouse_presses[b].0 = self.mouse_presses[b].0.wrapping_add(1);
        self.mouse_presses[b] = (self.mouse_presses[b].0, self.mouse_x, self.mouse_y);
        self.mouse_buttons |= if button == 0 { 1 } else { 2 };
        self.mouse_pending |= if button == 0 { 0x02 } else { 0x08 };
    }

    /// Release a mouse button (0 = left, 1 = right): bumps the int33 ax=6 release
    /// counter and flags the release event.
    pub fn mouse_release(&mut self, button: u16) {
        let b = (button as usize).min(1);
        self.mouse_releases[b].0 = self.mouse_releases[b].0.wrapping_add(1);
        self.mouse_releases[b] = (self.mouse_releases[b].0, self.mouse_x, self.mouse_y);
        self.mouse_buttons &= !(if button == 0 { 1 } else { 2 });
        self.mouse_pending |= if button == 0 { 0x04 } else { 0x10 };
    }

    pub fn run(&mut self, max_steps: u64) -> RunEnd {
        loop {
            if let Some(c) = self.exit_code {
                return RunEnd::Exited(c);
            }
            if self.cpu.steps >= max_steps {
                return RunEnd::StepBudget;
            }
            // Subtitle persistence: the game's per-frame reveal draw (0x93f8, main loop 0x12bd)
            // only redraws the subtitle when its gate flag gs:[0x27e2]&2 (or 5e64/67bc) is set.
            // The one-shot present (0xbe29) sets 27e2=2 then clears it, so on a triple-buffered
            // display the glyphs (drawn once to one page) get overwritten when the scene re-blits
            // that page. Refresh the gate each frame WHILE a subtitle is active — the game's own
            // "subtitle active" flag gs:[0xba0]&1 (set at 0xbe11, cleared when the line ends) —
            // so the game's own reveal draw renders the glyphs on the current page every frame.
            if self.force_sub && self.m.read8(0x0e84, 0x0ba0) & 1 != 0 {
                self.m.write16(0x0e84, 0x27e2, 2);
            }
            // pending mouse events -> user callback (a real DOS mouse driver far-calls it)
            if self.mouse_pending != 0 {
                match self.mouse_handler {
                    Some((mask, seg, off))
                        if self.mouse_pending & mask != 0
                            && self.cpu.iflag
                            && self.mouse_saved.is_none() =>
                    {
                        let ev = self.mouse_pending & mask;
                        self.mouse_pending = 0;
                        self.mouse_saved = Some((
                            self.m.regs,
                            self.cpu.cs,
                            self.cpu.ip,
                            self.cpu.iflag,
                            self.cpu.flags_high,
                        ));
                        let r = &mut self.m.regs;
                        r.set_ax(ev);
                        r.set_bx(self.mouse_buttons);
                        r.set_cx(self.mouse_x);
                        r.set_dx(self.mouse_y);
                        r.set_si(0);
                        r.set_di(0);
                        // far return frame -> trampoline stub
                        r.set_sp(r.sp().wrapping_sub(2));
                        let (ss, sp) = (r.ss, r.sp() as u32);
                        self.m.write16(ss, sp, STUB_SEG);
                        let r = &mut self.m.regs;
                        r.set_sp(r.sp().wrapping_sub(2));
                        let (ss, sp) = (r.ss, r.sp() as u32);
                        self.m.write16(ss, sp, 0x0420);
                        self.cpu.cs = seg;
                        self.cpu.ip = off;
                        self.m.regs.cs = seg;
                    }
                    Some(_) | None => self.mouse_pending = 0,
                }
            }
            // int 9 keyboard IRQ (only when the game hooked it; int 16h polls work regardless)
            if self.kbd_irq_pending > 0 && self.cpu.iflag && self.ivt_hooked(9)
                && self.pic_mask0 & 0x02 == 0
            {
                self.kbd_irq_pending -= 1;
                self.cpu.deliver_int(&mut self.m, 9);
            }
            // SoundBlaster completion IRQ (driver config block: base 220, IRQ 7 -> vector 0x0F)
            self.tick_sb_playback();
            if self.sb_irq_pending && self.cpu.iflag && self.pic_mask0 & 0x80 == 0 {
                self.sb_irq_pending = false;
                if self.ivt_hooked(0x0f) {
                    self.cpu.deliver_int(&mut self.m, 0x0f);
                }
            }
            // timer IRQ0
            if self.cpu.steps >= self.next_tick_at {
                if self.cpu.iflag && self.pic_mask0 & 0x01 == 0 {
                    self.next_tick_at = self.cpu.steps + self.steps_per_tick;
                    self.cpu.deliver_int(&mut self.m, 8);
                } else {
                    self.next_tick_at = self.cpu.steps + 1000; // retry once IF is set
                }
            }
            if let Some(h) = self.ip_sample.as_mut() {
                *h.entry((self.cpu.cs, self.cpu.ip)).or_default() += 1;
            }
            let budget = (max_steps - self.cpu.steps).min(4096);
            self.exit_counts[4] += 1;
            match self.cpu.run(&mut self.m, budget) {
                Exit::StepLimit => {}
                Exit::Int { vector } => {
                    self.exit_counts[2] += 1;
                    self.cpu.deliver_int(&mut self.m, vector)
                }
                Exit::Hlt => {
                    self.exit_counts[3] += 1;
                    let (cs, ip) = (self.cpu.cs, self.cpu.ip);
                    if cs == STUB_SEG && ip == 0x421 {
                        // mouse-callback trampoline: restore the interrupted context
                        if let Some((regs, ccs, cip, ifl, fh)) = self.mouse_saved.take() {
                            self.m.regs = regs;
                            self.cpu.cs = ccs;
                            self.cpu.ip = cip;
                            self.cpu.iflag = ifl;
                            self.cpu.flags_high = fh;
                            continue;
                        }
                        return RunEnd::Fatal("trampoline hlt without saved context".into());
                    }
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
                    self.exit_counts[0] += 1;
                    // A word-sized IN reads port then port+1 (hardware bus behavior — VGA/DMA
                    // index+data pairs rely on it).
                    let val = match size {
                        1 => self.port_in(port, 1),
                        _ => {
                            let lo = self.port_in(port, 1) & 0xff;
                            let hi = self.port_in(port.wrapping_add(1), 1) & 0xff;
                            lo | (hi << 8)
                        }
                    };
                    match size {
                        1 => self.m.regs.set_al(val as u8),
                        2 => self.m.regs.set_ax(val as u16),
                        _ => self.m.regs.eax = val,
                    }
                }
                Exit::Out { port, size, value } => {
                    self.exit_counts[1] += 1;
                    // A word-sized OUT writes AL to port and AH to port+1 — the standard VGA
                    // index+data idiom (`out dx, ax` with AL=index, AH=data).
                    match size {
                        1 => self.port_out(port, 1, value),
                        _ => {
                            self.port_out(port, 1, value & 0xff);
                            self.port_out(port.wrapping_add(1), 1, (value >> 8) & 0xff);
                            if size == 4 {
                                self.port_out(port.wrapping_add(2), 1, (value >> 16) & 0xff);
                                self.port_out(port.wrapping_add(3), 1, (value >> 24) & 0xff);
                            }
                        }
                    }
                }
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

    /// Lockstep capture: fast-forward `skip` steps, then record a per-instruction trace of the
    /// next `window` pure-CPU instructions for offline interp-vs-Unicorn differential replay
    /// (`re/tools/lockstep.py`). VRAM is routed to linear `mem` (vga_linear) so both interpreters
    /// share identical memory semantics — the goal is finding a CONTROL-FLOW (branch) divergence
    /// in game logic, not pixel fidelity.
    ///
    /// Trace format (little-endian): header = b"LSTP", window:u32, mem_len:u32, initial regs (48B),
    /// mem (mem_len B). Then events: type:u8 (0=pure-CPU 'X', 1=device 'D'), after-regs (48B), and
    /// for 'D' only: nwrites:u32 then nwrites*(addr:u32, val:u8).
    /// Regs (48B): eax,ebx,ecx,edx,esi,edi,ebp,esp (8*u32); cs,ds,es,ss,fs,gs,ip,flags (8*u16).
    pub fn lockstep_capture(&mut self, skip: u64, window: u64, out: &Path) -> std::io::Result<()> {
        fn snap(m: &Machine, cpu: &Cpu) -> [u8; 48] {
            let r = &m.regs;
            let flags: u16 = 0x0002
                | (r.cf as u16)
                | ((r.pf as u16) << 2)
                | ((r.af as u16) << 4)
                | ((r.zf as u16) << 6)
                | ((r.sf as u16) << 7)
                | ((cpu.iflag as u16) << 9)
                | ((r.df as u16) << 10)
                | ((r.of as u16) << 11)
                | cpu.flags_high;
            let mut b = [0u8; 48];
            let mut o = 0;
            for v in [r.eax, r.ebx, r.ecx, r.edx, r.esi, r.edi, r.ebp, r.esp] {
                b[o..o + 4].copy_from_slice(&v.to_le_bytes());
                o += 4;
            }
            for v in [cpu.cs, r.ds, r.es, r.ss, r.fs, r.gs, cpu.ip, flags] {
                b[o..o + 2].copy_from_slice(&v.to_le_bytes());
                o += 2;
            }
            b
        }
        // Fast-forward past boot (normal run, planar VGA).
        while self.exit_code.is_none() && self.cpu.steps < skip {
            match self.run(skip) {
                RunEnd::StepBudget | RunEnd::Exited(_) => break,
                RunEnd::Fatal(e) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e))
                }
            }
        }
        self.m.vga_linear = true; // from here, VRAM is linear on both sides

        let f = std::fs::File::create(out)?;
        let mut w = std::io::BufWriter::new(f);
        w.write_all(b"LSTP")?;
        w.write_all(&(window as u32).to_le_bytes())?;
        w.write_all(&(self.m.mem.len() as u32).to_le_bytes())?;
        w.write_all(&snap(&self.m, &self.cpu))?;
        w.write_all(&self.m.mem)?;

        // Emit a 'D' (device) event: after-regs + the memory writes captured in wlog.
        macro_rules! emit_d {
            ($w:expr, $s:expr, $writes:expr) => {{
                $w.write_all(&[1u8])?;
                $w.write_all(&$s)?;
                $w.write_all(&($writes.len() as u32).to_le_bytes())?;
                for (a, v) in &$writes {
                    $w.write_all(&a.to_le_bytes())?;
                    $w.write_all(&[*v])?;
                }
            }};
        }

        let mut xcount: u64 = 0;
        while self.exit_code.is_none() && xcount < window {
            // ---- pre-instruction injections (mirror run(); each clears IF so ≤1 fires) ----
            let mut injected = false;
            // int 9 keyboard IRQ
            if !injected
                && self.kbd_irq_pending > 0
                && self.cpu.iflag
                && self.ivt_hooked(9)
                && self.pic_mask0 & 0x02 == 0
            {
                self.kbd_irq_pending -= 1;
                self.m.wlog = Some(Vec::new());
                self.cpu.deliver_int(&mut self.m, 9);
                let wr = self.m.wlog.take().unwrap();
                emit_d!(w, snap(&self.m, &self.cpu), wr);
                injected = true;
            }
            self.tick_sb_playback();
            if !injected && self.sb_irq_pending && self.cpu.iflag && self.pic_mask0 & 0x80 == 0 {
                self.sb_irq_pending = false;
                if self.ivt_hooked(0x0f) {
                    self.m.wlog = Some(Vec::new());
                    self.cpu.deliver_int(&mut self.m, 0x0f);
                    let wr = self.m.wlog.take().unwrap();
                    emit_d!(w, snap(&self.m, &self.cpu), wr);
                    injected = true;
                }
            }
            if !injected && self.cpu.steps >= self.next_tick_at {
                if self.cpu.iflag && self.pic_mask0 & 0x01 == 0 {
                    self.next_tick_at = self.cpu.steps + self.steps_per_tick;
                    self.m.wlog = Some(Vec::new());
                    self.cpu.deliver_int(&mut self.m, 8);
                    let wr = self.m.wlog.take().unwrap();
                    emit_d!(w, snap(&self.m, &self.cpu), wr);
                    injected = true;
                } else {
                    self.next_tick_at = self.cpu.steps + 1000;
                }
            }
            if injected {
                continue;
            }

            // ---- one instruction ----
            match self.cpu.run(&mut self.m, 1) {
                Exit::StepLimit => {
                    // pure-CPU 'X' — Unicorn re-executes this; only after-regs recorded.
                    w.write_all(&[0u8])?;
                    w.write_all(&snap(&self.m, &self.cpu))?;
                    xcount += 1;
                }
                Exit::Int { vector } => {
                    self.m.wlog = Some(Vec::new());
                    self.cpu.deliver_int(&mut self.m, vector);
                    let wr = self.m.wlog.take().unwrap();
                    emit_d!(w, snap(&self.m, &self.cpu), wr);
                }
                Exit::Hlt => {
                    let (cs, ip) = (self.cpu.cs, self.cpu.ip);
                    if cs == STUB_SEG && ip == 0x421 {
                        if let Some((regs, ccs, cip, ifl, fh)) = self.mouse_saved.take() {
                            self.m.regs = regs;
                            self.cpu.cs = ccs;
                            self.cpu.ip = cip;
                            self.cpu.iflag = ifl;
                            self.cpu.flags_high = fh;
                            emit_d!(w, snap(&self.m, &self.cpu), Vec::<(u32, u8)>::new());
                            continue;
                        }
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "trampoline hlt without saved context",
                        ));
                    }
                    let v = if cs == STUB_SEG && ip >= 1 {
                        ((ip - 1) / 4) as u8
                    } else if cs == EMS_STUB_SEG {
                        0x67
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("hlt outside stubs at {cs:04x}:{ip:04x}"),
                        ));
                    };
                    self.m.wlog = Some(Vec::new());
                    let r = self.native_int(v);
                    let wr = self.m.wlog.take().unwrap();
                    if let Err(e) = r {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
                    }
                    emit_d!(w, snap(&self.m, &self.cpu), wr);
                }
                Exit::In { port, size } => {
                    let val = match size {
                        1 => self.port_in(port, 1),
                        _ => {
                            let lo = self.port_in(port, 1) & 0xff;
                            let hi = self.port_in(port.wrapping_add(1), 1) & 0xff;
                            lo | (hi << 8)
                        }
                    };
                    match size {
                        1 => self.m.regs.set_al(val as u8),
                        2 => self.m.regs.set_ax(val as u16),
                        _ => self.m.regs.eax = val,
                    }
                    emit_d!(w, snap(&self.m, &self.cpu), Vec::<(u32, u8)>::new());
                }
                Exit::Out { port, size, value } => {
                    self.m.wlog = Some(Vec::new());
                    match size {
                        1 => self.port_out(port, 1, value),
                        _ => {
                            self.port_out(port, 1, value & 0xff);
                            self.port_out(port.wrapping_add(1), 1, (value >> 8) & 0xff);
                            if size == 4 {
                                self.port_out(port.wrapping_add(2), 1, (value >> 16) & 0xff);
                                self.port_out(port.wrapping_add(3), 1, (value >> 24) & 0xff);
                            }
                        }
                    }
                    let wr = self.m.wlog.take().unwrap();
                    emit_d!(w, snap(&self.m, &self.cpu), wr);
                }
                Exit::Unimplemented { cs, ip, byte, what } => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("unimplemented {what} (op {byte:#04x}) at {cs:04x}:{ip:04x}"),
                    ));
                }
                Exit::Ret | Exit::Retf => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "depth-0 ret in lockstep",
                    ))
                }
            }
        }
        w.flush()?;
        Ok(())
    }

    // ---------------- port I/O ----------------

    fn port_in(&mut self, port: u16, _size: u8) -> u32 {
        match port {
            // 8237 DMA: current address/count with lo/hi flip-flop
            0x00 | 0x02 | 0x04 | 0x06 => {
                let ch = (port >> 1) as usize;
                let v = self.dma_addr[ch]; // current address not modelled separately
                self.dma_flipflop = !self.dma_flipflop;
                if self.dma_flipflop { (v & 0xff) as u32 } else { (v >> 8) as u32 }
            }
            0x01 | 0x03 | 0x05 | 0x07 => {
                let ch = (port >> 1) as usize;
                self.tick_sb_playback();
                let v = self.dma_cur_count[ch];
                self.dma_flipflop = !self.dma_flipflop;
                if self.dma_flipflop { (v & 0xff) as u32 } else { (v >> 8) as u32 }
            }
            0x08 => {
                let v = self.dma_tc as u32;
                self.dma_tc = 0;
                v
            }
            // SoundBlaster DSP (base 0x220)
            0x22a => {
                let v = self.sb_out.pop_front().unwrap_or(0xff) as u32;
                if self.trace_ints {
                    eprintln!("SB in 22a -> {v:#x} @{:04x}:{:04x}", self.cpu.cs, self.cpu.ip);
                }
                v
            }
            0x22c => 0x7f, // write-buffer status: ready
            0x22e => {
                let v = if self.sb_out.is_empty() { 0x7f } else { 0xff };
                if self.trace_ints {
                    eprintln!("SB in 22e -> {v:#x} @{:04x}:{:04x}", self.cpu.cs, self.cpu.ip);
                }
                v
            }
            0x224 | 0x225 => {
                if self.trace_ints {
                    eprintln!("SB MIXER read port {port:#x} idx={:#x} @{:04x}:{:04x}", self.cmos_idx, self.cpu.cs, self.cpu.ip);
                }
                0
            } // mixer
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
            0x60 => self.kbd_queue.pop_front().map(|(s, _)| s as u32).unwrap_or(0),
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
            0x21 => self.pic_mask0 as u32,  // 8259 master IMR
            0xa1 => self.pic_mask1 as u32,  // 8259 slave IMR
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
            0x00 | 0x02 | 0x04 | 0x06 => {
                let ch = (port >> 1) as usize;
                self.dma_flipflop = !self.dma_flipflop;
                if self.dma_flipflop {
                    self.dma_addr[ch] = (self.dma_addr[ch] & 0xff00) | v as u16;
                } else {
                    self.dma_addr[ch] = (self.dma_addr[ch] & 0x00ff) | ((v as u16) << 8);
                }
            }
            0x01 | 0x03 | 0x05 | 0x07 => {
                let ch = (port >> 1) as usize;
                self.dma_flipflop = !self.dma_flipflop;
                if self.dma_flipflop {
                    self.dma_count[ch] = (self.dma_count[ch] & 0xff00) | v as u16;
                } else {
                    self.dma_count[ch] = (self.dma_count[ch] & 0x00ff) | ((v as u16) << 8);
                    self.dma_cur_count[ch] = self.dma_count[ch];
                    if self.trace_ints {
                        eprintln!(
                            "DMA ch{ch} count={:#06x} addr={:#06x} page={:#04x} @step {}",
                            self.dma_count[ch], self.dma_addr[ch], self.dma_page[ch], self.cpu.steps
                        );
                    }
                }
            }
            0x0a => {} // mask
            0x0b => {
                let ch = (v & 3) as usize;
                self.dma_mode[ch] = v;
            }
            0x0c => self.dma_flipflop = false,
            0x0d => {
                self.dma_flipflop = false; // master clear
            }
            0x87 => self.dma_page[0] = v,
            0x83 => self.dma_page[1] = v,
            0x81 => self.dma_page[2] = v,
            0x82 => self.dma_page[3] = v,
            0x226 => {
                // DSP reset: 1 then 0 -> respond 0xAA
                if v == 1 {
                    self.sb_reset_state = 1;
                } else if v == 0 && self.sb_reset_state == 1 {
                    self.sb_reset_state = 0;
                    self.sb_out.clear();
                    self.sb_out.push_back(0xaa);
                    self.sb_cmd = None;
                    self.sb_play = None;
                }
            }
            0x22c => self.sb_dsp_write(v),
            0x224 | 0x225 => {} // mixer
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
                // PIT channel 0 divisor: lo byte then hi byte (phase reset by port 43h)
                if self.pit_phase == 0 {
                    self.pit_lo = v;
                    self.pit_phase = 1;
                } else {
                    self.pit_phase = 0;
                    let d = ((v as u32) << 8) | self.pit_lo as u32;
                    self.pit_divisor = if d == 0 { 0x1_0000 } else { d };
                    self.steps_per_tick =
                        (self.pit_divisor as u64 * STEPS_PER_SECOND / 1_193_182).max(1000);
                    if self.trace_ints {
                        eprintln!(
                            "PIT ch0 divisor {:#x} -> {} steps/tick",
                            self.pit_divisor, self.steps_per_tick
                        );
                    }
                }
            }
            0x43 => {
                if v & 0xc0 == 0 {
                    self.pit_phase = 0;
                }
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
                    match self.gc_idx & 15 {
                        0 => vga.set_reset = v & 0x0f,
                        1 => vga.enable_sr = v & 0x0f,
                        3 => {
                            vga.rotate = v & 7;
                            vga.logic_op = (v >> 3) & 3;
                        }
                        4 => vga.read_map = v & 3,
                        5 => vga.write_mode = v & 3,
                        8 => vga.bit_mask = v,
                        _ => {}
                    }
                }
            }
            0x3d4 => self.crtc_idx = v,
            0x3d5 => self.crtc[(self.crtc_idx & 31) as usize] = v,
            0x21 => self.pic_mask0 = v, // 8259 master IMR: the game masks/unmasks IRQ lines here
            0xa1 => self.pic_mask1 = v, // 8259 slave IMR
            0x20 | 0xa0 | 0x41 | 0x42 | 0x61 | 0x3c2 | 0x3c0 => {}
            _ => {
                if self.trace_ints {
                    eprintln!("out port {port:#x} = {value:#x}");
                }
            }
        }
    }

    // ---------------- native interrupt services ----------------

    fn ivt_hooked(&self, v: u8) -> bool {
        let off = self.m.read16(0, v as u32 * 4);
        let seg = self.m.read16(0, v as u32 * 4 + 2);
        (off, seg) != ((v as u16) * 4, STUB_SEG)
    }

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
                    self.m.log_range(0xb8000, 0x8000);
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
            0x0000 | 0x0021 => {
                self.m.regs.set_ax(0xffff); // mouse present
                self.m.regs.set_bx(2);
                self.mouse_handler = None;
                self.mouse_shown = -1;
                self.mouse_x = 320;
                self.mouse_y = 100;
            }
            0x0001 => self.mouse_shown += 1,
            0x0002 => self.mouse_shown -= 1,
            0x0003 => {
                self.m.regs.set_bx(self.mouse_buttons);
                self.m.regs.set_cx(self.mouse_x);
                self.m.regs.set_dx(self.mouse_y);
            }
            0x0004 => {
                self.mouse_x = self.m.regs.cx();
                self.mouse_y = self.m.regs.dx();
            }
            0x0005 => {
                let b = (self.m.regs.bx() as usize).min(1);
                let (n, x, y) = self.mouse_presses[b];
                self.mouse_presses[b] = (0, x, y);
                self.m.regs.set_ax(self.mouse_buttons);
                self.m.regs.set_bx(n);
                self.m.regs.set_cx(x);
                self.m.regs.set_dx(y);
            }
            0x0006 => {
                let b = (self.m.regs.bx() as usize).min(1);
                let (n, x, y) = self.mouse_releases[b];
                self.mouse_releases[b] = (0, x, y);
                self.m.regs.set_ax(self.mouse_buttons);
                self.m.regs.set_bx(n);
                self.m.regs.set_cx(x);
                self.m.regs.set_dx(y);
            }
            0x0007 | 0x0008 => {} // ranges: virtual coords already span the full screen
            0x000b => {
                self.m.regs.set_cx(0); // mickeys since last read: frontend feeds absolute
                self.m.regs.set_dx(0);
            }
            0x000c => {
                let mask = self.m.regs.cx();
                if mask == 0 {
                    self.mouse_handler = None;
                } else {
                    self.mouse_handler = Some((mask, self.m.regs.es, self.m.regs.dx()));
                }
            }
            0x0014 => {
                let (omask, oseg, ooff) = self.mouse_handler.take().unwrap_or((0, 0, 0));
                self.mouse_handler = Some((self.m.regs.cx(), self.m.regs.es, self.m.regs.dx()));
                self.m.regs.set_cx(omask);
                self.m.regs.es = oseg;
                self.m.regs.set_dx(ooff);
            }
            0x0015 => {
                self.m.regs.set_bx(0); // driver state save size: none needed
            }
            0x0016 | 0x0017 => {}
            0x0024 => {
                self.m.regs.set_bx(0x0805); // driver 8.05
                self.m.regs.set_ch(4);      // PS/2
                self.m.regs.set_cl(0);
            }
            _ => {}
        }
        self.cpu.emulate_iret(&mut self.m);
        Ok(())
    }

    /// Feed a mouse state change (DOS-virtual coords: x 0..639, y 0..199, buttons bit0=left
    /// bit1=right). Computes the int 33h event mask and queues the user callback if installed.
    pub fn mouse_event(&mut self, x: u16, y: u16, buttons: u16) {
        let mut mask = 0u16;
        if x != self.mouse_x || y != self.mouse_y {
            mask |= 1;
        }
        let old = self.mouse_buttons;
        for b in 0..2u16 {
            let was = old & (1 << b) != 0;
            let is = buttons & (1 << b) != 0;
            if !was && is {
                mask |= 2 << (b * 2);
                let n = self.mouse_presses[b as usize].0 + 1;
                self.mouse_presses[b as usize] = (n, x, y);
            }
            if was && !is {
                mask |= 4 << (b * 2);
                let n = self.mouse_releases[b as usize].0 + 1;
                self.mouse_releases[b as usize] = (n, x, y);
            }
        }
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_buttons = buttons;
        if mask != 0 {
            self.mouse_pending |= mask;
        }
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn crtc_reg(&self, i: usize) -> u8 {
        self.crtc[i & 31]
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
                                self.m.log_range(dst, EMS_PAGE);
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

    /// DSP command byte stream at 0x22C.
    fn sb_dsp_write(&mut self, v: u8) {
        if let Some((cmd, mut args, need)) = self.sb_cmd.take() {
            args.push(v);
            if args.len() < need {
                self.sb_cmd = Some((cmd, args, need));
            } else {
                self.sb_dsp_exec(cmd, &args);
            }
            return;
        }
        let need = match v {
            0x40 | 0xe0 | 0x10 | 0x48 => 1,
            0x14 | 0x16 | 0x17 | 0x41 | 0x42 => 2,
            _ => 0,
        };
        let need = if v == 0x48 { 2 } else { need };
        if need == 0 {
            self.sb_dsp_exec(v, &[]);
        } else {
            self.sb_cmd = Some((v, vec![], need));
        }
    }

    fn sb_dsp_exec(&mut self, cmd: u8, args: &[u8]) {
        if self.trace_ints {
            eprintln!("SB DSP cmd {cmd:#04x} args {args:x?} @step {}", self.cpu.steps);
        }
        match cmd {
            0x40 => {
                self.sb_time_constant = args[0];
                self.sb_rate_hz = 1_000_000 / (256 - args[0] as u32);
            }
            0x41 | 0x42 => {
                self.sb_rate_hz = ((args[0] as u32) << 8) | args[1] as u32;
            }
            0x14 | 0x16 | 0x17 => {
                // 8-bit single-cycle DMA playback, length = args+1
                let len = (((args[1] as u32) << 8) | args[0] as u32) + 1;
                self.sb_start_playback(len, false);
            }
            0x1c | 0x90 => {
                // auto-init: block size was set via 0x48 (stored in dma count)
                let len = self.dma_count[1] as u32 + 1;
                self.sb_start_playback(len, true);
            }
            0x48 => {} // block size: playback uses the DMA count
            0xd0 | 0xd3 => {} // pause / speaker off
            0xd1 | 0xd4 => {} // speaker on / continue
            0xda => self.sb_play = None, // exit auto-init
            0xe0 => {
                let a = args[0];
                self.sb_out.push_back(!a);
            }
            0xe1 => {
                self.sb_out.push_back(4); // DSP 4.05 — SB16 (matches DOSBox default sbtype=sb16;
                self.sb_out.push_back(5); // the game is launched as SB16 via the S162227 arg)
            }
            _ => {
                if self.trace_ints {
                    eprintln!("SB DSP cmd {cmd:#04x} args {args:?} (ignored)");
                }
            }
        }
    }

    fn sb_start_playback(&mut self, len: u32, auto: bool) {
        // Capture the PCM the game just queued (DMA ch1) — the audio-out tap.
        let ch = 1usize;
        let base = ((self.dma_page[ch] as usize) << 16) | (self.dma_addr[ch] as usize);
        let n = (len as usize).min(0x10000);
        if base + n <= self.m.mem.len() {
            self.sb_pcm.extend_from_slice(&self.m.mem[base..base + n]);
            self.sb_pcm_rate = self.sb_rate_hz;
        }
        self.sb_play = Some((ch, self.cpu.steps, len, auto));
        self.dma_cur_count[ch] = self.dma_count[ch];
    }

    /// Advance the modelled DMA count from the playback clock; fire the SB IRQ on completion.
    fn tick_sb_playback(&mut self) {
        let Some((ch, start, len, auto)) = self.sb_play else {
            return;
        };
        let elapsed = self.cpu.steps - start;
        let played = (elapsed * self.sb_rate_hz as u64 / STEPS_PER_SECOND) as u32;
        if played >= len {
            self.dma_tc |= 1 << ch;
            self.dma_cur_count[ch] = 0xffff;
            if auto {
                self.sb_play = Some((ch, self.cpu.steps, len, auto));
            } else {
                self.sb_play = None;
            }
            self.sb_irq_pending = true;
        } else {
            self.dma_cur_count[ch] = (self.dma_count[ch]).wrapping_sub(played as u16);
        }
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
                if self.trace_ints {
                    let found = self.resolve(&pattern, false).map(|p| p.exists()).unwrap_or(false);
                    eprintln!("FindFirst \"{pattern}\" -> {}", if found { "found" } else { "NONE" });
                }
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
                self.opened_files.push((self.cpu.steps, path.clone()));
                let write = self.m.regs.al() & 3 != 0;
                let opened = self.resolve(&path, false).ok().filter(|p| {
                    std::fs::OpenOptions::new().read(true).write(write).open(p).is_ok()
                }).is_some();
                if self.trace_ints {
                    eprintln!("int21 open \"{path}\" -> {}", if opened { "OK" } else { "FAIL" });
                }
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
                        self.m.log_range(base, n);
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

    /// One-line machine state summary for boot triage.
    pub fn debug_state(&self) -> String {
        let ivt8 = (self.m.read16(0, 0x22), self.m.read16(0, 0x20));
        let ivt9 = (self.m.read16(0, 0x26), self.m.read16(0, 0x24));
        let ivt1c = (self.m.read16(0, 0x72), self.m.read16(0, 0x70));
        format!(
            "cs:ip={:04x}:{:04x} iflag={} ticks={} steps_per_tick={} pit={:#x} \n\
             ivt8={:04x}:{:04x} ivt9={:04x}:{:04x} ivt1c={:04x}:{:04x} \n\
             seq={:02x?} gc={:02x?} crtc0c/0d/13={:02x}/{:02x}/{:02x} chain4={} mouse_handler={:?}",
            self.cpu.cs, self.cpu.ip, self.cpu.iflag, self.ticks, self.steps_per_tick,
            self.pit_divisor,
            ivt8.0, ivt8.1, ivt9.0, ivt9.1, ivt1c.0, ivt1c.1,
            self.seq, self.gc, self.crtc[0x0c], self.crtc[0x0d], self.crtc[0x13],
            self.m.vga.as_deref().map(|v| v.chain4).unwrap_or(true),
            self.mouse_handler,
        ) + &{
            let mut hooks = String::from("\nhooked vectors:");
            for v in 0..=255u8 {
                if v != 0x67 && self.ivt_hooked(v) {
                    let off = self.m.read16(0, v as u32 * 4);
                    let seg = self.m.read16(0, v as u32 * 4 + 2);
                    hooks += &format!(" {v:02x}->{seg:04x}:{off:04x}");
                }
            }
            hooks += &format!(
                "\nsb: play={:?} pcm_bytes={} rate={} dma1 base={:04x} cnt={:04x} cur={:04x} page={:02x} mode={:02x}",
                self.sb_play, self.sb_pcm.len(), self.sb_rate_hz,
                self.dma_addr[1], self.dma_count[1], self.dma_cur_count[1],
                self.dma_page[1], self.dma_mode[1],
            );
            hooks
        }
    }

    pub fn console_output(&self) -> &str {
        &self.console
    }

    /// Push a key event (scancode, ascii) — M3 wires real input to this.
    pub fn key_event(&mut self, scancode: u8, ascii: u8) {
        self.kbd_queue.push_back((scancode, ascii));
        self.bios_keys.push_back((scancode, ascii));
        self.kbd_irq_pending += 1;
    }
}
