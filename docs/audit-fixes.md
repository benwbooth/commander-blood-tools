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
