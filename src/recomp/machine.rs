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

    /// 16-bit `NEG` (0 - a). Flags as `SUB(0, a)`: CF set unless a==0.
    pub fn neg16(&mut self, a: u16) -> u16 {
        self.sub16(0, a)
    }

    /// 32-bit `ADD`/`SUB`/`AND`/`OR`/`XOR`/`SHL`/`SHR`/`CMP` with exact 386 flags. SF/OF use bit
    /// 31; PF is even-parity of the low byte; shift OF is exact only for count==1.
    pub fn add32(&mut self, a: u32, b: u32) -> u32 {
        let full = a as u64 + b as u64;
        let r = full as u32;
        self.cf = full > 0xffff_ffff;
        self.af = (a & 0xf) + (b & 0xf) > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }
    pub fn sub32(&mut self, a: u32, b: u32) -> u32 {
        let r = a.wrapping_sub(b);
        self.cf = a < b;
        self.af = (a & 0xf) < (b & 0xf);
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }
    pub fn cmp32(&mut self, a: u32, b: u32) {
        self.sub32(a, b);
    }
    fn logic32_flags(&mut self, r: u32) {
        self.cf = false;
        self.of = false;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }
    pub fn and32(&mut self, a: u32, b: u32) -> u32 {
        let r = a & b;
        self.logic32_flags(r);
        r
    }
    pub fn or32(&mut self, a: u32, b: u32) -> u32 {
        let r = a | b;
        self.logic32_flags(r);
        r
    }
    pub fn xor32(&mut self, a: u32, b: u32) -> u32 {
        let r = a ^ b;
        self.logic32_flags(r);
        r
    }
    pub fn shl32(&mut self, val: u32, count: u8) -> u32 {
        let count = count & 0x1f;
        if count == 0 {
            return val;
        }
        let mut r = val;
        let mut cf = false;
        for _ in 0..count {
            cf = r & 0x8000_0000 != 0;
            r = r.wrapping_shl(1);
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = (r & 0x8000_0000 != 0) != cf;
        r
    }
    pub fn shr32(&mut self, val: u32, count: u8) -> u32 {
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
        self.sf = r & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = val & 0x8000_0000 != 0;
        r
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

    /// 8-bit `INC`/`DEC` (CF not affected).
    pub fn inc8(&mut self, a: u8) -> u8 {
        let r = a.wrapping_add(1);
        self.af = (a & 0xf) + 1 > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = a == 0x7f;
        self.pf = r.count_ones() % 2 == 0;
        r
    }
    pub fn dec8(&mut self, a: u8) -> u8 {
        let r = a.wrapping_sub(1);
        self.af = a & 0xf == 0;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = a == 0x80;
        self.pf = r.count_ones() % 2 == 0;
        r
    }

    /// 32-bit `INC`/`DEC` (CF not affected).
    pub fn inc32(&mut self, a: u32) -> u32 {
        let r = a.wrapping_add(1);
        self.af = (a & 0xf) + 1 > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = a == 0x7fff_ffff;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }
    pub fn dec32(&mut self, a: u32) -> u32 {
        let r = a.wrapping_sub(1);
        self.af = a & 0xf == 0;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = a == 0x8000_0000;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 8-bit / 32-bit `NEG` (0 - a).
    pub fn neg8(&mut self, a: u8) -> u8 {
        self.sub8(0, a)
    }
    pub fn neg32(&mut self, a: u32) -> u32 {
        self.sub32(0, a)
    }

    /// 8-bit `SHL`/`SHR` (count masked to 5 bits like the 386).
    pub fn shl8(&mut self, val: u8, count: u8) -> u8 {
        let count = count & 0x1f;
        if count == 0 {
            return val;
        }
        let mut r = val;
        let mut cf = false;
        for _ in 0..count {
            cf = r & 0x80 != 0;
            r = r.wrapping_shl(1);
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        self.of = (r & 0x80 != 0) != cf;
        r
    }
    pub fn shr8(&mut self, val: u8, count: u8) -> u8 {
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
        self.sf = r & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        self.of = val & 0x80 != 0;
        r
    }

    /// 8-bit `SBB` (a - b - CF). CF/AF/OF as subtract-with-borrow; ZF/SF/PF from result.
    pub fn sbb8(&mut self, a: u8, b: u8) -> u8 {
        let bin = self.cf as u16;
        let full = a as u16 + b as u16 + bin; // for borrow detection
        let r = a.wrapping_sub(b).wrapping_sub(bin as u8);
        self.cf = full > 0xff; // borrow: a < b + cin
        self.af = (a & 0xf) < (b & 0xf) + bin as u8;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        r
    }
    /// 16-bit `SBB` (a - b - CF).
    pub fn sbb16(&mut self, a: u16, b: u16) -> u16 {
        let bin = self.cf as u32;
        let full = a as u32 + b as u32 + bin;
        let r = a.wrapping_sub(b).wrapping_sub(bin as u16);
        self.cf = full > 0xffff;
        self.af = (a & 0xf) < (b & 0xf) + bin as u16;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 8-bit / 16-bit `ADC` (a + b + CF) with exact carry/overflow flags.
    pub fn adc8(&mut self, a: u8, b: u8) -> u8 {
        let cin = self.cf as u16;
        let full = a as u16 + b as u16 + cin;
        let r = full as u8;
        self.cf = full > 0xff;
        self.af = (a & 0xf) as u16 + (b & 0xf) as u16 + cin > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x80 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x80 != 0;
        self.pf = r.count_ones() % 2 == 0;
        r
    }
    pub fn adc16(&mut self, a: u16, b: u16) -> u16 {
        let cin = self.cf as u32;
        let full = a as u32 + b as u32 + cin;
        let r = full as u16;
        self.cf = full > 0xffff;
        self.af = (a & 0xf) as u32 + (b & 0xf) as u32 + cin > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 8-bit unsigned `MUL`: AX = AL * src. CF=OF = (AH != 0); ZF/SF/PF undefined (assigned).
    pub fn mul8(&mut self, src: u8) {
        let r = self.al() as u16 * src as u16;
        self.set_ax(r);
        let of = r & 0xff00 != 0;
        self.cf = of;
        self.of = of;
        self.zf = r == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }
    /// 16-bit unsigned `MUL`: DX:AX = AX * src. CF=OF = (DX != 0).
    pub fn mul16(&mut self, src: u16) {
        let r = self.ax() as u32 * src as u32;
        self.set_ax(r as u16);
        self.set_dx((r >> 16) as u16);
        let of = r & 0xffff_0000 != 0;
        self.cf = of;
        self.of = of;
        self.zf = (r as u16) == 0;
        self.sf = r & 0x8000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }

    /// 8-bit / 16-bit unsigned `DIV`. AL/AX = quotient, AH/DX = remainder. A zero divisor or a
    /// quotient overflow is #DE on real hardware — the oracle discards those fuzzed vectors, so
    /// here we simply leave state unchanged (never reached by a kept vector). Flags undefined.
    pub fn div8(&mut self, src: u8) {
        if src == 0 {
            return;
        }
        let n = self.ax();
        let q = n / src as u16;
        if q > 0xff {
            return;
        }
        self.set_al(q as u8);
        self.set_ah((n % src as u16) as u8);
    }
    pub fn div16(&mut self, src: u16) {
        if src == 0 {
            return;
        }
        let n = ((self.dx() as u32) << 16) | self.ax() as u32;
        let q = n / src as u32;
        if q > 0xffff {
            return;
        }
        self.set_ax(q as u16);
        self.set_dx((n % src as u32) as u16);
    }

    /// 8-bit / 16-bit one-operand signed `IMUL`: AX / DX:AX = accumulator * src (signed). CF=OF
    /// set when the full product doesn't fit in the low half (sign-extended); other flags undefined.
    pub fn imul8_1(&mut self, src: u8) {
        let r = (self.al() as i8 as i16) * (src as i8 as i16);
        self.set_ax(r as u16);
        let of = r != (r as i8 as i16);
        self.cf = of;
        self.of = of;
    }
    pub fn imul16_1(&mut self, src: u16) {
        let r = (self.ax() as i16 as i32) * (src as i16 as i32);
        self.set_ax(r as u16);
        self.set_dx((r >> 16) as u16);
        let of = r != (r as i16 as i32);
        self.cf = of;
        self.of = of;
    }
    /// Two/three-operand signed `IMUL` (result truncated to 16 bits). CF=OF on overflow.
    pub fn imul16_2(&mut self, a: u16, b: u16) -> u16 {
        let full = (a as i16 as i32) * (b as i16 as i32);
        let r = full as u16;
        let of = full != (r as i16 as i32);
        self.cf = of;
        self.of = of;
        r
    }

    /// 16-bit `BSF` (bit scan forward): if `src==0`, ZF=1 and the destination is left unchanged;
    /// otherwise ZF=0 and the destination becomes the index of the lowest set bit. Returns the new
    /// destination value (the caller passes the current one through for the src==0 case).
    pub fn bsf16(&mut self, src: u16, dst_cur: u16) -> u16 {
        if src == 0 {
            self.zf = true;
            dst_cur
        } else {
            self.zf = false;
            src.trailing_zeros() as u16
        }
    }

    /// 8/16-bit rotates (count masked to 5 bits like the 386). `ROL`/`ROR` set CF to the bit
    /// rotated into the other end and OF (for count==1) to CF xor MSB; `RCL`/`RCR` rotate through
    /// CF (a 9-/17-bit rotation). AF/SF/ZF/PF unaffected by rotates.
    pub fn rol16(&mut self, val: u16, count: u8) -> u16 {
        let c = (count & 0x1f) % 16;
        if count & 0x1f == 0 {
            return val;
        }
        let r = val.rotate_left(c as u32);
        self.cf = r & 1 != 0;
        self.of = (r & 0x8000 != 0) != self.cf;
        r
    }
    pub fn ror16(&mut self, val: u16, count: u8) -> u16 {
        let c = (count & 0x1f) % 16;
        if count & 0x1f == 0 {
            return val;
        }
        let r = val.rotate_right(c as u32);
        self.cf = r & 0x8000 != 0;
        self.of = (r & 0x8000 != 0) != (r & 0x4000 != 0);
        r
    }
    pub fn rol8(&mut self, val: u8, count: u8) -> u8 {
        let c = (count & 0x1f) % 8;
        if count & 0x1f == 0 {
            return val;
        }
        let r = val.rotate_left(c as u32);
        self.cf = r & 1 != 0;
        self.of = (r & 0x80 != 0) != self.cf;
        r
    }
    pub fn ror8(&mut self, val: u8, count: u8) -> u8 {
        let c = (count & 0x1f) % 8;
        if count & 0x1f == 0 {
            return val;
        }
        let r = val.rotate_right(c as u32);
        self.cf = r & 0x80 != 0;
        self.of = (r & 0x80 != 0) != (r & 0x40 != 0);
        r
    }
    pub fn rcl16(&mut self, val: u16, count: u8) -> u16 {
        let c = (count & 0x1f) % 17;
        let mut r = val;
        for _ in 0..c {
            let newcf = r & 0x8000 != 0;
            r = (r << 1) | self.cf as u16;
            self.cf = newcf;
        }
        if c != 0 {
            self.of = (r & 0x8000 != 0) != self.cf;
        }
        r
    }
    pub fn rcr16(&mut self, val: u16, count: u8) -> u16 {
        let c = (count & 0x1f) % 17;
        let mut r = val;
        for _ in 0..c {
            let newcf = r & 1 != 0;
            r = (r >> 1) | ((self.cf as u16) << 15);
            self.cf = newcf;
        }
        if c != 0 {
            self.of = (r & 0x8000 != 0) != (r & 0x4000 != 0);
        }
        r
    }
    pub fn rcl8(&mut self, val: u8, count: u8) -> u8 {
        let c = (count & 0x1f) % 9;
        let mut r = val;
        for _ in 0..c {
            let newcf = r & 0x80 != 0;
            r = (r << 1) | self.cf as u8;
            self.cf = newcf;
        }
        if c != 0 {
            self.of = (r & 0x80 != 0) != self.cf;
        }
        r
    }
    pub fn rcr8(&mut self, val: u8, count: u8) -> u8 {
        let c = (count & 0x1f) % 9;
        let mut r = val;
        for _ in 0..c {
            let newcf = r & 1 != 0;
            r = (r >> 1) | ((self.cf as u8) << 7);
            self.cf = newcf;
        }
        if c != 0 {
            self.of = (r & 0x80 != 0) != (r & 0x40 != 0);
        }
        r
    }
    pub fn rol32(&mut self, val: u32, count: u8) -> u32 {
        let c = count & 0x1f;
        if c == 0 {
            return val;
        }
        let r = val.rotate_left(c as u32);
        self.cf = r & 1 != 0;
        self.of = (r & 0x8000_0000 != 0) != self.cf;
        r
    }
    pub fn ror32(&mut self, val: u32, count: u8) -> u32 {
        let c = count & 0x1f;
        if c == 0 {
            return val;
        }
        let r = val.rotate_right(c as u32);
        self.cf = r & 0x8000_0000 != 0;
        self.of = (r & 0x8000_0000 != 0) != (r & 0x4000_0000 != 0);
        r
    }

    /// 8-bit / 16-bit signed `IDIV`. AL/AX = quotient, AH/DX = remainder. #DE cases (zero divisor
    /// or quotient out of range) leave state unchanged — the oracle discards them.
    pub fn idiv8(&mut self, src: u8) {
        if src == 0 {
            return;
        }
        let n = self.ax() as i16;
        let d = src as i8 as i16;
        let q = n / d;
        if !(-128..=127).contains(&q) {
            return;
        }
        self.set_al(q as u8);
        self.set_ah((n % d) as u8);
    }
    pub fn idiv16(&mut self, src: u16) {
        if src == 0 {
            return;
        }
        let n = ((((self.dx() as u32) << 16) | self.ax() as u32) as i32) as i64;
        let d = src as i16 as i64;
        let q = n / d;
        if !(-32768..=32767).contains(&q) {
            return;
        }
        self.set_ax(q as u16);
        self.set_dx((n % d) as u16);
    }

    /// Two-operand signed `IMUL` (32-bit, result truncated to 32). CF=OF on overflow.
    pub fn imul32_2(&mut self, a: u32, b: u32) -> u32 {
        let full = (a as i32 as i64) * (b as i32 as i64);
        let r = full as u32;
        let of = full != (r as i32 as i64);
        self.cf = of;
        self.of = of;
        r
    }

    /// `SAR` (arithmetic shift right, sign-preserving) for 8/16/32-bit. CF = last bit shifted out;
    /// ZF/SF/PF from the result; OF = 0 (defined for count==1, deterministic otherwise).
    pub fn sar8(&mut self, val: u8, count: u8) -> u8 {
        let c = count & 0x1f;
        if c == 0 {
            return val;
        }
        let mut r = val as i8;
        let mut cf = false;
        for _ in 0..c {
            cf = r & 1 != 0;
            r >>= 1;
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r < 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = false;
        r as u8
    }
    pub fn sar16(&mut self, val: u16, count: u8) -> u16 {
        let c = count & 0x1f;
        if c == 0 {
            return val;
        }
        let mut r = val as i16;
        let mut cf = false;
        for _ in 0..c {
            cf = r & 1 != 0;
            r >>= 1;
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r < 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = false;
        r as u16
    }
    pub fn sar32(&mut self, val: u32, count: u8) -> u32 {
        let c = count & 0x1f;
        if c == 0 {
            return val;
        }
        let mut r = val as i32;
        let mut cf = false;
        for _ in 0..c {
            cf = r & 1 != 0;
            r >>= 1;
        }
        self.cf = cf;
        self.zf = r == 0;
        self.sf = r < 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.of = false;
        r as u32
    }

    /// `BTR` (bit test and reset) on a 16-bit destination: CF = old bit `bit % 16`, then clear it.
    pub fn btr16(&mut self, val: u16, bit: u8) -> u16 {
        let b = bit & 0xf;
        self.cf = (val >> b) & 1 != 0;
        val & !(1u16 << b)
    }

    /// 32-bit `ADC`/`SBB` (same flag semantics as the 16-bit forms, widened).
    pub fn adc32(&mut self, a: u32, b: u32) -> u32 {
        let cin = self.cf as u64;
        let full = a as u64 + b as u64 + cin;
        let r = full as u32;
        self.cf = full > 0xffff_ffff;
        self.af = (a & 0xf) as u64 + (b & 0xf) as u64 + cin > 0xf;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = (a ^ r) & (b ^ r) & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }
    pub fn sbb32(&mut self, a: u32, b: u32) -> u32 {
        let cin = self.cf as u64;
        let r = (a as u64).wrapping_sub(b as u64 + cin) as u32;
        self.cf = (a as u64) < b as u64 + cin;
        self.af = (a & 0xf) < (b & 0xf) + cin as u32;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000 != 0;
        self.of = (a ^ b) & (a ^ r) & 0x8000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        r
    }

    /// 32-bit `TEST` (AND flags, result discarded).
    pub fn test32(&mut self, a: u32, b: u32) {
        self.and32(a, b);
    }

    /// 32-bit unsigned `MUL`: EDX:EAX = EAX * src. CF=OF = (EDX != 0).
    pub fn mul32(&mut self, src: u32) {
        let r = self.eax as u64 * src as u64;
        self.eax = r as u32;
        self.edx = (r >> 32) as u32;
        let of = self.edx != 0;
        self.cf = of;
        self.of = of;
        self.zf = r == 0;
        self.sf = r & 0x8000_0000_0000_0000 != 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }
    /// 32-bit one-operand `IMUL`: EDX:EAX = EAX * src (signed). CF=OF = significant high half.
    pub fn imul32_1(&mut self, src: u32) {
        let r = (self.eax as i32 as i64).wrapping_mul(src as i32 as i64);
        self.eax = r as u32;
        self.edx = (r >> 32) as u32;
        let of = r != r as i32 as i64;
        self.cf = of;
        self.of = of;
        self.zf = r == 0;
        self.sf = r < 0;
        self.pf = (r as u8).count_ones() % 2 == 0;
        self.af = false;
    }

    /// 32-bit unsigned/signed `DIV`/`IDIV`: EDX:EAX / src -> EAX quotient, EDX remainder. #DE
    /// (zero divisor / overflow) leaves state unchanged (oracle discards those vectors).
    pub fn div32(&mut self, src: u32) {
        if src == 0 {
            return;
        }
        let n = ((self.edx as u64) << 32) | self.eax as u64;
        let q = n / src as u64;
        if q > 0xffff_ffff {
            return;
        }
        self.eax = q as u32;
        self.edx = (n % src as u64) as u32;
    }
    pub fn idiv32(&mut self, src: u32) {
        if src == 0 {
            return;
        }
        let n = (((self.edx as u64) << 32) | self.eax as u64) as i64;
        let d = src as i32 as i64;
        let q = n / d;
        if !(-0x8000_0000..=0x7fff_ffff).contains(&q) {
            return;
        }
        self.eax = q as u32;
        self.edx = (n % d) as u32;
    }

    /// `CWD` (DX = sign of AX) / `CDQ` (EDX = sign of EAX). No flags.
    pub fn cwd(&mut self) {
        self.set_dx(if self.ax() & 0x8000 != 0 { 0xffff } else { 0 });
    }
    pub fn cdq(&mut self) {
        self.edx = if self.eax & 0x8000_0000 != 0 { 0xffff_ffff } else { 0 };
    }
}

/// VGA video memory with plane semantics (256 KB as 4 × 64 KB planes). The game runs mode 13h
/// UNCHAINED (Mode-X-style planar 320x200): CPU writes at A000:off go to the planes selected by
/// the sequencer map mask, reads come from the GC read-map plane. With chain-4 on (stock mode
/// 13h) the low two address bits select the plane, matching linear addressing.
pub struct Vga {
    pub planes: Vec<u8>, // 4 * 0x10000; plane p at p * 0x10000
    pub map_mask: u8,
    pub read_map: u8,
    pub chain4: bool,
    /// The four latches, loaded by every VRAM read; unmasked/copied bits come from here.
    /// `Cell` so loads can happen on the `&self` read path (single-threaded machine).
    pub latches: std::cell::Cell<[u8; 4]>,
    pub write_mode: u8, // GC reg 5 bits 0-1
    pub bit_mask: u8,   // GC reg 8
    pub set_reset: u8,  // GC reg 0
    pub enable_sr: u8,  // GC reg 1
    pub logic_op: u8,   // GC reg 3 bits 3-4: 0=copy 1=AND 2=OR 3=XOR (with latch)
    pub rotate: u8,     // GC reg 3 bits 0-2
}

impl Default for Vga {
    fn default() -> Self {
        Self {
            planes: vec![0; 4 * 0x10000],
            map_mask: 0x0f,
            read_map: 0,
            chain4: true,
            latches: std::cell::Cell::new([0; 4]),
            write_mode: 0,
            bit_mask: 0xff,
            set_reset: 0,
            enable_sr: 0,
            logic_op: 0,
            rotate: 0,
        }
    }
}

impl Vga {
    #[inline]
    pub fn write(&mut self, off: usize, v: u8) {
        if self.chain4 {
            self.planes[(off & 3) * 0x10000 + (off >> 2)] = v;
            return;
        }
        let o = off & 0xffff;
        match self.write_mode & 3 {
            1 => {
                // mode 1: plane-to-plane copy from the latches
                let l = self.latches.get();
                for p in 0..4 {
                    if self.map_mask & (1 << p) != 0 {
                        self.planes[p * 0x10000 + o] = l[p];
                    }
                }
            }
            3 => {
                // mode 3: the rotated CPU byte AND the bit-mask register form the effective mask;
                // the color comes entirely from set/reset. Standard color-font blit mode — this
                // is what draws the subtitle glyphs.
                let l = self.latches.get();
                let rot = v.rotate_right(self.rotate as u32 & 7);
                let eff = rot & self.bit_mask;
                for p in 0..4 {
                    if self.map_mask & (1 << p) == 0 {
                        continue;
                    }
                    let sr = if self.set_reset & (1 << p) != 0 { 0xff } else { 0x00 };
                    let out = (sr & eff) | (l[p as usize] & !eff);
                    self.planes[p as usize * 0x10000 + o] = out;
                }
            }
            m => {
                let l = self.latches.get();
                let rot = v.rotate_right(self.rotate as u32 & 7);
                for p in 0..4 {
                    if self.map_mask & (1 << p) == 0 {
                        continue;
                    }
                    let mut val = match m {
                        2 => {
                            // mode 2: CPU bit p expands to a full byte
                            if v & (1 << p) != 0 { 0xff } else { 0x00 }
                        }
                        _ => {
                            // mode 0: optional set/reset substitution per plane
                            if self.enable_sr & (1 << p) != 0 {
                                if self.set_reset & (1 << p) != 0 { 0xff } else { 0x00 }
                            } else {
                                rot
                            }
                        }
                    };
                    val = match self.logic_op & 3 {
                        1 => val & l[p as usize],
                        2 => val | l[p as usize],
                        3 => val ^ l[p as usize],
                        _ => val,
                    };
                    let out = (val & self.bit_mask) | (l[p as usize] & !self.bit_mask);
                    self.planes[p as usize * 0x10000 + o] = out;
                }
            }
        }
    }
    #[inline]
    pub fn read(&self, off: usize) -> u8 {
        if self.chain4 {
            return self.planes[(off & 3) * 0x10000 + (off >> 2)];
        }
        let o = off & 0xffff;
        let l = [
            self.planes[o],
            self.planes[0x10000 + o],
            self.planes[0x20000 + o],
            self.planes[0x30000 + o],
        ];
        self.latches.set(l);
        l[self.read_map as usize & 3]
    }
}

/// Flat real-mode memory + registers. Addressing is `seg*16 + off` (20-bit, wraps at 1 MB like
/// the 8086's segment arithmetic — high-memory area aside, which BLOODPRG doesn't use).
/// When `vga` is present (the runtime enables it), accesses to A000:0..FFFF get plane semantics.
pub struct Machine {
    pub regs: Regs,
    pub mem: Vec<u8>,
    pub vga: Option<Box<Vga>>,
    /// Current instruction pointer, updated by the interpreter each step. Lets memory-write
    /// watches attribute a write to the code address that made it (diagnostics only).
    pub ip: u16,
    /// When set, `write8` records (cs,ip,ds,si) of any write of `watch_val` into `watch_range`.
    pub watch: Option<(u8, std::ops::Range<usize>)>,
    pub watch_hits: Vec<(u16, u16, u16, u16)>,
    /// When set, `write8` records (value,cs,ip) of EVERY write to this exact linear address.
    pub watch_addr: Option<usize>,
    pub addr_hits: Vec<(u8, u16, u16)>,
    /// When set, `write8` records (addr,value,cs,ip) of every write into this range (bounded).
    pub trace_range: Option<std::ops::Range<usize>>,
    pub range_hits: Vec<(usize, u8, u16, u16)>,
    /// Execution counters: (cs,ip) -> times the interpreter started an instruction there.
    pub trap_ips: std::collections::HashMap<(u16, u16), u64>,
    /// One-shot register snapshot at a target (cs,ip): (ss,ds,es,si,bp,bx).
    pub capture_ip: Option<(u16, u16)>,
    pub captured: Option<(u16, u16, u16, u16, u16, u16)>,
    /// Snapshot of (bp, byte-at-SS:bp) each time capture_ip2 hits, bounded.
    pub capture_ip2: Option<(u16, u16)>,
    pub captured2: Vec<(u16, u16)>,
    /// Return-address capture at capture_ip: (sp, [ss:sp], [ss:sp+2], [ss:sp+4]).
    pub capture_ret: Option<(u16, u16, u16, u16)>,
    /// (cs,ip) of the instruction executed immediately before the current one.
    pub exec_prev: (u16, u16),
    /// Snapshot of exec_prev captured at capture_ip.
    pub captured_prev: Option<(u16, u16)>,
    /// When set, at this (cs,ip) record regs.al into vm_ops (VM opcode trace).
    pub vm_trace_ip: Option<(u16, u16)>,
    pub vm_ops: Vec<u8>,
    /// At capture_ip, snapshot 64 bytes at ds:0 (the current ds segment, offset 0).
    pub captured_seg: Option<Vec<u8>>,
}

pub const MEM_SIZE: usize = 0x40_0000; // 4 MB — the EXE image (deterministic oracle mirrors it),
// real-mode + 32-bit-addressing reach, and the runtime's EMS logical-page store above 1 MB
// (0x100000.., see recomp::runtime). Power of two so `lin` can mask.

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
            vga: None,
            ip: 0,
            watch: None,
            watch_hits: Vec::new(),
            watch_addr: None,
            addr_hits: Vec::new(),
            trace_range: None,
            range_hits: Vec::new(),
            trap_ips: std::collections::HashMap::new(),
            capture_ip: None,
            captured: None,
            capture_ip2: None,
            captured2: Vec::new(),
            capture_ret: None,
            exec_prev: (0, 0),
            captured_prev: None,
            vm_trace_ip: None,
            vm_ops: Vec::new(),
            captured_seg: None,
        }
    }

    /// Linear address for a real-mode `seg:off` pair, wrapped to the 1 MB image. `off` is `u32`
    /// so 32-bit effective addresses (0x67-prefixed `[eax+edi]` etc.) add their full value to
    /// `seg*16` without a 16-bit truncation — matching the CPU's real-mode 32-bit addressing (the
    /// oracle records reads/writes up to ~0x7A000 past the segment base). 16-bit addressing forms
    /// are wrapped to 16 bits by the lifter *before* the value reaches here.
    #[inline]
    pub fn lin(seg: u16, off: u32) -> usize {
        ((seg as usize) * 16 + off as usize) & (MEM_SIZE - 1)
    }

    #[inline]
    pub fn read8(&self, seg: u16, off: u32) -> u8 {
        let a = Self::lin(seg, off);
        if let Some(vga) = self.vga.as_deref() {
            if (0xa0000..0xb0000).contains(&a) {
                return vga.read(a - 0xa0000);
            }
        }
        self.mem[a]
    }
    #[inline]
    pub fn write8(&mut self, seg: u16, off: u32, v: u8) {
        let a = Self::lin(seg, off);
        if let Some((wv, range)) = &self.watch {
            if v == *wv && range.contains(&a) && self.watch_hits.len() < 10000 {
                let hit = (self.regs.cs, self.ip, self.regs.ds, self.regs.si());
                if !self.watch_hits.iter().any(|h| h.0 == hit.0 && h.1 == hit.1) {
                    self.watch_hits.push(hit);
                }
            }
        }
        if let Some(wa) = self.watch_addr {
            if a == wa && self.addr_hits.len() < 5000 {
                self.addr_hits.push((v, self.regs.cs, self.ip));
            }
        }
        if let Some(range) = &self.trace_range {
            if v != 0 && range.contains(&a) && self.range_hits.len() < 20000 {
                self.range_hits.push((a, v, self.regs.cs, self.ip));
            }
        }
        if let Some(vga) = self.vga.as_deref_mut() {
            if (0xa0000..0xb0000).contains(&a) {
                return vga.write(a - 0xa0000, v);
            }
        }
        self.mem[a] = v;
    }
    #[inline]
    pub fn read16(&self, seg: u16, off: u32) -> u16 {
        u16::from_le_bytes([self.read8(seg, off), self.read8(seg, off.wrapping_add(1))])
    }
    #[inline]
    pub fn write16(&mut self, seg: u16, off: u32, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.write8(seg, off, lo);
        self.write8(seg, off.wrapping_add(1), hi);
    }
    #[inline]
    pub fn read32(&self, seg: u16, off: u32) -> u32 {
        (self.read16(seg, off) as u32) | ((self.read16(seg, off.wrapping_add(2)) as u32) << 16)
    }
    #[inline]
    pub fn write32(&mut self, seg: u16, off: u32, v: u32) {
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
        assert_eq!(
            r.eax, 0xDEAD_12AB,
            "16/8-bit writes preserve the high dword"
        );
        r.set_ax(0x5678);
        assert_eq!(r.eax, 0xDEAD_5678);
    }

    #[test]
    fn segmented_memory_addressing() {
        let mut m = Machine::new();
        m.write16(0x1000, 0x0004, 0xBEEF); // linear 0x10004
        assert_eq!(m.read16(0x1000, 0x0004), 0xBEEF);
        assert_eq!(m.read8(0x1000, 0x0005), 0xBE);
        // 4 MB image: normal real-mode addresses never wrap; the mask only bites past 4 MB
        // (0xFFFF0 + 0x10 = 0x100000 is a real address here, not 0).
        assert_eq!(Machine::lin(0xFFFF, 0x0010), 0x100000);
        assert_eq!(Machine::lin(0xFFFF, 0x30_0010), 0x0000); // 0xFFFF0 + 0x300010 = 0x400000 wraps to 0
    }
}
