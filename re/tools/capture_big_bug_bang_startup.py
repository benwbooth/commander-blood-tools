#!/usr/bin/env python3
"""Read original sequel startup state in an isolated DOSBox-X child.

Offline evidence only: no guest writes or host mouse control. An optional
recorded button press targets only the freshly allocated private X display.
The capture describes sampled allocations; it cannot prove which instructions
read them between samples. Unknown executable or emulator layouts fail closed.
"""

import argparse
from contextlib import contextmanager
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import select
import struct
import subprocess
import time

EXECUTABLE_SHA256 = "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
MZ_HEADER_SIZE = 2048
GLOBAL_FILE = 0xF7F0
CATALOG_SEGMENT_FILE = 0xE190
CATALOG_NAMES_FILE = 0xED94
PROFILE_TABLE_FILE = 0xF744
VM_FILE = 0x5820
PROFILE_INDEX = 0x6B50
PROFILE_HANDLES = 0x6AE2
PROFILE_BINDINGS = 0x6AEC
RESOURCE_COUNT = 155
NAME_SIZE = 16
HANDLE_SIZE = 8
TIME_OFFSET = 8368
GUEST_BYTES = 1048576
ROLES = ("var", "deb", "cod", "bas", "dic")
PTRACE_ATTACH = 16
PTRACE_DETACH = 17


def read_exact(stream, address, size):
    stream.seek(address)
    data = stream.read(size)
    if len(data) != size:
        raise ValueError(f"short read at {address:#x}: {len(data)} != {size}")
    return data


def locate_symbols(pid):
    """Resolve the actual child ELF, not a Nix shell wrapper on PATH."""
    executable = Path(f"/proc/{pid}/exe").resolve()
    header = executable.open("rb")
    with header:
        elf = header.read(20)
    if elf[:6] != b"\x7fELF\x02\x01":
        raise ValueError("capture requires a little-endian ELF64 emulator")
    kind = struct.unpack_from("<H", elf, 16)[0]
    if kind not in (2, 3):
        raise ValueError(f"unsupported ELF type {kind}")
    image_base = 0
    if kind == 3:
        for line in Path(f"/proc/{pid}/maps").read_text().splitlines():
            fields = line.split(maxsplit=5)
            if len(fields) == 6 and fields[5] == str(executable) and int(fields[2], 16) == 0:
                image_base = int(fields[0].split("-")[0], 16)
                break
        else:
            raise ValueError("emulator ELF base is not mapped")
    wanted = {"MemBase", "Segs", "cpu_regs"}
    symbols = {}
    for row in subprocess.check_output(["nm", "-P", str(executable)], text=True).splitlines():
        fields = row.split()
        if len(fields) >= 4 and fields[0] in wanted:
            symbols[fields[0]] = (image_base + int(fields[2], 16), int(fields[3], 16))
    if set(symbols) != wanted or symbols["MemBase"][1] != 8:
        raise ValueError("DOSBox-X memory/register symbols unavailable")
    if symbols["Segs"][1] != 136 or symbols["cpu_regs"][1] != 48:
        raise ValueError(f"unverified DOSBox-X register layout: {symbols}")
    return executable, symbols


def read_cpu(stream, symbols):
    values = struct.unpack("<10I", read_exact(stream, symbols["cpu_regs"][0], 40))
    # DOSBox may keep arithmetic flags lazily; this is not a materialized EFLAGS value.
    state = dict(zip(("eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "eip", "stored_flags"), values))
    for index, name in enumerate(("es", "cs", "ss", "ds", "fs", "gs")):
        state[name] = struct.unpack("<H", read_exact(stream, symbols["Segs"][0] + index * 8, 2))[0]
    return state


def inspect_guest(guest, executable):
    """Find the loaded module with independent data, code and catalog anchors."""
    if len(guest) != GUEST_BYTES:
        raise ValueError("expected exactly one MiB of DOS memory")
    anchor = executable[GLOBAL_FILE:GLOBAL_FILE + 44]
    names_raw = executable[CATALOG_NAMES_FILE:CATALOG_NAMES_FILE + RESOURCE_COUNT * NAME_SIZE]
    names = [names_raw[i:i + NAME_SIZE].split(b"\0", 1)[0].decode("ascii")
             for i in range(0, len(names_raw), NAME_SIZE)]
    candidates = []
    start = 0
    while (position := guest.find(anchor, start)) >= 0:
        start = position + 1
        module = position - (GLOBAL_FILE - MZ_HEADER_SIZE)
        catalog = module + CATALOG_SEGMENT_FILE - MZ_HEADER_SIZE
        vm = module + VM_FILE - MZ_HEADER_SIZE
        name_address = module + CATALOG_NAMES_FILE - MZ_HEADER_SIZE
        if module < 0 or module % 16 or position + 65536 > len(guest):
            continue
        if guest[vm:vm + 16] != executable[VM_FILE:VM_FILE + 16]:
            continue
        # The DOS loader uppercases catalog names in place as it opens files.
        if guest[name_address:name_address + len(names_raw)].upper() != names_raw.upper():
            continue
        candidates.append((position, catalog))
    if not candidates:
        return {"status": "module_not_found"}
    if len(candidates) != 1:
        return {"status": "ambiguous_modules", "candidates": candidates}
    global_base, catalog = candidates[0]
    profile = struct.unpack_from("<H", guest, global_base + PROFILE_INDEX)[0]
    handles = struct.unpack_from("<5H", guest, global_base + PROFILE_HANDLES)
    resources = []
    for identity, name in enumerate(names):
        segment, flags, size = struct.unpack_from("<HHI", guest, catalog + identity * HANDLE_SIZE)
        if flags & 3:
            resources.append({"id": identity, "name": name, "linear": segment * 16,
                              "allocated_bytes": size, "flags": flags})
    bindings = {}
    for index, role in enumerate(ROLES):
        offset, segment = struct.unpack_from("<HH", guest, global_base + PROFILE_BINDINGS + index * 4)
        linear = segment * 16 + offset
        owners = [row for row in resources if row["linear"] <= linear < row["linear"] + row["allocated_bytes"]]
        bindings[role] = {"handle": handles[index], "linear": linear,
                          "owners": [row["id"] for row in owners]}
    state = {"status": "module_found", "global_segment": global_base // 16,
             "catalog_segment": catalog // 16, "profile": profile,
             "bindings": bindings, "resident_resources": resources}
    if not 0 <= profile < 17:
        return state
    expected = struct.unpack_from("<5H", executable, PROFILE_TABLE_FILE + profile * 10)
    # Noninitial profiles retain the original VAR handle while replacing four companions.
    expected = (2 if profile else expected[0], *expected[1:])
    bindings_consistent = tuple(handles) == expected
    for role in ("var", "deb", "cod", "dic"):
        binding = bindings[role]
        bindings_consistent &= binding["owners"] == [binding["handle"]]
    state["bindings_consistent"] = bindings_consistent
    if not bindings_consistent:
        return state
    state["status"] = "profile_bound"
    var = next(row for row in resources if row["id"] == handles[0])
    var_start = var["linear"]
    var_end = var_start + var["allocated_bytes"]
    if var_end > len(guest):
        raise ValueError("VAR allocation exceeds captured RAM")
    state["var_sha256"] = hashlib.sha256(guest[var_start:var_end]).hexdigest()
    address = bindings["var"]["linear"] + TIME_OFFSET
    if address + 2 > len(guest):
        raise ValueError("time observation exceeds captured RAM")
    owners = [dict(row, offset=address - row["linear"]) for row in resources
              if row["linear"] <= address and address + 2 <= row["linear"] + row["allocated_bytes"]]
    state["time_storage"] = {"linear": address, "value": struct.unpack_from("<H", guest, address)[0],
                              "owners": owners, "belongs_to_var": any(row["id"] == handles[0] for row in owners)}
    return state


@contextmanager
def stopped_child(process):
    libc = ctypes.CDLL(None, use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long, ctypes.c_void_p, ctypes.c_void_p]
    if libc.ptrace(PTRACE_ATTACH, process.pid, None, None) != 0:
        raise OSError(ctypes.get_errno(), "ptrace attach to capture child failed")
    try:
        _pid, status = os.waitpid(process.pid, 0)
        if not os.WIFSTOPPED(status):
            raise RuntimeError(f"child did not stop: {status}")
        yield
    finally:
        if libc.ptrace(PTRACE_DETACH, process.pid, None, None) != 0:
            raise OSError(ctypes.get_errno(), "ptrace detach from capture child failed")


def stop(process):
    if process is None:
        return
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
    process.wait()
    if process.stdout is not None:
        process.stdout.close()


def positive_seconds(value):
    result = float(value)
    if not math.isfinite(result) or result <= 0:
        raise argparse.ArgumentTypeError("must be finite and positive")
    return result


def private_click(env, pid):
    """One button press without moving any pointer or editing game memory."""
    windows = subprocess.check_output(
        ["xdotool", "search", "--onlyvisible", "--pid", str(pid)], env=env, text=True, timeout=5).split()
    if len(windows) != 1:
        raise RuntimeError(f"expected one private DOSBox window, got {windows}")
    subprocess.run(["xdotool", "windowfocus", "--sync", windows[0]], env=env, check=True, timeout=5)
    subprocess.run(["xdotool", "mousedown", "1"], env=env, check=True, timeout=5)
    try:
        time.sleep(0.15)
    finally:
        subprocess.run(["xdotool", "mouseup", "1"], env=env, check=True, timeout=5)
    return {"kind": "private_x11_primary_click", "window": windows[0], "display": env["DISPLAY"],
            "pointer_moved": False, "guest_memory_written": False}


def capture(args):
    disc = args.disc.resolve()
    executable = (disc / "BLOOD2PG.EXE").read_bytes()
    if hashlib.sha256(executable).hexdigest() != EXECUTABLE_SHA256:
        raise ValueError("unrecognized BLOOD2PG.EXE; fixed offsets are not applicable")
    output = args.output.resolve()
    for path in (disc, output):
        if any(char in str(path) for char in ('"', '\n', '\r')):
            raise ValueError("DOS mount path contains command syntax")
    output.mkdir(parents=True, exist_ok=False)
    drive = output / "cdrive"
    (drive / "cblood").mkdir(parents=True)
    xvfb = game = None
    report = {"scope": "read-only periodic original-game allocation observations; inputs recorded separately",
              "executable_sha256": EXECUTABLE_SHA256, "disc": str(disc),
              "launch_arguments": args.game_args, "launch_arguments_provenance": "explicit capture setting, not recovered installer output",
              "samples": [], "input_events": []}
    try:
        with (output / "xvfb.log").open("wb") as xlog, (output / "dosbox.log").open("wb") as log:
            xvfb = subprocess.Popen(["Xvfb", "-displayfd", "1", "-screen", "0", "800x600x24", "-nolisten", "tcp"],
                                    stdout=subprocess.PIPE, stderr=xlog)
            if not select.select([xvfb.stdout], [], [], 10)[0]:
                raise RuntimeError("private Xvfb did not publish a display")
            display = xvfb.stdout.readline().decode("ascii").strip()
            if not display.isdecimal() or xvfb.poll() is not None:
                raise RuntimeError("private Xvfb startup failed")
            env = dict(os.environ, DISPLAY=f":{display}", SDL_VIDEODRIVER="x11", SDL_AUDIODRIVER="dummy")
            env.pop("WAYLAND_DISPLAY", None)
            command = [args.dosbox, "-conf", "/dev/null",
                       "-set", "sdl output=surface", "-set", "sdl autolock=false",
                       "-set", "cpu core=normal", "-set", f"cpu cycles={args.cycles}",
                       "-set", "dosbox memsize=16", "-set", "render frameskip=0",
                       "-set", "joystick joysticktype=none",
                       "-c", f'mount c "{drive}"', "-c", f'mount d "{disc}" -t cdrom',
                       "-c", "d:", "-c", f"BLOOD2PG.EXE {args.game_args}"]
            report.update(command=command, display=env["DISPLAY"])
            game = subprocess.Popen(command, cwd=output, env=env, stdout=log, stderr=subprocess.STDOUT)
            began = time.monotonic()
            symbols = None
            last_snapshot = None
            clicked = False
            while time.monotonic() - began < args.seconds:
                time.sleep(args.interval)
                if game.poll() is not None:
                    raise RuntimeError(f"DOSBox exited during capture: {game.returncode}")
                with stopped_child(game):
                    if symbols is None:
                        binary, symbols = locate_symbols(game.pid)
                        report["emulator_elf"] = str(binary)
                        report["emulator_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
                    with open(f"/proc/{game.pid}/mem", "rb", buffering=0) as mem:
                        base = struct.unpack("<Q", read_exact(mem, symbols["MemBase"][0], 8))[0]
                        guest = read_exact(mem, base, GUEST_BYTES) if base else None
                        cpu = read_cpu(mem, symbols)
                snapshot = inspect_guest(guest, executable) if guest is not None else {"status": "emulator_memory_not_initialized"}
                snapshot.update(elapsed_seconds=round(time.monotonic() - began, 3), cpu=cpu)
                report["samples"].append(snapshot)
                signature = {key: value for key, value in snapshot.items() if key not in ("cpu", "elapsed_seconds")}
                if signature != last_snapshot:
                    name = f"state-{len(report['samples']):04d}.bin"
                    if guest is not None:
                        (output / name).write_bytes(guest)
                        snapshot["guest_dump"] = name
                    print(json.dumps({key: snapshot.get(key) for key in ("elapsed_seconds", "status", "profile", "time_storage")}), flush=True)
                    last_snapshot = signature
                if args.click_after is not None and not clicked and time.monotonic() - began >= args.click_after:
                    subprocess.run(["import", "-window", "root", str(output / "before-click.png")], env=env, check=True, timeout=10)
                    event = {"requested_at_seconds": round(time.monotonic() - began, 3), "status": "requested"}
                    report["input_events"].append(event)
                    event.update(private_click(env, game.pid), status="sent")
                    clicked = True
            subprocess.run(["import", "-window", "root", str(output / "screen.png")], env=env, check=True, timeout=10)
            report["outcome"] = "observed_profile" if any(s["status"] == "profile_bound" for s in report["samples"]) else "no_bound_profile_observed"
    except BaseException as error:
        report.update(outcome="capture_error", error=f"{type(error).__name__}: {error}")
        raise
    finally:
        try:
            stop(game)
        finally:
            stop(xvfb)
            (output / "capture.json").write_text(json.dumps(report, indent=2) + "\n")
    return report["outcome"] == "observed_profile"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("disc", type=Path)
    parser.add_argument("output", type=Path, help="new directory; never overwrite a prior capture")
    parser.add_argument("--dosbox", default="dosbox-x")
    parser.add_argument("--seconds", type=positive_seconds, default=60)
    parser.add_argument("--interval", type=positive_seconds, default=0.5)
    parser.add_argument("--cycles", type=int, default=30000)
    parser.add_argument("--click-after", type=positive_seconds,
                        help="send one primary click on the private display, without pointer movement")
    parser.add_argument("--game-args", default="AMR S162227 EMS WRIC:\\cblood\\")
    args = parser.parse_args()
    if args.cycles <= 0:
        parser.error("cycles must be positive")
    if args.click_after is not None and args.click_after >= args.seconds:
        parser.error("click-after must occur before the capture ends")
    if any(char in args.game_args for char in "\n\r"):
        parser.error("game arguments must be one command line")
    raise SystemExit(0 if capture(args) else 1)


if __name__ == "__main__":
    main()
