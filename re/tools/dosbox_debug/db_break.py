#!/usr/bin/env python3
"""Break the heavy-debugger DOSBox-X at the credit frame and MEMDUMPBIN the gs data segment.

Runs BLOODPRG.EXE at real-time cycles (credit shows ~6s per the DOSBox capture), waits, sends the
debugger break hotkey (Alt+Pause) via xdotool to the SDL window, reads the curses debugger on the pty,
issues SR (to learn gs) then MEMDUMPBIN gs:0 0x10000, and copies the dump out for offline comparison.

Usage: python3 db_break.py <wrapper-dosbox-x> <root> <outdir> <display> <wait_seconds>
"""
import os
import pty
import re
import select
import shutil
import subprocess
import sys
import time

DBX, ROOT, OUT, DISP, WAIT = sys.argv[1], os.path.realpath(sys.argv[2]), sys.argv[3], sys.argv[4], float(sys.argv[5])
os.makedirs(OUT, exist_ok=True)
CAP = os.path.join(OUT, "cap")
os.makedirs(CAP, exist_ok=True)
env = dict(os.environ, DISPLAY=DISP, SDL_VIDEODRIVER="x11", TERM="xterm")

xvfb = subprocess.Popen(["Xvfb", DISP, "-screen", "0", "800x600x24"],
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(3)

args = [DBX, "-set", "sdl output=surface", "-set", "cpu cycles=max", "-set", f"dosbox captures={CAP}",
        "-c", f'mount c "{ROOT}/accuracy/cdrive"', "-c", f'mount d "{ROOT}/output/_tmp_iso"',
        "-c", "d:", "-c", r"md c:\cblood", "-c", r"BLOODPRG.EXE AMR S162227 EMS WRIC:\cblood\\"]
master, slave = pty.openpty()
proc = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=env, close_fds=True)
os.close(slave)


def drain(t=1.0):
    buf = b""
    end = time.time() + t
    while time.time() < end:
        r, _, _ = select.select([master], [], [], 0.2)
        if r:
            try:
                buf += os.read(master, 65536)
            except OSError:
                break
    return buf.decode("latin1", "replace")


def send(cmd, t=1.2):
    os.write(master, (cmd + "\r").encode())
    time.sleep(0.4)
    return drain(t)


print(f"initial run {WAIT}s...")
time.sleep(WAIT)
win = subprocess.run(["xdotool", "search", "--name", "DOSBox"], env=env,
                     capture_output=True, text=True).stdout.split()
print("dosbox windows:", win)
w = win[0] if win else None


def brightness(png):
    try:
        r = subprocess.run(["convert", png, "-colorspace", "Gray", "-format", "%[mean]", "info:"],
                           capture_output=True, text=True)
        return float(r.stdout.strip() or 0)
    except Exception:
        return 0


def break_in():
    subprocess.run(["xdotool", "key", "--window", w, "alt+Pause"], env=env)
    time.sleep(0.6)
    drain(1.0)


def resume():
    # DOSBox-X heavy debugger: F5 continues emulation (sent to the SDL window).
    subprocess.run(["xdotool", "key", "--window", w, "F5"], env=env)
    time.sleep(0.3)
    drain(0.3)


shot = os.path.join(OUT, "brk_shot.png")
got = False
for attempt in range(16):
    break_in()
    subprocess.run(["import", "-window", "root", "-gravity", "South", "-crop", "640x400+0+0",
                    "+repage", "-resize", "320x200!", shot], env=env, capture_output=True)
    b = brightness(shot)
    r1 = send("MEMDUMPBIN 0000:0000 100000", 4.0)
    src = os.path.join(ROOT, "MEMDUMP.BIN")
    hit = ""
    if os.path.exists(src):
        data = open(src, "rb").read()
        for s in (b"CRYO Inter", b"Commander BLOOD", b"WAIT COMMANDER"):
            if s in data:
                hit = s.decode()
                break
        if hit:
            shutil.copy(src, os.path.join(OUT, "mem_dump.bin"))
            subprocess.run(["cp", shot, os.path.join(OUT, f"scene_{attempt}.png")])
            os.remove(src)
            print(f"attempt {attempt}: brightness={b:.0f}  *** CAUGHT subtitle {hit!r} ***")
            got = True
            break
        os.remove(src)
    print(f"attempt {attempt}: brightness={b:.0f}  no subtitle string")
    resume()
    time.sleep(3.5)  # advance to next scene
dst = os.path.join(OUT, "mem_dump.bin")
if got and os.path.exists(dst):
    data = open(dst, "rb").read()
    print(f"=== CAUGHT credit-frame dump: {len(data)} bytes (base linear 0x0) ===")
    import re as _re
    for needle in (b"WAIT COMMANDER", b"CRYO Inter", b"Commander BLOOD", b"1995"):
        for m in _re.finditer(_re.escape(needle), data):
            print(f'  {needle!r:20} @ linear {m.start():#07x}  ctx={data[m.start()-4:m.start()+44]!r}')
            break
        else:
            print(f"  NOT FOUND: {needle!r}")
else:
    print("did not capture the credit frame in memory")
proc.terminate()
time.sleep(1)
xvfb.terminate()
