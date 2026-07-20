//! Real-mode 80386 interpreter over [`Machine`] — executes the ORIGINAL BLOODPRG.EXE bytes.
//!
//! Role in path B: the runtime fallback that makes the recompilation RUNNABLE before (and while)
//! static lift coverage completes. Code the lifter has verified runs as native Rust; everything
//! else executes here, instruction by instruction, over the same [`Machine`] state and the same
//! oracle-verified flag helpers on [`Regs`] that the lifted functions use. Faithfulness is by
//! construction: this module adds no game logic of its own.
//!
//! Boundary design: the interpreter is PURE CPU. Anything that crosses into the OS/hardware —
//! `int`, `in`, `out`, `hlt` — returns an [`Exit`] to the caller (the DOS/hardware layer), which
//! performs the effect on the `Machine` and resumes at `cpu.ip`. `ret`/`retf` at call depth 0
//! exit BEFORE executing (mirroring the oracle's stop-at-ret), so replaying an oracle vector is
//! exactly `Cpu::new(0, entry)` + `run` + compare.
//!
//! Verified by `recomp::tests::interp_replays_full_oracle_corpus`: every oracle vector in
//! `re/tools/oracle_vectors/` — the same corpus, same pass criteria as the lifted batches.

use super::machine::{Machine, Regs};

/// Why the interpreter stopped. `Ret`/`Retf` = about to execute a depth-0 return (state is the
/// pre-return state; `ip` has advanced past the instruction). `Int`/`In`/`Out` = the instruction
/// needs the OS/hardware layer; `ip` points AFTER it, so the caller applies the effect and
/// resumes. `Unimplemented` = decoder gap, fail loud.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    Ret,
    Retf,
    Int { vector: u8 },
    In { port: u16, size: u8 },
    Out { port: u16, size: u8, value: u32 },
    Hlt,
    Unimplemented { cs: u16, ip: u16, byte: u8, what: &'static str },
    StepLimit,
}

/// Execution state that lives OUTSIDE [`Machine`]: the instruction pointer, the call depth used
/// for depth-0 return exits, and the interrupt-enable flag (not modelled in `Regs` — the lifted
/// code never reads it, and `pushf` deliberately mirrors lift.py's flag-word encoding).
pub struct Cpu {
    pub cs: u16,
    pub ip: u16,
    pub depth: u32,
    pub iflag: bool,
    /// FLAGS bits 12-14 (IOPL/NT): storable in 386 real mode — programs detect a 386 by
    /// round-tripping them through popf/pushf, so they must persist. Bit 15 stays 0.
    pub flags_high: u16,
    pub steps: u64,
}

/// General register by 3-bit index, 16-bit view: AX CX DX BX SP BP SI DI.
fn r16(r: &Regs, i: u8) -> u16 {
    match i {
        0 => r.ax(),
        1 => r.cx(),
        2 => r.dx(),
        3 => r.bx(),
        4 => r.sp(),
        5 => r.bp(),
        6 => r.si(),
        _ => r.di(),
    }
}
fn w16(r: &mut Regs, i: u8, v: u16) {
    match i {
        0 => r.set_ax(v),
        1 => r.set_cx(v),
        2 => r.set_dx(v),
        3 => r.set_bx(v),
        4 => r.set_sp(v),
        5 => r.set_bp(v),
        6 => r.set_si(v),
        _ => r.set_di(v),
    }
}
/// 8-bit view: AL CL DL BL AH CH DH BH.
fn r8(r: &Regs, i: u8) -> u8 {
    match i {
        0 => r.al(),
        1 => r.cl(),
        2 => r.dl(),
        3 => r.bl(),
        4 => r.ah(),
        5 => r.ch(),
        6 => r.dh(),
        _ => r.bh(),
    }
}
fn w8(r: &mut Regs, i: u8, v: u8) {
    match i {
        0 => r.set_al(v),
        1 => r.set_cl(v),
        2 => r.set_dl(v),
        3 => r.set_bl(v),
        4 => r.set_ah(v),
        5 => r.set_ch(v),
        6 => r.set_dh(v),
        _ => r.set_bh(v),
    }
}
/// 32-bit view: EAX ECX EDX EBX ESP EBP ESI EDI.
fn r32(r: &Regs, i: u8) -> u32 {
    match i {
        0 => r.eax,
        1 => r.ecx,
        2 => r.edx,
        3 => r.ebx,
        4 => r.esp,
        5 => r.ebp,
        6 => r.esi,
        _ => r.edi,
    }
}
fn w32(r: &mut Regs, i: u8, v: u32) {
    match i {
        0 => r.eax = v,
        1 => r.ecx = v,
        2 => r.edx = v,
        3 => r.ebx = v,
        4 => r.esp = v,
        5 => r.ebp = v,
        6 => r.esi = v,
        _ => r.edi = v,
    }
}

/// ALU op by the standard 3-bit encoding: ADD OR ADC SBB AND SUB XOR CMP. `None` = no writeback.
fn alu8(r: &mut Regs, op: u8, a: u8, b: u8) -> Option<u8> {
    Some(match op {
        0 => r.add8(a, b),
        1 => r.or8(a, b),
        2 => r.adc8(a, b),
        3 => r.sbb8(a, b),
        4 => r.and8(a, b),
        5 => r.sub8(a, b),
        6 => r.xor8(a, b),
        _ => {
            r.cmp8(a, b);
            return None;
        }
    })
}
fn alu16(r: &mut Regs, op: u8, a: u16, b: u16) -> Option<u16> {
    Some(match op {
        0 => r.add16(a, b),
        1 => r.or16(a, b),
        2 => r.adc16(a, b),
        3 => r.sbb16(a, b),
        4 => r.and16(a, b),
        5 => r.sub16(a, b),
        6 => r.xor16(a, b),
        _ => {
            r.cmp16(a, b);
            return None;
        }
    })
}
fn alu32(r: &mut Regs, op: u8, a: u32, b: u32) -> Option<u32> {
    Some(match op {
        0 => r.add32(a, b),
        1 => r.or32(a, b),
        2 => r.adc32(a, b),
        3 => r.sbb32(a, b),
        4 => r.and32(a, b),
        5 => r.sub32(a, b),
        6 => r.xor32(a, b),
        _ => {
            r.cmp32(a, b);
            return None;
        }
    })
}

/// Shift/rotate group by /reg encoding: ROL ROR RCL RCR SHL SHR SHL SAR. `None` = helper gap.
fn shift8(r: &mut Regs, op: u8, v: u8, c: u8) -> Option<u8> {
    Some(match op {
        0 => r.rol8(v, c),
        1 => r.ror8(v, c),
        2 => r.rcl8(v, c),
        3 => r.rcr8(v, c),
        4 | 6 => r.shl8(v, c),
        5 => r.shr8(v, c),
        _ => r.sar8(v, c),
    })
}
fn shift16(r: &mut Regs, op: u8, v: u16, c: u8) -> Option<u16> {
    Some(match op {
        0 => r.rol16(v, c),
        1 => r.ror16(v, c),
        2 => r.rcl16(v, c),
        3 => r.rcr16(v, c),
        4 | 6 => r.shl16(v, c),
        5 => r.shr16(v, c),
        _ => r.sar16(v, c),
    })
}
fn shift32(r: &mut Regs, op: u8, v: u32, c: u8) -> Option<u32> {
    Some(match op {
        0 => r.rol32(v, c),
        1 => r.ror32(v, c),
        2 | 3 => return None, // rcl32/rcr32: no oracle-verified helper yet
        4 | 6 => r.shl32(v, c),
        5 => r.shr32(v, c),
        _ => r.sar32(v, c),
    })
}

/// Condition-code table (Jcc/SETcc low nibble).
fn cond(r: &Regs, c: u8) -> bool {
    match c & 0xf {
        0 => r.of,
        1 => !r.of,
        2 => r.cf,
        3 => !r.cf,
        4 => r.zf,
        5 => !r.zf,
        6 => r.cf || r.zf,
        7 => !(r.cf || r.zf),
        8 => r.sf,
        9 => !r.sf,
        0xa => r.pf,
        0xb => !r.pf,
        0xc => r.sf != r.of,
        0xd => r.sf == r.of,
        0xe => r.zf || (r.sf != r.of),
        _ => !r.zf && (r.sf == r.of),
    }
}

/// FLAGS word. Mirrors lift.py's `pushf` encoding EXACTLY (bit 1 set, no IF/TF) so interpreted
/// `pushf` writes the same stack byte the lifted code and the oracle vectors have.
fn flags_word(r: &Regs) -> u16 {
    0x0002
        | (r.cf as u16)
        | ((r.pf as u16) << 2)
        | ((r.af as u16) << 4)
        | ((r.zf as u16) << 6)
        | ((r.sf as u16) << 7)
        | ((r.df as u16) << 10)
        | ((r.of as u16) << 11)
}
fn set_flags_word(r: &mut Regs, f: u16) {
    r.cf = f & 1 != 0;
    r.pf = f & 4 != 0;
    r.af = f & 0x10 != 0;
    r.zf = f & 0x40 != 0;
    r.sf = f & 0x80 != 0;
    r.df = f & 0x400 != 0;
    r.of = f & 0x800 != 0;
}

/// A decoded ModRM operand: register index or memory (segment resolved, offset already wrapped
/// to 16 bits for 16-bit addressing forms).
enum Rm {
    Reg(u8),
    Mem { seg: u16, off: u32 },
}

macro_rules! unimpl {
    ($self:ident, $m:ident, $b:expr, $what:expr) => {
        return Some(Exit::Unimplemented {
            cs: $self.cs,
            ip: $self.ip,
            byte: $b,
            what: $what,
        })
    };
}

impl Cpu {
    pub fn new(cs: u16, ip: u16) -> Self {
        Self {
            cs,
            ip,
            depth: 0,
            iflag: true,
            flags_high: 0,
            steps: 0,
        }
    }

    /// Run until an [`Exit`] or `max_steps` instructions.
    pub fn run(&mut self, m: &mut Machine, max_steps: u64) -> Exit {
        for _ in 0..max_steps {
            if let Some(e) = self.step(m) {
                return e;
            }
        }
        Exit::StepLimit
    }

    /// Deliver an interrupt through the guest IVT (hardware IRQ or a re-dispatched `int`):
    /// push FLAGS (with IF), CS, IP; clear IF; jump to the vector. The handler's `iret`
    /// (or [`Self::emulate_iret`] from a native stub) unwinds it.
    pub fn deliver_int(&mut self, m: &mut Machine, v: u8) {
        let f = flags_word(&m.regs) | ((self.iflag as u16) << 9) | self.flags_high;
        let (cs, ip) = (self.cs, self.ip);
        self.push16(m, f);
        self.push16(m, cs);
        self.push16(m, ip);
        self.iflag = false;
        self.ip = m.read16(0, v as u32 * 4);
        self.cs = m.read16(0, v as u32 * 4 + 2);
        m.regs.cs = self.cs;
    }

    /// Pop an interrupt frame: IP, CS, FLAGS (restoring IF). Used by the `iret` opcode and by
    /// the runtime to complete a natively-serviced interrupt.
    pub fn emulate_iret(&mut self, m: &mut Machine) {
        self.ip = self.pop16(m);
        self.cs = self.pop16(m);
        m.regs.cs = self.cs;
        let f = self.pop16(m);
        set_flags_word(&mut m.regs, f);
        self.iflag = f & 0x200 != 0;
        self.flags_high = f & 0x7000;
    }

    /// Patch the CF bit in the FLAGS word of the interrupt frame at SS:SP (frame = IP,CS,FLAGS).
    /// DOS returns success/failure in the CALLER's carry flag — the one `iret` restores.
    pub fn patch_frame_cf(&self, m: &mut Machine, cf: bool) {
        let sp = m.regs.sp() as u32;
        let f = m.read16(m.regs.ss, sp.wrapping_add(4));
        let nf = if cf { f | 1 } else { f & !1 };
        m.write16(m.regs.ss, sp.wrapping_add(4), nf);
    }

    /// Patch the ZF bit in the interrupt frame's FLAGS (int 16h AH=1 keyboard status).
    pub fn patch_frame_zf(&self, m: &mut Machine, zf: bool) {
        let sp = m.regs.sp() as u32;
        let f = m.read16(m.regs.ss, sp.wrapping_add(4));
        let nf = if zf { f | 0x40 } else { f & !0x40 };
        m.write16(m.regs.ss, sp.wrapping_add(4), nf);
    }

    fn fetch8(&mut self, m: &Machine) -> u8 {
        let b = m.read8(self.cs, self.ip as u32);
        self.ip = self.ip.wrapping_add(1);
        b
    }
    fn fetch16(&mut self, m: &Machine) -> u16 {
        u16::from_le_bytes([self.fetch8(m), self.fetch8(m)])
    }
    fn fetch32(&mut self, m: &Machine) -> u32 {
        (self.fetch16(m) as u32) | ((self.fetch16(m) as u32) << 16)
    }

    fn push16(&mut self, m: &mut Machine, v: u16) {
        m.regs.set_sp(m.regs.sp().wrapping_sub(2));
        m.write16(m.regs.ss, m.regs.sp() as u32, v);
    }
    fn pop16(&mut self, m: &mut Machine) -> u16 {
        let v = m.read16(m.regs.ss, m.regs.sp() as u32);
        m.regs.set_sp(m.regs.sp().wrapping_add(2));
        v
    }
    fn push32(&mut self, m: &mut Machine, v: u32) {
        m.regs.set_sp(m.regs.sp().wrapping_sub(4));
        m.write32(m.regs.ss, m.regs.sp() as u32, v);
    }
    fn pop32(&mut self, m: &mut Machine) -> u32 {
        let v = m.read32(m.regs.ss, m.regs.sp() as u32);
        m.regs.set_sp(m.regs.sp().wrapping_add(4));
        v
    }

    fn segv(&self, m: &Machine, i: u8) -> u16 {
        match i {
            0 => m.regs.es,
            1 => self.cs,
            2 => m.regs.ss,
            3 => m.regs.ds,
            4 => m.regs.fs,
            _ => m.regs.gs,
        }
    }
    fn resolve_seg(&self, m: &Machine, ovr: Option<u8>, def_ss: bool) -> u16 {
        match ovr {
            Some(i) => self.segv(m, i),
            None if def_ss => m.regs.ss,
            None => m.regs.ds,
        }
    }

    /// Decode a ModRM byte (and SIB/displacement) into `(mod, reg, rm)`.
    fn modrm(&mut self, m: &Machine, ovr: Option<u8>, adsz: bool) -> (u8, u8, Rm) {
        let mb = self.fetch8(m);
        let md = mb >> 6;
        let reg = (mb >> 3) & 7;
        let rm = mb & 7;
        if md == 3 {
            return (md, reg, Rm::Reg(rm));
        }
        if !adsz {
            let (base, def_ss): (u16, bool) = match rm {
                0 => (m.regs.bx().wrapping_add(m.regs.si()), false),
                1 => (m.regs.bx().wrapping_add(m.regs.di()), false),
                2 => (m.regs.bp().wrapping_add(m.regs.si()), true),
                3 => (m.regs.bp().wrapping_add(m.regs.di()), true),
                4 => (m.regs.si(), false),
                5 => (m.regs.di(), false),
                6 => {
                    if md == 0 {
                        (0, false)
                    } else {
                        (m.regs.bp(), true)
                    }
                }
                _ => (m.regs.bx(), false),
            };
            let disp: u16 = match md {
                0 => {
                    if rm == 6 {
                        self.fetch16(m)
                    } else {
                        0
                    }
                }
                1 => self.fetch8(m) as i8 as u16,
                _ => self.fetch16(m),
            };
            let off = base.wrapping_add(disp);
            let seg = self.resolve_seg(m, ovr, def_ss);
            (md, reg, Rm::Mem { seg, off: off as u32 })
        } else {
            // 32-bit addressing (0x67 prefix): full modrm/SIB. Offsets are NOT wrapped to 16
            // bits — Machine::lin adds the full 32-bit value (matching the real CPU + oracle).
            let mut def_ss = false;
            let base: u32 = if rm == 4 {
                let sib = self.fetch8(m);
                let scale = sib >> 6;
                let idx = (sib >> 3) & 7;
                let bse = sib & 7;
                let mut a: u32 = 0;
                if idx != 4 {
                    a = r32(&m.regs, idx).wrapping_shl(scale as u32);
                }
                if bse == 5 && md == 0 {
                    a.wrapping_add(self.fetch32(m))
                } else {
                    if bse == 4 || bse == 5 {
                        def_ss = true;
                    }
                    a.wrapping_add(r32(&m.regs, bse))
                }
            } else if rm == 5 && md == 0 {
                self.fetch32(m)
            } else {
                if rm == 5 {
                    def_ss = true;
                }
                r32(&m.regs, rm)
            };
            let disp: u32 = match md {
                1 => self.fetch8(m) as i8 as u32,
                2 => self.fetch32(m),
                _ => 0,
            };
            let off = base.wrapping_add(disp);
            let seg = self.resolve_seg(m, ovr, def_ss);
            (md, reg, Rm::Mem { seg, off })
        }
    }

    fn rm_r8(&self, m: &Machine, rm: &Rm) -> u8 {
        match rm {
            Rm::Reg(i) => r8(&m.regs, *i),
            Rm::Mem { seg, off } => m.read8(*seg, *off),
        }
    }
    fn rm_w8(&self, m: &mut Machine, rm: &Rm, v: u8) {
        match rm {
            Rm::Reg(i) => w8(&mut m.regs, *i, v),
            Rm::Mem { seg, off } => m.write8(*seg, *off, v),
        }
    }
    fn rm_r16(&self, m: &Machine, rm: &Rm) -> u16 {
        match rm {
            Rm::Reg(i) => r16(&m.regs, *i),
            Rm::Mem { seg, off } => m.read16(*seg, *off),
        }
    }
    fn rm_w16(&self, m: &mut Machine, rm: &Rm, v: u16) {
        match rm {
            Rm::Reg(i) => w16(&mut m.regs, *i, v),
            Rm::Mem { seg, off } => m.write16(*seg, *off, v),
        }
    }
    fn rm_r32(&self, m: &Machine, rm: &Rm) -> u32 {
        match rm {
            Rm::Reg(i) => r32(&m.regs, *i),
            Rm::Mem { seg, off } => m.read32(*seg, *off),
        }
    }
    fn rm_w32(&self, m: &mut Machine, rm: &Rm, v: u32) {
        match rm {
            Rm::Reg(i) => w32(&mut m.regs, *i, v),
            Rm::Mem { seg, off } => m.write32(*seg, *off, v),
        }
    }

    /// Execute exactly one instruction (test/tools entry point). Returns the same `Exit` the
    /// run loop sees, or `None` if the instruction completed normally.
    pub fn step_public(&mut self, m: &mut Machine) -> Option<Exit> {
        self.step(m)
    }

    /// Execute one instruction. `None` = keep going.
    fn step(&mut self, m: &mut Machine) -> Option<Exit> {
        self.steps += 1;
        m.regs.cs = self.cs;
        let ip0 = self.ip;
        m.ip = ip0;
        if !m.trap_ips.is_empty() {
            if let Some(c) = m.trap_ips.get_mut(&(self.cs, ip0)) {
                *c += 1;
            }
        }
        if m.capture_ip == Some((self.cs, ip0)) && m.captured.is_none() {
            m.captured = Some((m.regs.ss, m.regs.ds, m.regs.es, m.regs.si(), m.regs.bp(), m.regs.bx()));
            let (ss, sp) = (m.regs.ss, m.regs.sp());
            let w = |off: u16| m.read16(ss, sp.wrapping_add(off) as u32);
            m.capture_ret = Some((sp, w(0), w(2), w(4)));
            m.captured_prev = Some(m.exec_prev);
        }
        m.exec_prev = (self.cs, ip0);
        if m.capture_ip2 == Some((self.cs, ip0)) && m.captured2.len() < 40 {
            // at the pixel write: record (es, di) = where the glyph pixel lands
            m.captured2.push((m.regs.es, m.regs.di()));
        }
        let mut ovr: Option<u8> = None;
        let mut opsz = false;
        let mut adsz = false;
        let mut rep: Option<bool> = None; // Some(true)=F3 rep/repe, Some(false)=F2 repne
        let op = loop {
            let b = self.fetch8(m);
            match b {
                0x26 => ovr = Some(0),
                0x2e => ovr = Some(1),
                0x36 => ovr = Some(2),
                0x3e => ovr = Some(3),
                0x64 => ovr = Some(4),
                0x65 => ovr = Some(5),
                0x66 => opsz = true,
                0x67 => adsz = true,
                0xf3 => rep = Some(true),
                0xf2 => rep = Some(false),
                0xf0 => {} // lock — single-CPU, no-op
                _ => break b,
            }
        };

        match op {
            // ---- push/pop segment registers ----
            0x06 | 0x0e | 0x16 | 0x1e => {
                let s = self.segv(m, (op >> 3) & 3);
                if opsz {
                    self.push32(m, s as u32)
                } else {
                    self.push16(m, s)
                }
            }
            0x07 | 0x17 | 0x1f => {
                let v = if opsz {
                    self.pop32(m) as u16
                } else {
                    self.pop16(m)
                };
                match op {
                    0x07 => m.regs.es = v,
                    0x17 => m.regs.ss = v,
                    _ => m.regs.ds = v,
                }
            }

            // ---- string port I/O: one element per execution, REP rewinds and retries ----
            0x6c | 0x6d => unimpl!(self, m, op, "ins"),
            0x6e | 0x6f => {
                if adsz {
                    unimpl!(self, m, op, "outs with 0x67");
                }
                let size: u16 = if op == 0x6e {
                    1
                } else if opsz {
                    4
                } else {
                    2
                };
                if rep.is_some() && m.regs.cx() == 0 {
                    // exhausted: no transfer
                } else {
                    let delta = if m.regs.df { size.wrapping_neg() } else { size };
                    let sseg = self.resolve_seg(m, ovr, false);
                    let si = m.regs.si() as u32;
                    let value = match size {
                        1 => m.read8(sseg, si) as u32,
                        2 => m.read16(sseg, si) as u32,
                        _ => m.read32(sseg, si),
                    };
                    m.regs.set_si(m.regs.si().wrapping_add(delta));
                    if rep.is_some() {
                        let cx = m.regs.cx().wrapping_sub(1);
                        m.regs.set_cx(cx);
                        if cx != 0 {
                            self.ip = ip0;
                        }
                    }
                    return Some(Exit::Out {
                        port: m.regs.dx(),
                        size: size as u8,
                        value,
                    });
                }
            }

            // ---- BCD adjust ----
            0x27 | 0x2f => {
                // daa/das
                let old_al = m.regs.al();
                let old_cf = m.regs.cf;
                let sub = op == 0x2f;
                let mut al = old_al;
                let mut cf = false;
                if (old_al & 0xf) > 9 || m.regs.af {
                    al = if sub {
                        al.wrapping_sub(6)
                    } else {
                        al.wrapping_add(6)
                    };
                    cf = old_cf || (!sub && old_al > 0xf9) || (sub && old_al < 6);
                    m.regs.af = true;
                } else {
                    m.regs.af = false;
                }
                if old_al > 0x99 || old_cf {
                    al = if sub {
                        al.wrapping_sub(0x60)
                    } else {
                        al.wrapping_add(0x60)
                    };
                    cf = true;
                }
                m.regs.set_al(al);
                m.regs.cf = cf;
                m.regs.zf = al == 0;
                m.regs.sf = al & 0x80 != 0;
                m.regs.pf = al.count_ones() % 2 == 0;
            }
            0x37 | 0x3f => {
                // aaa/aas
                let sub = op == 0x3f;
                if (m.regs.al() & 0xf) > 9 || m.regs.af {
                    let ax = m.regs.ax();
                    let nax = if sub {
                        ax.wrapping_sub(6).wrapping_sub(0x100)
                    } else {
                        ax.wrapping_add(6).wrapping_add(0x100)
                    };
                    m.regs.set_ax(nax);
                    m.regs.af = true;
                    m.regs.cf = true;
                } else {
                    m.regs.af = false;
                    m.regs.cf = false;
                }
                m.regs.set_al(m.regs.al() & 0xf);
            }

            // ---- the regular ALU block: 0x00..0x3D minus specials handled above ----
            _ if op < 0x40 && (op & 7) <= 5 => {
                let aluop = (op >> 3) & 7;
                match op & 7 {
                    0 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let a = self.rm_r8(m, &rm);
                        let b = r8(&m.regs, reg);
                        if let Some(r) = alu8(&mut m.regs, aluop, a, b) {
                            self.rm_w8(m, &rm, r);
                        }
                    }
                    1 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        if opsz {
                            let a = self.rm_r32(m, &rm);
                            let b = r32(&m.regs, reg);
                            if let Some(r) = alu32(&mut m.regs, aluop, a, b) {
                                self.rm_w32(m, &rm, r);
                            }
                        } else {
                            let a = self.rm_r16(m, &rm);
                            let b = r16(&m.regs, reg);
                            if let Some(r) = alu16(&mut m.regs, aluop, a, b) {
                                self.rm_w16(m, &rm, r);
                            }
                        }
                    }
                    2 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let a = r8(&m.regs, reg);
                        let b = self.rm_r8(m, &rm);
                        if let Some(r) = alu8(&mut m.regs, aluop, a, b) {
                            w8(&mut m.regs, reg, r);
                        }
                    }
                    3 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        if opsz {
                            let a = r32(&m.regs, reg);
                            let b = self.rm_r32(m, &rm);
                            if let Some(r) = alu32(&mut m.regs, aluop, a, b) {
                                w32(&mut m.regs, reg, r);
                            }
                        } else {
                            let a = r16(&m.regs, reg);
                            let b = self.rm_r16(m, &rm);
                            if let Some(r) = alu16(&mut m.regs, aluop, a, b) {
                                w16(&mut m.regs, reg, r);
                            }
                        }
                    }
                    4 => {
                        let imm = self.fetch8(m);
                        let a = m.regs.al();
                        if let Some(r) = alu8(&mut m.regs, aluop, a, imm) {
                            m.regs.set_al(r);
                        }
                    }
                    _ => {
                        if opsz {
                            let imm = self.fetch32(m);
                            let a = m.regs.eax;
                            if let Some(r) = alu32(&mut m.regs, aluop, a, imm) {
                                m.regs.eax = r;
                            }
                        } else {
                            let imm = self.fetch16(m);
                            let a = m.regs.ax();
                            if let Some(r) = alu16(&mut m.regs, aluop, a, imm) {
                                m.regs.set_ax(r);
                            }
                        }
                    }
                }
            }

            // ---- inc/dec r16/r32 ----
            0x40..=0x47 => {
                let i = op & 7;
                if opsz {
                    let v = m.regs.inc32(r32(&m.regs, i));
                    w32(&mut m.regs, i, v);
                } else {
                    let v = m.regs.inc16(r16(&m.regs, i));
                    w16(&mut m.regs, i, v);
                }
            }
            0x48..=0x4f => {
                let i = op & 7;
                if opsz {
                    let v = m.regs.dec32(r32(&m.regs, i));
                    w32(&mut m.regs, i, v);
                } else {
                    let v = m.regs.dec16(r16(&m.regs, i));
                    w16(&mut m.regs, i, v);
                }
            }

            // ---- push/pop r ----
            0x50..=0x57 => {
                let i = op & 7;
                if opsz {
                    let v = r32(&m.regs, i);
                    self.push32(m, v);
                } else {
                    let v = r16(&m.regs, i);
                    self.push16(m, v);
                }
            }
            0x58..=0x5f => {
                let i = op & 7;
                if opsz {
                    let v = self.pop32(m);
                    w32(&mut m.regs, i, v);
                } else {
                    let v = self.pop16(m);
                    w16(&mut m.regs, i, v);
                }
            }

            0x60 => {
                // pusha
                let sp0 = m.regs.sp();
                if opsz {
                    unimpl!(self, m, op, "pushad");
                }
                for v in [
                    m.regs.ax(),
                    m.regs.cx(),
                    m.regs.dx(),
                    m.regs.bx(),
                    sp0,
                    m.regs.bp(),
                    m.regs.si(),
                    m.regs.di(),
                ] {
                    self.push16(m, v);
                }
            }
            0x61 => {
                // popa
                if opsz {
                    unimpl!(self, m, op, "popad");
                }
                let di = self.pop16(m);
                let si = self.pop16(m);
                let bp = self.pop16(m);
                let _sp = self.pop16(m);
                let bx = self.pop16(m);
                let dx = self.pop16(m);
                let cx = self.pop16(m);
                let ax = self.pop16(m);
                m.regs.set_di(di);
                m.regs.set_si(si);
                m.regs.set_bp(bp);
                m.regs.set_bx(bx);
                m.regs.set_dx(dx);
                m.regs.set_cx(cx);
                m.regs.set_ax(ax);
            }

            0x68 => {
                if opsz {
                    let v = self.fetch32(m);
                    self.push32(m, v);
                } else {
                    let v = self.fetch16(m);
                    self.push16(m, v);
                }
            }
            0x6a => {
                let v = self.fetch8(m) as i8;
                if opsz {
                    self.push32(m, v as u32);
                } else {
                    self.push16(m, v as u16);
                }
            }
            0x69 | 0x6b => {
                // imul r, rm, imm
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let a = self.rm_r32(m, &rm);
                    let imm = if op == 0x69 {
                        self.fetch32(m)
                    } else {
                        self.fetch8(m) as i8 as u32
                    };
                    let r = m.regs.imul32_2(a, imm);
                    w32(&mut m.regs, reg, r);
                } else {
                    let a = self.rm_r16(m, &rm);
                    let imm = if op == 0x69 {
                        self.fetch16(m)
                    } else {
                        self.fetch8(m) as i8 as u16
                    };
                    let r = m.regs.imul16_2(a, imm);
                    w16(&mut m.regs, reg, r);
                }
            }

            // ---- Jcc rel8 ----
            0x70..=0x7f => {
                let rel = self.fetch8(m) as i8;
                if cond(&m.regs, op & 0xf) {
                    self.ip = self.ip.wrapping_add(rel as u16);
                }
            }

            // ---- group 1: ALU rm, imm ----
            0x80 | 0x82 => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                let imm = self.fetch8(m);
                let a = self.rm_r8(m, &rm);
                if let Some(r) = alu8(&mut m.regs, sub, a, imm) {
                    self.rm_w8(m, &rm, r);
                }
            }
            0x81 | 0x83 => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let imm = if op == 0x81 {
                        self.fetch32(m)
                    } else {
                        self.fetch8(m) as i8 as u32
                    };
                    let a = self.rm_r32(m, &rm);
                    if let Some(r) = alu32(&mut m.regs, sub, a, imm) {
                        self.rm_w32(m, &rm, r);
                    }
                } else {
                    let imm = if op == 0x81 {
                        self.fetch16(m)
                    } else {
                        self.fetch8(m) as i8 as u16
                    };
                    let a = self.rm_r16(m, &rm);
                    if let Some(r) = alu16(&mut m.regs, sub, a, imm) {
                        self.rm_w16(m, &rm, r);
                    }
                }
            }

            0x84 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let a = self.rm_r8(m, &rm);
                m.regs.test8(a, r8(&m.regs, reg));
            }
            0x85 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let a = self.rm_r32(m, &rm);
                    m.regs.test32(a, r32(&m.regs, reg));
                } else {
                    let a = self.rm_r16(m, &rm);
                    m.regs.test16(a, r16(&m.regs, reg));
                }
            }
            0x86 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let a = self.rm_r8(m, &rm);
                let b = r8(&m.regs, reg);
                self.rm_w8(m, &rm, b);
                w8(&mut m.regs, reg, a);
            }
            0x87 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let a = self.rm_r32(m, &rm);
                    let b = r32(&m.regs, reg);
                    self.rm_w32(m, &rm, b);
                    w32(&mut m.regs, reg, a);
                } else {
                    let a = self.rm_r16(m, &rm);
                    let b = r16(&m.regs, reg);
                    self.rm_w16(m, &rm, b);
                    w16(&mut m.regs, reg, a);
                }
            }

            // ---- mov ----
            0x88 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let v = r8(&m.regs, reg);
                self.rm_w8(m, &rm, v);
            }
            0x89 => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let v = r32(&m.regs, reg);
                    self.rm_w32(m, &rm, v);
                } else {
                    let v = r16(&m.regs, reg);
                    self.rm_w16(m, &rm, v);
                }
            }
            0x8a => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let v = self.rm_r8(m, &rm);
                w8(&mut m.regs, reg, v);
            }
            0x8b => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let v = self.rm_r32(m, &rm);
                    w32(&mut m.regs, reg, v);
                } else {
                    let v = self.rm_r16(m, &rm);
                    w16(&mut m.regs, reg, v);
                }
            }
            0x8c => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let v = self.segv(m, reg);
                self.rm_w16(m, &rm, v);
            }
            0x8d => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                match rm {
                    Rm::Mem { off, .. } => {
                        if opsz {
                            w32(&mut m.regs, reg, off);
                        } else {
                            w16(&mut m.regs, reg, off as u16);
                        }
                    }
                    Rm::Reg(_) => unimpl!(self, m, op, "lea reg"),
                }
            }
            0x8e => {
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                let v = self.rm_r16(m, &rm);
                match reg {
                    0 => m.regs.es = v,
                    2 => m.regs.ss = v,
                    3 => m.regs.ds = v,
                    4 => m.regs.fs = v,
                    5 => m.regs.gs = v,
                    _ => unimpl!(self, m, op, "mov cs, rm"),
                }
            }
            0x8f => {
                let (_, _, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let v = self.pop32(m);
                    self.rm_w32(m, &rm, v);
                } else {
                    let v = self.pop16(m);
                    self.rm_w16(m, &rm, v);
                }
            }

            0x90 => {} // nop
            0x91..=0x97 => {
                let i = op & 7;
                if opsz {
                    let t = m.regs.eax;
                    m.regs.eax = r32(&m.regs, i);
                    w32(&mut m.regs, i, t);
                } else {
                    let t = m.regs.ax();
                    let v = r16(&m.regs, i);
                    m.regs.set_ax(v);
                    w16(&mut m.regs, i, t);
                }
            }

            0x98 => {
                if opsz {
                    // cwde
                    m.regs.eax = m.regs.ax() as i16 as i32 as u32;
                } else {
                    // cbw
                    m.regs.set_ax(m.regs.al() as i8 as i16 as u16);
                }
            }
            0x99 => {
                if opsz {
                    m.regs.cdq();
                } else {
                    m.regs.cwd();
                }
            }

            0x9a => {
                // lcall seg:off
                let off = self.fetch16(m);
                let seg = self.fetch16(m);
                let (cs, ip) = (self.cs, self.ip);
                self.push16(m, cs);
                self.push16(m, ip);
                self.cs = seg;
                self.ip = off;
                m.regs.cs = seg;
                self.depth += 1;
            }
            0x9b => {} // wait/fwait

            0x9c => {
                let f = flags_word(&m.regs) | ((self.iflag as u16) << 9) | self.flags_high;
                if opsz {
                    self.push32(m, f as u32);
                } else {
                    self.push16(m, f);
                }
            }
            0x9d => {
                let f = if opsz {
                    self.pop32(m) as u16
                } else {
                    self.pop16(m)
                };
                set_flags_word(&mut m.regs, f);
                self.iflag = f & 0x200 != 0;
                self.flags_high = f & 0x7000;
            }
            0x9e => {
                // sahf
                let ah = m.regs.ah();
                m.regs.cf = ah & 1 != 0;
                m.regs.pf = ah & 4 != 0;
                m.regs.af = ah & 0x10 != 0;
                m.regs.zf = ah & 0x40 != 0;
                m.regs.sf = ah & 0x80 != 0;
            }
            0x9f => {
                // lahf
                let f = (flags_word(&m.regs) & 0xd7) as u8 | 0x02;
                m.regs.set_ah(f);
            }

            // ---- mov accumulator <-> moffs ----
            0xa0 => {
                let off = if adsz {
                    self.fetch32(m)
                } else {
                    self.fetch16(m) as u32
                };
                let seg = self.resolve_seg(m, ovr, false);
                let v = m.read8(seg, off);
                m.regs.set_al(v);
            }
            0xa1 => {
                let off = if adsz {
                    self.fetch32(m)
                } else {
                    self.fetch16(m) as u32
                };
                let seg = self.resolve_seg(m, ovr, false);
                if opsz {
                    m.regs.eax = m.read32(seg, off);
                } else {
                    let v = m.read16(seg, off);
                    m.regs.set_ax(v);
                }
            }
            0xa2 => {
                let off = if adsz {
                    self.fetch32(m)
                } else {
                    self.fetch16(m) as u32
                };
                let seg = self.resolve_seg(m, ovr, false);
                m.write8(seg, off, m.regs.al());
            }
            0xa3 => {
                let off = if adsz {
                    self.fetch32(m)
                } else {
                    self.fetch16(m) as u32
                };
                let seg = self.resolve_seg(m, ovr, false);
                if opsz {
                    m.write32(seg, off, m.regs.eax);
                } else {
                    m.write16(seg, off, m.regs.ax());
                }
            }

            // ---- string ops ----
            0xa4 | 0xa5 | 0xa6 | 0xa7 | 0xaa | 0xab | 0xac | 0xad | 0xae | 0xaf => {
                if adsz {
                    unimpl!(self, m, op, "string op with 0x67");
                }
                let size: u16 = if op & 1 == 0 {
                    1
                } else if opsz {
                    4
                } else {
                    2
                };
                let kind = match op {
                    0xa4 | 0xa5 => 0u8, // movs
                    0xa6 | 0xa7 => 1,   // cmps
                    0xaa | 0xab => 2,   // stos
                    0xac | 0xad => 3,   // lods
                    _ => 4,             // scas
                };
                let delta = if m.regs.df { size.wrapping_neg() } else { size };
                let sseg = self.resolve_seg(m, ovr, false);
                let cx0 = m.regs.cx();
                loop {
                    if rep.is_some() {
                        if m.regs.cx() == 0 {
                            break;
                        }
                        m.regs.set_cx(m.regs.cx().wrapping_sub(1));
                    }
                    let si = m.regs.si() as u32;
                    let di = m.regs.di() as u32;
                    let es = m.regs.es;
                    match (kind, size) {
                        (0, 1) => {
                            let v = m.read8(sseg, si);
                            m.write8(es, di, v);
                        }
                        (0, 2) => {
                            let v = m.read16(sseg, si);
                            m.write16(es, di, v);
                        }
                        (0, _) => {
                            let v = m.read32(sseg, si);
                            m.write32(es, di, v);
                        }
                        (1, 1) => {
                            let a = m.read8(sseg, si);
                            let b = m.read8(es, di);
                            m.regs.cmp8(a, b);
                        }
                        (1, 2) => {
                            let a = m.read16(sseg, si);
                            let b = m.read16(es, di);
                            m.regs.cmp16(a, b);
                        }
                        (1, _) => {
                            let a = m.read32(sseg, si);
                            let b = m.read32(es, di);
                            m.regs.cmp32(a, b);
                        }
                        (2, 1) => m.write8(es, di, m.regs.al()),
                        (2, 2) => m.write16(es, di, m.regs.ax()),
                        (2, _) => m.write32(es, di, m.regs.eax),
                        (3, 1) => {
                            let v = m.read8(sseg, si);
                            m.regs.set_al(v);
                        }
                        (3, 2) => {
                            let v = m.read16(sseg, si);
                            m.regs.set_ax(v);
                        }
                        (3, _) => m.regs.eax = m.read32(sseg, si),
                        (4, 1) => {
                            let b = m.read8(es, di);
                            let a = m.regs.al();
                            m.regs.cmp8(a, b);
                        }
                        (4, 2) => {
                            let b = m.read16(es, di);
                            let a = m.regs.ax();
                            m.regs.cmp16(a, b);
                        }
                        _ => {
                            let b = m.read32(es, di);
                            let a = m.regs.eax;
                            m.regs.cmp32(a, b);
                        }
                    }
                    // index updates
                    match kind {
                        0 | 1 => {
                            m.regs.set_si(m.regs.si().wrapping_add(delta));
                            m.regs.set_di(m.regs.di().wrapping_add(delta));
                        }
                        3 => m.regs.set_si(m.regs.si().wrapping_add(delta)),
                        _ => m.regs.set_di(m.regs.di().wrapping_add(delta)),
                    }
                    match rep {
                        None => break,
                        Some(cont_on_zf) if kind == 1 || kind == 4 => {
                            if m.regs.zf != cont_on_zf {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                // Charge REP iterations as steps: a 64000-byte blit is 64000 instructions'
                // worth of emulated time, not 1 — keeps pacing (and wall cost) realistic.
                if rep.is_some() {
                    self.steps += cx0.wrapping_sub(m.regs.cx()) as u64;
                }
            }

            0xa8 => {
                let imm = self.fetch8(m);
                let a = m.regs.al();
                m.regs.test8(a, imm);
            }
            0xa9 => {
                if opsz {
                    let imm = self.fetch32(m);
                    let a = m.regs.eax;
                    m.regs.test32(a, imm);
                } else {
                    let imm = self.fetch16(m);
                    let a = m.regs.ax();
                    m.regs.test16(a, imm);
                }
            }

            0xb0..=0xb7 => {
                let v = self.fetch8(m);
                w8(&mut m.regs, op & 7, v);
            }
            0xb8..=0xbf => {
                if opsz {
                    let v = self.fetch32(m);
                    w32(&mut m.regs, op & 7, v);
                } else {
                    let v = self.fetch16(m);
                    w16(&mut m.regs, op & 7, v);
                }
            }

            // ---- shift group ----
            0xc0 | 0xc1 | 0xd0 | 0xd1 | 0xd2 | 0xd3 => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                let count = match op {
                    0xc0 | 0xc1 => self.fetch8(m),
                    0xd0 | 0xd1 => 1,
                    _ => m.regs.cl(),
                };
                if op & 1 == 0 {
                    let v = self.rm_r8(m, &rm);
                    match shift8(&mut m.regs, sub, v, count) {
                        Some(r) => self.rm_w8(m, &rm, r),
                        None => unimpl!(self, m, op, "shift8 sub"),
                    }
                } else if opsz {
                    let v = self.rm_r32(m, &rm);
                    match shift32(&mut m.regs, sub, v, count) {
                        Some(r) => self.rm_w32(m, &rm, r),
                        None => unimpl!(self, m, op, "rcl32/rcr32"),
                    }
                } else {
                    let v = self.rm_r16(m, &rm);
                    match shift16(&mut m.regs, sub, v, count) {
                        Some(r) => self.rm_w16(m, &rm, r),
                        None => unimpl!(self, m, op, "shift16 sub"),
                    }
                }
            }

            // ---- returns (depth-0 stops mirror the oracle's stop-at-ret) ----
            0xc2 => {
                let imm = self.fetch16(m);
                if self.depth == 0 {
                    return Some(Exit::Ret);
                }
                self.ip = self.pop16(m);
                m.regs.set_sp(m.regs.sp().wrapping_add(imm));
                self.depth -= 1;
            }
            0xc3 => {
                if self.depth == 0 {
                    return Some(Exit::Ret);
                }
                self.ip = self.pop16(m);
                self.depth -= 1;
            }
            0xca => {
                let imm = self.fetch16(m);
                if self.depth == 0 {
                    return Some(Exit::Retf);
                }
                self.ip = self.pop16(m);
                self.cs = self.pop16(m);
                m.regs.cs = self.cs;
                m.regs.set_sp(m.regs.sp().wrapping_add(imm));
                self.depth -= 1;
            }
            0xcb => {
                if self.depth == 0 {
                    return Some(Exit::Retf);
                }
                self.ip = self.pop16(m);
                self.cs = self.pop16(m);
                m.regs.cs = self.cs;
                self.depth -= 1;
            }

            0xc4 | 0xc5 => {
                // les/lds r, m
                let (_, reg, rm) = self.modrm(m, ovr, adsz);
                match rm {
                    Rm::Mem { seg, off } => {
                        if opsz {
                            let v = m.read32(seg, off);
                            let s = m.read16(seg, off.wrapping_add(4));
                            w32(&mut m.regs, reg, v);
                            if op == 0xc4 {
                                m.regs.es = s;
                            } else {
                                m.regs.ds = s;
                            }
                        } else {
                            let v = m.read16(seg, off);
                            let s = m.read16(seg, off.wrapping_add(2));
                            w16(&mut m.regs, reg, v);
                            if op == 0xc4 {
                                m.regs.es = s;
                            } else {
                                m.regs.ds = s;
                            }
                        }
                    }
                    Rm::Reg(_) => unimpl!(self, m, op, "les/lds reg"),
                }
            }

            0xc6 => {
                let (_, _, rm) = self.modrm(m, ovr, adsz);
                let v = self.fetch8(m);
                self.rm_w8(m, &rm, v);
            }
            0xc7 => {
                let (_, _, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    let v = self.fetch32(m);
                    self.rm_w32(m, &rm, v);
                } else {
                    let v = self.fetch16(m);
                    self.rm_w16(m, &rm, v);
                }
            }

            0xc8 => {
                // enter imm16, imm8 (level 0 only — mirrors the lifter)
                let frame = self.fetch16(m);
                let level = self.fetch8(m);
                if level != 0 {
                    unimpl!(self, m, op, "enter level>0");
                }
                let bp = m.regs.bp();
                self.push16(m, bp);
                m.regs.set_bp(m.regs.sp());
                m.regs.set_sp(m.regs.sp().wrapping_sub(frame));
            }
            0xc9 => {
                // leave
                m.regs.set_sp(m.regs.bp());
                let bp = self.pop16(m);
                m.regs.set_bp(bp);
            }

            0xcc => return Some(Exit::Int { vector: 3 }),
            0xcd => {
                let v = self.fetch8(m);
                return Some(Exit::Int { vector: v });
            }
            0xce => {
                if m.regs.of {
                    return Some(Exit::Int { vector: 4 });
                }
            }
            0xcf => self.emulate_iret(m),

            0xd4 => {
                // aam
                let base = self.fetch8(m);
                if base == 0 {
                    return Some(Exit::Int { vector: 0 });
                }
                let al = m.regs.al();
                m.regs.set_ah(al / base);
                let r = al % base;
                m.regs.set_al(r);
                m.regs.zf = r == 0;
                m.regs.sf = r & 0x80 != 0;
                m.regs.pf = r.count_ones() % 2 == 0;
            }
            0xd5 => {
                // aad
                let base = self.fetch8(m);
                let r = m
                    .regs
                    .al()
                    .wrapping_add(m.regs.ah().wrapping_mul(base));
                m.regs.set_al(r);
                m.regs.set_ah(0);
                m.regs.zf = r == 0;
                m.regs.sf = r & 0x80 != 0;
                m.regs.pf = r.count_ones() % 2 == 0;
            }
            0xd7 => {
                // xlat
                let seg = self.resolve_seg(m, ovr, false);
                let off = m.regs.bx().wrapping_add(m.regs.al() as u16);
                let v = m.read8(seg, off as u32);
                m.regs.set_al(v);
            }
            0xd8..=0xdf => unimpl!(self, m, op, "x87 fpu"),

            // ---- loops ----
            0xe0 | 0xe1 | 0xe2 => {
                let rel = self.fetch8(m) as i8;
                let cx = m.regs.cx().wrapping_sub(1);
                m.regs.set_cx(cx);
                let taken = cx != 0
                    && match op {
                        0xe0 => !m.regs.zf,
                        0xe1 => m.regs.zf,
                        _ => true,
                    };
                if taken {
                    self.ip = self.ip.wrapping_add(rel as u16);
                }
            }
            0xe3 => {
                let rel = self.fetch8(m) as i8;
                let z = if adsz {
                    m.regs.ecx == 0
                } else {
                    m.regs.cx() == 0
                };
                if z {
                    self.ip = self.ip.wrapping_add(rel as u16);
                }
            }

            // ---- port I/O: host boundary ----
            0xe4 => {
                let port = self.fetch8(m) as u16;
                return Some(Exit::In { port, size: 1 });
            }
            0xe5 => {
                let port = self.fetch8(m) as u16;
                return Some(Exit::In {
                    port,
                    size: if opsz { 4 } else { 2 },
                });
            }
            0xe6 => {
                let port = self.fetch8(m) as u16;
                return Some(Exit::Out {
                    port,
                    size: 1,
                    value: m.regs.al() as u32,
                });
            }
            0xe7 => {
                let port = self.fetch8(m) as u16;
                return Some(Exit::Out {
                    port,
                    size: if opsz { 4 } else { 2 },
                    value: if opsz { m.regs.eax } else { m.regs.ax() as u32 },
                });
            }
            0xec => {
                return Some(Exit::In {
                    port: m.regs.dx(),
                    size: 1,
                })
            }
            0xed => {
                return Some(Exit::In {
                    port: m.regs.dx(),
                    size: if opsz { 4 } else { 2 },
                })
            }
            0xee => {
                return Some(Exit::Out {
                    port: m.regs.dx(),
                    size: 1,
                    value: m.regs.al() as u32,
                })
            }
            0xef => {
                return Some(Exit::Out {
                    port: m.regs.dx(),
                    size: if opsz { 4 } else { 2 },
                    value: if opsz { m.regs.eax } else { m.regs.ax() as u32 },
                })
            }

            // ---- call/jmp ----
            0xe8 => {
                if opsz {
                    unimpl!(self, m, op, "call rel32");
                }
                let rel = self.fetch16(m);
                let ip = self.ip;
                self.push16(m, ip);
                self.ip = self.ip.wrapping_add(rel);
                self.depth += 1;
            }
            0xe9 => {
                if opsz {
                    unimpl!(self, m, op, "jmp rel32");
                }
                let rel = self.fetch16(m);
                self.ip = self.ip.wrapping_add(rel);
            }
            0xea => {
                let off = self.fetch16(m);
                let seg = self.fetch16(m);
                self.cs = seg;
                self.ip = off;
                m.regs.cs = seg;
            }
            0xeb => {
                let rel = self.fetch8(m) as i8;
                self.ip = self.ip.wrapping_add(rel as u16);
            }

            0xf4 => return Some(Exit::Hlt),
            0xf5 => m.regs.cf = !m.regs.cf,
            0xf8 => m.regs.cf = false,
            0xf9 => m.regs.cf = true,
            0xfa => self.iflag = false,
            0xfb => self.iflag = true,
            0xfc => m.regs.df = false,
            0xfd => m.regs.df = true,

            // ---- group 3: test/not/neg/mul/imul/div/idiv ----
            0xf6 => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                match sub {
                    0 | 1 => {
                        let imm = self.fetch8(m);
                        let a = self.rm_r8(m, &rm);
                        m.regs.test8(a, imm);
                    }
                    2 => {
                        let v = !self.rm_r8(m, &rm);
                        self.rm_w8(m, &rm, v);
                    }
                    3 => {
                        let a = self.rm_r8(m, &rm);
                        let r = m.regs.neg8(a);
                        self.rm_w8(m, &rm, r);
                    }
                    4 => {
                        let a = self.rm_r8(m, &rm);
                        m.regs.mul8(a);
                    }
                    5 => {
                        let a = self.rm_r8(m, &rm);
                        m.regs.imul8_1(a);
                    }
                    6 => {
                        let a = self.rm_r8(m, &rm);
                        m.regs.div8(a);
                    }
                    _ => {
                        let a = self.rm_r8(m, &rm);
                        m.regs.idiv8(a);
                    }
                }
            }
            0xf7 => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                if opsz {
                    match sub {
                        0 | 1 => {
                            let imm = self.fetch32(m);
                            let a = self.rm_r32(m, &rm);
                            m.regs.test32(a, imm);
                        }
                        2 => {
                            let v = !self.rm_r32(m, &rm);
                            self.rm_w32(m, &rm, v);
                        }
                        3 => {
                            let a = self.rm_r32(m, &rm);
                            let r = m.regs.neg32(a);
                            self.rm_w32(m, &rm, r);
                        }
                        4 => {
                            let a = self.rm_r32(m, &rm);
                            m.regs.mul32(a);
                        }
                        5 => {
                            let a = self.rm_r32(m, &rm);
                            m.regs.imul32_1(a);
                        }
                        6 => {
                            let a = self.rm_r32(m, &rm);
                            m.regs.div32(a);
                        }
                        _ => {
                            let a = self.rm_r32(m, &rm);
                            m.regs.idiv32(a);
                        }
                    }
                } else {
                    match sub {
                        0 | 1 => {
                            let imm = self.fetch16(m);
                            let a = self.rm_r16(m, &rm);
                            m.regs.test16(a, imm);
                        }
                        2 => {
                            let v = !self.rm_r16(m, &rm);
                            self.rm_w16(m, &rm, v);
                        }
                        3 => {
                            let a = self.rm_r16(m, &rm);
                            let r = m.regs.neg16(a);
                            self.rm_w16(m, &rm, r);
                        }
                        4 => {
                            let a = self.rm_r16(m, &rm);
                            m.regs.mul16(a);
                        }
                        5 => {
                            let a = self.rm_r16(m, &rm);
                            m.regs.imul16_1(a);
                        }
                        6 => {
                            let a = self.rm_r16(m, &rm);
                            m.regs.div16(a);
                        }
                        _ => {
                            let a = self.rm_r16(m, &rm);
                            m.regs.idiv16(a);
                        }
                    }
                }
            }

            // ---- group 4/5 ----
            0xfe => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                let a = self.rm_r8(m, &rm);
                let r = match sub {
                    0 => m.regs.inc8(a),
                    1 => m.regs.dec8(a),
                    _ => unimpl!(self, m, op, "grp4 sub"),
                };
                self.rm_w8(m, &rm, r);
            }
            0xff => {
                let (_, sub, rm) = self.modrm(m, ovr, adsz);
                match sub {
                    0 => {
                        if opsz {
                            let a = self.rm_r32(m, &rm);
                            let r = m.regs.inc32(a);
                            self.rm_w32(m, &rm, r);
                        } else {
                            let a = self.rm_r16(m, &rm);
                            let r = m.regs.inc16(a);
                            self.rm_w16(m, &rm, r);
                        }
                    }
                    1 => {
                        if opsz {
                            let a = self.rm_r32(m, &rm);
                            let r = m.regs.dec32(a);
                            self.rm_w32(m, &rm, r);
                        } else {
                            let a = self.rm_r16(m, &rm);
                            let r = m.regs.dec16(a);
                            self.rm_w16(m, &rm, r);
                        }
                    }
                    2 => {
                        // call rm (near indirect)
                        let t = self.rm_r16(m, &rm);
                        let ip = self.ip;
                        self.push16(m, ip);
                        self.ip = t;
                        self.depth += 1;
                    }
                    3 => {
                        // lcall m16:16 (far indirect)
                        match rm {
                            Rm::Mem { seg, off } => {
                                let t_off = m.read16(seg, off);
                                let t_seg = m.read16(seg, off.wrapping_add(2));
                                let (cs, ip) = (self.cs, self.ip);
                                self.push16(m, cs);
                                self.push16(m, ip);
                                self.cs = t_seg;
                                self.ip = t_off;
                                m.regs.cs = t_seg;
                                self.depth += 1;
                            }
                            Rm::Reg(_) => unimpl!(self, m, op, "lcall reg"),
                        }
                    }
                    4 => self.ip = self.rm_r16(m, &rm),
                    5 => match rm {
                        Rm::Mem { seg, off } => {
                            let t_off = m.read16(seg, off);
                            let t_seg = m.read16(seg, off.wrapping_add(2));
                            self.cs = t_seg;
                            self.ip = t_off;
                            m.regs.cs = t_seg;
                        }
                        Rm::Reg(_) => unimpl!(self, m, op, "ljmp reg"),
                    },
                    6 => {
                        if opsz {
                            let v = self.rm_r32(m, &rm);
                            self.push32(m, v);
                        } else {
                            let v = self.rm_r16(m, &rm);
                            self.push16(m, v);
                        }
                    }
                    _ => unimpl!(self, m, op, "grp5 sub 7"),
                }
            }

            // ---- 0x0F extended map ----
            0x0f => {
                let ext = self.fetch8(m);
                match ext {
                    0x80..=0x8f => {
                        if opsz {
                            unimpl!(self, m, ext, "jcc rel32");
                        }
                        let rel = self.fetch16(m);
                        if cond(&m.regs, ext & 0xf) {
                            self.ip = self.ip.wrapping_add(rel);
                        }
                    }
                    0x90..=0x9f => {
                        let (_, _, rm) = self.modrm(m, ovr, adsz);
                        let v = cond(&m.regs, ext & 0xf) as u8;
                        self.rm_w8(m, &rm, v);
                    }
                    0xa0 => {
                        let v = m.regs.fs;
                        self.push16(m, v);
                    }
                    0xa1 => m.regs.fs = self.pop16(m),
                    0xa8 => {
                        let v = m.regs.gs;
                        self.push16(m, v);
                    }
                    0xa9 => m.regs.gs = self.pop16(m),

                    // bt/bts/btr/btc rm, r
                    0xa3 | 0xab | 0xb3 | 0xbb => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let kind = (ext >> 3) & 3; // a3->0 bt, ab->1 bts, b3->2 btr, bb->3 btc
                        let width: i32 = if opsz { 32 } else { 16 };
                        let bitidx = if opsz {
                            r32(&m.regs, reg) as i32
                        } else {
                            r16(&m.regs, reg) as i16 as i32
                        };
                        self.bit_op(m, &rm, kind, bitidx, width, true);
                    }
                    0xba => {
                        // group 8: bt/bts/btr/btc rm, imm8
                        let (_, sub, rm) = self.modrm(m, ovr, adsz);
                        let imm = self.fetch8(m);
                        if sub < 4 {
                            unimpl!(self, m, ext, "grp8 sub<4");
                        }
                        let width: i32 = if opsz { 32 } else { 16 };
                        let bitidx = (imm as i32) & (width - 1);
                        self.bit_op(m, &rm, sub - 4, bitidx, width, false);
                    }

                    // shld/shrd
                    0xa4 | 0xa5 | 0xac | 0xad => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let count = if ext & 1 == 0 {
                            self.fetch8(m)
                        } else {
                            m.regs.cl()
                        } & 0x1f;
                        let left = ext < 0xa8;
                        if count == 0 {
                        } else if opsz {
                            let a = self.rm_r32(m, &rm);
                            let b = r32(&m.regs, reg);
                            let wide = if left {
                                ((a as u64) << 32) | b as u64
                            } else {
                                ((b as u64) << 32) | a as u64
                            };
                            let (r, cf) = if left {
                                let r = (wide << count >> 32) as u32;
                                (r, (a >> (32 - count)) & 1 != 0)
                            } else {
                                let r = (wide >> count) as u32;
                                (r, (a >> (count - 1)) & 1 != 0)
                            };
                            self.rm_w32(m, &rm, r);
                            m.regs.cf = cf;
                            m.regs.zf = r == 0;
                            m.regs.sf = r & 0x8000_0000 != 0;
                            m.regs.pf = (r as u8).count_ones() % 2 == 0;
                        } else {
                            let a = self.rm_r16(m, &rm);
                            let b = r16(&m.regs, reg);
                            let wide = if left {
                                ((a as u32) << 16) | b as u32
                            } else {
                                ((b as u32) << 16) | a as u32
                            };
                            let c = count.min(16); // >16 is undefined on 386; keep deterministic
                            let (r, cf) = if left {
                                let r = (wide << c >> 16) as u16;
                                (r, (a >> (16 - c)) & 1 != 0)
                            } else {
                                let r = (wide >> c) as u16;
                                (r, (a >> (c - 1)) & 1 != 0)
                            };
                            self.rm_w16(m, &rm, r);
                            m.regs.cf = cf;
                            m.regs.zf = r == 0;
                            m.regs.sf = r & 0x8000 != 0;
                            m.regs.pf = (r as u8).count_ones() % 2 == 0;
                        }
                    }

                    0xaf => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        if opsz {
                            let a = r32(&m.regs, reg);
                            let b = self.rm_r32(m, &rm);
                            let r = m.regs.imul32_2(a, b);
                            w32(&mut m.regs, reg, r);
                        } else {
                            let a = r16(&m.regs, reg);
                            let b = self.rm_r16(m, &rm);
                            let r = m.regs.imul16_2(a, b);
                            w16(&mut m.regs, reg, r);
                        }
                    }

                    // lss/lfs/lgs
                    0xb2 | 0xb4 | 0xb5 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        match rm {
                            Rm::Mem { seg, off } => {
                                let v = m.read16(seg, off);
                                let s = m.read16(seg, off.wrapping_add(2));
                                w16(&mut m.regs, reg, v);
                                match ext {
                                    0xb2 => m.regs.ss = s,
                                    0xb4 => m.regs.fs = s,
                                    _ => m.regs.gs = s,
                                }
                            }
                            Rm::Reg(_) => unimpl!(self, m, ext, "lss/lfs/lgs reg"),
                        }
                    }

                    // movzx/movsx
                    0xb6 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let v = self.rm_r8(m, &rm) as u32;
                        if opsz {
                            w32(&mut m.regs, reg, v);
                        } else {
                            w16(&mut m.regs, reg, v as u16);
                        }
                    }
                    0xb7 => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let v = self.rm_r16(m, &rm);
                        if opsz {
                            w32(&mut m.regs, reg, v as u32);
                        } else {
                            w16(&mut m.regs, reg, v);
                        }
                    }
                    0xbe => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let v = self.rm_r8(m, &rm) as i8;
                        if opsz {
                            w32(&mut m.regs, reg, v as i32 as u32);
                        } else {
                            w16(&mut m.regs, reg, v as i16 as u16);
                        }
                    }
                    0xbf => {
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        let v = self.rm_r16(m, &rm) as i16;
                        if opsz {
                            w32(&mut m.regs, reg, v as i32 as u32);
                        } else {
                            w16(&mut m.regs, reg, v as u16);
                        }
                    }

                    0xbc => {
                        // bsf
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        if opsz {
                            let src = self.rm_r32(m, &rm);
                            m.regs.zf = src == 0;
                            if src != 0 {
                                w32(&mut m.regs, reg, src.trailing_zeros());
                            }
                        } else {
                            let src = self.rm_r16(m, &rm);
                            let cur = r16(&m.regs, reg);
                            let r = m.regs.bsf16(src, cur);
                            w16(&mut m.regs, reg, r);
                        }
                    }
                    0xbd => {
                        // bsr
                        let (_, reg, rm) = self.modrm(m, ovr, adsz);
                        if opsz {
                            let src = self.rm_r32(m, &rm);
                            m.regs.zf = src == 0;
                            if src != 0 {
                                w32(&mut m.regs, reg, 31 - src.leading_zeros());
                            }
                        } else {
                            let src = self.rm_r16(m, &rm);
                            m.regs.zf = src == 0;
                            if src != 0 {
                                w16(&mut m.regs, reg, 15 - src.leading_zeros() as u16);
                            }
                        }
                    }

                    _ => unimpl!(self, m, ext, "0x0f ext"),
                }
            }

            _ => unimpl!(self, m, op, "opcode"),
        }
        None
    }

    /// bt/bts/btr/btc on a decoded rm. `kind`: 0=bt 1=bts 2=btr 3=btc. For the register-source
    /// memory form (`reg_form`), the bit index addresses a bit STRING: the effective byte is
    /// `off + bitidx>>3` (sign-aware), matching the CPU's out-of-operand bit addressing.
    fn bit_op(&self, m: &mut Machine, rm: &Rm, kind: u8, bitidx: i32, width: i32, reg_form: bool) {
        match rm {
            Rm::Reg(i) => {
                let b = (bitidx & (width - 1)) as u32;
                if width == 32 {
                    let v = r32(&m.regs, *i);
                    m.regs.cf = (v >> b) & 1 != 0;
                    let nv = match kind {
                        1 => v | (1 << b),
                        2 => v & !(1 << b),
                        3 => v ^ (1 << b),
                        _ => v,
                    };
                    w32(&mut m.regs, *i, nv);
                } else {
                    let v = r16(&m.regs, *i);
                    m.regs.cf = (v >> b) & 1 != 0;
                    let nv = match kind {
                        1 => v | (1 << b),
                        2 => v & !(1 << b),
                        3 => v ^ (1 << b),
                        _ => v,
                    };
                    w16(&mut m.regs, *i, nv);
                }
            }
            Rm::Mem { seg, off } => {
                let (delta, bit) = if reg_form {
                    (bitidx.div_euclid(8), bitidx.rem_euclid(8) as u32)
                } else {
                    ((bitidx / 8), (bitidx % 8) as u32)
                };
                let addr = off.wrapping_add(delta as u32);
                let v = m.read8(*seg, addr);
                m.regs.cf = (v >> bit) & 1 != 0;
                let nv = match kind {
                    1 => v | (1 << bit),
                    2 => v & !(1 << bit),
                    3 => v ^ (1 << bit),
                    _ => v,
                };
                if kind != 0 {
                    m.write8(*seg, addr, nv);
                }
            }
        }
    }
}
