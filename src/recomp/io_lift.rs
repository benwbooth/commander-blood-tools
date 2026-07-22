//! I/O-boundary static lifts — the Runtime-context lift path.
//!
//! The pure-CPU static lifts in [`super::auto`] take `&mut Machine` and are oracle-
//! verified against Unicorn. The I/O leaves (int/out/in) can't be — Unicorn doesn't
//! model DOS/hardware — and a straight-line `Machine`-only lift can't service an
//! `int` (which needs Runtime state: file table, DOS/BIOS/port handlers). So I/O
//! functions are lifted here as **Runtime-context** functions that call the same
//! `Runtime::native_int` / port handlers the interpreter uses, and are verified
//! against the INTERPRETER (deterministic — same handlers). Each lift follows the
//! same shape: translate the CPU ops, and for an `int`, [`int_call`] pushes the
//! interrupt frame (FLAGS, CS, IP) so `native_int`'s IRET balances the stack.

use super::runtime::Runtime;

/// Push a 16-bit word onto the guest stack (SS:SP).
fn push16(rt: &mut Runtime, v: u16) {
    let sp = rt.m.regs.sp().wrapping_sub(2);
    rt.m.regs.set_sp(sp);
    let ss = rt.m.regs.ss;
    rt.m.write16(ss, sp as u32, v);
}

/// Pop a 16-bit word from the guest stack (SS:SP).
fn pop16(rt: &mut Runtime) -> u16 {
    let (ss, sp) = (rt.m.regs.ss, rt.m.regs.sp());
    let v = rt.m.read16(ss, sp as u32);
    rt.m.regs.set_sp(sp.wrapping_add(2));
    v
}

/// Service an `int vector` from a lifted function exactly as the CPU + handler do:
/// push the interrupt frame (FLAGS, CS, IP) as the real `int` instruction does, then
/// `native_int` services it and IRETs (popping the frame). Net stack effect zero.
fn int_call(rt: &mut Runtime, vector: u8) {
    let flags =
        super::interp::flags_word(&rt.m.regs) | ((rt.cpu.iflag as u16) << 9) | rt.cpu.flags_high;
    let (cs, ip) = (rt.cpu.cs, rt.cpu.ip);
    push16(rt, flags);
    push16(rt, cs);
    push16(rt, ip);
    let _ = rt.native_int(vector);
}

/// `func_cc0` (`set_video_mode_saved`, 0x0CC0): `push ax; xor ax,ax;
/// mov al,gs:[0x5232]; int 0x10; pop ax; retf` — set BIOS video mode to the byte
/// saved at gs:0x5232.
pub fn func_cc0(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    rt.m.regs.set_ax(0);
    let gs = rt.m.regs.gs;
    let al = rt.m.read8(gs, 0x5232);
    rt.m.regs.set_al(al);
    int_call(rt, 0x10);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_d4a` (`mouse_set_hrange`, 0x0D4A): `push ax,bx,cx,dx; cx=ax; dx=bx; ax=7;
/// int 33h; pop dx,cx; ax=8; int 33h; pop bx,ax; retf` — set the mouse cursor's
/// horizontal (fn 7) then vertical (fn 8) range to [AX,BX].
pub fn func_d4a(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.bx());
    push16(rt, rt.m.regs.cx());
    push16(rt, rt.m.regs.dx());
    let (ax, bx) = (rt.m.regs.ax(), rt.m.regs.bx());
    rt.m.regs.set_cx(ax);
    rt.m.regs.set_dx(bx);
    rt.m.regs.set_ax(7);
    int_call(rt, 0x33);
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let cx = pop16(rt);
    rt.m.regs.set_cx(cx);
    rt.m.regs.set_ax(8);
    int_call(rt, 0x33);
    let bx = pop16(rt);
    rt.m.regs.set_bx(bx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_cef` (`mouse_reset_hide`, 0x0CEF): reset the mouse driver (fn 0), hide the
/// cursor (fn 2), and set the mickey/pixel ratio (fn 0xF, cx=dx=0xC). The game draws
/// its own cursor.
pub fn func_cef(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.bx());
    push16(rt, rt.m.regs.cx());
    push16(rt, rt.m.regs.dx());
    push16(rt, rt.m.regs.es);
    rt.m.regs.set_ax(0);
    int_call(rt, 0x33);
    rt.m.regs.set_ax(2);
    int_call(rt, 0x33);
    rt.m.regs.set_cx(0xc);
    rt.m.regs.set_dx(0xc);
    rt.m.regs.set_ax(0xf);
    int_call(rt, 0x33);
    let es = pop16(rt);
    rt.m.regs.es = es;
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let cx = pop16(rt);
    rt.m.regs.set_cx(cx);
    let bx = pop16(rt);
    rt.m.regs.set_bx(bx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_d0e` (`poll_mouse`, 0x0D0E): `int 33h` fn 3 (get position + buttons) → store
/// cx/dx/bx to gs:[0xA2A]/[0xA2C]/[0xA2E]; if the position changed since the last poll
/// (gs:[0xA38]/[0xA3A]), latch the new position and clear the "cursor idle" counter
/// gs:[0xB3B]. Runs every frame; feeds the hit-test at 0x8269.
pub fn func_d0e(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.bx());
    push16(rt, rt.m.regs.cx());
    push16(rt, rt.m.regs.dx());
    rt.m.regs.set_ax(3);
    int_call(rt, 0x33); // cx=x, dx=y, bx=buttons
    let (cx, dx, bx) = (rt.m.regs.cx(), rt.m.regs.dx(), rt.m.regs.bx());
    let gs = rt.m.regs.gs;
    rt.m.write16(gs, 0xa2a, cx);
    rt.m.write16(gs, 0xa2c, dx);
    rt.m.write16(gs, 0xa2e, bx);
    // update UNLESS both coords equal the last-latched pair (jne then je in the original).
    let last_x = rt.m.read16(gs, 0xa38);
    let last_y = rt.m.read16(gs, 0xa3a);
    if cx != last_x || dx != last_y {
        rt.m.write16(gs, 0xa38, cx);
        rt.m.write16(gs, 0xa3a, dx);
        rt.m.write16(gs, 0xb3b, 0);
    }
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let cx = pop16(rt);
    rt.m.regs.set_cx(cx);
    let bx = pop16(rt);
    rt.m.regs.set_bx(bx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_bff` (`install_ctrl_break_handler`, 0x0BFF): `ds=cs`; `int 21h` AX=0x2523 (set
/// INT 23h = Ctrl-Break) to ds:0x619, then AX=0x2524 (set INT 24h = critical-error) to
/// ds:0x61A. Traps Ctrl-Break / critical-error so the game cleans up on exit. (A recomp-path
/// lift — installing a guest vector to a guest code address is segmented by nature.)
pub fn func_bff(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.dx());
    push16(rt, rt.m.regs.ds);
    let cs = rt.m.regs.cs;
    rt.m.regs.ds = cs; // mov ax,cs; mov ds,ax
    rt.m.regs.set_ax(0x2523);
    rt.m.regs.set_dx(0x0619);
    int_call(rt, 0x21); // set INT 23h -> ds:0x619
    rt.m.regs.set_al(0x24); // ax = 0x2524 (ah=0x25 set-vector, al=0x24)
    rt.m.regs.set_dx(0x061a);
    int_call(rt, 0x21); // set INT 24h -> ds:0x61a
    let ds = pop16(rt);
    rt.m.regs.ds = ds;
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_runtime() -> Runtime {
        Runtime::new(PathBuf::from("accuracy/cdrive"), PathBuf::from("output/_tmp_iso"))
    }

    /// The raw BLOODPRG.EXE image, mirrored at physical 0 (CS=0, IP=file offset) exactly as the
    /// pure-CPU oracle maps it — so the interpreter executes the REAL function bytes.
    fn load_exe() -> Option<Vec<u8>> {
        let raw = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
            .ok()?;
        let mut img = raw;
        img.resize(0x120000, 0);
        Some(img)
    }

    /// INTERPRETER ORACLE: execute the real function bytes at file `offset` (CS=0) through the
    /// interpreter until its terminal `retf` (depth-0 return), servicing each `int` through the
    /// SAME [`int_call`] path the lifts use. Leaves `rt` holding the real function's output state.
    /// This validates the hand-written CPU translation (register/stack/memory plumbing) against the
    /// original instruction stream — the interpreter is the oracle (Unicorn can't model DOS I/O).
    /// The caller must mirror the EXE at physical 0 (see the test) BEFORE seeding, so the seed's
    /// scratch memory survives — a 16-bit segment can't address above the 0x120000 mirror.
    fn interp_leaf(rt: &mut Runtime, offset: u16) {
        rt.cpu.cs = 0;
        rt.cpu.ip = offset;
        rt.m.regs.cs = 0;
        rt.cpu.depth = 0;
        for _ in 0..100_000 {
            match rt.cpu.run(&mut rt.m, 4096) {
                super::super::interp::Exit::Ret | super::super::interp::Exit::Retf => return,
                super::super::interp::Exit::Int { vector } => int_call(rt, vector),
                super::super::interp::Exit::StepLimit => continue,
                other => panic!("interp_leaf: unexpected exit {other:?} at offset {offset:#x}"),
            }
        }
        panic!("interp_leaf: {offset:#x} did not return within guard");
    }

    /// Set the register state each leaf is verified from (arbitrary sentinels in every register
    /// the lift touches, so a mis-plumbed push/pop is caught).
    fn seed(rt: &mut Runtime) {
        rt.m.regs.ss = 0x2000;
        rt.m.regs.set_sp(0x0100);
        rt.m.regs.set_ax(0xBEEF);
        rt.m.regs.set_bx(0x0130);
        rt.m.regs.set_cx(0xAAAA);
        rt.m.regs.set_dx(0xBBBB);
        rt.m.regs.es = 0x1357;
        // CS matches the oracle's interp run (CS=0) so func_bff's `ds=cs` is identical on both sides.
        rt.m.regs.cs = 0x0000;
        rt.m.regs.gs = 0x3000;
        rt.m.write8(0x3000, 0x5232, 0x03); // saved video mode for func_cc0
        // A distinct sentinel in the BIOS video-mode byte so func_cc0 genuinely changes it (3≠0xEE)
        // and the leaves that DON'T set it (d4a/cef) leave 0xEE on both sides. Written after the
        // EXE mirror so it isn't clobbered by the byte at physical 0x449.
        rt.m.write8(0x40, 0x49, 0xEE);
        // Mouse state for func_d0e: distinct position/buttons, and a stale latched position
        // (gs:[0xa38]/[0xa3a] default 0 ≠ 0x40/0x30) so the position-changed branch is taken.
        rt.mouse_x = 0x0040;
        rt.mouse_y = 0x0030;
        rt.mouse_buttons = 0x0002;
    }

    /// Every Runtime-context I/O lift reproduces the REAL function bytes exactly: run the lift and
    /// the interpreter-over-the-original-bytes from an identical seed state and assert the full
    /// observable state (all GP regs, SP, ES, video mode, mouse state) is bit-identical. This is
    /// the same "oracle-verified" standard the pure-CPU lifts have — here the interpreter is the
    /// oracle. Adding a leaf to this table is the gate for calling it verified.
    #[test]
    fn io_lifts_match_interpreter_oracle() {
        let Some(exe) = load_exe() else { return };
        // (name, file offset, lifted fn, gs offsets written, segment-0/IVT offsets written) —
        // every listed offset is compared word-for-word against the real bytes' output.
        let leaves: &[(&str, u16, fn(&mut Runtime), &[u16], &[u16])] = &[
            ("func_cc0", 0x0cc0, func_cc0, &[], &[]),
            ("func_d4a", 0x0d4a, func_d4a, &[], &[]),
            ("func_cef", 0x0cef, func_cef, &[], &[]),
            ("func_d0e", 0x0d0e, func_d0e, &[0xa2a, 0xa2c, 0xa2e, 0xa38, 0xa3a, 0xb3b], &[]),
            // INT 23h vector at 0:[0x8c/0x8e], INT 24h vector at 0:[0x90/0x92].
            ("func_bff", 0x0bff, func_bff, &[], &[0x8c, 0x8e, 0x90, 0x92]),
        ];
        for &(name, offset, lift, gs_checks, seg0_checks) in leaves {
            let mut rt_lift = test_runtime();
            seed(&mut rt_lift);
            lift(&mut rt_lift);

            let mut rt_oracle = test_runtime();
            // Mirror the EXE at physical 0 (CS=0, IP=offset) BEFORE seeding so the seed's scratch
            // memory (gs:0x5232, stack) isn't clobbered by the mirror.
            rt_oracle.m.mem[..exe.len()].copy_from_slice(&exe);
            seed(&mut rt_oracle);
            interp_leaf(&mut rt_oracle, offset);

            let l = &rt_lift.m.regs;
            let o = &rt_oracle.m.regs;
            assert_eq!(l.ax(), o.ax(), "{name}: AX (lift {:#x} vs real {:#x})", l.ax(), o.ax());
            assert_eq!(l.bx(), o.bx(), "{name}: BX");
            assert_eq!(l.cx(), o.cx(), "{name}: CX");
            assert_eq!(l.dx(), o.dx(), "{name}: DX");
            assert_eq!(l.sp(), o.sp(), "{name}: SP (stack balance)");
            assert_eq!(l.es, o.es, "{name}: ES");
            assert_eq!(
                rt_lift.m.read8(0x40, 0x49),
                rt_oracle.m.read8(0x40, 0x49),
                "{name}: BIOS video mode"
            );
            assert_eq!(rt_lift.mouse_shown, rt_oracle.mouse_shown, "{name}: mouse_shown");
            let gs = l.gs;
            for &off in gs_checks {
                assert_eq!(
                    rt_lift.m.read16(gs, off as u32),
                    rt_oracle.m.read16(gs, off as u32),
                    "{name}: gs:[{off:#x}]"
                );
            }
            for &off in seg0_checks {
                assert_eq!(
                    rt_lift.m.read16(0, off as u32),
                    rt_oracle.m.read16(0, off as u32),
                    "{name}: 0:[{off:#x}] (IVT)"
                );
            }
        }
    }

    /// The Runtime-context I/O lifts run correctly: they preserve their pushed
    /// registers (balanced push/pop, net-zero SP) and their `int`s route through
    /// native_int with the right side effects — proving the I/O-lift architecture.
    #[test]
    fn io_lifts_preserve_state_and_service_interrupts() {
        // func_cc0: sets video mode from gs:0x5232, preserves AX.
        let mut rt = test_runtime();
        rt.m.regs.ss = 0x2000;
        rt.m.regs.set_sp(0x0100);
        rt.m.regs.set_ax(0xBEEF);
        let gs = rt.m.regs.gs;
        rt.m.write8(gs, 0x5232, 0x03);
        func_cc0(&mut rt);
        assert_eq!(rt.m.regs.ax(), 0xBEEF, "cc0 preserves AX");
        assert_eq!(rt.m.regs.sp(), 0x0100, "cc0 net-zero SP");
        assert_eq!(rt.m.read8(0x40, 0x49), 0x03, "cc0 set the video mode");

        // func_d4a: set mouse H/V range; preserves AX/BX/CX/DX.
        let mut rt = test_runtime();
        rt.m.regs.ss = 0x2000;
        rt.m.regs.set_sp(0x0100);
        rt.m.regs.set_ax(0x0010);
        rt.m.regs.set_bx(0x0130);
        rt.m.regs.set_cx(0xAAAA);
        rt.m.regs.set_dx(0xBBBB);
        func_d4a(&mut rt);
        assert_eq!(rt.m.regs.sp(), 0x0100, "d4a net-zero SP");
        assert_eq!((rt.m.regs.ax(), rt.m.regs.bx()), (0x0010, 0x0130), "d4a preserves AX/BX");
        assert_eq!((rt.m.regs.cx(), rt.m.regs.dx()), (0xAAAA, 0xBBBB), "d4a preserves CX/DX");

        // func_cef: reset+hide the mouse; reset (fn 0) sets the driver's state.
        let mut rt = test_runtime();
        rt.m.regs.ss = 0x2000;
        rt.m.regs.set_sp(0x0100);
        rt.m.regs.set_ax(0x1234);
        func_cef(&mut rt);
        assert_eq!(rt.m.regs.sp(), 0x0100, "cef net-zero SP");
        assert_eq!(rt.m.regs.ax(), 0x1234, "cef preserves AX");
        assert_eq!(rt.mouse_shown, -2, "cef: reset(-1) then hide(-1) → -2");
    }
}
