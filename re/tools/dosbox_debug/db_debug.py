#!/usr/bin/env python3
"""Drive the heavy-debugger DOSBox-X headlessly via a pty to dump game memory at the credit frame.

Strategy: run BLOODPRG.EXE, let it reach the credit scene, break into the heavy debugger, read the
segment registers (to learn DOSBox's gs), then MEMDUMPBIN the gs data region. Compare gs:0x6780 /
0xe18 / 0x5e64 / 0x5e65 / 0xba3 / 0xade to the path-B runtime's values to find the divergent state.

Usage: python3 db_debug.py <dosbox-x-debug-binary> <root> <outdir> [display]
"""
import os
import pty
import select
import subprocess
import sys
import time

DBX = sys.argv[1]
ROOT = os.path.realpath(sys.argv[2])
OUT = sys.argv[3]
DISP = sys.argv[4] if len(sys.argv) > 4 else ":91"
os.makedirs(OUT, exist_ok=True)

env = dict(os.environ, DISPLAY=DISP, SDL_VIDEODRIVER="x11", TERM="xterm")
xvfb = subprocess.Popen(["Xvfb", DISP, "-screen", "0", "800x600x24"],
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(3)

# DOSBox-X heavy debugger: it reads debugger commands from the controlling TTY. Run it under a pty.
args = [
    DBX, "-set", "sdl output=surface",
    "-c", f'mount c "{ROOT}/accuracy/cdrive"',
    "-c", f'mount d "{ROOT}/output/_tmp_iso"',
    "-c", "d:", "-c", r"md c:\cblood",
    "-c", r"BLOODPRG.EXE AMR S162227 EMS WRIC:\cblood\\",
]
master, slave = pty.openpty()
proc = subprocess.Popen(args, stdin=slave, stdout=slave, stderr=slave, env=env, close_fds=True)
os.close(slave)


def drain(timeout=1.0):
    buf = b""
    end = time.time() + timeout
    while time.time() < end:
        r, _, _ = select.select([master], [], [], 0.2)
        if r:
            try:
                buf += os.read(master, 65536)
            except OSError:
                break
    return buf.decode("latin1", "replace")


def send(cmd):
    os.write(master, (cmd + "\n").encode())
    time.sleep(0.3)


# Let the game boot + reach the credit scene (~10s wall-clock in DOSBox).
print("booting; waiting for credit scene...")
time.sleep(12)
# Enter the debugger (DOSBox-X heavy debug hotkey is Alt+Pause; also try sending a newline to the TTY
# which some builds treat as a break). We first just probe what the debugger console shows.
out = drain(2)
print("=== initial debugger/tty output (last 2000 chars) ===")
print(out[-2000:])
# Try issuing a couple of harmless debugger commands to see if the console is live.
for c in ["", "HELP", "SR"]:
    send(c)
    print(f"--- after {c!r} ---")
    print(drain(1.5)[-1500:])

proc.terminate()
xvfb.terminate()
