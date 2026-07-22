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

/// Emit a byte `out port, al` from a lifted function — routes to the same port handler the
/// interpreter's run loop uses for `Exit::Out`.
fn out8(rt: &mut Runtime, port: u16, val: u8) {
    rt.port_out(port, 1, val as u32);
}

/// `func_79c` (`install_timer_isr_hook`, 0x079C): save the original INT 08h vector (int21 fn
/// 0x35) to gs:[0xB1D]/[0xB1F], install the game's own PIT handler (int21 fn 0x25 → cs:0x213),
/// then reprogram PIT channel 0 to ~200 Hz (out 0x43=0x36; out 0x40 = 0x1746 lo then hi) and set
/// the timer-state bytes/words at gs:[0xB21..0xB27]. Hooks the tick, chaining to the saved vector.
pub fn func_79c(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.bx());
    push16(rt, rt.m.regs.dx());
    push16(rt, rt.m.regs.es);
    push16(rt, rt.m.regs.ds);
    rt.m.regs.set_ax(0x3508); // get INT 08h vector -> es:bx
    int_call(rt, 0x21);
    let (bx, es) = (rt.m.regs.bx(), rt.m.regs.es);
    let gs = rt.m.regs.gs;
    rt.m.write16(gs, 0xb1d, bx);
    rt.m.write16(gs, 0xb1f, es);
    rt.m.regs.set_ah(0x25); // set-vector; al still 0x08
    let cs = rt.m.regs.cs;
    rt.m.regs.set_bx(cs);
    rt.m.regs.ds = cs; // mov bx,cs; mov ds,bx
    rt.m.regs.set_dx(0x0213);
    int_call(rt, 0x21); // set INT 08h -> cs:0x213
    // cli — no IF modelling needed for a leaf; the ints are already serviced.
    out8(rt, 0x43, 0x36); // PIT: ch0, mode 3, lo/hi byte
    rt.m.regs.set_ax(0x1746); // ~200 Hz divisor
    out8(rt, 0x40, rt.m.regs.al()); // divisor low (0x46)
    let ah = rt.m.regs.ah();
    rt.m.regs.set_al(ah); // mov al,ah
    out8(rt, 0x40, rt.m.regs.al()); // divisor high (0x17)
    rt.m.write8(gs, 0xb21, 1);
    rt.m.write8(gs, 0xb22, 0xb);
    rt.m.write16(gs, 0xb27, 0x19);
    rt.m.write16(gs, 0xb25, 3);
    // sti
    let ds = pop16(rt);
    rt.m.regs.ds = ds;
    let es = pop16(rt);
    rt.m.regs.es = es;
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let bx = pop16(rt);
    rt.m.regs.set_bx(bx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_2dd3` (`cmos_rtc_read`, 0x2DD3): select CMOS register 0 (out 0x70=0), read it (in 0x71)
/// into AL, duplicate into AH, and store the word to cs:[0xAEE] — seeds the game's PRNG from the
/// RTC seconds. Exercises the `in` path; AX is preserved (pushed/popped).
pub fn func_2dd3(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    rt.m.regs.set_ax(0); // xor ax,ax
    out8(rt, 0x70, 0); // select CMOS register 0
    let al = rt.port_in(0x71, 1) as u8; // in al, 0x71
    rt.m.regs.set_al(al);
    rt.m.regs.set_ah(al); // mov ah, al
    let cs = rt.m.regs.cs;
    let ax = rt.m.regs.ax();
    rt.m.write16(cs, 0xaee, ax); // cs:[0xaee] = ax
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_2f90` (`vga_palette_write`, 0x2F90): reset the DAC write index (out 0x3C8=0) then upload
/// 768 bytes (256 RGB triples) from ds:si to the DAC data port via `rep outsb` — loads the full
/// palette. SI advances during the copy but is restored (pushed/popped), so the caller's SI is
/// preserved. Assumes DF=0 (forward), the game's normal direction.
pub fn func_2f90(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.cx());
    push16(rt, rt.m.regs.dx());
    push16(rt, rt.m.regs.si());
    rt.m.regs.set_dx(0x3c8);
    rt.m.regs.set_al(0);
    out8(rt, 0x3c8, 0); // reset PEL write index
    rt.m.regs.set_dx(0x3c9); // inc dl
    rt.m.regs.set_cx(0x300);
    let ds = rt.m.regs.ds;
    let mut si = rt.m.regs.si();
    while rt.m.regs.cx() != 0 {
        let b = rt.m.read8(ds, si as u32); // rep outsb: out 0x3c9, [ds:si]
        out8(rt, 0x3c9, b);
        si = si.wrapping_add(1);
        rt.m.regs.set_cx(rt.m.regs.cx().wrapping_sub(1));
    }
    rt.m.regs.set_si(si);
    let si = pop16(rt);
    rt.m.regs.set_si(si);
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let cx = pop16(rt);
    rt.m.regs.set_cx(cx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_2fa6` (`vga_dac_clear`, 0x2FA6): reset the DAC write index (out 0x3C8=0) then write 768
/// zero bytes to the DAC data port (out 0x3C9) via a `loop` — blanking all 256 palette entries.
pub fn func_2fa6(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.cx());
    push16(rt, rt.m.regs.dx());
    rt.m.regs.set_dx(0x3c8);
    rt.m.regs.set_al(0); // xor al,al
    out8(rt, 0x3c8, 0); // reset PEL write index
    rt.m.regs.set_dx(0x3c9); // inc dl
    rt.m.regs.set_cx(0x300);
    while rt.m.regs.cx() != 0 {
        out8(rt, 0x3c9, 0);
        rt.m.regs.set_cx(rt.m.regs.cx().wrapping_sub(1)); // loop
    }
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
    let cx = pop16(rt);
    rt.m.regs.set_cx(cx);
    let ax = pop16(rt);
    rt.m.regs.set_ax(ax);
}

/// `func_b32` (`detect_cdrom`, 0x0B32): `int 2Fh` AX=0x1500 (MSCDEX installation check) →
/// BX = CD-ROM drive count; store gs:[0xAE6] = (BX != 0). A `near` ret helper (no register
/// preservation — it clobbers AX/BX). The game ships on CD, so this gates the CD path.
pub fn func_b32(rt: &mut Runtime) {
    rt.m.regs.set_ax(0x1500);
    rt.m.regs.set_bx(0);
    int_call(rt, 0x2f); // MSCDEX check → BX = drive count
    let present = rt.m.regs.bx() != 0; // or bx,bx; setne
    let gs = rt.m.regs.gs;
    rt.m.write8(gs, 0xae6, present as u8);
}

/// `func_7ea` (`program_pit` / restore-timer, 0x07EA): reprogram PIT channel 0 back to the
/// default ~18.2 Hz (out 0x43=0x36; out 0x40 = 0xFFFF lo/hi), clear the timer-active flag
/// gs:[0xB21], and restore the original INT 08h vector saved by [`func_79c`] at
/// gs:[0xB1D]/[0xB1F] (int21 fn 0x25). The teardown counterpart of func_79c.
pub fn func_7ea(rt: &mut Runtime) {
    push16(rt, rt.m.regs.ax());
    push16(rt, rt.m.regs.dx());
    push16(rt, rt.m.regs.ds);
    out8(rt, 0x43, 0x36);
    out8(rt, 0x40, 0xff);
    out8(rt, 0x40, 0xff); // divisor 0xFFFF -> default tick rate
    let gs = rt.m.regs.gs;
    rt.m.write8(gs, 0xb21, 0);
    let saved_seg = rt.m.read16(gs, 0xb1f); // ax=gs:[0xb1f]; ds=ax
    rt.m.regs.ds = saved_seg;
    let saved_off = rt.m.read16(gs, 0xb1d);
    rt.m.regs.set_dx(saved_off);
    rt.m.regs.set_ax(0x2508); // set INT 08h vector, al=0x08
    int_call(rt, 0x21);
    let ds = pop16(rt);
    rt.m.regs.ds = ds;
    let dx = pop16(rt);
    rt.m.regs.set_dx(dx);
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
        use super::super::interp::Exit;
        for _ in 0..100_000 {
            match rt.cpu.run(&mut rt.m, 4096) {
                Exit::Ret | Exit::Retf => return,
                Exit::Int { vector } => int_call(rt, vector),
                // Service port I/O exactly as Runtime::run does (byte-wise; these leaves only OUT
                // single bytes), through the same port handlers the lifts call.
                Exit::Out { port, size, value } => {
                    for i in 0..size as u16 {
                        rt.port_out(port.wrapping_add(i), 1, (value >> (i * 8)) & 0xff);
                    }
                }
                Exit::In { port, size } => {
                    let v = rt.port_in(port, 1);
                    match size {
                        1 => rt.m.regs.set_al(v as u8),
                        _ => rt.m.regs.set_ax(v as u16),
                    }
                }
                Exit::StepLimit => continue,
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
        // Pre-existing INT 08h vector so func_79c's get-vector (int21 fn 0x35) returns the same
        // es:bx on both sides (the lift has the Runtime default; the oracle has EXE-overlay bytes).
        rt.m.write16(0, 0x08 * 4, 0x1234);
        rt.m.write16(0, 0x08 * 4 + 2, 0x5678);
        // A saved INT 08h vector for func_7ea to restore (gs:[0xb1d]=off, gs:[0xb1f]=seg).
        rt.m.write16(0x3000, 0xb1d, 0x1234);
        rt.m.write16(0x3000, 0xb1f, 0x5678);
        // Non-zero DAC so func_2fa6's clear is observable (a wrong loop count leaves stale entries).
        rt.dac = [0x2a; 768];
        // Palette source for func_2f90 at ds:si = 0x3000:0x6000 (above the code overlay, clear of
        // the gs fields below 0x6000): 768 varied bytes so a mis-indexed copy is caught.
        rt.m.regs.ds = 0x3000;
        rt.m.regs.set_si(0x6000);
        for i in 0..768u32 {
            rt.m.write8(0x3000, 0x6000 + i, (i * 7 + 1) as u8);
        }
    }

    /// Every Runtime-context I/O lift reproduces the REAL function bytes exactly: run the lift and
    /// the interpreter-over-the-original-bytes from an identical seed state and assert the full
    /// observable state (all GP regs, SP, ES, video mode, mouse state) is bit-identical. This is
    /// the same "oracle-verified" standard the pure-CPU lifts have — here the interpreter is the
    /// oracle. Adding a leaf to this table is the gate for calling it verified.
    #[test]
    fn io_lifts_match_interpreter_oracle() {
        let Some(exe) = load_exe() else { return };
        // (name, file offset, lifted fn, gs offsets written, seg-0/IVT offsets written, cmp DAC?)
        // — every listed offset is compared word-for-word against the real bytes' output.
        let leaves: &[(&str, u16, fn(&mut Runtime), &[u16], &[u16], bool)] = &[
            ("func_cc0", 0x0cc0, func_cc0, &[], &[], false),
            ("func_d4a", 0x0d4a, func_d4a, &[], &[], false),
            ("func_cef", 0x0cef, func_cef, &[], &[], false),
            ("func_d0e", 0x0d0e, func_d0e, &[0xa2a, 0xa2c, 0xa2e, 0xa38, 0xa3a, 0xb3b], &[], false),
            // INT 23h vector at 0:[0x8c/0x8e], INT 24h vector at 0:[0x90/0x92].
            ("func_bff", 0x0bff, func_bff, &[], &[0x8c, 0x8e, 0x90, 0x92], false),
            // saved vector gs:[0xb1d/0xb1f], timer-state gs:[0xb21/0xb25/0xb27]; INT 08h at 0:[0x20/0x22].
            ("func_79c", 0x079c, func_79c, &[0xb1d, 0xb1f, 0xb21, 0xb25, 0xb27], &[0x20, 0x22], false),
            // clears gs:[0xb21]; restores INT 08h at 0:[0x20/0x22] to the saved gs:[0xb1d/0xb1f].
            ("func_7ea", 0x07ea, func_7ea, &[0xb21], &[0x20, 0x22], false),
            // CD-present flag gs:[0xae6] (byte); a near-ret leaf via int 2Fh.
            ("func_b32", 0x0b32, func_b32, &[0xae6], &[], false),
            // blanks all 256 DAC entries — compare the full 768-byte palette.
            ("func_2fa6", 0x2fa6, func_2fa6, &[], &[], true),
            // uploads 768 bytes from ds:si to the DAC (rep outsb) — compare the full palette.
            ("func_2f90", 0x2f90, func_2f90, &[], &[], true),
            // reads CMOS RTC (in 0x71) → cs:[0xaee] (cs=0 here, so a segment-0 word write).
            ("func_2dd3", 0x2dd3, func_2dd3, &[], &[0xaee], false),
        ];
        for &(name, offset, lift, gs_checks, seg0_checks, check_dac) in leaves {
            let mut rt_lift = test_runtime();
            seed(&mut rt_lift);
            lift(&mut rt_lift);

            let mut rt_oracle = test_runtime();
            // Mirror only the CODE region (all leaves live below 0x2000) at physical 0. Keeping the
            // mirror below the gs (0x3000→0x30000) and ss (0x2000→0x20000) segments means the gs
            // scratch and stack start pristine (zero) on BOTH sides — so a byte-sized gs write
            // compares equal even on its untouched neighbor byte. Seed runs after, as before.
            const OVERLAY: usize = 0x10000;
            let n = exe.len().min(OVERLAY);
            rt_oracle.m.mem[..n].copy_from_slice(&exe[..n]);
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
            if check_dac {
                assert!(rt_lift.dac == rt_oracle.dac, "{name}: DAC palette differs");
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
