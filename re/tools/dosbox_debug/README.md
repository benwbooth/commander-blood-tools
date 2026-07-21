# DOSBox-X heavy-debugger ground-truth harness

The environment's stock dosbox-x has no debugger, so we BUILD one with the heavy debugger
(`--enable-debug=heavy`, which provides MEMDUMPBIN + breakpoints + single-step):

    nix-build -E 'with import <nixpkgs> {}; dosbox-x.overrideAttrs (o: {
      configureFlags = (o.configureFlags or []) ++ ["--enable-debug=heavy"];
      buildInputs = (o.buildInputs or []) ++ [ ncurses ]; })' -o /path/dosbox-x-dbg

Run headless under Xvfb; the curses debugger appears on the controlling pty. Alt+Pause (via
xdotool to the SDL window) breaks in; `MEMDUMPBIN 0000:0000 100000` dumps 1MB to MEMDUMP.BIN in
DOSBox's CWD; F5 resumes. `db_break.py` automates: boot BLOODPRG.EXE with the oracle mounts/args
(c=accuracy/cdrive, d=output/_tmp_iso, `AMR S162227 EMS WRIC:\cblood\`), break, screenshot, dump.

Non-interactive tracing that DID work and confirmed facts:
- `-debug -log-fileio` logs every file open/read/seek. DOSBox opens the SAME files my runtime does
  (blood.dat, tb.big, descript.des, script1.*, btv.spr, CARTE.SPR, chart.fd; blood.sav open FAILS in
  both). So the credit divergence is NOT a file-I/O difference.
- Screenshot timeline (cycles=max): Microfolie's -> spaceship -> CRYO logo -> elephant+"CRYO
  Interactive Entertainment 1995" (CLEAN) at ~t56 -> "Commander BLOOD V 1.0". Matches my runtime's
  scene ORDER/pacing (pacing is consistent; the earlier "4.5x" was a wall-clock artifact).

DOSBox's game gs = 0x1505 region (loads lower than my runtime's 0x0e84). MEMDUMP goes to DOSBox CWD
(repo root). NEXT: the break keeps landing on transient "LOADING" screens; refine resume(F5)+timing
(or set BPINT on the descript.des read) to catch the credit frame, then read gs:0x6780/0x5e64/0xe18.
