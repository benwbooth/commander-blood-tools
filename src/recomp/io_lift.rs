//! I/O-boundary static lifts — the Runtime-context lift path.
//!
//! The pure-CPU static lifts in [`super::auto`] take `&mut Machine` and are oracle-
//! verified against Unicorn. The I/O leaves (int/out/in) can't be — Unicorn doesn't
//! model DOS/hardware — and a straight-line `Machine`-only lift can't service an
//! `int` (which needs Runtime state: file table, DOS/BIOS/port handlers). So I/O
//! functions are lifted here as **Runtime-context** functions that call the same
//! `Runtime::native_int` / port handlers the interpreter uses, and are verified
//! against the INTERPRETER (deterministic — same handlers). This module holds the
//! first such lift, proving the architecture; the rest follow the same shape.

use super::runtime::Runtime;

/// `func_cc0` (`set_video_mode_saved`, file 0x0CC0): restore the saved video mode.
/// `push ax; xor ax,ax; mov al,gs:[0x5232]; int 0x10; pop ax; retf` — sets BIOS video
/// mode `AH=0` to the byte saved at `gs:0x5232`. The `int 0x10` routes through
/// `Runtime::native_int` (the same DOS/BIOS service the interpreter uses).
pub fn func_cc0(rt: &mut Runtime) {
    // push ax
    let (ss, sp0) = (rt.m.regs.ss, rt.m.regs.sp());
    let sp = sp0.wrapping_sub(2);
    rt.m.regs.set_sp(sp);
    let ax = rt.m.regs.ax();
    rt.m.write16(ss, sp as u32, ax);
    // xor ax, ax
    rt.m.regs.set_ax(0);
    // mov al, gs:[0x5232]
    let gs = rt.m.regs.gs;
    let al = rt.m.read8(gs, 0x5232);
    rt.m.regs.set_al(al);
    // int 0x10 — BIOS video service (AH=0 set mode). Real `int` pushes the frame
    // (FLAGS, CS, IP); native_int services then IRETs (pops the frame). We push the
    // matching frame so the net stack effect is zero and the service runs correctly.
    let flags =
        super::interp::flags_word(&rt.m.regs) | ((rt.cpu.iflag as u16) << 9) | rt.cpu.flags_high;
    let (cs, ip) = (rt.cpu.cs, rt.cpu.ip);
    for w in [flags, cs, ip] {
        let sp = rt.m.regs.sp().wrapping_sub(2);
        rt.m.regs.set_sp(sp);
        rt.m.write16(rt.m.regs.ss, sp as u32, w);
    }
    let _ = rt.native_int(0x10); // services int 0x10 and IRETs the frame we pushed
    // pop ax
    let (ss, sp) = (rt.m.regs.ss, rt.m.regs.sp());
    let popped = rt.m.read16(ss, sp as u32);
    rt.m.regs.set_sp(sp.wrapping_add(2));
    rt.m.regs.set_ax(popped);
    // retf — the caller resumes; net stack effect is balanced (push/pop cancel).
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_runtime() -> Runtime {
        // Same roots the boot uses; construction alone (no boot) is enough here.
        Runtime::new(PathBuf::from("accuracy/cdrive"), PathBuf::from("output/_tmp_iso"))
    }

    /// The Runtime-context I/O lift runs correctly: it preserves AX (balanced
    /// push/pop), leaves SP net-unchanged, loads the saved mode into AL, and its
    /// `int 0x10` routes through `native_int` (BIOS video service) without fault —
    /// proving the I/O-lift architecture (Runtime-context + native_int) works.
    #[test]
    fn io_lift_func_cc0_executes_and_preserves_state() {
        let mut rt = test_runtime();
        // Set up a valid stack + the saved video mode at gs:0x5232.
        rt.m.regs.ss = 0x2000;
        rt.m.regs.set_sp(0x0100);
        rt.m.regs.set_ax(0xBEEF);
        let gs = rt.m.regs.gs;
        rt.m.write8(gs, 0x5232, 0x03); // saved mode = 3 (80x25 text)
        let (ss0, sp0) = (rt.m.regs.ss, rt.m.regs.sp());

        func_cc0(&mut rt);

        eprintln!(
            "after func_cc0: ax={:#06x} sp={:#06x} ss={:#06x} mode(40:49)={:#04x}",
            rt.m.regs.ax(),
            rt.m.regs.sp(),
            rt.m.regs.ss,
            rt.m.read8(0x40, 0x49)
        );
        // AX is restored (balanced push/pop); SP and SS are net-unchanged.
        assert_eq!(rt.m.regs.ax(), 0xBEEF, "AX preserved");
        assert_eq!(rt.m.regs.sp(), sp0, "SP net-unchanged");
        assert_eq!(rt.m.regs.ss, ss0, "SS unchanged");
        // The BIOS current-video-mode byte (40:0x49) reflects the set mode (3).
        assert_eq!(rt.m.read8(0x40, 0x49), 0x03, "int 0x10 set the video mode via native_int");
    }
}
