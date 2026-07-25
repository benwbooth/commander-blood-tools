# Accuracy audit — fix tracking

The assembly-vs-port accuracy sweep (18 subagent auditors, adversarial verification
against BLOODPRG.EXE) produced **40 confirmed candidate inaccuracies**. This tracks
their disposition. Full finding text: the workflow result for run `wf_97ed4d0c-94b`.

**Key caveat discovered during fixing:** the auditors diffed the RAW ASSEMBLY, but
several port constants were deliberately calibrated to ORACLE CAPTURES (the interpreter
running the real EXE). Where the two disagree, **the capture is what the game actually
displays** and wins. So a subset of the 40 are FALSE POSITIVES — the port is already
correct and the raw-assembly reading is the one that's off. Each geometry finding must
be oracle-re-verified before it is "fixed"; changing it blind regresses a pixel match.

## Fixed + committed (35) — assembly-cited, regression-tested, oracle-verified where visual

| area | fix | severity |
|---|---|---|
| vm-records | `field_offset` indexes by `bsf(kind)`, not `kind&0xF` (teleport/aboard guards) | HIGH |
| vm-records | `0xB7` single-bit test/set (was whole-word compare) | HIGH |
| vm-presentation | C5–C8/C3 empty-record query branches (tracer) | HIGH |
| vm-dispatch | **live** `step()` now executes C5–C8 (were an unhandled no-op) | HIGH (found while fixing) |
| subtitle | bold green console font throughout; no thin-white/`0xFF` settle | HIGH |
| bridge-clicks | choice-box + console-box click bands aligned (−3 / −2 removed) | MED/LOW |
| input | state-countdown beat at 8.011 Hz (was 3.2× slow) | MED |
| bridge-clicks | world-candidate box labels centered on their anchor | MED |
| hand-cursor | wrist depth-reach gesture (T recomputed via `st16` L) | MED |
| vm-presentation | owner-resolution primitive (object_offsets, enables 0x6946) | infra |
| vm-records | `0x6946` SET special-slot bookkeeping + raw `0xBC` store | MED |
| menus | concept/topic list menu row-count-centered (oracle-verified) | MED |
| vm-records | `0x6863` SET leaves record unchanged for non-{F5,F6,F7} | LOW |
| hnm-video | palette-block `count==0` is 0 entries, not 256 | LOW |
| audio | chatter burble 4-tick throttle (was 5) | LOW |
| menus | in-window (kind-3) concept box centers each label (was left-align x0+4) | LOW |
| ship3d | nav projection matrix term[1] negates before `>>15` (off-by-one) | LOW |
| bridge | seek initial-distance memo cleared at completion, not at arm | LOW |
| asset-decoders | unmapped subtitle chars skipped (no glyph/advance), not "?" | LOW |
| menus | choice-box min-width floor (100/55); centered box now matches the 40..160 hit-band | LOW |
| vm-records | `0xB8/B9/BD` SET invalidates the arche +0x16 dangling reference | MED |
| vm-records | `0xC4` mode-0 (SET) write guard (`0x6CC3..0x6D01`): both operand objects active + kind/already-set checks, else branch (was unconditional write) | HIGH |
| vm-records | `0xC4` mode-0 already-set/idempotence check (`cx==0xC4` + op2 selector-0x13 field) — same guard | MED |
| audio | chatter burble roll uses the game's PRNG (`BloodPrng` @`0x1CE:0x0B02`) + re-draw-until-different (`0xB8AB..0xB8B7`), replacing a fabricated glibc LCG + increment-on-collision | LOW |
| vm-dispatch | **live** `step()` now executes `0xC1` (`0x6B4C`): QUERY (resolved selector path + direct compare) + non-ship3d SET (`{0xC1, operand, 2}`); was an unhandled no-op | HIGH |
| bridge-clicks | console choice-box hit-band = the DRAWN box `[x0,x1]` (`0x84EE..0x84F6`, shared `choice_box_geometry`), not a fixed `40..160`; fixes the anchor-80 world box (~20px off) and any label wider than 100px | MED |
| bridge-clicks | ALL four choice-box hit-test callers share that geometry (console, MENU submenu, nav chooser, telephone) — band == drawn box by construction | MED |
| vm-dispatch | `0xA8` presentation request (`0x67F6..0x682F`): gated on the `gs:0x67AA` bit1 latch + `gs:0x274F`, sets active line 7 / latch / `0x1FA3=0xFFFF`; the `0xC9` teardown releases the latch | MED |
| ship3d | `0xC1` SET kind-`0x10` NAV path: source list built (`0x624B` ported), kind-1 gate (`es:[operand+2]&2`), and the write redirected to `owner + field_offset(0x13,0x10)` — the port previously wrote the operand record | HIGH |
| ship3d | world-click commit decoded + implemented (`VmMachine::world_click_select`, `0xB20C..0xB27B`): new target -> `gs:0x251B` + C1 record `{0xC1,target,0}` at `orxx+0xA`; same-target and back-row cases match the FSM | HIGH |
| ship3d | nav destinations = one marker per GRANTED destination, gated like the projector's `test [si],0x80` over entities `0x15..0x1F`, from the real baked world point `DS:0x4F09` (10200,12100,900) and camera `DS:0x2F65` (10000,12000,0) — replaces the fabricated 7x4=28-point grid; one APPROX left (marker spread) | HIGH |
| bridge | ring cursor snaps to the 8-unit frame grid each tick (`0x97F6 and [0xa2a],0xfff8`, the `0x97E4` sync every steer/seek path falls into) | LOW |
| vm-dispatch | `0xA8` latches the FIN flag `gs:[0x67BD]` on a `"fin."`-prefixed string (`0x67D8..0x67F0`, ungated) — the finale latch the port dropped | LOW |
| vm-dispatch | **live** `step()` now executes `0xC2` (`0x6E34`): QUERY owner-active + typed match, SET slot-insert + selector-`0x11` `0xFFFF` write (and its no-branch-on-failure asymmetry); was an unhandled no-op | HIGH |
| vm-records | **live** `0xC9` zeroes the whole 3-word record AND the `0xC4` reciprocal selector-`0x13` triple on the related object (`0x6FB9..0x6FF0`); the old 1-word clear left a stale `0xC4` that the new C4 write guard would refuse, wedging the actor out of later presentations | HIGH |
| vm-flags | `gs:0x274F` gets the game's real lifecycle: baked initial value is **0** (file `0xFB6F`), set on cryobox-screen entry (`0x18C4`), cleared on exit (`0x1A48`) — was forced `true` at VM construction, leaving `0xD1`'s Cap'n Bob block open from boot | MED |

## Verified FALSE POSITIVE for the PORT — finding correct for the assembly, wrong for the port's model (5)

- **palette 128–191 bank — ORACLE-REFUTED.** Dumped the live DAC buffer
  `gs:0x5b58` (768 bytes) from three deep savestates (`milestone_script2`,
  `location_visit`, `arrival_probe`) via `MEMDUMP=5b58:768` and diffed it against
  the port's `GAME_SCREEN_PALETTE_DAC`: **0 differing bytes out of 576 in colors
  0..191, and 0 out of 192 in colors 192..255** — byte-identical in all three.
  So the port's baked palette IS what the real game has resident; the per-screen
  loading machinery (fully decoded: staging `gs:0x5251` -> live `gs:0x5b58` at
  `0x8166`, backup `gs:0x5851` at `0xB563`, all 192-colour copies) demonstrably
  produces the same palette in every state that can be observed. The finding's
  premise — that the port's 128–191 bank is wrong — is refuted. CAVEAT: all three
  states are SCRIPT2-era, so this does not prove correctness for screens the
  harness cannot yet reach; if a future state shows a differing palette, the
  decoded machinery above is what to wire.

- **`0x6946` query nuance (0xAD/AF/B2/B3/BA/BB/BC):** VERIFIED the port's query arm is
  an EXACT match for `0x6954..0x6983`: the special-object -> `0xFFFF` wildcard
  substitution (`raw==gs:[0x674e]`), the `val == rec_read(off)` compare (= `cmp ax,
  es:[bx+di]`), and the four-way inversion branch `(eq && flipped) || (!eq && !flipped)`.
  The match-anything-vs-aboard subtlety was already fixed (in-code note at the arm). No
  change — the port is faithful here.

- **subtitle console multi-line pitch (10 vs asm `add dx,8`):** the on-console 10px
  pitch is TUTORIAL4 oracle-calibrated (console rows measured at y=8/18 = pitch 10);
  the raw-assembly `add dx,8` disagrees with what the game displays here. No change.


- **choice-box `[0xadd]=1` tall-mode (+10):** the `choice_box_bob_morlock.ppm` capture
  shows the 2-row box at y=89/100 = the current `+8` formula; `+18` would put it at 84
  and break the pixel match. No change. (`choice_box_top_y` comment records this.)
- **C4 query owner-active gate (staleness judgment call):** the assembly gates the C4
  query on the owning object's active bit (`0x6CA4 test es:[di+2],1`). CORRECTION: the
  VAR *does* load initial object active bits (121/228 SCRIPT2 objects have `obj+2 bit0`
  set), so owner_active isn't uniformly false — my fix-#14 revert reasoning was partly
  wrong. CORRECTION (2026-07): `owner_object_offset` (largest object offset strictly
  below the key) is NOT an approximation — it is EXACTLY `0x6034` (which walks the
  `0x672c` directory by `+0x10` threshold with `jbe` then backs up one, i.e. the largest
  threshold strictly less than the key; verified including the equality edge). And the
  runtime active-bit LIFECYCLE is now decoded (see below): the only runtime writer is
  `0x5B8D` (a CLEAR in the C1 world path), so the C4-QUERY staleness fear was overstated.
  The query gate is still left as `active_actor==Some(off)` only because the query answers
  "is THIS actor's presentation the active one?" — a stronger condition than "object
  active" that the single-presenter model already captures; the assembly's `es:[di+2]&1`
  is necessary-but-not-sufficient, so the port's model is not wrong here.

### Active-bit lifecycle — DECODED (unblocks the C4-write/`0xC1` guards)

The object "active" bit is `object_record[+2] bit0` (object records live in the
`0x6724` segment; kind at `+0`, flags at `+2`). Enumerating every `or/and byte
[reg+2],imm` site in BLOODPRG.EXE finds **exactly one** runtime writer: `0x5B8D`
`and byte [bx+2],0xfe` in the C1 world/ship-presentation ladder (`record_type_ladder`
`0x5B38`), which only CLEARS the bit of a kind-`0x20` linked object. There is **no**
`or [reg+2],1` setter anywhere. So the runtime NEVER activates a VAR-inactive object —
for the dialogue C4 flow the active bits are effectively VAR-initial static. This makes
reading them in the C4 SET guard faithful to the game's write/branch decision (the sole
divergence is the C1-clear case, a separate subsystem the live VM does not run). The C4
mode-0 write guard is now IMPLEMENTED on this basis (see the Fixed table).

## Remaining (1 sub-item) — `rec_103A`'s native writer

ACCOUNTING NOTE, to avoid inflating the tally: the last ledger row was ONE finding
covering TWO sub-items — the per-screen palette pipeline and the `rec_103A` writer.
The palette sub-item is now ORACLE-REFUTED (see the false-positive section: the live
DAC is byte-identical to the port's baked palette in three deep savestates), so the
port needs no change there. `rec_103A` is still open, so this row is NOT closed.

Everything below this note is HISTORICAL — the items were resolved earlier in the
session and are kept for their decode trails.

- **Geometry — choice-box x-band is now FULLY CLOSED:** all four hit-test callers
  (console box, MENU submenu, on-bridge nav-destination chooser, telephone contacts)
  route through the shared `choice_box_geometry` (decoded `0x84A1..0x84F6`), so each
  click band equals its drawn box by construction — the draw and hit-test read the same
  `console_box_kind`/labels in the same frame, so they agree whatever the box's
  anchor/width. Verified: `console_box_click_band_is_the_drawn_box_not_a_fixed_40_160`
  plus the existing nav/telephone click tests stay green.
- **Palette 128–191 bank — the FLOW IS NOW FULLY DECODED; what remains is wiring +
  per-screen verification.** Three buffers, every transfer 192 colors (0..191), so the
  192..255 console/text bank is never touched by per-screen loading — which is precisely
  why this finding is the 128–191 bank:
  * `gs:0x5251` — per-screen STAGING, filled from the master by the panorama frame
    loader (`0x981B`) when `gs:[0x5B53]&1`;
  * `gs:0x5b58` — the LIVE 768-byte DAC buffer, uploaded by `0x16A7` -> `0x2F90`;
  * `gs:0x5851` — a 576-byte BACKUP (saved at `0xB563`, restored via `0x1FA1`).
  The loader is `0x8166..0x816F` (`si=0x5251, di=0x5b58, cx=0x90 rep movsd`), run on
  entry to the ship/nav state right after `mov [0x24F3],1`. `0xB426` sets
  `[0x5B53]=1`/`[0x5B57]=1` on ship-3D sequence entry.
  PORT GAP: `src/tbbig.rs` renders from a STATIC `GAME_SCREEN_PALETTE_DAC` and models
  none of the three buffers nor the refresh flag. Wiring it needs each screen's staging
  source identified and the hub capture re-verified (the current cyan IS correct there),
  which is why this stays open rather than being patched blind.
  The original "restore baked bytes" hint is WRONG — it would
  make the HUB muddy and break its capture; the cyan IS correct for the hub. The real fix
  is per-screen palette loading via the `0x5251<->0x5b58` working-buffer flow.
- **Infrastructure-gated VM guards** — the active-bit lifecycle is now DECODED (see
  above; runtime never sets a VAR-inactive object). RESOLVED on that basis: C4 mode-0
  write guards, the C4 mode-0 already-set check (both in the `0x6CC3..0x6D01` decision),
  and **`0xC1` line-record state — the live opcode is now handled** (`step()` executes the
  QUERY resolved+direct paths and the non-ship3d mode-0 SET, `0x6B4C`). What remains of the
  C1 story is only the ship-3D nav-source SET path (`write_c1_record_state_ship3d`) and the
  frontend world-click that creates C1 records (`0xB272`) — both fold into the
  nav-destination/ship-3D rewrite below, which needs the frontend ship-3D runtime, not the
  VM opcode.
- **`0xA8` side effects — the FIN-FLAG HALF IS FIXED; the presentation-request half is
  not.** DECODED (`0x67C8`): after the string copy the handler sets `gs:[0x67BD]=1` when
  the operand starts with `"fin."` (**DONE** — ungated and unconditional in the assembly,
  `0x67BD` is far above the max VAR so it is alias-safe; now `VmMachine::fin_requested`),
  then (if `!(0x67AA&2)` and ship-active `0x24F3&1`/`0x274F&1`) fires a presentation
  request (`0x6788=7`, `0x67AA|2`, `0x1FB2=0`, `0x1FA3=0xFFFF`, `0xB3B=0`) — **STILL
  OPEN**. Three concrete blockers, all verified:
  (0) the gate's `gs:[0x274F]` operand is modelled as `flag_274f`, which the frontend
  sets `true` at VM CONSTRUCTION (`main.rs:805`) — an approximation of the game's runtime
  D1 flag. Since the gate is an OR, building on it would fire the request on the FIRST
  `0xA8` of every script, far more often than the real game; fixing `flag_274f`'s model
  is the prerequisite.
  (a) the FIRE GATE needs ship-active `gs:[0x24F3]&1`, which lives in the frontend
  (the VM holds only `flag_274f`), so a VM-only version would mis-fire; and
  (b) **the gs-offset ALIASING HAZARD below** makes the `0xB3B=0` write unsafe.

### gs-flag ALIASING HAZARD in the port's single-array state model (verified 2026-07)

The port keeps ONE array (`line_records` live / `state` in the tracer) that serves both
the VAR record table AND the gs-relative engine flags — the tracer already relies on this
(`state_u8(state, 0x67AA)`, `0x24F3`, `0x2751`, `0x1FB2`). In the REAL game these are two
DIFFERENT segments: records live behind the far pointer at `gs:0x6724`, while `0x67AA`
etc. are DS offsets, so they can never collide. In the port they share an index space, so
a gs flag only stays safe while its offset is ABOVE the loaded VAR length.

Measured VAR sizes: SCRIPT1 `0x123A`, SCRIPT2 `0x1312`, SCRIPT3 `0x144E`, SCRIPT4
`0x1534`, SCRIPT5 `0x1390` — max `0x1534`. So:
- SAFE (above every VAR): `0x1FA3`, `0x1FB2`, `0x24F3`, `0x2751`, `0x6788`, `0x67AA`,
  `0x67BD` — the flags the tracer already models.
- **UNSAFE**: `gs:[0xB3B]` (= 2875) sits INSIDE every VAR, so modelling A8's `0xB3B=0`
  as a state write would silently corrupt a real record. Any future gs-flag modelling
  must check the offset against `var_len` first (or move engine flags to their own map).
- **A6 per-line C4 gate** — DECODED (`0x6647..0x6683`): the play gate is FIVE conditions
  — (1) b5 bit7 active, (2) not `0x5E64||0x67B0` busy, (3) line record `+2` bit15 (already
  shown) clear, (4) the line record's selector-`0x13` field (`matrix[0x131]`) `== 0xC4`,
  (5) `vm_condition_5` (`0x6339`) random. The live port models #1 and #5 faithfully and
  approximates #2+#4 with the GLOBAL `presentation_busy` flag. Tightening #4 to a real
  per-line record check is SUBTRACTIVE (skips lines) and needs the presentation-start C4
  field set by the `0x5816` scan — so it risks MISSING dialogue, and the dialogue
  playthrough harnesses currently pass on the approximation. Coupled to the
  presentation-record lifecycle (same subsystem as C1), not a safe drop-in. (The port
  DOES already model the tracer-path per-line gate behind `with_text_presentation_record_gating`.)

- **Rewrites:** nav destinations = flag-gated entity set (fabricated pyramid grid; needs
  entity world coords + active bits); A6 reveal-busy serialization handshake (VM↔frontend).
- **World-destination click — DECODED and IMPLEMENTED at the VM layer.** The commit
  path is now `VmMachine::world_click_select` (tested: new target writes the C1 record
  at `orxx+0xA`, the current target is not rewritten, the back row `-1` clears the
  target and writes nothing). It has a real consumer: the live `0xC1` QUERY added this
  session lets scripts test that record. REMAINING (frontend binding only): the port's
  nav click yields a destination INDEX (`nav_destination_click` -> `SCRIPT3+i`), not the
  target's RECORD OFFSET, and the index->record mapping lives in the destination entity
  set `0x15..0x1F` — which is zeroed in every observable savestate (see dead_ends.md).
  Passing a made-up offset would be exactly the fabrication the prime rule forbids, so
  the binding waits on a state where those entities are populated.
  Original decode: the ship
  FSM (`0xAFA0`) calls `ship_3d_target_record_select` (`0xB2BB`) each frame, which scans
  the target list `DS:0x250B` (fallback `DS:0x2537`) and hit-tests the mouse against the
  projected target layout via `lcall 0x71E:0xC48` (gated by `0x27E6`, layout `DS:0x2AAB`/
  `DS:0x2545`). Its return is the selected target record: `0` = none, `-1` = back/exit
  (`0x24F3=0x11`), else a target. On a NEW target (`!= gs:0x251B`, `0xB21A`) it writes
  `gs:0x251B = target` (`0xB224`) and CREATES a C1 record `{0xC1, target, 0}` at
  `[0x6750]+0xa` (`0xB272`) — which the C1 ladder (`0x5B38`) then presents. So the
  world-click and the C1 subsystem are ONE mechanism: click a projected world target ->
  C1 record -> C1 presents it. Porting is thus part of the C1/ship-3D subsystem session,
  not a separate undecoded task. The remaining unknown is only the exact mouse->row
  rectangle math inside `0x71E:0xC48`.
- **Infrastructure-blocked:** ending fires on all-visited (the `rec_103A` runtime write,
  documented separately in port-validation.md).

### C1 nav-source SET — current precise state (was "needs the ship-3D runtime")

PORTED and tested at the VM layer:
- the source-list builder `0x624B` (`VmMachine::build_nav_source_list`) — pure
  record logic over the `0x672c` directory, no frontend state;
- the kind-`0x10` branch of the SET: gate on a source entry, then write
  `{0xC1, operand, 2}` at `owner + field_offset(0x13, 0x10)` (NOT the operand
  record, which is what the port used to do), destination-empty check included;
- the kind-1 entry gate `es:[operand+2] & 2`.

STILL OPEN — one narrow thing: the kind-2 entry gate. It is already faithfully
modelled in the tracer (`ship3d::select_ship_3d_c1_source_record`, which treats
the `0x6886` buffer as entries followed by a bitset indexed off the source
cursor — `si + field_offset(5,2) = si+30`). The live path can't use it yet because
`build_nav_source_list` returns only the entry `Vec`, and — the actual unknown —
**nothing identified so far POPULATES those bitset bytes**. Searching the binary
finds no direct reference to that address; the region is only ever reached through
a pointer walked from `0x6886` by the display/candidate builders
(`0x6FF3..0x70ED`, `0x70EE ship_3d_navigation_candidate_build`), so the writer is
one of those paths and needs tracing.

Until that writer is identified, kind-2 entries are treated as never passing.
Wiring them against a guessed source would silently mis-gate presentations.

## FUNCTION-AUDIT FINDING: `vm_token_advance` has a second MODE the port omits

`walk` / `token_len_at` model `vm_token_advance` (`0x62B6`) faithfully for the
EXECUTION path — verified line by line:
- entry = `table[0x6F18 + (op-0xA0)*2]`, two bytes per opcode;
- `[bp+1]` bit7 set = sentinel: `0xFF` sets mode 1, `0xFE` clears it, `0xFD`
  consumes an optional `0xA1`, each then using length `[bp+0]`;
- otherwise length = `[bp + gs:[0x67AD]]`, i.e. `b0` in mode 0 and `b1` in mode 1
  (`0x62CD..0x62D4`) — exactly the port's `if mode1 { b1 } else { b0 }`;
- a resolved length of ZERO falls into the variable-length path at `0x631A`, where
  `0xA6` skips 5 bytes then scans words to `0x0000` and everything else calls
  `vm_token_special` (`0x6293`) — matching `decode_text` and `scan_zero_word`.

**The gap:** at `0x6307`, BEFORE any of the above resolves a length,
`test gs:[0x67B2],1` jumps straight to the variable-length path. So when that flag
is set, EVERY token is treated as zero-word-terminated regardless of its table
length. The port has no equivalent.

The flag is real, not dead: baked 0, cleared at VM init (`0x55F9`), and SET to 1 at
`0x5710` and `0x73C2` — both of which immediately walk a stream (`0x73C2` loads
`gs:0x6724` + `gs:0x6720`). So the game has a SECOND token-walking mode used by
those scanners, distinct from execution.

NOT changed blind: the port's `walk` is pinned by `walks_real_scripts_to_documented_token_counts`
(214/3271/3281/1714/1869 tokens), and forcing the alternate mode would change every
count. The correct fix is to identify what `0x5710`/`0x73C2` are scanning and give
the port a separate scan-mode walker for those call sites only — not to alter the
execution walker.

## FUNCTION-AUDIT FINDING (FIXED): the post-update scan iterated MORE objects than the game

The scan at `0x5816` walks the `gs:0x672c` directory and — per the shared walk
idiom (`0x624B`, `0x604E`) — CONTINUES ONLY WHILE the next entry's `+0x12 == 1`,
stopping at the first entry whose kind is not 1.

Measured against the shipped DEBs, that terminator matters a lot:

| script | kind==1 prefix | total entries |
|---|---|---|
| SCRIPT1 | 122 | 136 |
| SCRIPT2 | 122 | 341 |
| SCRIPT3 | 130 | 352 |
| SCRIPT4 | 136 | 243 |
| SCRIPT5 | 130 | 243 |

`load_deb_objects` / `ExecutionContext::from_object_offsets` keep EVERY symbol, so
the port's `post_update_execution_state` loop iterates all 341 SCRIPT2 objects
where the game visits 122. The extra ~219 are still gated on the active bit
(`owner+2 & 1`), so they only take effect when active — but the VAR does load
initial active bits for a large share of objects, so this is not obviously inert.

CHECKED AND CLEARED (a suspicion that turned out wrong): the port SORTS
`object_offsets` while the game walks the directory in file order, which would
change scan order. Measuring the kind==1 prefix shows it is ASCENDING in all five
scripts, so sorted order and directory order agree over the scanned range. The
sort is also what makes the `0x6034` threshold lookup valid. No divergence there.

**FIXED 2026-07-24** in `VmMachine::load_deb_objects`, which now keeps only
kind-1 entries. Measurement made this safe and exact: the leading kind-1 PREFIX
equals EVERY kind-1 entry in all five scripts (122/122, 122/122, 130/130,
136/136, 130/130), so a simple kind filter reproduces the scanned set precisely.
The extract path already filtered this way; only the live VM did not. Beyond the
scan extent this also repairs `owner_object_offset`, which could previously return
a NON-OBJECT offset as the "largest below the key" and mis-resolve the owner for
the `0x6034` threshold lookup. Test: `load_deb_objects_keeps_only_kind1_entries`.

ORIGINAL FIX SHAPE (superseded by the simpler filter above): the scan should stop
at the first non-kind-1 directory entry. `VmMachine` already carries `directory`
as `(offset, kind)` pairs; `ExecutionContext` carries only offsets, so giving the
scan the kind-terminated prefix means threading kinds through it. The threshold
lookup must KEEP the full sorted list — the two uses need different views of the
same table, which is exactly what the current single `object_offsets` conflates.

## FUNCTION-AUDIT FINDING: the b3 -> voice-clip mapping has NO located binary support

`text_selector_voice_clip_index` / `text_selector_requests_voice` encode the rule
"`0x00` and `0xFF` are no-voice; `1..N` select a one-based `son.snd` talk clip".
Their own doc comment calls this "current evidence" — i.e. an inference. Two
searches now argue against it having a binary basis, at least on the obvious paths:

1. **The selector has exactly ONE consumer.** `gs:0x1FAB` (written from A6's `b3`
   at `0x668F`) is read at exactly one site in the image — `0x11F2`, which computes
   `+9 -> gs:0x6788`, the ACTIVE LINE ID that this session verified. No site reads
   it as a clip index.
2. **Both clip-play call sites are the chatter path.** `snd_play_clip` (`0xB8CD`)
   has exactly two near callers, `0xB895` and `0xB8C0`, and both sit inside the
   burble/chatter routine, selecting `7 + rand(10)` from `tb.snd`. Neither consults
   the selector.

NOT over-claimed: this does not prove the port is wrong. Voice could be reached by
a far call, through the SND bank loader (`0xC005`), or driven from the active line
id rather than the raw selector. What it does establish is that the mapping is
UNVERIFIED and the two most likely routes do not implement it.

WHY THIS MATTERS: this is content-shaped behaviour (which voice line plays) derived
from inference rather than from the binary, which is exactly what the prime rule
warns about. The rows stay UNVERIFIED with this note. The open task is to find the
routine that turns a line's selector into a played `son.snd` clip — a write-watch
on `gs:0x0C47` (the son.snd handle) or on the SND bank table during a spoken line
would locate it.

### …and the ACTUAL voice mechanism, located: `mu\<NAME>.voc`

Following the previous finding to its answer. `DS:0x0D2D` holds the TEMPLATE
string `mu\xxxxxxxx.voc`, and `0x125A` writes `0x78` (`'x'`) back into it — i.e.
the eight `x`s at `DS:0x0D30` are a patch field, not a literal name.

The patcher is at `0x77A9`:
- `di = 0x0D30` (the patch field);
- `lodsb` from a NAME string, terminating on any byte `<= 0x20` or negative
  (`0x77AD..0x77B3`);
- UPPERCASES lowercase input (`cmp al,0x61; jb; and al,0xdf` at `0x77B5..0x77B9`);
- compares each char against what is already there and, on any difference, sets
  `gs:0x0BA1 = 1` — a "filename CHANGED" latch (`0x77BB..0x77C0`);
- writes the char and loops; afterwards, if nothing changed, sets `gs:0x0BA0 |= 1`.

So character speech is a PER-RECORD `.VOC` FILE named after the record
(`mu\NAME.voc`), with a change-latch so an unchanged name can skip a reload. That
is a completely different mechanism from indexing clips out of `son.snd`.

CONSEQUENCE for `text_selector_voice_clip_index`: the port resolves speech as a
one-based `son.snd` clip index derived from A6's `b3`. The `son.snd` bank is real
and IS used — the chatter/burble path plays `7 + rand(10)` from `tb.snd` — but the
located character-voice path builds a filename instead. The port's model therefore
looks like the wrong mechanism for line speech, which is why those rows must stay
UNVERIFIED.

STILL NOT PROVEN, deliberately: I have not yet traced which name string `si` points
at when `0x77A9` runs, nor confirmed it is the speaking record's name, so the exact
selector -> filename relationship is open. That trace is the next task; the
mechanism, though, is now located rather than inferred.

#### Voice trace: how `0x77A9` is reached is STILL OPEN

Attempted to close the last step (which name string `si` points at). Result is
negative and recorded as such rather than guessed:

- `0x77A9` has NO near callers (`e8` scan over the whole image) and NO near jumps
  (`e9` scan).
- It is not the fall-through of `fs_name_area_read` (`0x7788`), which `ret`s at
  `0x77A8`.
- Its segment-`0x4DA` offset is `0x2409`, and that word does appear once at
  `0x7540` — but `0x7540` sits in a DATA blob (it disassembles as nonsense), and
  `0x2409` is NOT present in the VM opcode handler table at file `0x142D0`. So the
  hit is most likely coincidental, not a dispatch entry.

Therefore the patcher is reached by a FAR call or through a computed/table
dispatch that has not been identified. Until that caller is found, the binding
"which record's name becomes `mu\<NAME>.voc`" is unproven, even though the
mechanism itself is now certain.

NEXT STEP (concrete): the harness can settle this directly rather than by static
search — arm an execution watch on file `0x77A9` (the probe already supports
`EXECWATCHLIN`, and `Cpu::exec_watch_dump_regs` added this session prints the live
registers) and drive a spoken line. The dump gives both the caller (via the saved
return address the watch already captures) and `si`, i.e. the exact name source.

#### Voice trace, dynamic attempt: watch armed, ZERO hits (and sound was ON)

Ran the recorded next step rather than leaving it as advice. Added `EXECREGS=1` to
the scenario exec-watch so hits dump live registers, then armed file `0x77A9` over
`accuracy/scenarios/story_deep.tsv`:

    VERIFYSCRIPT=...story_deep.tsv EXECWATCHLIN=0x77A9 EXECREGS=1 -> 0 hits

The watch itself is proven functional: the same address formula
(`0x1a20 + file - 0x600`) produced hits earlier for the menu compare at
`0x8656`/`0x8663`. So the patcher genuinely never executed.

Hypothesis TESTED AND REJECTED: that the interpreter had sound disabled, which
would make `snd_play_clip`'s `test gs:[0xADE],1` skip the audio paths. Dumping
`gs:0x0ADE` from the SCRIPT2 milestone gives 1 — sound is ENABLED.

REMAINING LEAD (hypothesis, not proven): the concrete filename `mu\tablo2.voc`
sits at `DS:0x0D3D` right beside the template, and labels.csv ties `DS:0x0BA3`
(`voc_tablo2_active`) to NAVIGATION-CHOICE HANDLER 4 — which is one of the 18
functions this session measured as DORMANT. If this VOC machinery belongs to the
nav/tablo2 flow rather than to ordinary character speech, zero hits is exactly
what a dormant nav subsystem would produce, and the two findings are the same
problem again. That would ALSO mean the port's `son.snd` clip model is not
contradicted by this path at all.

Deliberately unresolved: proving it needs a scenario that reaches nav voice, which
is blocked behind the same wiring work. Recorded so the next attempt starts from
the lead rather than repeating the search.

## FIXED #41 — kind-10 choice box was 10px short (5px misplaced)

`0x8438` is the unified list widget. Its `[0xADD]&1` branch does TWO things and the
port modelled only one:

    0x8436  xor bp, bp      <-- default height seed 0    (port was right)
    0x8442  mov bp, 0xa     <-- kind-10 height seed 10   (port was MISSING this)
    0x8445  mov dx, 0x37    <-- kind-10 width floor 55   (port had this)

`bp` accumulates `add bp,0xB` per row (`0x847A`) then `add bp,8` (`0x84A7`), and the
box is centred at `(200-h)/2` (`0x84B9..0x84BF`). So the kind-10 (world/entity) box
is `rows*11 + 18` tall, not `rows*11 + 8`, and sat **5px too low** — along with its
clickable rows, since the hit-test shares the origin.

This was logged as an open task with the note that fixing only the kind-10 seed was
unsafe while the default seed was unverified — the default `bp` looked like it was
inherited from the caller (pushed at `0x842E`, no visible init). That was wrong: the
init is the two bytes `33 ed` at `0x8436`, between `push si` and `mov dx,0x64`, which
the first read had skipped over. `xor bp,bp` confirms the port's implicit 0, which
made the kind-10 fix safe.

Fix: `choice_box_top_y_seeded(rows, seed)` with `choice_box_text_top(&self)` picking
the seed from `console_box_kind`, used by BOTH the draw and the hit-test so they
cannot drift apart. Regression test asserts the 5px offset for kind 10 and no change
for other kinds.

## FIXED #42 — HONKF console font had `,` `:` `;` rotated

`console_glyph_index` mapped `,`=39, `:`=40, `;`=41. The bank's own 8x8 bitmaps say
otherwise: **39 = `:`, 40 = `;`, 41 = `,`**.

Read off the glyph cells, the three are told apart by two marks — an upper dot
(row 2/3) and a descender tail (row 7):

| frame | upper dot | tail | glyph |
|-------|-----------|------|-------|
| 38    | no        | no   | `.`   |
| 39    | yes       | no   | `:`   |
| 40    | yes       | yes  | `;`   |
| 41    | no        | yes  | `,`   |

Corroborated independently by the BUILT-IN font's translation table at `DS:0x7802`,
decoded earlier in this campaign, which orders `'.'=30, ':'=31, ';'=32`
consecutively with `','` separated at 37 — the same relative order.

How it was found: the routine is `#[allow(dead_code)]` with no runtime callers, and
the suspicion was that its mapping was INVENTED (the filename `honkf.spr` lives in
an alphabetical table of character-sprite names — `hanz`, `izwalito`, `jerry` — so
it looked like entity art, not a font). Checking the asset refuted the fabrication
theory: 49 frames, all 8x8, and the mapping covers exactly 0..48
(26 letters + 10 digits + 13 punctuation). Rendering the bank then confirmed
`0..25 = A..Z` and `26..35 = 0..9` exactly as mapped, and caught the rotated trio.

The regression test asserts the mark pattern read FROM THE BITMAPS rather than
restating the constants — a test that repeated the mapping would have agreed with
the bug. Verified it fails on the old ordering before restoring the fix.

## FIXED #43 — the font's xlat table was truncated to 128, dropping every accent

`GAME_FONT_CHAR_MAP` was `[u8; 128]`. The game's table at `DS:0x7802` (file
`0x14C22`) is **176 bytes** — it runs to the advance table at `0x14CD2`. The 48
missing entries are not padding; 14 of them are real characters the game draws:

    0x81 ü   0x82 é   0x84 ä   0x85 à   0x87 ç   0x89 ë   0x8A è
    0x8B ï   0x8D ì   0x94 ö   0x95 ò   0x97 ù   0xA8 ¿   0xAD ¡

Two defects compounded. Even with the full table, nothing would have rendered,
because game strings were decoded with `String::from_utf8_lossy` — and CP437 `0x82`
is an invalid UTF-8 lead byte, so `é` became U+FFFD before reaching the font. Both
are fixed: `cp437_string` for decoding, `cp437_byte` to index the table by CP437
byte rather than Unicode scalar (`é` is U+00E9 = 233, past the table).

Real data affected: `glycérium` in SCRIPT3.DIC — a DISPLAYED dictionary word, which
rendered as `glyc<?>rium` — and `porte_clés` in SCRIPT1/2.DEB.

WHY IT SURVIVED SO LONG (the part worth remembering): there was already a test named
`glyphs_match_bloodprg_exe_byte_for_byte`, and it passed. It looped `33u8..127` —
printable ASCII only — so it never looked at the range that was missing. A
byte-for-byte name over an ASCII-only comparison. It now asserts
`GAME_FONT_CHAR_MAP == exe[map..advances]` for the whole table, so the length itself
is pinned and any future truncation fails immediately.

This also corrects my own earlier campaign entry, which recorded the subtitle font as
verified byte-for-byte at "128 + 86 + 688 bytes". The 128 was wrong; it is 176.

## FIXED #44 — the CP437 decode bug was duplicated across 14 more sites

Fix #43 corrected the DIC/DEB decoders in `engine.rs`. The same parsers are
DUPLICATED elsewhere, carrying the identical bug:

- `concept_menu.rs` and `bas_vm.rs` — each re-parse the dictionary, and both feed
  DISPLAYED menu text. `glycérium` was corrupted on these paths too.
- `script.rs`, `extract/script.rs` (5 sites) — DEB symbol names.
- `descript.rs::decode_text`, `extract/descript.rs` (3 sites) — DESCRIPT text.

All now use `font::cp437_string`. The DESCRIPT ones are a NO-OP on shipped data
(measured: 1227 text runs in DESCRIPT.DES, none with high bytes) and are changed for
consistency, not because they were producing wrong output — recorded that way rather
than claimed as fixes.

DELIBERATELY NOT CHANGED: ffmpeg stderr in `extract/hnm.rs` and `extract/character.rs`,
and the debug needles in `bin/blood.rs`. Those are genuinely UTF-8 tool output, not
game data; converting them would be cargo-culting the fix.

### A measurement that was wrong twice before it was right

Checking whether COD inline strings (subtitle text) contain accents, the first scan
reported **94 of 130** strings with high bytes — which would have made this a much
bigger finding. It was an extraction artifact: the regex began runs at a preceding
BYTECODE byte that happened to fall in 0x80..0xAF, producing "strings" like
`¿aarche10.hnm` where `0xA8` is the opcode before the `aarche10.hnm` operand.

Bounding runs by NUL on both sides did NOT fix it (93/93) — the opcode sits directly
after the previous string's terminator. Only requiring an ASCII first byte and
counting INTERIOR high bytes gave the real answer: **0 of 5**. COD strings contain no
accents at all.

Worth keeping because the wrong number was the plausible one: a French-developed game
"obviously" has accented dialogue, and 94/130 confirmed the expectation. The correct
answer was zero.

## FIXED #45 — choice-menu rows were being spoken as part of the subtitle

An `0xA6` record's word list has TWO sections split by `0xFFFF`: the spoken line, then
the CHOICE-MENU rows. `load_dialogue` built subtitle text with `filter_map` over the
WHOLE list. `0xFFFF` is not a DIC key so the separator vanished silently — but the menu
rows after it were kept and glued onto the sentence.

Measured blast radius: **211 of the 3650 `0xA6` lines** across the five scripts carry a
menu, so every one of them rendered a corrupted subtitle. SCRIPT1.COD's line came out as

    Click quick, Cap'n Bob is waiting... explanations game

instead of ending at `...`.

This is the same defect found minutes earlier in the script disassembler (#44's
follow-on), which is what prompted looking for it on the gameplay path. Two of the
three VM consumers already did `take_while(|w| w != 0xFFFF)`; the disassembler and the
engine's subtitle builder did not. The lesson is that the correct handling existed in
the codebase and simply had not been applied uniformly — worth grepping for the other
`word_offsets` consumers whenever this record type changes.

### MENU_SUBMENU provenance closed

The menu rows are now retained per line offset (`menu_by_offset`) and
`menu_submenu_labels()` sources the submenu from the LOADED SCRIPT, upper-casing for
display exactly as the widget does. `MENU_SUBMENU` remains only as the documented
fallback for an engine with no script loaded (unit tests, bare `EngineState::new()`),
so the const is a default rather than the authority. The draw and the hit-test both
call the accessor, so the clickable band cannot disagree with what is drawn.

## FIXED #46 — swept every `0xA6` word-list consumer; 3 of 6 were wrong, 3 ways

After #45, swept the codebase for every place an `0xA6` word list is turned into text.
There are six. Three were correct; three were broken, and each failed DIFFERENTLY —
which is why no single earlier fix caught them:

| consumer | before | symptom |
|---|---|---|
| `vm.rs` (2 gameplay sites) | `take_while != 0xFFFF` | correct |
| `script.rs:272` | explicit split | correct |
| `vm_drive.rs:36` | `take_while != 0xFFFF` | correct |
| `engine.rs load_dialogue` | `filter_map` over all | menu glued to subtitle (#45) |
| `bas_vm.rs:206` | `filter_map` over all | menu glued to BAS line |
| `extract/script.rs decode_vm_words` | `collect::<Option<_>>()?` | **returned None**, and both call sites `continue` — silently DROPPED all 211 menu-bearing lines from extraction |

The third is the nastiest: it did not corrupt output, it removed it. A line with a
choice menu simply did not appear in the extraction, so the omission was invisible in
the result — nothing looked wrong, there was just less of it.

Added `resolving_a_word_list_never_yields_menu_words`, which pins the shared INVARIANT
instead of any one call site: resolving a list containing a separator must never yield
a menu word, a list without one must be unchanged, and `0xFFFF` must never be treated
as resolvable.

Method note worth keeping: the productive move was not decoding anything new. It was
taking a defect found in one place and grepping for every other consumer of the same
data shape. Three of the six were wrong; the correct handling had existed in the same
codebase the whole time.

## FIXED #47 — consolidated four byte-identical dictionary parsers into one

`parse_dictionary` existed in FOUR copies (`engine.rs`, `script.rs`,
`concept_menu.rs::parse_dic`, `bas_vm.rs::parse_dic_words`), all behaviourally
identical. That duplication is not incidental — it is the mechanism by which the CP437
defect (#43/#44) came to be fixed in one place and left wrong in three, and the same
shape produced the `0xA6` word-list divergence (#46).

The three copies now delegate to `script::parse_dictionary`, which is documented as
the single implementation.

### Two checks in the same sweep that CLEARED the code

Recording these because a sweep that only reports defects gives a false picture of
where the risk is:

- **DEB record parsers** (7 sites, `chunks_exact(20)`): all filter `kind` correctly for
  their purpose (`==1`, `1||5`, or an explicit match). The divergence that looked
  suspicious — some guard against empty names, some do not — cannot bite: empty names
  occur ONLY on `kind==0` header records (5, one per script), which every kind-filtered
  parser excludes, and both parsers that see all kinds do guard.
- **`start as u16` truncation** in the dictionary key: safe by the format, not by luck.
  Word offsets are `u16` in the COD, so a DIC cannot exceed 64 KiB; the largest shipped
  file is 24772 bytes.

## FINDING — the voice-clip index is a port invention (not yet removed)

`vm.rs::text_selector_voice_clip_index` maps the A6 selector `b3` to a son.snd talk
clip as `b3 - 1`. A byte search for every reference to `DS:0x1FAB` — where the handler
stores the sign-extended `b3` — gives the complete picture:

    0x668D   lodsb / cwde / mov gs:[0x1fab],ax     the ONE write of b3
    ~0x1A64, 0xB460, 0xB529   mov word [0x1fab],0xffff   resets
    0x11F2   mov ax,[0x1fab]; add ax,9             the ONE read -> line id gs:0x6788

Exactly one reader, and it forms the active line/scene id (traced onward to the
graphics dispatcher `0x9D10`, which has no audio call). Nothing indexes a sound table
from `b3`.

So: the son.snd BANK is the correct audio system for dialogue (loader `0xC005`, handle
`DS:0x0C47` "voices/SFX") — that part of the port is right. Choosing a clip by
`selector - 1` is the invented part.

NOT REMOVED, deliberately. Deleting it silences dialogue with nothing to put in its
place, which would trade a wrong-but-working surface for a missing one. The task is to
find how son.snd clips are actually selected and swap the rule.

Process note: while confirming this I first dismissed three of the five byte matches as
false positives, having disassembled from the OPERAND byte rather than the instruction
start. They are real — `mov word [0x1fab],0xffff` resets. Same mis-slicing that
produced a spurious "32 mismatches" on the pyramid vertices earlier in this campaign;
worth checking instruction boundaries before calling a match spurious.

## TRACE — son.snd clip selection: chatter found, dialogue path still open

Chasing how son.snd clips are chosen (to replace the invented `selector - 1` rule):

    0x00B9DE  snd_clip_player     AX = clip index; table DS:0x0BBF indexed by clip*4
    0x00B8E8  lookup              shl ax,2; add to DS:0x0BBF; read {offset,len}
    0x00B8AB  CHATTER picker      mov ax,0xA -> prng(10); re-roll while == DS:0x0C4D;
                                  then clip = value + 7

So this caller is the AMBIENT CHATTER picker — a random clip in 7..16 that never
repeats consecutively — which the port already models correctly in `main.rs`. It is
NOT the dialogue-line voice path, so the `selector - 1` question is still open.

Also settled on the way: `0x2DE2` IS the PRNG (`0x01CE:0x0B02`), not the "hash or
auxiliary PRNG" the label hedged at. Every call site passes a modulus in AX (5 at
`0x6339`, 10 at `0x8B8AB`, the VM's `0xA2` at `0x6588`), and the port already ships a
faithful `BloodPrng::next` for that exact address.

### Two mis-slicing incidents in one trace

Both times, disassembling from a byte that was not an instruction boundary produced a
confident but wrong reading:

1. Starting at `0xB8AC` gave `or al,[bx+si]`, hiding `mov ax,0xA` — which is the
   ARGUMENT to the PRNG call. Without it the call looks like it *returns* the clip
   index rather than taking a modulus, and the whole routine reads as a selector.
2. Earlier, starting at operand bytes made three real `mov word [0x1fab],0xffff`
   resets look like `stosw; pop ds` false positives.

The tell in both cases was a decode that did not fit its context. Disassemble from a
known boundary (a call target, or several candidate offsets until the stream
stabilises) before trusting a short window.

## STRUCTURAL — the audio trace ends at a driver boundary

Continuing the hunt for the dialogue-voice selection rule, two results bound how far
static tracing can go:

* The remaining `DS:0x0BBF` reference at `0x000623` is a genuine coincidence:
  `mov ax, 0xbbf; mov fs, ax` uses `0xBBF` as a SEGMENT. So the clip table has exactly
  three real users -- the chatter lookup (`0xB8F8`), the player itself (`0xBA22`), and
  the bank loader (`0xC041`). There is no separate dialogue lookup through it.
* Every play site calls `lcall [0x0CDB]`, an INDIRECT far vector. Six reads, and a byte
  search finds NO WRITER in the executable. The game ships `*MID` / `*SBP` / `*GRV`
  driver patterns at `DS:0x023E`, so the vector is filled at runtime by an external
  driver.

So the chain from clip index to audible output leaves the executable. What the port
must get right is the clip INDEX and the bank layout (`DS:0x0BBF`, `clip*4` ->
{offset,len}) -- driver internals are out of scope by construction, not by neglect.

That reframes the open question precisely: `text_selector_voice_clip_index` is still
known to be invented (b3 has exactly one reader and it forms the line id), but the
replacement rule cannot be found by following the sound module further. It has to come
from the DIALOGUE side -- find where a spoken line triggers a clip at all.

## RESOLVED — the dialogue-voice rule is a RANDOM roll, and the port already has it

The open question from the previous entries is answered. The trigger and the rule are
both in the executable, and they connect:

    0x66AF   mov byte gs:[0xCFB], 1     A6 accepts a line  -> SET the voice flag
    0xB898   test byte [0xCFB], 1       the clip picker's GATE
    0xB8AB   mov ax,0xA -> prng(10)     roll
    0xB8B3   cmp [0xC4D] / je back      re-roll until different from the last
    0xB8BC   add ax,7                   clip = roll + 7
    0x94CF   mov byte [0xCFB], 0        reveal completes -> stop

A byte search gives `DS:0xCFB` exactly ONE setter and ONE tester, so the linkage is not
inferred. The game's dialogue voice is therefore a RANDOM burble clip (7..16) played
while the line reveals, never repeating back to back — not a per-line or per-speaker
selection.

THE PORT ALREADY IMPLEMENTS THIS CORRECTLY. `main.rs` runs `chatter_prng.next(10)`,
re-draws while equal to the previous pick, plays `bank.clip(7 + pick)` on a 4-tick
throttle, and its comment already cites `0xB898`/`0xC4D`/`0xB2F`/`0x94CF`.

WHAT IS WRONG is the SECOND, parallel path: `text_selector_voice_clip_index(b3, talk_count)`
plays an additional per-speaker clip at line start. It is invented twice over — `b3` has
a single reader that forms the line id, and its bound `talk_count` is
`actor_record.talk_hnms.len()`, i.e. an audio clip index bounded by a count of talk
VIDEOS.

CORRECTION TO MY EARLIER NOTE: I previously declined to remove it on the grounds that
"deleting it silences dialogue with nothing to replace it". That was wrong — the
faithful chatter path runs independently, so the invented path is ADDITIVE. Removing it
leaves the correct voice, not silence. Recorded rather than acted on in this pass
because it touches four call sites across three modules and deserves its own change.

One thread genuinely unresolved: per-speaker banks DO exist (`sn\xxxxxxxxxxxx` template
at `DS:0x0D06`), so a bank may be chosen per speaker. But whichever bank is loaded, the
only clip SELECTION logic in the executable is the random roll above.

## DECODED — per-line asset selection is a TABLE, not arithmetic

The talk-clip question is now answered at the mechanism level:

    line_id = sign_extend(b3) + 9                       0x11F2
    entry   = DS:0x1FB5 + line_id*4                     0x9D65..0x9D6A
    asset   = word at entry+2   (0xFFFF = none)         0x9D6E, 0x9D71
    skip reload if asset == DS:0x1FA3                   0x9D76
    else cache + unpack (0x5B53/0x5B57, [0x5229], 0x1ce:0x91d)

The table is populated at runtime through a write cursor at `DS:0x1FAF`, seeded at
`0x7447` as `0x1FB5 + 0x26`. That constant pins the indexing exactly: `0x26` = 38 is
the entry for **b3 = 0** (line id 9, so `9*4 + 2`). The table is filled from the b3=0
entry upward.

CONSEQUENCE for `vm::text_selector_voice_clip_index`, which is the last thing keeping
those rows open: the game's effective per-line index is **b3 itself, 0-based**. The
port computes **b3 - 1** and additionally treats `b3 == 0` as "silent". Both differ
from the decoded mechanism.

NOT changed in this pass, deliberately. The port resolves talk-HNMs through DESCRIPT
records while the game resolves an asset id through a RUNTIME-POPULATED table; those
are different data paths, so shifting the index by one without knowing what fills
`DS:0x1FB5` would be swapping one unverified rule for another. What is now available is
the real mechanism to implement against, and the specific arithmetic to check it with.

## MECHANISM — the complete A6 `b3` chain (consolidated)

Four successive corrections were needed to get this right; consolidating it so nobody
re-derives it from scratch. Every hop is an instruction address, not an inference:

    A6 b3
      -> 0x668D  LODSB / CWDE / mov gs:[0x1FAB],ax      store sign-extended
      -> 0x11F2  mov ax,[0x1FAB]; add ax,9              LINE ID -> gs:0x6788
      -> 0x9D65  mov bx,ax; shl bx,2; add bx,0x1FB5     entry = table + line_id*4
      -> 0x9D6E  mov si,[bx+2]                          ASSET ID (0xFFFF = none)
      -> 0x9D76  cmp si,[0x1FA3]                        skip reload if unchanged
      -> 0x9D80  unpack: 0x5B53/0x5B57 gates, [0x5229], lcall 0x1ce:0x91d

The table is FILLED by the per-line record parser at `0x766F`, which fans one source
record into three places — a name to `DS:0x24C6`, the asset id through cursor
`DS:0x1FAF`, and a 26-byte record through cursor `DS:0x1FAD`. The asset id is stored as
`(byte-1)*16`, or sign-extended unchanged when negative (`0xFF` -> the `0xFFFF`
sentinel the reader tests).

Two independent corroborations that this is right rather than merely coherent:

* The cursor is seeded (`0x7447`) at `0x1FB5 + 0x26`, which is exactly `entry+2` for
  `b3 = 0` — precisely where the reader looks.
* The fill's negative passthrough produces exactly the sentinel the reader compares
  against. Neither fact was assumed from the other.

The `*16` identifies the id as a byte offset into a 16-byte-stride NAME table (the
stride of the sprite filename table at `DS:0x0669`), which connects to the 18-byte
by-name index at `0x7444`: ids become names, names resolve through that directory.

WHY THIS MATTERS for the port: `vm::text_selector_voice_clip_index` computes `b3 - 1`
as an ordinal. The game uses `b3` to INDEX A RUNTIME TABLE whose values are name-table
offsets. Those are not the same kind of thing, which is why no adjustment of the
arithmetic would have made the port faithful.

## SWEEP — DESCRIPT is parsed TWICE (duplication risk, no divergence found)

The format has two independent, both-live parsers:

* `descript::DescriptDb::parse` (lib) — used by the RUNTIME (`main.rs`, `script.rs`).
* `extract::descript::parse_descript` (binary) — used by the EXTRACTION tooling
  (`extract/mod.rs`, `extract/script.rs`). It does NOT import the lib's types; it
  declares its own `DescriptRecord`/`DescriptDb`.

That is the exact shape that produced the `parse_dictionary` divergence (one copy
fixed, three left wrong), and here it is split along a worse seam: what the game plays
comes from one parser, what we export for QA and port data comes from the other. A
divergence would show up as exported data that does not match runtime behaviour —
precisely the kind of thing a green test suite would not catch.

CHECKED, NO DEFECT FOUND. The record types differ in SHAPE — extract hoists
`backgrounds: Vec<(u8, String)>` into a field and uses `kind: u8`, while the lib keeps
`DescriptCommand::Background { slot, lbm }` in a generic `commands` stream with a
`RecordKind` enum — but both decode the same commands. Representation differs,
coverage does not.

NOT consolidated. Unlike the four identical `parse_dictionary` copies, these are
genuinely different shapes serving different consumers, so folding them is a real
design change rather than deleting duplication. Recorded as a standing risk with the
check that would settle it: parse the shipped `DESCRIPT.DES` with both and compare
record names, kinds and per-command sequences. That test can only live in the binary
crate, since it is the only one that can see both.

### Correction — the dirty-rect compositor was over-ranked

I listed porting `render_ship_3d_dirty_sprite_commands_indexed` into the frame loop as a
top-priority thread. On inspection it is low value: the engine ALREADY collects render
commands and blits from them, and the compositor adds double-buffering, remap tables and
copyback — semantics the nav markers do not use. It would move ~100 lines across a crate
boundary for no pixel change. Still worth doing eventually for the remap paths; not
worth doing ahead of work that changes behaviour.

## SWEEP — palette application sites: consistent, and they corroborate the DAC finding

Swept every `scene_palette` assignment in `engine.rs`. No defect; recording the clean
result and one genuinely useful cross-check.

The ranges differ by SITE and each difference is justified:

* HNM overlay video installs only `1..127` — "must survive the bridge background's
  palette install", i.e. the overlay must not own the scene bank.
* A full-screen LBM background (`bob_contact_bg`) installs all 256, which it legitimately
  owns.
* The two hand-mesh installs both narrow to `202..=251`, the skin ramp.

CROSS-CHECK WORTH KEEPING: those hand installs carry the comment "installing all of
128..=255 clobbered scene palettes whose images own 128..201 (the world rooms: the
cyan-cast defect found by the planet reference bank)". That is an EMPIRICAL finding,
arrived at from a rendering bug — and it lands on the same boundary as this session's
STATIC finding that DAC colours 128..191 differ from the baked image and are scene
state. Two independent lines of evidence, one from a visual defect and one from a byte
comparison, agreeing on where the scene bank starts. Noted in `palette.rs` so the APPROX
there is not read as a lone guess.

## FIXED #48 — the inline `0xA1` prefix skip was gated on mode; the binary is unconditional

`0x6C86` does `cmp al,0xA1` and `0x6C8E` does `inc si` — both BEFORE the mode test at
`0x6C9C`. The handler consumes the prefix whatever the mode. The decoder gated that skip
on `mode1` for `0xC1..0xC4`, while leaving `0xCD` ungated — internally inconsistent
before you even compare it to the binary.

Worse, it disagreed with the decoder's OWN length accounting. The `0xFD | 0xFB` arm
already did `if cod.get(pos+1) == Some(&0xA1) { l += 1 }`, unconditionally. So in mode 0
a token's `len` counted the prefix while its operand read did not skip it: the operands
came from one byte earlier than `len` claimed, and the next token would start correctly
while this one held shifted data.

MEASURED BEFORE CHANGING. Across all five `SCRIPT*.COD`, no affected opcode (`0xC1`,
`0xC2`, `0xC3`, `0xC4`, `0xCD`) is EVER followed by `0xA1` — 0 occurrences. An earlier
count of 18 was against `0xB0`/`0xCE`, where those bytes are operands, not prefixes. So
this is a latent inconsistency, not a live corruption, and the alignment is provably
behaviour-neutral on shipped scripts.

Fixed by dropping the `mode1 &&` gate at all five sites, which simultaneously matches
the binary, matches the length accounting, and makes the five sites agree with each
other. The regression test checks the operands are read PAST the prefix, that `len`
spans it, and includes a no-prefix control so the skip is driven by the byte rather than
applied blindly. Confirmed it fails on the old gated behaviour before restoring.

### #48 follow-up — verified the fix cannot desync

Dropping the `mode1` gate makes the operand read skip a byte it previously did not, so
the obvious risk is introducing the opposite inconsistency: an opcode whose `len` does
NOT account for the prefix while its operand read now skips it. That would desync the
walk.

Checked against `OPCODE_DESC` rather than assumed. The `l += 1` adjustment fires only
for `b1 ∈ {0xFD, 0xFB}`, and every opcode touched has `b1 = 0xFD`:

    0xC1 b0=0x05 b1=0xFD    0xC2 b0=0x05 b1=0xFD    0xC3 b0=0x05 b1=0xFD
    0xC4 b0=0x05 b1=0xFD    0xCD b0=0x07 b1=0xFD

So `len` and `operand_pos` now respond to the prefix for exactly the same opcode set.

Corroboration from a site I did not touch: `0xB7` (bit-flag) also has `b1 = 0xFD`, and
its decode ALREADY read `let clear = cod.get(pos+1) == Some(&0xA1)` with no mode gate.
It was consistent all along — the fix makes the record opcodes match a neighbour that
was already right, which is a stronger argument than matching the disassembly alone.

### #48 sweep closed — no other site gates the prefix skip

Checked every `Some(&0xA1)` occurrence in `vm.rs` (12 sites). After the fix, none gates
the PREFIX SKIP on mode.

The `mode1` conditions that remain (`vm.rs` ~2687-2757 and ~3069-3080) are a different
thing and are correct: they select WHICH OPCODE FORM applies in each mode
(`if !mode1 && ASSIGN_5.contains(&op)`, etc.), and inside each branch the prefix is
handled unconditionally (`let clear = cod.get(p) == Some(&0xA1); if clear { p += 1; }`).

That mirrors the handler exactly: mode selects behaviour at `0x6C9C`, while the prefix
skip at `0x6C8E` happens regardless. Distinguishing the two uses of `mode1` is the whole
point — one is dispatch, the other was a bug.

## BOUND — the dialogue asset chain's last hop is not statically reachable

Everything from `b3` to the unpack is decoded and landed as tested code. The one
remaining hop — what feeds the record parser's source stream at `ds:si` — hits a static
wall, and it is worth recording the wall rather than repeatedly probing it:

* `0x766F` (the parser) has NO near callers. It is a routine entry reached by a far call
  or through a table, and the preceding routine simply `ret`s at `0x766E`.
* `DS:0x24C6`, the name buffer the parser fills, has EXACTLY ONE reference in the whole
  image: the `mov di,0x24C6` that writes it. Nothing reads it by immediate, so its
  consumer receives the address rather than naming it.

So neither the entry nor the stream can be found by byte-searching for references. This
is the same wall the `0x0CDB` sound vector hit (six reads, no writer — filled by an
external driver) and the `gs:0x175` glyph lead hit (the address was a red herring).

THE RIGHT INSTRUMENT is the one that has worked three times this session: a WRITE WATCH
with a positive control. Arm `trace_range` over the table `DS:0x1FB5..+0x100` and drive
a dialogue load; the writer's `cs:ip` falls out immediately, and the control is already
known — the b3=0 entry must land at `0x1FB5 + 0x26`, so if the watched bytes do not show
that, the watch is mis-aimed rather than the table unwritten.

That is a scenario problem (reaching a dialogue load under the probe), not a decode
problem, which is why no further reading of the image will produce it.

## CORRECTION — the probe falsified part of the asset-table decode

Built the `DLGTABLE` probe specified in the previous entry (arm `trace_range` over
`DS:0x1FB5`, load the hub savestate, dump the live table). It contradicted my own static
reading, which is the point of building it.

CONFIRMED by the live data:

* The `+0` word steps by exactly `0x1A` (26) per entry — `0x2069, 0x207F, 0x2099,
  0x20B3, ...`. That is a POINTER into the 26-byte record array walked by the second
  cursor `DS:0x1FAD` (`add gs:[0x1FAD],0x1A` at `0x76A2`), exactly as the fill implies.
* `0xFFFF` really is the "no asset" value: lines 0-7 hold it, lines 8-23 do not.

FALSIFIED:

* I recorded that `+2` is "a BYTE OFFSET into a 16-byte-stride NAME table", from the
  fill's `dec ax; shl ax,4` at `0x768D..0x768E`. The live value is `0x0DD7`, which is
  NOT 16-aligned. `DS:0x0DD7` lies inside an `fd\xxxxxxxxxxxx` PATH TEMPLATE — the
  patchable name field, the same shape as `mu\xxxxxxxx.voc` at `DS:0x0D2C`.

So in this state `+2` holds a POINTER to a filename field, not a scaled index. Either
`0x7684` is not what populated it here, or its value is later replaced. The `*16`
reading was an inference from one arithmetic site, and I extended it into a claim about
what the field MEANS — the probe shows that extension was wrong.

What survives: the table base, the 4-byte stride, the `+2` field position, the `0xFFFF`
sentinel, and the `b3+9` indexing are all confirmed. What does not: the interpretation
of the stored value. The executable specification in `vm.rs` still encodes
`dlg_line_asset_id_from_source_byte` as `(byte-1)*16` — that function faithfully
describes the instructions at `0x7684`, so it stays, but it must NOT be read as "this is
what the table contains".

### The asset id resolved — a pointer into an `fd\` name-slot array

Following the falsified value to its target settles what `+2` actually holds.

`DS:0x0DC7` is `fd\` followed by consecutive 13-byte NAME SLOTS (12 placeholder chars +
NUL): slot 0 at `0x0DCA`, slot 1 at `0x0DD7`, stride 13. The slots still read
`xxxxxxxxxxxx` in the hub savestate — unpatched, exactly like `mu\xxxxxxxx.voc` before
`0x77A9` writes a name into it.

So the per-line asset id is a POINTER to one of those slots, and the line's asset is an
`fd` file — the location BACKGROUNDS (chart.fd, frigo.fd, orx.fd). That fits the
consumer: `0x9D10` raises the palette gates and unpacks, which is background work, and
it is why the whole chain sits on the graphics path rather than the audio one.

It also closes the arithmetic question definitively: `0x0DD7` = 3543 is not divisible by
16, so this value CANNOT have been produced by the `(byte-1)*16` at `0x7684`. That fill
is a real instruction sequence writing a real field, but it is not what populated this
field in this state. Two writers, one field — which is precisely the situation that made
the static-only reading confident and wrong.

### DLGTABLE probe — instrument validated, fill bounded to a script load

Three runs, each narrowing the question:

1. **Hub savestate, idle** — 0 writes, table already fully populated (24/24 entries).
   So the fill is not per-line.
2. **Hub savestate, 12 rounds of driven clicks** — still 0 writes. So it is not
   per-line-advance either; the table is written once and left alone for the whole
   conversation.
3. **From BOOT (`DLGTABLE_BOOT=1`), watch armed before any load** — with a wide
   `0x100` window this caught writers `0b13:029e` and `0370:048b`, but at offset 181
   (`DS:0x206A`), which is past the 4-byte table and inside the 26-byte RECORD ARRAY
   those `+0` pointers address. Narrowing to the table proper (`0x60` = 24 entries)
   gives 0 writes.

THE INSTRUMENT IS VALIDATED by that third run: the wide window demonstrably catches
writes when they occur, so the narrow window's zero is a real negative rather than a
watch pointed at the wrong address. That is the same control discipline as NAVWRITE and
PALBANK, arrived at here by accident — the too-wide range turned out to be the proof
that the probe works.

CONCLUSION: the 4-byte table is filled during a SCRIPT/SCENE LOAD that neither the boot
driving nor the in-conversation driving reaches. It is populated in the hub savestate,
so the fill happens somewhere between boot and that state.

NEXT STEP, precisely: drive a LOCATION CHANGE (nav -> visit a world) under
`DLGTABLE_BOOT`, since that loads a new script and must re-run the fill. This is
scenario construction, not decoding — exactly what the previous entry predicted.

### DLGTABLE — thread closed: blocked by the same progression wall as nav

Fourth scenario: reuse the `TRAVELPROBE` driving (Esc out of the consultation, advance,
then rotate toward the nav sector and click) so a LOCATION CHANGE loads a new script.
Still 0 writes to the table.

Four scenarios tried, all negative, with the instrument independently validated (the
over-wide window caught record-array writes, so the watch demonstrably fires):

| scenario | result |
|---|---|
| hub savestate, idle | 0 — table already populated |
| hub savestate, 12 conversation clicks | 0 — not per-line |
| from boot, before any load | 0 to the table (record-array writes caught) |
| hub savestate, travel driving | 0 — the location change does not complete |

THIS IS THE SAME WALL AS NAV, and that is the useful conclusion rather than a fifth
attempt. The `NAVENT` probe found the destination entities `0x15..0x1F` entirely
unpopulated with no writer, and `dead_ends.md` records that the console path never
grants a destination. A location change requires a granted destination; the dialogue
table's fill requires a location change. Both are gated behind progression that no
reachable savestate has.

So this is one blocker, not two: **no reachable state performs a script load**. Anything
that only happens at script load — the asset-table fill, the nav destination grant — is
unreachable together, and will stay so until a savestate taken after real progression
exists. That is a data-capture task (play the game to a granted destination and save),
not a decode or scenario-scripting task.

Recording it as a single named blocker so the next session does not rediscover it from
either end.

### Launch fix VALIDATED by capture — the driver now reaches the game

Ran the repaired `drive_real_game.sh` end to end (Xvfb + DOSBox-X + xdotool, 25s boot,
Escape, Return) and captured frames. The game reaches its title/nav screen — the
`Commander BLOOD V 1.0` banner over the BCARTE perspective-grid pyramids and the BORXX
eye-orb.

That is the confirmation the fix needed. Before it, one drive was mounted and BLOODPRG
was launched with no arguments, which loops the attract demo; the capture now shows a
real game surface. Frames kept at `accuracy/captures/drive_validation/`.

Note on a tempting misreading: the pyramids visible here are the STATIC BCARTE
perspective grid, not per-destination markers. They do NOT contradict the `NAVENT`
finding that destination entities `0x15..0x1F` are unpopulated — a granted destination
would draw an ADDITIONAL marker via the `0x9B98` projector. Worth stating because
"pyramids are on screen" looks at a glance like evidence against that finding.

The capture path is now usable for the post-progression savestate that unblocks both the
nav grant and the dialogue asset-table fill. Driving the game from this screen to a
granted destination is the remaining work.

### Save-slot discovery — `GAME1.SAV` was missing from the C: mount

Chasing the post-progression savestate turned up a second concrete gap, separate from
the launch defect.

`blood.sav` is not a save; it is the SAVE-SLOT DIRECTORY — ten 32-byte records of
`{name[16], filename[16]}`:

    slot 0: name='ab'  file='game1.sav'
    slot 1..9: unnamed, game2..game10.sav

Slot 0 is a real saved game. The 5887-byte `game1.sav` exists in `accuracy/cdrive/cblood/`
and `output/_tmp_iso/`, but was NOT in `accuracy/cblood_install/cblood/` — the directory
mounted as C:, which is where the game's `WRIC:\cblood\` write path points and therefore
where it looks for saves. So the game could LIST slot "ab" from `BLOOD.SAV` and then fail
to open it.

Copied it in (`GAME1.SAV`), which is additive — no existing file was replaced.

STATE OF THIS TASK: the driver reaches the game, and a loadable save now exists in the
right place. What remains is UI discovery — driving the title screen to the LOAD menu and
selecting slot "ab". Escape/Return from the title returns to the title, so the load entry
point has not been found yet, and each attempt costs a ~2 minute run. That is iterative
exploration rather than analysis, and worth doing with a batch of candidate input
sequences in one run rather than one guess per run.

The value already banked is that this path was IMPOSSIBLE before: the launcher looped the
attract demo and the save file was not where the game looks.

### Launch fix — re-validated against a CONTROL, and the claim held

I had called the fix validated on the strength of one capture showing a title screen with
the pyramid grid. That was weak: an attract demo would also show an animated title with
pyramids, and none of my driven inputs changed the screen state, which is exactly what an
input-ignoring attract loop looks like. So I ran the control I should have run first —
the OLD launch (one drive, no arguments) under identical conditions.

They are plainly different:

* **Broken launch** (`accuracy/captures/drive_validation/control_noargs_attract.png`) —
  a spaceship over a planet: the intro cinematic / attract sequence.
* **Fixed launch** (`.../fixed_launch_titlenav.png`) — the `Commander BLOOD V 1.0` title
  over the BCARTE pyramid grid and the BORXX orb.

So the fix does reach a further state and the original claim stands. Recording that the
doubt was checked rather than argued away — the control cost one run and converted "this
looks right" into "these are measurably different screens".

Corollary for the remaining task: the driven inputs not advancing past the title is NOT
evidence the launch is still broken. It means the title/intro sequence has its own
advance condition that Escape, Return, space, F1 and clicks at five positions do not
satisfy.

### The intro SELF-ADVANCES — stop driving input at it

A passive run (no input at all, shots at 25/65/105/145s) shows the sequence progressing
on its own:

    t025  boot / early
    t065  CHANGED
    t105  CHANGED
    t145  CHANGED — orange landscape on the viewscreen, version banner GONE

So the title/intro plays out over minutes above a PERSISTENT HUD: the BCARTE pyramid
grid and BORXX orb stay put while the upper viewscreen changes scene. That also explains
the earlier confusion — the pyramids never moving made the screen look static when it was
not.

CORRECTION TO MY OWN APPROACH: I spent three runs driving Escape/Return/space/F1 and
clicks at five positions trying to advance past the title, and concluded the sequence
"has an advance condition those inputs do not satisfy". The real answer is that it needs
no input; it needed TIME. The inputs were not failing to satisfy a condition, they were
irrelevant.

REMAINING: a longer passive run should reach the playable hub, which is the capture the
post-progression savestate needs. Wait, do not poke — and budget minutes per run, since
the tool timeout has to exceed the sequence length (one attempt already died at 2 minutes
against a 3.5 minute script).

### Capture thread — state after six minutes passive, and where it stops

A six-minute passive run (shots each minute) shows the viewscreen changing at EVERY
sample — six distinct frames, ending on a character in a panelled room. The BCARTE
pyramid HUD and BORXX orb persist throughout. No interactive console menu appears.

WHAT IS ESTABLISHED:

* The launch fix is real, confirmed against a control (no-args reaches a different
  screen entirely — a spaceship over a planet).
* `GAME1.SAV` is now present in the C: mount, where the game looks for it.
* The sequence self-advances and does not need input to progress between scenes.

WHAT IS NOT: reaching a state with granted destinations. Six minutes of scene changes
never produced a console menu, so either the intro is longer than that, or it waits for
an input whose timing/position I have not found, or this is a demo loop that the
arguments start differently but never exits.

STOPPING THIS THREAD HERE. Distinguishing those three requires knowing the game — what
to click and when — which is play-testing, not analysis, and each hypothesis costs a
multi-minute run. The infrastructure work is done and banked; the remaining step needs
someone who can drive the game deliberately, or a savestate captured by a human playing
it.

Frames under `accuracy/captures/drive_validation/` document each stage so the next
attempt starts from evidence rather than repeating these runs.

### #48 confirmed at a SECOND handler

Fix #48 (consume the inline `0xA1` prefix regardless of mode) was derived from the
`0xC4` handler at `0x6C7E`. The `0xC1` handler at `0x6B4C` shows the identical shape,
independently:

    0x6B52  xor dl,dl
    0x6B54  mov al,[si]
    0x6B56  cmp al,0xA1
    0x6B58  jne
    0x6B5A    inc dl        (the inverted flag)
    0x6B5C    inc si        (SKIP the byte)
    0x6B5D  lodsw           (operand read, past the prefix)
    ...
    0x6B73  test gs:[0x67AD],1   <-- the MODE test, AFTER the skip

Same ordering: flag, skip, read operands, and only then consult the mode. So the fix
rests on two handlers agreeing rather than on one reading, which matters because the
change touched five decode sites on the strength of the first.

Also visible here and NOT yet modelled: after `0x6B60` calls `0x6034`
(`vm_record_lookup_by_threshold`), the handler reads the record type from `es:[di]` —
the LOOKUP RESULT — while separately taking `cx = es:[bp]`, the raw operand's own type.
The port's `record_state_condition` reads its record words directly from
`record_offset`. Whether those coincide depends on what `0x6034` returns for this input,
which is not established here. Left as a specific open question rather than assumed
equivalent.

### Post-update `0xC6` branch — a scripted line trigger

While tracing the post-update ladder for `post_update_execution_state`, decoded the
`0xC6` branch at `0x5E22`. It is a small phase machine over `gs:0x2792` / `gs:0x2A7B` /
`gs:0x278B`, and its second phase does something worth naming:

    0x5E74  mov word gs:[0x6788], 0x2C

That writes the ACTIVE LINE ID directly — the same variable the `0xA6` handler forms as
`sign_extend(b3) + 9` and that `0x9D10` dispatches to scene/asset work. So a post-update
step can TRIGGER a specific line without an `0xA6` token being executed, by setting the
id the dispatcher reads.

That matters for the port's model: the line id is not only an A6-derived value. Anything
reproducing `gs:0x6788` faithfully has to account for native writers too, and this is
one.

Also corroborated here: `gs:0x1FB2`, which `vm.rs` models as `C2_PRESENTATION_GATE`, is
tested at `0x5E7E` in this same ladder — the third independent site for that gate after
`0x11FD` and `0x9D26`.

## SCOPE CORRECTION — the line id has 29 writers, only ONE from `b3`

Enumerated every write to `gs:0x6788` (the active line id) by byte-searching all the
`mov [0x6788], …` encodings. There are **29**:

* `0x11F8` — `mov [0x6788], ax` after `[0x1FAB] + 9`. **The only `b3`-derived writer.**
* 4 register writes (`0x1209`, `0x1242`, `0x1ECF`, `0xB00F`).
* 24 IMMEDIATE writes of specific ids: a contiguous cluster `0x27, 0x28, 0x29, 0x2A,
  0x2B, 0x2C` (repeated across `0x1887`–`0x1A42`, `0x5C99`–`0x5FC1`, `0x68xx`–`0x6Exx`,
  `0xB0xx`), the low ids `0x01, 0x02, 0x03, 0x06, 0x07`, and `0xFFFF` resets.

So the line id is PREDOMINANTLY set by native code triggering hardcoded lines — the
`0x27..0x2C` cluster looks like a block of system/UI lines — and only once from the
script's `0xA6` selector.

WHY THIS MATTERS, and it corrects my own framing from earlier today. I traced `b3` →
`+9` → line id → `0x9D10` → asset table and described that as "the chain", then said
`b3` "has exactly one reader". The reader claim is still true (`DS:0x1FAB` is read once).
But I let it imply the line id is a function of `b3`, and it is not: `b3` accounts for
1 of 29 writers. A port that models only the `b3` path reproduces one twenty-ninth of
what sets this variable.

The executable specification in `vm.rs` (`dlg_line_id_for_selector` etc.) is still
correct for the path it describes. It just describes far less of the mechanism than the
name suggests, and that is now recorded at the function.

## COVERAGE GAP — the port models 8 of the 12 field selectors the game uses

Enumerated every `mov ax,imm; call 0x6023` (`vm_field_offset`) site: **49 sites across
12 distinct selectors**.

    game uses: 0x02 0x05 0x08 0x09 0x0A 0x0B 0x0C 0x0D 0x0E 0x0F 0x11 0x13
    port has:  0x02      —    0x09 0x0A 0x0B 0x0C  —   0x0E  —   0x11 0x13

Unmodelled: **0x05, 0x08, 0x0D, 0x0F** — five sites:

    0x0F  @0x5720
    0x08  @0x5DC4, 0x5DEC
    0x0D  @0x5ED9
    0x05  @0x60A8

THEY CLUSTER, and where they cluster is the point. `0x5DC4`/`0x5DEC` are inside
`vm_post_update_c4_pair` (`0x5D8F`) — the routine whose `+4` processed-marker and
`0x67B6` gate I verified earlier today — and `0x5ED9` is in the `0xC6` branch at
`0x5E22`. That is exactly the region I described as untraced when I declined to mark
`post_update_execution_state`.

So the decision to leave those rows open was right for a better reason than I had at the
time: it is not merely that I had not read the whole function, it is that the function
performs field operations through selectors the port has no representation for. At
`0x5DC4` the handler resolves selector 8 and INCREMENTS the word at that field
(`inc word [eax+edi]`), which is state the port does not maintain.

Most-used selectors for context: `0x11` (12 sites), `0x0B` (9), `0x13` (8), `0x0C` (5).
The four gaps are the rare ones, which is consistent with them having been missed rather
than dismissed.

### The four missing selectors, specified

Decoded each unmodelled selector so the gap is a spec rather than a hole. All four are
defined for a SINGLE kind, which is why they are easy to miss:

| sel | kind | offset | use |
|---|---|---|---|
| `0x05` | 1 | `0x1E` | base of a TEN-WORD ARRAY, iterated (`mov cx,0xA; lodsw…`) at `0x60A8` |
| `0x08` | 1 | `0x36` | INCREMENTED COUNTER (`inc word [eax+reg]`) at `0x5DC4`/`0x5DEC`, then `or word [si+2],0x8000` |
| `0x0D` | 8 | `0x16` | read (`mov dx,[eax+edi]`) at `0x5ED9`, immediately paired with selector `0x0A` |
| `0x0F` | 1 | `0x46` | read + zero test gating the following branch, at `0x5720` |

The `0x08` one is the most consequential: the post-update ladder INCREMENTS a per-record
counter at offset `0x36` and flags bit15 of the record's `+2`. `post_update_execution_state`
maintains neither, so any behaviour that depends on how many times a record has been
post-updated is currently unmodelled.

Note they are all single-kind (three for kind 1, one for kind 8). A selector defined for
one kind looks like a rare special case in the matrix and is exactly the sort of thing a
port written from the common paths would omit.

## FIELD SPACE — full matrix audit by kind (54 defined pairs)

Audited `gs:0x6D60` column-by-column instead of by code path, since the four missed
selectors were all single-kind. The complete field space:

    kind  0:  6 fields   0x00->0x02 0x01->0x04 0x0E->0x20 0x11->0x06 0x13->0x08 0x14->0x10
    kind  1: 13 fields   0x00->0x02 0x01->0x16 0x02->0x1A 0x03->0x32 0x04->0x34 0x05->0x1E
                         0x07->0x38 0x08->0x36 0x0E->0x44 0x0F->0x46 0x10->0x14 0x11->0x18 0x13->0x3A
    kind  2:  6   kind 3: 5   kind 4: 6   kind 5: 1   kind 6: 1
    kind  7:  3   kind 8: 5   kind 9: 5   kind 10: 3

54 defined `(selector, kind)` pairs over kinds 0..10.

TWO RESULTS WORTH KEEPING:

1. **`selector 0x00 -> offset 0x02` for EVERY kind.** That is the flags/active word. So
   the port's `state_u8(owner + 2) & 1` active test is not a kind-1 assumption at all —
   it is invariant across the entire matrix. That distinguishes it sharply from
   `LOCATION_FIELD`, which genuinely varies by kind (`0x06/0x18/0x16/0x14/0x04`) and
   where the hardcode IS an assumption. Two superficially identical hardcoded offsets,
   one safe by the data and one safe only by context.

2. **The matrix defines 21 selector slots; the code calls only 12 by immediate.**
   Selectors `0x00, 0x01, 0x03, 0x04, 0x06, 0x07, 0x10, 0x12, 0x14` never appear as
   `mov ax,imm; call 0x6023`. They are reached with a register-loaded selector, or their
   fields are addressed directly by offset. Enumerating the immediate call sites
   therefore UNDERSTATES the field space — which is worth knowing before treating "12
   selectors" as the complete set.

Kind 1 is the object kind and carries 13 of the 54 fields; kinds 5 and 6 carry only the
universal flags word.

### `vm_field_offset` decoded — the kind is a BITMASK (`bsf`)

`0x6023` is `SHL AX,4` / **`BSF BX,BX`** / `ADD BX,AX` / `MOV AL,gs:[bx+0x6D60]`. Bit
Scan Forward, so the kind is a BITMASK and the matrix column is the index of its lowest
set bit — column *k* is kind value 2^k, not kind *k*. `vm.rs` models this exactly with
`kind.trailing_zeros()`, and its `kind == 0 -> None` guard matches BSF leaving the
destination undefined on a zero source.

That corrects how my own field-space audit should be read: what I tabulated as
"kind 8" is COLUMN 8, i.e. kind value `0x100` — which is why `SHIP_3D_OBJECT_KIND_POSITION_KIND100 = 256`
lands there. It also dissolves a concern I raised one step earlier: `vm_field_offset(0x13, 0x10)`
does not overflow into the next selector's row, because `0x10` means bit 4.

OPEN LEAD, recorded rather than claimed. In column 8 the selectors resolve to
`9->0x18, 10->0x1C, 12->0x14, 13->0x16`, and **`14 -> 0x00`** (undefined). The port's
`SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD` is 14, while selector 13 is the one that
has a real offset there — and 13 is exactly what `0x5ED9` uses. But `kind100_relation_word`
handles the zero case DELIBERATELY (`0 => Some(record.kind_flags)`), so it is not
accidentally reading a missing field. Whether that branch matches the binary is
unverified; the check is to locate the site `kind100_relation_word` models and see
whether it resolves selector 13 or 14.

### Lead CLOSED — selector 14 is resolved against the OTHER record's kind

Chased the `KIND100_RELATION_WORD = 14` lead to its site and it is **no defect**. At
`0x60E3`:

    mov ax,[si]      this record's kind
    cmp ax,0x100     is it kind100?
    jne …
    mov bx,[di]      the OTHER record's kind      <-- the kind passed to the resolver
    mov ax,0xE       selector 14
    call 0x6023

So selector 14 is resolved against the OTHER record's kind, not against `0x100`. The
port matches exactly: `kind100_relation_word(other_record)` calls
`vm_field_offset(14, other_record.kind_flags)`.

My concern came from looking up selector 14 in COLUMN 8 (kind `0x100`) because the
enclosing branch tests for kind100 — but the kind that reaches `bsf` is a different
record's. The general lesson, now that the resolver is decoded: **the kind argument is
not always the kind of the record being examined**, so reading a matrix cell requires
knowing which record supplies the kind, not just which branch you are in.

That is three leads this session that looked like defects and were not (`0x6034` under a
different name, the `write_record_entry` family, this one), against several that were
real. Recording the closures as carefully as the hits — a lead list that only accumulates
is worse than useless.

## MECHANISM — the selector-8 counter gates list membership

Traced the unmodelled selector `0x08` end to end. It is not bookkeeping; it decides what
the player sees.

WRITE (post-update ladder, `0x5DB0..0x5DFA`), symmetric over the C4 pair:

    di = ds:[bp+2]                       the related record
    if [si] kind == 1:  bx = [di];  sel 8 -> off;  inc word [eax+edi]   (the OTHER's counter)
    else if [di] kind == 1: bx = [si];  sel 8 -> off;  inc word [eax+esi]
    both branches: or word [si+2], 0x8000

Whichever partner is kind 1, the OTHER partner's counter is incremented. For kind 1 the
selector-8 offset is `0x36`.

READ — two sites, both gating on NON-ZERO:

    0x83DF   cmp word [si+0x36],0 / je skip   then cmp [si+0x18],bx / je skip
             (+0x18 is the selector-0x11 LOCATION field for kind 1)
    0x91DB   cmp word [si+0x36],0 / je next   else add si,4; lcall 0x299:0x202 (draw);
             add dx,0xA  -- one list ROW per included object

So the counter is a "has this object been encountered" flag, and LIST MEMBERSHIP depends
on it. The port models neither the increment nor the filter, so any list it builds from
objects is unfiltered where the game filters.

That upgrades the selector-`0x08` gap from "a field we do not track" to a behavioural
divergence with a visible surface. Implementing it needs both halves — the post-update
increment AND the list filter — since adding only the increment writes state nothing
reads, and adding only the filter would exclude everything.

### The list filter is a THREE-condition chain, and the port has the builder not the filter

`0x91C3` walks the source list at `gs:0x6886` — the same list `0x624B` builds and the
port mirrors as `VmMachine::build_nav_source_list` — and applies three tests before
drawing each entry:

    test word [si],   2     the object's kind must have bit 1
    test word [si+2], 1     the ACTIVE bit
    cmp  word [si+0x36], 0  the selector-8 SEEN COUNTER, must be non-zero

Only survivors are drawn (`lcall 0x299:0x202`, `add dx,0xA` per row).

So the earlier framing needs refining: the port is not missing "a filter on lists". It has
the BUILDER and the builder is faithful. What is missing is the draw-time chain, and
specifically its third condition — the counter — which no port code maintains or tests.

That also locates the fix precisely. Both halves belong on the CONSUMER side of the list
plus the post-update ladder:

1. post-update (`0x5DB0..0x5DFA`) increments the partner's counter,
2. the consumer applies all three tests before including an entry.

Implementing the filter without the increment would empty the list, which is why these
land together or not at all. Recorded here rather than attempted at the end of a long
session, because it changes what surfaces show and wants its own verification pass.

## LAYOUT IDENTITIES — every verified table closes exactly on its neighbour

Applied the `base + size == next table` check to every table verified in this campaign.
All seven checkable identities are EXACT:

    nav points   0x4F09 + 10*6   = 0x4F45  == angle table
    sqcaps xlat  0x7362 + 176    = 0x7412  == sqcaps widths
    sqcaps width 0x7412 + 48     = 0x7442  == sqcaps glyphs
    sqcaps glyph 0x7442 + 48*20  = 0x7802  == BUILT-IN font xlat
    builtin xlat 0x7802 + 176    = 0x78B2  == builtin widths
    builtin widt 0x78B2 + 86     = 0x7908  == builtin glyphs
    point cloud  0x2FC1 + 1000*8 = 0x4F01  == projector scratch

WHY THIS MATTERS: it independently confirms three of today's contested corrections, each
of which was originally established by a DIFFERENT method:

* The nav table is **10** records, not the 11 our label claimed — because `10*6` lands
  exactly on the angle table. (Originally found by dumping the image and live memory.)
* The square-caps glyph stride is **20**, not 16 — because `48*20` lands exactly on the
  built-in font's xlat. (Originally found by trial after a 1-of-25 match failure.)
* The built-in xlat is **176** entries, not 128 — because 176 lands exactly on its widths
  table. (Originally found by noticing where the advance table starts.)

Three findings, three methods, and now one arithmetic cross-check agreeing with all of
them. A wrong stride or count would break the chain visibly, which is exactly what my
16-byte guess did before I corrected it.

WORTH ADOPTING: for any newly-decoded table, compute `base + count*stride` and check it
against the next known address before trusting the stride. It is nearly free and it
catches the error class that cost the most time today.

### Layout identities keep paying — three more, and a new VM fact

Extending the `base + size == next` check into the VM's data region:

    entity table  0x6212 + 32*32 = 0x6612  == dirty-rect list   -> exactly 32 entities
    field matrix  0x6D60 + 336   = 0x6EB0  == handler table     -> exactly 0x15 selectors
    handler table 0x6EB0 + 104   = 0x6F18  == OPCODE_DESC       -> exactly 52 entries

The third is a new fact about the VM. The handler table is indexed by
`(opcode-0xA0)*2`, so 52 entries covers opcodes `0xA0..0xD3` — and the `0xD3` entry is
NULL, making `0xD2` the highest dispatched opcode (its handler offset `0x1118`
corresponds to `0x64B8`, the script-profile request, which is indeed the highest
handler in our labels).

So: **the VM executes only `0xA0..0xD2`, while `OPCODE_DESC` describes 96 opcodes
`0xA0..0xFF`.** The extra 45 entries exist purely so the token walker can compute
LENGTHS for non-executable data tokens. Those two tables have different jobs and
different extents, which is easy to conflate since they are adjacent and both indexed
from `0xA0`.

The entity identity is a nice bound too: 32 entities total is why the nav projector
iterates the LAST ELEVEN (`0x15..0x1F`) rather than an arbitrary window.

## CONFLICT — two verified constants read the SAME 192 bytes

The layout-gap map turned this up and it needs resolving by someone who can test it:

    palette buffer   DS:0x5B58  file 0x12F78  768 bytes -> ends 0x5E58
    pyramid vertices DS:0x5D98  file 0x131B8  192 bytes -> ends 0x5E58

`0x12F78 + 576 = 0x131B8`, so **palette colours 192..255 occupy exactly the bytes
`SHIP_3D_HUD_PYRAMID_VERTICES` reads.** Verified numerically: the port's
`GAME_SCREEN_PALETTE_DAC[576..768]` is byte-identical to the image at the vertex base,
and those same bytes unpack as the port's 96 vertex `i16`s.

BOTH CONSTANTS "VERIFY" against the image, because both faithfully copy the same bytes.
That is exactly why a byte-for-byte check cannot settle this — it confirms the copy, not
the interpretation. I marked both as verified earlier in this campaign on that basis.

WHY IT IS NOT OBVIOUS WHICH IS WRONG:

* As COLOURS they are coherent. All 192 bytes are ≤63 (valid 6-bit DAC), and colours
  240..249 read as an ascending teal ramp — `(17,30,37) (19,32,39) (23,31,36) …` —
  precisely the "teal DAC ramp 240..249" that `manu3_hand` independently documents for
  the hand's flat shading.
* As VERTICES they are also coherent: 32 `(x,y,z)` triples forming a plausible mesh, and
  `re/tools/dump_dosbox_mem.py` uses this byte pattern as its DS ANCHOR to locate the
  data segment in live memory — which works.

RESOLVING IT needs a runtime observation, not more static reading: dump `gs:0x5B58+576`
during a frame where the hand renders with its teal shading, and see whether the DAC
range in use matches these bytes. If it does, the vertex table's base address is wrong.
If the live bytes differ, the palette default is picking up vertex data the game
overwrites before upload.

RESOLVED BY PROBE — the alias is REAL and neither constant is wrong.

Dumped `gs:0x5B58+576` live: **192/192 bytes match the image**, so the game does not
overwrite that range. And the two readings are simply different groupings of one stream:

    bytes:  00 00 00 | 09 03 0C | 08 03 0B ...
    as i16: 0, 2304, 3075, …          -> the port's vertices
    as RGB: (0,0,0), (9,3,12), …      -> the port's colours

Grouped by 2 they are the vertex table; grouped by 3 they are palette colours. The game
uses BOTH: `0x2F90` uploads all 768 bytes to the DAC (`cx=0x300`), and the HUD projector
reads `DS:0x5D98` as vertices. So the original deliberately overlays a vertex table on
the palette's upper range, and both port constants are correct.

WHAT I ALMOST GOT WRONG: while checking, I hand-grouped the bytes as `00 00 | 09 03 |
0C 08` and concluded the port's vertices did not match the file — because I split the
stream on the wrong boundary. Two automated comparisons had already said they matched.
The lesson is the mirror of the earlier stride mistakes: when a hand-check contradicts a
mechanical one, suspect the hand-check first.

## FIX #49 — the selector-8 ENCOUNTER COUNTER, both halves, plus the panel it feeds

The previous session left this as the one item explicitly deferred: "implementing
it needs both halves — the post-update increment AND the list filter — since
adding only the increment writes state nothing reads, and adding only the filter
would exclude everything." Both are in now, and so is the consumer they feed.

FIRST, A CORRECTION TO THE EARLIER DECODE. The notes called selector 8 a KIND-1
field. It is not. The matrix row is

    FIELD_OFFSETS[8] = 00 36 00 00 ...

and `vm_field_offset` (`0x6023`) resolves the column with **BSF**, so column *k*
is the kind whose LOWEST SET BIT is *k* — column 1 is kind VALUE 2, not kind 1.
Reading the column index as a kind value is what produced the error. Both readers
settle it independently: `0x83D4` gates on `cmp word [si],2` and `0x91CE` on
`test word [si],2`. So the counter is a **kind-2 field at +0x36**, and the same
correction applies to the LOCATION field the roster reads (`+0x18` =
`FIELD_OFFSETS[0x11][1]`, also kind 2).

That also explains what the ladder is doing. `0x5DB0..0x5E06` is symmetric over
the pair: whichever partner is kind 1, the OTHER partner's counter is bumped,
resolved against THAT partner's kind. Since only kind 2 has the field, the
mechanism is "a kind-2 object was paired with a kind-1 object" — an ENCOUNTER.
Ported as `post_update_encounter_counter`, called from
`post_update_actor_record_pair` between the processed-marker write (`0x5DAC`) and
the `0xC4` write (`0x5E09`), exactly where the real ladder runs it.

THE FILTERS (`source_list_display_rows`, `source_list_text_rows`). Both consumers
walk the SAME list the port already built faithfully; the filtering lives at the
consumer, once per row. The drawn panel applies three tests (kind bit 1, ACTIVE
bit, counter non-zero); the text roster applies a fourth — `cmp [si+0x18],bx`
with `bx = gs:0x6758` = the built-in object `Ark` — which DROPS objects aboard
your own ship. So the roster answers "who else is here", not "who is here".

THE SURFACE. Tracing the text consumer's caller turned it into a screen rather
than an abstraction: entry `0x82E8` gates on three flags and then HIT-TESTS the
mouse against the widget rect at `DS:0x65F2+8`. It is the nav chart's HOVER
PANEL, composing a CR-separated block at `0xE18`:

    <PLANET: |SHIP: |BLACK HOLE: ><location name>
    LIFE SUPPORT:
    <roster, one name per line>

Ported whole as `VmMachine::location_status_block`. The four UI strings are the
game's own bytes, pinned to the image at `DS:0x12E/0x137/0x13E/0x14B` by
`the_status_headers_are_the_games_own_strings` — the same standard as
`OPTION_BOX_LABEL`, not transcriptions.

TWO THINGS FOUND ON THE WAY:

1. **An object's name is stored INLINE at record+4.** Both consumers copy from
   `si+4`, which only makes sense if `+4` is text. Checked against shipped data
   (`re/tools/check_object_inline_names.py`): 630 of 640 kind-1 objects across
   SCRIPT1..5 hold exactly their DEB name there. The ten exceptions are `blood`
   and `orxx` in each script — the two built-ins whose record bytes the engine
   reuses for other purposes (`orxx+0xA` is the C1 presentation slot). The port
   no longer needs the DEB symbol table to label a row: `object_inline_name`
   reads what the game reads.

2. **The composer's `UNKNOWN` fallback is DEAD CODE.** `0x83FD` is an
   unconditional `jmp 0x840E` over it; a whole-image relative-branch scan
   (`re/tools/find_jump_target.py 0x83FF`, new tool) finds ZERO branches to
   `0x83FF`, and `mov si,0x16C` occurs exactly once — inside the block itself.
   The shipped game never prints it. Recorded so nobody ports a string the
   original cannot display.

WHAT IS STILL OPEN. `location_status_block` is a faithful VM-side routine with
regression tests, but the port's nav view does not yet DRAW it (the port's
`render_ship_view` has its own destination list). Wiring the panel into the nav
frame — including its `DS:0x65F2` hover rect — is the next step, and it is a
frontend task now, not an RE one.

## FIX #50 — the destination info panel: the second consumer, decoded end to end

The roster has two consumers. #49 ported the text-buffer one; this is the DRAWN
one (`0x9137..0x91EC`), and tracing its caller turned it into a complete screen.

THE SELECTION COMMIT (`0x8FF4..0x905B`) picks the object, and it explicitly
refuses the one you are standing on:

    0x9016  bx = [0x6752] (arche) + 0x16      the CURRENT LOCATION
    0x901D  cmp ax,es:[bx] / je               same place -> no panel
    0x9022  [0x27BF] = ax                     otherwise, THIS is the panel's object

so the panel always describes somewhere else. It then seeds a 4x4 rect at the
cursor (`[0x2AAB] = {mouse x, mouse y, 4, 4}`), sets 8 interpolation steps,
`[0x2788]=1`, `[0x2789]=0`, and DISABLES the mouse (`[0xA3E]=0`).

THE FSM is three states on `[0x2788]`: 1 = zoom open (interpolate the cursor rect
toward the panel rect each frame, `CF` set when done -> state 0), 0 = open and
drawn (re-enabling the mouse starts the close), 2 = zoom shut, ending with
`[0x27BF]=0`. The interpolator is `0x8B:0xFAD` — already decoded and ported as
`step_ship_3d_interpolation_gate` — and it DRAWS the interpolated rect rather
than storing it, which is why nothing ever writes the rect back.

THE GEOMETRY is all static, and I checked that rather than assuming it: a
byte-search for every store form to `0x2780` finds NO writer anywhere in the
image — the only references are the two `mov si/di,0x2780` in this routine and
the draw. So the panel rect is a constant, `(100, 20, 160, 70)`, and the text
layout sits inside it exactly: header at `(110, 25)`, `LIFE SUPPORT:` at
`(110, 35)`, roster rows from `(110, 45)` at pitch 10 — five rows before the box
ends at y=90. Header/name in colour `0xEE`, rows in `0xFE`.

A DETAIL THAT WOULD HAVE BEEN WRONG BY GUESSING. The name is placed at
`x + [0x27CD] + 6`, where `0x27CD` is "the width of the string just drawn". It is
NOT the pen distance: the draw entry `0x3192` zeroes it, and only the glyph path
adds to it (`0x3215`). The SPACE path does `add di,6 / jmp` — pen moves,
accumulator untouched — and unmapped bytes are skipped entirely. Since every
header ends in a space ("PLANET: "), summing advances the obvious way would put
the name 6px too far right on every panel. Ported as
`font::game_font_drawn_width` with a test that pins the difference to exactly one
space advance.

PORTED: `VmMachine::location_panel_rows` returns the positioned, coloured rows
from record state alone. NOT ported: the window CHROME — the blit at
`0x299:0x40E` takes its source from `[0xAC8] = 0x5F11`, a handle whose resolution
is undecoded. That is the next piece, and it is recorded at `0x339E` rather than
guessed at.

## FIX #51 — the audit ledger destroyed its own curation on every regeneration

`CLAUDE.md` says to regenerate `docs/function-audit.tsv` after each verification
pass. Doing that threw the pass away.

`tools/audit_inventory.py` assigns each row a status from doc-comment
HEURISTICS — it can only ever produce `ASM?`, `ORACLE?`, `DATA?`, `INFRA?` or
`UNVERIFIED`. The settled statuses (`ASM`, `ORACLE`, `DATA`, `INFRA`) are what a
human writes after checking a row against the disassembly. The script wrote a
fresh file every run, so those 263 hand-verified rows reverted to guesses. I hit
this by regenerating and watching `ASM 103 -> 0`, `ORACLE 126 -> 0`.

The fix reads the previous ledger first and carries settled statuses forward,
with three levels of identity because `(item, file)` is NOT a key:

1. unique name in the file, both before and after -> carry it;
2. name occurs more than once but the CITED ADDRESSES differ, and the origin is
   unique on both sides -> carry by origin (this is what separates the token
   walker `walk` from a nested helper);
3. still indistinguishable (two `default` trait impls) but the file has the SAME
   NUMBER of them -> carry BY POSITION.

Anything that survives all three is DROPPED and printed, because crediting the
wrong function is worse than losing a row. The run now reports every recovery
path and every drop.

Verified: regenerating from the committed ledger reproduces its exact settled
counts (ASM 103, ORACLE 126, DATA 10, INFRA 24) and a second run is idempotent.
One row is genuinely unresolvable — a second `owner_object_offset` in `src/vm.rs`
that the old ledger never had a settled row for — and it is reported every run
rather than silently guessed.

Also renamed the nested `walk` inside `build_nav_source_list` to
`walk_selector11_children`: it collided with the token walker in the same file,
and level 2 could not separate them because neither carries an origin. Making the
name unique is a better fix than teaching the tool to guess.

## FIX #52 — the three open items on the info-panel thread, closed

The previous entry left three named gaps. All three are done.

**(1) The zoom FSM is ported.** `gs:0x2788` is a bitfield, not an enum — the
dispatcher at `0x9083` tests bit0 for zoom-open and `0x9125` tests bit1 for
zoom-shut, with zero meaning "open and drawn". Ported as `LocationPanelState`
plus `open_/close_/step_location_info_panel`, driving the game's own
interpolation gate over the four rect words.

One thing worth pinning: the zoom NEVER LANDS on the panel rect. The gate
computes `dest + (src-dest)/total*step` with a truncating `idiv bl` (`0x1E74`),
so from a 4x4 cursor rect at (40,50) the eighth and final step draws
`(96,26,156,68)` against a target of `(100,20,160,70)` — short by the remainder,
after which the drawn panel takes over. I asserted the intuitive answer first and
the test caught it; the real numbers are now what the test encodes.

**(2) Nav slots carry their entity ids.** The projector writes
`0x6212 + ((i + 0x15) << 5)` (`0x9B98`), so slot `i` IS entity `0x15 + i` — and
slot 10 is entity `0x1F`, whose record starts at `DS:0x65F2`, which is exactly
the address the hover gate reads. That identity was the whole blocker: the port's
slots were anonymous, so nothing could answer "is the mouse over entity 0x1F".
`Ship3dObjectSpriteDescriptor::entity_id` closes it and
`nav_hover_status_active` implements the gate's own inclusive box test over the
same `+0x08/+0x0A` position and `+0x0C/+0x0E` extent the sprite-slot setters
write.

**(3) The selection is decoded and ported** — `nav_chart_pick` (`0x92A3`), the
chart's object hit-test, with per-kind boxes and the black hole's TWO chart
positions (it shows the second when `obj+0x14` differs from `arche+0x22`: the two
ends of one wormhole). `open_location_info_panel` enforces `0x901D`'s refusal to
open on the object you are already at.

**A table found on the way, and worth more than the panel.** The panel's first
zoom frame resolves the selected object's ARTWORK by walking 22-byte records at
`DS:0x2BC7`, comparing display names and loading `[si+0x10] | 0x8000`. That is a
42-entry NAME -> RESOURCE table covering every world in the game, and it ends at
`DS:0x2F63`, two bytes before the nav camera origin — the layout identity that
bounds it. Ported as `WORLD_ART_DIRECTORY` with `parse_world_art_table` reading
it straight from the image, and a test that pins all 42 rows byte-for-byte AND
checks every id resolves to a real filename record.

The table is also proof that this could not have been guessed: `Oddland` is
`trou.ext`, `Bonus` is `forest.ext`, `Troma` is `glacia.ext`, and `Trashlando`
shares `kortex.ext` with `Kortex`. Any mapping inferred from asset names would
have been wrong for a quarter of the worlds.

## FIX #53 — the nav chart's object list, and what the shipped data says through it

The panel needed a SELECTION; the selection needs a LIST. Both are decoded now,
so the whole chart chain runs on records instead of script-derived labels:

    directory gs:0x672C
      -> 0x604E   keep kind-1 entries whose object has flag bit1 (IN PLAY)
      -> 0x721A   keep kinds with `test bx,0x118` = 0x08 | 0x10 | 0x100
      -> 0x92A3   hit-test each marker at +0x18/+0x1A, box sized by kind
      -> 0x9022   the selection, which opens the info panel

`0x118` is exactly the three kinds the picker sizes boxes for, which is a nice
cross-check: the filter and the hit-test agree on what a chart object is.

RUNNING IT ON THE SHIPPED DATA, which is the part worth recording. SCRIPT5's
initial `.VAR` charts exactly ONE object: `Oddland`, kind `0x100` — a BLACK HOLE
— at marker `(132, 34)`, whose artwork resolves through `DS:0x2BC7` to id 72 =
`trou.ext`. "Trou" is French for hole. The kind, the name and the asset all agree
on the same fact from three independent tables, which is about as good as
static verification gets.

SCRIPT1..4 chart NOTHING from their initial `.VAR`, and that is the DATA's answer
rather than a hole in the port: the in-play bit `0x604E` gates on is state the
story sets as you play. The test asserts both halves — one object for SCRIPT5,
none for the others — so a future change that quietly starts charting everything
will fail.

## FIX #54 — the panel is IN THE GAME: the nav view is now record-driven

The last entry ended with the panel computed but not shown, blocked on the port's
nav destinations being `(label, dialogue-lines)` pairs from scripts with no
object offset to hand it. That model is replaced.

`NavChartObject` carries what the game's chart entry actually is — the record
offset (what `gs:0x27BF` holds once selected), the inline name, the kind, the
marker and the artwork id — built from `build_nav_chart_list`. The nav view draws
each name AT ITS OWN MARKER, which is where the picker hit-tests it, so what you
see and what you can click are the same thing by construction. `nav_chart_marker`
lives in the VM precisely so the drawn position and the clickable position cannot
drift: one function, applying the black-hole two-endpoint rule once.

The click path now follows the game's: `nav_chart_click` hit-tests
(`0x92A3`), opens the panel on a hit (`0x9022` -> the `0x2788` FSM), and treats a
click while the panel is up as the mouse coming back on — the `0x912E` edge that
closes it. `render_nav_info_panel_frame` is the dispatcher: zooming states draw
the interpolated rect, `Open` draws the panel. `main.rs` refreshes the chart each
nav frame, because the in-play bit `0x604E` filters on is STORY state.

DRIVEN END TO END on the real chart (`examples/navpanel.rs`, SCRIPT5 + CHART.FD):
one chart object (`Oddland`, kind `0x100`, marker `(132,34)`, art 72), a click at
the marker opens the panel, the zoom runs, and the open frame carries
`BLACK HOLE: ` at `(110,25)`, `Oddland` at `(200,25)` and `LIFE SUPPORT:` at
`(110,35)`. The name's x falls out of the drawn-width rule, and `200 + 7 glyphs`
lands just inside the box's right edge at 260 — a layout that would not close if
the width rule were wrong.

ONE PROPERTY WORTH ASSERTING, because it would regress silently: the window is
TRANSLUCENT. Measured on the real chart, 25 distinct colours survive inside the
panel against 37 outside — the nebula still shows through, darkened. About a
quarter of the pixels bottom out at black because the chart's palette holds
nothing darker, which is the algorithm's real output rather than a flattened box.
The test now fails if the rect ever collapses to a solid fill.

The old script-derived destination list is kept as the stand-in for the case the
chart list is empty — which, per FIX #53, is the shipped answer for SCRIPT1..4 at
boot.

## FIX #55 — a verification pass over the ledger, and what checking it properly turned up

Working the ledger rather than the game this round. Four findings, in the order
they mattered.

**The recomp oracle really does cover all 75 auto-lifted functions — but I nearly
reported that it didn't.** My first coverage scan looked for `{name}.json` and
`{name}_det.json` and found 43 functions with no vectors, which would have meant
`auto_lifted_batch_matches_oracle` verifying 32 of 75 while claiming all of them.
The generic verifier reads `{name}_generic.json`. With the right suffix: 75/75,
every one with non-empty vectors. Same lesson as the palette bytes — when a hand
check contradicts a mechanical one, suspect the hand check.

**The hole was real anyway, just latent.** Every vector loader did
`Err(_) => return` on a missing file, so deleting or renaming one would drop that
function from verification while the test still passed. Both batches now assert
their coverage (all-or-nothing: a checkout without vectors skips the batch, a
batch missing SOME is a failure), and the nine per-function tests panic outright.
Positive control: moving `func_6023_generic.json` aside now fails with
`1/52 auto-lifts have NO oracle vectors and were silently skipped`, where before
it passed green.

With that enforced, the 75 auto-lifted plus 9 hand-lifted functions are honestly
ORACLE: bit-exact registers, memory writes and (for the generic batch) all six
flags against Unicorn traces of the real code.

**Five ported routines had no citation at all.** The `run_ship_3d_nav_choice_*`
handlers carried no doc comment, which is a prime-rule defect independent of
whether the code is right. The dispatcher at `0x86F1` gives all five addresses:
it makes the committed choice 0-based, doubles it and does
`call word cs:[bx+0xF29]` — a five-entry table (`0F33 0F4C 0FDD 1068 108C`) whose
CS base is `0x8709 - 0xF29 = 0x77E0`, so the handlers are `0x8713`, `0x872C`,
`0x87BD`, `0x8848`, `0x886C`. Entry 1 resolving to the already-labelled
`nav_choice_handler_1` at `0x872C` is what confirms the base. Handler 0 then
verified line by line (phase bit `[0x2565]&1`, `Honk` from `[0x6754]`, record type
`0xC3` into `[0x6768]`, phase cleared) and is settled ASM; the other four are
cited and labelled, awaiting the same treatment.

**The ledger was counting shader source as port code.** `gpu.rs` holds WGSL in
raw string literals, and the inventory regex happily matched `fn vs`, `fn fs`
and `struct VOut` inside them — six phantom rows, three of them ambiguous enough
that the settle tool refused to touch them. Raw strings are now skipped, and the
denominator dropped from 1423 to 1414 items that are actually port code.

**And one near-miss worth recording.** Settling the infra rows by bare NAME hit
`run` in four unrelated files — `vm.rs`, `extract/mod.rs`, `recomp/interp.rs`,
`recomp/runtime.rs` — none of which is plumbing. `audit_settle.py` now takes
`file:item` and accepts `UNVERIFIED` as a status so a mis-settle can be undone,
which is how those four got put back.

Ledger after the pass: 1414 items, 404 settled (ORACLE 209, ASM 136, INFRA 49,
DATA 10), 810 UNVERIFIED.

## FIX #56 — the 128-vs-176 font truncation was still live in the EXTRACTOR

Reading `src/bloodprg.rs` row by row turned up the same defect this campaign
already fixed once, still standing in a second place:

    pub const DIALOGUE_FONT_ASCII_MAP_LEN: usize = 128;

`font.rs` was corrected to 176 earlier — the map runs from `0x14C22` to the
advance table at `0x14CD2`, which is 176 bytes — but `dialogue_font_tables()`
was still slicing 128, so every consumer of the EXTRACTED font silently lost the
accented characters. Verified against the image before changing anything: the 48
entries past 128 are all in range, and **14 of them are live glyph indices**
(`0x82`->`é`, `0x84`->`ä`, `0x85`->`à`, `0x87`->`ç`, `0x94`->`ö`, nine more).

WHY IT SURVIVED A TEST. The test asserted

    assert_eq!(font.ascii_map.len(), DIALOGUE_FONT_ASCII_MAP_LEN);

which is unfalsifiable: the extractor sliced with that constant, so the assertion
holds for ANY value of it. This is the self-referential-test trap, and it is worth
naming because the row looked covered.

Replaced with checks that can fail:
* the map closes exactly on the advance table, and the advances exactly on the
  glyph rows (extent by LAYOUT, not by constant);
* every entry is `0xFF` or a valid glyph index;
* the high half holds exactly 14 real mappings;
* the extractor's bytes equal `font.rs`'s baked copy;
* and the strongest one — the three offsets are read back out of `render_string`
  as IMMEDIATES (`mov bx,0x7802` @`0x31C9`, the `gs:[eax+0x78b2]` displacement
  @`0x31E7`, `mov bp,0x7908` @`0x31EC`), so the port's constants are checked
  against the instructions that use them.

POSITIVE CONTROL: perturbing `DIALOGUE_FONT_GLYPHS_FILE_OFFSET` by 2 now fails
three tests. Before this change it failed none of the font ones.

A sweep found seven more `X.len() == CONST` assertions of the same shape
(`bloodprg.rs` x2, `bloodsav.rs` x4, `ship3d.rs` x1). They are recorded here as
the next ones to re-ground; the font was taken first because it is the one every
text surface depends on.

## FIX #57 — the remaining unfalsifiable length assertions, re-grounded

FIX #56 listed seven `X.len() == CONST` assertions of the shape that hid the font
truncation. The save-file pair went with #56; the other three are done here, each
now tied to something that can disagree:

| constant | was | now |
|---|---|---|
| `SHIP_3D_POINT_CLOUD_LEN` 1000 | `points.len() == THE_CONST` | the randomizer's own `mov cx,0x3E8` at `0x9B6A`, plus `mov di,0x2FC1` for the record base |
| `RENDER_SPRITE_BLITTER_ENTRY_COUNT` 8 | same shape | the dispatcher's `and bx,0x0E` at `0x44B7` — the index is masked to eight even word slots — plus the table's ninth word being zero |
| `SCRIPT_RESOURCE_PROFILE_*` | same shape | `mov si,0x11F4`, `mov dx,0x000A`, `mov cx,5` at `0x53CC/0x53CF/0x53D8`, and the count bounded by the DATA (rows 0..4 populated, the sixth all zeros) |

A cross-check fell out of the blitter one: `CS:0x1592` -> file `0x4522` puts that
table's segment base at `0x2F90`, which is segment `0x299` — the same code segment
as the string draw (`0x299:0x202`) and the tint blit (`0x299:0x40E`) decoded
earlier this session. Three independent decodes agreeing on one segment base is
the kind of corroboration a single byte comparison cannot give.

POSITIVE CONTROLS, all three at once: setting slots to 6, blitter entries to 7 and
the point cloud to 999 fails exactly three tests, one per constant. Before this
change all three perturbations passed.

Remaining from the #56 sweep: none. The `X.len() == CONST` shape is gone from the
tree except where the length is checked against something independent as well.

## FIX #58 — a tree-wide DS/file offset sweep, and four false alarms on the way

`OPTION_BOX_LABEL` carries a hand-written assertion that its DS offset and its
file offset describe the same byte (`file == 0xD420 + ds`). Nothing checked that
for any other constant, and a drifted pair is invisible to ordinary tests because
each half is individually plausible. `tools/check_offset_pairs.py` now does it for
the whole tree, wired in as a test.

RESULT: 17 pairs, all consistent. No defect in this class.

WHAT IT COST TO GET A TRUSTWORTHY ANSWER is the part worth recording, because the
tool reported FOUR mismatches before it reported the true one (zero):

1. Pairing by POSITION with an 80-char window matched one item's DS offset against
   the NEXT item's file offset — `"bitmaps at file 0x145CA = DS:0x71AA, map at
   file 0x1451A = DS:0x70FA"` paired `0x71AA` with `0x1451A`. Both halves of both
   pairs were correct.
2. Tightening the separator killed the false alarms but dropped coverage from 17
   pairs to 7 — precision bought with recall.
3. Pairing by SET correspondence instead (does each file offset have a DS partner
   anywhere in the block?) restored coverage, and then flagged `FS:0x11F4
   (file 0x0D3E4)`: a file offset under a DIFFERENT segment base, which cannot
   pair with a DS offset at all.
4. And `file 0x006BEA..0x006C04` — a CODE address, truncated to `0x006BE` by a
   `{4,5}` hex limit.

The rule that resolves both is one line: a file offset BELOW the DS base is not
DS-relative, full stop. That subsumes the segment-prefix special case, since code
addresses rarely carry one.

Every one of those four would have been reported as a port defect by a less
careful pass. The discipline that caught them is the same one from the palette
bytes and the recomp vector scan: verify the tool before believing the finding.

POSITIVE CONTROL: drifting `OPTION_BOX_LABEL_FILE_OFFSET` by 2 now fails the sweep
with the exact pair named. The test also asserts the sweep still FINDS at least 15
pairs, so a regex that silently stops matching cannot pass forever.

## FIX #59 — every quoted instruction now checked against the disassembly

The port's doc comments quote the binary constantly:

    ///   0x5DB4  mov ax,[si] / cmp ax,1 / jne 0x5DE3   owner kind == 1?

Nothing verified that the byte at `0x5DB4` decodes to `mov`. A wrong address in a
comment is worse than no comment — it sends the next reader to the wrong routine
while making the claim look sourced. `tools/check_cited_instructions.py`
disassembles every cited address and compares the mnemonic; it is now a test.

RESULT: 37 quoted instructions verified, 4 wrong — all four MINE, from the
tint-table work, and all four the same mistake. I had written

    0x22F5  push (pct*bx)/100

meaning "the block from `0x22F5` computes that and pushes it". `0x22F5` is
`mul bx`; the push is at `0x22FC`. In a block that otherwise quotes real
instructions (`0x22F1  neg ax`), a summary in quotation shape reads as a literal
quote and is not one. Corrected so each address names the instruction actually
there, with the computing range mentioned separately.

That is the whole value of the check: the semantics were right, the addresses
were not, and nobody would have noticed until someone followed one.

TWO THINGS WORTH KEEPING FROM BUILDING IT:

* Adding `re/tools` to `sys.path` before importing capstone breaks it — that
  directory holds a `dis.py` which shadows the stdlib `dis` capstone needs
  through `inspect`. `re/tools/dis.py` pops its own directory for this reason;
  the new tool imports capstone first.
* 33 lines were skipped as non-mnemonic (`0x9016  bx = [0x6752]` and similar
  prose). The tool reports that count rather than hiding it, so the verified
  number means what it says.

POSITIVE CONTROL: shifting one cited address by 1 (`0x91CE` -> `0x91CF`) fails
with `doc says 0x091cf is 'test', disassembly says 'add'`.

## FIX #60 — the baked tables that nothing compared to the image

Twelve constants in the port are TABLES COPIED OUT OF THE BINARY and say so in
their docs. Checking which had an image-comparison test:

| table | had one |
|---|---|
| `GAME_FONT_CHAR_MAP`, `SQUARE_CAPS_*`, `OPCODE_DESC`, `FIELD_OFFSETS` | yes |
| `GAME_FONT_GLYPHS`, `GAME_FONT_WIDTHS` | **no** |
| `NAV_DESTINATION_POINTS`, `SHIP_3D_HUD_PYRAMID_VERTICES` | **no** |

The two font ones are the actual glyph bitmaps and advances — what every text
surface in the game draws — and nothing checked them against `0x14CD2`/`0x14D28`.
They match; they are now asserted to, along with the three-table chain identity.

`NAV_DESTINATION_POINTS` deserves its own note, because it looks like a bug:

    pub const NAV_DESTINATION_POINTS: [[i16; 3]; 10] = [[10200, 12100, 900]; 10];

Ten IDENTICAL points reads as a placeholder someone forgot to fill in. It is not
— the shipped table at `DS:0x4F09` really is ten copies of `(10200, 12100, 900)`,
verified against the image. That is corroboration for the standing note that the
destination LAYOUT is the runtime-gated piece of the nav render: the baked table
cannot spread the markers because every entry is the same point, so the game must
write real positions at runtime. Worth having the test say so, since the next
reader will have the same suspicion I did.

The vertex test also re-pins the palette alias resolved earlier in the campaign
(`DS:0x5D98`'s 192 bytes ARE `GAME_SCREEN_PALETTE_DAC[576..768]`), so a change to
either constant surfaces the overlay rather than silently breaking the other.

POSITIVE CONTROLS: perturbing one glyph advance fails two font tests; perturbing
one nav point fails the geometry test and the existing engine test that asserts
the points coincide.

## FIX #61 — the ledger was hiding a third of the port from itself

Settling the newly verified tables turned up `NOT IN LEDGER` for all of them, and
the reason was worse than a missing feature.

**Constants were never enumerated.** The inventory matched `fn|struct|enum`, so
the port's TABLES — the most directly falsifiable things in the codebase, copied
byte-for-byte out of the binary — had no rows at all. Four of them had no image
comparison either (FIX #60), and nothing in the ledger could say so.

**And `in_tests` LATCHED.** Once a file contained `#[cfg(test)]`, every item after
it was skipped for the rest of the file. `src/font.rs` keeps real code after its
test module — `BoldConsoleFont`, `draw_square_caps`, and all three `SQUARE_CAPS`
tables — so that entire tail was invisible to the campaign. The flag now tracks
the module's braces and clears when it closes.

Effect on the numbers, which is the point worth being blunt about: the denominator
went from 1415 to **2106**. The ledger did not get worse; it stopped
under-reporting. 691 items existed in the port with no row, and any percentage
computed before this was measuring a subset that happened to exclude the tables.

Settled 11 of the recovered rows immediately as DATA — every table with a
byte-for-byte image test (`GAME_FONT_*`, `SQUARE_CAPS_*`, `NAV_DESTINATION_POINTS`,
`SHIP_3D_HUD_PYRAMID_VERTICES`, `FIELD_OFFSETS`, `OPCODE_DESC`,
`WORLD_ART_DIRECTORY`).

The lesson generalises past this project: a coverage metric that silently drops
items is worse than no metric, because it reports progress on the part it can see.
Both defects here made the ledger look BETTER than reality.
