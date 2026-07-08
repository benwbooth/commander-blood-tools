//! 1-to-1 static recompilation of BLOODPRG.EXE (path B).
//!
//! Each function here is lifted directly from the disassembly to operate on the shared
//! [`Machine`], reproducing the exact register + memory effects of the original. Correctness is
//! established per-function by the Unicorn oracle: `re/tools/gen_oracle_vectors.py` fuzzes the
//! real DOS function and dumps (input-state → output-state) vectors, and the tests below replay
//! each vector through the lifted Rust and assert bit-exact equality. When every reachable
//! function is lifted + verified and composed in the binary's call graph, the whole program runs
//! identically by construction (see re/tools/README_oracle.md).

pub mod machine;

use machine::Machine;

/// `prng_2de2` — the game PRNG at file 0x2DE2 (far 0x1CE:0x0B02).
///
/// Input: `AX` = modulus. State (in the code segment): `cs:0xAEE` seed_word (u16), `cs:0xAF0` a,
/// `cs:0xAF1` b, `cs:0xAF2` counter. Output: `AX` = a value in `[0, modulus)` (or unchanged if
/// modulus == 0), with a/b/counter advanced. `bx/cx/dx` are preserved (push/pop). Lifted 1-to-1
/// from the disassembly (0x2DE2..0x2E32); verified bit-exact vs the binary by the oracle vectors.
pub fn prng_2de2(m: &mut Machine) {
    let cs = m.regs.cs;
    let modulus = m.regs.ax; // mov dx, ax  (dx is scratch, restored by pop)

    // mov bl,cs:[0xAF0]; mov bh,cs:[0xAF1]; mov cx,8; xor ax,ax (clears CF)
    let mut bl = m.read8(cs, 0xaf0);
    let mut bh = m.read8(cs, 0xaf1);
    let mut ax: u16 = 0;
    let mut cf = false;
    // 8x: rcr bl,1 ; rcl ax,1 ; rcl bh,1 ; rcl ax,1
    for _ in 0..8 {
        let carry_out = bl & 1;
        bl = ((cf as u8) << 7) | (bl >> 1);
        cf = carry_out != 0;

        let carry_out = ax >> 15;
        ax = ax.wrapping_shl(1) | cf as u16;
        cf = carry_out != 0;

        let carry_out = bh >> 7;
        bh = bh.wrapping_shl(1) | cf as u8;
        cf = carry_out != 0;

        let carry_out = ax >> 15;
        ax = ax.wrapping_shl(1) | cf as u16;
        cf = carry_out != 0;
    }

    // mov bx,cs:[0xAEE]; shr bx,3  — dead (bx restored, bl overwritten below); read for fidelity.
    let _ = m.read16(cs, 0xaee) >> 3;

    // xor ax, cs:[0xAEE]  (mix in seed_word)
    ax ^= m.read16(cs, 0xaee);

    // inc byte cs:[0xAF2]; mov bl,cs:[0xAF2]; sub byte cs:[0xAF1],bl; rol bl,1; xor byte cs:[0xAF0],bl
    let counter = m.read8(cs, 0xaf2).wrapping_add(1);
    m.write8(cs, 0xaf2, counter);
    let new_b = m.read8(cs, 0xaf1).wrapping_sub(counter);
    m.write8(cs, 0xaf1, new_b);
    let rotated = counter.rotate_left(1);
    let new_a = m.read8(cs, 0xaf0) ^ rotated;
    m.write8(cs, 0xaf0, new_a);

    // or dx,dx; jne .. ; je ..  then  while ax >= dx: ax -= dx   (dx == modulus)
    if modulus != 0 {
        while ax >= modulus {
            ax = ax.wrapping_sub(modulus);
        }
    }
    m.regs.ax = ax;
    // bx/cx/dx are pushed then popped -> unchanged; we never touched m.regs.{bx,cx,dx}.
}

/// `func_a734` — file 0xA734: `add [DS:0xD8C],ax ; add [DS:0xD9A],ax ; clc ; ret`.
/// Adds `AX` into the two word globals (with full ADD flags from the second add), then clears
/// CF. `AX`/`DS` unchanged. Lifted 1-to-1; oracle-verified (return regs, memory, all 6 flags).
pub fn func_a734(m: &mut Machine) {
    let ds = m.regs.ds;
    let ax = m.regs.ax;
    let v1 = m.read16(ds, 0xd8c);
    let r1 = m.regs.add16(v1, ax);
    m.write16(ds, 0xd8c, r1);
    let v2 = m.read16(ds, 0xd9a);
    let r2 = m.regs.add16(v2, ax);
    m.write16(ds, 0xd9a, r2);
    m.regs.cf = false; // clc
}

/// `func_a744` — file 0xA744: initialise three word globals to constants (no input, no flags).
pub fn func_a744(m: &mut Machine) {
    let ds = m.regs.ds;
    m.write16(ds, 0xd62, 0x0000);
    m.write16(ds, 0xd64, 0xffff);
    m.write16(ds, 0xd66, 0xffff);
}

/// `func_9f80` — file 0x9F80: `bx=0x1FB5; add bx,ax (x4); mov bx,[bx]; ret`. Computes the table
/// address `0x1FB5 + 4*ax` (16-bit wrapping), reads the word there (DS-relative) into `BX`.
/// Flags come from the 4th `add`. Lifted 1-to-1; oracle-verified (BX + all 6 flags).
pub fn func_9f80(m: &mut Machine) {
    let ax = m.regs.ax;
    let mut addr: u16 = 0x1fb5;
    for _ in 0..4 {
        addr = m.regs.add16(addr, ax);
    }
    m.regs.bx = m.read16(m.regs.ds, addr);
}

#[cfg(test)]
mod tests {
    use super::machine::Machine;
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Flags {
        cf: bool,
        pf: bool,
        af: bool,
        zf: bool,
        sf: bool,
        of: bool,
    }

    #[derive(Deserialize)]
    struct A734Vec {
        ax: u16,
        w1: u16,
        w2: u16,
        ax_out: u16,
        flags: Flags,
        w1_out: u16,
        w2_out: u16,
    }

    #[test]
    fn func_a734_matches_oracle_vectors() {
        let raw = match std::fs::read_to_string("re/tools/oracle_vectors/func_a734.json")
            .or_else(|_| std::fs::read_to_string("../re/tools/oracle_vectors/func_a734.json"))
        {
            Ok(s) => s,
            Err(_) => return,
        };
        const DS: u16 = 0x2000;
        let vecs: Vec<A734Vec> = serde_json::from_str(&raw).unwrap();
        assert!(!vecs.is_empty());
        for (i, v) in vecs.iter().enumerate() {
            let mut m = Machine::new();
            m.regs.ds = DS;
            m.regs.ax = v.ax;
            m.write16(DS, 0xd8c, v.w1);
            m.write16(DS, 0xd9a, v.w2);
            func_a734(&mut m);
            assert_eq!(m.regs.ax, v.ax_out, "vec {i}: AX");
            assert_eq!(m.read16(DS, 0xd8c), v.w1_out, "vec {i}: [0xD8C]");
            assert_eq!(m.read16(DS, 0xd9a), v.w2_out, "vec {i}: [0xD9A]");
            assert_eq!(m.regs.cf, v.flags.cf, "vec {i}: CF");
            assert_eq!(m.regs.pf, v.flags.pf, "vec {i}: PF");
            assert_eq!(m.regs.af, v.flags.af, "vec {i}: AF");
            assert_eq!(m.regs.zf, v.flags.zf, "vec {i}: ZF");
            assert_eq!(m.regs.sf, v.flags.sf, "vec {i}: SF");
            assert_eq!(m.regs.of, v.flags.of, "vec {i}: OF");
        }
    }

    #[derive(Deserialize)]
    struct A744Vec {
        a: u16,
        b: u16,
        c: u16,
    }

    #[test]
    fn func_a744_matches_oracle_vectors() {
        let raw = match std::fs::read_to_string("re/tools/oracle_vectors/func_a744.json")
            .or_else(|_| std::fs::read_to_string("../re/tools/oracle_vectors/func_a744.json"))
        {
            Ok(s) => s,
            Err(_) => return,
        };
        const DS: u16 = 0x2000;
        let vecs: Vec<A744Vec> = serde_json::from_str(&raw).unwrap();
        assert!(!vecs.is_empty());
        for (i, v) in vecs.iter().enumerate() {
            let mut m = Machine::new();
            m.regs.ds = DS;
            func_a744(&mut m);
            assert_eq!(m.read16(DS, 0xd62), v.a, "vec {i}: [0xD62]");
            assert_eq!(m.read16(DS, 0xd64), v.b, "vec {i}: [0xD64]");
            assert_eq!(m.read16(DS, 0xd66), v.c, "vec {i}: [0xD66]");
        }
    }

    #[derive(Deserialize)]
    struct F9f80Vec {
        ax: u16,
        word: u16,
        bx_out: u16,
        flags: Flags,
    }

    #[test]
    fn func_9f80_matches_oracle_vectors() {
        let raw = match std::fs::read_to_string("re/tools/oracle_vectors/func_9f80.json")
            .or_else(|_| std::fs::read_to_string("../re/tools/oracle_vectors/func_9f80.json"))
        {
            Ok(s) => s,
            Err(_) => return,
        };
        const DS: u16 = 0x2000;
        let vecs: Vec<F9f80Vec> = serde_json::from_str(&raw).unwrap();
        assert!(!vecs.is_empty());
        for (i, v) in vecs.iter().enumerate() {
            let mut m = Machine::new();
            m.regs.ds = DS;
            m.regs.ax = v.ax;
            let addr = (0x1fb5u16)
                .wrapping_add(v.ax)
                .wrapping_add(v.ax)
                .wrapping_add(v.ax)
                .wrapping_add(v.ax);
            m.write16(DS, addr, v.word);
            func_9f80(&mut m);
            assert_eq!(m.regs.bx, v.bx_out, "vec {i}: BX");
            assert_eq!(m.regs.cf, v.flags.cf, "vec {i}: CF");
            assert_eq!(m.regs.pf, v.flags.pf, "vec {i}: PF");
            assert_eq!(m.regs.af, v.flags.af, "vec {i}: AF");
            assert_eq!(m.regs.zf, v.flags.zf, "vec {i}: ZF");
            assert_eq!(m.regs.sf, v.flags.sf, "vec {i}: SF");
            assert_eq!(m.regs.of, v.flags.of, "vec {i}: OF");
        }
    }

    /// One oracle vector: the input machine state and the DOS binary's resulting output state,
    /// captured by running the real function in Unicorn (re/tools/gen_oracle_vectors.py).
    #[derive(Deserialize)]
    struct PrngVec {
        cs: u16,
        ax_in: u16,
        seed: u16,
        a: u8,
        b: u8,
        counter: u8,
        ax_out: u16,
        a_out: u8,
        b_out: u8,
        counter_out: u8,
    }

    #[test]
    fn prng_2de2_matches_oracle_vectors() {
        let raw = match std::fs::read_to_string("re/tools/oracle_vectors/prng_2de2.json")
            .or_else(|_| std::fs::read_to_string("../re/tools/oracle_vectors/prng_2de2.json"))
        {
            Ok(s) => s,
            Err(_) => return, // vectors not generated in this checkout
        };
        let vecs: Vec<PrngVec> = serde_json::from_str(&raw).unwrap();
        assert!(!vecs.is_empty());
        for (i, v) in vecs.iter().enumerate() {
            let mut m = Machine::new();
            m.regs.cs = v.cs;
            m.regs.ax = v.ax_in;
            m.write16(v.cs, 0xaee, v.seed);
            m.write8(v.cs, 0xaf0, v.a);
            m.write8(v.cs, 0xaf1, v.b);
            m.write8(v.cs, 0xaf2, v.counter);
            prng_2de2(&mut m);
            assert_eq!(m.regs.ax, v.ax_out, "vec {i}: AX");
            assert_eq!(m.read8(v.cs, 0xaf0), v.a_out, "vec {i}: a");
            assert_eq!(m.read8(v.cs, 0xaf1), v.b_out, "vec {i}: b");
            assert_eq!(m.read8(v.cs, 0xaf2), v.counter_out, "vec {i}: counter");
            // seed_word (0xAEE) must be unchanged by this function.
            assert_eq!(m.read16(v.cs, 0xaee), v.seed, "vec {i}: seed unchanged");
        }
    }
}
