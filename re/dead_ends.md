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
(tracking the pointer from its load site) or DYNAMIC tracing. That same dynamic harness is
also the only way to establish the outstanding BEHAVIORAL-EQUIVALENCE bar (cycle/output parity
vs the DOS binary), which static labeling cannot prove.

## UPDATE (RESOLVED via dataflow): the indirect slots are EXTERNAL entry points
The register-indirect call slots above were traced to their store sites and are NOT
un-enumerated internal functions - they are external system/driver entry points captured at
boot, which a decompile invokes as services:
- **gs:[0xa4a]/[0xa4c]** = the **XMS driver** far entry point. Boot does int 2Fh AX=4300
  (XMS install check, AL=80 if present) then AX=4310 (get driver entry -> ES:BX), stored at
  0xa1b/0xa20. The ~18 `call gs:[0xa4a]` sites call HIMEM.SYS (AH = XMS function code). External.
- **gs:[0xb1d]/[0xb1f]** = the **saved original INT 08h (PIT timer) vector**. install_timer_isr_hook
  (0x79c) does int 21h AX=3508 (get vector -> ES:BX, stored 0x7a6) then AX=2508 to install the
  game's own ISR at cs:0x213 (file 0x813). The game's timer ISR chains to the original via
  `call gs:[0xb1d]`. The internal ISR (0x813) IS labeled; the chained target is the BIOS handler.
  Verified: ISR 0x813 gates on gs:0xb21 then services gs:0xadf (the audio mixer sub-flag) -
  i.e. the PIT ISR drives the software audio mixer, consistent with the whole audio chain.
- **gs:[0xa96]** = a third indirect slot (call sites 0x169e, 0xb5fe); its store is via a
  far-pointer load (les/lds), same external-entry shape - likely another saved vector/driver.

Net: the "register-indirect targets" are overwhelmingly EXTERNAL (XMS driver, saved interrupt
vectors), not missing internal code. This shrinks the static gap to essentially the computed
input jump-table plus proving behavioral equivalence. The former remains a genuine static
limit; the latter is the dynamic/behavioral phase (not yet built out).

## Input dispatch: MECHANISM decoded, full enumeration blocked by runtime DS (REFINED)
The "input jump-table" is now precisely located (input_dispatch 0x210e):
  read event (lcall 0x1ce:0x39d) -> AL, else AH -> optional `or al,0x80` (alt path)
  -> `mov bx,0x113e; xlatb`  (translate the input code via a 256-byte table at DS:0x113e)
  -> if the translated value has bit7 set, ignore; else `add ax,ax; mov bx,ax`
  -> `call cs:[bx+0x123e]`   (handler table at file 0x183e; entry 0 = handler @ file 0x1f06)
The handler table is CS-relative and its first entries decode to clean code. What is NOT
statically resolvable is the COMPLETE live-handler set, because the xlatb translate step reads
DS:0x113e and DS at the dispatch is inherited from the caller (the function sets no DS):
  - Reading the table CS-relative (file 0x173e) yields a plausible-looking but INCONSISTENT
    index set (indices up to 125, most of whose handler slots decode to non-code) -> wrong base.
  - Reading it DGROUP-relative (DS base 0xD420 -> file 0xE55E) yields 256 zero bytes -> also wrong.
Neither candidate DS base produces a valid 256-entry translate table, so the effective DS is
some other runtime segment (set by whoever calls 0x210e). Pinning it needs dynamic tracing.
CONCLUSION: mechanism fully decoded; exact handler enumeration requires the runtime DS value
= a dynamic fact. This supersedes the older "input jump-table handler count unknowable" note
with the concrete mechanism and the precise reason (DS-relative xlat table, runtime DS).

## Input dispatch table: CS vs DS base BOTH inconclusive statically (FINAL, dynamic-only)
Further attempt to resolve the input handler table (10+ tool calls, stopping):
- The table at **CS:0x113e (file 0x173e) IS populated** (237/256 nonzero) and the handler
  table at CS:0x123e (file 0x183e) is populated (231/252 nonzero) - so if DS=CS at dispatch,
  this is the live table. But validation is weak: of the 51 xlatb-selected indices, only
  **3/51 (5%)** of the handler-table entries land on a KNOWN function start, and 46/51 merely
  fall somewhere in the 0x600-0xd000 code range - which for random 16-bit offsets happens ~79%
  of the time anyway. So "points into code" is NOT evidence the table is correctly resolved.
- The DGROUP copy (DS base 0xD420 -> file 0xE55E) is 0/256 (empty in the static image).
- No code writes to 0x113e (no runtime-fill found), so the table is not obviously built at boot.
CONCLUSION (final): the handler entries could be legitimate mid-function code labels (normal
for a dispatch table) OR the CS-base assumption is wrong; static analysis cannot decide, because
the effective DS for the xlatb source and the semantics of the entries both require observing a
live dispatch. This is a DYNAMIC-tracing task (single-step the real game at 0x2137, read BX and
the resolved target). Mechanism = fully decoded; table contents = deferred to the dynamic phase.
Do not re-attempt statically.

## Input dispatch table: RESOLVED via dynamic ptrace read (DS=CS confirmed)
Built a live-memory reader (scratchpad/read_input_table.py) reusing dump_dosbox_mem.py's
ptrace+DS-anchor logic, and read the candidate table locations from the RUNNING game:
- **DGROUP-DS:0x113e** live = the ASCII bytes "SCRIPT5.VAR" (a filename buffer), 13/256 nonzero
  - definitively NOT a 256-entry translate table.
- **DGROUP-DS:0x123e** live = all zero.
This empirically RULES OUT the DGROUP base and confirms the xlatb translate table is the
populated CS:0x113e (file 0x173e, 237/256 nonzero), with the handler table at CS:0x123e (file
0x183e) - i.e. **DS = CS at the input dispatch (0x210e)**. This resolves the base ambiguity the
earlier static analysis could not. The mechanism + table location are now fully pinned; the
only residual is per-handler exact semantics, which (if needed) comes from single-stepping the
live dispatch at 0x2137 to read BX and the resolved target. Supersedes the "statically
inconclusive" note above with a dynamic answer.

## Framebuffer-pixel parity: GS render arena not locatable from DGROUP (blocked, 2026-07)
Palette per-byte parity succeeded because the palette buffer GS:0x5b58 is in DGROUP (readable
via the vertex-table DS anchor; matched CHART.FD 120/120). Attempted the same for the
FRAMEBUFFER (to compare rendered pixels), and hit a wall:
- The linear back-buffer is a far ptr at gs:0x5229; the mode-X screen buffer at gs:0x521d
  (static value A000:0000 = VGA). Read live from DGROUP: 0x5229/0x521d/0x5221 all = 0.
- The GS work-arena globals 0xa98 (the 64K arena segment stored by mem_alloc_64k), 0x671c
  (objects), 0x672c (VM table) also all read 0 from DGROUP at 55s.
- Yet DGROUP:0x5b58 (palette) HAS valid data. => GS is a SEPARATELY-ALLOCATED segment (EMS/XMS
  or a malloc'd 64K arena), NOT the DGROUP that the anchor locates. Its base is not stored in a
  readable DGROUP global (0xa98=0), so the render pointers can't be resolved from the anchor.
- The VGA path (0x521d -> A000:0000) is mode-X PLANAR and lives in DOSBox-X's separately-paged
  vga.mem, not at conventional-RAM base+0xA0000, so a flat read there won't get live pixels.
Two viable future approaches (not yet done): (a) locate DOSBox-X's vga.mem region in the
process by searching for a known de-interleaved frame signature; (b) use a DOSBox-X debugger
build to read the live GS register at a render call, giving the arena base directly. Recorded
tools: re/tools/read_live_framebuffer.py (linear-bb attempt). Framebuffer-pixel parity remains
OPEN; palette per-byte parity (CHART.FD, 120/120 scene colors) stands as the positive result.

## Framebuffer read: UNBLOCKED (GS=DS); real frame captured; layout decode pending — 2026-07
The prior "GS render arena not in DGROUP" blocker was WRONG about the cause. Confirmed from the
code: there is exactly ONE `mov gs` in the whole binary, at boot 0x61e (`mov ax,ds; mov gs,ax`),
so **GS == DS == DGROUP for the entire program**. The earlier live reads got 0 at gs:0x5229
because that run happened to catch an HNM cutscene (which renders via its own path, back-buffer
unallocated), NOT the star-map.
- Re-ran during a CONFIRMED star-map (palette 120/120): gs:0x5229 = 3cef:0000 (allocated linear
  back-buffer), gs:0x521d = a000:4000 (VGA mode-X page 1). Read 64000 bytes from 3cef:0000.
- The data is a REAL frame, not garbage: histogram = 51% index-0 (black) with a clean fully-zero
  top band and stddev-of-counts 2064 (highly skewed => real image; uniform garbage would be ~15).
- BUT: rendered with the (verified) palette, neither a linear 320x200 nor a 4x16000 plane-bank
  nor a 4-interleaved layout produces a recognizable star-map - the bottom renders as per-pixel
  noise (adjacent pixels uncorrelated), so the exact buffer layout is still wrong.
NEXT (concrete): (a) try reading the VGA page itself at dos_base + 0xA0000 + 0x4000 (the actual
displayed mode-X page) and de-interleave; (b) determine the game's back-buffer stride/paging
from the blit routine (0x509d dirty-rect blit / 0x3e46 full blit) which encodes the exact
src->dst layout. The frame DATA is captured and proven real (re/tools/read_live_framebuffer.py);
only the layout mapping remains. This is progress from the prior null-pointer state.

## Display-frame parity via DOSBox vga.mem: BLOCKED on emulator-internal layout (2026-07)
Extensive attempt (multiple turns/runs) to read the DISPLAYED mode-X frame from DOSBox-X memory,
after establishing that most screens render direct to VGA (gs:0x521d=A000), not the linear
texture buffer gs:0x5229:
- dos_base+0xA0000 (conventional-RAM VGA window) = all zeros: DOSBox pages VGA separately.
- Enumerated all process mmaps; the VGA-sized anonymous rw regions (256KB, 132KB) are 0-1%
  nonzero = NOT the active framebuffer. The live frame is inside a LARGER emulator-internal
  allocation (DOSBox-X SVGA memsize, several MB) whose internal layout (plane storage order,
  page base within vga.mem) is emulator-private and not identifiable by region size alone.
- Locating it would require either matching a de-interleaved window against the paired
  screenshot across many MB (expensive, fragile) or reading DOSBox-X's C++ `vga.mem.linear`
  symbol via a debug build. Neither is a quick memory read.
CONCLUSION: confound-free DISPLAY-frame pixel parity is not reachable with plain /proc/mem +
the DS anchor (which only reaches DGROUP-relative data: palette 0x5b58, VM/object state, and
the starfield texture 0x5229). Reachable via memory: palette (DONE, 120/120), and any
DGROUP-relative game STATE. NOT reachable this way: the composited VGA display frame.
Alternatives for a future session: (a) a DOSBox-X debug build exposing vga.mem; (b) compare the
geometry-correct SCREENSHOT (capture_real_game_native.sh) accepting the scaler/palette confound
as a bounded qualitative check; (c) pivot verification to DGROUP-relative STATE parity (VM
records, object table, nav state) which IS memory-reachable. Stopping the vga.mem drill.

## Object-descriptor value parity: blocked by EMS/XMS banking (2026-07)
Tried to close object-state value parity by following the entity record's +4 descriptor far
pointer (live gameplay: rec0 = 0x7979:0x004d) and reading width/height at descriptor +0/+2 to
compare vs our decoded sprite dimensions. All descriptor reads at dos_base + seg*16 + off = 0.
Reason: the far-ptr segments (0x7979, 0x7545, ...) point into EMS/XMS-BANKED resource memory,
NOT conventional RAM. The game loads sprites/descriptors into EMS (int 67h) / XMS banks; the
linear dos_base+seg*16 formula only resolves CONVENTIONAL-RAM segments, so a banked segment
reads as unmapped/zero at that linear address (the same 16-bit segment maps different physical
pages depending on the current int-67h page-frame mapping).
CONSEQUENCE - the confound-free /proc/mem + DS-anchor method reaches exactly the CONVENTIONAL-RAM
DGROUP state: palette (0x5b58), nav/camera (0x2F65/0x4F09), render clip (0x5235/0x5239), the
entity RECORD fields (0x6212, flags/id/ptrs) - all VERIFIED. It does NOT reach the DATA behind
far pointers into EMS/XMS banks (descriptors, sprite pixels, resource bodies). Reaching those
would require reading the live int-67h EMS page-frame mapping (gs:0xa60/0xa66 area) and following
the bank->physical translation - a further layer, not done. This is the memory-side analogue of
the VGA-mem blocker: DGROUP state is reachable; banked resource data + the VGA display frame are
not, via plain linear /proc/mem. The 4 value-level confirmations stand on the reachable state.

## CORRECTION: object descriptors are NOT EMS-banked - EMS/XMS is inactive (2026-07)
The prior note blamed EMS/XMS banking for the zero descriptor reads. That was WRONG - corrected:
- Read the EMS/XMS state live: DS:0xa60 (EMS page frame) = 0, DS:0xa66 (EMS handle) = 0, DS:0xa4a
  (XMS entry off) = 0. EMS/XMS is NOT ACTIVE in this run (DOSBox-X here provides no EMS/XMS, so
  the game runs from conventional memory - no banking).
- The descriptor segments (0x7979, 0x7545) ARE inside the mapped conventional region, but the
  64-byte windows there read all-zero. So the pointers are set but the pointed-to descriptors
  were UNPOPULATED at the 72s capture instant (records mid-load, or the +4 values were stale/
  transitional), NOT banked-out.
CONSEQUENCE (reopens the path): with EMS/XMS inactive, ALL resource data (descriptors, sprite
pixels) lives in CONVENTIONAL RAM and IS reachable via dos_base+seg*16+off. The zero reads were
a timing/population issue, not an addressing wall. Also confirmed dos_base+0xA0000 IS inside the
mapped region (so the VGA-window zeros are DOSBox not shadowing VGA there, a separate matter).
NEXT: capture object descriptors at a STABLE fully-loaded gameplay moment (not a transition),
or follow a record whose +4 ptr lands on non-zero memory, to read real width/height and compare
to our decoded sprite descriptor. The descriptor-value parity is NOT blocked - it needs a clean
populated capture. Honest correction of the previous EMS claim.

## Display page 0x5221 RAM buffer: coherent composited frame, but off-screen (not the display) (2026-07)
Fresh angle on framebuffer parity: the game double-buffers between a RAM page (gs:0x5221) and
VGA (gs:0x521d=A000). Read the gs:0x5221 RAM page during the attract intro (seg 0x2cee, 47%
nonzero) and rendered it LINEARLY: 94% horizontal-neighbour equality = a highly COHERENT frame,
showing the CRYO logo (recognisable). So unlike the 0x5229 starfield TEXTURE, the 0x5221 page IS
a real composited game frame in conventional RAM, LINEAR layout, readable via the anchor.
BUT it does NOT match the live display: the paired screenshot shows CRYO centred on black, while
the 0x5221 buffer shows CRYO upper-left on green - i.e. the 0x5221 RAM page is the OFF-SCREEN
double-buffer page (mid-composite / previous frame), not the frame VGA is currently scanning out.
The DISPLAYED frame is in VGA mode-X (separately paged vga.mem), still unreachable.
NET: I can now read a coherent linear composited frame from the 0x5221 off-screen page - a real
advance over the texture (0x5229) and VGA (unreachable) dead-ends. Byte-exact DISPLAY parity
still needs either (a) catching the off-screen page at the exact frame our engine also renders
(frame-index alignment), or (b) the VGA vga.mem read. The read+decode of the off-screen composited
frame WORKS (linear, palette-correct); only the same-frame alignment vs our engine remains.

## SCRIPT1 tutorial completion via blind clicking (sess: whole-game RE)
TRIED: TUTORIAL mode in runtime_boot — fast-skip to the ship console (~45M steps),
then click the centre orb (the pointing-hand target) + all 5 menu rows in rotation for
48 rounds (~250M more steps), watching opened_files for script2.* and reading the
subtitle at gs:0xe18.
WHY-FAILED: never advanced past SCRIPT1 — opened_files stayed at 16 (script1.* only,
plus bappel/izwalito from TELEPHONE clicks); the console stayed on "Click quick, Cap'n
Bob is waiting…". The tutorial gates progression on a SPECIFIC interaction (a particular
button, order, or timed click) that blind rotation doesn't hit. ALSO: gs:0xe18 is NOT
the live tutorial subtitle buffer — it read stale "WAIT COMMANDER" (the attract/credit
text, cf. credit-divergence) the whole run while the SCREEN showed the tutorial lines.
BETTER-APPROACH: (a) find the real tutorial-subtitle buffer (search RAM for "Click
quick"/"You found" ASCII, then read that offset each round to know tutorial STATE);
(b) trace the SCRIPT1 VM to find which console object/button its dialogue branch waits
on, then click exactly that; (c) find a launch-arg / savestate that starts past the
tutorial. This unblocks OPTION + the interactive gameplay (progression, mini-games).
CONFIRMED THIS RUN (positive): reached the console; MENU opens the {EXPLANATIONS, GAME}
submenu (3rd confirmation); console = CHART.FD + grayscale portrait orb + orange orb
button + pointing hand + golden menu.

## Headless DOSBox-X mouse drive of the real game (sess: whole-game RE, 2026-07-21)
GOAL: reach interactive gameplay in the REAL game (which, unlike the recomp emulator,
proceeds past the credit -> shows "CRYO Interactive Entertainment 1995", not WAIT COMMANDER)
so OPTION/mini-games/progression can be observed + decoded.
TRIED: (1) drive_real_game.sh + args, Esc/Return/center-clicks -> cycles the attract
(CRYO logo -> crew showcase -> static -> repeat), never stable interactive control.
(2) autolock=false absolute clicks -> DOSBox menu bar appears, still cycles.
(3) autolock=true + mousemove_relative (home to 0,0 then relative to target) + click -> the
mouse is NEVER CAPTURED (DOSBox menu bar stays visible = pointer not grabbed), so neither
absolute nor relative motion drives the int33 cursor; content keeps cycling the attract.
WHY-FAILED: headless Xvfb + DOSBox-X mouse capture/hit-testing -- xdotool events don't reach
the game's int33 mouse. The game is mouse-driven (pyramid-nav + dialogue clicks) so keyboard
alone can't drive it. Same wall a prior dedicated session hit (sess 003). machine=svga_s3,
cycles=max, autolock on/off all tried.
BETTER-APPROACH: (a) run DOSBox-X NON-headless (real X display) where mouse capture works;
(b) the USER runs the game once + captures reference frames of OPTION/gameplay (user confirms
the intro auto-ends into gameplay on a real machine); (c) DOSBox-X MAPPER-scripted input
instead of xdotool; (d) fix the recomp emulator credit divergence so the emulator (which HAS
programmatic inject_key/mouse_press) can reach interactive play. Any ONE unblocks the
interactive-systems RE. STOP-RULE: do NOT keep retrying headless xdotool mouse -- confirmed wall.

## blood.sav byte-format decode (sess: whole-game RE, 2026-07-21)
TRIED: (1) static — strings blood.sav@DS:0x00FC/0x02A9, SAVE@DS:0x258B, game1..10.sav
slot table@DS:0x25FD (spaced 0x20). No `mov dx,imm` (BA xx xx) loads them and 0x25FD has
0 imm16 refs — the game BUILDS the full path ("C:\cblood\"+"blood.sav") in a buffer, so the
open uses the buffer offset, not the raw string offset. imm16 refs to 0x00FC are false
positives (0xFC in jump displacements). (2) dynamic-at-boot — the emulator's 0x3c create /
0x40 write handlers write real files to accuracy/cdrive/cblood/, but NO *.sav is ever created:
the game only OPENS (0x3d) blood.sav at boot (fails, file absent) and never writes it there.
WHY-FAILED: blood.sav is written ONLY on an interactive SAVE (player picks a slot), which is
behind the walled interactive-gameplay gate (see the DOSBox/emulator dead-ends above). Static
anchoring needs tracing the path-builder + the int21 0x3c create site (buffer offset), a deeper
disassembly effort. LOW VALUE: the port already has its own save format; byte-compat blood.sav
is marginal. BETTER: decode it opportunistically IF interactive play is ever reached (a real
save then writes a real blood.sav to accuracy/cdrive/cblood/ to examine directly).

## RESOLVED (2026-07-22) — square-caps glyphs harvested from the concept-menu capture; font is PROPORTIONAL
The "glyph GENERATOR" approach below (watch 0xE8 writes to find the RLE builder)
was abandoned. RESOLUTION: harvest the glyph bitmaps DIRECTLY from the ground-
truth `accuracy/captures/bridge/concept_menu.ppm` (the psychotherapy concept
menu, which the dead-end itself flagged as the reliable state). Method: the grey
text (RGB 138,138,138 = DAC value 34 = palette index 0xE8) is read cell-by-cell;
re-extracting the already-stored 'T'/'A' glyphs at x=170,y=34 matched them
bit-for-bit, validating the convention, then `_` (4px baseline bar) and `4` were
extracted. KEY DISCOVERY: the face is PROPORTIONAL, not fixed-10 — advance =
glyph_pixel_width + 2 (measured: 'I' w1→adv3, most letters w8→adv10, 'W' w9→adv11,
'_' w4→adv6; LIBIDO's glyph starts [170,180,183,193,196,206] only reproduce with
proportional advance). Ported: `_`/`4` added to SQUARE_CAPS_GLYPHS, draw_square_caps
now advances by `square_cap_width(glyph)+2`, list geometry corrected to x=170/y=34/
pitch11. VERIFIED: new oracle `concept_menu_text_matches_live_game_capture` scores
IoU = 1.000 (all 1342 grey text pixels reproduced exactly across the 11 glyph-
count-verified rows TALK..HOW). The RLE-builder decode is no longer needed for
these labels; a full generator is only required for glyphs absent from every
menu capture (J/Q/Z, most digits).

## Square-caps glyph GENERATOR via 0xE8-write watch (2026-07-22)
Tried: watch value==0xE8 writes into the gs:0x175 stream region (and the chunky
buffer seg 0x266c) while opening the MENU submenu from the SCRIPT2 savestate,
to find the builder that bakes the square-caps glyphs into the RLE overlay.
Why it failed: (1) from the resumed post-tutorial state the MENU-row click does
not reliably open the submenu box, so the builder never runs in the watch
window (only UI-state writers gs:0x0a2a/0x0ab4 appear); (2) even when it runs,
0xE8 in the RLE stream is a run/length/fill byte, so a value==0xE8 watch is
ambiguous. Better approach: EXEC-watch the panorama-unpacker's caller for the
box (the routine that sets ds:si=gs:0x175 then calls unpack) and single-step
its stream construction from a state where the box IS open (capture such a
state first — e.g. the psychotherapy concept menu, which opens reliably via the
HOOKSNAP orb click). Non-blocking: 19 letters harvested (span-majority) cover
common words; generator = 100% coverage only.

## Dense hand-atlas from the SCRIPT2 savestate (2026-07-22)
Tried: capture a dense cursor-grid hand atlas by resuming accuracy/script2.state
(fast) and diffing screen_indices vs the decoded panorama. Failed: at the
post-tutorial SCRIPT2 state the background is NOT the clean panorama (dialogue
text, different frame), so every diff blob = the whole 320x200 screen (no hand
isolated). Densifying the atlas must use the CLEAN pre-tutorial console state
(the ~50M-step boot path, as the original working 10-sprite atlas did), not a
savestate whose scene differs from the base panorama.

## Mindscape HNM in the RUNNABLE Rust oracle via naive crop (2026-07-22)
Tried: add mind.hnm frame 0 vs a content-cropped frame_01.png to
tests/oracle_suite.rs. Failed: a naive magick crop+resize (720x540+40+30 ->
320x200) misaligns (mean_abs 63.57, vs the Python compare_oracle.py's proper
1.09). The DOSBox capture is aspect-corrected + scaled; pixel alignment needs
compare_oracle.py's normalization, not a fixed crop. Mindscape stays covered by
the PYTHON oracle (intro-mind-frame01 scenario, 1.09); the runnable Rust suite
covers the bridge ring (exact palette-index decode, no scaling). Don't re-add a
fixed-crop Mindscape scenario — port compare_oracle.py's crop logic if a Rust
HNM oracle is wanted.

## Rust Mindscape oracle — palette gamma mismatch (2026-07-22, follow-up)
Followed up the crop dead-end by SWEEPING mind.hnm frames (0..60) + crops vs a
320x200 resize of frame_01.png: ALL frames sit at ~52-62 mean_abs (best 51.6),
never near the Python oracle's 1.09. Root: it is NOT crop or frame timing — the
DOSBox capture's brightness/gamma differs from the port's 6-bit DAC expansion,
which compare_oracle.py normalizes (brightness/contrast) before diffing. A raw
Rust mean_abs can't match. CONCLUSION: keep Mindscape in the Python oracle
(normalized, 1.09); the runnable Rust suite is exact-palette-index decode of the
bridge (no capture-gamma dependence). To add HNM scenes to the Rust suite would
require porting compare_oracle.py's normalization — not worth it for one scene.

## Rust scene oracle for mind.hnm — frame-selection, not decode (2026-07-22)
Captured the EMULATOR's Mindscape boot frame (2M steps, same-palette, avoids the
DOSBox gamma issue) and compared the port's mind.hnm sequential decode: best 75
mean_abs. Root: mind.hnm is a MULTI-LOGO REEL (frame 0 black -> Mindscape ->
frame 80 = "Microfolie's"); a naive decode-frames-0..N doesn't land on the same
Mindscape frame the emulator shows at 2M steps — it is a FRAME-SELECTION/timing
mismatch, NOT a decoder break (the export/compare_oracle.py pipeline already
verifies Mindscape at 1.09). To add an HNM scene to the runnable Rust oracle,
port the export pipeline's frame-timing (which mp4 frame maps to which HNM
frame at which step), or capture the emulator + port at the SAME reel position.
Not worth it for one scene; the bridge oracle (6 scenarios, exact-decode) is the
solid representative suite. USEFUL BYPRODUCT: confirmed the port's HNM decoder
renders the reel correctly (black->Mindscape->Microfolie's), just at different
frame indices than a fixed step count.
