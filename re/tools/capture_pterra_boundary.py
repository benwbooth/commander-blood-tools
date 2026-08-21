#!/usr/bin/env python3
"""Freeze the guest at the Pterra materialization boundary and dump its state.

The crash signature: entering Pterra creates PTERRA1D/G/F.LBM in the write
directory, then (relinked build only) the guest corrupts memory and dies in an
INT-1 storm. The moment those files FIRST appear on the host is a matched
story-state boundary that exists in BOTH binaries -- original and relinked --
so dumping there makes their states directly comparable.

Launches dosbox-x exactly like BLOOD.BAT does, polls the write directory for
the first PTERRA*.LBM create, then ptrace-attaches, locates DS via the same
anchor strings as dump_dosbox_mem.py, and writes a JSON snapshot:

    { cpu: cs:ip + segments + registers,
      ivt: 1024 bytes of interrupt vectors,
      resource_band: words DS:0x0A40..0x0B00,
      back_buffer_area: bytes DS:0x5219..0x5240 }

Usage:
  python3 -P re/tools/capture_pterra_boundary.py \
      --cd-dir output/recovered_dos_package/cd --executable BPRG_RE.EXE \
      --install-parent /tmp/... --output state.json [--display :83]
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
import re
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LOCATOR_ANCHOR = b"386 minimum "
PTRACE_ATTACH = 16
PTRACE_DETACH = 17
PTRACE_CONT = 7


def libc_ptrace():
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long,
                            ctypes.c_void_p, ctypes.c_void_p]
    return libc


def locate_cpu_state(pid):
    executable = os.path.realpath(f"/proc/{pid}/exe")
    symbols = {}
    output = subprocess.check_output(
        ["nm", "-P", executable], text=True, stderr=subprocess.DEVNULL)
    for line in output.splitlines():
        fields = line.split()
        if len(fields) >= 3 and fields[0] in ("Segs", "cpu_regs"):
            symbols[fields[0]] = int(fields[2], 16)
    if set(symbols) != {"Segs", "cpu_regs"}:
        return None
    image_base = None
    with open(f"/proc/{pid}/maps", encoding="ascii") as maps:
        for line in maps:
            fields = line.split()
            if len(fields) < 6:
                continue
            mapped = fields[-1].removesuffix(" (deleted)")
            if os.path.realpath(mapped) != executable:
                continue
            start = int(fields[0].split("-", 1)[0], 16)
            offset = int(fields[2], 16)
            image_base = start - offset
            break
    if image_base is None:
        return None
    return {name: image_base + offset for name, offset in symbols.items()}


def read_cpu_state(mem, addresses):
    if addresses is None:
        return None
    mem.seek(addresses["cpu_regs"])
    registers = struct.unpack("<8I", mem.read(32))
    ip = struct.unpack("<I", mem.read(4))[0]
    segments = []
    for index in range(6):
        mem.seek(addresses["Segs"] + index * 8)
        segments.append(struct.unpack("<Q", mem.read(8))[0] & 0xffff)
    es, cs, ss, ds, fs, gs = segments
    return {"cs": cs, "ip": ip & 0xFFFF, "ds": ds, "es": es, "ss": ss,
            "fs": fs, "gs": gs,
            # dosbox cpu_regs stores AX,CX,DX,BX,SP,BP,SI,DI (x86 order)
            "ax": registers[0] & 0xFFFF, "cx": registers[1] & 0xFFFF,
            "dx": registers[2] & 0xFFFF, "bx": registers[3] & 0xFFFF,
            "si": registers[6] & 0xFFFF, "di": registers[7] & 0xFFFF}


def find_ds_anchor(pid, mem):
    best = None
    guest_memory_base = None
    for line in open(f"/proc/{pid}/maps"):
        pr = line.split()
        if "r" not in pr[1] or "-" not in pr[0]:
            continue
        a, b = [int(x, 16) for x in pr[0].split("-")]
        if b - a > 300_000_000:
            continue
        try:
            mem.seek(a)
            buf = mem.read(b - a)
        except Exception:
            continue
        for match in re.finditer(re.escape(LOCATOR_ANCHOR), buf):
            anchor = a + match.start()
            mem.seek(anchor + 0x0A46)
            free_bytes = struct.unpack("<I", mem.read(4))[0]
            mem.seek(anchor + 0x0A9E)
            crtc_port = struct.unpack("<H", mem.read(2))[0]
            if 0 < free_bytes <= 0x000A0000 and crtc_port in (0, 0x03D4):
                best = anchor
                guest_memory_base = a
                break
        if best:
            break
    return best, guest_memory_base


def bridge_prefix_actions() -> list[str]:
    """The proven gate prefix: logos -> title click -> CRYOBOX -> Bob."""
    return [
        "wait 6", "key Escape", "wait 2", "click 348 344", "wait 2",
        "move_relative -300 0", "wait 4",
        "move_relative -300 0", "wait 3",
        "move_relative 100 -20", "wait 0.5",
        "move_relative 100 -20", "wait 0.5",
        "mouse_button 1", "wait 1",
        "move_relative -100 -20", "wait 0.5",
        "move_relative -100 -20", "wait 0.5",
        "mouse_button 1", "fastforward 8", "wait 3",
        "move_relative 100 0", "wait 0.5", "mouse_button 1",
        "fastforward 5", "wait 1",
        "shot d_title",
        "key_down space", "fastforward 30", "key_up space",
        "fastforward 10", "wait 2", "shot d_bridge", "wait_bridge",
    ]


def rotation_lap(lap: int) -> list[str]:
    """One full rotation attempt: park right edge, center, orb click.

    The orb click coordinates come from accuracy/scenarios/nav_probe.tsv
    (125,118 in 320-space = 250,236 in the 640x400 window).
    """
    actions: list[str] = []
    for _ in range(6):
        actions += ["move 310 100", "wait 2"]
    actions += ["move 160 100", "wait 1"]
    for index in range(4):
        actions += [f"click {246 + 8 * index} 236", "wait 1"]
    actions += ["wait 3"]
    return actions


def run_driver(actions: list[str], display: str, executable: str) -> None:
    """Feed drive_real_game.sh's action vocabulary through xdotool directly.

    Re-implemented here (not via the shell driver) because the game is
    already running under this script's control.
    """
    env = dict(os.environ, DISPLAY=display, SDL_VIDEODRIVER="x11")
    window_id = ""
    for _ in range(40):
        output = subprocess.run(
            ["xdotool", "search", "--name", f"{executable}|DOSBox"],
            capture_output=True, text=True, env=env).stdout
        lines = [line for line in output.splitlines() if line.strip()]
        if lines:
            window_id = lines[0]
            break
        time.sleep(0.5)
    if not window_id:
        print("drive: game window not found")
        return
    # Focus + activate once, then use GLOBAL (XTEST) button events like
    # drive_real_game.sh does: per-window synthetic button events are
    # ignored by SDL's event pump on some windows.
    subprocess.run(["xdotool", "windowactivate", "--sync", window_id],
                   env=env)
    subprocess.run(["xdotool", "windowfocus", "--sync", window_id], env=env)

    def emit(action: str, a: str, b: str) -> None:
        if action == "click":
            subprocess.run(["xdotool", "mousemove", "--window", window_id,
                            a, b], env=env)
            time.sleep(0.3)
            subprocess.run(["xdotool", "mousedown", "1"], env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "mouseup", "1"], env=env)
        elif action == "move":
            subprocess.run(["xdotool", "mousemove", "--window", window_id,
                            str(int(a) * 2), str(int(b) * 2)], env=env)
        elif action == "move_relative":
            subprocess.run(["xdotool", "mousemove_relative", "--", a, b],
                           env=env)
        elif action == "mouse_button":
            subprocess.run(["xdotool", "mousedown", a], env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "mouseup", a], env=env)
        elif action == "key":
            subprocess.run(["xdotool", "keydown", "--window", window_id, a],
                           env=env)
            time.sleep(0.2)
            subprocess.run(["xdotool", "keyup", "--window", window_id, a],
                           env=env)
        elif action == "key_down":
            subprocess.run(["xdotool", "keydown", "--window", window_id, a],
                           env=env)
        elif action == "key_up":
            subprocess.run(["xdotool", "keyup", "--window", window_id, a],
                           env=env)  # keys stay window-targeted (safe synth)

    for line in actions:
        parts = line.split()
        if not parts:
            continue
        verb = parts[0]
        arguments = parts[1:] + ["", ""]
        if verb == "wait" or verb == "fastforward":
            duration = float(arguments[0])
            if verb == "fastforward":
                subprocess.run(["xdotool", "keydown", "--window", window_id,
                                "Alt_L"], env=env)
                subprocess.run(["xdotool", "keydown", "--window", window_id,
                                "F12"], env=env)
                time.sleep(duration)
                subprocess.run(["xdotool", "keyup", "--window", window_id,
                                "F12"], env=env)
                subprocess.run(["xdotool", "keyup", "--window", window_id,
                                "Alt_L"], env=env)
            else:
                time.sleep(duration)
            continue
        elif verb == "wait_bridge":
            # Feedback gate: the bridge frame is blue-dominant with low
            # green deviation; the title/cinematics are brighter and
            # noisier. Retry dismissal until the classifier agrees.
            for attempt in range(8):
                probe = f"/tmp/opencode/driveshots/probe_{attempt}.png"
                subprocess.run(["import", "-window", window_id, probe],
                               env=env)
                stats = subprocess.run(
                    ["magick", probe, "-format",
                     "%[fx:mean.r] %[fx:mean.g] %[fx:mean.b] "
                     "%[fx:standard_deviation.g]", "info:"],
                    capture_output=True, text=True, env=env).stdout
                values = [float(text) for text in stats.split()]
                if len(values) == 4 and values[2] > 0.12 \
                        and values[0] < 0.15 and values[3] < 0.12:
                    print(f"drive: bridge reached on attempt {attempt}")
                    break
                emit("key", "Escape", "")
                time.sleep(1)
                emit("click", "348", "344")
                time.sleep(3)
        elif verb == "shot":
            out_dir = os.environ.get("DRIVE_SHOT_DIR", ".")
            subprocess.run(["import", "-window", window_id,
                            f"{out_dir}/{arguments[0]}.png"],
                           env=env)
        else:
            emit(verb, arguments[0], arguments[1])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cd-dir", type=Path, required=True)
    parser.add_argument("--executable", default="BPRG_RE.EXE")
    parser.add_argument("--install-parent", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--display", default=":83")
    parser.add_argument("--timeout", type=float, default=900.0,
                        help="seconds to wait for the boundary")
    parser.add_argument("--drive", action="store_true",
                        help="script the navigation instead of a human")
    parser.add_argument("--display-for-drive", default=None,
                        help="X display holding the game when driving")
    args = parser.parse_args()

    env = dict(os.environ, DISPLAY=args.display, SDL_VIDEODRIVER="x11")
    xvfb = None
    if not args.display.startswith(":0"):
        xvfb = subprocess.Popen(
            ["Xvfb", args.display, "-screen", "0", "800x600x24"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(2.0)
    libc = libc_ptrace()
    snapshot = None
    try:
        dosbox_args = [
            "dosbox-x", "--noprimaryconf", "--nolocalconf",
            "-set", "sdl output=surface",
            "-set", "cpu cycles=max",
            "-set", "cpu core=dynamic",
            "-set", "render frameskip=10",
            "-c", f"mount c {args.install_parent}",
            "-c", f"mount d {args.cd_dir} -t cdrom",
            "-c", "d:",
            "-c", f"{args.executable} AMR S162227 EMS WRIC:\\cblood\\",
        ]
        db = subprocess.Popen(dosbox_args, env=env,
                              stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL)
        if args.drive:
            import threading
            drive_display = args.display_for_drive or args.display
            drive_actions = bridge_prefix_actions()
            for lap in range(6):
                drive_actions += [f"shot g_lap{lap}"] + rotation_lap(lap)

            def drive() -> None:
                time.sleep(3.0)  # let the window appear
                run_driver(drive_actions, drive_display, args.executable)

            threading.Thread(target=drive, daemon=True).start()
        deadline = time.time() + args.timeout
        marker = args.install_parent / "cblood"
        hit = None
        while time.time() < deadline:
            if db.poll() is not None:
                print(f"dosbox exited early with {db.returncode}")
                break
            try:
                hit = next(marker.glob("PTERRA*"), None)
            except StopIteration:
                hit = None
            if hit is not None:
                break
            time.sleep(0.02)
        if hit is None:
            print("boundary never reached (no PTERRA file created)")
            return
        print(f"boundary marker: {hit.name}")
        time.sleep(0.05)  # let the creating instruction fully retire
        if libc.ptrace(PTRACE_ATTACH, db.pid, None, None) != 0:
            print("ptrace attach failed", ctypes.get_errno())
            return
        os.waitpid(db.pid, 0)
        with open(f"/proc/{db.pid}/mem", "rb") as mem:
            cpu_addresses = locate_cpu_state(db.pid)
            best, guest_base = find_ds_anchor(db.pid, mem)
            if not best:
                print("DS anchor not found")
                return
            snapshot = {}
            state = read_cpu_state(mem, cpu_addresses)
            if state:
                snapshot["cpu"] = state
                delta_segments = {
                    key: value - (best - guest_base) // 16
                    for key, value in state.items()
                    if key in ("ds", "es", "ss", "fs", "gs", "cs")
                }
                snapshot["segments_minus_ds_anchor"] = delta_segments
            mem.seek(guest_base)
            snapshot["ivt"] = mem.read(0x400).hex()
            band = {}
            for offset in range(0x0A40, 0x0B00, 2):
                mem.seek(best + offset)
                band[f"{offset:#06x}"] = struct.unpack(
                    "<H", mem.read(2))[0]
            snapshot["resource_band"] = band
            mem.seek(best + 0x5219)
            snapshot["back_buffer_area"] = mem.read(0x5240 - 0x5219).hex()
            snapshot["marker"] = str(hit)
        args.output.write_text(json.dumps(snapshot, indent=1))
        print(f"wrote {args.output}")
    finally:
        if True:
            # keep or kill? kill: the relinked guest would storm anyway.
            try:
                libc.ptrace(PTRACE_DETACH, db.pid, None, None)
            except Exception:
                pass
            try:
                os.kill(db.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        xvfb.terminate()


if __name__ == "__main__":
    main()
