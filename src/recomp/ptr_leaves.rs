//! Pointer-chasing pure-CPU leaves verified by the INTERPRETER oracle (not the Unicorn fuzz
//! oracle). These functions scan or decode variable-length data through a pointer (a table
//! search, an RLE stream, …). Under random-fuzz memory their loops don't terminate cleanly or
//! read the code region, so the generic/det oracles can't collect ≥120 clean vectors and skip
//! them (see [[commander-blood-path-b-recomp]]). But with realistic seeded data they run fine,
//! so we lift them by hand and verify bit-exact against the ORIGINAL bytes run through the
//! interpreter — the same "interpreter is the oracle" method as [`super::io_lift`], minus the
//! Runtime (these are pure-CPU: no int/out/in), so a bare [`super::interp::Cpu`] suffices.

use super::machine::Machine;

/// `func_6293` (`vm_token_special`, 0x6293): a byte-granular table search. Scan forward from
/// ds:SI comparing the WORD at ds:SI to AX; on a match, advance SI past that word (`+2`) and, if
/// the following byte equals AL, advance one more. Leaves SI pointing just past the matched entry.
/// Called from `token_advance` for the length-0 opcodes (A8/AC/CC/D3) of the conversation VM.
pub fn func_6293(m: &mut Machine) {
    let ds = m.regs.ds;
    let ax = m.regs.ax();
    let mut si = m.regs.si();
    // cmp ax,[si]; je out; inc si; jmp — scan byte-by-byte until the word at SI equals AX.
    while m.read16(ds, si as u32) != ax {
        si = si.wrapping_add(1);
    }
    si = si.wrapping_add(2); // add si,2 — step past the matched word
    // cmp al,[si]; jne ret; inc si — consume one more byte if it equals AL.
    if m.regs.al() == m.read8(ds, si as u32) {
        si = si.wrapping_add(1);
    }
    m.regs.set_si(si);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recomp::interp::{Cpu, Exit};

    fn load_exe() -> Option<Vec<u8>> {
        let raw = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
            .ok()?;
        let mut img = raw;
        img.resize(0x120000, 0);
        Some(img)
    }

    /// Run the ORIGINAL bytes at file `offset` (CS=0) through the interpreter until the leaf's
    /// depth-0 `ret`/`retf`, leaving `m` in the real function's output state.
    fn interp_leaf(m: &mut Machine, offset: u16) {
        let mut cpu = Cpu::new(0, offset);
        cpu.depth = 0;
        for _ in 0..1_000_000 {
            match cpu.run(m, 4096) {
                Exit::Ret | Exit::Retf => return,
                Exit::StepLimit => continue,
                other => panic!("interp_leaf: unexpected exit {other:?} at {offset:#x}"),
            }
        }
        panic!("interp_leaf: {offset:#x} did not return");
    }

    /// `func_22e0` (`abs_negate_gs_setup` / 3D-vertex projection, 0x22E0..0x23C4) reproduces the
    /// original bytes exactly. It rebases ds=es=gs, negates the AX argument, scales the projection
    /// inputs by /100, then reads the vertex table at gs:[0x5251] (via lodsb) through stack-frame
    /// locals and writes one projected byte to gs:[DI]. All registers are restored (retf), so the
    /// only persistent output is the gs-segment write — we compare the whole 64 KB gs window.
    /// Inputs are chosen small (AX≈-16, coords ≤0x20, vertex bytes <0x80) so no /100 overflows #DE.
    /// This is the Unicorn oracle's blind spot (its random inputs make the scaled products index
    /// the code region → vectors discarded) that the interpreter oracle covers directly.
    #[test]
    fn func_22e0_matches_interpreter_oracle() {
        let Some(exe) = load_exe() else { return };
        const GS: u16 = 0x3000;
        const SS: u16 = 0x4000; // phys 0x40000 — just above the 64 KB gs window, clear of it
        let seed = |m: &mut Machine| {
            m.regs.gs = GS;
            m.regs.ds = GS;
            m.regs.es = GS;
            m.regs.ss = SS;
            m.regs.set_sp(0x0800);
            m.regs.set_ax(0xFFF0); // neg → 0x10, small
            m.regs.set_bx(0x0020);
            m.regs.set_cx(0x0018);
            m.regs.set_dx(0x0010);
            // vertex table at gs:[0x5251]: small positive bytes so CBW keeps AX small.
            for k in 0..128u32 {
                m.write8(GS, 0x5251 + k, (0x10 + (k & 0x3f)) as u8);
            }
        };

        let mut m_lift = Machine::new();
        m_lift.mem[..0x10000].copy_from_slice(&exe[..0x10000]);
        seed(&mut m_lift);
        super::super::ptr_leaves_gen::func_22e0(&mut m_lift);

        let mut m_oracle = Machine::new();
        m_oracle.mem[..0x10000].copy_from_slice(&exe[..0x10000]);
        seed(&mut m_oracle);
        interp_leaf(&mut m_oracle, 0x22e0);

        // Compare the entire 64 KB gs segment (phys GS*16 .. +0x10000) — captures the projected
        // byte written to gs:[DI] wherever DI landed, plus confirms nothing else diverged.
        let base = (GS as usize) * 16;
        assert!(
            m_lift.mem[base..base + 0x10000] == m_oracle.mem[base..base + 0x10000],
            "func_22e0: gs-segment output differs from the real bytes"
        );
    }

    /// func_6293 reproduces the original bytes exactly across several table layouts: the target
    /// word at varying offsets (aligned + unaligned), with the trailing byte matching AL (= the
    /// target's low byte, since the original loads AX once) or not. The table lives at ds:0x6000
    /// (above the 0x10000 code mirror, so the mirror can't clobber it). The interpreter runs the
    /// real bytes; we assert the lifted SI == the real SI.
    #[test]
    fn func_6293_matches_interpreter_oracle() {
        let Some(exe) = load_exe() else { return };
        const DS: u16 = 0x3000;
        const BASE: u32 = 0x6000;
        // (target word AX, byte offset of the matching word, trailing byte == AL?)
        let cases: &[(u16, u32, bool)] = &[
            (0xBEEF, 0, true),   // immediate match, trailing byte == AL
            (0xBEEF, 0, false),  // immediate match, trailing byte != AL
            (0x1234, 6, true),   // match a few bytes in, word-aligned
            (0x1234, 5, true),   // match at an UNALIGNED offset (byte-granular scan)
            (0x00FF, 13, false), // deeper, trailing != AL
        ];
        for (i, &(target, off, trail_eq_al)) in cases.iter().enumerate() {
            let al = (target & 0xff) as u8;
            let trail = if trail_eq_al { al } else { al ^ 0xff };
            let seed = |m: &mut Machine| {
                m.regs.ds = DS;
                m.regs.set_si(BASE as u16);
                m.regs.set_ax(target); // AL is target's low byte, as in the original
                // fill a non-matching window, then plant the target word + its trailing byte.
                for k in 0..64u32 {
                    m.write8(DS, BASE + k, (0x40 + k) as u8);
                }
                m.write16(DS, BASE + off, target);
                m.write8(DS, BASE + off + 2, trail);
            };

            let mut m_lift = Machine::new();
            m_lift.mem[..0x10000].copy_from_slice(&exe[..0x10000]);
            seed(&mut m_lift);
            func_6293(&mut m_lift);

            let mut m_oracle = Machine::new();
            m_oracle.mem[..0x10000].copy_from_slice(&exe[..0x10000]);
            seed(&mut m_oracle);
            interp_leaf(&mut m_oracle, 0x6293);

            assert_eq!(
                m_lift.regs.si(),
                m_oracle.regs.si(),
                "case {i}: SI lift {:#x} vs real {:#x}",
                m_lift.regs.si(),
                m_oracle.regs.si()
            );
        }
    }
}
