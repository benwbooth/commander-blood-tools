#!/usr/bin/env python3
"""Dump BLOODPRG's live DOS memory from a running DOSBox-X, via ptrace.

This DOSBox-X build has no savestate/debugger, but DOSBox-X is a Linux process and the
DOS RAM lives in its address space. Under yama ptrace_scope=1 a process can ptrace its
own CHILD, so this script LAUNCHES dosbox-x itself, then PTRACE_ATTACHes and reads
/proc/<pid>/mem — no root needed.

It locates BLOODPRG's DS by finding the static vertex table (DS:0x5D98, file 0x131B8)
in memory; every DS-relative global is then a fixed offset from that anchor, independent
of the load segment. Reads the requested DS offsets (default: the star-map nav state —
the 11 destination records at DS:0x4F09 and the camera origin at DS:0x2F65).

Usage: nix develop --command re/tools/dump_dosbox_mem.py <game-dir> [wait_secs]
NOTE: the star-map's 0x4F09 records are the *default* (10200,12100,900) until the game
is in ACTIVE navigation — drive it there (see drive_real_game.sh) before dumping.
"""
import ctypes, subprocess, time, os, re, struct, sys

ANCHOR = bytes.fromhex("00000009030c08030b07040b07030a06")  # DS:0x5D98 vertex table
DS_ANCHOR = 0x5D98
# DS globals of interest -> (name, offset, count_words)
GLOBALS = [
    ("origin_2F65", 0x2F65, 3),   # camera origin x,y,z
    ("angle_2F71", 0x2F71, 1),    # camera angle
    ("angle_2F6D", 0x2F6D, 1),    # compass angle
    ("nav_recs_4F09", 0x4F09, 33),  # 11 records x 3 words
]


def main():
    game = os.path.realpath(sys.argv[1])
    wait = int(sys.argv[2]) if len(sys.argv) > 2 else 40
    libc = ctypes.CDLL("libc.so.6", use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long, ctypes.c_void_p, ctypes.c_void_p]
    PTRACE_ATTACH, PTRACE_DETACH = 16, 17
    env = dict(os.environ); env["DISPLAY"] = env.get("DISPLAY", ":53"); env["SDL_VIDEODRIVER"] = "x11"
    xvfb = subprocess.Popen(["Xvfb", env["DISPLAY"], "-screen", "0", "800x600x24"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(3)
    db = subprocess.Popen(["dosbox-x", "-set", "sdl", "output=surface",
                           "-c", f"mount c {game}", "-c", "c:", "-c", "BLOODPRG.EXE"],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env)
    time.sleep(wait)
    pid = db.pid
    try:
        if libc.ptrace(PTRACE_ATTACH, pid, None, None) != 0:
            print("ptrace attach failed errno", ctypes.get_errno()); return
        os.waitpid(pid, 0)
        mem = open(f"/proc/{pid}/mem", "rb")
        best = None
        for line in open(f"/proc/{pid}/maps"):
            pr = line.split()
            if 'r' not in pr[1] or '-' not in pr[0]:
                continue
            a, b = [int(x, 16) for x in pr[0].split('-')]
            if b - a > 300_000_000:
                continue
            try:
                mem.seek(a); buf = mem.read(b - a)
            except Exception:
                continue
            for m in re.finditer(re.escape(ANCHOR), buf):
                A = a + m.start()
                # the correct DS-aligned copy has origin_z == 0
                mem.seek(A - (DS_ANCHOR - 0x2F69)); z = struct.unpack('<h', mem.read(2))[0]
                if z == 0:
                    best = A; break
            if best:
                break
        if not best:
            print("DS anchor not found (DS-aligned)"); return
        for name, off, n in GLOBALS:
            mem.seek(best - (DS_ANCHOR - off))
            vals = struct.unpack(f'<{n}h', mem.read(n * 2))
            print(f"{name}: {vals if n > 1 else vals[0]}")
    finally:
        libc.ptrace(PTRACE_DETACH, pid, None, None)
        db.kill(); xvfb.kill()


if __name__ == "__main__":
    main()
