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

/// The 8086 register file (16-bit general/segment registers + FLAGS). 8-bit halves are accessed
/// through methods so `al`/`ah` stay consistent with `ax`, matching the hardware aliasing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs {
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    pub si: u16,
    pub di: u16,
    pub bp: u16,
    pub sp: u16,
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub ss: u16,
    pub fs: u16,
    pub gs: u16,
    /// Carry flag (bit 0 of FLAGS). Only the flags the lifts actually use are modelled explicitly;
    /// extend as needed and keep them oracle-verified.
    pub cf: bool,
    pub zf: bool,
    pub sf: bool,
    pub of: bool,
    pub pf: bool,
    pub af: bool,
    pub df: bool,
}

macro_rules! byte_halves {
    ($lo:ident, $set_lo:ident, $hi:ident, $set_hi:ident, $reg:ident) => {
        #[inline]
        pub fn $lo(&self) -> u8 {
            self.$reg as u8
        }
        #[inline]
        pub fn $set_lo(&mut self, v: u8) {
            self.$reg = (self.$reg & 0xff00) | v as u16;
        }
        #[inline]
        pub fn $hi(&self) -> u8 {
            (self.$reg >> 8) as u8
        }
        #[inline]
        pub fn $set_hi(&mut self, v: u8) {
            self.$reg = (self.$reg & 0x00ff) | ((v as u16) << 8);
        }
    };
}

impl Regs {
    byte_halves!(al, set_al, ah, set_ah, ax);
    byte_halves!(bl, set_bl, bh, set_bh, bx);
    byte_halves!(cl, set_cl, ch, set_ch, cx);
    byte_halves!(dl, set_dl, dh, set_dh, dx);

    /// 16-bit `ADD` with exact 8086 flag semantics: returns the truncated result and sets
    /// `cf/pf/af/zf/sf/of` on `self`. Reused by every lifted arithmetic instruction so flag
    /// state stays bit-exact (a caller may branch on it). PF is even-parity of the low byte.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_halves_alias_the_word_registers() {
        let mut r = Regs::default();
        r.ax = 0x1234;
        assert_eq!((r.al(), r.ah()), (0x34, 0x12));
        r.set_al(0xAB);
        assert_eq!(r.ax, 0x12AB);
        r.set_ah(0xCD);
        assert_eq!(r.ax, 0xCDAB);
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
