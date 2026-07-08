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

## RESOLVED (2026-06-14) — voice-clip selection from 0xA6 b3
- **Resolution (sess 002)**: the production heuristic `(b3==0xFF) → clip = b4` was
  WRONG — it read the b4 *control-flag word* as a clip index, spuriously voicing
  **513 / 1942 voiced lines (26%)** (every b3==0xFF narrator/menu/tutorial line;
  every b4==0x08 → fake clip 8, b4==0x00 → fake clip 0). Correct formula
  (confirmed): `b3 == 0xFF` or `0x00` → NO voice; `b3 ∈ 1..=N` → `clip = b3 - 1`
  (1-based index into the actor's son.snd talk clips). Evidence: (1) the +9 reader
  @0x11F2 (`mov ax,[0x1fab]; add ax,9; mov [0x6788],ax`); (2) the SND player
  @0xb8cd takes `AX`=clip, `shl ax,2` (clip*4 table @DS:0x0BBF), sign-bit selects
  streamed vs in-mem; (3) the shipped export-data distribution: every b3∈1..N row
  is monotonic genuine character dialogue, every b3==0xFF row is non-character
  text. **Residual wall (documented, not blocking)**: the final AX is supplied via
  `lcall [0xcdf]` (@0xbba8), a registered SND-driver callback fn-ptr the
  disassembler can't statically resolve — so the *exact* arithmetic isn't
  byte-proven, but the formula is confirmed by the +9 reader + player + data and
  the b4-as-flag bug is unambiguous. **Fixed in `src/extract/script.rs`** (removed
  the `(0xFF,b4)` branch). The audio loop counter `gs:0x24F5` writing `gs:0x6788`
  @0xB00F is the talk-animation FSM, not clip selection (separate concern).
- **Session**: 001/002

## SUPERSEDED — voice-clip selection from 0xA6 b3 (original stalled trace)
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

## Cyberspace loader — cyber.ext file-load path (Active)

- **Goal**: find the BLOODPRG routine that opens `cyber.ext`/`cyber2.ext`/`cyber3.ext`
  (the cyberspace network-graph levels) to decode the minigame's graph format from its
  consumer.
- **Confirmed facts**: filename strings at file 0xd034 (`SEG 0x0ca3:0x04`) and 0xd849
  (`DS:0x429`) — two copies. `CYBER.EXT` (13189 B) is DATA, not code: begins
  `02 00 00 01 00 00 00 81 3f 02 ...` — a graph/table structure, word[0]=2.
- **Failed approaches (do NOT re-try these quick searches, sess 007)**:
  1. Immediate-operand search for `0x429` (push/mov ax/si/di/dx 0x429) — ZERO hits.
  2. Raw-byte search for `29 04` anywhere in the file — ZERO hits. So the `DS:0x429`
     copy is never referenced by that offset; the loader uses the other copy or builds
     the name at runtime.
  3. Filename pointer-table search (`0x429`..`0x459` consecutive) — none.
  4. `xref.py` far call/jmp to both SEG:OFFs — none (it only finds code targets; the
     string is data, so this can't find data refs by construction).
- **Why deep**: the reference is indirect — a far-pointer/segment-register load
  (DS or a const seg set, then a small/computed offset), or the name is assembled. The
  0xd034 copy at `SEG 0x0ca3:0x0004` (offset 4) can't be found by immediate search
  because offset `0x0004` is too common to disambiguate.
- **MECHANISM FOUND (sess 007, continue from here)**: filenames are gs-relative + path-
  assembled, which is why every offset search fails. The fopen sites (int 21h AH=3D, e.g.
  0xf70 `mov ax,0x3d00; int 21h`) are preceded by `mov dx,<off>` + `lcall 0x1ce:0x3b3`
  (file 0x2693) — a path BUILDER. That builder does `mov es,gs; mov si,dx; mov di,0x259;
  call 0x25a4` — i.e. **DX = the filename's offset in the `gs` string segment**, copied
  into a path buffer at DS:0x259, then `.ext`/dir prepended (calls 0x27e9/0x26cf/0x27c3).
  So the loader for cyber.ext calls the fopen wrapper with `DX = cyber.ext's gs-offset`.
- **Remaining unknown**: `gs`'s base segment + cyber.ext's offset within it. The two
  known copies (DS:0x429, SEG 0x0ca3:0x04) may not be the gs-segment copy; there may be a
  third copy in the gs resource/string segment. `mov dx,0x429` has zero hits, so gs≠DS
  here (or the gs copy is at a different offset).
- **Next approach**: (a) determine gs's base (set at startup — trace `mov gs,...`), then
  find cyber.ext in that segment to get its offset, then search `mov dx,<offset>` +
  nearby fopen; OR (b) find the cyberspace *entry* routine (runs on entering the modem/
  network) and trace forward to its level load. Then decode CYBER.EXT's graph format
  (word[0]=2, then byte records) from that consumer.
- **Session**: 007

## "Combat" is NOT a subsystem in this game (RESOLVED sess 007)

- Repeatedly listed "combat" as a remaining undecoded subsystem this session - that was an
  UNVERIFIED assumption, now checked and FALSE.
- String search for combat/weapon/attack/damage/health/fight/laser/shoot/enemy/hit: ZERO
  hits in BLOODPRG.EXE.
- Commander Blood is a dialogue/exploration/puzzle adventure (CRYO). The actual gameplay
  systems are: dialogue (VM A6 text opcode + DESCRIPT), navigation (ship-3D + star map),
  alien examination (scrut/croolis), comms/Hate-TV, cyberspace (hyper_* + CYBER*.EXT graph),
  and the manu3 menu - all decoded/ported this session. There is no combat/action layer.
- Correction: stop listing "combat" as remaining work. The genuinely-remaining decode is
  the deep .ext world-body record semantics + the ~70% of exe functions not yet touched
  (utility/init/hardware/overlay-specific), NOT a combat subsystem.

## ret-preceded prologue scan: confirmed false positives (RESOLVED)
The ret-preceded clean-prologue scan (0x600-0xd000) leaves 5 addresses that are NOT
function entries, confirmed by disassembly context:
- 0x00dd8, 0x0220d, 0x02216, 0x02f73: `pop.../retf` sequences - these are function
  EPILOGUES (mid-function tails) that happen to follow a ret byte; not entries.
- 0x02bee: `inc dx; dec bp; and [bx+si],al` - data (a table/constant) misparsed as code.
These 5 are the residue of the byte-scan heuristic and require no labels. With them
excluded, the ret-preceded verified-start scan window is fully accounted for.

## Far-call target scan: 0x8d5d false positive (RESOLVED)
The flat 9A-opcode far-call scan (file = 0x600 + seg*16 + off) yields 108 targets in the code
segment; 107 are real functions (now all labeled). The 108th, 0x8d5d, disassembles to
`add [bx+di],al` (bytes 00 00) - not a valid prologue. It is a false positive: the 0x9A byte
that produced it is embedded in DATA (a table/constant), not a real call site, and its
following 4 bytes coincidentally resolve into the code range. Excluding it, the far-call-
dispatched function set in the code segment is 107/107 labeled - fully enumerated.

## The static-analysis boundary: register-indirect dispatch (CHARACTERIZED)
With the ret-preceded-prologue scan exhausted, the far-call (9A) target set 107/107 labeled,
and the VM opcode handler table (51 handlers) enumerated, the functions NOT reachable by
static enumeration are exactly those reached through register-indirect dispatch:
- **gs:[0xa4a]** - a stored callback, `call gs:[0xa4a]` appears 8x. There are NO immediate
  stores to it (`65 C7 06 4A 0A ..` finds zero hits) and none to its seg half 0xa4c, so the
  pointer is loaded from a register/memory at runtime, not a constant. Its target(s) cannot
  be resolved by byte-pattern or single-pass sweep.
- **gs:[0xb1d]**, **gs:[0xa96]** - two more indirect call slots, same situation.
- **The input jump-table** (documented earlier) - handler offsets are computed, not immediate.
- **The VM computed dispatch** `call gs:[(op-0xA0)*2 + 0x6eb0]` - table IS resolved (handler
  file = entry + 0x53a0), the 51 entries are known; listed here only for completeness.

Why static analysis stops here: resolving these needs either full inter-block dataflow
(tracking the pointer from its load site) or DYNAMIC tracing. The correct next tool is the
real-game headless oracle (re/tools/capture_real_game.sh, DOSBox-X+Xvfb): instrument the call
sites and log the actual targets at runtime. That same dynamic harness is also the only way
to establish the outstanding BEHAVIORAL-EQUIVALENCE bar (cycle/output parity vs the DOS
binary), which static labeling cannot prove. This is the boundary between the static RE (now
substantially complete for directly- and far-called functions) and the dynamic/behavioral
verification phase (not yet built out).
