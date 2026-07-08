//! The shared machine state for the 1-to-1 static recompilation of BLOODPRG.EXE.
//!
//! Path B (see re/tools/README_oracle.md): every DOS function is lifted to a Rust function that
//! operates on this [`Machine`] — the 8086 register/flag file plus a flat 1 MB real-mode memory
//! image — reading and writing exactly the bytes and registers the original code does. Each lift
//! is verified bit-exact against the real binary by the Unicorn oracle (fuzzed input state →
//! output state vectors). When every function is verified and composed in the binary's call
//! graph, the whole program runs identically **by construction**.
//!
//! This is deliberately NOT idiomatic — it mirrors the CPU. Idiomatic Rust lives in the engine
//! crate; this module exists only to be provably identical to the DOS binary.

/// The 80386 register file: 32-bit general registers (`eax`..`esp`) with 16-bit (`ax`) and 8-bit
/// (`al`/`ah`) sub-register accessors that alias them exactly like the hardware, plus segment
/// registers and the arithmetic flags. BLOODPRG is 386 code (0x66/0x67 prefixes, `eax`/`esi`…),
/// so registers are 32-bit; a 16-bit op writes the low word and leaves the high word intact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub ss: u16,
    pub fs: u16,
    pub gs: u16,
    /// Arithmetic flags. Only the flags the lifts use are modelled; extend as needed and keep
    /// them oracle-verified. Instructions leaving a flag *architecturally undefined* (e.g. OF/AF
    /// after a multi-bit shift) still assign it deterministically here, but such flags are not
    /// asserted in the oracle tests (the real program never depends on them).
    pub cf: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
    pub pf: bool,
    pub af: bool,
    pub df: bool,
}

macro_rules! word_reg {
    ($w:ident, $set_w:ident, $lo:ident, $set_lo:ident, $hi:ident, $set_hi:ident, $e:ident) => {
        #[inline]
        pub fn $w(&self) -> u16 {
            self.$e as u16
        }
        #[inline]
        pub fn $set_w(&mut self, v: u16) {
            self.$e = (self.$e & 0xffff_0000) | v as u32;
        }
        #[inline]
        pub fn $lo(&self) -> u8 {
            self.$e as u8
        }
        #[inline]
        pub fn $set_lo(&mut self, v: u8) {
            self.$e = (self.$e & 0xffff_ff00) | v as u32;
        }
        #[inline]
        pub fn $hi(&self) -> u8 {
            (self.$e >> 8) as u8
        }
        #[inline]
        pub fn $set_hi(&mut self, v: u8) {
            self.$e = (self.$e & 0xffff_00ff) | ((v as u32) << 8);
        }
    };
}

macro_rules! word_only {
    ($w:ident, $set_w:ident, $e:ident) => {
        #[inline]
        pub fn $w(&self) -> u16 {
            self.$e as u16
        }
        #[inline]
        pub fn $set_w(&mut self, v: u16) {
            self.$e = (self.$e & 0xffff_0000) | v as u32;
        }
    };
}

impl Regs {
    word_reg!(ax, set_ax, al, set_al, ah, set_ah, eax);
    word_reg!(bx, set_bx, bl, set_bl, bh, set_bh, ebx);
    word_reg!(cx, set_cx, cl, set_cl, ch, set_ch, ecx);
    word_reg!(dx, set_dx, dl, set_dl, dh, set_dh, edx);
    word_only!(si, set_si, esi);
    word_only!(di, set_di, edi);
    word_only!(bp, set_bp, ebp);
    word_only!(sp, set_sp, esp);

    /// 16-bit `ADD` with exact 8086 flag semantics: returns the truncated result and sets
    /// `cf/pf/af/zf/sf/of`. Reused by every lifted arithmetic instruction so flag state stays
    /// bit-exact. PF is even-parity of the low byte.
    pub fn add16(&mut self, a: u16, b: u16) -> u16 {
        let full = a as u32 + b as u32;
        let r = full as u16;
        self.cf = full > 0xffff;
        self.af = (a & 0xf) + (b & 0xf) > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 16-bit `SUB` with exact 8086 flags (borrow in CF/AF). Returns the truncated difference.
    pub fn sub16(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_sub(b);
        self.cf = a < b;
        self.af = (a & 0xf) < (b & 0xf);
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 16-bit `AND`: clears CF/OF, sets ZF/SF/PF. AF undefined (assigned false, not asserted).
    pub fn and16(&mut self, a: u16, b: u16) -> u16 {
        let r = a & b;
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
        r
    }

    /// 16-bit `OR`: clears CF/OF, sets ZF/SF/PF. AF undefined (assigned false, not asserted).
    pub fn or16(&mut self, a: u16, b: u16) -> u16 {
        let r = a | b;
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
        r
    }

    /// 16-bit `XOR`: returns `a ^ b`, clears CF/OF, sets ZF/SF/PF from the result. AF undefined
    /// (assigned false, not oracle-asserted).
    pub fn xor16(&mut self, a: u16, b: u16) -> u16 {
        let r = a ^ b;
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
        r
    }

    /// 8-bit `CMP` (a - b, result discarded): sets all six flags exactly like `SUB`. Used for
    /// the many `cmp byte …` branch conditions.
    pub fn cmp8(&mut self, a: u8, b: u8) {
        let r = a.wrapping_sub(b);
        self.cf = a < b;
        self.af = (a & 0xf) < (b & 0xf);
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
    }

    /// 16-bit `CMP` (a - b, discarded): sets all six flags like `SUB`.
    pub fn cmp16(&mut self, a: u16, b: u16) {
        self.sub16(a, b);
    }

    /// 16-bit `TEST` (a & b, discarded): clears CF/OF, sets ZF/SF/PF. AF undefined.
    pub fn test16(&mut self, a: u16, b: u16) {
        let r = a & b;
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }

    /// 8-bit `ADD` with exact flags.
    pub fn add8(&mut self, a: u8, b: u8) -> u8 {
        let full = a as u16 + b as u16;
        let r = full as u8;
        self.cf = full > 0xff;
        self.af = (a & 0xf) + (b & 0xf) > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        r
    }

    /// 8-bit `SUB` with exact flags.
    pub fn sub8(&mut self, a: u8, b: u8) -> u8 {
        let r = a.wrapping_sub(b);
        self.cf = a < b;
        self.af = (a & 0xf) < (b & 0xf);
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        r
    }

    /// 8-bit `AND`/`OR`/`XOR`: clear CF/OF, set ZF/SF/PF; AF undefined.
    fn logic8_flags(&mut self, r: u8) {
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        self.af = false;
    }
    pub fn and8(&mut self, a: u8, b: u8) -> u8 {
        let r = a & b;
        self.logic8_flags(r);
        r
    }
    pub fn or8(&mut self, a: u8, b: u8) -> u8 {
        let r = a | b;
        self.logic8_flags(r);
        r
    }
    pub fn xor8(&mut self, a: u8, b: u8) -> u8 {
        let r = a ^ b;
        self.logic8_flags(r);
        r
    }

    /// 8-bit `TEST` (a & b, result discarded): clears CF/OF, sets ZF/SF/PF from the AND. AF is
    /// undefined (assigned false here, not oracle-asserted).
    pub fn test8(&mut self, a: u8, b: u8) {
        let r = a & b;
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        self.af = false;
    }

    /// 16-bit `INC` (a + 1). Sets ZF/SF/PF/OF/AF; **CF is not affected** (unlike ADD).
    pub fn inc16(&mut self, a: u16) -> u16 {
        let r = a.wrapping_add(1);
        self.af = (a & 0xf) + 1 > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = a == 0x7fff;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 16-bit `DEC` (a - 1). Sets ZF/SF/PF/OF/AF; **CF is not affected**.
    pub fn dec16(&mut self, a: u16) -> u16 {
        let r = a.wrapping_sub(1);
        self.af = a & 0xf == 0;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = a == 0x8000;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 16-bit `SHR` by `count` (logical). CF = last bit out; ZF/SF/PF from the result. OF defined
    /// only for count==1 (= MSB of the original); undefined otherwise (assigned, not asserted).
    pub fn shr16(&mut self, val: u16, count: u8) -> u16 {
        let count = count & 0x1f;
        if count == 0 {
            return val;
        }
        let mut r = val;
        let mut cf = false;
        for _ in 0..count {
            cf = r & 1 != 0;
            r >>= 1;
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = val & 0x8000 != 0;
        r
    }

    /// 16-bit `SHL` by `count` (386: count masked to 5 bits). Sets the DEFINED flags exactly:
    /// CF = last bit shifted out, and ZF/SF/PF from the result. OF is defined only for count==1
    /// (OF = SF xor CF); AF is undefined — both are assigned here but NOT oracle-asserted for
    /// count>1. A count of 0 changes no flags.
    pub fn shl16(&mut self, val: u16, count: u8) -> u16 {
        let count = count & 0x1f;
        if count == 0 {
            return val;
        }
        let mut r = val;
        let mut cf = false;
        for _ in 0..count {
            cf = r & 0x8000 != 0;
            r = r.wrapping_shl(1);
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = (r & 0x8000 != 0) != cf; // exact for count==1; deterministic otherwise
        r
    }
}

/// Flat real-mode memory + registers. Addressing is `seg*16 + off` (20-bit, wraps at 1 MB like
/// the 8086's segment arithmetic — high-memory area aside, which BLOODPRG doesn't use).
pub struct Machine {
    pub regs: Regs,
    pub mem: Vec<u8>,
}

pub const MEM_SIZE: usize = 0x10_0000; // 1 MB

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            regs: Regs::default(),
            mem: vec![0u8; MEM_SIZE],
        }
    }

    /// Linear address for a real-mode `seg:off` pair, wrapped to the 1 MB image.
    #[inline]
    pub fn lin(seg: u16, off: u16) -> usize {
        ((seg as usize) * 16 + off as usize) & (MEM_SIZE - 1)
    }

    #[inline]
    pub fn read8(&self, seg: u16, off: u16) -> u8 {
        self.mem[Self::lin(seg, off)]
    }
    #[inline]
    pub fn write8(&mut self, seg: u16, off: u16, v: u8) {
        self.mem[Self::lin(seg, off)] = v;
    }
    #[inline]
    pub fn read16(&self, seg: u16, off: u16) -> u16 {
        u16::from_le_bytes([self.read8(seg, off), self.read8(seg, off.wrapping_add(1))])
    }
    #[inline]
    pub fn write16(&mut self, seg: u16, off: u16, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.write8(seg, off, lo);
        self.write8(seg, off.wrapping_add(1), hi);
    }
    #[inline]
    pub fn read32(&self, seg: u16, off: u16) -> u32 {
        (self.read16(seg, off) as u32) | ((self.read16(seg, off.wrapping_add(2)) as u32) << 16)
    }
    #[inline]
    pub fn write32(&mut self, seg: u16, off: u16, v: u32) {
        self.write16(seg, off, v as u16);
        self.write16(seg, off.wrapping_add(2), (v >> 16) as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_halves_alias_the_word_registers() {
        let mut r = Regs::default();
        r.eax = 0xDEAD_1234;
        assert_eq!(r.ax(), 0x1234);
        assert_eq!((r.al(), r.ah()), (0x34, 0x12));
        r.set_al(0xAB);
        assert_eq!(r.ax(), 0x12AB);
        assert_eq!(r.eax, 0xDEAD_12AB, "16/8-bit writes preserve the high dword");
        r.set_ax(0x5678);
        assert_eq!(r.eax, 0xDEAD_5678);
    }

    #[test]
    fn segmented_memory_addressing_wraps_at_1mb() {
        let mut m = Machine::new();
        m.write16(0x1000, 0x0004, 0xBEEF); // linear 0x10004
        assert_eq!(m.read16(0x1000, 0x0004), 0xBEEF);
        assert_eq!(m.read8(0x1000, 0x0005), 0xBE);
        assert_eq!(Machine::lin(0xFFFF, 0x0010), 0x0000); // 0xFFFF0 + 0x10 wraps to 0
    }
}
