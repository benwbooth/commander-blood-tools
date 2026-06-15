# Dead Ends & Investigation Notes

Read before starting a session. Append when stuck after 10+ tool calls with no
progress on a single task.

Lifecycle: **Active** → **RESOLVED (date)** → delete after 20+ sessions.

## Entry format

## <Subsystem or Function Name>
- **Tried**: what approach was attempted
- **Failed because**: root cause of failure
- **Better approach**: what to do instead
- **Session**: NNN

---

## RESOLVED (2026-06-14) — "linear COD walking desyncs, needs a control-flow VM"
- **Tried**: full-opcode linear walk; it hit an INVALID byte after ~9 tokens on
  SCRIPT2; concluded the VM follows control flow so linear walking can't work.
- **Failed because**: the real cause was the length-0 opcodes `0xA8/0xAC/0xCC/0xD3`,
  which are NOT 1-byte — helper `0x6293` advances them by scanning byte-by-byte
  for a `0x0000` word terminator (then +2, +1 if a third zero). Treating them as
  1-byte desynced the walk.
- **Resolution**: `scan_zero_word` in `vm.rs`. With it, the walker walks ALL 5
  scripts cleanly to the `0xFF` end marker (0 invalid): SCRIPT1=214 tok/111 text,
  SCRIPT2=3271/1157, SCRIPT3=3281/1048, SCRIPT4=1714/719, SCRIPT5=1869/652. The
  COD **is** linearly walkable — no control-flow interpreter needed for a full
  execution-order pass. This unblocks execution-order scene-state tracking.
- **Session**: 001

## control-flow interpreter (IF/gosub) — first model WRONG (Active)
- **Control-flow opcodes found**: `0xA0 <u16>` pushes a target onto a gosub stack
  (gs:0x6820, SP gs:0x6884) and sets mode 1; `0xA1` pops + mode 0; conditional
  handlers call `0x6462` which pops the stack and sets `si = popped target`
  (restore PC) on fail. So execution is NON-linear (IF/gosub), even though the
  tokens are laid out linearly.
- **Tried**: model `0xA0 <target>` as "push else-target; mode1", conditions in
  mode1 jump to target on fail, `0xA1` → mode0 then-body. Python prototype.
- **Failed because**: it REGRESSED — only ~834/3687 lines reached (jumps land on
  wrong offsets → early `0xFF`/invalid), valid-location coverage 47% < the linear
  interpreter's 63%. So the 0xA0 target encoding and/or the jump/pass-fail
  semantics are not what I assumed (target may be relative, or conditionals use
  the skip-count `gs:0x67AB` path not the gosub pop, or pass/fail is inverted).
- **DO NOT ship the control-flow version** until it BEATS the linear 63% on
  valid-location coverage. The shipped Rust interpreter is the LINEAR one (61% in
  export) — keep it; it is the current best validated result.
- **Investigated**: the 32 `0xA0` targets in SCRIPT2 are ALL valid forward
  in-range offsets landing on opcode boundaries — but they **chain block→block**
  (`0xA0`→`0xA0`→…→`0xA6`), i.e. `0xA0` is a per-block navigation marker (the
  target is the *next* block), not a simple IF-else skip. Naïve jump-following
  therefore corrupts the walk (the regression). The gosub/`0xA0` machinery is the
  engine's runtime block navigation; it does NOT change which *assignments* set a
  character's location.
- **Conclusion**: for per-line LOCATION, the LINEAR interpreter is the correct
  model (assignments execute in COD order; that's what the shipped Rust version
  does, 61% in export, ~63% valid-location). Following the block-navigation flow
  does not raise location accuracy and risks regressions. The remaining gap is
  dominated by the ~22% no-speaker lines (many legitimately narrator/locationless)
  + script-only locations not in DESCRIPT — NOT missing control flow.
- **Session**: 001

## location-assignment opcode (Active)
- **Tried**: assumed a single "write state[obj+24]=location" opcode; checked the
  0xAF family (0x6946) and the length-7 family (0x6863).
- **Found instead**: 0x6946 = conditional `IF state[op1]==op2 {skip}` (12 sites
  test `state[char+24]==Location`); 0x6863 = binary comparison/expression
  operators (0xF0=ne, 0xF3=le, …) reading `state[bx+di]`. Neither writes state.
- **Why it's hard**: location is runtime state gated by conditionals, so the
  *actual* per-line location requires EXECUTING the script (state area + assigns
  + conditionals/branches), and some branches depend on playthrough (player
  choices), giving static resolution an inherent ceiling. The walk is solved;
  the state is not, without a bounded interpreter.
- **RESOLVED — assignment opcodes found**: the state WRITES are at 0x68FD
  (`mov es:[bx+di],cx`, in the 0x6863 family: b1/b4-b6/be-c0) and 0x69C2
  (`mov es:[bx+di],ax`, in the 0x6946 family: ad/af/b2/b3/ba/bb/bc). These
  opcodes are compound assignments `state[op1] = f(state[op1], op2)` with an
  operator byte (0xF0=ne, 0xF3=le, … seen at 0x6893/0x689F — a "set" operator
  among them does `state[op1]=op2`). When op1 = char+24 this sets the character's
  location. So ALL interpreter opcodes are now identified: walk (done), assign
  (0x6863/0x6946 families, write state[op1]), conditional/branch (0xAF family).
- **Better approach**: build the bounded interpreter — model the gs:0x6724 state
  area (init from VAR), execute assign + conditional/branch opcodes per script,
  snapshot actor+location at each 0xA6. Remaining sub-task: decode the operator
  byte set (which operator = plain "set"). Cutscenes are mostly deterministic so
  resolve well even where interactive branches don't.
- **Session**: 001

## voice-clip selection from 0xA6 b3 (Active)
- **Tried**: trace `gs:0x1FAB`(b3) → `gs:0x6788`(=b3+9) → expected a single reader
  that indexes a son.snd clip.
- **Failed because**: `gs:0x6788` has 41 accessors across 3 segments (0x008B
  display, 0x0971 per-frame updater, 0x0B1B audio/clear). It's the active-line id
  consumed by a whole subsystem, not a direct clip index. The actual son.snd
  playback is 1-2 hops past the dialogue updater (via 0xA1B4 / 0xA40B in seg
  0x0971).
- **Better approach**: map the audio subsystem top-down instead — find where
  son.snd is opened/its file handle, and where son.snd clip offsets are seeked
  (the SND bank format is already parsed in `src/extract/audio.rs`; cross-ref the
  clip-index math there). Then connect the active-line id to a clip. Treat as a
  subsystem-mapping task, not a single xref chase.
- **Progress (sess 001)**: audio subsystem LOCATED = segment 0x0B1B. son.snd/
  mus.snd are temp files (handles DS:0x0C47/0x0C49) extracted from BLOOD.DAT and
  deleted on scene change (cleanup @0x12E8). Voice reads (int21 AH=3F) at file
  0xBA00–0xC0FF; bank loaded to memory, clips via in-bank offset table. SND clip
  player @0xB9DE takes AX=clip index; gs:0x0A5A = current clip slot.
- **Static trace stalled (sess 001)**: `gs:0x6788` (active line id) has MULTIPLE
  writers across subsystems — the VM 0xA6 path (`b3+9`) AND an audio-loop counter
  `gs:0x24F5` that cycles 0..5 (@0xB00F). So the line→clip mapping is not a single
  static formula; it's a runtime interaction of the VM, the display updater, and
  the audio loop. Static RE keeps fanning out without converging.
- **Better approach**: switch to **dynamic analysis** (the skill's Ph5.5
  scriptable emulator, or instrument dosbox-x) — run a known dialogue line and
  observe which son.snd clip actually plays; that gives the formula directly.
  OR gate on the user's inspection: if exported voices are audibly wrong, that
  confirms the bug and justifies the emulator effort; if they sound right, the
  current heuristic is adequate and this drops in priority. Do NOT keep tracing
  statically.
- **Session**: 001

## RESOLVED (2026-06-14) — "is BLOODPRG.EXE a 32-bit / flat / DOS-extender binary?"
- **Tried**: assuming a 1994 game with large memory must use a flat 32-bit extender.
- **Failed because**: no PE/LE/LX overlay (MZ image == whole file), no
  DOS/4GW/PMODE/Phar Lap/CauseWay/DJGPP/DPMI signatures. It is a 16-bit
  real-mode MZ that uses 386 instructions + EMS/XMS banking.
- **Resolution**: load as DOS MZ, CS_MODE_16, 80386 ISA. Confirmed by mzfile.py.
- **Session**: 001
