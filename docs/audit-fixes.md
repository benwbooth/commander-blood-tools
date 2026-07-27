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

## FIX #62 — validating the RE knowledge base itself

`re/labels.csv` is what every future decode session reads first, so an error there
propagates into work that never re-derives the claim. Nothing validated it.
`re/tools/check_labels.py` now does, three ways: every flat address is inside the
image, every code address decodes, and where a comment quotes an instruction the
mnemonic matches.

RESULT: 553 code labels and 227 data labels, 0 problems. The knowledge base is
clean.

THE TOOL WAS WRONG FOUR TIMES BEFORE IT WAS RIGHT, which is now a pattern worth
naming explicitly, because it has held for three tools in a row:

* `0x0006EC,startup_fail_or_main,"jmp target after early init"` — flagged because
  the comment's first word is `jmp`. It describes the address as a jump TARGET.
* `0x014C22,font_ascii_map` — flagged as "does not decode". It is a font TABLE;
  data does not decode.
* Then, extending the check to instructions quoted INLINE, 17 more "problems",
  every single one English prose: comments say "gs:[0x523B] and the clip",
  "[0x250B] or its fallback". `and`, `or`, `not`, `sub`, `add`, `test`, `in` and
  `int` are ordinary words as well as mnemonics. Requiring BACKTICKS around the
  instruction fixed it — at the cost of dropping from 17 candidate claims to 3
  real ones.

Zero of the 21 initial "findings" were real. THE PATTERN: prose is not a
machine-readable format, and a checker over prose reports its own grammar
mistakes as defects in the subject. Every one of these tools (the DS/file sweep,
the instruction sweep, this one) produced a first run that was entirely false
positives. The discipline that makes them useful anyway is refusing to act on a
finding until the tool is verified — and building a POSITIVE CONTROL, which is
what separates "0 problems" from "checking nothing".

The control here needed care too: my first attempt perturbed rows whose comments
the checker does not examine, so it reported 0 problems and proved nothing. The
real control shifts a label whose comment OPENS with a quoted `mov` (caught: "the
code is `push es`") and pushes another past the end of the image (caught).

## FIX #63 — unicorn in the flake, and 13,000 FRESH vectors say the lifts are right

`re/tools/README_oracle.md` had carried this for the whole campaign:

> `pip install unicorn` (not yet in the nix flake — add it there to make this a
> permanent test, or run in a venv as the PoC does).

`python3Packages.unicorn` is in nixpkgs (2.1.4). Added. That was the whole
blocker, and removing it changes what the recomp verification MEANS.

Replaying the committed vectors proves each lift matches THOSE inputs. It cannot
distinguish a correct lift from one that happens to agree on 200 fixed random
states. Regenerating is a different claim — new random register/memory states run
through the REAL DOS bytes under Unicorn, compared bit-exactly against the Rust.

DONE, for the whole generic batch: **52 functions, 13,000 freshly generated
vectors, every one bit-exact.** Not a replay; independent evidence.

`re/tools/reverify_lifts.sh` makes it repeatable — it regenerates, replays, and
restores the committed vectors so the tree stays clean (`--keep` to keep them).
The committed corpus is deliberately left in place: it is already proven, and
swapping 13,000 equally-valid random vectors would be a large diff with no gain.

What this does NOT do is generate vectors for anything new — the batch is the
functions the clean-lift pipeline already produces. But the pipeline can now be
RUN, which is the difference between a documented blocker and a working tool.

## FIX #64 — the console choice box was PAINTED; the game TINTS it

Sweeping `engine.rs`'s unverified rows, `draw_choice_box` had well-cited geometry
(`0x84A1`, `0x847A`, `0x8508`, `0x857D`, the three anchors) and four colours that
were not cited at all:

    const BORDER: u8 = 0x15;   // "from the live index dump"
    const FILL: u8 = 0xE0;     // "measured from choice_box_bob_morlock.ppm"

Following the prime rule — find the code that produces the surface — the widget
does no such thing. At `0x84D8` it loads `si = [0xAC8]` (`0x5F11`, the
50%-toward-black remap table built by `0x22E0`) and calls `0x299:0x40E`: the same
TRANSLUCENT-WINDOW primitive the destination info panel uses, decoded earlier this
session. There is no border/fill pair anywhere in the routine. The `[0x27E6]&1`
branch at `0x84CD` is a query-only early return, not an alternate draw.

So the console's choice box darkens whatever it covers. The port painted flat
black over it.

WHY THE CAPTURE WASN'T MISREAD — it was OVER-GENERALISED. The measured box sits
over the panorama's dark orb socket, where a 50% tint really does resolve to index
`0xE0` for most pixels. The porter recorded what was there. The error was
concluding that the box IS `0xE0`, rather than that `0xE0` is what a tint produces
HERE. Over any lighter surface the two diverge completely. The rewritten test
makes exactly that distinction: it does NOT assert `0xE0` disappears (it doesn't),
it asserts the region stays VARIED, which a flat fill cannot be and a tint always
is.

The label colours turned out to be in the assembly after all, and the port was
missing one: `mov al,0xE8` (`0x8565`) unselected, `mov al,0xEF` (`0x858B`)
selected, and `mov al,0xFE` (`0x8595`) for the selected row WHILE THE MOUSE IS
ENABLED (`test byte gs:[0xA3E],1`). The port had the first two and no notion of
the third.

The old test was named `choice_box_matches_the_measured_spec`. That name was the
tell: under the prime rule there is no such thing as a measured spec.

## FIX #65 — the list menu drew every label flush left; the widget centres them

Continuing the `engine.rs` sweep along the same seam as FIX #64. `draw_list_menu`
put every label at a fixed `x = 170`, documented as "the capture-matched left
edge".

The widget centres each label on its anchor: `label_x = x0 + 10 +
(widest - width)/2` with `x0 = anchor - (widest+20)/2` (`0x84AD`,
`0x857D..0x8582`) — the same math `draw_choice_box` already implements, since it
is the same widget. `170` is what that formula produces for the ~105px label in
the capture. Frozen as a constant, it made narrow labels wrong by half their
width difference: `EGO` (28px) belongs 38px right of `BOB_MORLOCK` (105px), and
the port drew them at the same x.

The SHAPE is the defect, not the number. A flush-left list cannot be right for
more than one label width, so no choice of constant fixes it. The test asserts
the shape directly — the narrower label sits further right, by exactly
`(widest - width)/2` — rather than checking either label against a measured
position.

Same lesson as #64, from the other direction: #64 was a capture generalised past
where it held; this is a DERIVED value frozen as though it were a constant. Both
look like "we matched the capture", and both are only right in the one frame that
was measured.

ALSO FIXED, found while running the suite: my FIX #60 insertion had swallowed the
`#[test]` attribute of `randomize_point_cloud_fills_all_records_and_consumes_
three_rng_calls_each`, so that test silently stopped running, and left a stray
duplicate attribute above the new one. Both repaired; rustc's
`duplicate_macro_attributes` warning is what surfaced it.

## FIX #66 — there is no save screen; it is the slot list with one row being typed

Third defect on the `engine.rs` capture seam, and the largest. The port composed a
SAVE screen by hand:

    a grey 0xE8 bar at x63..137, y39..48
    the typed name in bold 0xEF at (73, 40)
    CANCEL at (73, 150)

all "oracle-measured (vs_011, the live save flow)". None of those positions exist
in the game. The save flow at `0x1BAB` sets `[0x2734]` to the slot record being
renamed and copies its 16 name bytes into the edit buffer at `DS:0x273B`
(`rep movsd cx=4` @`0x1BBD`); the list widget then substitutes that buffer for the
matching row as it draws (`cmp si,[0x2734] / jne / mov si,0x273B` @`0x8573`).

**The save UI is the ordinary ten-row slot list with one row being typed into.**
The capture showed a "grey bar with a name in it" because that is what a
highlighted list row containing two characters looks like.

To render it the port needed the slot names, which it had documented and never
parsed. `bloodsav::parse_slot_directory` now reads `blood.sav`'s ten 32-byte
records — verified against the real file: every slot names its own `game<N>.sav`
in order, slot 1 carries the `ab` typed during the live save, the rest are blank.

The EDIT LAW was already right and stays: `0x1DD8` gives digits and lowercase
only, 14 characters, Enter commits. That part was decoded from the binary; only
the rendering was measured.

The old test was `save_slot_ui_renders_and_edits_like_the_oracle`, and — as with
`choice_box_matches_the_measured_spec` — the name was the tell. Its edit-law half
survives unchanged; its render half now asserts the typed row draws in the
widget's own band and colour instead of a measured rectangle.

THREE FOR THREE on this seam (#64 tint, #65 centring, #66 save UI): every surface
whose comment said "measured" turned out to be a shared widget the port had
re-implemented from a photograph of one frame.

## FIX #67 — half the harvested console band was a copy of a constant the port already had

The intro montage's console band is `include_bytes!`'d from two capture files.
The palette half is gone: `console_band.dac`'s entries `224..255` are
BYTE-IDENTICAL to `GAME_SCREEN_PALETTE_DAC` over the same range — 0 of 96 bytes
differ — so the harvested DAC duplicated a constant already sourced from the image
at `0x12F78`. The overlay uses the constant now, and a test pins the equivalence
so the removal fails loudly if it ever stops holding.

The INDEX half is still a capture, and this narrowed it usefully rather than
leaving it as "harvested art":

* the band uses exactly SIXTEEN indices, `224..=239` — a dedicated console bank;
* it is NOT a slice of the bridge panorama. I checked, expecting it would be:
  `TB.BIG`'s frames draw in indices `0..~75`, a DISJOINT range, and every frame
  differs from the band in 100% of bytes. The port already renders that panorama
  pixel-exactly, so had it been a slice this would have been a two-line fix.
* raw byte statistics over the shipped `.SPR`/`.EXT` files do not identify the
  source — they are compressed, and a dozen unrelated files show ~50% of their
  bytes in the `224..239` window by chance. Recorded so the next attempt does not
  repeat it.

So the remaining work is to decode the console-band DRAW CALL rather than to
search the assets, which is a better-posed task than the one this started as.

## FIX #68 — the game has a confirm dialog; the port had none

Chasing the console band's draw call turned up a different surface entirely: a
`mov dx,0x8C` (y=140) search led to `0x14E6`, which is not the band but the
`ARE_YOU_SURE?` CONFIRM DIALOG — decoded completely, and absent from the port:

```text
  0x14E6  bx=0x5A cx=0x50 dx=0x8C bp=0x28   the box rect (90,80,140,40)
  0x14F2  lcall 0x299:0xCDC                 draw it
  0x14F7  mov al,0xE8                       text colour
  0x14FE  si=0x17B "ARE_YOU_SURE?"          bx += 0x0A, dx = 0x58   -> (100, 88)
  0x150C  si=0x189 "YES"                    bx += 0x14, dx += 0x11  -> (120, 105)
  0x151A  si=0x18D "NO"                     bx += 0x3C              -> (180, 105)
  0x1525  bp=0x2555 / 0x255D                the two hit regions
```

THE CORROBORATION IS THE NICE PART. The draw positions are computed by successive
adds on `bx`/`dx`; the hit regions are a separate pair of records at `DS:0x2555`
and `DS:0x255D`. They agree exactly: `YES` draws at x=120 and its region is
`(120, 105, 30, 10)`; `NO` draws at x=180 with `(180, 105, 20, 10)`. Two
independent tables describing one layout — the test asserts the relationship
rather than the numbers, so a wrong constant in either breaks it.

Ported as `draw_confirm_box` / `confirm_box_click`, with the three strings pinned
to their image bytes and the box rect read back from the `mov` immediates at
`0x14E7..0x14F1`.

The console band, meanwhile, is still open — but the search that missed it is
recorded, and the ARE_YOU_SURE? box is a surface the port simply did not have.

## FIX #69 — sweeping the UI STRING TABLE for surfaces the port never knew about

The confirm dialog (#68) was found by accident, searching the binary rather than
the port. This does it deliberately: `re/tools/check_ui_strings.py` takes every
string in the UI table (`DS:0x100..`) and asks whether any `mov reg,imm` in the
image carries its offset — i.e. whether a draw site can reach it.

21 strings. The answer splits three ways:

**DEAD — five strings no code references at all:** `ABSENTE`, `GO`, `ON`, `OFF`,
`REC`. Shipped but unreachable. Worth recording precisely because they look like
obvious features to implement — a `REC` indicator, an `ON`/`OFF` toggle — and
implementing any of them would be inventing a surface the game does not have.
(`ABSENTE`'s pair `PRESENTE` IS referenced, once, at `0xB7C9`, stored as a far
pointer beside the audio flags.)

**LIVE and already ported:** the four status-panel headers, `CANCEL`,
`ARE_YOU_SURE?`/`YES`/`NO`, the filenames.

**LIVE and MISSING — three of them:**

```text
  0x16BC  si=0x159 "LOADING"  ax=0x82 bx=0x60  dl=0xEF   lcall 0x299:0xD6
  0x1ABB  si=0x166 "PAUSE"    bx=0x87 dx=0x60  al=0xE8   lcall 0x299:0x498
  0x1B58  si=0x161 "LAST"     -> DS:0x270D, [0x2734], jmp vm_state_save
```

The first two are centred status overlays, both on the y=96 band. The third is
better than a label: it is the QUICKSAVE. The game copies the literal `LAST` into
the slot-name buffer, points the save flow's `[0x2734]` at it — the same global
the interactive rename uses (FIX #66) — and jumps straight into `vm_state_save`
with no prompt. The port had no quicksave at all.

All three ported, with the positions, colours and buffer address read back from
the `mov` immediates.

METHOD NOTE: this is the second time a sweep of the BINARY's own tables found
something the port's documentation could not have suggested. A ledger of what the
port HAS will never list what it LACKS; only the game's own data will.

## FIX #70 — the resource sweep that could NOT work, and why that is worth knowing

Applied the UI-string method one level up: which of the 95 shipped resources can
be proven reachable? Two results, one good and one instructive.

**The good one is an extent.** The resource-name table's size was never pinned.
It is a layout identity: `0xCDF4 + 95*16 = 0xD3E4`, exactly the script-profile
table — so 95 records, ids `0..94`. Corroborated independently: 94 (`ondoya.ext`)
is the highest id the world-art table uses. My first run walked past the end and
happily reported `id 99 'minimum !'` and `id 122 'CANCEL'` as resources, which is
the profile rows and the UI string table being read as filename records. Now
pinned in `levels.rs` too, alongside the note that the port's `LEVEL_DIRECTORY`
holds 53 of the 95 — the subset it needs, not the whole table.

**The instructive one: the sweep reported 13 "unreferenced" resources, and the
list is not trustworthy.** Id 16 is `borxx.spr` — the EYE ORB the nav HUD draws
every frame. It appears in no profile row, no world-art record and no
`mov ax,16`, and it is obviously live.

WHY THE METHOD TRANSFERS TO STRINGS BUT NOT TO RESOURCES: a draw site must load a
string's DS offset as an IMMEDIATE — there is nowhere else for the offset to come
from, so absence of an immediate proves absence of a draw. Resource ids arrive in
`AX` from DATA: the `.ext` object records feed `entity_object_populate` (`0x40D0`)
with ids the executable never mentions. Absence of an immediate proves nothing.

So the tool now says what it can prove — reachability — and states plainly that
its zero-hit list means "route unknown", with `borxx.spr` named in the source as
the counter-example. Publishing that list as "dead resources" would have invited
someone to delete or skip live content.

FOUR TOOLS THIS SESSION have needed their first output disbelieved (DS/file pairs,
quoted instructions, labels.csv, this). The pattern is stable enough to state as a
rule: a new checker's first run is a test OF THE CHECKER. Only once it survives a
positive control and a known-good counter-example does its output mean anything.

## FIX #71 — following the sharpened question found the second tint table's consumer

The band hypothesis (#70) predicted the game repaints only the top rows during the
montage and leaves the band alone. Following that into the intro path confirmed it
and turned up something else:

```text
  0x7AC3  bx=0 cx=0 dx=0x140 bp=0xC8 si=DS:0x6011
  0x7AD1  lcall 0x299:0x40E     remap the WHOLE 320x200 screen through table 0x6011
  0x7AD6  ax=0 bp=0x8C cx=0
  0x7ADE  lcall 0x299:0xCDC     draw a (0,0,*,140) region — the FILM area only
  0x7AE3  swap [0x5221]/[0x5229]  page flip
```

Two things fall out.

**The prediction held.** The per-frame work provably never touches rows 140..200,
which is why the band persists. The frame-size evidence and the code now agree.

**`DS:0x6011` has a consumer.** Earlier this session `0x45C8` was decoded as
selecting between two adjacent 256-byte tint tables, `0x5F11` and `0x6011`, with
the choice stored at `gs:0x524B` — but only `0x5F11`'s users were known (the info
panel, the choice box). `0x6011` is the montage's: every cinematic frame is
presented through a FULL-SCREEN remap.

That last part is a port gap in its own right, separate from the band: the port
plays the montage untinted and pastes a captured band over it. The correct shape
is a full-screen remap through the second table, with the film drawn into the top
140 rows and the band left standing from before.

## FIX #72 — the montage's remap table has a second builder, and it reframes the band

#71 found the montage remaps the whole screen through `DS:0x6011`. That table is
all zeros in the image and never appears as a `mov di` destination for the tint
builder `0x22E0` — which, taken alone, would mean the remap is a clear-to-black.
That reading is tempting and would have been wrong; the real-game captures show
the band VISIBLE during the montage.

The resolution: **there is a SECOND builder.** All five calls to `0x1CE:0x0000`
(`0x22E0`) target `0x5F11` with `ax=0xFFCE` — 50% toward black. `0x6011` is filled
by `0x1CE:0x014D` (`0x242D`) at `0x9622`, with `ax=0xE0` and `bx=0x6011`. It walks
the same live palette at `DS:0x5251` over 256 entries with a different rule.

`0xE0` is 224 — the base of the 16-colour CONSOLE BANK the intro band's pixels all
lie in. So the montage's full-screen remap plausibly maps the screen INTO that
bank, which would explain the harvested band's index range without a separate art
asset at all: the band is whatever was on screen, remapped.

Stated as plausibly as the evidence supports — the parameter matches the bank base
and the consumer matches the surface, but the builder's per-entry arithmetic is
not yet read line by line. What makes that cheap: `func_242d` is ALREADY LIFTED
BIT-EXACTLY in the port's recomp module and oracle-verified, so the port can
execute it and observe the table rather than deriving it by hand.

WHAT THIS CHANGES ABOUT THE BAND. It was "harvested art, source unknown", then
"drawn once before the film", and now most likely "not drawn as art at all" — a
side effect of the presentation remap. Three sessions of hypotheses, each one
cheaper than the last because the eliminations were written down.

## FIX #73 — the band is not art: the montage reduces the whole screen to 16 colours

#72 left this as plausible. Settling it took executing the game's own builder
rather than reasoning about it — `func_242d` is lifted bit-exactly in the port's
recomp module and oracle-verified, so it can simply be RUN against the real
palette and the answer observed.

The result is unambiguous. `DS:0x6011` maps every one of the 256 palette indices
into `224..=239` — the 16-colour CONSOLE BANK — with the bank fixed under the map
(so the remap is idempotent). The montage's per-frame setup (`0x7AC3`) pushes the
WHOLE 320x200 screen through it before drawing the film into the top 140 rows.

**So the harvested console band was never art.** Its pixels are all in
`224..=239` because during the montage EVERYTHING on screen is: the console
already standing from the previous state comes out in the bank like the rest of
the frame. The port's `include_bytes!` of a captured 320x60 band is a photograph
of a colour-reduced console, not an asset the game contains.

A WRONG TURN WORTH RECORDING. The obvious reimplementation — map each colour to
the nearest bank entry by squared RGB distance — reproduces only 68 of the 256
entries. I nearly wrote it as a "native" version of the routine. The port now
calls `func_242d` itself, and the test asserts that nearest-colour DISAGREES, so
anyone tempted to simplify the call into a loop fails immediately.

`palette::build_console_bank_remap_table` exposes it. What remains for the band is
frontend sequencing — having the console on screen before the montage so there is
something to reduce — rather than any decode.

## FIX #74 — the montage now presents through the game's remap

With `DS:0x6011`'s contents settled (#73), the montage's presentation is portable.
`0x7AC3` pushes the whole 320x200 screen through the console-bank table before
drawing the film into the top 140 rows; `EngineState::apply_console_bank_remap`
does exactly that, using the table the game's own builder produces.

The test asserts the property that makes it recognisable: start with a spread of
indices mostly OUTSIDE the bank, remap, and every pixel is in `224..=239` — and a
second remap changes nothing, since the bank is fixed under the map.

The captured band still overlays afterwards, and stays until the intro sequencing
puts the console on screen ahead of the montage — but the presentation itself is
now the game's, so the film area is banked exactly as the game banks it rather
than played in full colour with a photograph pasted underneath.

## FIX #75 — stale docs cleared, and one `accuracy/` blob shown to be game data

Two pieces of housekeeping the band work exposed, both about provenance CLAIMS
rather than code.

**The hand-atlas comment described a deleted capture.** `draw_hand_at_mouse`
carried 15 lines documenting `accuracy/captures/bridge/hand/hand_<x>_<y>.bin` —
sprites harvested per cursor position from the emulator — as the port's hand
source. That atlas was deleted a session ago; the port renders the real
`manu3.xdb` skeletal mesh. The comment survived the deletion and would have sent
the next reader looking for capture files as the source of truth. Replaced with
what is actually true, including a note that the atlas is gone so the next person
does not go looking.

**`trig_tables.bin` is not a snapshot.** Four blobs live under `accuracy/manu3/`
and are `include_bytes!`d, which reads as captured memory. Checked against the
shipped `manu3.xdb`:

| blob | provenance |
|---|---|
| `trig_tables.bin` (4100 B) | **all 4100 bytes are `manu3.xdb` at `0x1396`** — game data |
| `manu3_seg4_1c94.bin` (64 KiB) | a segment dump whose first 17948 bytes are the xdb's texture |
| `manu3_ds.bin`, `manu3_seg2_1b76.bin` (64 KiB each) | genuine segment dumps |

So one of the four is the game's own file and is now documented and TESTED as
such, pinned to its offset. The other three are honest runtime dumps and stay
labelled that way — knowing which is which is the point, since "lives under
accuracy/" had been carrying all four.

Also: the port no longer `include_bytes!`s a capture at RUNTIME at all. The two
console-band captures now appear only inside tests, as fixtures the composed-from-
asset version is checked against — which is exactly the role the prime rule
assigns an oracle.

## FIX #76 — the prime rule, enforced by a test

Six items this session were the same shape: a comment claiming a value came from a
capture. Three were DEFECTS (the choice box's colours, the list menu's `x`, the
save UI's whole layout) and three were STALE NOTES left behind by the fixes (the
hand atlas, the square-caps advances, the viewscreen band). That is frequent
enough to deserve a guard rather than another grep, so `tools/check_provenance.py`
is now a test.

THE RULE IT ENFORCES: no comment in RUNTIME code may say a value was measured off
a capture unless the same comment run either cites a binary address — the routine
that replaced it — or says it no longer applies. That is exactly the shape the
prime rule allows: "was measured, now derived, here is the routine."

Two exemptions, both principled:
* TEST code. Comparing rendered output against a capture is what the oracle is
  FOR; the rule constrains where behaviour comes from, not what a test checks.
* `src/bin/runtime_boot.rs`, the oracle harness. Measuring the real game is its
  entire job.

Standing at 9 claims in runtime code, 0 unexplained. The test also asserts the
sweep still FINDS claims, so a regex that quietly stops matching cannot pass
forever.

POSITIVE CONTROL, and it took two tries — worth recording because the first
attempt was a bad control, not a bad tool. Injecting the claim as a `///` line
directly above `draw_status_overlay` did NOT fire: it merged into that function's
existing doc run, which cites `0x16BC` and `0x1ABB`, so the checker correctly read
it as explained. Placing the claim in an isolated comment inside a function body
fires as intended. A control that passes can mean the guard is broken OR that the
control was wrong; distinguishing those is the same discipline as the four tools
that needed their first output disbelieved.

## FIX #77 — the self-referential assertion, guarded

The second class-wide guard, on the shape that hid the font truncation:

    let ascii_map = slice(MAP_OFFSET, DIALOGUE_FONT_ASCII_MAP_LEN);   // 128
    assert_eq!(font.ascii_map.len(), DIALOGUE_FONT_ASCII_MAP_LEN);    // always true

The real table is 176 entries. The extractor read 128 — dropping every accented
character — and the test agreed with itself for a whole campaign. Seven such
assertions were re-grounded (#56, #57); `tools/check_selfref_asserts.py` now runs
as a test so an eighth cannot appear unnoticed.

TWO REFINEMENTS THE FIRST RUN FORCED, both cases of the tool being wrong rather
than the code:

* It flagged four `len() == W` assertions that were really `len() == W * H`. The
  regex stopped at the first constant of a product. A dimensional identity ("the
  framebuffer is width times height") is not an extent claim about game data;
  the RHS must now be a SINGLE constant.
* It flagged `bloodsav`'s synthetic round-trip, whose constants ARE grounded — in
  a sibling test that reads them back from the writer's `mov cx,imm` immediates.
  Grounding is now searched file-wide, since evidence in another test is still
  evidence.

Standing at 7 assertions, 0 ungrounded. POSITIVE CONTROL: an isolated
`len() == CONST` in a file with no independent evidence fires; removing it clears.

That is five checkers now running as tests — DS/file pairs, quoted instructions,
labels.csv, capture provenance, and this. Every one of them was wrong on its first
run, and every one now guards a class that produced a real defect this session.

## FIX #78 — the nav-choice handlers, verified rather than merely cited

FIX #55 gave the five handlers their addresses (the `cs:[bx+0xF29]` dispatch table)
and verified handler 0. Handlers 1, 2 and 3 are now read against the disassembly
too, and all three match:

* **1 (`0x872C`)** — adjusts the EXISTING target list in place: `add ax,4` per
  entry to the `0xFFFF` terminator, reset `[0xADB]`, layout prepass, advance phase.
* **2 (`0x87BD`)** — REBUILDS the list from the special-slot array at `DS:0x6D3E`
  (the 16-word block `0x53FF` clears with `cx=0x10`, matching the port's
  `SPECIAL_OBJECT_SLOT_COUNT`), skipping zero slots, storing `slot + 4`, stopping
  at the sentinel.
* **3 (`0x8848`)** — structurally handler 0 with two differences: the deferred
  record's related object is `menu` (`gs:0x6756`) not `Honk` (`gs:0x6754`), and it
  reloads `sn\radio.snd` (`DS:0xD16`, confirmed by reading the string).

A CORROBORATION FELL OUT. Handlers 1 and 2 both bracket their layout call with
`[0x27E6]=1` / `[0x27E6]=0`. That is the widget's QUERY-ONLY flag, whose early
return (`0x84CD` -> `0x85D3`) was decoded independently while establishing that the
choice box is a tint rather than a painted box — a different investigation, days of
work apart in the transcript, arriving at the same flag from the other side.

One port-side difference worth stating rather than hiding: where the original's
rebuild loop would run off the end of the slot array if no sentinel appeared, the
port returns `None`. It refuses to invent a list instead of reading past the array.

Nine ship3d rows settled as ASM across this and the previous pass.

## FIX #79 — the citation guard caught me making the same mistake it was built for

Verifying `render_ship_3d_point_cloud` against `0x9A10` — the starfield batch loop
— went cleanly: count `0x3E8` at `0x9A1D`, cloud at `0x2FC1`, matrix at `0x2F95`,
target segment from `[0x5223]`, and a per-point camera translation whose SUBTRACT
`projection_component` reproduces exactly. `DS:0x2F65` is the same camera origin
the nav renderer already cites as `(10000, 12000, 0)`.

Then the suite went red. `check_cited_instructions.py` (FIX #59):

    MISMATCH src/ship3d.rs:1885: doc says 0x09a3f is `sub`, disassembly says `mov`
    MISMATCH src/ship3d.rs:1886: doc says 0x09a44 is `sub`, disassembly says `mov`

I had written `0x9A3F  sub word [di],[0x2F65]`. The real sequence is

    0x9A3F  mov ax,[0x2F65]
    0x9A42  sub word [di],ax

— the address of the block that computes the value, not of the instruction named.
That is EXACTLY the error FIX #59 was built to catch, made again, in new work,
within minutes of writing the guard's own justification.

Two things follow. The guard earns its place: this would have shipped as a wrong
citation that a future reader would have followed to the wrong instruction. And
the error class is evidently not something care alone prevents — I knew about it,
had just written it up, and still made it. That is the argument for turning each
found defect into a check rather than into a resolution to be careful.

Rows settled: `render_ship_3d_point_cloud` and `projection_component`.

## FIX #80 — the engine's PRNG was never tied to the oracle; now it is

`recomp::prng_2de2` — the LIFTED generator — has been oracle-verified against 300
vectors for the whole campaign. `ship3d::BloodPrng::next` is a separate
HAND-WRITTEN reimplementation, and it is the one the engine actually calls: the
starfield, and every other randomised surface, run on it. Nothing tied it to the
real generator. Its ledger row said `ASM?` — an address cited, nothing checked.

Now the same 300 vectors run through both. The native implementation agrees
exactly: result, both mixing bytes, the counter, and the seed word staying
unchanged.

That is a case worth naming, because it is invisible from either side alone. The
lift was verified and the port had a citation, so both looked covered — but they
are DIFFERENT CODE, and only a differential test says whether the one the game
actually runs on matches. The row moves from `ASM?` to `ORACLE`, and if the two
ever drift the suite says so.

Generalises: wherever the port has BOTH a lifted function and a native
reimplementation, the lift's verification does not transfer. `recomp` holds 84
oracle-verified lifts; any of them with a native twin deserves the same treatment.

## FIX #81 — the field-offset resolver, swept against its own lift

Acting on #80's generalisation: where the port has BOTH a lifted function and a
native reimplementation, the lift's verification does not transfer. Twenty-one
port items cite a lifted address; this takes the one that matters most.

`vm_field_offset` is the resolver every selector lookup in the VM goes through —
`shl ax,4`, `bsf bx,bx`, `mov al,gs:[bx+0x6D60]`. Its lift `func_6023` is
oracle-verified; the native version was `ASM?`, an address cited and nothing
checked. A divergence there would mis-resolve a record field, which is the
quietest way this port could go wrong: no crash, no visual tell, just the wrong
word read for the rest of the run.

The new differential sweeps the WHOLE REAL DOMAIN — every selector row of the
matrix against every single-bit kind, 336 combinations — running the lift in the
recomp Machine with the matrix seeded from the image, and requiring the native
result to match each time. It also asserts the port's baked `FIELD_OFFSETS` IS the
image's bytes.

They agree everywhere. Three rows move to `ORACLE`.

Remaining twins worth the same treatment, from the same scan: `special_slot_insert`
/`special_slot_remove` (`0x5FF6`/`0x5FD8`), `square_caps_text_width` (`0x30CD`),
`entity::advance_state` and `toggle` (`0x41D1`/`0x420D`), `save_ui_key` (`0x1DD8`),
`step_ship_3d_nav_state` (`0xB75C`).

## FIX #82 — the special-slot pair: a latent divergence, and a `[bp]` trap

Second of the twin differentials. `special_slot_insert`/`special_slot_remove`
manage the 16-word list at `DS:0x6D3E`; their lifts `func_5ff6`/`func_5fd8` are
oracle-verified, the native versions were `ASM?`.

A REAL DIVERGENCE, though a latent one. `0x5FD8` clears THE FIRST match and
returns `stc`; the port cleared EVERY match and returned nothing. Not observable
today — `special_slot_insert` refuses duplicates, so a value cannot normally
appear twice — but it is not what the game does, and the discarded carry flag is
real information. Both fixed: first match only, boolean returned.

The differential drives both through one script (insert, duplicate, remove
present, remove absent, fill, overflow) and compares the LIST CONTENTS and the
flag after every step, including the full-list `clc` that separates "inserted"
from "no room".

THE TRAP, worth recording for the next differential. The first run showed the
lift doing NOTHING — its list stayed all zeros while the native inserted
correctly. The routine addresses the list as `mov bp,0x6D3E / cmp ax,[bp]`, and a
`[bp]` operand defaults to **SS**, not DS. The test had `ss = 0x9000` for a stack,
so the lift was faithfully reading and writing a list in a segment nothing else
touched. Setting `ss = gs` fixed it.

That is a property of the ORIGINAL worth knowing: this list is reached through the
stack segment, so any harness that runs these routines must have SS pointing at
the data segment, as the game does.

## FIX #83 — the text measure, and a latent divergence recorded rather than papered over

Third twin differential. `0x30CD` measures a string: pick a face from `AX` (0 =
square caps at `DS:0x7362`/`0x7412`, else the game font at `0x7802`/`0x78B2`),
accumulate each glyph's width via `xlatb` / `add dl,gs:[eax+edi]` / `adc dh,0`,
subtract 2 for the trailing gap. Every menu's centring depends on the answer.

Native and lifted agree across the real menu vocabulary — `TALK`, `CANCEL`,
`BOB_MORLOCK`, `EGO`, `LIBIDO`, `REMEMBER`, `BYE_BYE`, single letters, the widest
and narrowest glyphs. Two rows to `ORACLE`.

THE DIVERGENCE I FOUND WHILE READING IT, and did not fix. For a character whose
xlat entry is `0xFF`, the original does NOT skip: it indexes the 48-entry width
table with `0xFF` and adds whatever byte lies at `DS:0x7412 + 0xFF` — which is
inside the GLYPH ROWS at `0x7442`. The port contributes 0.

Every label the game measures comes from its own DIC or the built-in strings, and
all of those map, so the case cannot arise in play. Reproducing it would mean
importing a garbage read for no observable gain. But "the port adds 0" is a
CHOICE, not the original's behaviour, and the difference is now written where the
function is rather than left for someone to rediscover as a mystery. That is the
same standard as the special-slot removal (#82) — state the divergence, say why it
is latent, and let the next reader decide.

## FIX #84 — the entity pair: a stale citation and an exhaustive sweep

Fourth twin differential, and it found a citation pointing at the wrong routine.

`EntityObject::toggle` cited `0x420D` as the toggle family. `re/labels.csv`
CORRECTED `0x420D` long ago to `sprite_slot_set_draw_position` — the nav
projector's `AX=id, BX=x, CX=y` setter, which is not a toggle at all. The
correction never reached the port. Repointed at `0x428C` and its siblings, with
the actual sequence quoted (`or al,al / jns`, `xor al,0x40`, `test al,1 / je`,
`or al,2`) and the detail that the state-advance test happens AFTER the toggle —
which the port already had right.

`advance_state` is now swept EXHAUSTIVELY against `func_41d1`: all 256 flag
values, comparing the stored word. They agree everywhere, including the paths
where the original stores an unchanged word back.

A HARNESS BUG WORTH RECORDING. The first run failed on input `0x0000` with
"lift 0x0000 vs native 0x0082". The port was fine; my harness built the entity
with `EntityObject::populate(flags, ..)`, and `populate` applies the ACTIVATION
formula `([si]&4)|0x83` from `0x40D0` rather than storing the word. So the test
was comparing "advance the state of a freshly activated entity" against "advance
the state of a raw one". Setting `flags` directly fixed it.

That is the fourth harness-side failure this session mistaken for a port defect at
first sight — after the DS/file pairing, the labels prose, and the `[bp]`-defaults-
to-SS trap in #82. The reflex to check the harness before the subject is worth
more than any individual finding.

## FIX #85 — the save-name editor does not track its own length

Fifth twin differential. `0x1DD8` is the edit law: Enter commits unless the length
is zero, digits and lowercase only, fourteen characters, backspace steps back and
writes a SPACE. The port's `save_ui_key` matches it — verified key by key against
`func_1dd8`, comparing the committed name after each keystroke.

WHAT THE FIRST RUN EXPOSED, and it is a property of the ORIGINAL rather than a
harness slip this time. The lift reported `"b"` where the port had `"ab"`: the
second character had overwritten the first. `0x1DD8` reads the current length from
`[0x272E]` and stores at `[bx+si]` — but never advances it. A whole-image search
for writers finds exactly two: `mov word [0x272E],0` at `0x1BF3`, and an `inc` at
`0x1C05` inside a loop that SCANS the buffer counting characters up to a NUL or a
space.

So the editor does not maintain its own cursor. The surrounding flow re-derives it
from the buffer contents between keystrokes, which is why the buffer is
space-padded rather than NUL-terminated — a space is both "no character here" and
"stop counting". That also explains the shipped `blood.sav` reading
`"ab             \0"`, and why backspace writes `0x20` instead of truncating: it
IS the deletion, because the rescan then stops one character earlier.

The harness now models that rescan, and the two agree across typing, rejected
uppercase, the 14-character cap, and backspace. `save_ui_key` moves to `ORACLE`.

Recorded at `DS:0x272E` in labels.csv so the next harness does not lose an hour to
the same thing.

## FIX #86 — the last twin, and what the six of them cost and bought

`step_ship_3d_depth_scroll` against `func_b75c`, swept over 120 combinations of
depth, step and the two direction flags — including the values where `add al,
[0x2531]` being a LOW-BYTE add on a word actually shows (which is why the port has
`add_to_low_byte` rather than a plain addition). They agree everywhere. Two rows
to `ORACLE`.

That completes the twin campaign FIX #80 opened. Six differentials, and the tally
is worth stating because it argues for the technique:

| twin | outcome |
|---|---|
| `BloodPrng::next` (`0x2DE2`) | agreed over 300 vectors — the engine's own PRNG had never been tied to the oracle at all |
| `vm_field_offset` (`0x6023`) | agreed over the whole 336-cell matrix domain |
| special slots (`0x5FF6`/`0x5FD8`) | **DIVERGED** — the port cleared every match and dropped the carry; fixed |
| `square_caps_text_width` (`0x30CD`) | agreed on the real vocabulary; one latent divergence documented (unmapped chars) |
| `entity::advance_state` (`0x41D1`) | agreed over all 256 flag values; a stale `0x420D` citation corrected on the way |
| `step_ship_3d_depth_scroll` (`0xB75C`) | agreed over 120 combinations |

One live defect, one latent divergence, one wrong citation, and four confirmations
— from code that ALREADY had an oracle sitting beside it, unused. The premise
holds: a lift's verification does not transfer to a native twin, and the cheapest
verification available in this codebase is wherever ground truth already exists.

THREE HARNESS TRAPS, all of which first looked like port defects:
* `[bp]` operands default to **SS**, not DS (`0x5FF6`) — with a separate stack
  segment the lift operates on a list nobody can see;
* `EntityObject::populate` applies the ACTIVATION formula, so it cannot be used to
  seed a raw flags value;
* `0x1DD8` does not advance its own length — the caller re-scans the buffer, so a
  harness driving the editor alone overwrites position 0 forever.

The last of those is a genuine finding about the game, not just about testing: it
explains the space-padded save buffer, why backspace writes `0x20`, and the
`"ab             \0"` in the shipped `blood.sav`.

## FIX #87 — checking THIS session's own hand decode against the lift

The twin campaign verified code written by earlier sessions. This turns the same
instrument on work done today: `nav_chart_pick` was written by reading `0x92A3`
off the disassembly during FIX #52, and `func_92a3` is an oracle-verified lift of
the same routine. Running one against the other is an independent check of MY
reading.

Twelve probes — inside each marker, on the far edge, one pixel past it, in the
gaps between markers, off-chart, and the black hole's second endpoint — and the
hand decode matches everywhere. The per-kind hit boxes, the `(x-2, y-2)` origin,
the inclusive bounds and first-hit-wins all hold.

That is worth more than it looks. Every decode this session rests on my reading of
a disassembly listing, and the sessions's own tally says that reading is fallible:
four wrong instruction citations (#59, #79), a dead end that had the right answer
in it (#73), three harness traps mistaken for defects (#82, #84, #85). Where a
lift exists, the decode can be CHECKED rather than trusted — and this one survived.

Three rows to `ORACLE`. The remaining twins with lifts available are
`confirm_box_click` (`0x8295`), `build_active_object_list` (`0x604E`) and
`game_font_drawn_width` (`0x3192`) — all also decoded this session, all checkable
the same way.

## FIX #88 — the drawn-width accumulator, and two clip cells that are not what they look like

Second self-check: `game_font_drawn_width` (FIX #59) against `func_3192`. It
matches across every string the port measures — the four status headers, the
confirm dialog's three labels, the status overlays, `Oddland`, and the pathological
`"a b"` and `"  "` that exercise the space rule.

FOUND ON THE WAY, and it is a real correction to the port's understanding of two
globals. The first run had the lift returning 0 for everything. The routine's
guards are

```text
  0x31A1  cmp dx,gs:[0x523B] / ja      bail
  0x31AD  cx = gs:[0x5239] - 8
  0x31B0  cmp dx,cx / jle              bail
```

Both test DX — and DX is the ROW, not a dimension: `0x31BA..0x31C1` build the
framebuffer offset as `dx*320 + bx`, so BX is the column. `0x5239`/`0x523B` are
therefore the clip's TOP and BOTTOM, not a width and height. Seeding them as
`320`/`200` made every call bail before the first glyph, which is exactly the
symptom that exposed it.

`re/labels.csv` had them as "clip X to width / clip Y to screen height" from an
earlier pass. Corrected there too, since anything else running these blitters
would hit the same wall.

That is the fourth harness trap this session that turned into a finding about the
ORIGINAL rather than about testing — after `[bp]` defaulting to SS, `populate`'s
activation formula, and the save editor's re-scanned length.

## FIX #89 — every decode this session that had a lift is now checked against it

The self-check sweep is complete. Four routines decoded by hand this session,
each with an oracle-verified lift sitting beside it, each now differentialled:

| decode | lift | result |
|---|---|---|
| `nav_chart_pick` (FIX #52) | `func_92a3` | matches over 12 probes incl. edges and the black hole's second endpoint |
| `game_font_drawn_width` (#59) | `func_3192` | matches on every measured string; exposed the clip-cell correction |
| `build_active_object_list` (#53) | `func_604e` | matches, including the STOP at the first non-kind-1 entry |
| `confirm_box_click` (#68) | `func_8295` | matches over 11 cursor positions incl. both inclusive edges |

Nothing diverged. Four hand decodes, all correct — which is worth stating in the
same breath as the session's error tally (four wrong instruction citations, one
dead end containing the right answer, four harness traps), because it says the
errors cluster in the CITATIONS and the HARNESSES rather than in the logic. Where
the port reproduces behaviour, the readings held; where I wrote down an address or
built a test rig, they did not.

That suggests the cheapest remaining safeguards are the ones already built — the
instruction-citation guard catches the first class automatically, and the harness
traps are now documented at the globals they concern (`DS:0x272E`, `DS:0x5239`).

Twelve rows moved to `ORACLE` across the twin and self-check campaigns.

## FIX #90 — the tint builder was lifted all along, under a wrong name

`build_palette_blend_remap_table` is the port's hand reimplementation of `0x22E0`
— the table behind the info panel's window, the console choice box and (via its
twin) the montage. It had no oracle, and `cfg_clean.json` listed `0x22E0` among
the functions the fuzz oracle CANNOT verify, so it looked unverifiable.

It was lifted the whole time. `src/recomp/ptr_leaves_gen::func_22e0` exists and is
checked against the interpreter oracle — but its test describes it as
"3D-vertex projection ... writes one projected byte to gs:[DI]", which is the old
`abs_negate_gs_setup` guess: the first four instructions read, the rest assumed.
Searching for a lift by BEHAVIOUR found nothing; the lift was there under a name
for something else.

Run side by side over the real palette with the game's own arguments
(`ax=0xFFCE`, black target), the hand-written builder matches the lift byte for
byte across all 256 entries. Row to `ORACLE`, and both the test's description and
`labels.csv` now say what the routine is.

THE LESSON IS ABOUT SEARCHING, not about tables. A wrong name hides a function as
effectively as a missing one: I decoded `0x22E0` from scratch this session (FIX
#73's chain) while a verified lift of it sat in the tree. The cheap check I did
not do was "is this address lifted?" — by ADDRESS, which is stable, rather than by
what anything calls it.

## FIX #91 — the twin worklist, mechanised

FIX #90's lesson — a lift can hide under a wrong name, so match by ADDRESS — is
now a tool. `tools/check_liftable_twins.py` collects every `fn func_<hex>` across
the recomp modules and cross-references them against the audit ledger's origin
column, reporting which port items cite an address that already has oracle-verified
ground truth beside it.

Standing: 75 lifted addresses, 26 port items citing one — 12 already
differentialled, 14 candidates left. Those 14 need no decoding and no captures;
the ground truth is already in the tree.

Took one of them: `scan_zero_word` against `func_6293`, the length-0 opcode
advance. `0x6293` scans BYTE by byte for a word equal to AX, skips it, and skips
one more byte when its low half matches too — the byte-wise step matters, since a
word-wise scan would miss a terminator at an odd offset. Six streams including
that odd-offset case; they agree. Row to `ORACLE`.

A FIFTH HARNESS TRAP, and the same shape as the others. The first run diverged on
a stream ENDING in the terminator: the lift advanced one further than the native.
Cause — I wrote a zero guard word past the end of the buffer in memory so a
runaway scan would stop, and the lift dutifully consumed it under the
"skip one more zero" rule, while the native saw a slice that simply ended. Both
were behaving correctly on the inputs they were given; the inputs differed.
Appending the guard to the native's slice aligned them.

That is now five for five: every differential failure this session has been the
harness, never the port. Worth holding onto as a prior — but not as a certainty,
since the special-slot divergence (#82) was found by READING the disassembly, not
by a failing test.

## FIX #92 — the PRNG's seed, and the whole generator chain is now verified

`BloodPrng::seeded_from_rtc_seconds` against `func_2dd3`. `0x2DD3` selects CMOS
register 0 (`out 0x70,0`), reads the RTC SECONDS (`in al,0x71`), copies AL into AH
and stores the word at `cs:0xAEE` — so the seed word is the seconds byte doubled
into both halves. The port computes `seconds * 0x0101`; they agree, and the test
derives the byte from whatever the runtime models rather than hardcoding it, so it
follows the emulation if that changes.

With this the ENTIRE generator chain is oracle-verified end to end: the seed
(`0x2DD3`, here), the generator (`0x2DE2`, FIX #80) and its consumer the point
cloud (`0x9B67`/`0x9A10`, FIX #79). The starfield now rests on checked code from
the CMOS read to the plotted pixel.

Worklist: 13 differentialled, 13 candidates left.

## FIX #93 — an 8-bit truncation the port did in 16 bits

`LocationInfoPanel::entity_draw_scale` reproduced `0x9240`'s zoom scale as
`(3 * scale as u16 / 2 + 1) as u8`. The original is four instructions:

```text
  0x924B  mov al,3 / mul bh      ax = 3 * scale, a 16-bit product
  0x924F  mov bh,al              ...but only the LOW BYTE survives
  0x9251  shr bh,1 / inc bh      then /2 and +1, on the byte
```

The truncation happens BEFORE the shift. At scale 86 the original gives 2; the
16-bit form gives 130. The panel's zoom counter runs 0..8, so this was latent —
but the faithful form is no harder to write, and the sweep now covers all 256
values with the divergent case called out by name.

This is the SECOND divergence of exactly this kind, after the special-slot
removal (#82): both latent, both cheap to fix, both found by reading the
instructions rather than by any test failing. A differential over the reachable
domain would have passed either one.

`build_console_bank_remap_table` also settles to `ORACLE` here — not by
differential but by construction: it RUNS `func_242d`, so its output is the
game's code executing, not a reimplementation of it.

Worklist: 15 differentialled, 11 candidates left.

## FIX #94 — sweeping the widen-then-narrow pattern

#93's divergence was an 8-bit computation the port did in 16. That is a CLASS, so
I swept the tree for its signature — arithmetic widened to `u16` and narrowed back
to `u8` — and found six sites. Four are port-side representation conversions (6-bit
DAC to 8-bit RGB, which the game never performs at all) and one is test code.

The sixth is real: `snd_mix_average`, porting `0xBB6D..0xBB74`'s
`lodsb / add al,es:[di] / rcr al,1 / stosb`. Its doc ARGUES the 16-bit average is
equivalent, because the add's carry becomes bit 7 during the rotate. The argument
is correct — but #93 was a case where 8-bit and 16-bit arithmetic looked
interchangeable and were not, so the argument is worth replacing with a check.

The whole domain is 65536 pairs, so the sweep is exhaustive rather than sampled:
for every `(source, destination)`, the port's result equals a faithful
`add`-then-`rcr` simulation. Now checked, not reasoned.

The distinction matters for what the ledger MEANS. "The doc explains why this is
equivalent" and "every input has been tried" look similar in a code review and are
not the same claim.

## FIX #95 — the port manufactured a label the game never has

Sweeping for `saturating_*` in code that cites an address turned up
`square_caps_text_width`'s `.saturating_sub(2)`. The original is `sub ax,2` at
`0x30FE`, which WRAPS: an empty string measures `0xFFFE`, not 0. The lift confirms
it.

That looked like a trivial fidelity fix until the consumer decided otherwise. The
widget's max-width compare at `0x8472` is `cmp ax,dx / jb` — **unsigned**. So a
`0xFFFE` measurement would win the max and produce a box 65534 wide. The game
plainly does not do that, which means the game NEVER MEASURES AN EMPTY LABEL.

It does not, because unused save slots in `blood.sav` hold FIFTEEN SPACES, not an
empty string, and a space maps to glyph 47 with a real width. The empty case
cannot arise.

THE PORT MANUFACTURED IT. `parse_slot_directory` trims trailing spaces — reasonable
for a name — and FIX #66's `draw_save_ui_rows` then passed those trimmed strings
to the widget, so blank slots became `""`. The port's `saturating_sub` returned 0
and hid it. Two port-side choices, each defensible alone, combining into an input
the original never produces.

Fixed at the cause: the save rows are padded to the record width, as the directory
holds them. `square_caps_text_width` keeps saturating — with an unsigned return
type, wrapping would hand the widget a 65534 that the game's SIGNED-free unsigned
compare would honour. The divergence is documented at both the measure and the
differential, with the reason it is safe.

WHAT THIS SAYS ABOUT THE METHOD. The differential found the divergence; only
reading the CONSUMER showed which side was wrong. A test that had simply asserted
"port matches lift" here would have pushed a 65534-wide box into the renderer.

## FIX #96 — two rules, two copies each: deduplicated so verification transfers

The twin worklist's leftovers turned out to be duplicates rather than candidates,
and duplicates are how a verified rule stops covering the code that uses it.

**Two field-offset resolvers.** `vm_field_offset` (swept against `func_6023` over
the whole matrix domain, FIX #81) and `field_offset` — a second `bsf`-column
lookup differing only in returning `None` for a zero cell rather than `Some(0)`.
Both readings are usable, since the original returns `AX=0` and callers do
`or ax,ax / je`, but only one had been verified. `field_offset` now delegates and
keeps its `None`-on-zero contract as a filter.

**Two per-kind hit-box ladders.** `VmMachine::nav_chart_hit_box` (used by the
picker verified against `func_92a3`, FIX #87) and `NavChartObject::hit_box` (used
by the engine's click routing) each spelled out the same three kind tests. A box
that stopped matching the hit-test using it would misroute clicks with nothing
failing. Both now call one `nav_chart_hit_box_for_kind`.

Neither pair had actually drifted — checked before collapsing them. The point is
that verification attaches to a FUNCTION, and a second copy of the rule is outside
it by construction. Three rows inherit `ORACLE` by delegation rather than by a new
test, which is the cheapest verification there is.

## FIX #97 — a second copy of the font, carrying three already-fixed defects

Sweeping for addresses cited by more than one port FUNCTION turned up
`subtitle_draw_glyph` defined in BOTH `src/font.rs` and `src/extract/render.rs`.
The extraction module had its own private copy of the glyph struct, the lookup,
the advance and ALL THREE FONT TABLES — and that copy still carried three defects
the shared one had fixed:

* `GAME_FONT_CHAR_MAP` truncated to **128 entries** (the real table is 176 — the
  same truncation as FIX #56, in a third place), so every accented character was
  unmapped;
* indexed by `ch as usize`, a UNICODE SCALAR, where the table is indexed by CP437
  BYTE — so `é` (U+00E9 = 233) fell past the end even at 176 entries;
* unmapped characters fell back to `'?'`, where `render_string` (`0x31CE`:
  `xlatb / or al,al / js`) skips them with NO glyph and NO advance.

So the extraction/QA path rendered `?` where the game renders nothing, and mangled
accents twice over. Deleted — 115 lines of duplicated tables and accessors — and
delegated to `crate::font`.

Its test asserted the `'?'` fallback as correct behaviour, so it had been LOCKING
the defect in. Rewritten against `0x31CE`, plus an assertion that `é` now maps.

THE PATTERN, third instance this session: `bloodprg.rs` had the 128-truncation
(#56), `font.rs` had it before that, and `extract/render.rs` had it still. One
decode, three copies, fixed one at a time over three separate passes because
nothing connected them. The address-collision sweep is what finally connected them.

## FIX #98 — the duplicate-rule sweep, made permanent

Three duplications found this session, all the same shape: a decoded rule
implemented twice, with verification attached to one copy.

* `subtitle_draw_glyph` in `font.rs` and `extract/render.rs` — the second with a
  128-entry map, Unicode indexing and a `'?'` fallback (#97);
* two field-offset resolvers, one swept against `func_6023` (#96);
* two per-kind hit-box ladders (#96) and, here, two marker BOX TESTS — the VM
  picker's and the engine's click routing, each spelling out `(x-2, y-2)` with
  inclusive bounds. Collapsed into `nav_chart_marker_contains`, so the routing
  cannot drift from the hit-test verified against `func_92a3`.

`tools/check_duplicate_rules.py` now runs as a test. It clusters ledger rows by
cited address and FAILS on the strongest signal — the same function name in two
files — while reporting weaker collisions for judgement, since a routine and its
helper legitimately share an address.

Positive control: adding a second `subtitle_draw_glyph` in another module fails
with both files named.

That is six guards now, each built from a defect this session actually produced:
DS/file offset pairs, quoted instructions, `labels.csv` validity, capture
provenance, self-referential assertions, and duplicated rules. The through-line is
that every one of them encodes a mistake I made or found, so the next pass cannot
repeat it silently.

## FIX #99 — the owner lookup, verified and de-duplicated

`owner_object_offset` against `vm_record_lookup_by_threshold` `0x6034`:

```text
  0x603B  cmp ax,[si+0x10] / jbe    advance while the entry offset is BELOW ax
  0x6040  add si,0x14
  0x6045  sub si,0x14               step BACK one entry
  0x6048  mov ax,[si+0x10]          and return ITS object offset
```

The port's `.rev().find(|o| o < off)` is the same answer over an ascending list.
ONE DELIBERATE DIFFERENCE, now written down: when `off` is at or below the FIRST
entry, the original's `sub si,0x14` steps in front of the table and returns
whatever lies there. The port returns `None` rather than reproducing an
out-of-bounds read.

AND IT WAS WRITTEN TWICE — identical bodies in `ExecutionContext` and in
`VmMachine`, both in `src/vm.rs`. The duplicate-rule guard (#98) missed it because
it compared names ACROSS FILES, and Rust allows one name per impl block. The
same-file case is now in scope and noted in the tool's source; both call one
`owner_object_offset_in`.

Fourth duplication this session, and the second whose two copies sat in one file.
Each guard has needed widening once it met a case its author had not pictured —
which is an argument for building them from real defects rather than from
imagination.

## FIX #100 — the disassembler prints `cwde` where the CPU does `cbw`

Verifying the `0xD2` handler (`0x64B8`: `lodsb / cbw / dec ax / mov gs:[0x6780],ax`)
against the port's `(operand as i8 as i16) - 1` produced an apparent contradiction:
`re/tools/dis.py` shows the middle instruction as `cwde`, not `cbw`, and the two
are NOT interchangeable here.

* `cbw` sign-extends AL into AX — so the stored word is the operand, sign-extended,
  minus one. That is what the port computes.
* `cwde` sign-extends AX into EAX and leaves AX's low half alone — so after
  `lodsb`, AH still holds whatever the dispatcher left, and the stored value would
  depend on CALLER STATE rather than on the operand at all.

The port is right. Opcode `0x98` in 16-bit mode with no `0x66` prefix IS `cbw`;
capstone simply prints it as `cwde` in every mode. Confirmed directly: capstone
renders the byte identically under `CS_MODE_16` and `CS_MODE_32`.

Two consequences, both now handled. `re/tools/dis.py`'s header records the quirk
(and its sibling, `0x99` printing as `cdq` where it is `cwd`), because a listing
that says `cwde` will otherwise mislead anyone reading it. And
`check_cited_instructions.py` treats the pairs as equivalent — without that, a
comment quoting the ARCHITECTURALLY CORRECT `cbw` would have been reported as a
wrong citation, which is the guard punishing accuracy.

Profile operands are 1..5, so the sign extension is a no-op in play. The reading
still has to be right, and a tool that renders one instruction as another is worth
knowing about before it costs someone an afternoon.

## #101 — the quirk had already spread: 7 wrong mnemonics and a phantom instruction

Knowing capstone renames `cbw`→`cwde` (#100) raised the obvious question: how many
notes had already copied the tool's spelling instead of the architecture? Sweeping
for it found EIGHT sites — but the interesting one was not a mnemonic at all.

`re/tools/check_opsize_mnemonics.py` settles each citation from the bytes, because
the two mnemonics are the same opcode at different operand sizes and the tell is
INSTRUCTION LENGTH: a bare `0x98` is one byte (`cbw`), a prefixed `66 98` is two
(a real `cwde`). Seven citations (six in `labels.csv`, two doc lines in `vm.rs`)
called a one-byte `0x98` `cwde` and were corrected. Crucially the sweep also
CLEARED `0x379B`, which really is `66 98` — a genuine 32-bit `cwde` feeding
`mov ebx,eax`. The check discriminates; it does not blanket-rewrite.

The eighth was worse than a spelling error. The row

    0x00B142,audio_helper_b6dd,"audio helper: cdq; call 0xb6dd..."

cites an address that is not an instruction boundary at all. `0xB142` is INSIDE
`9a cb 0e 99 02` = `lcall 0x299:0x0ecb` at `0xB140` — the `cdq` was the `0x99`
byte of the far call's SEGMENT `0x299`. The real stream is `0xB145 pop ds /
0xB146 push cs / 0xB147 call 0xb6dd`: the push-cs-then-near-call idiom for
entering `ship_3d_plane_band_copy`, which ends in `RETF`. Nothing audio about it.
A label anchored mid-instruction is more dangerous than a wrong mnemonic, because
every other claim it makes about the routine is suspect too.

Finding that required fixing the checker first. Walking forward from a misaligned
address makes capstone resynchronize into a phantom stream, so the tool now
requires BOUNDARY CONSENSUS — the instruction must also appear when decoding from
several earlier anchors. x86 is self-synchronizing, so a correctly aligned entry
decodes the far call as one 5-byte instruction and the phantom `cdq` vanishes.
Misalignment is reported as its own class.

One more iteration was needed for the same reason as #100's alias table: the first
tightened run flagged the `vm.rs` line that DOCUMENTS the trap ("`dis.py` prints it
`cwde`"). A guard that punishes the note warning the next reader is worse than no
guard, so lines describing tool output are exempt.

Registered as a lib test. Also removed the dead `GAME_FONT_SPACE_ADVANCE` copy in
`extract/render.rs` (`font.rs` owns that rule and its `0x31D7` citation) and made
the file's `game_font_glyph` alias `#[cfg(test)]`, so an unwatched non-test copy
cannot drift back in — the duplication class from #97.

## #102 — the remap primitive clips vertically against the WRONG global, and the port must too

Chasing a dead-code warning in `extract/render.rs` turned into a real divergence.
`sprite::remap_rect_indexed` clamped its rect to the framebuffer, and its doc
claimed that matched `0x33B2..0x33F4`. It does not. The ladder clips against the
CLIP-WINDOW globals `DS:0x5235..0x523B` — and its vertical check is

    0x33E6  mov ax,cx / add ax,bp / sub ax,[0x5237]

`cx` is y and `bp` is height (pinned by the address math at `0x33FA`:
`ax=y; xchg ah,al; cx<<=6; ax+=cx; ax+=bx` = `y*320+x`). So the VERTICAL extent is
clipped against `DS:0x5237`, the HORIZONTAL RIGHT bound. Every other clip ladder
in the binary uses `DS:0x523B` for the bottom — the point plot at `0x9B04` checks
x against `0x5235`/`0x5237` and y against `0x5239`/`0x523B`, and the string row
draw at `0x3437` guards with `cmp dx,gs:[0x523B]`. This one is a bug in the
original.

It survived because `right` (320) exceeds `bottom` (200), so on a full-screen
window the vertical clamp never fires for a rect that fits on screen — which is
every rect the game asks it to tint. It becomes observable under a LETTERBOX
window: `0x33E2` still pulls y down to the top bound while the bottom is left
effectively open, so a tall tint runs past the letterbox floor. A port that
"fixed" this would draw a shorter box than the game does.

So the ladder is transcribed verbatim, with the clip window as an explicit input
(`ClipWindow`, the four globals) instead of an assumption baked into the loop. The
one deliberate divergence is memory safety: the game writes `y*320+x` unbounded,
so the quirk lets it run past the visible page; the port stops at the buffer.

The dead twin in `extract/render.rs` implemented the sensible-but-wrong reading,
clipping vertically to `clip_bottom` — and `render.rs` had a TEST asserting that
behaviour. A test agreeing with a fabricated rule is the shape from #97. The copy
is deleted and the test now asserts the real thing: the row past `bottom` IS
remapped. The new test discriminates — a bottom-clamping implementation fails it.

`check_cited_instructions.py` caught one bad citation in the new doc block
(`0x33BA` is the `add`, not the `jns`) before it was committed.

### The ledger could not see 33 rows it already had evidence for

Settling the new function was REFUSED for want of a cited address, though its doc
is nothing but citations. `audit_inventory.py` cleared the pending doc comment on
any line that was not a comment or an item declaration — including ATTRIBUTES, so
`#[allow(clippy::too_many_arguments)]`, `#[derive(...)]` and `#[inline]` stripped
the item below them of its origin, permanently. It also harvested addresses only
from the 400-character evidence truncation, so a long transcription's citations
fell off the end.

Both fixed: attributes no longer break the association, and addresses come from
the whole doc while the evidence column stays truncated for readability. 33 rows
regained an origin they always had — the same shape as constants being left out of
the ledger entirely, and the reason the denominator moved once before.

## #103 — checking constants against the binary, and the checker clearing them tautologically

Constants are the most directly checkable claims in the port: `MENU_REST_FRAME =
0x2C` citing `0x8642` either has a `0x2C` at that address or it does not.
`tools/check_cited_immediates.py` disassembles each cited address and looks for
the value. 24 constants were settled on that evidence.

It is a CLASSIFIER, not a guard, because plenty of correct constants are not
immediates: an opcode constant cites its HANDLER and the value is a dispatch-table
index appearing nowhere in the handler's bytes; a stride can be a shift count
(`DLG_ASSET_NAME_STRIDE = 0x10` is `shl ax,4` at `0x768E`). Those are reported as
NEEDS READING, never as defects — a tool that called them wrong would train the
reader to ignore it.

Three iterations, because the first two versions cleared constants they should
not have:

* **Masked immediates matched by coincidence.** Every immediate was compared at 8
  and 16 bits, so `GAME_FONT_WIDTH = 8` "verified" against `add ax,0x808` and an
  entry count of 8 against a `[di+8]` displacement. Only the immediate itself
  counts now, plus its truncations when NEGATIVE — where capstone reports the
  sign-extended form of a byte the code really contains (that is what legitimately
  matches `DLG_LINE_ASSET_NONE = 0xFFFF` to `cmp si,-1`).
* **A rule that cleared constants with their own doc comments.** "Value equals the
  cited address" was meant to recognise a table base. What it actually matched was
  the constant's value appearing in its own doc: `DLG_LINE_ASSET_NONE = 0xFFFF`
  passed because the doc calls `0xFFFF` a sentinel, and the address regex read the
  sentinel as an address. That is precisely the self-referential shape
  `check_selfref_asserts.py` exists to catch, written into a new tool ten minutes
  after being reminded of it. Removed. All eight constants it had been clearing
  turned out to have real instruction evidence anyway.
* **Small values are weak evidence** and are marked as such, since a one-digit
  number matches something almost anywhere.

Settled: 463 -> 487 of 2143.

## #104 — opcode constants verified through the dispatch table, and 20 given the citation they lacked

`check_cited_immediates.py` could only report the `OP_*` constants as NEEDS
READING: an opcode's value is a table INDEX, so it appears nowhere in its
handler's bytes. But the claim their docs make is checkable, and it is a stronger
claim than an immediate match —

    /// `0xA4` unconditional JUMP (PC = operand). Handler 0x65db.

says entry `0xA4` of the VM dispatch table points at `0x65DB`. The table is at
file `0x142D0` (`vm_opcode_handler_table_static`, copied to `GS:0x6EB0` at init):
52 near offsets into VM code segment `0x4DA`, covering `0xA0..0xD3`, so
`handler = 0x600 + 0x4DA*16 + table[op - 0xA0]`. `tools/check_opcode_handlers.py`
resolves every constant that way.

All 9 existing handler citations are correct. Twenty more constants cited no
handler at all, so the table supplied one — decoded from the binary, not
transcribed from anywhere — and each resolved to a handler ALREADY LABELLED in
`re/labels.csv` under a matching name (`0xD2` → `vm_op_d2_script_profile_request`,
`0xC4` → `vm_op_c4_actor`). Independent corroboration: the table and the labels
were derived separately and agree. `0xB8`, `0xB9` and `0xBD` all dispatch to the
same handler `0x6B06`, which is why they are the "pair record" family.

Two checker bugs first, both of the same kind — reading more into a doc than it
says:

* Not clearing the doc run between constants let every constant inherit the GROUP
  comment above the block, so 18 of them "cited" the same five addresses and all
  18 looked wrong.
* Treating every address in a doc as a handler claim flagged `0xCE`, whose doc
  cites the game-flag words `[0x2793]`/`[0x252a]` — it never claimed those were
  handlers. Only addresses introduced as "Handler 0x..." count now.

`OP_MAX = 0xFE` is correctly outside the table's range: it is the TOKEN bound, not
the dispatch bound — the distinction recorded in commit aa8a3b8.

Registered as a lib test. Settled: 487 -> 515 of 2143.

## #105 — layout identities are evidence too, and one ledger "improvement" was measured and reverted

`DIALOGUE_FONT_ASCII_MAP_LEN = 176` appears as an immediate nowhere, because it is
not one: the map runs from `0x14C22` to the advance table at `0x14CD2`, and
`0x14CD2 - 0x14C22 = 0xB0`. That LAYOUT IDENTITY is how the 128-vs-176 truncation
was settled in the first place, and it is mechanical, so
`check_cited_immediates.py` now recognises a value that equals the distance
between two addresses the same doc cites.

Two ledger blind spots, one fixed and one rejected after measuring it:

* **Plain `//` comments were not read as evidence.** Only `///` and `//!` counted,
  so a constant declared inside a function body -- where a doc comment on a local
  item is unidiomatic -- could never carry an origin. The three `TEXT = 0xE8`
  colour constants in `engine.rs` are each documented with a `//` citation
  (`mov al,0xE8` @`0x14F7` and @`0x8565`), all three verified against the
  disassembly, and none of them settleable. Fixed: 334 -> 367 rows with an origin.

* **Letting a doc run carry across consecutive items was NOT kept.** It looked
  like a large win -- 367 -> 594 rows with an origin -- and the motivation was
  real: `const TEXT` and `const TEXT_SELECTED` share one comment block. But
  sampling the result showed most of the gain was BLEED. A long run of
  `pub const` lines in `bloodprg.rs` handed the font map's "176, NOT 128" doc to
  `RENDER_SPRITE_BLIT_RAW_OPAQUE_OFFSET` and `SHIP_3D_TRANSITION_STATE_OFFSET`,
  which have nothing to do with it. An origin asserts that a row IS evidenced, so
  a false one is worse than a missing one: it makes a row look settleable and
  invites settling it on someone else's citation. Reverted; the grouped constants
  got their own comments instead, which is the honest fix and costs three lines.

`audit_settle.py` also grew a `file:line:item` form. It correctly refuses to
settle a name that is not unique in its file -- settling the wrong function is the
failure mode it exists to prevent -- but it had no way to disambiguate, so three
identically-named local constants were unsettleable no matter how well evidenced.

Settled: 515 -> 522 of 2143.

## #106 — two more evidence shapes, and the checker was reading the WRONG BINARY

Working the remaining NEEDS-READING constants turned up two more mechanical
shapes and one correctness bug in the checker itself.

**A constant that names an operand's ADDRESS.** `*_IMMEDIATE` constants do not
hold a value the game uses — they hold the file offset of one, so the port can
read or patch it. `and bx,0xe` at `0x44B7` is `83 e3 0e`, so its imm8 lives at
`0x44B9`; `mov di,0x2fc1` at `0x9B71` puts its operand at `0x9B72`. Verifying
that means decoding the instruction that CONTAINS the offset. First attempt
cleared three constants against PHANTOM instructions — `0x44B9` "matched" a
`jcxz` at `0x44B8` because decoding from an arbitrary earlier byte
resynchronises. Same disease as the misaligned label in #101, same cure: require
the containing instruction's start to be a boundary independent anchors agree on.
With that, `0x44B9` resolves to the real `and bx,0xe` and the phantoms are gone.

**The SUM identity.** `WORLD_ART_TABLE_FILE_OFFSET = 0xFFE7` is `0xD420 + 0x2BC7`
— the DS base plus a DS-relative offset, i.e. the file offset of `DS:0x2BC7`.
The difference form (#105) covers table lengths; this covers address conversions.

**The bug: overlay addresses resolved against BLOODPRG.EXE.** `croolis.rs`
documents the alien OVERLAYS (`croolis.xdb`, `amer.xdb`, `scrut.xdb`), and
`manu3.rs`/`manu3_hand.rs` likewise. Their cited offsets are into those overlays,
so looking them up in the main image makes every match a coincidence and every
miss meaningless: `ALIEN_POSITION_WRAP` cites method `0x999`, which is
mid-instruction garbage in BLOODPRG.EXE. The checker now detects an
overlay-documenting module header and skips the file, saying so.

Nothing had been settled wrongly on that basis — the only two candidates from
those files were cleared by the tautological rule removed in #103, and no
`croolis.rs` row is settled at all. Verifying them needs `re/tools/dis_xdb.py`
against the overlay, which is a task, not a blocker.

Settled: 522 -> 528 of 2143.

## #107 — a "probe dump" constant that was static in the image all along

`STATION_REST_FRAMES` cited its own provenance honestly and wrongly:

    /// Station rest frames observed in the live table (`BRIDGEPROBE` dump of
    /// `DS:0x2A1B` at the console: targets 0x000/0x05A/0x0B4/0x10E doubled)

A runtime memory dump is better than a screenshot — it reads the game's own data
rather than pixels — but it is still an observation standing in for the binary,
and the prime rule wants the binary. So: is `DS:0x2A1B` static?

The first look said no. `DS:0x2A1B` is file `0xD420 + 0x2A1B = 0xFE3B`, and those
bytes are `0x0001, 0x0011, 0xFFFF, 0, 0...` — nothing like the probe's values.
That looked like a table built at runtime, which would have made the dump the only
available source.

It was the wrong question. Cross-referencing the address found the walker at
`0x7DAB`:

    mov bp,0x2A1B / mov cx,6 / ... / add bp,0x18 / loop 0x7DAE

— SIX records of 0x18 bytes, dispatched through `call word cs:[bx+0x6D4]`, with
`0x9642` clearing the array the same way and `0x985F` filling `+0xC..+0x1C` with
`0xFFFFFFFF` (four words: the orb box, matching `StationRecord::orb_box`). The
rest angle is field `+0xA`, so the first dump had only covered record 0 — whose
angle is 0. Reading `+0xA` across all six records gives

    0x000, 0x05A, 0x0B4, 0x10E, 0, 0   at 0xFE45, 0xFE5D, 0xFE75, 0xFE8D

exactly what the probe saw. The data was static the whole time; the dump was
reading the file back to itself.

The constant now cites the static table, the record layout is decoded into named
constants (count, stride, field offset), and
`station_rest_frames_match_the_static_record_table` reads the bytes out of
BLOODPRG.EXE and checks that each frame is half its recorded angle (2° per
panorama frame over 180 frames). `re/tools/ds_dump.py` was added to make "what is
actually at this DS address" a one-liner, since that question is what separates a
decoded constant from an observed one.

`CONSOLE_REST_FRAME = 55` in `tbbig.rs` is the remaining constant of this kind
(`observed via BRIDGEPROBE`). It is TEST-ONLY — nothing in the port's render path
reads it — so it is a fixture, not a behaviour defect, and it is not one of the
four station rest frames. Left as `ORACLE?` deliberately.

Settled: 528 -> 533 of 2147.

## #108 — game text hardcoded in the port: Bob's greeting, the console prompt, and two menus

Widening the provenance guard to catch "the captured <noun>" as a noun phrase
found a stale note in `main.rs` — the bridge comment still described the hand
cursor as "the captured pointing-hand cursor" and told the reader to
"regenerate with runtime_boot BRIDGEPROBE HANDATLAS" long after that atlas was
deleted and the hand became `manu3_hand::HandMesh`, decoded from `manu3.xdb`.
A stale note is not cosmetic here: it names the wrong SOURCE and hands the next
reader a command to restore the wrong thing.

The same sweep then found real defects. `main.rs` carried GAME TEXT:

    "HONK! You worthless heap of wires... Are  \nyou working?"   // Bob's greeting
    "What do you want Commander ?"                              // the console prompt
    vec!["TALK", "REMEMBER", "BYE_BYE"]                          // the HONK menu

each as a "no-VM fallback" for content SCRIPT2's bytecode already provides
(Bob is record 132 rel 40). This is the defect the prime rule names outright, and
the menu list is its worked example: conversation menus come from the `0xA6` line
records' `0xFFFF`-separated word lists, executed by the VM, not from lists
transcribed off the screen. All three are gone. If the VM yields nothing, the
contact does not open and the box is empty — inventing a line is worse than
showing none.

`tools/check_content_literals.py` now guards the class, and building it was mostly
learning to tell the port's own prose from the game's. The first run reported 102
"findings", nearly all of them the `BinarySymbol` label table describing DS
globals and match arms classifying addresses — documentation carried as data.
Vocabulary heuristics helped and were the wrong instrument; what worked was
STRUCTURAL exclusion: a `comment:`/`kind:`/`name:` field, and a match arm yielding
a string, are documentation by construction. 102 -> 1, and the 1 was real.

### Two label lists remain, and they are now tracked rather than silent

    engine.console_box = vec!["BOB_MORLOCK".into(), "CANCEL".into()];       // row 2
    vec!["TEXT", "MUSIC_OFF", "SAVE", "LOAD", "QUIT", "CANCEL"]             // OPTION

These are NOT fallbacks — they run unconditionally. Every one is a DIC word in the
game's own dictionary (`Bob_Morlock` in SCRIPT2/3/4/5.DIC, `music_off`, `save`,
`load`, `text` across SCRIPT1..5.DIC), so the content lives in the data. The port
even has the parser already — `bas_vm::parse_menu_block` walks a `0xA3` menu
head's word list and its `0xA6` responses — and it is called from NOWHERE.

Fixing them needs the menu record each console row opens, which is a decode, not a
patch. Both are recorded in `docs/port-validation.md` as OPEN and carry an entry
in the guard's `KNOWN_OPEN`, so the class cannot grow while they are unfixed: a
NEW hardcoded label list fails the check. Verified the detector actually fires on
them with the allowlist disabled — an allowlist that hides a detector that never
worked would be worse than no guard.

## #109 — the contact menu is whoever is aboard, not two words in a file

One of the two open label lists from #108 is closed, by decoding what the console
row actually does rather than patching what it shows.

The bridge click at `0x86A4` sets `[0x2A19] = row+1`, ORs `0xC` into the game-flag
word `[0x2793]` — the word opcode `0xCE` branches on, so console rows are
script-visible — places a choice box at `x=0x64` (the already-decoded
`CHOICE_BOX_CENTER_X`) and `y = row*0x12 + 0x50`, then dispatches:

    0x8700  call word ptr cs:[bx+0xf29]      bx = row*2

CS here is segment `0x071E` (base file `0x77E0`), NOT the `0x299` that carries the
blit family — so the table is at file `0x8709`, immediately after this routine's
`ret` at `0x8708`, which is exactly where a local jump table belongs. Entries
`0x0F33, 0x0F4C, 0x0FDD, 0x1068, 0x108C, 0x06F6` give handlers `0x8713`, `0x872C`,
`0x87BD`, `0x8848`, `0x886C`, `0x7ED6`.

Row 2's handler `0x87BD` is the contact menu:

```text
  mov si,0x6D3E          the 16-entry ship-slot array
  mov di,0x2B13          the menu word list
  lodsw                  next slot
  or ax,ax / je          EMPTY slot -- skip, do not emit a blank
  cmp ax,-1 / je         0xFFFF terminates
  add ax,4 / stosw       emit RECORD+4 -- the object's INLINE NAME
```

`DS:0x6D3E` reads as all zeros in the image (file `0x1415E`), which at first looks
like a dead end — but it is runtime state, and the port ALREADY MODELS IT. The
insert, find and remove scans at `0x5FD8`, `0x5FF6` and `0x6008` all walk it with
`mov cx,0x10`, and those are `VmMachine::ship_slots`. The `+4` is
`object_inline_name`, also already decoded and checked against the shipped data
(630 of 640 kind-1 objects hold their DEB name there).

So every piece existed; nothing connected them. `ship_contact_menu_words` is the
handler transcribed, and `main.rs` calls it. The menu is now whoever is aboard,
named from their own record — and EMPTY when nobody is, because the port does not
invent entries. The test pins all three rules: empty slots skipped rather than
emitted blank, a live slot behind the `0xFFFF` sentinel not emitted, names read
from `record+4`.

Removed from the guard's `KNOWN_OPEN`, so the hardcoded pair cannot return. The
OPTION menu (`TEXT / MUSIC_OFF / SAVE / LOAD / QUIT / CANCEL`) is still open and
still listed.

## #110 — the OPTION menu was in the game's data too, and it is richer than the copy

The second hardcoded label list is closed the same way as the first: by reading
the game's own table.

Console row 4's handler `0x886C` (same per-row table, entry 4 = `0x108C`) does
`mov si,0x2567` and calls the list widget at `0x8428`. `DS:0x2567` (file
`0x0F987`) is a `0xFFFF`-terminated list of DS pointers into NUL-terminated
strings that sit immediately after it:

    DS:0x2573 TEXT   0x2581 MUSIC_OFF   0x258B SAVE   0x2590 LOAD   0x2595 QUIT

Reading the table instead of copying it turned up two things the transcribed
version had flattened:

* **`CANCEL` is not part of this menu.** It is `DS:0x0174`, the list widget's
  SHARED trailing entry — already labelled `ship_3d_target_extra_label`, because
  the ship-3D target list appends the same string. The port had it as a sixth
  peer of `QUIT`, which hides that it belongs to the widget, not the menu.
* **`MUSIC_ON` exists, at `DS:0x2578`** — between `TEXT` and `MUSIC_OFF`, and
  deliberately ABSENT from the pointer list. It is the toggle's other face,
  swapped in by state. A six-string copy of the menu can never show `MUSIC_ON`,
  so the port's music toggle could only ever have rendered one way.

`option_menu_labels` walks the pointer list, `list_widget_cancel_label` and
`music_on_label` read the two strings the list omits, and the test checks all of
it against the shipped binary. `main.rs` reads them once at startup.

The guard's `KNOWN_OPEN` is now EMPTY: both lists that lived there are read from
data, so any new hardcoded label list fails the check outright.

One more list is visible and not yet wired: `DS:0x259D` points at a text-speed
submenu (`VERY FAST`, ...). Recorded in `docs/port-validation.md` rather than
left as a surprise.

## #111 — a wrong subtitle speed, and an oracle failure my own grep had been hiding

### The text-speed step

`DEFAULT_SUBTITLE_TEXT_SPEED_STEP = 5`, documented as "the default/mid step
observed in the binary-derived notes". Both halves of that are wrong. The image
ships `DS:0x0ACA = 2` (file `0x0DEEA`), and 5 is not a value the game can produce
at all: the writer at `0x1B29..0x1B3D` maps the five OPTION labels through

    add ax,ax / cmp ax,8 / (add ax,4) / shr ax,1 / inc ax

giving steps 1, 2, 3, 4, **7** — `VERY SLOW` jumps because of that `cmp ax,8`.
The shipped default of 2 is FAST, the second entry, not the middle one. Since
`subtitle_reveal_chars_per_second` divides by the step, exported subtitles
revealed at 2/5 of the real rate.

Two extract tests pinned the old timings. They are synthetic (text `"abc"`, a
fabricated clip), so updating them is legitimate — but one of them hardcoded
`1.25` where its SIBLING assertion was already written derivationally
(`2.0 + 2.0/rate`). The hardcoded one silently encoded the wrong constant, so it
is now derived too.

While decoding this I wrote `bloodprg::text_speed_step_for_index` — and
`vm::text_speed_step_from_setting` already existed, transcribing the same
addresses including the `cmp ax,8` case. A twin, of exactly the kind
`check_duplicate_rules.py` guards, created by me. Deleted; the test-name collision
is what surfaced it.

### The list menu does NOT centre its labels

Running the suite to completion exposed `concept_menu_text_matches_live_game_capture`
failing at IoU 0.183 — and it was failing BEFORE this session's changes. My own
`grep -E "^test result" | head -3` had been cutting off before `oracle_suite`
printed, so "93 integration green" was a claim about the first three test
binaries, not the suite. Corrected here rather than quietly.

The failure is real and it indicts an earlier "fix". The port centred each label
per `0x857D` (`sub bx,[bp] / shr bx,1 / add bx,cx`), replacing a flush `x = 170`
that had been capture-measured. The behaviour was right and the constant was
wrong — but the fix changed the behaviour too. Measured per row against
`concept_menu.ppm`:

    row  0 TALK          port x=206   live x=170
    row  4 END_OF_MONTH  port x=170   live x=170
    row 10 HOW           port x=211   live x=170

Both masks span x 170..280 identically with 1405 vs 1379 pixels — correct band,
wrong placement inside it, which is precisely the IoU-0.18 signature. The list is
flush-left; only the widest label happened to match.

Now `lx = x0 + 10`, DERIVED from the concept anchor `0xE1` (@`0x89A6`) and the
widest label — which evaluates to 170 here, so the hardcoded constant did not come
back. The lib test that asserted centring is replaced by one asserting a shared
left edge at the formula's x. Which widget `0x857D` centres is recorded as open.

The diagnostic that settled it (`concept_menu_mask_bounds`, `#[ignore]`) stays in
the tree: an IoU number says how wrong, and this says WHICH WAY.

## #112 — seven captures nothing read, and the formula confirmed at two values

The concept-menu error survived because nothing compared the port against that
screen until the comparison was written. So: how much captured ground truth is in
the tree with no test reading it? Seven files —

    console_band.idx      honk_talk_menu.ppm      mission_briefing_eye.ppm
    nav_screen_opened.ppm post2_menu_choice.ppm   psychotherapy_topics.ppm
    script2_first_frame.ppm

Two are menu screens, and measuring them settles #111 independently:

    psychotherapy_topics.ppm   12 rows @ 11px pitch   text starts x=170   widest span 111px
    honk_talk_menu.ppm         10 rows @ 11px pitch   text starts x=173   widest span 105px

Two DIFFERENT left edges, and the decoded formula predicts both:
`anchor 0xE1 - (widest+20)/2 + 10` gives 170 for widest 110 and 173 for widest
104. A centring implementation cannot satisfy either — it places only the widest
row at that x. The new test imports nothing from the captures except two
measurements (where text starts, how wide the widest row is), so it verifies the
decode instead of copying a layout out of it.

`tools/run_tests.sh` now runs the whole suite with `--no-fail-fast` and prints
every binary's result plus a total. The truncated-grep habit that hid the
concept-menu failure cannot repeat: 635 tests across 10 binaries, stated as one
line.

Five captures remain unread (`mission_briefing_eye`, `nav_screen_opened`,
`post2_menu_choice`, `script2_first_frame`, `console_band.idx`). Each is a screen
the port renders and nothing checks — the same condition that hid this one.

## #113 — reading three more of the unread captures, and guarding a headline decode

Continuing #112's list. Three of the seven now have tests:

* **`post2_menu_choice.ppm`** verifies the CHOICE BOX anchor. Its single text row
  spans x 73..130, centre 101 — the decoded `0x64` (`mov [0xAC6],0x64` @`0x86D9`),
  not the concept list's `0xE1`. The two anchors are 125px apart, so the check
  cannot pass with the wrong one. Only the centre is taken from the image.
* **`honk_talk_menu.ppm`** and **`psychotherapy_topics.ppm`** were done in #112.

* **`console_band.idx`** is the important one. The claim that the intro montage's
  console band IS `TB.BIG` frame 90 rows 140..200 pushed through the console-bank
  remap — that the band was never separate art — is a headline decode, and it had
  been proven ONCE, BY HAND. No test read `console_band.idx`. Changing the frame
  index or the remap builder would have gone unnoticed, which is exactly how the
  list-menu centring error survived. The proof now runs every time: all 19200
  bytes, zero differing.

That is the pattern worth naming. A decode that was verified once and then left
unguarded is indistinguishable, later, from a decode that was never verified —
and the more impressive the finding, the less likely anyone re-checks it.

Still unread: `mission_briefing_eye.ppm`, `nav_screen_opened.ppm`,
`script2_first_frame.ppm`. The nav screen resisted a clean geometric check (it is
a detailed starfield, so row-transition heuristics find edges everywhere rather
than a band boundary); it needs a real render comparison, not a measurement.

## #114 — the bridge windows are black, and the capture that shows it had never been read

`nav_screen_opened.ppm` is the fourth of #112's seven unread captures, and reading
it took three attempts — two of which were my setup being wrong, not the port.

First attempt: `on_ship` nav view, mean_abs 105. Rendering it as ASCII showed the
port drawing full-screen `CHART.FD` while the capture showed a starfield over a
console band. Second: stepping 180 ticks to let the ship-3D transition arm changed
nothing at all. Third: the capture is in captures/**bridge**/, and the starfield
belongs to `render_bridge_background` — so the screen is the BRIDGE at the nav
station, not the `on_ship` nav view. Comparing against the bridge at
`STATION_REST_FRAMES[2]` (frame 90, the pyramid nav room) dropped the non-zero
pixel count from 63987 to 29541 and mean_abs to 102.

Still far, and the ASCII says why: the port draws the panorama with BLACK windows
where the live game shows stars. `render_bridge_background` composites in the
right order — starfield first, then the panorama with colour 0 transparent so the
windows show through — but the star layer contributes nothing on this path, so the
transparent windows reveal black.

This is a WIRING gap, not a decode gap, and that distinction is the useful part.
The point cloud is fully decoded and settled: `SHIP_3D_POINT_CLOUD_LEN` = 1000
(`mov cx,0x3e8` @`0x9B6A`), base `DS:0x2FC1` (`mov di,0x2fc1` @`0x9B71`),
`randomize_ship_3d_point_cloud` at `0x9B67`, the projection and the depth shading.
All of it exists. Something between it and `render_bridge_background`'s non-GPU
branch does not connect — the same shape as `bas_vm::parse_menu_block` sitting
unused while `main.rs` hardcoded the menus it could have built (#108).

NOT asserted with a tolerance. A threshold chosen to accommodate mean_abs 102
would encode the bug as the expected result, which is how a fabricated rule gets
protected by its own test (#111). The measurement stays `#[ignore]` until the star
path is connected, at which point it becomes a real comparison. Recorded in
docs/port-validation.md as OPEN.

Two captures remain unread: `mission_briefing_eye.ppm`, `script2_first_frame.ppm`.

## #115 — WITHDRAWING #114: that capture is static, not stars

#114 is wrong and is withdrawn. It claimed the bridge's windows should show a
starfield and render black in the port, at mean_abs 102 against
`nav_screen_opened.ppm`. The capture is not a bridge starfield.

Its top 135 rows hold exactly two colours — black (23315px) and white (19855px) —
in per-pixel noise, mean run length 1.87px across a row. That is the binary STATIC
of the presentation/boot screen, with the console band below it. The screen was
never the bridge.

The measurement that should have stopped me was in the same output: the port's
star layer plots **33 pixels**, and a 1000-point cloud plots at most ~1000. The
capture has 19855 white pixels. Nothing that renders 1000 points can produce that
image, so "the starfield is not wired up" could not explain the difference — the
arithmetic ruled it out before the conclusion was written. I read a dense white
field, recognised it as sky, and stopped.

What the episode actually shows is a filename doing the work of evidence.
`nav_screen_opened.ppm` sits in `captures/bridge/`, so three successive setups
were tried against it — nav view, nav view stepped 180 ticks, bridge at frame 90 —
each treated as "my setup is wrong" rather than "this may not be that screen." The
first check should have been WHAT THE IMAGE IS: two colours and a run length would
have said "static, rows 0..140" in one command.

Corrected: the port-validation row is withdrawn, and the test's doc says what the
capture is. The star layer's 33 pixels remain unexplained AS SUCH — that may or
may not be a real gap, but nothing here is evidence either way, and it is not
recorded as one.

Three captures now genuinely unread: `mission_briefing_eye.ppm`,
`script2_first_frame.ppm`, and `nav_screen_opened.ppm` — the last needs comparing
against the PRESENTATION screen, which is where it belongs.

## #116 — the misread capture, read correctly: the viewscreen static now has a test

`nav_screen_opened.ppm` is the VIEWSCREEN CONSOLE — binary static above, console
band below — which is what #115 established after #114 mistook it for a bridge
starfield. Pointed at the right screen, it is checkable, and the port renders
exactly that shape in `render_viewscreen_console`.

Not pixel-by-pixel: the static is generated noise, so two runs of the REAL GAME
would not match each other. Asserting pixel identity would be wrong in principle,
not merely brittle. What is deterministic is the distribution, and the capture
confirms all of it:

* **Two colours only** — the top two cover >99% of the region (the port emits
  index 224 and 239, the console bank's extremes).
* **The split** — 23315 dark to 19855 light is 54.0%/46.0%, matching the port's
  documented "~54% black (224) / ~46% white (239)" from oracle intro_215M.
* **Per-pixel noise** — mean run length 1.87px along a row, so it is noise rather
  than blocks or a dithered pattern.
* **The boundary** — static stops at the console band top (140).

That last check failed on the first attempt, asserting zero white below the band.
There are 483, because the band has bright content of its own. The distinction
that matters is DENSITY: 46% white above against 2.5% below, so the assertion is
now a ratio. A test that demanded zero would have been "correct" only until
someone looked at the band.

Four of the seven captures from #112 now have tests. Two remain unread
(`mission_briefing_eye.ppm`, `script2_first_frame.ppm`).

## #117 — a tool that answers "what IS this capture?", and what it says about the last two

#114's root error was letting a filename stand for evidence: three port states
were tried against `nav_screen_opened.ppm` before anyone asked what the image was.
`identify_capture` answers that mechanically — decode every frame of every HNM
under the asset root and rank by mean absolute difference against the capture.

Run against the two captures still unread, over **701 assets**:

    mission_briefing_eye.ppm    best 33.87  (hyper_01.hnm frame 25)
    script2_first_frame.ppm     best 36.40  (hyper_07.hnm frame 26)

Those are NEGATIVE results, not weak hits. A real match lands near zero — the
console-band comparison in #113 is byte-exact across 19200 bytes. At ~34 the
"best" candidates are simply the least-different images in a library of dark
scenes, which is what a ranking always produces whether or not the answer is
present.

So both captures are COMPOSITED screens: a scene plus overlays (subtitle text,
box chrome, palette state), which no single asset frame equals. That is worth
knowing before anyone tries to match them against an asset for a third time.

The tool's limits are stated in its own doc, because a negative result is only as
good as its coverage: it compares full 320x200 frames only, and only the first 40
of each file, so a match hiding in a talk-head band or late in a long clip would
be missed. Widen those before concluding a capture has no source.

Reproducing either capture needs the composite pipeline — VM state, scene, and
overlays together — which is the scenario harness's territory, not a frame diff.

## #118 — verifying overlay constants against the overlays, instead of skipping them

#106 stopped the checker resolving `croolis.rs`/`manu3.rs`/`manu3_hand.rs`
addresses against BLOODPRG.EXE, because those modules document `.xdb` OVERLAYS and
a lookup in the main image is meaningless in both directions. That was correct and
incomplete: it made those constants unverifiable rather than wrongly verified. The
blocker was the task.

The overlays are raw 386 images whose runtime `cs` maps 1:1 to file offsets
(`re/tools/dis_xdb.py`), so every check already written — immediates, shift counts,
operand offsets, layout identities — works unchanged on the right bytes. The
checker now loads `croolis.xdb`/`amer.xdb`/`scrut.xdb` for `croolis.rs` and
`manu3.xdb` for the two hand modules.

Immediate result: `ALIEN_POSITION_WRAP = 0x4000` VERIFIES. Its doc cites "method
`0x999`", which in BLOODPRG.EXE decodes to mid-instruction garbage — that mismatch
is what made it look suspect. In croolis.xdb, `0x99F` is `mov di,0x4000`, six bytes
into the cited method. Settled.

The other three overlay constants turn out to have CITATION problems rather than
value problems, and one of them was mine:

* `MENU_ANGLE_MASK = 0x0FFC`'s doc says "`0xFFC` = a 10-bit angle scaled x4". The
  address regex read the constant's own value as an address and then reported
  "not an immediate at 0x0FFC" — the tautology from #103, this time on the INPUT
  side. Numbers equal to the constant's value are no longer treated as addresses.
* `STATE_BASE = 0x2274` cites `ds[0x2274..0x2974]` — data offsets, not code.
* `ALIEN_COLONY_FRAME_GATE = 7` cites `0xB72`, which is data in croolis.xdb.

None is a wrong constant; all three want a citation naming the instruction that
uses them. That is a real, small task rather than an unverifiable class.

Settled: 540 -> 541 of 2156.

## #119 — three overlay constants given real citations, and one of them was pointing at the wrong number

Following #118's unblocking, the three overlay constants that had citation
problems now have answers. `re/tools/find_imm.py` was written to get them: find
every instruction whose immediate OR memory displacement equals a value, with the
boundary-consensus filter from #101 so phantoms resynchronised mid-instruction are
rejected rather than reported.

**`MENU_ANGLE_MASK = 0x0FFC`** — `mov bx,0xffc` at manu3.xdb `0x283`. The mask is
loaded once and applied to three per-node angle fields (`and ax,bx` @`0x289`,
`and si,bx` @`0x28E`, `and di,bx` @`0x290`, reading `[di+0x52]`, `[di+0x4E]`,
`[di+0x50]`). `0xFFC` keeps 10 bits with the low two clear — an angle index scaled
×4. Settled.

**`ALIEN_COLONY_FRAME_GATE = 7`** — the doc cited `cs:0xB72`, the gate's STORAGE.
That cell holds **2** in the shipped image, its idle value, so the citation read
literally was wrong and the constant looked wrong with it. The reload is
`mov word cs:[0xb72],7` at `0x11C5` and `0x12F9`. The VALUE was right all along;
citing the cell instead of the instruction that writes it made a correct constant
unverifiable. Settled.

**`STATE_BASE = 0x2274`** — searched the whole overlay as an immediate AND as a
displacement: zero confirmed hits. No manu3.xdb instruction names it. It is an
offset into the loaded data segment, reached through a base register, so its
position is a data-layout fact and not a code citation. The doc now says that
outright, so the next reader does not go looking for a `mov` that is not there.
Left unsettled, honestly, rather than settled on a citation that does not exist.

`check_cited_instructions.py` needed the same overlay awareness as the immediate
checker — it flagged the new, CORRECT manu3.xdb citation because `0x283` in
BLOODPRG.EXE is an `or`. Two guards had the same blind spot; the second only
surfaced when a citation finally existed to trip it.

Settled: 541 -> 543 of 2156.

## #120 — auditing the guards themselves, and re-learning a lesson the tool had already written down

#119 ended on a hazard worth acting on: a guard over an empty set passes forever,
and looks healthy precisely because nothing exercises it. So every checker was run
and its DENOMINATOR read:

    cited_immediates     95 constants        cited_instructions  109 instructions
    content_literals    367 literals         offset_pairs         21 pairs
    opcode_handlers      29 constants        provenance           11 claims
    selfref_asserts       7 assertions       opsize_mnemonics     11 conversions
    object_inline_names 640 objects          labels              568 + 238 rows

All live. But one number was off: `check_labels.py` verified only **9 of 568**
code labels, because it matched instruction-then-address (`` `mov ax,1` @0x1234 ``)
while `labels.csv` mostly writes address-first — its own comment gives
"0x91DB cmp word [si+0x36],0" as an example of a form it was not checking.

Adding that order found five "problems" on the first run. Four claimed `and`, and
their addresses were DS data offsets. They were prose: "DS:0x2578 and the ...".
The file's existing comment says this outright — "`and`, `or`, `not`, `sub`,
`add`, `test`, `in` and `int` are all ordinary words as well as mnemonics, so
prose adjacency proves nothing" — and I had relaxed the backtick requirement
anyway, reasoning that a preceding address anchors the mnemonic. It does not. The
address does not make the next word an opcode; the BACKTICKS are what mark a
quote, which is why they were required in the first place.

With backticks required in both orders: 9 inline claims verified, up from 7, zero
problems. A modest gain, honestly obtained.

The coverage is still 18 of 568, and that is not a parser problem to solve by
loosening it further — most label comments describe behaviour in prose rather than
quoting an instruction, and prose is not checkable. The way that number rises is
writing citations in quotable form, not teaching the checker to guess.

## #121 — 27 rows that could never be settled by decoding anything

The ledger's unsettled mass is 1340 rows with no citation and 191 with one. Those
are not the same problem, and a third kind was hiding in the first: functions that
encode NO decoded rule at all. `ship_slots_pub` returns a field. `rec_write_pub`
delegates to `rec_write` with the same arguments. There is nothing in either to
verify against the binary, so leaving them UNVERIFIED inflates the queue with work
that cannot be done.

`tools/classify_plumbing.py` finds them, and its test is deliberately strict
because a one-line function absolutely CAN encode a rule — `entity_draw_scale` is
`(3*scale >> 1) + 1`, decoded from a `mul`. A body qualifies only as a field read,
a borrow/clone of one, or a delegation whose arguments are plain identifiers.
Arithmetic, an index, a conditional or a constant disqualifies it.

The first run returned 34, and two were wrong in an instructive way:

    decode_frame           -> self.decode_frame_impl(idx, fb, pal, false)
    decode_character_frame -> self.decode_frame_impl(idx, fb, pal, true)

A boolean LITERAL matched the identifier pattern. Those two functions differ by a
decoded MODE, not by nothing — the flag is the claim. `true`/`false`/`None` and
SCREAMING_CASE constants are now rejected as arguments, taking the count to 27.

Settled as INFRA: 543 -> 570 of 2160. Not decoded, and correctly so — the honest
statement about a getter is that there is nothing to decode, which is different
from "not yet verified" and should not sit in the same bucket.

What remains in the queue is now closer to real work: 810 uncited functions plus
the constants, structs and enums.

## #122 — checking the claims that name their own verifier

The strongest provenance in this tree is a doc that names the test proving it:

    /// Verified byte-exact against the binary by `tests::angle_table_matches_binary`.

If the test is real. A named test that does not exist is worse than no claim at
all — the row reads as settled and nothing runs. `tools/check_claimed_tests.py`
resolves every such name and reports whether the test opens anything the game
shipped.

Result: 4 claims, all real, none missing, none self-referential. Two name tests
that read BLOODPRG.EXE or an asset; two name `func_<hex>` LIFTS, which are the
original instruction stream transliterated and oracle-verified — differentialling
against one is stronger than any hand-written test, not weaker.

Getting there took two wrong versions, and the pair is worth recording because
they failed in OPPOSITE directions:

* Case-insensitive throughout matched SCREAMING_CASE and reported the env var
  `CBLOOD_DATA` as a missing test function.
* Case-sensitive throughout then dropped every claim whose sentence begins
  "Verified ...", and the count fell from 5 to 1 — a quieter failure, and the more
  dangerous one, since a guard finding nothing looks like a guard finding no
  problems.

The keyword is matched case-insensitively and the captured NAME case-sensitively.
Both halves needed their own rule.

The denominator is only 4, so this guard's reach is small — most docs cite an
ADDRESS rather than naming a test, which the other checkers cover. It exists for
the failure it would catch, not for its coverage.

`SHIP_3D_ANGLE_TABLE` settled as DATA on the strength of its test: 180
`(cosine, sine)` Q14 pairs at `DS:0x4F45`, compared byte-for-byte against the
shipped image. Ledger: 570 -> 571 of 2160.

## #123 — "320x200" contains "0x200"

`SHIP_3D_HUD_BAND_TOP` sat in the ledger as ASM? with origin `0x200`. Its doc
cites no such address. What it says is "the bottom rows of the **320x200** frame"
— and `320x200` contains the substring `0x200`, which every address regex in the
tree was happily harvesting as a citation.

Seventeen ledger rows cited `0x200`; only two of those citations are real. Eleven
rows were provisionally ASM? on the strength of a screen dimension — rows that
look evidenced and are not, which is worse than UNVERIFIED because the provisional
status invites settling them.

Fixed in six tools at once (`audit_inventory.py`, `check_cited_immediates.py`,
`check_duplicate_rules.py`, `check_liftable_twins.py`, `check_opcode_handlers.py`,
`check_opsize_mnemonics.py`) with a negative lookbehind: an address must not be
preceded by an alphanumeric. 167 dimension strings across the source were feeding
this.

One row was ALREADY SETTLED with `0x200` as its only origin — `Vga` in
`src/recomp/machine.rs`, status ORACLE. Checked rather than assumed: its ORACLE
status comes from the recomp differential suite (37 tests, replaying the oracle
corpus and diffing against unicorn), not from any citation. The phantom origin was
incidental to it, and the status stands. No other settled row depended on one.

The general shape is worth keeping in mind for anything that harvests structure
from prose: the pattern was not wrong about what it matched, it was wrong about
what that match MEANT. `0x200` really is in the text.

## #124 — a check that could not be made sound, and was removed

Nothing verified that the port's cited addresses land on INSTRUCTION BOUNDARIES.
`check_labels.py` asks this of `labels.csv`, and the misanchored row in #101 —
`0x00B142` cited as `cdq; call 0xb6dd` while sitting inside `lcall 0x299:0x0ecb` —
is the defect it catches. Two attempts to bring it to the port's docs both failed,
in opposite directions, and the method was abandoned. Recording that so it is not
retried.

**Attempt 1, a standalone checker over the ledger's origins.** 19 of 133 flagged,
and reading them showed almost all were DATA addresses (`DS:0x1FAB`) or VALUES
(`OBJECT_FLAG_PAIR_SEEN`'s own `0x8000`, `ANGLE_UNITS_PER_REVOLUTION`'s `0x5A0`)
rather than misanchored code. The ledger's origin column CONFLATES code addresses,
data addresses and values, so nothing downstream can tell them apart. Deleted
before committing.

**Attempt 2, boundary consensus inside the existing mnemonic guard.** Three flags,
one of which was `0x8713` — a console row handler from #109, reached only through
the jump table at `0x8709`. An entry point has the PREVIOUS routine's bytes before
it, so no linear decode from earlier aligns to it. Requiring consensus condemns
every indirect-jump target, which is a large share of the addresses worth citing.

**Attempt 3, flag only an instruction that STRADDLES the address.** That is the
true signal — `lcall` at `0xB140` spans `0xB143` — but scanning back byte by byte,
some misaligned decode straddles almost any address by construction. 82 "wrong".

Consensus over-trusts and straddle-anywhere over-flags. Alignment is not decidable
from a bare address without knowing where the routine ENTERS, which the doc does
not say. #101 was caught with that context in hand, not by a rule. The mnemonic
check stands on its own (109 verified, 0 wrong); the alignment guessing is gone.

A guard that cannot distinguish its target from ordinary correct code is worse
than no guard, because its noise trains the reader to skip it — and this one would
have flagged the jump-table decode that #109 got RIGHT.

## #125 — verifying two rows by reading the handlers, and citing code instead of a flag

`enter_query`/`exit_query` cited `gs:0x67ad` — the query-mode FLAG. That is a data
global, so disassembling file offset `0x67AD` shows unrelated code, and the row
could not be verified from what it cited. The verification is the HANDLER, which
the dispatch table names: `0xA0` -> `0x6559`, `0xA1` -> `0x6572`.

    0x6559  mov byte gs:[0x67ad],1        query ON
    0x655F  mov ax,gs:[0x6884]            stack pointer
    0x6565  add ax,2 / mov gs:[0x6884],ax POST-increment
    0x656C  lodsw / mov [bp+0x6820],ax    operand into the slot it vacated

    0x6572  mov byte gs:[0x67ad],0        query OFF
    0x657C  cmp ax,2 / je 0x6587          pointer at the base: DO NOT pop
    0x6581  sub word gs:[0x6884],2

Both port arms match, including the guard: `0xA1` uses `Vec::pop()`, whose no-op
on an empty stack IS the `cmp ax,2` behaviour rather than an approximation of it.
Worth checking rather than assuming — an implementation that decremented
unconditionally would underflow exactly where the game refuses to.

The docs now quote the handlers, so `check_cited_instructions.py` covers them:
109 -> 117 verified. That is the concrete way the label-coverage problem from #120
gets better — write the citation in a form a checker can read, one row at a time.

Settled: 571 -> 573 of 2160.

## #126 — a decoded rule implemented twice, and the LIVE copy was the wrong one

Verifying `apply_operator` (cited `0x6863`) turned up something better than a
citation: the function is called from TESTS ONLY. All 19 references are in
`vm.rs`, and the sole one outside the test module is its own definition. Seven
opcodes dispatch to `0x6863` — `0xB1`, `0xB4`, `0xB5`, `0xB6`, `0xBE`, `0xBF`,
`0xC0` — and the execution loop implements the family INLINE, with its own copy of
the rule.

The copies agreed on almost everything: the `0xC0`/`0xC2` marker making the
operand indirect (`0x6877`/`0x687B`), signed compares, F5/F6/F7 mutating in set
mode. They disagreed on the operator the ladder does not recognise:

    0x6891  xor al,al                    <- al cleared BEFORE the ladder
    0x6893  cmp ah,0xf0 / setne  ...     each arm is an explicit compare
    0x68CF  cmp ah,0xf5 / sete
    0x68DB  or al,al / jne 0x6900        zero -> fall through to vm_branch

An unrecognised operator reaches `0x68DB` with al still zero, and zero BRANCHES.
`apply_operator` returns `false` for those — correct. The inline copy ended its
match with `_ => cur == operand_i`, folding every unknown operator into an
equality test that can decline to branch. The correct implementation was the dead
one; the live path had the defect, which is the worst arrangement of the two.

Fixed by routing the arm through `apply_operator`, so there is one implementation
and the tests cover the code that runs. The new test pins the ladder's fallthrough
including `0xF6`/`0xF7` — SET-mode operators with no query-mode arm, where equal
operands must NOT rescue them.

`check_duplicate_rules.py` could not have caught this: it matches ledger items by
cited address, and an inline `match` arm inside a 900-line exec function is not a
ledger item. Duplication that hides inside a function body needs a different
instrument, and I do not have one.

Settled: 573 -> 574 of 2160.

## #127 — a dead rule defended by its own test, carrying a refutation that never reached it

#126's lesson — a decoded rule can be implemented twice, with the LIVE copy wrong
— generalises into something detectable. `tools/check_unrouted_rules.py` finds
`pub fn`s with a binary citation and NO runtime caller, counting callers only
outside `#[cfg(test)]`, because a rule exercised solely by its own tests is
verified against itself and connected to nothing.

45 cited functions have no runtime caller. Working the first, `record_op` (cited
`0x674e`, `0x6946`), found the inverse of #126.

Its doc says the `gs:0x674e` wildcard "makes an operand match anything", and its
body is `wildcard == Some(op) || op == cur`. The LIVE arm for that handler family
(`0xAD/0xAF/0xB2/0xB3/0xBA/0xBB/0xBC`) says otherwise, in a comment recording how
the truth was found:

    an RHS equal to the SPECIAL OBJECT maps to 0xFFFF (the aboard value) before
    the compare — it is NOT a match-anything wildcard (the old `|| val==0xFFFF`
    made every aboard-guard pass; the matched-drive lane's first transcript diff
    caught it)

So the live path was corrected and the dead copy kept the refuted rule — with a
test asserting it: `record_op((7,9),(123,9),Some(7)) == QueryMatched`, wildcard 7
matching 123. Anyone wiring the "already tested" function up would have
reintroduced the bug a transcript diff once caught, and the test would have
agreed with them.

Removed: the function, the `RecordOpResult` enum, and the test, with a note where
they were saying why and naming the authority. A correction that reaches one
implementation and not its twin leaves the twin looking verified.

I damaged the file on the first attempt — the cut spanned `impl QuerySetMode`
and deleted `enter_query`/`exit_query`/`apply_operator` with it. `cargo build`
caught it immediately, `git checkout -- src/vm.rs` restored it, and the second
attempt worked line by line. Worth noting only because the recovery was cheap
precisely because the work was committed first.

## #128 — two decodes of one routine, disagreeing about what it BUILDS

Triaging the 45 unrouted rules split them by whether their cited address appears
elsewhere in the tree: cited once or twice means UNWIRED (a decoded feature the
port does not use — `text_speed_labels`, `parse_world_art_table`,
`mix_unsigned_pcm_average`), cited many times means DUPLICATED (a live copy
exists somewhere).

That surfaced something about #109. The console row-handler table I decoded there
— `cs:[bx+0xF29]` in segment `0x071E`, file `0x8709`, entries giving `0x8713`,
`0x872C`, `0x87BD`, `0x8848`, `0x886C` — was ALREADY decoded in `ship3d.rs` as
`run_ship_3d_nav_choice_handler_0..4`, all five settled ASM. My values agree
exactly with theirs, which is real corroboration between two independent readings.
It also means #109 rediscovered a table the port already had, because I searched
the binary and not the source.

The two readings disagreed on one thing, and the disagreement was worth having.
`handler_2` describes the loop as copying "special slots -> target records" and
`add ax,4` as storing "slot + header". I read `DS:0x2B13` as a menu word list and
`+4` as the object's inline NAME. Both cannot be right.

The consumer settles it. `0x87DF`, immediately after the loop, is

    mov si,0x2b13 / call 0x8428

— the same `list_widget_layout_unified` the OPTION menu enters with `si=0x2567`,
whose entries are POINTERS TO NUL-TERMINATED STRINGS (`DS:0x2573` `TEXT`,
`0x2581` `MUSIC_OFF`). So `0x2B13`'s entries are pointers as well, and `record+4`
is the inline name — the reading `object_inline_name` already implements, checked
against the shipped data for 630 of 640 objects.

`handler_2`'s doc is corrected and now names its consumer, so the next reader is
not left choosing between two plausible descriptions of the same six
instructions. The instruction transcription was right in both; only the noun was
wrong, and a noun is what tells you which of them to port.

## #129 — "what do we already know about this address?", and 55 answers nobody could find

#128's real cost was not the rediscovery, it was the ORDER: I searched the binary
before searching the source, and `ship3d.rs` had already decoded the table.
`re/tools/whatis.py` makes that one command — for an address it prints every
`labels.csv` row naming or citing it, every ledger row whose origin includes it,
and every source line mentioning it.

Its first run, on `0x8709`, found a duplicate I had created: `labels.csv` carried
BOTH `nav_choice_subdispatch_table` (the pre-existing row) and
`console_row_handler_table` (mine, from #109) for the same table. Merged into one
row holding the union — the entries, the `CS` segment `0x071E` that makes the file
offset `0x8709`, the handlers, and the click setup at `0x86A4`.

Then the obvious question: how many more? `check_labels.py` now groups rows by
address, and the answer is **55**. Among them:

    0x09b04  ship_3d_projected_point_plot / ship_3d_plot_point
    0x142d0  vm_handler_table / vm_opcode_handler_table_static
    0x0cdf4  resource_name_table / ..._full / ..._extent

Not all are defects — the `resource_name_table` trio plausibly records different
facets on purpose. But each means a reader who finds one row has no reason to look
for the other, which is exactly how #128's two readings of `DS:0x2B13` coexisted:
one called it target records, one called it a menu list, and neither knew about
the other.

Reported as its own category rather than as failures, because resolving 55 rows is
a task and blocking the suite on them would just get the check disabled. 0
problems, 55 duplicates, counted where they can be worked down.

The lesson is about accumulation. Every one of these was added by someone who
looked at the binary and wrote down what they found, which is exactly the right
instinct — and the knowledge base has no way to say "that address already has a
name" unless something checks.

## #130 — merging the nine addresses that carried the SAME NAME twice

Of #129's 55 duplicate addresses, nine were unambiguous: one address, one name,
two or more rows. `entity_object_populate`, `vm_field_offset`, `mouse_hit_test`,
`screen_mode_update`, `ui_region_table_scan`, `vm_post_update_c4_pair`, and the
three `nav_choice_handler_2/3/4` — the last of which I had given a THIRD name in
#109 (`console_row2_contact_menu_build`).

The tempting merge is "keep the longest comment", and it is wrong. `0x8269`'s
shorter row records the FAMILY (`0x8269/0x8295`) and the gate (`[0xa3e]`); the
longer one records the rect layout and the inside test. Neither contains the
other. Every group was merged by UNION instead, each surviving row carrying every
distinct comment with a marker saying so — 745 characters for `0x87BD`, holding
the interpolation note, the dispatch-table entry, and the contact-menu decode
together for the first time.

Eleven rows removed, 55 duplicates down to 46, and no information discarded:
checked afterwards that `0x87BD`'s merged row still contains all three sources.

The remaining 46 are the harder kind — one address under DIFFERENT names, where
merging requires deciding which name is right, and sometimes which READING is
(`0x22E0` is both `abs_negate_gs_setup` and `palette_blend_remap_table_build`;
only the second describes what the routine is for). Those get worked one at a
time, because that is where the #128-style disagreements live.

## #131 — corrections filed NEXT TO the claims they corrected

Working the different-name duplicates found a worse pattern than duplication. At
`0x22E0`, the long row opens

    CORRECTED (was abs_negate_gs_setup, which only described the first four
    instructions). THE TINT REMAP-TABLE BUILDER ...

— and `abs_negate_gs_setup` was still in the file, unchanged, as its own row. The
correction knew about the mistake; the mistake did not know about the correction.
A reader who greps `0x22E0` and hits the wrong row learns that the routine is an
"abs/negate helper", with nothing pointing onward.

`0x183E` and `0x210E` are worse: whole chains kept as separate rows —
`input_action_jump_table`, then `..._CORRECTION`, then
`..._static_limit_CONFIRMED`; `input_action_dispatch`, then
`..._UNRESOLVED` labelled "HONEST RETRACTION (2nd correction)". The retraction is
admirably explicit and sits beside the claim it retracts, which cancels most of
its value.

Four chains merged, seven superseded rows folded in. The old readings are KEPT,
labelled `SUPERSEDED READING \`name\``, because the history is genuinely useful —
knowing that `0x22E0` looks like an abs/negate helper for four instructions is
what stops the next reader repeating the error. What is not useful is that text
standing alone under its own name.

42 duplicate addresses remain. One is a genuine unresolved DISAGREEMENT rather
than a stale name: `0x00813` is both `state_gate_b21` ("conditional gate on
gs:[0xb21]&1") and `timer_isr_handler` ("game timer ISR, installed at cs:0x213 by
install_timer_isr_hook 0x79c"). Those are not two descriptions of one thing; one
of them is wrong, and deciding which needs the routine read. Left flagged rather
than merged, because merging it would manufacture agreement.

## #132 — the one disagreement that was a real question, answered

#131 left `0x00813` flagged rather than merged: `state_gate_b21` ("conditional
gate on `gs:[0xb21]&1`") against `timer_isr_handler` ("game timer ISR, installed
at `cs:0x213` by `install_timer_isr_hook 0x79c`"). Not two descriptions of one
thing — one had to be wrong.

Reading the installer settles it. `0x79C` is

    mov ax,0x3508 / int 21      GET the IRQ0 (INT 08) vector
    mov gs:[0xB1D],bx / gs:[0xB1F],es   save the ORIGINAL handler
    mov ah,0x25 / mov bx,cs / mov ds,bx / mov dx,0x213 / int 21   SET the new one

and with this segment based at file `0x600`, `CS:0x213` IS file `0x813`. It then
reprograms the PIT (`out 0x43,0x36`; `out 0x40` with `0x1746`). The routine ends
the two ways an ISR must: `0x92F` `pop ax / ljmp gs:[0xB1D]` chaining to the saved
original, and `0x935` `mov al,0x20 / out 0x20,al / iret` sending EOI.

So it is the timer ISR, and `state_gate_b21` described its FIRST INSTRUCTION —
`test byte gs:[0xB21],1`, the gate choosing chain-vs-handle. Exactly the shape of
`abs_negate_gs_setup` at `0x22E0` (four instructions of a table builder) and
`ds_es_rebase_gs` at `0x242D` (a prologue). A shallow name is not a wrong
observation; it is a correct observation mistaken for the answer, and it outranks
the real answer whenever a reader hits it first.

Merged with the evidence in the row and the old reading kept as SUPERSEDED. 41
duplicates remain.

That is three sessions' worth of the same lesson from different directions: #127
(a dead rule keeping a refuted reading), #128 (two names for one routine
disagreeing about what it builds), #131 (corrections filed beside their claims),
and now this. The knowledge base does not converge on its own — every one of these
was written by someone who read the binary correctly and stopped at a different
depth.

## #133 — seven more shallow/deep merges, and a stale number I kept repeating

The pattern #132 named is mechanically findable: one address, two names, the
shorter comment describing a prologue or a single facet. Seven remained —
`input_action_xlat_table`, `clipped_blit_w8_a`, `gfx_draw_mode_d`,
`vm_bit_set_test_6aa7`, `vm_helper_604e`, `vm_text_helper_6886`, and
`ship_3d_target_query_layout`, the last shadowing `list_widget_layout_unified`,
the widget BOTH the OPTION menu and the contact menu enter (#110, #128).

Merged keeping the deeper name, the narrow reading retained and labelled. The
wording is "NARROWER EARLIER READING", not "superseded": `0x0173e`'s pair is a
table NAME plus a MEASURED BOUND, complementary rather than one replacing the
other, and calling that superseded would misdescribe it. 34 duplicates remain.

### The count

Deleting `record_op` and `RecordOpResult` in #127 removed two ledger rows, one of
them settled, taking the ledger from 2160/574 to **2158/573**. Four subsequent
reports quoted the old figure, because the number was carried forward instead of
re-read after a commit that changed it.

Small, and worth fixing precisely because the ledger is the thing this campaign
reports progress against. A denominator that moves when rows are DELETED is
working correctly — removing a refuted implementation should shrink the queue —
but only if the number is re-read rather than remembered.

## #134 — TEXT opens a submenu; the port was cycling a value

`text_speed_labels` was one of #128's UNWIRED rules — decoded from `DS:0x259D`
(`VERY FAST`, `FAST`, `MEDIUM`, `SLOW`, `VERY SLOW`) and called by nothing. What
the port did instead, on clicking TEXT in the OPTION menu:

    engine.text_speed_step = match engine.text_speed_step {
        1 => 2, 2 => 3, 3 => 4, 4 => 7, _ => 1,
    };

The cycle produces the right STEPS — those are the decoded values — while
skipping the surface entirely. The game shows a five-row list and the player picks
one; a click-to-cycle control cannot show which speed is currently selected, and
reaching `VERY SLOW` from `VERY FAST` takes four clicks instead of one.

Corroboration that TEXT opens a list rather than toggling: `handler_4`'s selection
0 writes `[0x259B]=1` and `[0x259C]=1` — the two bytes immediately BEFORE the
pointer list at `0x259D`. The flags and the list they gate are adjacent.

TEXT now opens the submenu, built from the game's own labels, with the current
speed preselected; the row index IS the setting, mapped by
`vm::text_speed_step_from_setting` (the `0x1B29..0x1B3D` init, `VERY SLOW`
jumping to 7 via `cmp ax,8`). The new test pins the one-to-one correspondence and
that no two settings share a step — which is what preselecting the active row
relies on.

43 cited-but-unrouted rules remain, one fewer than before. The lesson from this
one: a port can compute the right VALUES through the wrong INTERFACE, and a
value-level test will never notice. Nothing about the cycle was numerically wrong.

## #135 — the port does not mix, it stacks

`mix_unsigned_pcm_average` was another unwired rule. Chasing its callers first
fixed a tool: `re/tools/find_near_callers.py` opened `bin/BLOODPRG.EXE` as a bare
relative path, so it only ran from inside `re/` and raised FileNotFoundError from
the repo root — where CLAUDE.md says tools are run. It uses the shared loader now.

With that working, `0xBB6D` has NO near callers, which is the right answer: it is
a fragment inside the SND player, not a callable routine —
`lodsb / add al,es:[di] / rcr al,1 / stosb`, where the add's carry becomes bit 7
during the rotate, giving `floor((src+dst)/2)`.

The gap is what the port does instead. It runs THREE independent `MusicPlayer`
streams — music, voice, chatter — and lets the audio backend sum them. So two
simultaneous sounds play at FULL amplitude and can clip, where the game halves
each and cannot. That is audible, not cosmetic, and it is the same shape as #134:
the port produces a plausible result through an interface the game does not have.

Recorded in `docs/port-validation.md` as OPEN. Closing it needs a mixing output
path rather than a patch, and the primitive is already decoded and settled —
`snd_mix_average`'s test walks all 65536 input pairs, modelling the `add`/`rcr`
pair independently and comparing, which is verification against the instruction
rather than against itself.

## #136 — the settle tool counted no-ops as progress

#135 settled `snd_mix_average` and `mix_unsigned_pcm_average`, the tool said
"settled 2 row(s)", and the ledger total did not move. Both were ALREADY `ASM`.

`audit_settle.py` reported every row it TOUCHED rather than every row it CHANGED,
so re-settling an already-settled row printed exactly like settling a new one. On
a campaign that reports progress by this number, a tool that cannot distinguish
"I verified something" from "I re-stated something" is quietly corrosive: the
transcript shows movement, the ledger shows none, and only a recount catches it.

Now it separates them — `settled 0 row(s) as ASM; 2 already ASM`.

Noticed only because the total was RE-READ after the settle instead of assumed,
which is the same habit #133 was about. Two counting mistakes in one session, both
from trusting a number instead of recomputing it, and both cheap to catch the
moment the recount became routine.

## #137 — the world appears, but the game's record of it never gets written

`world_click_select` is the third unrouted rule in a row, and the clearest case of
the pattern. It ports `0xB20C..0xB27B` completely — nothing hit returns false; the
`0xFFFF` back row clears the target; a target ALREADY equal to `gs:0x251B` is not
rewritten (`cmp ax,[0x251b]` @`0xB21A`); anything else sets `gs:0x251B` and writes
a C1 record `{0xC1, target, 0}` at `[0x6750]+0xA`, the built-in object `orxx`,
which the C1 ladder at `0x5B38` presents on a later frame.

`world_target` — the field standing for `gs:0x251B` — is touched by that function
and its two tests, and by nothing else in the tree. `main.rs` selects a
destination by calling `engine.visit_world(...)` directly.

So the OUTCOME is right: the world appears. The MECHANISM is absent: no C1 record
is written, so the VM's presentation ladder never runs for that target, and any
script logic gated on that record cannot fire. This is invisible to any test of
"does the world load", which is exactly why it survived — the surface works.

Three consecutive findings of the same shape (#134 cycling instead of opening the
submenu, #135 stacking streams instead of mixing, this) suggest it is the dominant
remaining defect class: the port reaching a plausible end state by a route the
game does not take. A value test cannot see it, a screenshot cannot see it, and it
only shows up when something downstream depends on the STATE the real route would
have left behind.

Settled `world_click_select` as ASM (decoded and tested), and recorded the wiring
gap in `docs/port-validation.md` rather than leaving it implied by an unrouted
function.

## #138 — naming the blocker instead of bridging it

Wiring #137's world-destination commit stopped at a decision worth recording. The
VM's `world_click_select` takes a target RECORD. The frontend's destination path
has a world NAME, chosen by `compass_angle * n / 180` — arithmetic over
`nav_world_labels`, where the game hit-tests a nav-chart object.

The obvious bridge is to match the name against `object_inline_name` over
`build_nav_chart_list()`. It would work, and it would be a rule the game does not
have: the original never needs a name->record mapping, because it commits the
object the player CLICKED. Adding one to make the pieces fit is the fabrication
the prime rule names, and it would be harder to spot later than a hardcoded
string, because it would look like plumbing.

So the row in `docs/port-validation.md` now records the blocker exactly, and the
fact that every piece of the REAL route already exists: `build_nav_chart_list`
(`0x721A`), `nav_chart_object_click`/`nav_chart_pick` (`0x92A3`),
`world_click_select`, `object_inline_name` — and `nav_chart_click` is already
wired, to the info panel. The task is routing the destination COMMIT through that
click, which changes which surface selects a world, and is a frontend change worth
doing deliberately with an oracle comparison rather than squeezed in behind a
name-matching helper.

Stopping at "here is the blocker" is the right outcome when the alternative is
inventing the missing piece.

## #139 — four UI strings the content guard could not see

`location_status_block` built its text from four `&str` constants in `vm.rs`:
`"PLANET: "`, `"SHIP: "`, `"BLACK HOLE: "`, `"LIFE SUPPORT:"`. They are the
game's headers at `DS:0x12E`, `0x137`, `0x13E`, `0x14B` — the `mov si,imm`
constants in `0x8369..0x839F`.

`check_content_literals.py` never flagged them. Its string scan needs 12+
characters and its prose test needs sentence shape, so a short ALL-CAPS label
slipped under both. The guard now recognises that shape — and it is worth being
precise about what it caught, because the previous state was better than "a
hardcoded string":

`STATUS_STRING_TABLE` paired each literal with its DS offset AND its file offset,
and a test compared the literal to the image bytes and checked the two offsets
described the same byte. These were VERIFIED TRANSCRIPTIONS. That is a real
standard, and the guard cannot see it, which is why the flag needed reading rather
than obeying.

Still changed, for one reason: a pinned copy BREAKS against a differing build
where a read FOLLOWS it. `bloodprg::location_status_headers` reads the four
strings, `StatusHeaders` carries them, and `location_status_block` /
`location_panel_rows` take them as an argument — so the frontend supplies the
game's own text and `vm.rs` holds no copy. `STATUS_STRING_TABLE` survives as the
address evidence, minus the strings.

The threading reached further than expected: `location_panel_rows` shares the
headers, so the example and `main.rs` both needed the real source wired in. That
is the cost of removing a copy — and the reason a copy is tempting.

## #140 — proving the widened guard can still see

#139 taught `check_content_literals.py` the UI-label shape, but its string scan
still required 12+ characters — right for prose, and the reason `"SHIP: "` (6) and
`"PLANET: "` (8) were invisible in the first place. A second, shorter scan now
runs for labels only.

It reports NOTHING, which is the answer that needs checking rather than
celebrating: a widened guard finding zero is indistinguishable from a widened
guard that does not work. Run against `vm.rs` as it stood one commit before #139,
the same pattern finds all four headers, 19 occurrences.

So the guard is live and the tree is clean of that class. Worth the extra minute:
of the three counting or coverage mistakes this session (#133's stale figure,
#136's no-op settles, #122's case-sensitivity collapse), two showed up as a number
that looked fine, and the third as a guard quietly matching nothing.

## #141 — a note attached itself to the next constant, and 93 ASM rows cite nothing

Two findings, both about the ledger believing things nobody claimed.

**A comment adopted the item below it.** `DIALOGUE_FONT_GLYPH_HEIGHT` carried
origin `0x1B29,0x1B3D` — the TEXT-SPEED addresses. Those come from #119's note
about not duplicating the step mapping, which sits above it. Moving the note away
did not help: `audit_inventory.py` only ended a comment run on a non-empty,
non-comment line, so a BLANK LINE did not separate them. Now it does, and nothing
legitimate is lost because Rust doc comments must be adjacent to their item. Three
rows lost phantom origins, this one included.

**93 rows are settled ASM with no cited address.** `ASM` means verified against
the assembly, so an ASM row whose ledger origin is empty is asserting something
the ledger cannot show. (`DATA`, `INFRA` and `ORACLE` legitimately have none —
they are verified against game files, are plumbing, or are differentialled.)

Sampling them found the sharper version of the problem: `choice_box_row_at` and
`list_menu_click` were settled ASM while their own docs said the geometry was
"the same measured geometry as the draw". The VALUES are decoded — the box centre
is `mov word [0xAC6],0x64` @`0x86D9`, the pitch is `add bp,0xB` @`0x847A`, and the
widget's hit-test is `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`) — but the wording
was left over from when they were not, and it undercut the status the rows
carried. Both now cite the divide they reproduce, which also brings them under
`check_cited_instructions.py`.

The remaining ASM-without-citation rows are a real queue, not a bug: each needs
either a citation written or a status corrected. Counted rather than fixed in
bulk, because deciding which of the two applies is the work.

## #142 — writing one of the 91 missing citations

#141 counted 91 rows settled `ASM` with no cited address. Working the first showed
what the queue actually contains.

`update_ship_3d_transition_state` had NO doc comment at all — settled as verified
against the assembly, with nothing recorded about which assembly. Its three
constants (`SHIP_3D_TRANSITION_OPEN_STEP` 4, `CLOSE_STEP` 8,
`OPEN_TIMER_THRESHOLD` 120) sat undocumented at the top of the file with a dozen
others.

`labels.csv` already knew the routine — `0x00B692,ship_3d_transition_state_update`
— which is what `re/tools/whatis.py` exists to surface (#129). Reading it:

```text
  0xB692  test byte [0x2533],1     ARMED?
  0xB699  cmp word [0xb3b],0x78    not armed: hold timer vs 120
  0xB69E  jbe 0xb6dc               at or below -> nothing happens
  0xB6A0  mov byte [0x2531],4      open step
  0xB6B1  cmp word [0xb3b],0       armed: timer exhausted?
  0xB6B8  mov byte [0x2531],8      close step -- twice the open rate
  0xB6C2  mov byte [0x2533],0      disarmed
```

Every constant confirmed, and two details the port's bare numbers did not state:
closing steps at twice the opening rate, and `jbe` means the transition arms only
when the timer is ABOVE 120, not at it.

Cited instructions verified: 117 -> 127. The queue: 91 -> 90.

That ratio is the honest measure of this work. One row costs a routine read, and
the value is not the settled count — it is that "step 4" now says WHY it is 4 and
where to look when it turns out to be wrong.

## #143 — the band copy, and where three magic numbers came from

Second row off #141's queue. `copy_ship_3d_plane_bands` was settled ASM with no
doc, and three constants it depends on sat bare at the top of `ship3d.rs`:
`SHIP_3D_PLANE_ROW_BYTES` 80, `SHIP_3D_PLANE_BASE_ROWS` 35,
`SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET` `0xC000`.

`labels.csv` named the routine already — `0x00B6DD ship_3d_plane_band_copy` — and
it is the same routine whose CALL SITE #101 corrected, where a mis-anchored label
had read the `0x99` of `lcall 0x299:0x0ecb` as a `cdq`. Reading it:

```text
  0xB6E5  test byte [0x252e],1     the copy-enabled gate
  0xB6F0  cmp word [0x524d],0xa    scroll mode 10 SKIPS the scroll update
  0xB6F7  ax = bx+bx / cmp ax,0x64 / jle / mov ax,0x64   clamp 2*depth to 100
  0xB703  sub ax,0x64 / neg ax / mov [0x524f],ax         store 100 - that
  0xB70B  mov dx,0x3c4 / mov ax,0xf02 / out dx,ax        map mask = all 4 planes
  0xB718  mov si,0xc000            SOURCE PAGE 0
  0xB71C  ax = bx+0x23 / dl=0x50 / mul dl                (depth + 35) * 80
```

All three constants are in that one multiply and load: `0x23` is 35, `0x50` is 80,
`0xC000` is the page. The port's `ship_3d_plane_band_byte_count` computes
`(depth + 35) * 80` and now says why.

Cited instructions 127 -> 134; the queue 90 -> 89.

Two rows in, the pattern of this queue is clear: these are not unverified
functions. They are verified functions whose verification was never written down,
and the routine is usually already in `labels.csv` — the cost is a read, not a
decode.

## #144 — the interpolation gate divides before it multiplies

Third row off #141's queue, and the first where the missing doc was hiding a
detail worth stating rather than just an address.

`step_ship_3d_interpolation_gate` had no doc. `labels.csv` had the routine —
`0x001E5D ship_3d_interpolation_gate`, plus `DS:0x0ADA` (duration) and `DS:0x0ADB`
(tick). Reading it:

```text
  0x1E63  mov bl,[0xada]           the DURATION
  0x1E67  cmp bl,[0xadb] / je      duration == current tick -> complete
  0x1E6D  inc byte [0xadb]         advance the tick FIRST
  0x1E71  lodsw / sub ax,[di]      delta = source - dest
  0x1E74  idiv bl                  delta / duration  (SIGNED, 8-bit quotient)
  0x1E76  imul byte [0xadb]        * the tick
  0x1E7A  mov dx,[di] / add dx,ax  dest + that
```

The ORDER matters. The game divides and then multiplies, so every step carries
the truncation of an 8-bit quotient; multiplying first — the arrangement a port
naturally reaches for, and the one that loses less precision — gives a different
value for most non-exact divisions.

The port already had it right, including `checked_i16_div_i8_to_i8` modelling
`idiv bl` down to the overflow the CPU traps on. Nothing needed fixing; what was
missing was any record that the order was a decision rather than an accident. A
future refactor "simplifying" it to `delta * tick / duration` would look like an
improvement and would be wrong.

Cited instructions 134 -> 141; the queue 89 -> 88.

## #145 — mode-X, and why a two-line function needed a paragraph

Fourth row off #141's queue. `mode_x_to_linear` is

    byte_offset * 4 + plane

with no doc, settled ASM. The forward mapping lives in the mode-X plot at `0x3428`
(`graphics_plot_modex`, `SEG 0x299:0x498`), already in `labels.csv`:

```text
  0x3461  and cl,3     plane  = x & 3
  0x3464  shr bx,2     column = x >> 2
  0x3467  add ax,bx    + the row base, then `add di,ax`
  0x346B  mov dx,0x3c4 / mov al,2 / out dx,al   select the map-mask register
```

So `offset = y*80 + (x>>2)`, `plane = x & 3`, and the port's expression is the
inverse. The part worth writing down is WHY it works beyond a single row: the
plane stride is 80 and `80 * 4 = 320`, so multiplying the whole offset by 4 scales
the row base into place at the same time as the column. Read cold, `* 4 + plane`
looks like it should only be valid within a row.

Cited instructions 141 -> 145; the queue 88 -> 87.

Four rows in, and the case for the queue is not the settled count. It is that
`byte_offset * 4 + plane`, `delta / duration * tick`, and `(depth + 35) * 80` are
all correct, all unexplained, and all one plausible-looking edit from being wrong.

## #146 — the info panel is a tint, like everything else in this UI

Fifth row off #141's queue. `render_location_info_panel` had no doc. Reading
`0x9137..0x91EC`:

```text
  0x9142  mov bx,[0x2780] / cx,[0x2782] / dx,[0x2784] / bp,[0x2786]
                                   the window rect (x, y, w, h)
  0x9152  mov si,[0xac8]           the remap table
  0x9156  lcall 0x299:0x40e        THE TINT BLIT
  0x915B  mov bx,0x6e              text x = LOCATION_PANEL_X
```

The panel background is not painted — it goes through `0x299:0x40E`, the same
tint primitive as the choice box (`list_widget_box_is_a_tint` in `labels.csv`) and
the confirm dialog. That is why the port remaps a rect here instead of filling
one, and it is the third surface traced to that one call.

Cited instructions 145 -> 149; the queue 87 -> 86.

Five rows in, `0x299:0x40E` has now appeared in the citation for the choice box,
the confirm dialog and this panel. A reader who meets any one of them cold would
reasonably assume a filled rectangle; the routine says otherwise every time.

## #147 — a citation can hide in the body, and my measurement of how often was wrong

`draw_subtitle_revealed` was settled ASM with no ledger citation, and its body was
full of them — renderer `0x3630`, the `0xFF`/`0xFE`/`0xFD` reveal-colour law, even
a dated retraction of an earlier reading. The evidence existed; the ledger reads
the DOC comment, so none of it counted.

Two fixes followed, one of them mine to correct.

`audit_inventory.py` now scans a function's BODY COMMENTS when its doc has no
address — brace-bounded, comments only, because a bare literal in code is a VALUE
and treating it as an address is how `"320x200"` became a citation in #123.
`console_box_click` gained `0x8508,0x84E6,0x84EE,0x84F6`, which is exactly right:
those are the hit-test's own citations, written where the author was working.

Then I measured how general this was and got 56 of 83. That number was WRONG. It
came from a fixed 60-line window with no brace counting, so it read addresses out
of the NEXT function and attributed them to this one — the same over-reach the
scan itself was written to avoid, in the tool I used to check the scan.
Brace-bounded, the true count of uncited-ASM functions with an address in their
own body comments is **0**: the rows that had one already gained it.

So the queue is 74 rows that genuinely say nothing about their basis, not 27. The
correction matters more than the count: I nearly recorded "most of these are just
misplaced citations" as the character of the remaining work, on the strength of a
measurement that was measuring its neighbours.

## #148 — the nav sector was a literal range in two places

`bridge_nav_destination_click` gated on `(72..=107).contains(&frame)` with no doc,
and the same literal range appeared again in the render path. Two copies of one
condition, neither saying what it meant.

The range is not invented — `TB.BIG`'s frame headers carry a STATION per frame,
and `tbbig`'s own test pins frames 72..=107 to station 2, the pyramid navigation
room. So the numbers were right and derived; they were just spelled out instead of
read.

Both sites now ask the header: `bridge_station() == Some(NAV_ROOM_STATION)`. Three
things improve. The gate follows the data if the archive ever disagrees. The
condition says WHICH station rather than which frames. And the drawn surface and
the clickable one share one expression, so they cannot drift apart — two copies of
a frame range is exactly how a widget ends up clickable where it is not visible.

The render site's comment also said the interaction pattern was "captured live
from the real game". Reworded: the capture confirms the box's appearance, it does
not source the behaviour. That phrasing was one `check_provenance.py` pattern away
from being flagged, and it describes the prime rule backwards.

One behaviour change worth naming: with no archive loaded, `bridge_station()` is
`None`, so both paths now decline where the old range check could pass on a stale
frame number. Nav destinations without a bridge archive are not a state the game
has.

## #149 — the starfield's depth model is one `jne`

Three more rows off #141's queue, all from the point plot at `0x9B04`.

`ship_3d_projected_point_offset` computes `y*320 + x`. The game does it without a
multiply:

```text
  0x9B25  mov di,bx      di = y
  0x9B27  xchg bh,bl     bx = y << 8   (y * 256)
  0x9B29  shl di,6       di = y * 64
  0x9B2C  add di,bx      y*64 + y*256 = y * 320
```

`ship_3d_projected_point_shade` is `0xEF - (depth >> 12)`, built as `neg al` then
`add al,0xef` — so it wraps in 8 bits exactly as the port's `wrapping_sub` does,
which is the kind of agreement that is luck unless someone checks. `SHADE_BASE`
239 and `SHADE_SHIFT` 12 are that `0xEF` and `0xC`.

The detail worth having is in `plot_ship_3d_projected_point`, and it is one
instruction: `mov al,es:[di] / or al,al / jne 0x9B44`. A point draws ONLY where
the pixel is still empty, so the starfield keeps the FIRST point at each position
rather than the last. There is no z-buffer in this path — that ordering rule IS
the depth model, and a port that wrote unconditionally would look right in a still
frame and wrong in motion.

Cited instructions 149 -> 158; the queue 75 -> 73.

## #150 — compare before write, or the dirty list means nothing

Two more rows off #141's queue.

`build_ship_3d_projection_matrix` (`0x98B9`) loads the angle table with
`mov bp,0x4f45` — literally `SHIP_3D_ANGLE_TABLE`'s address — reads the angle
words, and leaves the matrix at `DS:0x2F95` for `ship_3d_point_cloud_project`
(`0x9A10`) to run the 1000 records through. The port's odd-looking field names
(`angle_2f71`, `projection_angle_2f6d`, `angle_2f6f`) are named for their globals
because that is the only thing that distinguishes three consecutive words, and
keeping the names makes the correspondence checkable.

`update_ship_3d_sprite_slot_position` (`0x420D`) is the more interesting one:

```text
  0x4210  shl ax,5 / mov bx,0x6212 / add bx,ax   slot = 0x6212 + id*32
  0x421D  test al,0x81 / je                      ACTIVE mask 0x81
  0x4221  cmp dx,gs:[bx+8]  / je / or al,2 / mov gs:[bx+8],dx
  0x422D  cmp cx,gs:[bx+0xa]/ je / or al,2 / mov gs:[bx+0xa],cx
```

Each coordinate is COMPARED before being written, and the dirty bit is set only
when the value actually changes. Moving a slot to where it already is marks
nothing. The port mirrors that per field — and writing both unconditionally, which
is the obvious simplification, would dirty every slot every frame and quietly
defeat the dirty-rect list the renderer walks at `DS:0x6612`. The screen would
still be correct; only the reason for the dirty list would be gone.

Cited instructions 158 -> 165; the queue 73 -> 72.

## #151 — `btr` puts the old bit in the carry, and the port knew

`update_ship_3d_sprite_slot_extent` (`0x42CD`), the sibling of #150's position
update, and the subtlest of the queue so far:

```text
  0x42E1  lds si,[bp+4]                   the SOURCE dimensions
  0x42E4  cmp cx,[si] / cmp dx,[si+2]     width/height vs source
  0x42ED  btr ax,4                        matches: CLEAR extent-changed
  0x42F1  jae 0x430D                      ...and if it was ALREADY clear, stop
  0x42F3  or al,2                         otherwise mark dirty
  0x42F7  cmp cx,gs:[bx+0xc] / gs:[bx+0xe]   differs: vs the slot's own extent
  0x4303  or al,0x12                      extent-changed AND dirty, together
```

`btr` is bit-test-and-RESET: it clears bit 4 and leaves the OLD value in CF, so
the `jae` immediately after reads "the flag was already clear". That single
instruction encodes a conditional the port spells out as
`if flags & EXTENT_CHANGED != 0`, and it is the reason CLEARING the flag still
counts as a change worth dirtying. `0x12` is the two flags at once, not a third
flag — which is exactly the kind of constant that gets mis-transcribed as
`EXTENT_CHANGED = 0x12`.

The port had all of it right. Cited instructions 165 -> 176; the queue 72 -> 71.

Ten rows in, not one has been WRONG. What they lacked was any record that their
odd shapes — a divide before a multiply, a compare before a write, a bit-test
whose carry is the condition — were transcriptions rather than choices.

## #152 — a ported function that is deliberately less than its routine

`commit_ship_3d_sprite_slot_dirty_geometry` is the third slot function, and the
first in this queue where the port and the routine do NOT correspond one-to-one.

`sprite_slot_commit_dirty_range` @`0x43F7` takes a slot RANGE packed into `ebp`
(`shl ebp,0x10 / mov bp,bx`), walks it from `0x6212 + first*32`, and carries a
second path entirely:

```text
  0x4412  test word [0x5249],1 / je 0x4435   the clip-SNAPSHOT flag
  0x441D  mov eax,[0x5235] / stosd           left+right as ONE dword
  0x4423  mov eax,[0x5239] / stosd           top+bottom
  0x4429  mov word [di],0xffff               terminate the list
  0x442D  mov word [0x5249],0                and clear the flag
```

That path pushes the WHOLE clip window into the dirty-rect list at `DS:0x6612` as
a single entry instead of per-slot rectangles — the "everything changed, stop
tracking pieces" escape hatch.

The port implements only the per-slot commit. Neither the range walk nor the
snapshot is ported, because this engine redraws every frame and keeps no dirty
list. That is a legitimate difference, and the point of writing it down is that an
UNDOCUMENTED partial port is indistinguishable from an incomplete one: the next
reader finds a routine with two branches and a function with neither, and cannot
tell whether the omission was reasoned.

Worth noting what still matters: the dirty BIT is read by this commit even though
the dirty LIST is not built, so #150's compare-before-write is not dead weight.

Cited instructions 176 -> 181; the queue 71 -> 70.

## #153 — the layout formula, finally cited at its source

`layout_ship_3d_target_list` had no doc. The routine is `0x84A1..0x84C6`, inside
`list_widget_layout_unified` (`0x8428`):

```text
  0x84A1  add dx,0x14                        width  = widest + 20
  0x84A7  add bp,8                           height = rows*pitch + 8
  0x84AD  shr dx,1 / sub dx,[0xac6] / neg dx     x = anchor - width/2
  0x84B9  sub bp,0xc8 / neg bp / shr bp,1        y = (200 - height)/2
```

`0xAC6` is the anchor the caller sets — `0x64` for the console box, `0xE1` for the
in-window concept list — and `0xC8` is 200.

This closes a loop. #111 corrected the list menu from per-label centring to
flush-left at `x0 + 10`; #112 confirmed `anchor - (widest+20)/2` against two
captures at two different left edges, 170 and 173. That was verification against
screens. The formula's actual SOURCE is these four instructions, and until now no
port function cited them — the geometry was right, agreed with by the game's own
pixels, and unattributed.

Cited instructions 181 -> 187; the queue 70 -> 69.

Twelve rows in, this is the second time the queue has produced the citation for
something an earlier fix had already established empirically (#143 did it for the
band copy's constants). The queue is not only about undocumented code — it is
where the evidence for things already believed turns out to have been sitting.

## #154 — one-based rows, and a bound that is not the box height

`hit_test_ship_3d_target_list`, the sibling of #153's layout:

```text
  0x84E6  add cx,4                       the row origin is box_y + 4
  0x84F8  mov ax,[0xa2c] / sub ax,dx     dy = mouse_y - that origin
  0x84FD  js 0x853B                      above the box: miss
  0x84FF  sub bp,8 / cmp ax,bp / jge     below (height - 8): miss
  0x8506  mov bl,0xb / div bl            row = dy / 11
  0x850A  inc al                         ...+ 1, so rows are ONE-BASED
  0x850C  mov [0x27c7],al                the hovered row
```

Three things a reimplementation gets wrong by default, all of which the port has
right:

* The bound is `height - 8`, not the height. The 8px of chrome that
  [`layout_ship_3d_target_list`] ADDED (`add bp,8`) is not clickable, so the box
  is deliberately larger than its hit area.
* The row is ONE-BASED after `inc al`, so `0` means "no row" rather than "the
  first row" — and `DS:0x27C7` holds that value for the draw to compare against.
* `div bl` is UNSIGNED, which is why the `js` before it is load-bearing: a
  negative `dy` would divide as a large positive and land on a row instead of
  missing.

The 4px inset is shared with the DRAW, which is what keeps the clickable band from
drifting off the drawn one — the same failure #148 removed by making the nav
sector's draw and click share one expression.

Cited instructions 187 -> 194; the queue 69 -> 68.

## #155 — citing the constant, not just the function that uses it

#154 documented `hit_test_ship_3d_target_list` and the settle tool immediately
REFUSED `SHIP_3D_TARGET_HIT_TEST_TOP_INSET`: "ASM needs a cited address". Correct
— the citation had gone on the FUNCTION, and the constant is its own ledger row
with its own evidence requirement.

That distinction is not bookkeeping. A constant used in one place today gets used
in three tomorrow, and the reader who finds it at its definition sees `= 4` with
nothing attached. Four now carry their own instruction:

    SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT  200  sub bp,0xc8  @0x84B9
    SHIP_3D_TARGET_HIT_TEST_TOP_INSET      4  add cx,4     @0x84E6
    SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET   8  sub bp,8     @0x84FF
    SHIP_3D_TARGET_HOVER_PRESENTATION_MODE 6  cmp [0xa34],6 @0x850F

The pairing is the useful part: `add bp,8` in the layout and `sub bp,8` in the hit
test are the same 8, which is why the box is drawn larger than it is clickable.
Documented apart, they are two magic eights; documented together, one is the
chrome and the other is its exclusion.

The tool refusing to settle a row whose evidence sits somewhere else is the
behaviour that made this visible, and it is worth keeping strict for exactly that
reason.

## #156 — the gate that runs when the bit is CLEAR

`update_ship_3d_nav_choice_dispatch`, the dispatcher behind #109's handler table:

```text
  0x86F1  test byte [0x2793],8 / jne 0x8705   bit 3 SET -> return, do nothing
  0x86F8  dec bx / add bx,bx                  choice -> zero-based, then *2
  0x86FB  test byte [0x2565],1                the phase bit, NOT branched on
  0x8700  call word cs:[bx+0xf29]             the per-row handler table
```

Two readings a port gets backwards.

The gate is inverted. `jne` skips the dispatch when bit 3 is SET, so the
dispatcher runs only while the bit is CLEAR. `0x86A4` — the click — ORs `0xC`
into that same word, bits 2 AND 3, which is how a click both arms the surface and
suppresses the dispatcher until the click is handled. Read as "run when set", the
whole interaction inverts: the dispatcher would fire on the frames it is supposed
to sit out.

And `[0x2565]` is TESTED without a branch. The flags go to the HANDLER, which is
why each `run_ship_3d_nav_choice_handler_*` opens by examining the phase itself
rather than being called only when the phase is set — the dispatch is
unconditional once the gate passes.

Cited instructions 194 -> 198; the queue 68 -> 67.

## #157 — the guard caught me writing an instruction x86 cannot encode

`project_ship_3d_point` (from `ship_3d_point_cloud_project` @`0x9A10`) translates
by the camera origin and dots with the matrix's third row:

```text
  0x9A31  mov bp,0x2f95                      the matrix built at 0x98B9
  0x9A3F  mov ax,[0x2f65] / sub [di],ax      translate by the ORIGIN
  0x9A44  mov ax,[0x2f67] / sub [di+2],ax
  0x9A4A  mov ax,[0x2f69] / sub [di+4],ax
  0x9A50  movsx eax,[di] / imul eax,[bp+0x18]   DEPTH first: the third row
```

The matrix is nine 32-bit terms, so `[bp+0x18]` is term 6 — the depth row is
`terms[6..=8]`, which is why the port computes depth from those and not the first
row. The translate is a plain 16-bit `sub` BEFORE any widening, so a point far
from the origin wraps rather than saturating, and the `movsx` then sign-extends
whatever that left.

My first version of this doc wrote the translate as

    0x9A3F  sub [di],[0x2f65]

which is wrong twice over: the mnemonic at `0x9A3F` is `mov`, and `sub mem,mem` is
not an encodable x86 instruction at all. `check_cited_instructions.py` flagged all
three lines immediately.

That is the fifth time this session a guard has caught my own citation rather than
someone else's (#100, #101, #141, #147, this). The value of a guard that runs on
every commit is not that it catches the careless — it is that it catches the
person who has just spent an hour in the disassembly and is confident.

Cited instructions 198 -> 202; the queue 67 -> 66.

## #158 — the nav destinations start at entity 0x15, and the loop runs backwards

`project_ship_3d_object_sprite`, from `ship_3d_object_sprite_project` @`0x9B98`:

```text
  0x9BBA  dec word [0x2f77]              the object counter, walked DOWN
  0x9BBE  js 0x9CFB                      negative -> done
  0x9BD1  mov ax,[0x2f77]
  0x9BD4  add ax,0x15                    + SHIP_3D_NAV_ENTITY_BASE
  0x9BD7  shl ax,5                       * 32, the entity stride
  0x9BDA  add ax,0x6212                  + SHIP_3D_ENTITY_TABLE
```

Two facts a port needs and neither is visible from the port's own code.

The nav destinations do NOT occupy entity slots `0..n`. They start at `0x15`,
sharing the entity table with whatever holds the low slots — so an off-by-`0x15`
does not fail loudly, it writes over other objects.

And the counter is decremented BEFORE use, so the loop indexes `n-1..0`. Combined
with the plot's first-write-wins rule (#149), the ORDER decides which destination
survives when two project to the same pixel. Walking up instead of down is a
change no test would catch and the wrong sprite would win.

Cited instructions 202 -> 209; the queue 66 -> 65.

## #159 — a point list that must still write the buffer

Two more rows off #141's queue.

`menu_submenu_click` delegates to `choice_box_row_at`, so its basis is the widget
hit-test cited in #154 — `div bl,0x0B` @`0x8508`, rows stepped by `add bp,0xB`,
the 4px origin inset at `0x84E6`. Its labels come from the same source the DRAW
uses, which is what stops the clickable band describing a menu the screen is not
showing.

`ship_3d_point_cloud_points` returns the starfield as a POINT LIST for the GPU
path rather than a rendered buffer — and it still allocates and writes the buffer.
That looks like waste and is not: the plot's first-write-wins gate (#149) means
whether a point is EMITTED depends on what earlier points already wrote. Testing
coordinates alone would emit points the game discards, and the divergence appears
only where the field is dense — which is where a starfield is interesting.

So the buffer is not an implementation detail of the rendering path; it is part of
the selection rule. Removing it is the obvious optimisation for a function that
returns a list, and it would silently change which stars exist.

The queue: 65 -> 63.

## #160 — an unsigned compare against -2

`ship_3d_binary_sqrt` — `binary_u32_sqrt` @`0x2E33`, the helper behind object
distances — seeds its estimate from the input's magnitude:

```text
  0x2E3B  or dx,dx / je 0x2E4F      high == 0 ?
  0x2E3F  mov bx,0xfff              high != 0: estimate 0x0FFF
  0x2E42  or dh,dh / je 0x2E5C      ...unless the TOP byte is set
  0x2E46  mov bh,0xff               then 0xFFFF
  0x2E48  cmp dx,-2 / jae 0x2E6E    high >= 0xFFFE: return the input
  0x2E53  mov bx,0xf                low only: 0x000F, or 0x00FF if ah is set
```

Two things worth having in writing.

`mov bh,0xff` does not load `0xFFFF` — it OVERWRITES the high byte of the `0x0FFF`
already in `bx`. The value is the same; the shape tells you the ladder is refining
one estimate rather than choosing between four.

And `cmp dx,-2 / jae` is an UNSIGNED compare, so it reads "the high word is at or
above `0xFFFE`", not anything about negative numbers. That is the case where the
root will not fit a `u16`, answered by returning the input — and `jae` rather than
`jge` is the whole distinction. A port that reached for a signed comparison here
would take the early exit almost never, and only for inputs it will not see.

Cited instructions 209 -> 217; the queue 63 -> 62.

## #161 — two coordinates, two different field selectors

`ship_3d_position_field_distance` is `sqrt(dx^2 + dy^2)` and takes its coordinates
as arguments. `ship_3d_position_distance` @`0x60DD`, the routine it comes from,
spends most of its length deciding WHERE those coordinates live:

```text
  0x60E5  cmp ax,0x100 / jne 0x6114   the first object must be kind 0x100
  0x60EA  mov bx,[di]                 the second object's kind
  0x60EC  mov ax,0xe / call 0x6023    vm_field_offset(selector 0xE, that kind)
  0x60F4  mov dx,[bx+di]              read the field it resolved to
  0x60F6  mov bx,0x100 / mov ax,0xc / call 0x6023   selector 0xC for kind 0x100
```

The two coordinates come from DIFFERENT selectors — `0xC` for the kind-`0x100`
object and `0xE` for the other — each resolved per kind through `vm_field_offset`
(`0x6023`, the `BSF`-column resolver whose kind argument is a BITMASK, not an
index). Kind `0x100` is `vm::LOCATION_KIND_BLACK_HOLE`, the same bit the status
header tests, which is a connection nothing in either file previously made.

The port splits this cleanly: resolution belongs to the VM's field machinery, and
this function does the arithmetic. That is a reasonable division and it left the
selector pair undocumented on both sides — the VM knows how to resolve a selector,
this knows how to measure a distance, and only the routine knows that distance
means `0xC` against `0xE`.

Cited instructions 217 -> 222; the queue 62 -> 61.

## #162 — four small helpers, and the one that exists because 8 bits is not 16

Four more rows, all small functions whose whole content is a decoded detail.

`ship_3d_plane_band_byte_count` — `(depth + 35) * 80` from `0xB71C`. The add
happens in 8 bits, because `mul dl` takes AL, which is why the port wraps the row
count as a `u8` before multiplying instead of widening first.

`ship_3d_scroll_value` — `100 - min(2*depth, 100)`, built as `sub ax,0x64 / neg
ax` rather than a reversed subtract, reaching 0 exactly when the depth passes 50.
Scroll mode `0xA` skips it entirely.

`start_closing_transition` — the three writes at `0xB6B8` that always occur
together: close step, closing flag, disarmed. One function rather than three
assignments at the call site, because in the original they are one branch.

`add_to_low_byte` — the reason this function exists at all is that the original's
`add` is 8-bit on word-sized state, so a wrap does NOT carry into the high byte. A
16-bit add would, and would be the natural way to write it. This helper is the
port refusing that.

That last one is the clearest case of what this queue keeps turning up: a function
whose entire justification is a CPU width, invisible in Rust, and undocumented
until now. The name says what it does; nothing said why anyone would want it.

The queue: 61 -> 58.

## #163 — `sar`, not `shr`, and the shift that happens once

Two more helpers, both about arithmetic that Rust makes look automatic.

`fixed_mul_shift_15` is the Q15 multiply: `imul` then `sar eax,0xf`
(`0x9957`/`0x995F` in `matrix3d_mul_fixed` @`0x994D`). The instruction is `sar`,
an ARITHMETIC shift, so a negative product keeps its sign instead of becoming a
large positive. Rust's `>>` on `i32` is arithmetic and matches — but only because
the argument is typed `i32`. The same expression over `u32` compiles to the wrong
instruction and stays silent about it, which makes the TYPE the load-bearing part
of a one-line function.

`projection_dot` accumulates three products in 32 bits with NO intermediate
shift — `imul eax,[bp+0x18]` / `mov ecx,eax` / `imul eax,[bp+0x1c]` / `add ecx,eax`
at `0x9A50..0x9A66`. The Q15 shift happens once, on the result. Shifting per term,
which is what a naive "multiply in fixed point" helper does, discards the low bits
of each product before they are summed.

Neither is a bug in the port; both are decisions the port makes correctly and
records nowhere. The queue: 58 -> 56.

## #164 — the perspective divide, and what `cdq` is doing there

`project_ship_3d_axis` is `numerator / depth + center`. The routine at
`0x9AD9..0x9AE2`:

```text
  0x9AD9  sar eax,7      pre-scale the dotted numerator
  0x9ADD  cdq            sign-extend into edx:eax
  0x9ADF  idiv ecx       divide by the DEPTH
  0x9AE2  add ax,0x64    + the screen centre (100)
```

`cdq` is not bookkeeping. `idiv` divides `edx:eax`, so without the sign extension
a negative numerator divides as an enormous positive and the point lands off
screen instead of on the other side of centre. In Rust the sign comes free from
the `i32` type — which means the correctness rests on the SIGNATURE, and a
plausible refactor to `u32` would change the behaviour without touching a single
operator. (Same hazard as `sar` vs `shr` in #163, one type away.)

The `+ 0x64` comes AFTER the divide, so the projection yields an offset from
centre, not an absolute coordinate.

`scale_ship_3d_object_dimension` widens to 32 bits BEFORE multiplying, because the
product of two words overflows 16 bits routinely — the original keeps it in `eax`
for that reason, and the port's `u32::from` on both operands is that, not caution.

The queue: 56 -> 55.

## #165 — the dirty list ends at a SIGN, not at 0xFFFF

`collect_ship_3d_dirty_sprite_slot_render_commands` —
`sprite_slot_dirty_range_render` @`0x4471`:

```text
  0x448A  mov di,0x6612          the dirty-rect list
  0x448F  or ax,ax / js 0x4516   TERMINATED BY SIGN
  0x4495  mov bx,bp / shr ebp,0x10   unpack the slot range
  0x44A2  mov di,0x6212 / shl bx,5 / add di,bx / sub di,0x20
```

The terminator test is `js`, so ANY negative word ends the list. `0xFFFF` is
merely the value the writers happen to use (`0x1001`, `0x4429`). A port comparing
against `0xFFFF` exactly agrees on every list the game builds and disagrees on any
other negative sentinel — the kind of difference that survives all testing until
the one data set that uses a different one.

The slot walk also runs BACKWARD (`sub di,0x20` per step), which matters for the
same reason #158's backward object loop did: with first-write-wins plotting, order
decides what survives.

`re/tools/find_imm.py` needed fixing to get here — `--max 4` left the `4` in the
positional list, where it was read as a filename and raised
`FileNotFoundError: '4'`. Flags now consume their values.

And `check_cited_instructions.py` caught this doc claiming `0x4495` is `shr` when
it is `mov bx,bp` (the `shr` is at `0x4497`) — the sixth time this session it has
corrected my own citation rather than someone else's.

## #166 — two dwords, not four words, and a bound the port added

`commit_ship_3d_global_clip_snapshot` (`0x4412`) copies the four clip bounds as
TWO dwords:

```text
  0x441D  mov eax,[0x5235] / stosd   left+right as ONE dword
  0x4423  mov eax,[0x5239] / stosd   top+bottom as one more
  0x4429  mov word [di],0xffff       terminate
  0x442D  mov word [0x5249],0        and CLEAR the flag
```

`DS:0x5235..0x523B` are contiguous, so left/right and top/bottom each pair into a
single 32-bit move — which is why the port reads them as pairs rather than four
separate words. And the flag is ONE-SHOT: the clear at `0x442D` is what stops
every subsequent frame becoming a full-window redraw. Dropping it produces output
that looks perfect and defeats the dirty list entirely.

`matrix_pair_for_angle` doubles each table entry because `0x990C` does
(`movsx` then `add ebx,ebx`), turning the Q14 table into Q15 terms. Its `None` for
an out-of-range angle is NOT the game's behaviour — the game indexes after a
modulus and would read past the table — and saying so matters, because an
undocumented `Option` reads as "the original had a failure case here" when it is
the port declining to reproduce an out-of-bounds read.

The queue: 54 -> 52.

## #167 — a half-open clip, and the one input where abs has no answer

`plot_ship_3d_projected_point` (`0x9B04`) rejects points outside the clip with
`jl` on the low bounds and `jge` on the high:

```text
  0x9B0A  cmp ax,[0x5235] / jl    reject left of the clip
  0x9B10  cmp ax,[0x5237] / jge   reject at or past the right
  0x9B19  cmp bx,[0x5239] / jl    y against 0x5239/0x523B likewise
```

Both are SIGNED, which is why the port compares through `signed_i16` instead of on
`u16` — a point behind the camera arrives as a large unsigned value, and an
unsigned compare would place it on screen rather than rejecting it. And the pair
`jl`/`jge` makes the clip HALF-OPEN: `left` is inside, `right` is not. Symmetric
bounds are the natural way to write it and would draw one column too many.

`binary_abs_word_diff` negates when bit 15 is set — a sign-bit TEST, not a signed
comparison. The consequence is that a difference of exactly `0x8000` negates to
itself and stays `0x8000`: the single input for which "absolute value" has no
representable answer. The original does not special-case it, so neither does the
port, and now that is a recorded decision rather than an accident waiting to be
"fixed" with a `saturating` call.

The queue: 52 -> 50. Cited instructions: 235 -> 239.

## #168 — the opcode "families" are decode groups, not handler groups

`is_record_entry_opcode`, `is_record_state_opcode` and `is_pair_record_opcode`
read like three of a kind. Against the dispatch table they are not:

```text
  0xB8 0xB9 0xBD      -> 0x6B06                           ONE handler
  0xC1 0xC2           -> 0x6B4C, 0x6E34                   two
  0xC5 0xC6 0xC7 0xC8 -> 0x6D18, 0x6D80, 0x6DCF, 0x6F62   FOUR
```

Only the pair-record trio is a behavioural family. `C5..=C8` is four distinct
handlers that happen to share an OPERAND LAYOUT — and that is all the token
decoder needs, since it walks the stream deciding how many bytes to consume, not
what they will do.

The distinction is worth writing down because the code does not carry it. Three
predicates in the same shape invite the reading that they mean the same kind of
thing, and acting on that — merging the `C5..=C8` handlers, or splitting the
pair-record trio — would break in opposite directions. The grouping criterion is
invisible from the port and obvious from the table.

The queue: 50 -> 48.

## #169 — the active flag is tested as a sign

Three A6-handler helpers, all resolved by two instructions in `0x660C`'s body.

`text_flags_are_active` checks `b5 & 0x80`. The game does not mask: `lodsw`
@`0x661B` reads `b4` and `b5` together into `cx`, so bit 7 of `b5` is bit 15 of
the pair, and the test is `or cx,cx / jns 0x67A0` @`0x6647` — the SIGN of the
word. Same predicate, and the shape explains why the port's parameter is `b5`
alone while the original never separates them.

`text_line_flags_offset` and `text_line_already_shown` both come from
`test word es:[di+2],0x8000` @`0x665A`: the flags word is at line record `+2`, and
`0x8000` is the already-shown bit. `di` is the record the handler resolved at
`0x6613` (`les di,gs:[0x6724]`, then `add di,ax` with the line index).

The detail worth keeping is where those two branches GO. `jns` on the active test
and `jne` on the already-shown test both jump to `0x67A0` — the same exit. A line
that is inactive and a line already displayed are not distinguished downstream;
they leave by one door. A port that gave them separate paths would be adding a
distinction the game does not make, and would look more careful for it.

The queue: 48 -> 45.

## #170 — high-bit-first is not a convention, it is a consequence

`bit_flag_byte_offset` and `bit_flag_mask` implement `0xB7`'s bit addressing:
bit 0 is mask `0x80`, bit 7 is `0x01`, bit 8 starts the next byte at `0x80`. The
existing comment stated that; nothing said where it came from.

The byte split is plain (`0x6AC0`: `and cl,7` for the bit, `shr ax,3` for the
byte). The ORDER falls out of how the handler tests the bit:

```text
  0x6AD0  mov al,es:[bx+di]
  0x6AD3  shl al,cl        shift the target bit up by (bit & 7)
  0x6AD5  shl al,1         once more, into CARRY
  0x6AD7  jae 0x6AE2       carry clear -> the bit was 0
```

Bit 0 reaches the carry after a SINGLE shift, so bit 0 must be the byte's high
bit. High-bit-first is not a choice the game made and the port copied — it is
what `shl`-into-carry means, and `0x80 >> (bit & 7)` is that sequence rewritten
as a mask.

Worth the distinction: a stated convention invites "surely this should be
`1 << n`", and the answer is that `1 << n` would require the test to be `shr`
instead. The mask and the shift direction are one fact.

The queue: 45 -> 44. Cited instructions: 239 -> 246.

## #171 — the zero that is a register, not a literal

`record_entry_stored_related_offset` returns `0` for opcode `0xC8` and the operand
for everything else. The port states the exception; the handler explains it.

`0xC8` (`0x6F62`) reaches its set path only through a guard:

```text
  0x6F9A  mov bx,es:[bp]        the record's first word
  0x6F9E  or bx,bx / jne 0x6FB4 NON-empty -> vm_branch instead of writing
  0x6FA2  mov word es:[bp],0xc8 write the type
  0x6FA8  mov es:[bp+2],bx      ...and BX, which the guard just proved is 0
  0x6FAC  mov word es:[bp+4],0
```

The related word is not written as a constant — it is written from `bx`, which
reached that line only because `or bx,bx` found it zero. The value and the guard
are the same fact.

So `0xC8` writes an EMPTY record: it fires only on an already-empty slot, and
stores that emptiness back. Writing the operand there instead — which is what
every sibling opcode does, and what a reader unifying the family would reach for —
would put a value in a field the game guarantees empty.

That is the fourth time in this queue where the original's shape encodes a
constraint the port's cleaner spelling drops: the compare-before-write (#150), the
`btr` carry (#151), the sign-terminated list (#165), and now a zero that is a
proof.

The queue: 44 -> 43. Cited instructions: 246 -> 251.

## #172 — the port was missing a built-in object, and nothing could have noticed

`VmNamedObjectOffsets` resolves the engine's built-in objects by name. Checking
whether that was faithful — the game might have used fixed indices — turned up
better than a citation.

The game DOES match by name: `0x5490` loads a name pointer, `lcall 0x1CE:0x2C4`
compares, and on a match `mov gs:[0x674e],ax` @`0x549D` stores the object's
`[si+0x10]` offset into that built-in's global. The names are packed
NUL-terminated strings from `DS:0x67BE`:

```text
  0x67BE blood    0x67C4 orxx      0x67C9 Honk    0x67CE menu
  0x67D3 arche    0x67D9 cryobox   0x67E1 Ark     0x67E5 Scruter_Jo
  0x67F0 vbio
```

NINE names. The struct had EIGHT — **`cryobox` was missing**. An object the engine
resolves and assigns a global was not resolved by the port at all.

Nothing could have caught this. The struct is self-consistent, its `set` returns
`false` for unknown names so nothing errored, and every test that used it passed:
the absent field simply meant one built-in never got an offset. The gap was
visible only by enumerating the GAME's table and comparing.

Added, with a test that reads the table out of the image rather than restating it
— so a tenth built-in in the data, or a ninth removed from the port, now fails.

My first attempt at reading the table assumed a 6-byte stride and produced
`'onk'`, `'nu'`, `'he'`, `'obox'` — mid-string tails. The entries are packed by
length, not aligned, which is exactly the sort of assumption that makes a table
look shorter than it is.

The queue: 43 -> 41.

## #173 — the guard was right about my own documentation

`owner_object_offset` exists twice in `vm.rs` — once on the execution context,
once on `VmMachine` — because both types hold their own `object_offsets`. Both
are one-line delegations to `owner_object_offset_in`, which carries the `0x6034`
rule. They are plumbing, and the recurring "STILL ambiguous" warning from
`audit_inventory.py` was about exactly that pair.

Settled both as INFRA (using the `file:line:item` form #133 added, which is what
made them settleable at all). Then `check_duplicate_rules.py` FAILED the build:

    49 addresses cited by more than one port function

because I had written `0x6034` into BOTH docs. One name, one address, two
implementations — which is precisely the signal that guard exists to raise, and it
could not know the two are delegations rather than copies.

The fix is not to weaken the guard. It is that a delegation should not carry the
citation at all: the address belongs to the helper holding the rule, and repeating
it on both wrappers manufactures the appearance of a duplicated decode. Removed
from both, with a note saying why.

Also extended `classify_plumbing.py` to private functions — it only scanned
`pub fn`, so these two were invisible to it. That found nothing new today (the
unsettled set no longer contains any), but the restriction was arbitrary.

The queue: 41 -> 40.

## #174 — I inserted a function between a doc and the thing it documented

`bridge_nav_destination_click` was still in the uncited queue despite #148 giving
it a full doc with a citation. The doc was there; it was attached to the wrong
function.

#148 added the `bridge_station` helper and placed it BETWEEN the existing doc
comment and the function that doc described. Rust attaches a doc to whatever item
follows it, so `bridge_station` inherited the click handler's documentation, the
click handler was left bare, and the two texts ran together mid-sentence:

    /// `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`).
    /// The station the current panorama frame belongs to, read from its header.
    pub fn bridge_station(&self) -> Option<u16> {

Reordered so each doc precedes its own function. `bridge_nav_destination_click`
now carries `0x8508` and `bridge_station` has a doc of its own.

This is the same failure mode as #141's blank-line problem and #119's note
adopting the next constant — three instances now of documentation attaching to the
wrong item, each introduced by an edit that was correct about everything except
placement. The ledger caught all three, which is an argument for its origin column
being derived from the source rather than maintained by hand: a hand-kept citation
would have stayed with the function and hidden the mistake.

## #175 — six helpers whose basis is a trap the CPU takes

Six small functions documented, and two are worth naming.

`checked_i16_div_i8_to_i8` models `idiv bl` — AX by an 8-bit divisor, quotient in
AL. Its `Option` is not defensive programming: it covers the two cases the CPU
TRAPS on, a zero divisor and a quotient too large for AL. The port stops where the
game would fault, instead of continuing with a wrapped number. That is a decision
about what to do at a boundary the game never crosses, and reading the signature
alone it looks like ordinary caution.

`signed_i16` exists because the game's coordinate and clip tests are `jl`/`jge`.
A point behind the camera is simultaneously a large unsigned number and a small
negative one, and only the second is correct — so port comparisons must route
through this rather than comparing `u16`s. A one-line cast whose absence would be
invisible in review and wrong on screen.

The other four are derivations from already-cited facts: the presentation record
offset is `line + TALK_FIELD`; `is_record_state_opcode` and
`is_global_compare_opcode` are token-shape groups over opcodes with SEPARATE
handlers (`0x6B4C`/`0x6E34`, `0x64E5`/`0x6510`); `bit_flag_mask` is the mask form
of the `shl`-into-carry test at `0x6AD3`.

The queue: 40 -> 35.

## #176 — the bytecode modifies itself, and the port keeps a side table

`TextTokenRuntimeFlags` had no doc, and what it does is one of the more
surprising facts in the VM: the A6 handler MODIFIES ITS OWN BYTECODE. On accepting
a line it clears bit 7 of `b5` in the COD stream (`and byte [si+1],0x7F` after
`0x668D`, unless `b4 & 1` preserves it), so a line that has played will not
display again.

The port cannot write to the shipped script, so this type holds the modified `b5`
per stream offset and `flags_b5` reads through it. Same observable behaviour by a
different mechanism — and worth writing down twice over. A reader will not assume
the bytecode is self-modifying, and a port that treats the stream as read-only
WITHOUT a side table replays every accepted line.

Both functions now carry `0x668D` individually. Putting the address only on the
`impl` block left both rows uncited, which is #155's lesson again: the ledger row
is the function, and a citation one scope up is invisible to it.

The queue: 31 -> 29.

## #177 — `0xA1` is a family-wide prefix, not a `0xC4` quirk

The `0xC4` actor family, from the handler at `0x6C7E`:

```text
  0x6C86  mov al,[si] / cmp al,0xa1 / jne   the 0xA1 PREFIX...
  0x6C8C  inc dl / inc si                   ...sets the INVERT flag
  0x6C92  call 0x6034                       resolve the record's owner
  0x6C98  mov cx,es:[bp]                    the record's TYPE word
  0x6C9C  test byte gs:[0x67ad],1 / je      query or set
```

A query matches on THREE conditions together: the owner active, the type word
`0xC4`, and the stored related offset equal to the operand. Any one failing is a
miss, and `0xA1` before the opcode inverts the whole result.

That prefix is not specific to `0xC4`. The same `cmp al,0xa1 / inc dl / inc si`
opens `0x6D18` and `0x6F62` — it is a family-wide modifier, which is why the port
threads `inverted` through several handlers rather than special-casing one. Worth
recording as a shared fact rather than three coincidences.

`write_actor_record` writes ZERO at `+4` every time rather than leaving what was
there, which is what keeps a freshly written record distinguishable from one
carrying state from a previous use.

The queue: 25 -> 24. Cited instructions: 254 -> 261.

## #178 — four opcodes that decode alike and write differently

`write_record_entry_mode0` is where #168's claim becomes concrete. `0xC5..=0xC8`
share a token layout and a writer, and their SET guards are entirely different:

* `0xC5` (`0x6D18`) demands the operand object be active, its type word be exactly
  `0x0200`, and the destination record be EMPTY.
* `0xC6` (`0x6D80`) writes unconditionally.
* `0xC8` (`0x6F62`) writes only into an empty record, and stores ZERO rather than
  the operand (#171).

So the range that looked like a family in the port's predicate is four behaviours
sharing a decode length. Anyone unifying their guards — the obvious tidy, since
they already share `write_record_entry` — would give `0xC6` conditions it does not
have and `0xC8` an operand it must not store.

`write_c2_record_state_direct` (`0x6E34`) has three gates, and two are worth
naming. The `0x20` flag it tests belongs to the TARGET, not the owner — a
different object's bit decides whether this write is allowed. And the special-slot
insert is a GATE, not a side effect: a full 16-slot array declines the write
outright, which is the caller contract #175 recorded on `insert` being honoured
here.

The queue: 20 -> 18.

## #179 — an opcode is not an address

Five rows stayed in the uncited queue AFTER being documented, and the reason is
worth stating: their docs cite OPCODES (`0xC4`, `0xC9`) and the ledger's address
pattern needs three or more hex digits. `0xC4` is a value; `0x6C7E` is where the
code that acts on it lives.

That is the right rule, not a technicality. "This writes a `0xC4` record" describes
the data; "handler `0x6C7E`" says where to go and check. Only the second is a
citation, and a reader following the first has nowhere to look.

Five docs gained the handler they derive from — `write_actor_record` `0x6C7E`,
`clear_record_words` `0x6FB9`, `ship3d_record_state_slot` `0x6B4C`,
`actor_record_is_active` `0x6073`, `actor_object_offset_from_record` `0x660D` —
and the queue fell 18 -> 13, since several other rows in the same files were
carrying the same gap.

The queue has now gone 91 -> 13 across #142-#179. What remains is mostly functions
whose basis genuinely is another function (delegations, and helpers over
already-cited constants) rather than a routine of their own.

## #180 — 91 rows down to 6

The uncited-ASM queue #141 opened at 91. It now stands at 6.

The last stretch resolved two ways, and the split is the useful record:

**Cited.** Rows whose basis is a routine got its address — `0x6EEE` for the `0xC3`
writers, `0x6D18`/`0x6D80`/`0x6DCF`/`0x6F62` for the entry family, `0x5816` for the
kind-1 post-update, `0x6B4C` for the ship-3D slot layout.

**Reclassified.** `record_owner_object_offset` and `bridge_station` are
delegations, and per #173 a delegation must NOT carry the citation — the address
belongs to the helper holding the rule, and repeating it manufactures a duplicate.
INFRA is the honest status: there is nothing in them to verify.

That distinction is the whole shape of this queue. A row settled `ASM` with no
citation is either a decode nobody wrote down (most of the 91) or a function with
no decode in it (a handful). Treating them alike — bulk-citing or bulk-downgrading
— would have been wrong in both directions.

What the 91 produced along the way: a missing built-in object (#172), a live bug
in the operator ladder (#126), a refuted rule in a dead function (#127), a wrong
`0x2B13` reading (#128), and roughly two dozen decisions that were correct,
undocumented, and one plausible edit from being wrong — divide-before-multiply,
compare-before-write, first-write-wins, `btr`'s carry, `sar` not `shr`, the
sign-terminated list, the half-open clip, and the zero that is a proof.

## #181 — the queue closes at one

91 rows in #141, now **1**. The last several:

* `post_update_kind2_presentation_handoff_target` (`0x5816`) gates a handoff on
  FOUR flags — presentation active, plus the C2 gate, the handoff gate and the
  start lock all clear. Three separate "not already busy" flags rather than one,
  so a handoff cannot slip through a start that has begun but not finished, nor
  through another handoff.
* `c1_record_state_resolved_mode1_condition` (`0x6B4C`) and the direct comparison
  are EXCLUSIVE: operand 1 or 2 against a non-`0xC1` record resolves the owner's
  state, anything else compares words. Its `None` routes to the other path rather
  than reporting failure.
* `VmNamedObjectOffsets::set` returns whether the name is a built-in, so `false`
  is an ordinary answer used to skip non-built-ins — not an error, which matters
  because that return is exactly what hid `cryobox` (#172).

The one remaining is `select_ship_3d_target_record`, whose routine I have not
identified. Recorded as still open rather than given a plausible neighbouring
address — the whole point of this queue was that an unjustified citation is worse
than a missing one, and the last row is not the place to abandon that.

## #182 — the 33 pixels, explained

#115 withdrew a claimed bridge-starfield defect and left one number unexplained:
the port's star layer plots 33 pixels of 64000. I recorded it as "unexplained
rather than a defect", which was right and unfinished.

Instrumenting the path accounts for all of it:

    1000 points -> 758 project -> 64 inside the viewport -> 33 plotted

The last step is the plot's first-write-wins gate (#149): 31 of the 64 land on
pixels already taken. The field is sparse BY CONSTRUCTION — a cloud spread over
the full `u16` space, viewed from `0x8000`, puts most points outside a 320x200
window after the perspective divide.

The second question was whether the port regenerates the cloud correctly.
`ship_3d_point_cloud_randomize` (`0x9B67`) has EXACTLY ONE caller — the far call
at `0x0FD3`, on a setup path that first sets `[0x27D9]=1`. The game randomizes
ONCE; its starfield is stable for the session and does not twinkle.

The port calls the randomizer every render. That looks wrong and is not:
`starfield_seed` is a constant 17 and never mutated, so the same 1000 points come
back each frame. Equivalent behaviour, opposite mechanism — and the note now says
so, because "randomize every frame" invites either a caching optimisation or a
per-frame seed, and the second would add shimmer the original never had.

One real divergence, recorded rather than fixed: the game seeds from the RTC, so
its star positions differ per session. The port's fixed seed makes them
reproducible, which the oracle comparisons rely on.

## #183 — mixing N sources, and stopping before the part I cannot verify

#135 recorded that the game MIXES by averaging while the port runs three
independent cpal streams the driver sums at full amplitude. The primitive was
decoded; nothing used it.

`mix_unsigned_pcm_sources` now applies it to N sources, and writing it settled two
things that a naive mixer gets wrong:

* **Order matters.** `0xBB6D` averages ONE source into the destination, so mixing
  three is that applied three times — each earlier source halved again by every
  later mix, the last dominating. That is not a rounding artefact to correct into
  an equal-weight average; it is what the routine does, and the test asserts that
  swapping two sources changes the result.
* **Silence is `0x80`, not `0`.** Unsigned PCM's zero level is mid-scale; starting
  the buffer at `0` would drag every mix toward full-negative.

A shorter source stops contributing without truncating the mix, so the output
length is the longest source.

What I did NOT do: rewire `audio.rs`. That means replacing three cpal streams with
one mixed stream, and this environment has no audio device — the change would be
unverifiable here, and an unverifiable edit to the output path is precisely what
this campaign should not make on faith. The port-validation row now says the rule
is decoded and tested at both levels and names the remaining work as the rewiring.

Same judgement as #138: implement what can be verified, state the blocker, do not
bridge it to make the pieces appear connected.

## #185 — twelve handlers, twenty-eight names

The `vm_op_*` labels were the largest remaining duplicate cluster: twelve
addresses carrying two or three names each, twenty-eight rows for twelve
routines. `0x6B06` alone had `vm_pair_record_6b06`, `vm_op_b8_record_compare` and
`vm_op_b8_record_readwrite`.

They are not disagreements — each name is the same handler described at a
different stage of understanding. `..._compare` is what `0xB8` does in QUERY mode;
`..._readwrite` is what it does across both modes, which is the fuller reading and
the one kept. `vm_pair_record_6b06` puts the address in the name, which is what
you write before you know what the routine is for.

Merged by union, keeping the name from the longest comment (the fullest decode)
and folding the others in as `ALSO RECORDED as`. Twenty rows removed; the
duplicate count falls 28 -> 16, and 55 -> 16 since the check was written in #129.

Checked afterwards that `check_opcode_handlers.py` still resolves all 29 opcode
constants through the dispatch table — the merge touched the labels, not the
citations, and that is the guard which would notice if a handler address had been
lost.

## #186 — zero duplicate addresses, and one that was two halves

The duplicate-address count is **0**, from 55 when the check was written in #129.

The last sixteen merged by union. One deserved a closer look first: `0x0B591`
carried `ship_3d_temp_snd_setup` ("temporary sn\3D.snd path") and
`alien_overlay_cycle` ("the {amer, croolis, scrut}.xdb overlay CYCLE"). Those read
as a genuine disagreement — different subsystems entirely.

They are both right. Reading the data settles it:

```text
  DS:0x0ACC -> 0x0087 'amer.xdb'   0x0090 'croolis.xdb'   0x009C 'scrut.xdb'
  DS:0x0D23 -> 'sn\3D.snd'
  0xB5B5    inc ah / cmp ah,3 / jne / xor ah,ah    [0x0AE5] cycles 0..2
  0xB5DC    lcall 0xB1B:0x855                      the SND bank load
```

One routine cycles the alien overlay AND swaps the SND bank. Each label described
a different half, and neither author was wrong — which is exactly the case that
looks most like a conflict and is not.

Union merging is what made this safe to do in bulk: it keeps every claim, so the
only judgement is which NAME leads. A "keep the better one" merge would have
discarded a true statement in this row and I would not have noticed.

`check_opcode_handlers.py` still resolves all 29 opcode constants afterwards.

## #187 — two constants that live in a table, not an instruction

`TALK_FIELD = 0x3A` and `LOCATION_FIELD = 0x18` were both NEEDS-READING, and both
for the same reason: they are ENTRIES IN THE FIELD MATRIX at `DS:0x6D60`, so no
instruction carries them as an immediate.

`LOCATION_FIELD` sits at `[selector 6][column 2]` (and `[9][8]`), reached through
`vm_field_offset` (`0x6023`). `TALK_FIELD` is `[0x13][1]`, and `0x6664` fetches it
WITHOUT the resolver:

```text
  0x6664  mov ax,0x13          selector 19
  0x6667  shl ax,4             * 16
  0x666A  inc ax               column 1
  0x666D  mov al,gs:[bx+0x6d60]
```

Column 1 is kind bit 1 — kind 2. The code knows the kind at that point, so it
hardcodes the column instead of resolving it, which is why the constant looked
uncited: the citation was right and pointed at a table read.

`field_matrix_entries_match_the_constants` now reads the matrix out of the image
and pins both, plus the observation that selector 0 is uniform `0x02` across its
live columns — a field every kind shares, which is worth a failing test if it ever
stops being true.

Both settled DATA. That is the fourth structural class the immediate checker
cannot see, after dispatch indices, shift counts and layout identities: a value
fetched from a data table by an indexed read.

## #188 — a constant stored as its own negative

`LOCATION_PANEL_TINT_PERCENT = 50` had no `0x32` anywhere near its citations. It
is there as `0xFFCE`:

```text
  0x90ED  mov ax,0xffce        the caller passes -50
  0x22F1  neg ax               the blend builder negates on entry -> 50
  0x22F5  mul bx / mov bx,0x64 / div bx   ... * component / 100
```

`0x64` is the 100 it divides by, confirming `ax` is a percentage. Neither address
holds the value on its own: the caller has the negative, the builder makes it
positive, and a search for 50 finds nothing at either.

`check_cited_immediates.py` now looks for the two's complement — the fifth
structural class it handles, after shift counts, operand addresses, layout
identities and sums.

The first version searched 8-bit negations too, and immediately "found"
`OP_MAX` (`0xFE` negates to `0x02`) and `TALK_FIELD` (`0x3A` -> `0xC6`). Both are
ordinary values that appear near almost anything. `0xFFCE` is distinctive;
`0x02` is not, and a rule that accepts either is not a rule. Restricted to 16-bit,
where exactly one constant matches — the one this was written for.

## #189 — a quotient that has to come out exact

Two more constants off the NEEDS-READING list, neither an immediate and each for
a different reason.

`TEXT_SPEED_STEP_INITIAL = 2` is a DATA value: the byte lives in the initialised
data segment at `DS:0x0ACA` (file `0x0DEEA`), so no instruction carries it. Its
existing test already reads it back out of the image; the doc now says that is why
the immediate checker cannot see it.

`ANGLE_UNITS_PER_FRAME = 8` is DERIVED —
`ANGLE_UNITS_PER_REVOLUTION / PANORAMA_FRAME_COUNT` = `0x5A0 / 180`. Both operands
are cited (`add bx,0x5a0` @`0x9807`, and the panorama's own directory length), and
the new test checks the division leaves NO REMAINDER.

That last part is the actual verification. A quotient asserted as `8` proves
nothing; a quotient that must divide exactly fails the moment either operand is
wrong. The same test also pins that the station rest angles (`0x000`, `0x05A`,
`0x0B4`, `0x10E`) are all even, since #107 established the frame is half the
recorded angle — an odd one would mean a station resting between frames.

## #190 — the width is the byte

`GAME_FONT_WIDTH = 8` is not an immediate anywhere, and asking why settles it: a
glyph ROW IS ONE BYTE, so the width is that byte's bit count.

The row table starts at `0x14D28` and runs `86 * 8` = 688 bytes, ending exactly at
`0x14FD8` where a differently-shaped table begins. The last eight bytes are a real
glyph bitmap (`00 7c 82 82 7e 02 7c 00`, recognisably an 'a'), not padding.

The test checks the extent against the image and asserts the last glyph has both
ink and blank rows — a zero-filled tail past a mis-sized table would fail that,
which is the failure mode a pure length check would miss.

Same shape as `DIALOGUE_FONT_ASCII_MAP_LEN` (#105): the font's constants are
LAYOUT, and layout is checkable by arithmetic on addresses the tree already knows.
That is now four font constants pinned to the image rather than to each other —
which matters because a self-consistent font is exactly what hid the 128-vs-176
truncation for a whole campaign.

## #191 — a stride is a shape

`WORLD_ART_RECORD = 0x16` is not an immediate; it is the record's LAYOUT — a
16-byte NUL-padded name followed by three `u16` fields (id, group, extra), which
is 22 bytes.

The data shows it directly: `Kortex` at `+0`, `Kukaracha` at `+0x16`, `Ekatomb` at
`+0x2C`. A wrong stride lands mid-name on the very next record, so the test walks
eight records at the claimed stride and requires each to start with a printable
NUL-terminated name AND for the padding after that name to be actually zero.

That second assertion is the one with teeth. A stride that is too LARGE still
finds a plausible name at each step for a while; what it cannot do is keep the
name-field padding clean, because it is reading the previous record's trailing
words as part of the next name.

Settled DATA. The NEEDS-READING list is now down to constants whose basis is
genuinely elsewhere — a capture-observed frame (`CONSOLE_BAND_FRAME`), a data-
segment base with no code reference (`STATE_BASE`, #119), and the opcode family.

## #192 — the mapping I said the game did not have is a `sub`

`docs/port-validation.md` justified leaving the world-destination commit unwired
with this reasoning: the VM commit takes a target RECORD, the frontend has only a
world NAME, and "the game never needs a name->record mapping because it commits
the object the player CLICKED; inventing one in the port would be a fabricated
rule."

The premise was wrong, and the function that disproves it was the LAST uncited-ASM
row in the ledger — `select_ship_3d_target_record`. Disassembling `0xB2BB`:

  * the world-destination hit-test is not spatial. It is the unified list widget
    (`0x71E:0xC48` -> `list_widget_layout_unified` `0x8428`), the same one the
    OPTION and contact menus enter;
  * its word list `DS:0x250B` holds `RECORD+4` pointers — pointers to the name
    INSIDE each record;
  * `sub ax,4` @`0xB33D` converts the selected row back to the record.

So there IS a name->record mapping. It is subtraction, and it is the exact inverse
of the `add ax,4` @`0x87D5` that builds such a list in the first place (already
ported, as `ship_contact_menu_words`). One constant, both directions.

The fallback branch is what makes the reading certain rather than plausible. When
`DS:0x250B` is empty the widget is pointed at `DS:0x2537` with `es = ds`, so the
names are DS-relative and NOT inside records — and the code then throws the
subtraction away, returning `[0x251B]`, the current target. The author knew `sub 4`
is only meaningful for record-backed names. Because `world_click_select` rejects a
target equal to the current one, the consequence is a behavioural rule worth
stating: THE FALLBACK LIST CAN NEVER COMMIT A NEW DESTINATION.

Two corrections fell out of reading the surrounding code:

1. `world_click_select`'s doc said the back row "leaves the world view
   (`[0x24F3]=0x11`)". `0xB288` is `test byte [0x252f],1 / jne` AROUND that
   teardown, and the selector SETS `[0x252F]` when the back row is picked
   (`0xB331`). The back row SUPPRESSES the teardown. The doc had the branch
   polarity inverted.
2. `[0x252F]` has four setters, so it is not "the back-row flag". The corrected
   doc states the polarity at the one site it verified and claims nothing wider —
   the failure mode of #114 was exactly this kind of extrapolation.

Still open, and now precisely: the frontend route. `main.rs` reaches a world via
`targeted_world_name()`/`visit_world`, so no C1 record is written at runtime. The
port now has every decoded piece of the real path.

## #193 — the writer confirms the reader

#192 decoded `sub ax,4` @`0xB33D` as the name->record mapping. That reading rested
on one instruction, so the useful next step was to find the code that WRITES the
list and see whether it agrees.

`entity_candidate_list` (`0x7259`) does, and it was disassembled without reference
to `0xB2BB`:

  * `test bx,0x98` @`0x727E` — the kind mask (0x08, 0x10 SHIP, 0x80);
  * `test byte es:[di+2],2` @`0x7284` — a readiness bit;
  * `cmp di,gs:[0x6752]` @`0x728B` — exclude `arche`, so a location never offers
    itself as a destination;
  * `add ax,4` @`0x7292` — emit `RECORD+4`;
  * `mov word [bp],0xffff` @`0x729D` — terminate.

Writer and reader agree that a list entry is `RECORD+4`, from opposite directions.
That is worth more than either decode alone: neither rests on the other, so the
`+4`/`-4` pair is now confirmed by two independent instruction streams (three,
counting `0x87D5`, which builds the contact menu the same way).

Ported with a round-trip test — build the list, select a row, commit the C1 record
— so the port checks the composition, not just the halves.

WHAT I DID NOT DO, and why it is recorded rather than done: a single
`destination_candidate_records(target)` composite would be the natural API, but
`0x7259` tests the object in DI BEFORE walking the list (`mov ax,di` @`0x726F`,
`jmp 0x727B`), and `0x624B` neither saves DI nor obviously restores it across its
recursion. So DI is either the caller's target or the walk's last object, and the
two differ observably: the second can emit a candidate twice. `entity_candidate_list`
therefore takes `first` as a parameter and the composite is absent. The open
question is in `re/dead_ends.md` with the approach that would settle it.

## #194 — a prologue's push list is not a register's lifetime

#193 left DI at `0x726F` undecoded and refused to build the composite around a
guess. The evidence I had used was `0x624B`'s ENTRY push list — `push ds/si/bx/ax`,
no DI — from which I inferred DI was unpreserved and might come back as the last
object of the depth-first walk.

That inference was wrong, and reading twenty more instructions settled it:

```text
  0x6276  push di          <- saved HERE, not at the prologue
  0x6277  mov di,ax
  0x627A  call 0x624b      the recursion
  0x627D  pop di           <- restored
```

The save is local to the one instruction that clobbers DI. So `0x624B` returns DI
unchanged, `0x7259` tests the CALLER'S TARGET first, and the "emitted twice"
hazard that justified withholding the composite does not exist.

`destination_candidate_records(target)` now composes the chain, with a test
pinning both halves of the consequence: the target appears as candidate zero when
it passes the filter, and vanishes when it IS `arche` (`0x728B`).

The generalisable mistake: a routine's prologue push list answers "what does the
caller get back" only when the register is untouched elsewhere. For a register a
routine deliberately reassigns — a recursion root, a loop cursor — the save is
usually AT the reassignment, and the prologue says nothing. Read the clobber site.

Kept in `re/dead_ends.md` as RESOLVED rather than deleted: the wrong inference is
the reusable part.

## #195 — the caller names the root, so the port does not have to guess it

The destination chain was complete but rootless: `destination_candidate_records`
takes a target, and nothing in the port knew which one the game passes. The
obvious guess was `arche`. Guessing was unnecessary — the caller says so.

`0x7259` has no near callers; it is entered far as `0x4DA:0x1EB9`. Searching for
that far-call encoding found exactly two sites, `0xB0EE` and `0xB105`, both inside
`ship_click_commit`, and `0xB0EA` is `mov di,[0x6752]` — `arche`, read from the
instruction rather than assumed.

The rest of the routine decodes cleanly and settles the commit:

  * `0xB0F3  mov ax,es:[di+0x16]` — the location the arche points at;
  * `0xB0F7  mov di,[0x250b]` — the candidate list's head, read BEFORE the branch;
  * `0xB0FB  test word es:[eax],0x140` — the location's kind chooses:
      kind HAS it  -> commit the first CANDIDATE;
      kind LACKS it -> commit the LOCATION object, re-rooting the list at it.

Two details worth stating because they look like mistakes until they are read
carefully:

1. `add di,4` @`0xB10A` has no meaning of its own. It pre-compensates for the
   `sub word [0x251b],4` @`0xB111` that both branches share, so the location
   branch commits its object whole while the candidate branch strips the `+4`.
   One `sub`, two meanings, because one branch pays it forward.
2. Because DI is `arche` on entry, the `arche` exclusion @`0x728B` fires on EVERY
   call from this path. The root can never appear in its own candidate list. That
   is not a special case in the filter; it is what the filter is for.

The empty-list case is ported as arithmetic, not smoothed: `[0x250B]` holds the
terminator, `0xB0F7` loads `0xFFFF`, `0xB111` subtracts 4, and the port yields
`0xFFFB`. The test asserts that value rather than a tidier `None`.

One guard catch along the way: the test called `selector_field_offset`, which does
not exist. Cheap because it was a compile error — but it is the same class as the
citation slips, an API recalled rather than checked.

## #196 — wiring the commit without inventing the choice

`world_click_select` was `check_unrouted_rules.py`'s flagship example: a decoded,
correct, tested rule that nothing ran. Four sessions of decode (#192-#195) built
the chain above it — filter, builder, selector, root — and this closes it.

`main.rs::commit_world_destination` runs the game's own path when the port enters
a world: `ship_click_initial_target` (rooted at `arche` from `0xB0EA`, picking by
the location's kind at `0xB0FB`) hands its record to `world_click_select`, which
writes `{0xC1, target, 0}` at `orxx+0xA` for the presentation ladder at `0x5B38`.
The guard no longer flags it.

What makes this a wiring rather than a fabrication: every value comes from the
VM's own records. The frontend supplies only the MOMENT. That was exactly the
distinction #192 got wrong in the other direction — it treated "the frontend has
a name, the VM wants a record" as an unbridgeable gap, when the bridge was inside
the VM the whole time and the frontend never needed to carry a record at all.

WHAT IS STILL APPROX, stated rather than absorbed: which world the port enters is
`compass_angle` arithmetic in `targeted_world_name`, where the game commits the
object its candidate list offered. The validation row is rewritten to cover the
CHOICE only, because the commit is no longer the open part. Wiring the choice
means driving world entry from `destination_candidate_records` rows instead of a
heading, which needs the DEB-loaded field matrix at runtime -- the next task on
this thread, and a real one rather than a blocker.

## #197 — the name was in the record all along

#196 wired the commit and left the CHOICE approximate: the port picked a world by
`compass_angle` arithmetic. Closing that looked like it needed a name->world
mapping. It needed nothing.

A candidate list entry is `RECORD+4` (`add ax,4` @`0x7292`). `object_inline_name`
reads from `object+4`. So the word the list stores IS the string pointer — record
and name are two views of one value, and `destination_rows` returns both by
subtracting and re-adding the same 4.

That coincidence is also the strongest internal check this chain has. A wrong `+4`
anywhere along it would not merely shift a record by four bytes; it would make
every destination name read as garbage, because the same offset feeds the string
walk. The names coming out legible is evidence for the whole `+4`/`-4` reading.

`main.rs` now enters the world its chosen row names and commits that row's record.
`compass_angle` survives only as the no-DEB fallback, where there are no records to
offer at all — and `engine.rs` had already recorded that the angle merely pans the
view, so it was never the game's chooser. That note sat in the tree while the
frontend used the angle to choose anyway; the port contradicted its own decode.

REMAINING APPROX, stated narrowly: the port CYCLES the destination row on the
world-entry key rather than hit-testing a pointer, because no pointer selection is
decoded for this screen yet. The rows, their order, their names and the committed
record are all the game's; only which one the cursor lands on is not.

## #198 — two mix loops, and the one thing that still blocks the rewiring

The audio row has stood as DECODED, NOT WIRED with the note that this environment
has no audio device. Treating that as the task rather than the excuse: what can be
verified without a device is the RULE, and reading `0xBB40..0xBB76` found a second
one the port did not have.

`0xBB53 test byte gs:[0xba2],1` selects between two mix loops. `0xBB6D` is the
known one. `0xBB5B` differs in exactly one respect: it reads `[si]` WITHOUT a
`lodsb` and does `inc si` only when the loop counter is even (`test cl,1 / jne`),
so the source is consumed once per two output samples — the voice plays at half
its sample rate, mixed by the identical average. Ported and tested on both counter
phases, because the parity sets which sample doubles first (`A,B,B,C,C` for an
even count, `A,A,B,B,C` for an odd one) and the game's phase depends on where the
buffer boundary falls.

Two things worth recording about the process:

1. My first dump started at `0xBB40` and produced `xor ax,0xf803`, which does not
   exist — re-anchoring from `0xBB10` shows `0xBB41: add di,ax`. The classic
   self-synchronisation phantom, caught because the aligned decode disagreed. The
   mix loops themselves appeared IDENTICALLY in both dumps, which is what made
   them trustworthy.
2. The test caught my own arithmetic, not the code's: I asserted 3 source samples
   fill 6 slots, and it is 5. An even counter advances on the very FIRST sample,
   so the head plays once and only the rest double. The passing assertion three
   lines above already said `A,B,B,C,C`; I had written the count without reading
   my own expected sequence.

WHY THE LIVE PATH IS STILL NOT SWITCHED, stated exactly rather than as "no device":
`0xBB21 les di,[bp] / add di,6` mixes into a buffer owned by a voice STRUCT, so
whether a lone sound is halved toward silence or written at full amplitude is
decided by whoever initialises that buffer each frame — which is undecoded.
`mix_unsigned_pcm_sources` assumes a silence pre-fill. Switching `audio.rs` to it
would change EVERY sound's amplitude on the strength of that assumption. The next
decode is the buffer's per-frame owner; the `lcall gs:[0xcf3]` @`0xBB28` is the
lead.

## #199 — following the lead undercut the row that named it

#198 said the next decode was the mix buffer's per-frame owner, lead
`lcall gs:[0xcf3]` @`0xBB28`. Following it produced something better than an
answer to that question: evidence that the question's framing was wrong.

Reading the prologue at `0xBAE8`:

  * `0xBAF7  mov ah,0x3F / int 0x21` — a DOS file READ. This is a STREAMER.
  * `0xBAFD  sub cx,6` — the chunk carries a 6-byte header.
  * `0xBB0B  add cx,cx` — the half-rate flag DOUBLES the output sample count.
  * `0xBB0D/0xBB10` — exactly TWO voice structs, `0xB89` and `0xB91`, and the one
    in state 3 (`cmp byte [bp+6],3`) receives the chunk.
  * `0xBB28..0xBB4E` — the write lands at an offset from the device's play
    position, clamped to the buffer, remainder wrapping at `0xBB76`.

That is a ring buffer fed by a file stream. It is not three simultaneous sources
being averaged together, which is what `docs/port-validation.md` has described for
this row — and what `mix_unsigned_pcm_sources` implements, complete with a silence
pre-fill that no instruction here asks for.

So the row's proposed FIX was wrong, not just unwired. Rewiring `audio.rs` to
`mix_unsigned_pcm_sources` would have replaced one invented model (three cpal
streams summed by the OS) with another (N sources averaged over silence), and the
tests would have passed, because they test that function against itself.

`mix_unsigned_pcm_average` remains correct: it is `0xBB6D` element-wise, and it is
what a streamed chunk does to a buffer that already holds something.
`mix_unsigned_pcm_sources` is now labelled as the generalisation it is.

Ported instead: `stream_mix_span`, the ring-buffer arithmetic, tested including
the wrap. Also `0xBB0B` independently confirms #198's half-rate loop from an
unrelated instruction — one flag, two consequences, agreeing.

Second test-expectation slip in this area (after #198's "6 slots"): I asserted a
clamp using a position that does not overflow, because `offset = |position -
length|` means a SMALL position gives a LARGE offset. Both slips were mine and
both were caught by the assertion, which is the argument for writing the expected
SEQUENCE out rather than a summary count.

## #200 — the buffer holds the sound, and the rate is in the file

#199 left one question: what fills a voice buffer before the stream mixes into it,
since that decides whether a lone sound is halved toward silence. `0xBBE4..0xBC2F`
answers it, and adds something better.

The answer: THE LOADED SOUND DATA. `les di,[0xbb7]` points voice A straight at the
file, length `0x4000`; voice B starts `0x4008` later. The two voices are the data's
two halves with the 8-byte header between them, and nothing writes silence
anywhere. So a lone sound is not attenuated — `0xBB6D` averages an incoming chunk
with sound that is already there. `mix_unsigned_pcm_sources`'s silence pre-fill was
invented, as #199 suspected; this is the instruction-level proof.

The better finding is four bytes away. `0xBBFE cmp byte es:[di+4],0xd3` /
`0xBC05 mov byte [0xba2],1`: THE HALF-RATE FLAG IS READ FROM THE SOUND FILE'S
HEADER. `0xD3` is the Sound Blaster time constant for 22222 Hz
(`1000000/(256-211)`), so the file itself declares the rate that needs decimating.

That matters beyond audio. The port could have "fixed" playback rate by picking a
constant that sounded right, and it would have been a content-bearing literal
standing in for a byte the game reads out of its own data — the defect class
CLAUDE.md names first. Ported as `snd_header_is_half_rate`, which reads the byte,
with a test that a value at `+3` or `+5` does NOT trigger it.

Three sessions on this row have now inverted its premise twice: from "wire the
decoded averaging" (#198) to "that averaging model is invented" (#199) to "the
buffer already holds sound and the rate is data" (#200). The remaining work for
`audio.rs` is a structural rewrite — two `0x4000` ring buffers fed by a chunk
streamer — not the one-line swap the row asked for when this began.

## #201 — building the core the blocker said could not be verified

The audio row's blocker was "this environment has no audio device, and an
unverifiable change to the output path is exactly the kind this campaign should
not make on faith." True about the OUTPUT PATH. It was never true about the rule,
and treating the two as one thing is what kept the row shut for several sessions.

`snd::SndStream` is the streaming mixer as `0xBAE8..0xBB93` builds it, with no
device in it anywhere: two voices sliced from the loaded sound data, the half-rate
flag read from that data's header, `active_voice` (state 3, first-preferred
because `0xBB0D` tests `0xB89` and only reaches `0xB91` through the `xchg` at
`0xBB19`), and `mix_chunk` composing `stream_mix_span` with the two mix loops and
the `0xBB76` wrap pass. Every piece decoded in #198-#200 now sits behind one call
that tests can drive.

Writing the tests found a real omission rather than confirming what I had:
`0xBB24 add di,6` mixes PAST a 6-byte header in each voice buffer, matched on the
source side by `mov si,0x7d06` @`0xBB00`. I had mixed at `buffer + offset`. The
test failed on an assertion about the buffer's CONTENT — the header byte I had
planted at `data[4]` was inside voice A's buffer, which it should be, and the
mixing should have skipped it. That also explains the `0x4008` stride that had
looked like an arbitrary gap: 6 bytes of header, `0x4000` of samples, 2 spare.

`half_rate_source_consumed` exists because the wrap pass has to know where the
SOURCE left off, and at half rate that is not the number of samples written. It is
derived from the same parity rule as the loop it accompanies, and tested against
what the loop actually walks rather than against my arithmetic — the third
expectation slip in this area (#198, #199) argued for checking it that way.

What is left is genuinely the output path: `audio.rs` still drives cpal with three
independent `MusicPlayer` streams. Feeding one `SndStream` instead is mechanical
against a tested core. A device is needed to confirm it SOUNDS right; it is no
longer needed to know it IS right.

## #202 — checking the core against the game's own bytes, and finding the real gate

The obvious next move was to migrate `audio.rs` onto `SndStream`. Two things
came out of preparing that, and the second changed the plan.

First, the core is now exercised on REAL GAME DATA rather than on fixtures.
`snd_stream_mixes_a_real_clip_from_the_shipped_bank` pulls a clip out of the
shipped `sn/tb.snd`, mixes it into a non-uniform bed, and asserts SAMPLE BY SAMPLE
that each output equals `snd_mix_average(clip_byte, original_byte)`. A test over
bytes I chose can only show the code does what I think; over the game's bytes it
shows the rule survives real content. It also checks the clip mean sits near the
`0x80` midpoint, which is the assumption the whole averaging model rests on.

It closed a loop too: `SND_HEADER_HALF_RATE_TIME_CONSTANT` (`0xD3`, from `0xBBFE`)
must agree with `snd_sample_rate`, decoded earlier from a different routine, and
it does — `1000000/(256-211)` = 22222 Hz. Two independent decodes, one number.

Second, and the reason `audio.rs` is still not migrated: the port already parses
the bank and plays clips, so the sources DO belong in this path — but playing
through `SndStream` needs the DOUBLE-BUFFER SWITCH rule. When does the driver flip
voice A to voice B, and what advances the position `gs:[0xcf3]` returns? That is
driver-side (`snd_driver_call` `0xBB9D`, the indirect `lcall gs:[0xcdb]` and
`gs:[0xa4a]`) and undecoded.

Writing a switch policy that sounds plausible would be a fabricated timing rule
wired into the live audio path — the same class of defect as the destination
`compass_angle` arithmetic that #197 removed, and worse for being unhearable here.
So the gate is named rather than stepped over: decode `0xBB9D` and the position
call, then migrate.

## #203 — hunting a driver pointer, found a transcribed manifest

The task was to map the sound driver's vector slots, which needed the `.drv`
loader. Neither `nosound.drv` nor `dnsdb.drv` is referenced by any immediate in
the executable, and that absence was the clue: the code does not POINT at those
strings, it INDEXES them.

They sit in a 95-slot table of 16-byte NUL-padded filenames at `FS:0x0c04` (file
`0xCDF4`) — the game's file manifest, the same 16-byte name-record shape as the
world-art table settled in #191.

`levels::LEVEL_DIRECTORY` is 53 of those slots, copied into Rust source. That is a
content-bearing literal, and it is also a PREFIX: 42 entries missing, including
further `.ext` worlds and the whole of script3/4/5 (slots 76..90). The frontend loads
`SCRIPT3..5` by name already, so the port has been reaching for resources its own
directory does not list.

`parse_level_directory` reads the table from the image; `level_entry_from_image`
resolves any slot. The transcription check was a real test rather than a
formality — it could have found a copying error in any of 53 rows, and found none,
which is worth knowing precisely because I would not have assumed it.

One boundary kept explicit: the table stores FILENAMES ONLY. `LevelKind` is the
port's classification by extension, not a field the game carries, and
`level_entry_from_image` says so where a reader would otherwise assume the kinds
were decoded too.

The driver-slot mapping that started this is still open. It is now the only thing
between the tested `SndStream` and the `audio.rs` migration, and the lead is
unchanged: find the code that reads a `.drv` by directory index and follow where
it stores the far pointers.

## #204 — the guard was checking a third of what it appeared to

Adding two citations to `levels.rs` produced no change in
`check_cited_instructions.py`'s count. That should have been impossible, so I
corrupted one deliberately — `shr` for `shl` — and the guard still reported clean.

Its pattern only ever matched the DUMP form, a doc line beginning with an address:

```text
///   0x3FD9  shl ax,4
```

Every citation written in PROSE — `` `shl ax,4` @`0x3FD9` ``, which is how most of
this session's are written — was invisible to it. The reassuring "0 wrong" covered
321 citations while the tree held 389.

Extending it found five mismatches, and all five were bugs in MY NEW RULE rather
than errors in the docs. Two prose shapes it misread:

  * ``mov si,0x137` @`0x836C`'s branch` — `0x836C` is the `cmp` that GUARDS the
    branch; the `mov` is at `0x8373`. The possessive is the tell: the address is
    the sentence's subject, not the quoted instruction's location.
  * ``mov al,es:[di]` / `or al,al` / `jne` @`0x9B30`` — the address anchors the
    FIRST item of a `/`-separated run. The regex had taken the last.

Both are now handled explicitly, and the corrupted-mnemonic test that exposed the
gap is the acceptance test for the fix: it is reported, then clean again once
restored.

68 previously unchecked citations are now verified, all correct. The finding is
not that the docs were wrong — it is that a guard reporting "0 wrong" had been
silently ignoring most of its input, which is the same failure as the truncated
test greps that once hid a failing oracle test for an unknown number of sessions.
A guard's coverage needs testing as much as its verdict does.

## #205 — the second table, and why the driver hunt keeps paying

Following `lcall 0x4B9:0` (file `0x5190`), the loader's post-read hook, produced
the RESOURCE DESCRIPTOR table — the companion to the filename table found in #203.

`shl bx,3` @`0x51A5` turns a resource ID into a descriptor offset with NO base
added, so the records are 8 bytes based at `FS:0x0000`. Two fields decode
immediately:

  * `+0` the SEGMENT the resource loaded at (`mov ax,[bx] / mov ds,ax` @`0x51B7`);
  * `+2` flags — `test word [bx+2],3` @`0x51AC` asks "already resident?", and on a
    hit the loader sets bit 1 (`or word [bx+2],2` @`0x51B3`) and returns without
    re-reading the file.

The two tables are consistent with each other, which is a check rather than a
restatement: 95 descriptors occupy `0x2F8` bytes and the name table starts at
`FS:0x0C04`, so they cannot overlap. The test asserts that relationship instead of
taking it on faith.

Every constant here is pinned to INSTRUCTION BYTES rather than to a doc: the test
reads `c1 e3 03` and derives the stride as `1 << exe[0x51A7]`, reads
`f7 47 02 03 00` and takes the offset and mask out of it. A doc rewrite cannot
drift from the code, and neither can a careless constant edit.

Worth noting what this hunt has produced. It began as "map the driver's vector
slots so `audio.rs` can be migrated" and has so far yielded: the shipped drivers
and their ABI (#202), the 95-slot file manifest and the discovery that the port's
directory was a transcribed 53-entry prefix (#203), a coverage hole in the
citation guard that hid a third of the tree (#204), and now the descriptor table.
None of those were the goal. The goal is still open — but "follow the thing you
cannot yet explain" has been worth more than the answer would have been.

Next link: an ID's descriptor gives the loaded SEGMENT, so the driver's far
pointers are that segment plus its vector offsets. What remains is finding where
the host writes `gs:0x0CDB`/`0x0CDF`/`0x0CF3` from it.

## #206 — "no write site" meant there is no write

The driver-slot mapping had been open for four entries. Every attempt to find the
code that fills `gs:0x0CDB`/`0x0CDF`/`0x0CF3` failed: an immediate search turns up
only `lcall` users, and no register ever loads those addresses.

That absence was the answer. The slots are STATIC DATA. Reading
`DS:0x0CD0..0x0D00` out of the image shows nine far pointers, four bytes apart,
offsets `0x100, 0x103, ... 0x118`, segments zero (filled at load time). Nothing
writes them because they are already correct in the file.

The `3` spacing identifies them beyond doubt: a `.drv` opens with `E9 rel16` near
jumps, three bytes each, and the driver loads COM-style at `0x100`. Slot k is
vector k.

THE STRIDE GUESS IN #202 WAS WRONG, and this is why it was not wired in. It put
the table at `0x0CDB`, making `0xCDF` vector 1 and `0xCF3` vector 6. The table
starts eight bytes earlier: `0xCDF` is vector 3 and `0xCF3` is VECTOR 8. Vector 6
had already turned out to be a buffer-queue routine — so the guess would have
attributed the position query to code that queues buffers, and every later
inference would have been built on it.

Vector 8 (`DRV:0x01CA`) settles what "position" means: it reads the 8237 DMA
controller's current-count register — `dl = cs:[0x49d]` (the channel),
`dx = channel*2 + 1`, two `in`s and an `xchg`. The value is the REMAINING count
and it counts DOWN. That retroactively explains `sub ax,[bp+4] / neg ax`
@`0xBB33`: `length - remaining` is how far playback has got. `stream_mix_span`
already took the absolute difference, transcribed without knowing why; now the
doc says why.

Pinned by a test that reads BOTH binaries — the slot offsets from `BLOODPRG.EXE`,
the `E9` vectors and vector 8's `pushf/cli` prologue from `dnsdb.drv`.

WHAT THIS LEAVES for `audio.rs`, and it is honestly a different kind of problem: a
cpal callback has no DMA controller to interrogate. The port must derive an
equivalent cursor from its own output clock. That is a design question about the
host, not a decoding question about the game, and it is the first time this row's
remaining work has been outside the binary.

## #207 — the count I wrote down was wrong, twice, in three places

#203 reported the resource directory as "54 of 95 slots transcribed, 41 missing".
Counting the literal's `index:` fields: it holds 53 entries (0..=52), so 42 are
missing. Both figures in #203 and in `docs/port-validation.md` were wrong.

Worse, the test I wrote to pin the finding asserted
`names[54] == "forest.ext"` with the comment "the first slot the literal omits".
The ASSERTION was true — slot 54 really is `forest.ext` — and the COMMENT was
false, because the first omitted slot is 53 (`erazor3.ext`). A true assertion with
a wrong explanation is the worst shape available: it passes forever and teaches
the next reader something incorrect.

Fixed by asserting the things that were actually claimed:

    assert_eq!(LEVEL_DIRECTORY.len(), 53, "the literal's size, checked not assumed");
    assert_eq!(names[53], "erazor3.ext", "the first slot the literal omits");
    assert_eq!(names.len() - LEVEL_DIRECTORY.len(), 42, "entries never transcribed");

Now the count cannot drift from the literal, because the literal's own length is
asserted rather than eyeballed. That is what #203 should have done in the first
place: I derived "54" by reading a printed list instead of counting the source of
truth, which is the same failure as the stale ledger figures corrected earlier in
this campaign — a number restated from a previous glance rather than recomputed.

## #208 — replacing the literal, and the flaky test it exposed

#203 pinned the transcribed resource directory to the image; this replaces it.
`init_level_directory(image)` installs the parsed 95-slot table in a `OnceLock`,
`directory()` backs both `entry()` and `primary_worlds()`, and `main.rs` calls it
at startup. `LEVEL_DIRECTORY` survives only as the no-image fallback.

The acceptance test is the strong form: the derived directory must equal the
transcribed one stem-for-stem, kind-for-kind, index-for-index across all 53 shared
slots. It does. The literal is now CHECKED BY the parse rather than trusted
beside it, and slots 53..94 are reachable for the first time.

MEASURED CONSEQUENCE: `primary_worlds()` returns 32 instead of 16. Sixteen
top-level `.ext` worlds were absent from the port's model of the game. The nav map
draws `take(7)` so nothing visible changes, but every enumeration of worlds had
been working from a little over half the set.

THE FLAKY TEST THIS EXPOSED, which matters more than the count.
`primary_worlds_are_the_named_planets` asserted a bare `names.len() == 16`. The
moment `init_level_directory` existed, that assertion's truth depended on WHETHER
AN EARLIER TEST IN THE SAME BINARY HAD INSTALLED THE REAL TABLE — a global
`OnceLock` makes test order significant. It failed on this run; it could as easily
have passed and failed later for no visible reason.

The fix is not a bigger constant. The test now derives its expectation from
`directory().len()` and additionally asserts the FILTER'S SHAPE (no `cyber`, no
numbered sub-levels) which holds under either directory. Re-run three times to
confirm stability. Updating `16` to `32` would have left the order-dependence in
place and made the next such failure look like a regression in the code.

Process note: the count would not have been caught by review. It surfaced because
a test asserted a specific number against changed global state — the same reason
#207's mis-stated count surfaced only when something asserted the literal's own
length. Numbers in this project need asserting, not writing down.

## #209 — measuring the last two captures, and verifying one in the right direction

Two captures had sat unread behind "needs composite reproduction". Measured with a
new `re/tools/ppm_stats.py` — written so the numbers are reproducible, because
#114's withdrawn claim came from reasoning about how a capture LOOKED:

`script2_first_frame.ppm`: 50 colours, mean run 4.58px, and rows 0..39 exactly one
colour. The tempting move is to add a 40-row constant to the port. That would be
deriving geometry from a capture, which the prime rule forbids — and it would also
be WRONG, because the panorama is full-screen and nothing draws a band there. The
40 rows are the frame's own content.

Which turns it into a claim about the ARCHIVE, testable in the allowed direction:
decode `TB.BIG` with the port's decoder and ask whether any frame opens with 40+
uniform rows. One does. The pixels come from the game's file through decoded code;
the capture only confirms them.

`mission_briefing_eye.ppm`: 173 colours, mean run 2.45px, no flat band anywhere.
Those are the statistics of a dithered full-screen VIDEO frame, not a UI surface —
so it is an HNM still, and reproducing it is an IDENTIFICATION task (which
DESCRIPT record names the clip) rather than a decoding one, since the port already
has the HNM decoder. Left open rather than guessed at.

The useful distinction this drew: a capture can support a test WITHOUT becoming
the source of the behaviour, provided the assertion is about something the decoded
path produces independently. "Some decoded frame opens with a flat band" is that
shape. "The band is 40 rows tall because the capture says so" is not.

## #210 — the difference between a match and the match

`CONSOLE_BAND_FRAME = 90` was the last constant carrying an oracle flavour. Its
doc already claimed proof by construction: frame 90's rows 140..200, through the
console-bank remap, equal the harvested `console_band.idx` in all 19200 bytes.

That is a strong claim and the test really did check it — but only for frame 90.
Nothing established that 90 was the ONLY frame satisfying it, and without that the
index is still something someone identified and then confirmed, rather than
something the archive determines.

The test now searches all 180 frames the same way and asserts the match set is
exactly `[90]`. It is. That closes the gap: given the band, the data leaves no
other choice, so the capture is the TARGET of a search and never the source of the
number. Settled DATA.

The general shape is worth naming, because several constants in this tree have the
same structure: verifying that a chosen value works is weaker than verifying that
no other value would. The first is consistent with a lucky guess; the second is
not. Where the candidate space is small and enumerable — 180 frames, 95 directory
slots, 52 opcodes — the stronger test costs a loop.

## #211 — a list that mixes settled with open hides the open

`check_cited_immediates.py` reported "27 need reading". Two of them were
`TALK_FIELD` and `LOCATION_FIELD`, which `field_matrix_entries_match_the_constants`
has been asserting against the image all along — reading both out of the matrix at
`DS:0x6D60`. They were never open. They were in the list because the checker knew
only one kind of grounding: "is this value an immediate at a cited address?"

That is the #204 failure in a different costume. A queue that mixes settled work
with open work does not just overstate the total; it hides the real items among
plausible-looking noise, and the noise never shrinks so nobody looks.

The checker now separates them. A constant counts as GROUNDED when a test both
mentions it and opens something the game shipped — deliberately narrow, because a
test comparing the port to itself grounds nothing (`check_selfref_asserts.py`'s
whole subject). Result: 100 directly encoded, 11 grounded by a data test, 18
needing reading.

Of those 18, seventeen are `OP_*` dispatch indices, which
`check_opcode_handlers.py` validates against the real table at `0x142D0` — grounded
by a DIFFERENT guard, and worth stating rather than leaving to look unresolved.
The eighteenth is `STATE_BASE`, a documented non-citation (#no literal exists in
the overlay; it is reached through a base register).

`RESOURCE_DESCRIPTOR_SEGMENT = 0` came off the list on its own merit, and the
argument is pleasing: `mov ax,[bx]` encodes as `8b 07`, a ModRM with mod=00, which
carries NO displacement byte. A field at any other offset would need one. The
ABSENCE of the byte is the zero, and the test asserts the two-byte encoding.

## #212 — the hit-test was already ported, just not used here

The world-destination row cursor was the last APPROX on that row: the port cycled
rows on a keypress "because no pointer selection is decoded for this screen".

That was wrong, and the evidence was already in the tree. The destination list
goes through the SAME unified widget as every other menu (`0x8428`, established
back in #192), and `engine::console_box_click` has implemented that widget's row
hit-test — `div bl,0x0B` @`0x8508` — for the concept and contact boxes all along.
Nothing needed decoding. The existing decode needed USING.

The world-entry key now opens the box (rows from `destination_rows`, trailing
CANCEL read from `DS:0x0174` rather than written as a literal) and a click selects
a row, committing that row's record through `world_click_select`. The click arm
sits before the chart handlers because an open list takes precedence in the game
too — `0xB2DC` keeps the FSM inside the list while it is up.

The row is now ASM end to end: rows, order, names, hit-test, committed record and
cancel label all come from the game.

The recurring shape, third time this campaign: a gap described as "not decoded"
turned out to be "decoded elsewhere and not wired" (#196's commit, #197's names,
now the hit-test). The cost each time was not decoding effort but a wrong
description of the blocker, which then justified an invention -- key-cycling here,
`compass_angle` in #197. It is worth checking what the port ALREADY knows before
concluding the binary has not been read.

## #213 — layered mixing, and two claims the tests refuted

`mix_unsigned_pcm_sources` mixes every source over a silence pre-fill, which
#199/#200 showed the game never does. The two code paths into a voice buffer are:

  * the loader OVERWRITES it — `int 21h`/`AH=3Fh` @`0x4049` reads the file
    straight in;
  * the streamer AVERAGES into it — `lodsb / add al,es:[di] / rcr al,1` @`0xBB6D`.

So the first sound in a buffer is unattenuated and later ones layer in.
`mix_unsigned_pcm_layered` implements that, and the silence-prefill version is now
explicitly the wrong shape for playback.

TWO CLAIMS I WROTE WERE FALSE, and the tests caught both:

1. "Order matters: averaging is not associative, so the FIRST source dominates."
   Wrong twice over. `(s + d) / 2` is SYMMETRIC, so with exactly two sources order
   changes nothing — the assertion failed immediately. And the weights run the
   other way: after three sources they are `c/2 + b/4 + a/4`, so the MOST RECENTLY
   mixed source dominates. The first source is merely unattenuated at the start,
   which is a different property from dominating.

2. The citation `` `add al,es:[di] / rcr al,1` @`0xBB6D` `` — `0xBB6D` is `lodsb`;
   the `add` is at `0xBB6E`. Caught by the guard extended in #204, which pairs an
   address with the FIRST mnemonic of a `/`-separated run. That extension has now
   caught an error in the same session it was written, on a citation I wrote
   while explaining the very instruction.

The test now asserts the corrected structure: two sources symmetric, three
order-dependent, and each output equal to the explicit fold
`avg(c, avg(b, a))` — which pins the weights rather than describing them.

## #214 — one stream, and the API that made it free

The audio row opened as "the port runs THREE independent `MusicPlayer` streams and
lets the backend sum them, so sources play at full amplitude and can clip, where
the game halves each and cannot." Six entries later the decode is complete, and
the wiring turned out to need no caller changes at all.

`audio.rs` now opens ONE cpal stream. Every `MusicPlayer` is a handle on a
process-wide `AudioMixer`, and the callback folds active sources with
`mix_unsigned_pcm_layered`: first source unattenuated (the loader's overwrite
@`0x4049`), later ones averaged in (`0xBB6D`). `MusicPlayer::start`/`start_once`/
`stop` keep their signatures, so every call site in `main.rs` gained the game's
mixing without an edit — the change is entirely behind the type.

That was worth aiming for. The alternative was rewriting the music/voice/chatter
call sites, which are interleaved with scene logic I cannot exercise here; leaving
the API fixed meant the risky part of the change was a module I can test.

`AudioMixer::render` is device-free by construction, which is what makes any of
this checkable in an environment with no sound card: silence when idle, a lone
source unattenuated, two sources averaged sample-by-sample, play-once reaped and
loops never. Four assertions that would each have failed under the old
three-stream arrangement.

WHAT A DEVICE IS STILL FOR, precisely: confirming it SOUNDS right. Not confirming
the mixing is right. That distinction is the whole reason this row moved — it sat
closed for several sessions behind "no audio device", which was true of the output
path and never true of the rule.

## #215 — a validation row is a status line, not a diary

The audio row had grown to roughly 5000 characters: six sessions of decode
history accreted into one table cell, each entry appended as the understanding
changed. Every sentence was true when written, and several were superseded by
later ones in the same cell — including the original "not done here: this
environment has no audio device", still sitting alongside the entry that closed it.

That is worse than untidy. `docs/port-validation.md` is the WORK QUEUE; a row is
read to answer "what is the state of this, and what is left". A row containing its
own refuted premises answers neither, and the reader has to reconstruct the
chronology to find the current position.

Compacted to the current state plus pointers: the rule, where it is wired, what
the tests cover, the supporting decodes by address, and the one remaining item.
The derivation lives in `audit-fixes.md` #198-#201, #206, #213-#214, which is the
file whose whole purpose is chronology.

The general policy, worth applying to other long rows: port-validation says WHAT
IS TRUE NOW; audit-fixes says HOW IT WAS FOUND, including what was believed and
refuted along the way. Appending to a validation row instead of rewriting it
conflates the two and slowly makes the queue unreadable.

## #216 — 180 rows were already verified; the ledger had not noticed

`tools/audit_suggest.py` applies #211's finding at scale: search the UNVERIFIED
rows for ones whose evidence ALREADY exists in the tree. 180 turned out to be
exercised by tests that read the game's own files — `snd_entry_call_sites` by
`snd_entry_call_sites_recover_constant_ax_indices` against the real
`BLOODPRG.EXE` fixture, and so on. Settled TESTED.

BE CLEAR ABOUT WHAT THAT IS AND IS NOT. Nothing was verified today by settling
them. The tests already existed and already passed; the ledger simply had not
recorded that they cover those items. The settled figure moving 650 -> 827
(29.2% -> 37.1%) is BOOKKEEPING CATCHING UP WITH REALITY, not 177 items of new
decoding, and reporting it as progress without that caveat would overstate the
work by an order of magnitude.

TESTED is also a WEAKER level than ASM: it says "something checks this against
real game data", not "this transcribes a cited routine". Several of these rows
should eventually become ASM with a citation. Marking them TESTED records what is
true now.

The tool's first run was, as always, a test of the tool: 259 suggestions including
`parse`, `summary` and `header_size` — generic identifiers matching some
data-reading test anywhere in the tree. Two narrowings fixed it: match per-FILE
(a Rust unit test sits beside its item, so cross-module collisions vanish) and
require a REFERENCE (`name(`, `::name`, `.name(`, `name {`) rather than the word
appearing in a comment. 259 -> 181, and three spot-checks confirmed the survivors
are real.

WHAT THIS LEAVES is the honest queue: 791 rows with neither a citation nor a data
test. That is the number to work, and it is now findable because the noise is out
of it — the same reason #211 mattered.

## #217 — fixing the inventory, and the diff that caught me deleting evidence

Working the real queue turned up two ledger rows that are not items at all: one
literally named `fn`, and `W`/`H` repeated at three line numbers in `ship3d.rs`.

  * `const fn e(...)` was parsed as a CONST named `fn`. The pattern took the first
    keyword and then the next word, so every `const fn` in the tree became an
    unsettleable row for an item that does not exist.
  * `W`/`H` are FUNCTION-LOCAL aliases inside `render_star_map_navview_projected`
    (`const W: isize = SHIP_3D_PROJECTION_SCREEN_WIDTH as isize`), making no
    independent claim about the game.

The fix for the second nearly cost more than it saved. "Function-local items are
not port surface" is WRONG as stated: `engine.rs` has `const TEXT_SELECTED = 0xEF`
inside a function carrying `mov al,0xEF` @`0x858B`, a SETTLED ASM row. The first
cut deleted it, along with `TEXT_SELECTED_MOUSE`, `CREDIT_RECORD` and others —
silently, because a smaller ledger looks like progress.

What caught it was diffing the item SET before and after, not the counts. The
counts moved from 2228 to 2197 and the percentage rose; nothing about those two
numbers said "four settled rows and a decoded constant just vanished". Any change
to the inventory needs the set diff, and now the entry says so.

Refined twice more from the diff's evidence: keep a local whose DOC cites an
address (`TEXT_SELECTED`), and keep one whose own DECLARATION carries a hex
literal (`const PAL_DS: u32 = 0x5251`, a recomp-machine address with no doc at
all). Erring toward keeping is deliberate — an extra row costs a slightly larger
denominator, while a dropped decoded value leaves the ledger silently smaller.

Final: 15 rows removed, 4 of them settled, and all fifteen confirmed by name to be
local helpers (`Music`, `VmInspection`, `Ev`, `ROWS`, `ClipInfo`, the `fn`
phantoms, the `W`/`H` aliases). 2212 items, 817 settled — the percentage went
DOWN, 37.1% to 36.9%, which is the honest direction when phantom rows leave a
denominator that also held their settled siblings.

## #218 — the fourth time: the answer was in another module

#192 flagged `DS:0x252F` as unresolved. Its exact words: "`[0x252F]` has four
setters (`0x9F40`, `0xB331`, `0xB4EA`, `0xB6A5`) and is not the back-row flag
alone, so nothing broader is claimed for it here." Correct restraint at the time.

Working the `ship3d.rs` struct queue found the answer sitting in the tree:
`update_ship_3d_transition_state` (`0xB692`) already decodes `mov byte
[0x252f],1` @`0xB6A5` as the TRANSITION OPENING flag, with `0x2530` closing,
`0x2531` the step and `0x2533` the armed latch — one of the four setters #192
listed, documented in a module I was not reading.

So the back row's `[0x252F]=1` / `[0x2531]=6` @`0xB331` ARMS AN OPENING
TRANSITION, and `0xB288` skips the world-view teardown because a transition is
running. That is sharper than #192's "the back row suppresses the teardown", and
it explains the otherwise arbitrary step 6 sitting between open's 4 and close's 8.

FOURTH INSTANCE of the same pattern this campaign — after #196 (the commit),
#197 (the names) and #212 (the hit-test). Each time something recorded as
undecoded was decoded elsewhere in the port. The cost is never wasted decoding; it
is a doc that says "unknown" while the knowledge exists, which then justifies
either an invention or a needless investigation.

`re/tools/whatis.py` exists precisely to prevent this — it searches labels, ledger
AND source for an address. #192 did not run it on `0x252F`. Running it costs one
command; four entries of this file are the price of not running it.

`Ship3dTargetSelectorState` now documents each field's DS byte with the
instruction that touches it, and marks `target_animation_tick` as claiming no
address, because none was decoded.

## #219 — 27 docs, 0 settled, and that is the correct outcome

27 structs in `ship3d.rs` had no doc at all while sitting beside functions settled
ASM: `Ship3dTransitionState` next to `update_ship_3d_transition_state`,
`Ship3dProjectionMatrix` next to `build_ship_3d_projection_matrix`, and so on.
Each is the parameter or result SHAPE of a cited routine and carries no rule of
its own. An undocumented struct beside a cited function reads as unexamined when
it is simply the function's shape, so each now says which routine it belongs to.

I then tried to settle all 27 as ASM. `audit_settle` REFUSED every one: "ASM needs
a cited address". The docs deliberately point at the function instead of restating
its addresses — duplicating a citation onto the type would also trip
`check_duplicate_rules.py`, which exists because two copies of a rule drift apart.

That refusal is right and I am leaving it. Documentation is not verification.
27 rows are now easier to understand and none of them is any better VERIFIED than
this morning, so the ledger should say exactly that. Settling them would have
bought 27 rows of apparent progress for zero evidence — the failure #216 was
careful to avoid, arriving from a different direction.

What would actually settle them: a test that exercises each shape through its
routine against real game data, which is real work and is not done here.

Real queue after this: 779 rows with neither a citation nor a data test, down from
791 only because the inventory fix removed phantoms.

## #220 — the test #219 said was missing

#219 documented 27 shape structs and settled none, noting that what would settle
them is "a test that exercises each shape through its routine against real game
data". This is that test for the target-list cluster.

`real_game_labels_lay_out_and_hit_test_back_to_their_own_rows` takes the OPTION
menu's labels OUT OF `BLOODPRG.EXE`'s string table, measures them with the port's
font, lays them out with `layout_ship_3d_target_list` (`0x84A1`), and hit-tests
each row's own centre with `hit_test_ship_3d_target_list` (`0x84E6`). Every row
must come back as itself; a point left of the box and a point above the first row
must hit nothing, which are the `>= layout.x` and `row_offset >= 0` gates.

It found nothing wrong, and that is a real result rather than a formality: layout
and hit-test are separate transcriptions of separate routines, and a disagreement
between them — an off-by-one inset, a wrong row pitch — would have shown up as a
row hit-testing to its neighbour. They agree on the game's own label widths.

A note on how the last two rows got settled. The suggester saw only
`Ship3dTargetHitState` at first, because the other two types were never NAMED in
the test — `layout` was an inferred binding and the result was consumed as
`hit.is_some()`. The fix was to make the test genuinely use them (`let layout:
Ship3dTargetListLayout`, and destructure the result to assert `hit.inside`,
`hit.hover_row`, `hit.activated`) rather than to loosen the matcher. Asserting on
the result instead of its Option-ness is a better test anyway, which is the tell
that the strict matcher was pointing at a real weakness.

Three of the 27 are now TESTED on evidence. The remaining 24 need the same
treatment, one cluster at a time.

## #221 — a property that a long fixed-point chain cannot fake

`build_ship_3d_projection_matrix` (`0x2F95`) folds three angle pairs into nine
terms through a chain of `imul`/`sar 15`, including the deliberate
`neg`-before-shift at `0x2FB1` that differs from negate-after by one unit. Nothing
in that chain announces an error: a swapped term, a lost shift or a transposed
pair still yields plausible-looking numbers, and a test comparing the port to
itself would confirm whatever it does.

But the result has to be a ROTATION, and a rotation's rows have unit length. In
this fixed point that is `sum(t^2) ~= (1<<15)^2` per row, using the game's own
`SHIP_3D_ANGLE_TABLE` (already verified against the binary by
`angle_table_matches_binary`). The test sweeps the table rather than sampling one
angle, because a term that only misbehaves when a sine is negative would survive a
single favourable sample.

It passes at every sampled angle, within 1.5% — the tolerance is for `sar`
truncation, which loses up to a unit per multiply.

THEN I TESTED THE TEST, which matters more here than usual, because a
property-based assertion can be vacuous in ways an equality assertion cannot.
Perturbing one term by 900 produced `row 1 has length^2 0.9458, not 1` and a
failure; restoring it passed. The invariant has teeth.

This is the strongest kind of check available for transcribed arithmetic: it
verifies a structural property the ORIGINAL must satisfy, without needing a
reference output to compare against. Worth reaching for wherever the port
transcribes maths rather than table lookups.

`Ship3dProjectionMatrix` and `Ship3dMatrixAngles` settled TESTED on that evidence.

## #222 — Cauchy-Schwarz as a regression test

#221 proved the projection matrix is a rotation. That makes a second property
free: a rotation's row has unit length, so a dot product with it CANNOT exceed the
vector's own length. `project_ship_3d_point` (`0x2F65`) computes depth as exactly
that dot, so `depth <= distance` must hold — for the original as much as the port.

The test sweeps 180 view angles against a fixed point and asserts the bound, then
asserts the bound is NOT VACUOUS: some angle must produce a depth above half the
distance, or a broken implementation that always returned 1 would pass. A property
test without that second half proves very little, and it is the half that is easy
to forget.

Perturbing the shift (`>> (SHIFT - 2)`) gives `depth 2000.0 exceeds the distance
616.4` and fails; restoring it passes. A lost shift is precisely the error this
kind of transcription makes.

Also added: a point translated onto the origin has depth 0 and is culled, which
pins the `depth <= 0` branch rather than leaving it to the sweep's luck.

A PROVENANCE NOTE, because these tests differ from the ones `audit_suggest`
recognises. They open no shipped file; they use `SHIP_3D_ANGLE_TABLE`, an embedded
const. That table is the GAME'S data — `angle_table_matches_binary` verifies it
byte-for-byte against `BLOODPRG.EXE` — so the provenance is transitive rather than
direct. Settled TESTED on that basis, stated here because the suggester's
file-opening heuristic would not have found it, and a future reader should know
the chain rather than assume the tool covered it.

`Ship3dProjectionPoint`, `Ship3dProjectedPoint`, `Ship3dProjectionOrigin` settled.

## #223 — the chain to the pixel, and a threshold that was not a measurement

The projection cluster now runs end to end in one test: the game's angle table
builds a matrix (#221), the matrix projects points (#222), and
`plot_ship_3d_projected_point` (`0x9B04`) clips and writes them. Two decoded rules
are asserted where they finally bite:

  * a rejected point writes NOTHING. The test snapshots the buffer and compares
    after every `None`, which catches the specific bug a missing sign check causes
    — a negative coordinate wrapping into a valid offset and drawing somewhere
    else entirely.
  * FIRST WRITE WINS (`mov al,es:[di] / or al,al / jne` @`0x9B30`). Replaying an
    accepted point must be rejected and must leave the original shade. Deleting
    that gate makes the test fail with "a second point at the same offset was
    accepted"; restoring it passes.

The first run failed on `only 96 points plotted; the sweep proves little` — my own
coverage floor, not a defect. Every real assertion had passed. I widened the sweep
(step 3 -> 2) rather than lowering the threshold, because the floor exists to stop
the other assertions being vacuous, and moving it defeats its only purpose. The
comment now says which it is, since a bare `assert!(plotted > 100)` reads like a
measured property of the game rather than a guard on the test itself.

`Ship3dProjectionViewport` and `Ship3dProjectedPixel` settled TESTED. The
projection cluster — matrix, angles, point, projected point, origin, viewport,
pixel — is now seven shapes verified through three decoded stages, from one table
proven byte-exact against the binary.

## #224 — the FSM's version of "never write outside the viewport"

The nav-choice cluster is a dispatch FSM, so #221-#223's geometric invariants do
not apply. The equivalent question is: can the machine ever select a handler that
does not exist? The committed choice becomes an index into the FIVE-entry table at
`CS:0x0F29`; past the end it reads a garbage far pointer and calls it. Nothing in a
return value would show that.

The test sweeps axis, mouse position and gate value, and asserts three decoded
rules: a blocking gate reports `gated` and never selects; a `gate_value` outside
`40..=60` never selects; and every hovered, dispatched and highlighted value lands
inside its real range — choices in `1..=5`, palette indices inside the choice bank
at `0x7B`.

TWO CORRECTIONS TO MY OWN TEST before it said anything:

  * it scored ZERO selections at first. The box is anchored at
    `SHIP_3D_NAV_CHOICE_AXIS_BIAS` (45) and I swept `axis` from 0 with a step of
    23, so the sweep never entered the hit box. A sweep that misses its target
    passes every assertion vacuously — the coverage floor caught it, which is the
    second time in three entries that floor has earned its place.
  * `selected_choice` is only written when `input.activate` is set (`0x86F1`'s
    commit path), and I had swept with `activate: false`. The hover path and the
    commit path are different branches and I was testing neither.

Removing the hit-test's `choice >= COUNT` guard produces `hovered choice 6 is
outside 1..=5`; restoring it passes. The guard is real and now pinned.

`Ship3dNavChoiceState`, `Gates`, `Input` and `Result` settled TESTED.

## #225 — a perturbation that perturbed nothing

The dirty-rect collector's contract is now pinned: a command is emitted only where
the rects genuinely intersect; `dispatch_index` and `destination_remap_mode` stay
inside their 3- and 2-bit fields (a wrong mask is the sprite version of #224's
handler-table overrun); an inactive slot emits nothing; and EVERY slot walked has
its dirty flag cleared, active or not — the clear sits outside the active branch,
which is exactly the kind of line a later tidy-up moves inward.

The process note is the useful part. My first attempt to prove the dirty-flag
assertion had teeth only re-indented the statement, leaving it in the same scope.
The test passed, and I nearly recorded that as "the assertion is real". Rust does
not care about indentation; I had perturbed the formatting, not the behaviour.

The real perturbation moves the clear INSIDE `if flags & ACTIVE != 0`, so inactive
slots keep their dirty flag. That fails with "the dirty flag survived the pass
(seed 2)". Restoring it passes.

Worth stating as a rule, since this is the sixth or seventh time I have tested a
test in this campaign: a perturbation must change what the program DOES. Moving
whitespace, renaming a local, or reordering independent statements proves only
that the test is insensitive to things it should be insensitive to.

NOT SETTLED, deliberately, and this is the third such refusal (#219, #223's
threshold, now this). The test drives synthetic slots rather than game data, so it
verifies the port against the decoded rules — a regression test, which CLAUDE.md
asks for — but not against the original. The ledger rows stay UNVERIFIED.

## #226 — two routines agreeing, and a count I got wrong again

`ship_3d_nav_entity_for_slot` builds an entity record address as
`0x6212 + (id << 5)` from the ship-3D projector's decode. Separately, `engine.rs`
decodes the nav hover panel reading ENTITY `0x1F` directly at `si = 0x65F2`
(`0x830A`), with no reference to a table base at all.

Two independent decodes of one structure must line up, and the identity is
checkable: `0x6212 + 31*32 == 0x65F2`. If the base, the stride or the count were
wrong, the last entity would not land where the other routine reads it. Four
constants, two routines, one arithmetic identity — a cross-check rather than the
port agreeing with itself. The test also asserts the nav slots fill the table's
tail exactly, each address distinct and none past the end.

THE COUNT I REPORTED LAST TURN WAS WRONG. I said "834 settled (37.7%)" after
`audit_settle` reported 4 rows; the real figure was 832, because I added the tool's
output to a remembered number instead of recomputing. That is precisely the failure
#207 recorded — a number restated from a previous glance — and the interval between
writing that entry and repeating the mistake was about ten commits.

It did not reach any doc: the wrong figure existed only in my messages, and the
ledger itself was always right. The lesson is not "be careful"; it is that the
count must come from the ledger every time it is stated, and a settle tool's
per-call output is not a running total. When the numbers next looked odd
(834 -> 832 after ADDING a test), the discrepancy was mine, not the ledger's.

## #227 — a literal array that turned out to be readable

`SHIP_3D_TEMP_SND_CALLBACK_OFFSETS = [0x0087, 0x0090, 0x009c]` sat in the port as
a plain array beside `SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET = 0x0acc`. The second
constant is the FIRST one's address: `DS:0x0ACC` is where the game keeps that
table, so the array never needed to be trusted — it can be read.

It matches, word for word. Two further things fall out of reading rather than
transcribing:

  * the word PAST the table is zero, which confirms the count is 3 because the
    DATA says so, not because three entries were transcribed;
  * the offsets ascend, which a set comparison would not catch — a transposed pair
    would map phase 1 to phase 2's callback and still "match".

Settled DATA, all three constants.

The general point, and the reason to look at neighbouring constants: an array of
hex literals sitting next to an address constant is usually not two facts. It is
one fact and its location, and the location makes the array checkable. This tree
has more such pairs — `check_offset_pairs.py` already validates 21 DS/file pairs,
and the same idea applies to any table whose base the port already knows.

## #228 — a new checker's first run found a real bug in its first minute

`tools/check_literal_tables.py` generalises #227: pack every `const NAME: [T; N]`
in the port little-endian and look for those bytes in the shipped images. A table
that IS in the image can be read instead of trusted; one that is ABSENT wants a
reason.

Ten tables qualified. Five are in `BLOODPRG.EXE` at unique offsets — the two font
tables at `0x14C22`/`0x14CD2` (matching `GAME_FONT_WIDTH`'s existing citation), the
two SQUARE_CAPS tables, and `LOCATION_PANEL_BOX`. Five are absent.

One of those five was a genuine defect. `EXT_WORLD_MAGIC` was eight bytes,
`02 00 00 01 00 00 00 81`, with a doc claiming it was "verified identical across
the planet worlds AND the cyberspace levels". Measuring all 50 shipped `.EXT`
files:

```text
   0200000100000081  x37      <- the transcribed value
   0200000100000080  x10
   0200000200000000  x1
   0200000100000084  x1
   020000010000008b  x1
```

`is_ext_world` therefore REJECTED 13 of 50 shipped worlds. The test that
"verified" the claim named eight worlds by hand — and all eight sit inside the 37.
The sample chose the evidence that confirmed it.

Fixed on both axes. The constant is now the three bytes every file actually
shares, documented as a WEAK signature and as a PORT-SIDE HEURISTIC: the game
never sniffs these bytes, it loads worlds by resource ID through
`resource_load_by_id` (`0x3FC7`), so there is no decoded rule to copy and the
varying bytes are left undescribed because nothing has decoded what they mean.
The test now sweeps the directory and asserts it saw at least 40 files, so a
future narrowing of the constant cannot hide behind a lucky sample.

The lesson is the same one as #210's uniqueness check, arriving from the other
side: where the candidate set is enumerable — 180 frames, 95 directory slots, 50
world files — SWEEP IT. Sampling proves the sample.

## #229 — four absences, four different reasons

#228's checker left five ABSENT tables. Each turned out to be absent for its own
reason, and only one of them was a defect:

  * `EXT_WORLD_MAGIC` — a real bug, fixed in #228 (rejected 13 of 50 worlds).
  * `NAV_CAMERA_ORIGIN` — WIDENED. The game stores three WORDS at `DS:0x2F65`;
    the port holds `i32`, so the packed bytes never match. Reading the words back
    confirms `[10000, 12000, 0]`, and the test also pins the word AFTER them
    (16) so a fourth component cannot silently join the vector. Settled DATA.
  * `STATION_REST_FRAMES` — DERIVED. The file stores ANGLES
    (`0x000, 0x05A, 0x0B4, 0x10E`) and the port halves them, because a panorama
    frame is 2 degrees. The stored bytes already have their own test; this array
    is the conversion, not a copy. Doc now says so.
  * `GAME_SCREEN_PALETTE_DAC` — known, already labelled APPROX for 128..191.
  * `SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR` — GENUINELY UNEXPLAINED, and now
    documented as such rather than given a plausible story. `0x0140` is 320 and
    `0x00C8` is 200, so it reads as a full-screen viewport, and that is all the
    values support. Where the game builds it is undecoded; it is presumably
    assembled by consecutive stores, which is why no table exists to find.

The useful shape here: "absent from the image" is a QUESTION, not a verdict. Three
of the five had good answers that make the port's version verifiable in a
different way, one was a bug, and one is honestly open. A checker that reported
absence as failure would have produced four false alarms and buried the real one —
which is why it prints ABSENT with a byte count and leaves the judgement to a
reader.

## #230 — checking a table you cannot search for

`check_literal_tables.py` skipped anything under 8 bytes, because a short byte
sequence matches somewhere in a 86KB image by chance. That floor also made the
tool blind to the very case that inspired it: `SHIP_3D_TEMP_SND_CALLBACK_OFFSETS`
is SIX bytes, and #227 verified it by hand.

The fix is to stop searching and start looking. If a scalar constant within a few
lines holds a plausible DS offset, check the bytes AT that address. A six-byte
match at one specific address is strong where a six-byte match anywhere is
meaningless — the address is the extra evidence that replaces length.

Running it reproduces #227's manual result: `SHIP_3D_TEMP_SND_CALLBACK_OFFSETS`
confirmed at `0x0DEEC`, the address `SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET`
names. That the new rule independently arrives at a known-good answer is the
validation — a new checker's first run is a test of the checker, and this one had
a right answer waiting for it.

Eleven tables remain unverifiable by either route: too short to search, with no
address constant beside them. That is an honest residue rather than a queue —
nothing about them is wrong, there is simply no evidence available from the images
alone, and saying so beats leaving them in a "skipped" count that reads like
neglect.

The tool's docstring now describes all three outcomes, including that ABSENT is a
question (four good reasons, one real defect) rather than a failure.

## #231 — names that asserted what nothing here proved

`bloodprg.rs` carries a block of thirty-odd bare constants:
`SHIP_3D_OPENING_FLAG_DS_OFFSET = 0x252f`, `SHIP_3D_CLOSING_FLAG_DS_OFFSET =
0x2530`, and so on. No docs, no citations — just a name asserting a meaning.

Those names are CLAIMS. "OPENING_FLAG" says the game treats `DS:0x252F` as a
transition opening flag, which is a decoded fact about the original — and the
decode establishing it lives in `ship3d.rs` (`0xB6A5`), with nothing in
`bloodprg.rs` connecting the two. A reader of this file alone had a name and a
number and no way to check either.

Ten now carry the instruction that touches them, all from decodes made earlier in
this campaign: the selector's list and phase bytes (#192), the transition's
opening/closing/step/armed set (#218), the exit-pending byte at `0xB288` whose
polarity #192 got backwards. The citation guard verifies all ten — its count went
401 -> 411 with 0 wrong.

Settled ASM. The other twenty-odd in the block are code offsets within segments
rather than DS addresses, and need the segment resolved before an instruction can
be named; that is the next slice rather than something to guess at.

ONE PROCESS FAILURE, found while chasing this. `snd_entry_call_sites` showed
UNVERIFIED despite #216 settling it. Tracing the ledger through fourteen commits:
in #216 I ran `git checkout docs/function-audit.tsv` to undo a timed-out loop —
AFTER generating the suggestion list. The reset undid that row's settle, and the
list only held then-UNVERIFIED rows, so the batch never re-settled it. One row
lost to my own sequencing, not a tool bug. Re-settled, and `audit_suggest` now
reports 2 exercised-only rows rather than 1, which is the check that nothing else
was lost the same way.

## #232 — offsets that name what lives at them

The rest of `bloodprg.rs`'s ship-3D block is CODE offsets rather than DS
addresses: `SHIP_3D_TARGET_RECORD_SELECT_OFFSET = 0x031b`,
`SHIP_3D_TRANSITION_STATE_OFFSET = 0x06f2`, and so on. #231 left them because an
offset needs its segment before an instruction can be named.

The segment is two lines above them. `SHIP_PRESENTATION_SEGMENT = 0x0a9a` gives
file base `0x600 + 0x0A9A*16 = 0xAFA0`, and every offset resolves onto a routine
this campaign has already read:

```text
   0x031b -> 0xB2BB   the target selector      (#192)
   0x03ae -> 0xB34E   the navigation update
   0x06f2 -> 0xB692   the transition updater   (#218)
```

So the names are CHECKABLE CLAIMS, and the test checks them three ways. The two
independently disassembled routines must show their exact opening bytes —
`56 06 57` (`push si/es/di`) and `F6 06 33 25` (`test byte [0x2533],1`). Every
constant NAMED as a call site must land on `E8` or `9A`. Every routine offset must
land on a prologue rather than mid-instruction.

That last one is what makes this more than restating the constants: x86 is
variable-length, so a wrong offset lands inside an instruction and shows up as a
nonsense opcode. Shifting `0x06f2` to `0x06f3` fails with "not `test byte
[0x2533],1`"; restoring it passes.

Twelve rows settled ASM, including the segment itself. The block that was thirty
bare numbers an hour ago is now twenty-two cited constants and a handful of
genuinely undocumented ones.

## #233 — five more addresses, and the eighth guard catch

Five further `bloodprg.rs` DS constants now carry the instruction that touches
them, each sourced from a decode already in the tree rather than from a fresh
disassembly: the selector's zoom rect (`0xB305`) and query-mode byte (`0xB2E3`)
from #192, the list widget's anchor (`0x84AD`) and hover row (`0x850C`) from the
target-list decode, and the mouse-on-row bit (`0x858D`) from `engine.rs`.

One was wrong and the guard said so: I cited `sub dx,[0xac6]` at `0x84AD`, but
`0x84AD` is `shr dx,1` — the `sub` is two instructions further on. The doc I took
it from says exactly that (`shr dx,1 / sub dx,[0xac6] / neg dx`); I quoted the
part that mentioned the address I wanted rather than the part that starts at the
address. Now cited as the sequence, with a note that the `sub` is not at the head.

That is the EIGHTH time this guard has caught one of my own citations, and the
second time (#213) it caught the specific error of quoting a sequence's middle. It
is worth noting that the failure mode is stable: when I want to document address
X's ROLE, I reach for the instruction that expresses the role, which is often not
the instruction AT X. The guard's rule — an address pairs with the FIRST mnemonic
of a run — is the correct discipline, and #204's extension is what makes it
enforceable in prose docs at all.

## #234 — three cited, five left alone

The remaining nav-choice gate constants (`0x2565`, `0x2736/7`, `0x259b`, `0x2795`,
`0x2a19`) all have `find_imm` hits, so all five COULD have been cited in a
minute. Three were; five were not, and the reason is worth recording.

`find_imm` confirms an instruction boundary by agreement across earlier decode
anchors, which suppresses most phantoms but not all — and several hits for these
addresses sit at very low file offsets (`0x010af`, `0x010b3`, `0x010b7`) in the
header/relocation region, where "an instruction" is a decode of data. The citation
guard would NOT catch that: it disassembles at the address and compares the
mnemonic, so a phantom whose mnemonic I copied from the same phantom agrees with
itself perfectly.

So the three cited are the three I could corroborate a second way — each sits
inside an already-labelled routine, confirmed by an ALIGNED disassembly from an
earlier anchor:

  * `0x2565` — `test byte [0x2565],1` @`0x86FB`, four instructions into
    `nav_choice_dispatch`;
  * `0x2793` — `test byte [0x2793],8` @`0x86F1`, that routine's FIRST instruction
    and its gate;
  * `0x27d8` — `mov byte [0x27d8],1` @`0x9EBB`, inside `travel_activate_a`.

The other five stay uncited. A citation whose only support is the tool that found
it is not evidence, it is a restatement — and this project's guard cannot tell the
difference, which is precisely why the judgement has to happen before the citation
is written rather than after.

## #235 — measuring whether a bytecode walker is aligned

`parse_script_disassembly` walks `SCRIPT*.COD`: variable-length tokens, where one
wrong length desynchronises everything after it AND THE OUTPUT STILL LOOKS LIKE A
DISASSEMBLY. No row says "I am misaligned"; they all say `opcode`, `mnemonic`,
`operands` as usual.

What does say it is the VM's own dispatch range. The table at file `0x142D0` has
52 entries covering `0xA0..=0xD3`, and a desynced walker starts reading OPERAND
bytes as opcodes — record offsets, string indices, coordinates, mostly outside
that range. So the share of in-range opcodes measures alignment directly, and
across the game's five shipped scripts it is above 99% of 4611 tokens.

That is a property of the ORIGINAL data, not of the port: the real bytecode
contains only dispatchable opcodes at token boundaries, so a correct walker
inherits the property and a broken one cannot fake it.

TWO PROCESS NOTES:

  * I concluded `script.rs` had no test module from
    `grep -n 'mod tests' ... | head -3`. It has one at line 4114 — the `head -3`
    truncated the output. That is the same mistake as the truncated test greps
    that once hid a failing oracle test, made while writing about not making it.
  * The test passed immediately, which proves nothing on its own, so I forced the
    token count into a failure message: 4611 tokens across the five scripts.
    Without that check, a path that found no `.COD` files would have returned
    early and passed just as quietly.

## #236 — two invariants the TEXT tokens carry themselves

`parse_script_text_flags` decodes the `0xA6` TEXT token (`0x660C`): line index,
voice selector, flag bytes, optional LOOP TARGET. Two properties must hold of any
correct parse of the real scripts, and neither belongs to the port:

  * offsets STRICTLY INCREASE within a script. The walker only moves forward, so a
    repeat or a step back means it lost its place — a different symptom of the
    desync #235 measures by opcode range, and one that catches a walker which
    stays in range while stalling.
  * a LOOP TARGET points INTO the same stream. It is an offset in this script's
    bytecode, so it cannot exceed the file; a misread operand pair yields targets
    in the tens of thousands.

Both hold across 3655 text tokens and 170 loop targets in the five shipped
scripts.

The measurements are in the test rather than in my head. Both coverage floors were
raised to a deliberately impossible value to read the real numbers back
(3655 tokens, 170 targets), then set just under them. Without that, "no loop
targets seen" would have been indistinguishable from "the loop-target half checks
nothing" — and I have now written three tests whose first version could have
passed while exercising nothing (#223's plotted count, #224's zero selections,
this one).

The pattern is worth stating once: EVERY property test needs a second assertion
about how much it actually saw. The property is the claim; the coverage floor is
what stops the claim being vacuous, and it has to be measured rather than
guessed.

## #237 — the bug I nearly reported, and the rule I found instead

The plan was a cross-check between two independent walks of the same bytecode:
`parse_script_disassembly` reports every token's offset, so those offsets ARE the
token boundaries, and a branch's offset ought to be one. The test failed
immediately — `SCRIPT1: branch at 0xc3 is not a token boundary`.

Before calling that a defect I checked whether the premise held, and it appeared
to: the disassembly is perfectly CONTIGUOUS, every row's `offset + len` equal to
the next row's offset, zero gaps across 166 rows. So its offsets really are a
complete boundary set for what it walked.

Then the bytes. `0xBD` holds `0xAF`; the disputed `0xC3` holds `0xA1`, itself a
valid opcode — so the question was whether `0xAF`'s token is 6 bytes or 7. The
VM's own length table at `DS:0x6F18` gives `0xAF` the word `0xFD05`, and reading
`vm_token_advance` (`0x62B6`) settles what that means:

```text
  0x62C6  mov al,[bp+1]          the HIGH byte selects the rule
  0x62CD  mov al,gs:[0x67ad]     non-negative: add the MODE byte and re-read
  0x62F7  cmp al,0xfd            this class...
  0x62FB  mov al,[si] / cmp al,0xa1 / inc si   ...SWALLOWS a trailing 0xA1
```

So `0xAF` is opcode + 5 operands + a swallowed `0xA1` = 7 bytes, exactly what the
disassembly said. TOKEN LENGTH IS MODE-DEPENDENT (`gs:0x67AD`) and one class
absorbs a following `0xA1`. The port's `vm::token_len_at` already implements both,
including the `0xFB` sibling.

So the disassembly was right. Was the branch trace wrong? No: 808 branch offsets
are not token starts, and they are not all `0xA1`, which means `event.offset` is
simply not indexed by token start — it is where the VM recorded a decision, and
that can sit inside a token. NEITHER WALKER IS WRONG. The assumption connecting
them was mine.

Kept as the weaker check that does hold: every branch offset and target lies
inside its script's bytecode. That still catches a wild operand read, which is
the failure it was aimed at.

The lesson is about what a failing cross-check licenses. It says two things
disagree — not which is wrong, and not that either is. Three separate pieces of
evidence (contiguity, the length table, the `0x62F7` ladder) were needed before
the disagreement could be attributed, and the answer was "to me".

## #238 — a test withdrawn because there was nothing to test

`parse_script_post_update` reports the encounter ladder's events with owner,
related and target RECORD offsets — offsets into the DEB, a different file from
the bytecode being walked. That makes a clean cross-FILE bound: a misread operand
yields an offset the DEB cannot contain.

The test failed on its coverage floor: `only 0 record references checked`. The
diagnostic says why — across the five shipped scripts the exporter produces ONE
row, for SCRIPT1, with every offset `None`.

So the test was withdrawn. There is nothing to bound, and an assertion over zero
references is theatre: it would pass forever, look like coverage in the count, and
mean nothing. #219 declined to settle rows that documentation alone had touched;
this is the same refusal applied to a test.

WHAT IS RECORDED INSTEAD, on the function itself: the measurement, and the two
readings it permits. Either the encounter ladder genuinely almost never fires in
the shipped bytecode, or this export needs context it is not given — it takes an
optional `DescriptDb` and the measurement passed `None`. Which is true is
undecided, and the doc says so, because the useful thing here is that nobody
should treat this exporter's output as evidence until someone decides.

The coverage floor earned its keep for the fourth time (#223, #224, #236, now).
Every one of those was a test that would have passed while proving nothing, and in
this case the floor did not just save a vacuous test — it surfaced that a whole
export path is producing nothing.

## #239 — a second layout for a surface the game lays out once

`engine.rs` places the choose-a-location list at `x=6, y=22`, pitch 10, width
150. Four constants, no citation, and a comment calling the result "the game's
list-box nav".

The game has no such layout. Its unified widget (`0x8428`) MEASURES the labels and
derives the box from them — width = widest + 20 @`0x84A1`, height = rows*pitch + 8
@`0x84A7`, x = anchor - width/2 @`0x84AD`. The port implements exactly that as
`ship3d::layout_ship_3d_target_list`, and #220 tested it against the OPTION menu's
own strings out of `BLOODPRG.EXE`. So the port has carried two layouts for the
same kind of surface: one decoded and tested, one invented and asserted.

This is #197's defect class — frontend arithmetic standing in for a decoded rule —
and it survived longer because the comment claimed the opposite. A reader
checking whether the nav list was faithful would have read "the game's list-box
nav" and moved on. That is the specific harm of a provenance claim in a comment:
it does not just fail to help, it actively stops the check.

Labelled APPROX in the source with `layout_ship_3d_target_list` named as the
replacement, and a row added to `docs/port-validation.md`.

NOT FIXED IN PLACE, deliberately. #212 already routed destination SELECTION
through the decoded widget via `console_box`, so this drawing path is a DUPLICATE
to delete rather than a layout to correct. Deleting a live draw path deserves its
own change with the two surfaces compared first — re-laying it out now would
entrench a path that should not exist.

## #240 — correcting #239: not a duplicate, and nearly deleted a feature

#239 called the nav destination list "a DUPLICATE to be removed" and deferred the
deletion to its own change. This is that change, and the first step was tracing
what actually fills the list — which shows the plan was wrong.

`main.rs` builds `nav_destinations` from the SCRIPT3..5 BUNDLES: the label is a
bundle's first actor record, the entries are its parsed dialogue lines. It is a
PORT-SIDE AFFORDANCE for reaching scenes. The game's destination list is a
different thing entirely — DEB candidate records from `0x7259`, routed through
`console_box` since #212, whose click arm sits BEFORE this one in the event loop
and so wins whenever a DEB is loaded.

They are not two renderings of one surface. Deleting this one would have removed
scene access that the decoded path does not provide when no DEB is loaded — a
feature deletion dressed as a faithfulness fix.

The real defect is narrower than #239 wrote, and #239's own fix already addressed
it: the comment claimed "the game's list-box nav" for geometry nothing cites. The
false provenance is gone. The four constants stay, labelled APPROX, because a
reader must not mistake them for decoded values — but there is no decoded rule
they are failing to implement, since the game has no such surface.

WHAT WENT WRONG IN #239: I inferred "duplicate" from the two paths pointing at
similar-sounding UI, without reading what populated either. The correction cost
one command (`grep set_nav_destinations`). Writing "duplicate to delete" into a
validation row made it a scheduled action rather than a hypothesis — and the next
session would have executed it.

## #241 — geometry that WAS decoded, and could be read back

`NAV_PICK_BOX_DEFAULT/BLACK_HOLE/SHIP` carried a one-line comment naming three
addresses and nothing else. Unlike #239's invented `NAV_DEST_*`, these turned out
to be real: each pair is two IMMEDIATES the picker writes into its own scratch
words.

```text
  0x92BF  mov word [0x277a],0xc  / 0x92C5 mov word [0x277c],0xb   default
  0x92CB  test word es:[di-0x18],0x100                            BLACK HOLE?
  0x92D3  mov word [0x277a],0x13 / 0x92D9 mov word [0x277c],0xc   -> its box
  0x92F4  test word es:[di-0x18],0x10                             SHIP?
  0x92FC  mov word [0x277a],0x15 / 0x9302 mov word [0x277c],0xa   -> its box
```

The test decodes the `c7 06 <disp> <imm>` encodings and checks BOTH halves: the
immediate matches the constant AND the displacement is `0x277A`/`0x277C`. A pair
written to the wrong scratch word would still "match" on values alone.

Two structural facts fell out that the old comment did not carry. The default is
written FIRST and overwritten by whichever gate hits, which is why a record
matching neither keeps `(0xC, 0xB)` — the fallback is an ordering, not a branch.
And the gates test `0x100` and `0x10`, the same bits `NAV_CHART_KIND_MASK`
selects on, so a record's hit box and its presence on the chart come from one kind
word; the test asserts that overlap rather than leaving it as a coincidence of two
constants.

Worth putting beside #239: two geometry constants, a day apart, one invented and
one decoded — and neither could be told apart by looking at it. The difference was
entirely in whether the addresses in the comment led anywhere.

## #242 — the VM confesses desync if you let it run

`ExecutionHalt` has four variants and two of them are confessions. `EndMarker` and
`StepLimit` are legitimate ends — a script finishing, or a scene looping while it
waits on input. `InvalidOpcode` and `InvalidTarget` are not: in the game's own
bytecode every token boundary holds a dispatchable opcode and every branch points
inside the script, so either one means the VM lost its place.

That makes running the shipped scripts a test. #235 measures alignment STATICALLY
(the share of decoded opcodes inside `0xA0..=0xD3`); this catches the same failure
by EXECUTION, and it is stricter — a static walk can tolerate a stray byte in the
99% it allows, while an execution halt is absolute.

All five scripts run to a legitimate halt, producing 1534 branch events, every one
inside its script.

Both coverage floors were measured rather than guessed, and the first attempt hid
the second: raising the script count to an impossible value fired before the
branch-event assertion was reached, so I had a number for one and nothing for the
other. Measuring them one at a time gave 5 and 1534. The lesson from #236 — every
property test needs a coverage assertion — has a corollary: when there are two,
raise them SEPARATELY, because the first failure masks the rest.

## #243 — an event that must sit on the opcode that causes it

`ScriptProfileRequestEvent` records the offset where a `0xD2` request fired. That
is checkable against the bytecode: the byte at that offset must BE
`OP_SCRIPT_PROFILE_REQUEST`. An event attributed to the wrong place looks
identical to a correct one — same fields, same plausible values — so no amount of
internal consistency would reveal it. Only the bytecode can.

Also pinned: `pending_script_profile` filters the `0xFFFF` sentinel (`gs:0x6780`
empty, `cmp word [0x6780],-1` @`0x108E`), so a trace whose last request is the
sentinel reports nothing pending, and one whose last request is real reports
exactly that index.

THE COVERAGE IS THIN AND THE TEST SAYS SO. The five shipped scripts issue exactly
TWO profile requests between them, so the offset->opcode assertion runs twice.
That is enough to be non-vacuous and not enough to be reassuring, and the comment
says which — `assert!(events > 0)` would have read as coverage while meaning
almost nothing.

This is the fifth measured coverage floor in this stretch (#223, #224, #236, #242,
now), and the first where the measurement argued AGAINST confidence rather than
for it. That is the more useful direction: a floor that only ever confirms
adequacy is decoration.

## #244 — answering #238, and correcting myself inside ten minutes

#238 left two readings of an empty exporter: the encounter ladder almost never
fires, or `parse_script_post_update` is missing context it is not given. Two
measurements settle it.

Running all five shipped scripts through the VM with a DEFAULT context yields
zero actor pairs, zero presentation handoffs, zero counter bumps. That rules out
"the exporter drops events" — there are none to drop.

I then wrote, in that test's own comment, that this made #238's SECOND reading
(missing context) the live one. That was wrong, and the next test disproved it:
supplying each script's OWN DEB — the context the exporter would get from a
`DescriptDb` — produces zero as well.

So the answer is #238's FIRST reading. THE LADDER DOES NOT FIRE ON SHIPPED
BYTECODE, with or without resolved records, and the exporter reporting nothing is
reporting the truth. Its emptiness is a fact about the scripts: the ladder is
exercised by game state they do not reach on their own.

The correction took ten minutes because the second measurement was already
planned. The mistake was writing a conclusion into a doc after the first of two
measurements — the same shape as #239's "duplicate to delete", where an inference
was recorded as a finding before the evidence that would have refuted it was
gathered. Both times the fix was cheap; both times what made it cheap was that the
next step happened to be the one that checked.

Now settled with numbers instead of a suspicion: `PostUpdateTrace`,
`PostUpdateActorRecordPair`, `PresentationHandoffEvent` and the exporter itself.

## #245 — INFRA is a claim too, so it needs the same care

`recomp/runtime.rs` opens with ten constants that had sat UNVERIFIED: stub
segments, the EMS page frame and store, the modelled CPU speed, the PSP and
environment segments, the memory top. Most were already well documented — the EMS
stub segment's doc even explains why it needs its own segment (the standard
presence check reads `seg(IVT[67h]):000A` for "EMMXXXX0").

Settling them INFRA is the right answer, but it is a CLAIM: it says these make no
assertion about `BLOODPRG.EXE`, so no citation could exist and none is missing.
That is true of nine of them — a host emulator picks where to put a PSP, and the
program only learns the answer from the registers it starts with, so any pair
works.

`MEM_TOP_SEG = 0xa000` is the exception and now says so. It is not a port choice:
`0xA000` is the top of DOS conventional memory, where the VGA window begins, and a
program allocating past it on real hardware writes into video RAM. A platform fact
rather than a game fact, which is still INFRA — but the doc distinguishes them,
because "we chose this" and "the hardware is this" fail in different ways when
someone later changes it.

The three undocumented ones (`PSP_SEG`, `ENV_SEG`, `MEM_TOP_SEG`) got docs before
being settled. Settling an undocumented constant as INFRA would record a judgement
nobody can check — the same objection #219 raised against settling documented-but
-unverified structs, pointed the other way.

## #246 — a real PRNG bug, found by differentialling against the lift

`vm::rand` ports the engine's PRNG (`0x2DE2`). Differentialling it against
`recomp::auto::func_2de2` — the transliterated instruction stream — the DRAWN
VALUES agreed and the STATE diverged after the second draw:

```text
  lifted   af0=6  af1=253
  native   af0=4  af1=254
```

The eight rounds rotate `bl`/`bh` in REGISTERS to build AX. Then `0x2E00 mov bx,
cs:[0xaee]` overwrites BX wholesale with the seed, DESTROYING both rotated bytes,
and the feedback operates on memory:

```text
  0x2E17  sub byte cs:[0xaf1],bl    af1 -= counter
  0x2E1E  xor byte cs:[0xaf0],bl    af0 ^= rol(counter,1)
```

The port used the rotated `bh`/`bl` for that feedback. Fixed to use the stored
bytes.

WHY IT SURVIVED. From an all-zero state the rotated and stored values coincide,
so the first draw matches and the sequence only separates from the second onward.
Every existing test started from a fresh `VmMachine` — `prng_af0/af1 = 0` — and so
sat exactly on the one state where the bug is invisible. The port's own comment
documented the wrong rule too, in the same terms, which is how it read as
deliberate.

WHAT IT AFFECTED: every consumer of engine randomness — the subtitle chatter's
burble picker (`prng(10)+7`), the ship-3D transition gates — drew from a sequence
that diverged from the game's after two values. Not crashes; wrong choices,
forever, in a way nothing would flag.

This is the argument for differentials over citations. `rand` was settled ASM with
an accurate-looking comment citing the right routine, and it was wrong. A lift
does not read the code, it IS the code, so agreeing with it is a stronger claim
than agreeing with a reading of it. `check_liftable_twins.py` lists 27 more
FUNCTION candidates (its raw list of 123 is mostly constants, which cannot be run
beside anything) — each one is this same opportunity.

Settled ORACLE, the status reserved for differentialled rows.

## #247 — the differential queue is 43 functions long

#246's PRNG bug came from running the port beside its lift. The obvious next
question is how many more can be done that way, and the answer is measurable:
matching every unsettled port function against the 112 `func_<hex>` lifts by cited
address gives 43 candidates.

(`check_liftable_twins.py` reports 123, but most of those are CONSTANTS that merely
cite a lifted address — `0x6023`'s field-offset routine accounts for dozens. A
constant cannot be run beside anything. The 43 are the ones where "differential"
means something.)

Second one done: `scan_zero_word` against `func_6293`, agreeing across 200
generated buffers. The two are not identical by construction and the test says
why — the lift is the general routine (scan until the word at SI equals AX, step
past, consume one more byte if it equals AL) and the port's is the `AX = 0`
specialisation WITH A BOUND, because a Rust slice cannot run off its end the way
the original happily does. So they can only be required to agree where the
terminator is in range, which is every case the game's data produces, and the test
states that rather than quietly choosing inputs that hide it.

`0x1E5D` (the interpolation gate) was the next candidate and is left for its own
change: unlike the PRNG's four state bytes, it reads its inputs from DS arrays,
so the differential needs the memory layout established first. That is setup, not
difficulty — but it is more than fits beside another finding.

The queue is worth stating plainly because it is the highest-yield work
identified in this session: 43 functions, each with a transliteration of the
original sitting beside it, and the first one checked found a bug that had
survived every citation-based review.

CORRECTED BY #248: that 43 is wrong. Excluding the lifts themselves, wrappers
that merely cite a routine they call, and addresses already differentialled
against their canonical twin leaves essentially NOTHING. The queue is exhausted,
not full.

## #248 — the differential queue I advertised does not exist

#247 reported 43 differentiable functions and called them "the highest-yield work
identified in this session". That was wrong, and the correction matters more than
the claim did, because a work queue in a doc is what the next session picks up.

Counting properly:

```text
   42  unsettled port fns citing an address that has a lift
   28  DISTINCT addresses among them
   16  of those already differentialled against some port fn
   12  "fresh" addresses
    1  of those 12 attached to a port function at all
    0  of those 1 actually a transliteration of the lifted routine
```

The 12 "fresh" entries are, with one exception, THE LIFTS THEMSELVES —
`func_cc0`, `func_d4a` and friends live in `io_lift.rs` and are ledger rows like
anything else. A lift cannot be differentialled against itself. The exception,
`move_mouse_rel`, cites `0xd0e` because that is the game's mouse handler; it is
host input plumbing, not a port of that routine.

And the 16 already-differentialled addresses are not 16 more opportunities: their
canonical twin is already ORACLE, and the OTHER port functions citing them are
wrappers, callers and variants. `nav_chart_click` cites `0x92A3` because it USES
the pick, not because it transliterates it.

SO THE QUEUE IS EFFECTIVELY EMPTY. Where a port function transliterates a lifted
routine, it has been differentialled — #246 (`rand`, which found a real bug) and
#247 (`scan_zero_word`) were the two that remained obvious, and that is the end of
it until more routines are lifted.

I made the same error twice in two turns, and it is the error I had just
diagnosed: #247 opens by correcting `check_liftable_twins`'s inflated 123 down to
43 by excluding constants — then reports 43 without excluding lifts, wrappers or
already-done addresses. Filtering one confounder and stopping felt like rigour.
The check that would have caught it is the one #246 used on the PRNG: pick the
first item off the list and actually try it.

## #249 — auditing what a guard SKIPS, not just what it reports

The citation guard's summary line ends "83 non-mnemonic lines skipped". That
number had been printed for months and never examined. It is the guard's own
blind spot, reported honestly and read by nobody — including me, for the eight
entries in which I quoted the line.

Breaking the skips down: most are prose that merely looks like a citation —
`si = 0x6752`, "ship-3D … @0x1234", where the first word after the address is a
register or an English word (`si`, `ax`, `ship`, `the`, `kind`). Those are
correctly skipped.

Two were not. `movsx` is real x86 and was simply absent from the guard's mnemonic
set, so `0x9A50 movsx eax,[di]` — how the projector sign-extends its 16-bit
inputs before the depth dot product — went unchecked. Sign versus zero extension
is exactly the error #222's Cauchy-Schwarz bound was written to catch, and the
guard had no opinion on the comment that describes it.

Added `movsx`/`movzx` and the other valid mnemonics missing from the set (`rcl`,
`rcr`, `cwd`, `cdq`, the string compares, the parity/overflow jumps). Verified
count 426 -> 430, skips 86 -> 83, and corrupting the `movsx` citation to `movzx`
now fails with a MISMATCH where before it was silently ignored.

The general point, and the third time this shape has appeared (#204's uncovered
prose form, #211's mixed queue, now this): A TOOL'S OWN "SKIPPED" COUNT IS A
CLAIM ABOUT WHAT IT CANNOT SEE, and deserves the same scrutiny as its findings. A
guard reporting "0 wrong" alongside "83 skipped" is reporting two things, and only
one of them is reassuring.

## #250 — auditing two more quiet numbers, and finding them sound

#249 audited the citation guard's skip count and found a real gap. The same
treatment applied to two other quiet claims produced NEGATIVE results, which are
worth recording because an audit that only reports when it finds something is
indistinguishable from one that was never run.

`classify_plumbing.py`: "674 unsettled fn(s); 0 are pure plumbing". Zero looked
implausible for a tree this size. An independent check — a different pattern,
written without reference to the tool's — also finds zero single-expression
accessors among unsettled rows. The claim holds: the trivial accessors were
settled INFRA in earlier passes, and what remains genuinely carries rules.

`check_content_literals.py`: "369 long string literals in runtime code, 0 reading
as game text". Probing for the shape it would most plausibly miss — ALL-CAPS
multi-word strings, which is what this game's UI text looks like — surfaced 14.
Every one checks out:

  * `HELLO COMMANDER`, `CAP'N BOB SPEAKS`, `CLICK ON THE RED BUTTON`,
    `LIFE SUPPORT:` and the rest sit INSIDE `#[cfg(test)]` modules, where they are
    the EXPECTED values of text read out of `BLOODPRG.EXE` — the opposite of a
    hardcoded literal, since each one asserts the port got it from the binary;
  * `COMMANDER-BLOOD-SAVE 1` is the port's own save-file magic, and its doc says
    so ("the port's line-based save text"). The port also writes the DOS-format
    slot files separately; this is a labelled port format, not a claim about the
    game's.

So both guards are sound where I could probe them. That is a smaller result than
#249's, and it is the reason to record it: the value of auditing a tool's quiet
numbers does not depend on the audit finding a bug, and reporting only the hits
would leave the impression that unaudited numbers are more suspect than audited
ones that came back clean.

## #251 — the last quiet number, and a claim I had been repeating unverified

`check_cited_immediates` ends "18 need reading". I have described those in three
separate turns as "17 `OP_*` dispatch indices covered by
`check_opcode_handlers.py`, plus `STATE_BASE`" — a claim I had never actually
checked. Checking it now:

  * `vm.rs` defines 30 `OP_*` constants; the opcode guard resolves 29 through the
    real dispatch table at `0x142D0`.
  * All 17 of the NEEDS-READING names are among them. The claim is true.
  * The 30th — the one the guard reports as "outside `0xA0..0xD3`" — is
    `OP_MAX = 0xFE`, and it is outside DELIBERATELY.

That last point is the interesting one, because a guard reporting an item as
out-of-range usually means a defect. Here it means the opposite: `OP_MAX` is the
TOKEN bound, not the DISPATCH bound. `OPCODE_DESC` at `DS:0x6F18` has 96 entries
covering `0xA0..=0xFF` and the walker indexes it for every byte; the HANDLER table
at `DS:0x6EB0` is 104 bytes — 52 entries, `0xA0..0xD3` — pinned by the layout
identity `0x6EB0 + 104 = 0x6F18`. So `0xD3..=0xFE` have lengths but no handlers,
and a constant naming the token bound must sit outside the dispatch range or it
would be naming the wrong thing.

The guard's "1 outside" line is therefore correct AND expected, which is the
worst kind of number to leave unexamined: it looks like a finding, it has sat
there for many runs, and confirming it is fine took one command.

That closes the guard-audit sweep begun in #249: one real gap found (`movsx`
unchecked), two claims verified sound (#250), and one repeated-but-unverified
claim now actually verified. Every quiet number in the guard suite has been
opened at least once.

## #252 — 381 rows were filed as "cited" on addresses from other people's comments

Auditing `snd::SILENCE` — origin `0x4049,0xBB6D`, status ASM? — neither address
contains `0x80`. `0x4049` is `int 21h`; `0xBB6D` is `lodsb`. The constant's own
doc names no address at all.

The addresses came from a TEST comment EIGHTY LINES LATER, one I wrote myself in
#213 ("0x4049 overwrites, 0xBB6D averages").

The cause is #141's fix reaching too far. That change taught the inventory to look
for citations in a function's BODY comments, walking forward until the body's
braces close. A `const` has no braces, so `started` never became true, the loop
never terminated early, and it swept up every comment in an 80-line window ahead
of the declaration.

Fixed by abandoning the scan when no brace has appeared within two lines of the
declaration — a real body opens immediately, and anything else is not a body.

THE SCALE: 381 rows changed, every one ASM? -> UNVERIFIED with its origin
cleared. `BLOODPRG_FILE_SIZE` and `BLOODPRG_SHA256` had been filed as citing
`0x14C22,0x14CD2,0x2567` — the font tables — because those appear in a doc further
down the file. The ASM? bucket I have been quoting as "531 rows with citations"
was inflated by more than two thirds; the real figure is 212.

Settled totals are unaffected (ASM? is not settled), and the 421 fn rows that
legitimately carry body-comment citations still do — #141's actual case survives,
since a function's brace appears on the declaration line or the next.

The lesson is about heuristics that widen. #141 fixed a real blind spot by
scanning further; the fix had no stopping condition for items that do not have the
structure it assumed, and it then manufactured citations for six years' worth of
constants. A heuristic that reaches for more evidence needs a rule for when there
is none to reach for.

## #253 — turning constant-to-constant identities into image checks

#252 cleared 381 manufactured citations, which left `bloodprg.rs`'s segment bases
honestly uncited. They are not unverifiable, though — they were being checked the
wrong way.

`parses_mz_header_and_address_conversions` asserted
`fs_to_file(RESOURCE_NAME_TABLE_FS_OFFSET) == RESOURCE_NAME_TABLE_FILE_OFFSET`
and three more of that shape. Those are LAYOUT IDENTITIES: they prove the segment
base and the declared file offset agree with each other. Both could be wrong
together and every assertion would pass.

Now tied to the image. The computed offset must contain `fupcom.spr` and, one
16-byte stride later, `nosound.drv` — the first two slots of the resource table
decoded in #203. The `DS:0x6F18` offset must contain `0xFF03`, the packed length
word for opcode `0xA0` read in #237. Shifting `FS_SEGMENT` by one fails with
"does not land on the resource name table"; before, it would have failed nothing,
because the identity would have shifted with it.

Six rows settled DATA on that basis.

`BLOODPRG_SHA256` is the one that stays unverified, and now says why: the port has
no hashing dependency, so nothing confirms the shipped image is that one. The
constant is a DECLARATION of which binary every address in this project refers
to — worth recording, not the same as a checked fact. `BLOODPRG_FILE_SIZE` beside
it IS checked against the real length, and the content checks above would almost
certainly catch a substituted binary, so the gap is narrow. Adding `sha2` for it
has not seemed worth a dependency, and that judgement is in the doc rather than
left as a silent omission.

## #254 — the same weakness in a guard, and a miscount I introduced fixing it

#253 found `parses_mz_header_and_address_conversions` proving two constants agreed
WITH EACH OTHER rather than with the image. `check_offset_pairs.py` has the same
shape by design: it verifies `file == 0xD420 + ds`, which holds whenever the pair
was written consistently — so a pair naming the wrong table drifts together and
the guard stays quiet.

An image check here is genuinely ambiguous, which is why it was not there. Many DS
offsets are RUNTIME STATE and read as zeros in the shipped file: `DS:0x6D3E`, the
ship-slot array, is all zeros and entirely correct (#192 documented exactly that).
So the guard now REPORTS rather than judges — which pairs land on shipped data,
which on an empty region — and leaves the reader to know which kind each should
be. 20 land on data; the single zero-region hit is `DS:0x6D3E`, the one already
documented as runtime state.

Then I misreported it. The first summary read "22 checked; 20 grounded, 1 zeros",
implying a pair had vanished. It had not: `checked` counts THREE paths (a
name-suffix pair, a doc-block pair, an inferred lone orphan) and only the doc-block
path is classified. Fixed to report the classification against its own total —
"of the 21 named in a doc block" — because a summary line that does not add up
sends the next reader hunting for a bug that is not there.

That is the second time in two entries that adding a number to a tool's output
created a wrong impression before it created a right one (#252's origin column,
this summary). A count is a claim, and a count printed next to another count is
also a claim about their relationship.

## #255 — six offsets grounded, and two wrong assumptions about a dispatch table

#252 left six `bloodprg.rs` file-offset constants uncited. An offset that names a
table is checkable by looking at what is there, and each kind has a signature:

  * `OPCODE_HANDLER_TABLE_FILE_OFFSET` — 52 near offsets, the last NULL;
  * `SCRIPT_RESOURCE_PROFILE_TABLE_FILE_OFFSET` — the words 2, 3, 4, 5;
  * `DIALOGUE_FONT_ASCII_MAP_FILE_OFFSET` — opens with `0xFF` unmapped entries;
  * `DIALOGUE_FONT_ADVANCES_FILE_OFFSET` — glyph widths in `1..=24`;
  * `DIALOGUE_FONT_GLYPHS_FILE_OFFSET` — bitmap rows, not uniform;
  * `RENDER_SPRITE_BLITTER_TABLE_FILE_OFFSET` — ascending near offsets.

I got the handler table wrong TWICE before measuring it. First I asserted the
entries mostly ASCEND; they do not — handler addresses are wherever the linker put
the routines, and only 30 of 51 adjacent pairs ascend. Then I asserted they are
mostly DISTINCT; they are not — 36 distinct for 51 live opcodes, because opcodes
SHARE routines, which the port's own constants already record (`OP_PAIR_RECORD_A`,
`_B` and `_C` all cite `0x6B06`). Both assumptions came from what a dispatch table
"obviously" looks like, and the data said otherwise both times.

The advances check needed a second pass for a different reason. Shifting the
constant by one byte PASSED: the table opens with a run of `0x09`, so a range check
cannot localise it. What pins it is the BOUNDARY — the byte before is `0xFF`, the
ASCII map's unmapped tail, and the first byte of the table is not. With that,
shifting by one fails.

That is the useful shape for any "does this offset point at the right table"
check: uniform data cannot localise an offset, so the assertion has to be about
where the uniformity STARTS.

Six rows settled DATA.

## #256 — I repeated #255's lesson one commit after writing it

Fourteen more `bloodprg.rs` constants — the nine `RENDER_*` entry offsets and the
four sound ones — resolve through their segment bases onto function prologues.
Five are corroborated independently: `0x2F90`, `0x2FA6`, `0x30CD`, `0x3192` and
`0x339E` each have a `func_<hex>` lift, so the recompiler's own boundary analysis,
which never read these constants, picked the same addresses as entry points.

Then I settled all fourteen on a check that does not localise them.

Perturbing `RENDER_UI_TEXT_OFFSET` by one byte PASSED. A prologue is a RUN of
pushes, so `0x0177` lands on `push bx` and "starts with a push" is satisfied — the
identical failure #255 had just diagnosed for the font advances, where a run of
`0x09` swallowed a one-byte shift. I wrote that lesson down, committed it, and
made the same mistake in the next commit against a different kind of run.

Fixed the same way: anchor to the BOUNDARY. Every one of these routines is
preceded by `retf` (`0xCB`) — the end of the previous routine — except the
segment's first function at offset 0, which is preceded by padding. A push is not
a retf, so a one-byte shift now fails.

The generalisation, stated more carefully than in #255: A CHECK THAT AN OFFSET
POINTS AT THE RIGHT KIND OF THING CANNOT LOCALISE IT WHEN THAT KIND REPEATS.
Uniform data, push runs, NOP padding, sentinel fills — all of them absorb an
off-by-one silently. The assertion must be about the TRANSITION into the thing,
not the thing itself. I now expect this to be wrong by default whenever I write
"lands on a <category>".

## #257 — a constant that localises itself

`TEXT_SPEED_POINTER_LIST_DS` and `OPTION_MENU_POINTER_LIST_DS` name `0xFFFF`-
terminated lists of DS pointers. Following them from the image yields the game's
own strings — `VERY FAST / FAST / MEDIUM / SLOW / VERY SLOW` and
`TEXT / MUSIC_OFF / SAVE / LOAD / QUIT`.

Worth recording alongside #255 and #256 because this constant is the OPPOSITE
case. Those two needed a boundary anchor: an offset into uniform data or a push
run absorbs an off-by-one silently. A pointer list does not — shifting the offset
by one byte MISALIGNS every word in the list, so the pointers stop resolving to
NUL-terminated text and the test fails on its own. Verified by doing it.

So the rule from #256 needs its converse stated, or it will be applied
mechanically where it is not needed: an offset localises itself when the data it
points at is SELF-VALIDATING — pointer lists, checksummed records, anything whose
interpretation fails loudly under misalignment. It needs an external anchor only
when the data is uniform or repetitive enough to survive being read from the wrong
place.

Two rows settled DATA.

## #258 — two tables decoded apart, agreeing about the same fact

`SCRIPT_RESOURCE_PROFILE_TABLE` reads out of the image as five rows of five
words — `2..6`, `37..41`, `76..80`, `81..85`, `86..90`, then zeros. That alone
settles `COUNT = 5`, `SLOT_COUNT = 5` and `STRIDE = 10` from the data's shape
rather than from anyone's count, which is the #191 argument again (a stride is a
record's shape, not a number to trust).

The stronger result is what those numbers ARE. Each row is five CONSECUTIVE
resource IDs, and looking them up in the directory decoded in #203 gives exactly
one script's `.cod`, `.bas`, `.var`, `.dic`, `.deb` — in that order, five times
over. The test asserts the extension sequence, not just membership.

Two tables read from different places in the image, neither consulted while
decoding the other, agree about which resources belong to which script. That is
the same evidential shape as #226's entity-table identity and #199's
writer-confirms-reader: not a stronger reading of one thing, but two things that
would have to be wrong together.

It also retroactively supports #203's directory beyond the transcription check it
had. That check compared the port's literal to the image; this one shows the
image's OWN indices are used as script resource sets by a different table, which
is evidence the directory means what #203 said it means.

Four rows settled DATA.

## #259 — decoding forward from a verified entry instead of scanning

#234 left five nav-choice gate addresses uncited on purpose: `find_imm` had hits
for them, but several sat at file offsets like `0x010af`, inside the MZ header,
where "an instruction" is a decode of data. A citation supported only by that scan
is a restatement, and the citation guard cannot tell — it disassembles at the same
phantom address and agrees with itself.

`re/tools/refs_in_routine.py` inverts the method. Given a routine's ENTRY, it
decodes forward linearly to the terminating `ret`/`retf` and reports every fixed
DS displacement the instructions reference. Every hit is inside real code at a
real instruction boundary, because the decode started somewhere known — and the
entries worth passing are exactly the ones #232 and #256 verified land on a
prologue preceded by a `retf`.

Validated against a known answer first: run on `0xB692` it reproduces #218's
transition decode exactly — `0x252F`, `0x2530`, `0x2531`, `0x2533`, `0x0B3B`, each
at the instruction that entry names. A new tool's first run is a test of the tool,
and this one had a right answer waiting.

Eight constants cited from it, all inside routines with verified entries:
`0x0AE4`/`0x0AE5` (the temp-SND gate and phase), `0x252A`, `0x252E`, `0x2527`,
`0x5219` (a FAR pointer — `les di`), `0x524D`, `0x524F`. The guard verifies all
eight; 430 -> 439 checked, 0 wrong.

The general lesson is about search direction. Scanning for a value asks "where
does this number appear?", which in an 86KB image has answers everywhere and no
way to rank them. Decoding from a known entry asks "what does this routine touch?"
— fewer answers, all of them real. When both are available the second is strictly
better, and #234's caution was the right call made with the wrong tool.

## #260 — closing #234's deliberate gap with the evidence it asked for

#234 declined to cite five nav-choice gate addresses because the only support was
a global scan whose hits included the MZ header. It said the citations would need
corroborating a second way. `refs_in_routine.py` (#259) is that second way, and
running it on the five handlers from the `CS:0x0F29` dispatch table finds all of
them inside handler 4:

```text
   0x2736  mov byte [0x2736],1  @0x892C     left motion gate
   0x2737  mov byte [0x2737],1  @0x893C     right motion gate
   0x259B  mov byte [0x259b],1  @0x88C7     menu gate
   0x0B13  mov byte [0xb13],2   @0x8947     sound gate -- value 2, not 1
   0x2A19  mov word [0x2a19],0  @0x87B0     the committed choice, cleared
   0x0ADB  mov byte [0xadb],0   @0x8741     the interpolation tick, reset
```

Six settled ASM, guard 439 -> 445 checked, 0 wrong.

Two details the scan could never have given, because they come from context rather
than from the value:

  * the SOUND gate is set to `2` where the motion gates beside it are set to `1`.
    A scan for `0x0B13` finds the address; only reading the routine shows the
    value differs from its neighbours, which is the kind of thing a port
    transcribing "set the gate" would get wrong.
  * `0x2A19` and `0x0ADB` are touched by THREE handlers each, at
    `0x87B0`/`0x883B`/`0x8956` and `0x8741`/`0x87E4`/`0x887B`. They are shared
    teardown state, not handler-4 locals — visible only because the tool was run
    across all five entries.

`0x2795` remains uncited: it appears in none of the five handlers, so whatever
touches it is elsewhere and #234's caution still applies to it alone.

## #261 — running the routine scan across everything verified so far

#259 built `refs_in_routine.py` and #260 used it on one cluster. Applied to ALL 37
entry points this campaign has verified — the ship-3D segment, the render and
sound segments, the five nav-choice handlers, the resource loader, the candidate
builder, the list widget, the mixer — it reports 144 distinct DS addresses touched
by real instructions at real boundaries.

38 previously uncited constants match one. Each now carries the instruction that
touches it, generated FROM the tool's output rather than transcribed by hand,
because hand-transcription is where eight of this campaign's citation errors came
from (#233's `shr`/`sub` was the last). The guard verifies all of them: 445 -> 483
checked, 0 wrong. Settled ASM.

Among them is `0x2795`, which #260 recorded as appearing in none of the five
handlers and therefore left uncited. That was right — it is touched by
`ship_click_commit` at `0xB0B1` instead. The narrow claim held and the wider search
found it, which is the outcome that makes narrow claims worth making.

WHAT THIS METHOD IS AND IS NOT. It proves the game's code touches an address, at a
named instruction, inside a routine whose entry was independently verified. It
does NOT prove the port's constant is used the same way the game uses it — a
citation says "this address is real and here is where it is read", not "the port's
semantics match". Those 38 rows are ASM in the sense the ledger means (transcribed
from cited assembly), and the semantic question is what the regression tests and
differentials are for.

## #262 — the lifts are 90 more verified entry points

#261 scanned the 37 entries this campaign verified by hand. The `func_<hex>` lifts
are 90 more: each address is a function boundary the RECOMPILER identified, which
is an independent source of entry points and exactly what
`refs_in_routine.py` needs.

Scanning all 90 finds 126 distinct DS addresses. Only 8 uncited constants match —
a much smaller yield than #261's 38, and the reason is informative: the lifts
cover LEAF routines (that is what made them liftable), and leaves touch fewer
fixed addresses than the FSM-heavy code the hand-verified entries covered. The
method did not get worse; the remaining population is different.

The eight are all VM presentation state — `gs:0x67B1`, `gs:0x67F8`, `gs:0x6762`,
`gs:0x6782`/`0x6784`, `gs:0x6776`, `0x67BB`, `0x27E8` — cited from `0x0579D`
onward, inside lifted routines. Guard 483 -> 491 checked, 0 wrong.

DIMINISHING RETURNS ARE THE POINT HERE. 37 entries yielded 38 citations; 90
entries yielded 8. The scan is now close to exhausted against the constants that
have plain hex values, and the rows that remain uncited mostly do not name a DS
address at all — they are counts, sizes, palette indices and port-side values,
which no amount of routine scanning will reach. Saying so now avoids a third
turn spent widening a net that has stopped catching anything.

## #263 — extending a tool, then measuring why the extension must not be used in bulk

`refs_in_routine.py` reported memory displacements only. A constant naming an
ADDRESS appears as `mov ax,[0x2527]`; one naming a VALUE — a mask, a sentinel, a
step — appears as `mov word [0x524d],0xa`, so the tool found the address while
missing the value beside it. Extended to report both.

Validated against known-good work first: run on the transition updater it produces
`mov byte [0x2531],4` @`0xB6A0`, `8` @`0xB6B8` and `cmp word [0xb3b],0x78`
@`0xB699` — which are, verbatim, the citations `SHIP_3D_TRANSITION_OPEN_STEP`,
`CLOSE_STEP` and `OPEN_TIMER_THRESHOLD` already carry. The extension reproduces
existing citations rather than finding new ones there, which is the outcome that
makes it trustworthy.

THEN THE MEASUREMENT THAT MATTERS. Across 18 routines there are 230 distinct
immediates. 137 uncited constants share a value with one — a tempting batch, the
same shape as #261's 38. But 86 of those 137, nearly two thirds, match an
immediate that appears in MORE THAN ONE routine.

A DS address is unique to the thing it names. The number 4 is not. Bulk-citing on
value would have manufactured 137 citations of which most were coincidence, and
the citation guard would have passed every one — it checks that the instruction at
the address has the claimed mnemonic, not that the constant has anything to do
with it.

So the immediate output supports a citation there is already reason to believe,
and is not a source of new ones. Recorded in the tool's own docstring, because the
next person to see "137 matches" will feel the same pull I did.

## #264 — an automatable check that does not work, and one function verified by reading

674 unsettled FUNCTIONS is the largest bucket left, and constants-style automation
does not reach them. I tried anyway, with a plausible idea: a function that
TRANSCRIBES a routine should reference the same DS addresses that routine touches,
so match the port's address constants appearing in a function's body against
`refs_in_routine.py`'s output for its cited entry.

It scores ZERO across 110 candidates, and the reason is that the port is written
well. `update_ship_3d_transition_state` takes a `&mut Ship3dTransitionState` and
writes `state.depth_step`; it never mentions `SHIP_3D_DEPTH_STEP_DS_OFFSET`. The
game's addresses are abstracted into struct fields, which is correct design and
makes the structural check inapplicable. Recorded so the idea is not had again.

So functions get read. `update_ship_3d_transition_state` against `0xB692`,
instruction by instruction:

```text
   test [0x2533],1 / jne     -> if !transition_armed
   cmp [0xb3b],0x78 / jbe    -> if hold_ticks > 120   (jbe: strictly greater)
   [0x2531]=4 [0x252f]=1 [0x2533]=1  -> depth_step/opening/armed
   cmp [0xb3b],0 / jne       -> if hold_ticks == 0 -> start_closing_transition
   [0x2531]=8 [0x2530]=1 [0x2533]=0  -> that helper's three writes
   test [0x252f],1 / jne     -> if !opening
   mov ax,0x14 / lcall rand / or ax,ax / je close
```

The last line is the interesting one. The port does NOT call the PRNG here — it
takes `random_gate_zero: bool`, so the routine is split at the RNG call and the
`0x14` lives in the caller. Following it there: `engine::step_ship_3d_nav_state`
does `self.ship3d_prng.next(20) == 0`. 20 is `0x14`. The transcription is faithful
ACROSS the function boundary, which no per-function check would have shown — and
which is worth confirming rather than assuming, because a split like that is
exactly where a bound gets dropped.

Three rows settled ASM. This is what the remaining 671 cost: one routine, one
port function, one caller, read against each other.

## #265 — reading the depth scroll, and an interaction the port already had right

`step_ship_3d_depth_scroll` against `0xB75C`, line by line, matches. Two details
in it are the kind that get lost in transcription, and the port has both:

  * `add al,[0x2531]` is an EIGHT-BIT add into AL, so the step affects only the
    LOW BYTE of the depth offset. The port has `add_to_low_byte`, named for
    exactly that, rather than a plain `wrapping_add` on the u16.
  * `cmp ax,0x41 / jl` is a SIGNED compare. The port casts to `i16` before
    comparing rather than using the u16 ordering.

Those two interact, which is why both matter and why the doc now says so: because
the add is eight-bit, a step that carries `al` past `0x7F` produces a value the
SIGNED compare reads as negative — so the clamp does not fire and the low byte
keeps its wrapped value. That is the original's behaviour, and a port using a
16-bit add or an unsigned compare would silently diverge only in that corner.

`SHIP_3D_MAX_DEPTH_OFFSET` was REFUSED by `audit_settle` on the first attempt
("ASM needs a cited address") — it is the `0x41` in three instructions and its doc
named none of them. Cited to all three (`0xB768`, `0xB771`, `0xB776`) and settled.
That refusal is the tool doing its job: the constant was verified in my head while
reading the routine, and a settle on that basis leaves nothing for the next reader.

Guard 491 -> 493 checked, 0 wrong. Three rows this turn.

## #266 — a faithful transcription that nothing calls

Reading `copy_ship_3d_plane_bands` against `0xB6DD` confirms the transcription,
including two details worth the reading: the scroll value is
`0x64 - min(depth*2, 0x64)` with a SIGNED `jle` (the port casts to `i16`), and the
whole computation is skipped when the scroll mode is `0xA`
(`SHIP_3D_SCROLL_MODE_HOLD = 10`, `cmp word [0x524d],0xa` @`0xB6F0`).

Then #264's boundary check: the game WRITES the scroll value to `DS:0x524F`, the
port RETURNS it as `new_scroll_value`. Following that to its callers — there are
none. `new_scroll_value` appears in the function and in two tests. So does
`copy_ship_3d_plane_bands` itself: `check_unrouted_rules.py` flags it directly.

THE VGA PLANAR BAND COPY DOES NOT RUN. It is decoded correctly, tested, and
connected to nothing.

Running that guard properly: 111 `pub fn`s have no runtime caller, and 53 of them
carry a binary citation. Fifty-three decoded rules that execute only in their own
tests. A row added to `docs/port-validation.md`, because this is invisible in the
accuracy ledger — a settled ASM row and an unrouted one look identical there, and
I have spent this session raising that ledger without once asking whether the code
it counts is reachable.

The list is not work to do blindly. Some entries are legitimately unused: test
hooks like `special_slot_insert_pub`, alternates like
`ship_3d_target_record_select` whose caller now supplies rows another way. #240
withdrew exactly such a conclusion after tracing what filled a list. But 53 is a
number worth having, and I did not have it.

## #267 — a writer with its own copy of the format

Working the 53 unrouted rules from #266, `bloodsav::parse_slot_directory` stood
out: the port claims to write "the DOS-format slot files exactly as the original
does", so the decoded READER having no caller was worth explaining.

It is not that nothing writes the directory. `main.rs` writes it — with its own
hand-built records carrying literal `15`, `16` and `32`, while `bloodsav` owns
`SLOT_NAME_LEN`, `SLOT_RECORD_LEN` and `SLOT_COUNT` decoded from `0x1BAB`/`0x1BBD`.
Two copies of one format, and the reader was not exercised against the writer.

Checked before changing anything: the layouts AGREE. The writer fills 15 spaces
and writes a name of at most 14 (the edit law's cap at `0x1DD8`), leaving the NUL
at byte 15 that the reader splits on; the filename sits at 16. So this was a
latent hazard rather than a live bug — but "latent" is what a duplicated format
always is, and the way it stops being latent is somebody correcting one copy.

The writer now uses the decoded constants, and a test writes a directory exactly
as `main.rs` does and parses it back, asserting the 14-character name survives,
the filenames land in the right field, and an untyped slot reads as empty rather
than as a run of spaces.

`to_bytes` and `flag_bit` remain unrouted and are NOT the same case — those are
the VM-state save, and whether the port should route through them is a separate
question about `to_dos_save`, which is what `main.rs` calls instead.

## #268 — the aliens animate in place

Third entry working #266's unrouted list, and the first where the gap is visible
on screen rather than latent.

`engine.rs` calls `AlienObject::step()` per frame — the ANIMATION state machine
(`0x16A4`), correctly wired. Its three siblings are called by nothing:
`update_position` (`0x999`), `proximity_visible` (`0xA30`) and `reset` (`0x36A`).
`dispatch()` compounds it by returning `false` for every method except
`AnimStateMachine`.

So the port's aliens cycle animation frames IN PLACE. They do not move, and
nothing culls them by proximity. The game's objects do both, and all three rules
are decoded, tested, and sitting there.

The blocker is real and worth naming exactly rather than filing as "wire it up":
`proximity_visible` and `update_position` both take `camera: [i16; 3]`, and the
alien view has NO camera state in the engine. `NAV_CAMERA_ORIGIN` is the nav
chart's camera, a different surface — reaching for it would be #197's
`compass_angle` mistake in a new place. What is missing is a decode: which overlay
cell holds the alien view's camera, and what updates it.

Recorded in `docs/port-validation.md`. This is the shape #266 predicted the
unrouted list would have — not fifty-three things to connect, but a handful of
real gaps mixed with test hooks and alternates, each needing its own judgement.
Three examined so far: one duplicated format (#267, fixed), one dormant draw path
(#266, blocked on a page the port does not maintain), one dormant behaviour
(here, blocked on an undecoded camera).

## #269 — decoding the blocker #268 named, and finding the port's type is wrong

#268 recorded the alien behaviours as blocked on an undecoded camera. Decoding it
took two commands, and the answer changes what "wire it up" means.

`croolis.xdb 0xA70` adds `word [0x22ec]` for the X term; `0xA62` adds
`word [0x22f0]` for Y. `0x22EC` is genuinely a word (`movsx eax,word ptr
[0x22ec]` @`0xBFA`). `0x22F0` is not a variable at all: the cell at `0x22EE` is
accessed as a DWORD (`mov ecx,dword ptr [0x22ee]` @`0x791`, `add dword ptr
[0x22ee],eax` @`0x1FD5`), and a dword there spans `0x22EE..0x22F1` — so
`[0x22F0]` IS ITS HIGH WORD.

The camera Y is the integer part of a 32-bit fixed-point accumulator, read by
taking the top sixteen bits.

That matters more than the address does. `proximity_visible` and
`update_position` take `camera: [i16; 3]`, and three independent words CANNOT
represent a fixed-point accumulator — the fraction is where the smooth motion
lives. Wiring them by adding an `i16` camera field would have produced aliens
that step rather than glide, which reads as a rendering bug and would have been
debugged as one.

So #268's "not blocked by anything except nobody having done it" was right about
the camera and wrong about the cost: the fix is a signature change, not a call
site. Both docs updated, and `re/CLAUDE.md` gains the `XDB:<name>:0xNNNN` address
form — a third address space beside the executable and the drivers, needed because
the same offset means different things in each overlay.

## #270 — making the type able to hold what the game stores

#269 found the port's `camera: [i16; 3]` could not represent the alien camera,
because Y is the high word of a 32-bit accumulator at `0x22EE`. This changes the
type rather than working around it.

`AlienCamera` carries `x: i16` (`0x22EC`, genuinely a word) and `y_fixed: i32`
(the accumulator), exposing `y()` as the high word — the value `0xA62` adds — and
`axis(i)` for the loop `update_position` walks. Both behaviours take it.

The test is about the fraction, because that is the whole reason for the change:
a third-of-a-unit step must leave the integer part alone for two frames and carry
on the third. An `i16` camera rounds each frame's movement away and, under a small
enough step, never moves at all. It also pins the negative case — `sar` floors
toward negative infinity, so `y_fixed = -1` reads as `-1`, not `0`.

`z` is present for the axis loop and defaults to zero, with a doc saying no
overlay cell is decoded for it. That is the alternative to inventing a third
address to make the type look symmetric.

WHAT IS STILL OPEN, narrowed: the three behaviours still have no runtime caller.
The remaining question is who advances `y_fixed` and by how much —
`add dword ptr [0x22ee],eax` @`0x1FD5` is where it is written, and what computes
`eax` there is the next decode. That is a smaller and better-specified question
than "the camera is undecoded", which is where #268 left it two entries ago.

## #271 — correcting #269 and #270: all three axes are accumulators

#269 decoded the alien camera's Y as the high word of a 32-bit accumulator at
`0x22EE`, and recorded X at `0x22EC` as "genuinely a word" on the strength of
`movsx eax,word ptr [0x22ec]` @`0xBFA`. #270 built `AlienCamera` on that
asymmetry: `x: i16`, `y_fixed: i32`, `z: i16`.

Following the remaining question — who advances `y_fixed` — shows the asymmetry
was mine:

```text
   0x1FC5  add dword ptr [0x22ea], eax     X accumulator
   0x1FD5  add dword ptr [0x22ee], eax     Y
   0x1FE5  add dword ptr [0x22f2], eax     Z
   0x1FEA  movsx ebx, word ptr [0x22ec]    X's HIGH WORD (0x22EA + 2)
   0x1FF0  movsx ecx, word ptr [0x22f0]    Y's
   0x1FF6  movsx esi, word ptr [0x22f4]    Z's
```

Three accumulators, three high words, one shape. `movsx ...word ptr [0x22ec]`
reads sixteen bits because it wants the INTEGER PART — not because the storage is
sixteen bits — and the writer four instructions earlier settles it. Each axis
steps by `[0x22d2 | 0x22d6 | 0x22da] * ebx >> 3`.

`AlienCamera` is now three `i32`s with `axis(i)` returning the high word.

WHAT I SHOULD HAVE DONE. #269 read ONE cell's writer and generalised from a reader
of another. The instruction that stores a value tells you its width; the
instruction that loads it tells you only what the caller wanted. I had the right
rule for `0x22EE` (found the `add dword`) and did not apply it to `0x22EC` (took
the `movsx word` at face value) — in the same entry, four instructions apart in
the same routine.

Two entries built on a half-decode. Both cheap to fix because the next step
happened to look at the writer, which is the same luck #244 relied on.

## #272 — a checker whose first run refuted its own premise

#271's mistake was mechanical — inferring a cell's width from an instruction that
LOADS it rather than one that STORES it — so I built `re/tools/cell_widths.py` to
find it elsewhere: decode forward from known entries, record the widest write to
each DS address, and flag any port constant typed `u16` that names a 32-bit cell
or its high word.

Across 29 verified routines in the executable it found exactly one hit:
`SHIP_3D_PLANAR_FRAMEBUFFER_PTR_DS_OFFSET = 0x5219`, typed `u16`, with four bytes
written at that address.

It is a FALSE POSITIVE, and the reason kills the check. The constant is `u16`
because a DS OFFSET is sixteen bits. The four bytes there are a FAR POINTER —
`les di,ptr [0x5219]`, which #259 cited. The constant's type describes the
ADDRESS, not the contents, and that is true of every DS constant in this tree. So
the match had nothing to say and could only ever produce this.

The real #271 bug lived in a STRUCT FIELD (`x: i16` holding what should have been
a 32-bit accumulator), which no type-matching over constants could reach.

The matching half is deleted. The WIDTH REPORT stays, because that part is real
decode information — which cells the game writes 32 bits wide is exactly what
revealed #271, and having it a command away is worth keeping even though the
automated conclusion I wanted from it does not exist.

Fourth checker in this campaign whose first run was a test of the checker (#204,
#211, #228, now). The difference here is that the test FAILED: the premise was
wrong, not the implementation. Recording that is the point — the alternative is a
tool that reports one known-benign hit forever and slowly trains its reader to
ignore it.

## #273 — the same depth, divided two different ways

`project_star_map_point` read against `0x9BBA`'s projector. The dot products,
the `sar eax,7`, the `add ax,0xa0` / `add ax,0x64` screen centres — all match.

The part worth the reading is the depth. The routine:

```text
   0x9C29  add ecx, 0x10000        the "if negative" fixup
   0x9C30  mov eax, 0x8000000
   0x9C36  shr eax, 7              -> 0x100000, built not literal
   0x9C3D  div ecx                 UNSIGNED, for the scale reciprocal
   0x9C6F  idiv ecx                SIGNED, for each screen axis
```

The SAME depth value is divided unsigned once and signed twice. The port does
both with Rust's `/` on `i32`, which is signed — and that is correct here ONLY
because the `depth += 0x10000` fixup has already made depth positive, where the
two agree. The port has the fixup; it now also has a comment saying what the
fixup is load-bearing FOR, because a reader tidying up "an unnecessary branch on a
value we already know is positive" would remove exactly the thing that makes it
positive.

`0x100000` is likewise not a literal in the routine: `mov eax,0x8000000` then
`shr eax,7`. The port writes the result, which is right, and the doc now records
where it comes from so nobody looks for `0x100000` in the disassembly and fails to
find it.

Guard 493 -> 501 checked, 0 wrong.

## #274 — a bitset that runs the other way

`select_ship_3d_c1_source_record` and the helper it calls, read against `0x6210`.
The routine scans the `gs:0x672c` directory for the object (`di += 0x14` per
entry, counting the index), takes `vm_field_offset(5, 2)` for the bitset field,
adds `index >> 3` to reach the byte — and then tests the bit like this:

```text
   0x6236  and cl, 7
           inc cl            ; cl = (index & 7) + 1
           shl al, cl        ; the tested bit lands in CF
```

`shl al, cl` with `cl = (index & 7) + 1` leaves bit `7 - (index & 7)` in the
carry. So the bitset is HIGH-BIT-FIRST: index 0 is bit 7, index 7 is bit 0 — the
opposite of the `1 << i` anyone writes without looking.

The port has it right (`bit_flag_mask(i) = 0x80 >> (i & 7)`), and now says WHY it
is right, because the equivalence between a shift-into-carry and a mask is not
visible from either side alone. Someone simplifying `0x80 >> i` to `1 << i` would
invert every membership test in the C1 source selection, and the failure would be
"the wrong object is picked sometimes", not a crash.

Everything else matches too: the selector and kind are FIXED at 5 and 2
(`mov ax,5` @`0x6229`, kind 2 — not derived from the object, which the routine's
label already warned about), and `index >> 3` is `shr ax,3` @`0x623B`.

## #275 — a camera origin off by 12000, found by reading a Default impl

`Ship3dCameraApproach::default()` is documented as "phase-3 reset immediates
(`0x8AF2..0x8AFE`)". Reading those three instructions:

```text
   0x8AF2  mov word [0x2f69], 0x4e20     Z = 20000
   0x8AF8  mov word [0x2f71], 0          yaw = 0
   0x8AFE  mov word [0x2f65], 0x2710     X = 10000
```

They never touch `[0x2F67]` — Y. The port's default said `origin_y: 0`, which is
what you get by assuming an unmentioned field starts at zero.

It does not. `0x8CB4..0x8CC0` is the FULL origin reset and writes
`(0x2710, 0x2EE0, 0)`; the shipped image carries the same three words at
`DS:0x2F65`; and `0x2F67` has exactly one writer in the whole executable
(`mov word [0x2f67],0x2ee0` @`0x8CBA`) against two readers, both projectors. So Y
is 12000 whenever the phase-3 reset runs, and the port fed 0 to the projector
instead — a camera origin off by 12000 units on one axis, for every frame of the
approach.

Fixed, with a test that pins all three components three ways: the shipped words at
`DS:0x2F65`, the `c7 06` encodings of both resets, and the ABSENCE of `0x2F67` in
the phase-3 reset's byte range — because "this routine does not write that cell"
is the actual claim the default rests on, and it deserves an assertion rather than
my word.

The general shape: a struct field that a routine does NOT set is a claim about
what came before it, and zero is a guess. #229 had already read the shipped origin
as `(10000, 12000, 0)` and recorded it for `NAV_CAMERA_ORIGIN`; the same three
numbers were sitting in this file with the middle one replaced by a default.

## #276 — auditing the other defaults after #275

#275 found a `Default` whose zero was a guess about a field the routine it cited
never wrote. That is a lens, so I turned it on the tree's other hand-written
`Default` impls.

`Ship3dNavigationFinalResetState` is all zeros, and legitimately: it is the INPUT
state a reset function consumes, not a claim about any cell's value — the reset's
own constants (`SHIP_3D_FINAL_RESET_SCROLL_MODE` and friends) carry the after
values. A derive would do the same job.

`BloodPrng::default()` claimed "the static (unseeded) state from the shipped
BLOODPRG.EXE image", which IS a claim about cells and was not checked anywhere.
It is true: the five state bytes sit at `cs:[0xAEE..0xAF2]` with `cs = 0x1CE`
(base `0x22E0`, confirmed because `0x22E0 + 0xB02 = 0x2DE2`, the PRNG's entry),
and the image holds five zero bytes there. Now read back by a test, along with the
seeder's `mov ah,al` doubling — a seeded PRNG differs from the default in exactly
one field, which is the property #246's bug made worth pinning.

So one of the two claims was worth verifying and both are now recorded as what
they are. The distinction matters more than the result: a zero that MEANS the
game's value and a zero that means "nothing set this yet" look identical in Rust,
and only one of them is safe to rely on.

## #277 — an approximation that had already been replaced

`render_star_map_navview`'s doc says it is "a VISUAL APPROXIMATION ... without the
exact recovered geometry/projection", verified against a DOSBox capture. By the
prime rule that is an APPROX row waiting to be written: a capture-shaped surface
standing in for a decode.

It did not need writing. `engine.rs` renders the nav view with
`render_star_map_navview_projected`, which goes through `project_star_map_point`
— the exact `0x9BBA` arithmetic I verified instruction by instruction two entries
ago in #273. The approximation is reachable only from its own `_panned` wrapper
and from tests.

So the live path is the decoded one and has been for some time. What remained was
a fabricated surface sitting beside the real one with a doc that reads like a
description of what the port does. Marked SUPERSEDED in the source, and a
port-validation row records that the geometry question is closed rather than open.

Kept rather than deleted: its tests exercise the pyramid/orb composition, and
#240 is the standing reminder that "unused duplicate" is a conclusion to reach
after tracing, not before. The end state is removal once those tests point at the
projected renderer — which is a smaller and safer change than deleting a draw path
whose callers I had not checked.

Worth noting how this was found: not by looking for fabricated surfaces, but by
working through the ASM? list and reading a doc that described its own function
honestly. The approximation labelled itself; nothing else in the tree knew it had
been superseded.

## #278 — the intro camera's phases, and the same citation slip a third time

`Ship3dCameraApproach::step` read against `0x8A6A..0x8B5A`. All four phases match,
and all six constants are the routine's immediates: `0x2328` (9000), `0x64` (100),
`0xB4` (180), `0x4E20` (20000), `0x64` again for the Z acceleration, `0x2710`
(10000).

Two signedness differences, checked rather than waved past:

  * P1 ends on `cmp ax,0x2328 / jl` — SIGNED. The port compares `u16 >= 9000`,
    unsigned. They agree for every X the animation reaches (it starts at 10000 and
    falls) and would differ only above `0x7FFF`.
  * P1's yaw wraps on `dec ax / jns / mov ax,0xb4`: it wraps when the DECREMENT
    goes negative, so `0 -> 180`. The port's `if angle == 0 { 180 } else
    { angle - 1 }` is identical over `0..=180` and differs only above `0x8000`,
    which the wrap prevents.

P2's `ja` IS unsigned and matches the port's `<=` directly, and the accumulate
order is the routine's — `z += accel` at `0x8ABF` before `accel += 0x64` at
`0x8AC3`, which is the order that makes the first frame move by zero.

THE CITATION SLIP, for the third time (#213, #233, now). I wrote
`cmp ax,0x2328 / jl` @`0x8A80` — but `0x8A80` is the `jl`; the `cmp` is at
`0x8A7D`. Same for the P2 pair. The guard caught both.

The pattern is now unmistakable: when documenting a COMPARISON, I reach for the
address of the branch, because the branch is what expresses the meaning I am
describing. The rule that fixes it is mechanical — an address pairs with the FIRST
instruction of the sequence quoted — and I have restated it twice without it
sticking. Writing citations from tool output rather than by hand (#261) is the
version that actually works; hand-written ones need the guard every time.

## #279 — making the guard name the fix, not just the fault

Three entries (#213, #233, #278) record the same citation slip: documenting a
COMPARISON, I cite the BRANCH's address, because the branch is what expresses the
meaning. The guard catches it every time and I restate the rule every time, and
it has not stuck.

So the guard now looks for the claimed mnemonic in the twelve bytes before the
cited address and, when it finds it, says where:

```text
   MISMATCH src/ship3d.rs:3425: doc says 0x08a80 is `cmp`, disassembly says `jl`
                                 -- `cmp` is at 0x08a7d, 3 byte(s) earlier
```

Verified by reintroducing #278's exact mistake and watching the message name the
address I had to go and find by hand an hour ago.

That is the useful move once an error recurs: not another restatement of the rule,
but a smaller gap between the report and the correction. The guard already knew
the answer — it had the disassembler, the address and the claimed mnemonic — and
was throwing that away to print a complaint.

The hint is deliberately conservative: it only fires when a backward decode lands
EXACTLY on the probe address with the claimed mnemonic, so it cannot invent a
plausible-looking earlier instruction out of a mid-instruction resynchronisation
— the phantom problem that #106 and #234 are about. When it has nothing to say it
says nothing, and the plain mismatch stands.

## #280 — a doc 110 lines from its struct, and the citation that followed it

Reading `engine.rs`'s ASM? functions, `civil_from_days` claimed `0x0FFB`. It is
Howard Hinnant's civil-from-days algorithm — a date conversion for the TV
channel's seasonal variants — and `0x0FFB` is the game's main-loop coordinator.
Nothing connects them.

The doc above it reads "Per-frame engine state — the subset of the `DS`/`gs`
globals the main loop (`0x0FFB`) touches". That is `EngineState`'s doc. The struct
is 110 lines further down and had NO doc of its own; this one had been stranded
above an unrelated function with no item between them, so the inventory attached
it to the date algorithm.

Two wrongs from one displacement: a port-side utility carrying a binary citation
it has no claim to (and sitting in the ASM? queue as if it transcribed something),
and the engine's central state struct documented nowhere.

Moved to the struct. `civil_from_days` now has no origin and is settled INFRA,
which is what a date algorithm with no game content is; `EngineState` carries the
`0x0FFB` citation that was always meant for it.

This is the same failure mode as #252's manufactured citations, arriving by a
different route: there the scan reached forward past an item's end, here a doc
was simply written in the wrong place and every tool downstream believed it. Both
turn "has a citation" into a claim about proximity rather than about content —
which is why #261's practice of generating citations from tool output, rather than
inheriting them from whatever is adjacent, keeps mattering.

## #281 — 126 undocumented public items, and the hint learns to look forward

#280's stranded doc suggested looking for others, so: 126 public items outside
tests and `recomp` carry no comment of any kind. (My first count said the same
number for the wrong reason — it only checked the line immediately above, so
`OPCODE_DESC`, which has a `///` block followed by a `//` NOTE, showed as
undocumented. Fixing that dropped it out and the total stayed 126 by coincidence.)

Two of them were worth doing immediately, being central decoded helpers used
everywhere:

  * `vm_field_offset` (`0x6023`) — `shl ax,4` for the matrix row, then `bsf bx,bx`
    on the kind. The `bsf` is the point: KIND IS A BITMASK, so column `k` is kind
    `2^k` and kind 0 has no column, which is why the port returns `None` rather
    than reading row-relative garbage.
  * `bit_flag_mask` — the mask form of #274's shift-into-carry, high bit first.

THE GUARD CAUGHT ME AGAIN (tenth time) and this one exposed a gap in #279's hint.
I cited `0x6023` for `shl ax,4`; `0x6023` is the `push bx` prologue and the shift
is at `0x6024`. #279 taught the hint to look BACKWARD — the branch-vs-cmp slip —
but this is the opposite: citing a ROUTINE'S ENTRY for an instruction inside it.
The hint now searches both directions:

```text
   MISMATCH src/vm.rs:646: doc says 0x06023 is `shl`, disassembly says `push`
                            -- `shl` is at 0x06024, 1 byte(s) later
```

Two distinct habits, then: for a comparison I reach for the branch, for a routine
I reach for its entry. Both are "the address I was thinking about" rather than
"the address of the instruction I quoted", and the hint now covers both.

## #282 — finding the routine a constant family had never named

Working #281's list of 126 undocumented items, the `SHIP_3D_FINAL_RESET_*` family
stood out: seven constants asserting specific values (`0x0009`, `50`, `0xff`,
`0xfc`, two `0xFFFF` sentinels) with no docs — and the function that consumes
them, `run_ship_3d_navigation_final_reset`, has NO ORIGIN either. A whole cluster
claiming to describe a reset the game performs, with nothing pointing at where.

`refs_in_routine.py`'s immediate reporting (#263) finds it in one command. Sweeping
the ship-3D entries for those values puts all four distinctive ones in the tail of
`ship_3d_navigation_update` (`0xB34E`):

```text
   0xB505  mov word ptr [0x2793],9      the HUD flag word
   0xB511  mov word ptr [0x279d],0x32   50 ticks
   0xB54D  and byte ptr [0x67aa],0xfc   a MASK, not a value
   0xB57B  mov byte ptr [0x5b52],0xff   the dirty marker
```

So the "final reset" is the tail of the navigation update, not a routine of its
own — which is why nothing had named it.

The `and ...,0xfc` is worth the doc it now has: it CLEARS the low two bits of
what is already there rather than writing a value, so a port storing `0xfc` as a
value would be wrong in a way the constant's name hides.

The two `0xFFFF` sentinels stay UNSOURCED and say so. `0xFFFF` is this tree's
usual empty marker and the reset plausibly writes it, but a scan for it returns
too many hits to attribute — and #263 is exactly the entry establishing that a
value match without a reason is not evidence. Four cited, two honest, none
guessed.

This is the immediate-reporting extension paying off in the way #263 said it
could: not as a bulk citation source, but as a way to find WHERE something lives
once you already know what you are looking for.

## #283 — a selector family whose mismatch value is an `inc`

`resolve_ship_3d_position_field` and its five selector constants had no origin
between them — six items asserting how the game picks a position field, pointing
nowhere. The immediate scan puts the kind ladder in `0x60DD`
(`ship_3d_position_distance`'s front half):

```text
   0x60E3  mov ax,[si]                  the kind
   0x60E5  cmp ax,0x100                 KIND100 -> the comparing branch
   0x60EC  mov ax,0xe  / call 0x6023    relation word, from the DI record
   0x60F9  mov ax,0xc  / call 0x6023    match word, kind 0x100, from SI
   0x6101  mov ax,9
   0x6104  cmp dx,[bx+si] / je          equal -> selector 9
   0x6108  inc ax                       otherwise -> selector 10
```

All five port values match. The one worth a doc of its own is
`..._POSITION_MISMATCH = 10`: in the game it is not a constant at all, it is
`inc ax` on the match selector. The port naming it separately is fine — but a
reader looking for a `10` in the disassembly finds nothing, and would conclude the
value was invented, which is the same trap `0x100000` set in #273 (built by
`mov eax,0x8000000 / shr eax,7` rather than written).

Two constants derived by arithmetic from a third, in two different routines,
within ten entries of each other. Worth stating as a habit of this codebase: when
a port constant cannot be found in the binary, the next question is not "is it
invented" but "is it computed".

Six rows settled ASM; guard 513 -> 527 checked, 0 wrong.

## #284 — three C1 constants, and a citation the guard passed for the right reason

Continuing #281's undocumented sweep into the C1 source/destination constants,
all of which had empty origins.

`SHIP_3D_SOURCE_BITSET_SELECTOR = 5` and `_KIND = 2` are the pair #274 already
established as FIXED at the bit test's call site — `mov ax,5` @`0x6229`,
`mov bx,2` @`0x622C`, `call 0x6023` @`0x622F`. Now cited rather than only
described in a neighbouring entry.

`SHIP_3D_C1_DESTINATION_SELECTOR = 19` comes from the C1 SET path: `0x6C2F` calls
the bit test and takes `jb` to `0x6C48` when it returns carry, where
`mov ax,0x13` / `mov bx,0x10` resolves the destination field and
`mov word es:[bp],0xc1` writes the record type.

A NOTE ON THE GUARD. My first draft wrote "`mov bx,2` feeding the same
`call 0x6023` @`0x622F`", and the guard passed it. That is correct behaviour, not
a miss: its rule pairs an address with the LAST backticked run adjacent to the
`@`, which here is `call 0x6023` — and `call` IS what is at `0x622F`. The prose
mentioned `mov bx,2` without claiming an address for it, so there was nothing to
check.

Which is a limitation worth naming: the guard verifies the citations you MAKE, and
prose can describe an instruction without citing it. Tightened to give `bx` its own
address, because an uncheckable mention beside a checkable one reads as though
both were verified.

## #285 — two constants, two modules, one selector

`SHIP_3D_FIELD_SELECTOR_PARENT_LINK = 17` and `vm::VM_FIELD_OFFSET_SELECTOR_C2 =
0x11` are the same number, and both come from the same instruction:
`mov ax,0x11` @`0x625B`, inside the nav source-list builder (`0x624B`), where the
walk resolves that selector to find whether an object is a CHILD of the current
target.

Both had empty origins. Both now cite it, and each says the other exists.

NOT MERGED, deliberately. The names describe different things about one value:
the ship-3D name says what the FIELD MEANS at that call site (a parent link), the
VM name says which OPCODE FAMILY reaches it. Collapsing them into one constant
would force a single name to carry both, and the loser would be whichever module
did not own it. `check_duplicate_rules.py` reports no same-name duplicates, and
its own output says the clusters it does show are "for judgement" — this is one
of those judgements, now recorded so the next reader does not spend the same
minutes deciding whether it is a bug.

The generalisable point is small but real: a duplicated VALUE is not automatically
a duplicated RULE. #267 found two copies of one format that genuinely needed
merging, because a writer and a reader must agree byte for byte. Two names for one
selector, each used in its own module for its own reason, do not.

## #286 — the three C1 source-kind constants, cited from the routine that branches on them

`SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG = 1`, `SHIP_3D_C1_SOURCE_KIND_BITSET = 2`
and `SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG = 2` all had empty origins. Per #263's
rule they could not be attributed by value — 1 and 2 appear in nearly every
routine in the image, so a value match would have manufactured evidence rather
than found it. They had to come from the code that GATES on them.

Route to it: the 0xC1 handler is `0x6B4C` (`dump_handler_table.py`). Its kind-0x10
arm rebuilds the source list (`bp = 0x6886`, `call 0x624B` @`0x6C11`) and then
scans it from `0x6C1C`. All three constants are three consecutive branches there:

    0x6C1C  lodsw                          ; next source entry
    0x6C1D  cmp ax, -1 / je                 ; SHIP_3D_TARGET_EXIT_SENTINEL
    0x6C24  mov ax, word ptr es:[bx]        ; the record's KIND word
    0x6C27  cmp ax, 2   / jne 0x6C36        ; ..._KIND_BITSET
    0x6C36  cmp ax, 1   / jne 0x6C1C        ; ..._KIND_OPERAND_FLAG
    0x6C3B  mov bx, word ptr [0x6736]
    0x6C3F  test byte ptr es:[bx + 2], 2    ; ..._OPERAND_STATE_FLAG

The third one settled more than its own value. The port's
`select_ship_3d_c1_source_record` takes an `operand_state_flags: u8` parameter,
which on its own says nothing about WHERE that byte comes from. `bx` here is
loaded from `[0x6736]` — the operand the handler stashed at `0x6B6D` — so the
byte tested is the OPERAND RECORD's `+2` flags, which is what the port passes.
A parameter that was merely plausible is now pinned to a specific byte of a
specific record.

DECODED BEHAVIOUR THAT WAS NOT TESTED: `jne 0x6C1C` @`0x6C39` targets the `lodsw`
at the TOP of the loop, so a kind that is neither 1 nor 2 RESUMES the scan. That
is the port's `_ => {}` arm, which until now read like defensive padding. Added
`c1_source_selection_skips_unknown_kind_and_keeps_scanning`, using kind `0x10` —
the kind whose match ENTERS this scan (`cmp ax,0x10` @`0x6C07`), so the one most
likely to reach the loop without being one of its arms.

PERTURBED to check the test is not vacuous (#225's lesson): changing `_ => {}` to
`_ => return None` fails it. It also fails
`c1_source_selection_uses_current_source_cursor_for_kind2_bitset`, which was
already covering the same arm incidentally — worth knowing, since that coverage
was accidental and would have vanished the moment that test's fixture changed.

Citations: 542 verified (from 534), 0 wrong. 597 lib tests, 0 failures.

### #286a — two settled counts, and which one this project reports

Settling the three constants above moved the ledger from 1308 to 1311 "settled",
a number 305 higher than the 1006 carried in the previous report. Nothing was
mass-settled: the two numbers use DIFFERENT RULES, and the gap is exactly the 302
provisional rows plus this entry's 3.

    total 2216
      UNVERIFIED             905   nothing recorded
      provisional (ends ?)   302   ASM? 206, DATA? 46, ORACLE? 41, INFRA? 9
      CONFIRMED             1009

    STRICT   1009 / 2216 = 45.5%   provisional counts as OPEN
    LENIENT  1311 / 2216 = 59.2%   provisional counts as SETTLED

STRICT IS THE NUMBER THIS PROJECT REPORTS. An `ASM?` row means someone wrote down
a plausible-looking origin that has NOT been checked against the image — that is
the state eleven citation slips were found in this session (#213, #233, #278,
#281 among them), every one of which looked settled until the guard disassembled
the address. A rule that counts unchecked claims as done would have hidden all
eleven, and would report the most progress precisely when the least checking has
happened.

Recording it because the lenient number is the flattering one and it is one
`collections.Counter` away at any moment. The trap is not arithmetic, it is that
both numbers are true statements about the same file and only one of them is
about VERIFIED work.

Ledger after #286: 2216 items, 1009 CONFIRMED (45.5%), 1207 open — 905 with
nothing recorded and 302 carrying a claim that no tool has checked.

## #287 — the two 0xFFFF sentinels: stop scanning the image, decode the routine

#282 cited four of the navigation final reset's immediates and left two
UNSOURCED with an explicit reason: "a scan for `0xFFFF` returns too many hits to
attribute". That reason was about the METHOD, not about the binary. `0xFFFF` is
untraceable across a 200KB image; inside ONE routine it is nearly unique.

Decoding the reset tail forward, there are exactly two `0xFFFF` word stores:

    0xB529  mov word ptr [0x1fab], 0xffff
    0xB52F  mov word ptr [0x6788], 0xffff

`labels.csv` already names both cells, from the dialogue work: `DS:0x1FAB` is
`vm_text_selector` (the signed per-line SELECTOR the 0xA6 TEXT handler writes
from its third byte) and `DS:0x6788` is `vm_active_line` (the active dialogue
line id, whose own label ends "reset 0xffff on clear"). Selector -> the SELECTOR
constant, active line -> the ACTIVE RECORD constant, and the port's two fields
are already written in that order.

A SECOND finding fell out of reading the tail rather than grepping it: from
`0xB521` the reset stops being navigation-specific. That address is labelled
`dlg_clear_b` — the routine INLINES the dialogue clear instead of calling it
(`dlg_clear_a` at `0x1A5E` clears the same pair). `xor ax,ax` there zeroes a
register that every following store reuses, which is exactly why these two
sentinels are full `mov word [addr],imm` instructions while their neighbours are
one-byte `mov [addr],al`. The instruction FORM is what made them findable.

WHAT NO TEST CAN CHECK HERE, stated rather than papered over: both constants are
`0xFFFF`, so `assert_eq!(state.scene_selector, SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL)`
passes even if the two fields are swapped. The existing reset test asserts both
and would survive the mapping being backwards. The mapping rests on the labels
and the store order, NOT on anything the port is able to assert — and it cannot
be strengthened until the port models these as DS cells rather than as named
struct fields. Recording it because a green test next to a citation reads like
confirmation, and here it is not.

Citations: 546 verified (from 542), 0 wrong. 597 lib tests, 0 failures.

## #288 — the viewport descriptor: a table no search could ever have found

`SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR` was the clearest UNVERIFIED row left in
`ship3d.rs` — sixteen bytes with no citation, no derivation, and a doc that said
so plainly: "WHERE the game builds it is undecoded — presumably assembled by
consecutive stores rather than copied from a table, which is why no table exists
to find." `check_literal_tables.py` agreed, reporting it ABSENT from every
shipped image.

The guess was right and the routine is the temp-SND setup's tail. Route to it:
`SHIP_3D_TEMP_SND_SETUP_OFFSET = 0x05F1` in the ship-presentation segment
`0x0A9A` (base `0xAFA0`) = `0xB591`, decoded forward to `0xB629`:

    0xB629  les di, ptr [0x522d]   ; FAR POINTER destination, not a DS cell
    0xB62D  xor eax, eax
    0xB630  stosw                  ; [0] = 0
    0xB631  inc ax
    0xB632  stosw                  ; [1] = 1
    0xB633  add ax, 3
    0xB636  stosd                  ; [2],[3] = dword 4
    0xB638  mov ax, 0x140 / stosw  ; [4] = 320
    0xB63C  mov ax, 0xc8  / stosw  ; [5] = 200
    0xB640  xor eax, eax
    0xB642  stosd                  ; [6],[7] = dword 0

WHY NO SEARCH COULD HAVE FOUND IT, which is the transferable part. Three of the
six values are COMPUTED — `xor`, `inc`, `add ax,3` — so the bytes `00 00 01 00
04 00` never appear in the image, and only `0x140` and `0xC8` exist as
immediates. A byte search fails, `find_imm` on 0/1/4 is useless (#263), and
`check_literal_tables.py` correctly reported ABSENT. Absence of the DATA is not
absence of the TABLE; it is evidence the table is BUILT. #229 already recorded
that ABSENT has good explanations — this is a fourth one, and the most common
kind: arithmetic.

A STRUCTURAL CORRECTION fell out of it. The two `stosd`s mean this is NOT eight
independent words: indices 3 and 7 are the HIGH HALVES of 32-bit stores. The real
shape is `u16, u16, u32, u16, u16, u32`. The port keeps `[u16; 8]` because that
is how the block is copied, but anything that starts INTERPRETING index 3 or 7 as
a field is reading half of a dword — the same class of mistake as #271, where a
16-bit load hid a 32-bit accumulator, caught here before it could be made.

TEST: `temp_snd_viewport_descriptor_matches_the_stores_that_build_it` re-derives
the array by EXECUTING the store sequence rather than restating it — asserting
the constant equals itself would prove nothing. Perturbing the constant's `4` to
`5` fails it. The high halves are asserted separately so a later edit cannot
quietly promote them to fields.

Citations: 557 verified (from 546), 0 wrong — all eleven instructions in the
block above confirmed against the image by the guard. 598 lib tests, 0 failures.
Labelled `0xB629` as `ship_3d_temp_snd_viewport_descriptor_write`.

## #289 — documenting a struct turned up a divergence, and a doc I had to retract mid-edit

`Ship3dPositionRecord` had no doc at all. Writing one meant answering what each
field IS, and that produced three results, one of which was a correction to what
I had already typed.

WHAT THE STRUCT IS. Not a byte-layout mirror. In the game a record has no fixed
field positions — every access goes through `vm_field_offset(selector, kind)`
(`0x6023`), which indexes the matrix at `DS:0x6D60` by selector and by the
record's own kind, so one selector lands at different offsets in records of
different kinds. The struct pre-resolves the three selectors the walk needs.
`offset` is the record's ADDRESS; the rest are fetched VALUES.

CONFIRMED FROM THE MATRIX, not from the names: selectors 9, 10 and 12 are
nonzero in column 8 and nowhere else, and column 8 is kind `0x100` (the column is
the kind's lowest set bit, `bsf` @`0x6027`). So the `kind100_` fields really do
exist only for kind-`0x100` records, and `None` elsewhere is the table's answer
rather than missing port data. Pinned in `field_matrix_entries_match_the_constants`,
which reads the image.

THE RETRACTION. I had already written that "a `vm_field_offset` result of 0 means
THE KIND HAS NO SUCH COLUMN" and that "the walk tests exactly that". Both wrong,
caught by checking instead of shipping the sentence. `0x6023` has no
zero-handling, and its callers differ: the distance routine adds the result
UNCONDITIONALLY (`add ax,si` @`0x6121`), so for kind `0x40` — whose selector-11
column IS 0 — the position field legitimately sits AT the record's start. Zero is
a real offset. Only code that explicitly tests it treats zero as absence, and the
position walk does not.

WHICH EXPOSED A REAL DIVERGENCE. The port's arm does
`if parent_field == 0 { return None }`; `0x61C9` does `add si,ax / mov si,[si]`
with no test, so the game would read the record's KIND WORD as the next pointer.
Kind `0x20` is the live candidate (its parent column is 0 and neither branch
catches it first). NOT FIXED, deliberately: kinds `0x20`/`0x40` populate
selectors {0, 21, 22, 23}, disjoint from the object kinds' {11, 17, ...} — the
signature of a different record family the walk is never handed. That is an
argument from the table's shape, not a traced call, so it is written up as an
OPEN RE QUESTION in docs/port-validation.md with what would settle it. Changing
the code on a suggestive-but-unproven basis would trade a decoded path for an
undecoded one.

ALSO CORRECTED: #283 described this function as `0x60DD`'s front half. It merges
TWO routines — `0x60DD` tests only `0x100` and `0x40` and delegates the rest to
`0x61A6` (`call` @`0x6126`), which is where kinds 8/`0x10`/`0x200` and the parent
walk actually live. The union `{8, 0x10, 0x40, 0x200}` is behaviourally right and
all four resolve selector 11, but NO SINGLE ROUTINE tests all four, and the doc
now says so.

Citations: 570 verified (from 557), 0 wrong. 701 tests across all binaries, 0
failures.

## #290 — "wired but never fed": a gap neither the tests nor the unrouted checker can see

Chasing #289's kind-`0x20` question meant asking what real records reach the
position walk. The answer is none, and the reason is structural.

`resolve_ship_3d_position_field` is called from the VM's `step()` — production
code — so `check_unrouted_rules.py` reports it as routed. But its input lives in
`Ship3dC1PositionRuntime`, populated only by `with_ship_3d_c1_positions`, and
every call site of that builder is inside `#[cfg(test)]`. `#[cfg(test)]` opens at
`src/vm.rs:7190`; the call sites are at 13625, 13686, 13756. In a real run the
field is always `None` and the C1 arm early-returns "no redirect", so the game's
distance gate (`call 0x60DD` @`0x6BEA`, redirect when nonzero) is a branch the
port never takes.

WHY NOTHING FLAGGED IT. The tests supply the records themselves, so they exercise
every branch and pass — a green suite is exactly what a wired-but-unfed subsystem
produces. The unrouted checker asks "does this rule have a caller?", and the
answer is legitimately yes. Neither instrument is wrong; the class simply sits in
the gap between them.

`tools/check_unfed_runtime.py` now reports it: public builders whose call sites
are ALL in the test module, with the field each one writes. Nine of them.

THE CHECKER MISSED ITS OWN MOTIVATING CASE on the first run, which is worth
recording because it nearly shipped. The regex required `mut self` on the same
line as `pub fn with_...(`, and `with_ship_3d_c1_positions<I, J>(` wraps its
receiver to the next line — so the tool found seven builders and silently omitted
the two that prompted it. A checker that cannot find the case it was written for
is worse than none, because its clean output reads as evidence. Fixed by matching
the name and then looking for the receiver over the following lines; both now
appear.

THIS SUBSUMES #289's open question. The kind-`0x20` parent-link divergence cannot
manifest while the walk never runs on real data, so the order of work is FEED THE
RUNTIME FIRST — find what builds the position record list in the game and wire it
— and only then is the zero test observable. Both docs/port-validation.md entries
now say so.

Not a defect in every case: some builders legitimately construct fixtures for a
subsystem fed another way. Each row is a QUESTION — "in a real run, what is
supposed to call this?" — and the tool prints rather than fails.

## #291 — feed the position runtime from the state table (necessary, NOT yet sufficient)

#290 found `Ship3dC1PositionRuntime` populated only by a test-only builder, so in
every real run the C1 distance gate took its "no redirect" arm and the decoded
position subsystem never executed. This derives that runtime from the state table
instead.

NO NEW DECODE — every value is read the way the game reads it. The records ARE
the `gs:0x6724` table (`les di,gs:[0x6724]` @`0x6B4D`); a record's kind is the
word at its start (`mov ax,[si]` @`0x61AB`); links resolve through
`vm_field_offset(selector, kind)` (`0x6023`); the coordinate pair is two
consecutive words at the resolved field offset (`lodsw` @`0x6176` for x,
`mov bx,[si]` @`0x617D` for y); and a `0xFFFF` link means fall back to the arche
object (`cmp si,-1` @`0x61CD`, `mov si,gs:[0x6752]` @`0x61D2`), which the port
already resolves BY NAME exactly as `0x5490` does.

REACHABLE-CLOSURE, not enumeration. The walk only ever looks records up by
offset, so deriving the offsets it can reach — the two operands, the arche
fallback, and whatever the parent links lead to — makes every lookup succeed
without inventing an object list the port does not have. Bounded at 64 so
malformed save data cannot spin.

NEW CONSTANT, not a reuse: `SHIP_3D_PARENT_LINK_SENTINEL`. #285's rule cuts both
ways — a shared VALUE is not a shared RULE — and this `0xFFFF` has its own
instruction, routine and meaning ("no parent, use the arche"), which none of the
reset sentinels share.

VERIFIED BY EQUIVALENCE, which is the part worth trusting:
`execution_trace_ship3d_c1_positions_derived_from_state_match_supplied` runs the
SAME state and SAME runtime as the existing fixture test with the positions
REMOVED, and reaches the same end state — now by resolving kind `0x10`/`8`
records and reading selector-11 fields at `+0x18` (matrix columns 3 and 4 both
hold `0x18`) rather than by early-returning. A second test pins parent-link
following and the `0xFFFF` fallback directly.

STILL NOT REACHED IN PRODUCTION, and this must not be read as "#290 is fixed".
`write_c1_record_state_ship3d` early-returns when `context.ship3d_c1_runtime` is
`None`, and `with_ship_3d_c1_runtime` remains test-only — `check_unfed_runtime.py`
still reports it, and I re-ran it to confirm rather than assuming the change
helped. So the position half no longer needs a fixture, but the OUTER gate still
does. Three things must be derived before any of this executes on real data:
`navigation_records` (readable from state the same way), `object_table_records`,
and `source_list_bytes` (which should not be an input at all — the game BUILDS it
at that moment, `call 0x624B` with `bp=0x6886` @`0x6C11`, and the port already has
the builder). That is the next task, and it is the whole of what stands between
this subsystem and a real run.

Citations: 577 verified (from 570), 0 wrong. 703 tests across all binaries, 0
failures.

## #292 — the live C1 handler was missing the distance redirect; #290's diagnosis was half right

Deriving the outer C1 runtime (#291's stated next task) turned out to rest on a
wrong premise, and finding that out was most of the work.

WHAT #290 SAID: `Ship3dC1PositionRuntime` is fed only by a test-only builder, so
in every real run the C1 distance gate takes its "no redirect" arm and the
decoded position subsystem is inert.

WHAT IS ACTUALLY TRUE: there are TWO C1 implementations in this tree.

  * `VmMachine::c1_set_plan` — the LIVE path, the one `main.rs` and `vm_drive.rs`
    run. It IS fed: `self.directory` is the DEB object directory and
    `build_nav_source_list` is a faithful port of `0x624B`.
  * the `ExecutionContext` path (`resolve_c1_record_state_ship3d_target`) — a
    trace/analysis path, which production reaches only with
    `ExecutionContext::default()`, hence unfed.

So the conclusion "the position machinery never runs live" was right, but the
REASON was wrong. It was not that the C1 path is starved; it is that the live
handler is NARROWER. `c1_set_plan` implemented the owner-active check, the
kind-`0x10` source list, the kind-1 arm and the destination write — and skipped
`0x6BE0`..`0x6C02` entirely, so an owner that should have been redirected to its
parent first was judged on its own kind. The fuller decode existed since #283 but
only on the path nothing feeds.

FIXED ON THE LIVE PATH: `c1_set_plan` now runs the redirect —

    0x6BE0  cmp ax,2 / je      the operand word selects the mode...
    0x6BE5  cmp ax,1 / jne     ...being 1 or 2, else skip to 0x6C04
    0x6BEA  call 0x60DD        distance between the operand and owner records
    0x6BED  or ax,ax / je      zero distance -> no redirect
    0x6BF3  mov ax,0x11 ...    otherwise follow the owner's parent link
    0x6BFF  cmp ax,0x10 / jne  the redirected target MUST be kind 0x10

with `c1_position_records`, the same derivation as #291 against `rec_read`
instead of a byte array — `VmMachine` stores records word-addressed. The DECODE
is shared; only the accessor differs.

VERIFIED BY PERTURBATION, because a passing test proves nothing here: the new
`c1_distance_redirect_moves_the_write_to_the_parent_on_the_live_path` uses WHERE
the record lands as the discriminator (`parent + sel13` vs `owner + sel13`).
Disabling the redirect leaves the parent slot at 0 instead of 0xC1 and the test
fails; the zero-distance half pins the `or ax,ax / je` arm from the other side.

STILL MISSING on the live path, named so it is not mistaken for done: the KIND-2
BITSET arm (`cmp ax,2` @`0x6C27` -> `call 0x6210`). `c1_set_plan` tests only
`rec_read(entry) == 1`. That arm needs the source list as raw BYTES for the
bitset base, and `build_nav_source_list` returns a `Vec<u16>` of offsets, so it
is a real piece of work rather than a line.

Citations: 579 verified (from 577), 0 wrong. 705 tests across all binaries, 0
failures.

## #293 — why the live kind-2 arm needs a BUFFER: the offset is 0x1E

#292 left the kind-2 bitset arm unported on the live path and gave the reason as
"it needs the source list as raw bytes". That was a description. This is the
measurement, and it changes the shape of the remaining work.

`0x6210` ends in `mov al, byte ptr [si]` @`0x6240`. That is DS-relative, and DS
is GS at the C1 call site (`mov ax,gs / mov ds,ax` @`0x6C15`), so the byte comes
out of the `DS:0x6886` SCRATCH BUFFER — not out of a record. `si` is the
post-`lodsw` cursor, and the routine adds `vm_field_offset(5, 2)` before reading.

THAT OFFSET IS 30. Read from the matrix, now pinned by a test against the image:
selector 5 is populated in exactly ONE column, kind 2's, which is also why
`0x6210` can fix both its selector and kind (`mov ax,5` @`0x6229`,
`mov bx,2` @`0x622C`) instead of deriving them — there is no other kind to
derive.

The consequence: the bitset byte sits at `cursor + 0x1E + (index >> 3)`. For any
source list shorter than about sixteen entries that is PAST the `0xFFFF`
terminator, in whatever the scratch region happens to hold. The read is real and
the port's `ExecutionContext` path reproduces it exactly, because it takes
`source_list_bytes` and indexes it the same way.

`VmMachine::build_nav_source_list` returns `Vec<u16>` of ENTRIES. There is no
buffer to index thirty bytes into, so the live kind-2 arm cannot be ported by
translating the branch — it needs the port to maintain a real `0x6886` region
with the lifetime the game gives it. Writing the arm against a synthesised
"entries then zeros" buffer would produce a bit test that always reads zero and
therefore never fires, which would look implemented and behave as if it were
absent. That is worse than the current honest gap, so it is NOT done.

RECORDED RATHER THAN GUESSED: whether the game intends to read past the
terminator, or whether the scratch always holds a longer list by the time C1
runs, is undecided here. The instruction is unambiguous; its intent is not.

Citations: 581 verified (from 579), 0 wrong. 706 tests across all binaries, 0
failures.

## #294 — the kind-2 bitset arm on the live path, via a PERSISTENT 0x6886 scratch

#293 said this arm could not be ported by translating the branch, because the
bitset byte sits at `cursor + 0x1E` — past the terminator for a short list — and
`build_nav_source_list` returns a `Vec<u16>` with nothing to index into. It also
said building a synthesised "entries then zeros" buffer would be worse than the
gap, since the test would always read zero and never fire.

The way out was the region's SIZE. `DS:0x6886` is the source list; the next
labelled cell is `DS:0x6A16`, the active-object candidate list `0x604E` builds.
So the scratch is `0x190` = 400 bytes, and `cursor + 0x1E` stays INSIDE it. The
bytes the bit test reads are not undefined memory — they are whatever the
PREVIOUS build left in the same buffer.

That makes it modellable without inventing anything: `VmMachine` now carries
`nav_source_scratch`, 400 persistent bytes. `refresh_nav_source_scratch` writes
entries plus the `0xFFFF` terminator (`mov word ptr [bp],0xffff` @`0x6289`) and
TOUCHES NOTHING ELSE, exactly as `0x624B` does. `c1_set_plan` then runs the full
scan through the already-tested `select_ship_3d_c1_source_record`, so both arms
and the unknown-kind fall-through come from one implementation.

VERIFIED FROM THREE SIDES, because a single passing assertion would not
distinguish this from the old kind-1-only code. With a one-entry list the cursor
is 2 and the byte read is `scratch[0x20]`, past the terminator at offset 2:
  * bit SET there before stepping (as a previous build would have left it) -> the
    arm fires and the record is written;
  * bit CLEAR -> nothing written;
  * the WRONG bit set (`0x80`, index 0) with the operand at directory index 1 ->
    nothing written, which pins the high-bit-first mask `0x80 >> (i & 7)` from
    the failing side rather than the passing one.
The test also asserts the builder did not clear that byte, which is the property
the whole design rests on.

DISCREPANCY FOUND, NOT FIXED. The three exits of the scan differ in the binary:
the write path (`jmp 0x6C7A`) and the branch path (`0x6C73`, which calls
`vm_branch`) both `pop si / pop ds` first, but the SENTINEL path
(`cmp ax,-1 / je 0x6C7C` @`0x6C20`) jumps straight to `pop di / ret`, skipping
those two pops AND the branch. The port maps sentinel and rejection to the same
`Some(None)` -> `branch()`. Two things follow and only one is settled: the port
branches where the game does not, and — on my reading of the pushes at `0x6B4C`,
`0x6B71` and `0x6B72` — the sentinel path also leaves two words on the stack.
A stack imbalance in shipped code is far more likely to be MY misreading than a
real bug, so it is recorded as a question rather than acted on. Either way the
missing `vm_branch` is a genuine behavioural difference worth settling before
anyone relies on the sentinel arm.

Citations: 583 verified (from 581), 0 wrong. 707 tests across all binaries, 0
failures.

## #295 — the C1 scan's sentinel does NOT branch; a port defect a test was pinning

#294 flagged the sentinel exit as a discrepancy and guessed it was my misreading.
Chasing it settled one half and hardened the other.

SETTLED, and it was a real port defect. The 0xC1 handler has three non-writing
outcomes and the port collapsed them to two:

    owner inactive        je  0x6C73  @0x6BD3  -> vm_branch
    destination occupied  jne 0x6C73  @0x6C5B  -> vm_branch
    scan found nothing    je  0x6C7C  @0x6C20  -> NO vm_branch

`vm_branch` is reached ONLY through `0x6C73`. `0x6C7C` is `pop di / ret`, verified
from the raw bytes (`5f c3`), and the sentinel jumps over `0x6C73` entirely
(`74 5a`, disp 90 from `0x6C22`). So a kind-0x10 owner whose source list rejects
every entry must fall through, not branch.

The port returned `Some(None)` for all three, and `step()` maps that to
`branch()`. Fixed by returning `None` for the scan case — `step()`'s `None => {}`
arm is exactly the third outcome, neither writing nor branching.

A TEST WAS PINNING THE WRONG BEHAVIOUR: `c1_set_kind10_target_writes_the_
selector13_destination` asserted `m.pc == 0x99` with the comment "no source entry
passes -> branch". It has been corrected to assert the opposite. Worth stating
plainly because that test passed for as long as it existed and the ledger counted
its subject as settled — a green assertion is only as good as the decode behind
it, and this one had been written from the branch path without noticing the
sentinel took a different exit.

NOT SETTLED, and #294's speculation is withdrawn rather than upheld. I suggested
the sentinel path might also leave two words on the stack, since `0x6C7C` skips
the `pop si / pop ds` that both other exits perform. That reading does not
survive: the QUERY path jumps straight to `0x6C7C` from two more sites
(`0x6BC2`, `0x6BCB`), and those are the ordinary matched/mismatched outcomes that
run constantly. A stack imbalance there would break the game immediately, so the
pattern — every no-branch exit direct to `0x6C7C`, every branch exit via
`0x6C73` — is systematic and intentional, and my pairing of the `push ds`/`push
si` at `0x6B71`/`0x6B72` must be wrong in a way I could not locate statically.

Recorded as an OPEN QUESTION with the shape of its answer: single-step the
handler under the interpreter oracle and watch SP across the three exits. That is
what the oracle is for — verification of a decode I cannot close by reading.

Citations: 583 verified, 0 wrong. 707 tests across all binaries, 0 failures.

## #296 — the stack question, settled by EXECUTION: #294 was right, #295's withdrawal was wrong

#294 read the 0xC1 handler's direct-to-`0x6C7C` exits as leaving two words on the
stack. #295 withdrew that on an argument: the query path reaches `0x6C7C` from
`0x6BC2` on its ordinary outcome, which runs constantly, so an imbalance would
break the game at once — therefore my push/pop pairing must be wrong.

The argument was reasonable and it was wrong. `re/tools/probe_c1_stack.py` runs
the handler in Unicorn from its real entry and records SP at each exit. Driven to
`0x6BC2`:

    0x6BC2  SP = -6   di, ds, si all pushed
    0x6C7C  SP = -6   `pop di` reached with the pop si / pop ds SKIPPED
    0x6C7D  SP = -4   `ret` takes SI's value as the return offset
    FAULT             UC_ERR_EXCEPTION

The balanced exits confirm the same probe is measuring correctly: via `0x6C73` or
`0x6C7A`, SP returns to exactly its entry value and the routine rets cleanly.

So the imbalance is REAL and structural — three pushes, one pop on that path. No
calling convention fixes it, because the handler's own `0x6B71`/`0x6B72` pushes
are the unmatched pair.

WHAT THIS DOES NOT ESTABLISH, and I am not going to assert it: that the shipped
game faults here. It establishes that the path faults WHEN ENTERED BY A PLAIN
NEAR CALL WITH ONLY A RETURN ADDRESS BELOW. Two possibilities remain and the
probe cannot separate them: the path may be unreachable with real data, or the
handler may be entered some other way. `0x6BC2` needs `es:[bp] == 0xC1` (the
record already typed by a previous C1 SET), `ax == es:[bp+2]`, and no `0xA1`
prefix — i.e. a C1 query that PASSES. A C1 query against an empty record gives
`cx == 0` and exits via `0x6BC5` -> `0x6C73`, which is balanced. So the fault
path is exactly "a passing, non-inverted C1 query", which may simply never occur
in the shipped scripts.

THREE BUGS IN MY OWN PROBE before it produced anything, each worth noting because
each returned a confident wrong answer rather than an error:
  * the COD stream was written at LINEAR 0x4000 while `lodsw` reads `DS:si` —
    the handler read EXE bytes as its operands;
  * `gs:[0x672c]` pointed at zeros, so the record-lookup helper (`0x6034`)
    scanned forever and the run died on the instruction limit;
  * `ES` was left zero, but it comes from `les di,gs:[0x6724]` @`0x6B4D`, so
    `es:[bp]` read EXE bytes and gave `cx = 0xC700` instead of `0xC1` — the
    handler took the MISMATCH path and reported a balanced exit.
That third one is the dangerous shape: the probe ran, faulted nothing, printed a
clean balanced trace, and would have "confirmed" #295's withdrawal. It was only
caught by tracing the compare operands instead of trusting the exit it reached.

Recorded, not acted on. Nothing in the port changes: #295's behavioural fix (the
sentinel does not branch) stands on its own reading of `0x6C20`/`0x6C7C` and is
unaffected by the stack question.

## #297 — measuring #296's reachability: the query fault path is not in the shipped data

#296 proved the `0x6BC2` exit unbalanced and left the honest question open:
does that path execute? It needs QUERY mode, `dl == 0` (no `0xA1` after the `0xC1`
opcode byte), and a matching record. The first two are static properties of the
bytecode, so they can be COUNTED rather than argued about.

Walking every shipped COD with the game's OWN token lengths (`token_len_at` =
`OPCODE_DESC` plus the mode rules), not by scanning for the byte `0xC1`, which
appears constantly inside operands:

    SCRIPT1  0     SCRIPT2  3     SCRIPT3  9     SCRIPT4  5     SCRIPT5  6
    23 C1 tokens total; 0 inverted; 0 reached in QUERY mode by a linear walk

Two results. First, NO shipped C1 token is followed by `0xA1`, so `dl == 0`
everywhere — the inversion half of the condition is satisfied at every site, and
that is the half I would have guessed was rare. Second, none is reached in query
mode, and `0x6BC2` lies beyond `test byte gs:[0x67ad],1 / je 0x6BCE` @`0x6B73` —
the QUERY side. So the exit #296 showed faulting is, on this evidence, not on any
executed path: C1 appears in shipped scripts only as a SET.

That is what reconciles #296's fault with a game that plainly works, and it does
so without needing my push/pop pairing to have been wrong. It was not wrong; the
path is simply not taken.

WHAT IS STILL OPEN, and it is narrower but real: the SCAN SENTINEL at `0x6C20`
reaches the same `0x6C7C` and lies on the SET side, past `cmp ax,0x10 / jne`
@`0x6C07`. So a C1 SET on a kind-`0x10` owner whose source list yields no passing
entry would take the unbalanced exit. Whether that combination occurs depends on
runtime record contents, which a static token walk cannot decide.

THE MODE TRACKING IS AN APPROXIMATION and the test says so: `0xA0` sets query and
`0xA1` clears it (`0x6559`/`0x6572`), but a linear walk cannot know the mode at a
site reached by a branch. The counts are pinned as assertions so a data or
walker change cannot move them silently, with the approximation stated at the
assertion rather than buried in a comment.

603 lib tests, 0 failures.

## #298 — reviewing ASM? rows one at a time, and the first two disagreed

The ledger's `ASM?` status is a HEURISTIC: an address appears somewhere in the
item's doc. `ASM` means a human checked the row against the disassembly. With 299
provisional rows the temptation is to promote them in bulk on the grounds that
`check_cited_instructions.py` already verifies 586 citations with none wrong.

That reasoning does not hold, and the first two rows I reviewed show why — the
guard checks that the INSTRUCTION AT AN ADDRESS matches the mnemonic beside it.
It cannot check that the address is the RIGHT ONE for the claim.

`text_selector_active_line_id` VERIFIED. Its doc claims `cbw; mov gs:[0x1FAB],ax`
then `mov ax,[0x1FAB]; add ax,9; mov [0x6788],ax`. All of it holds: `cbw` @`0x668E`
(one byte `98`, so `cbw` — capstone renders it `cwde` in 16-bit mode, the known
artifact, told apart by LENGTH), `mov gs:[0x1fab],ax` @`0x668F`, then `0x11F2`,
`0x11F5`, `0x11F8` in `dlg_line_activate`. The port sign-extends and adds 9.
Settled.

`reveal_frames_per_char` DID NOT. Its doc cited `0x94BA` for
`gs:[0xB31] = step >> 2`. `0x94BA` is `dlg_reveal_complete_hold` — a different
routine, writing a different cell, by a different rule
(`gs:0x0B35 = gs:0x0ACA * 4`, the end-of-reveal hold). The arithmetic in the port
was right; the address was wrong, and the doc's own hedge ("see REVERSE.md
@0x94BA") was pointing at a REGION rather than an instruction, which is what a
correct-looking citation degrades into.

The real writer is fourteen bytes earlier:

    0x94AB  mov ax, word ptr [0xaca]
    0x94AE  shr ax, 2
    0x94B1  mov word ptr [0xb31], ax

Corrected, and the `.max(1)` justified from the loop rather than left as a fudge:
`mov ax,[0xb31] / or ax,ax / jne` @`0x94A4` skips the reveal while the countdown
is nonzero, so a stored zero still costs the frame that runs the check.

ONE IN TWO of the rows I checked had a wrong address behind a right answer. That
is the rate that makes bulk promotion indefensible: it would have marked this row
"checked against the disassembly" while its citation pointed at the wrong
routine, and the guard would have kept reporting 0 wrong throughout.

Citations: 586 verified (from 583), 0 wrong. 603 lib tests, 0 failures.

## #299 — a citation in a FIFTH address space, and why the guard was blind to it

Continuing #298's row-by-row review. Two more rows, two more outcomes.

`record_end_hold_ticks` VERIFIED exactly. Doc: "`0x7378..0x738C`:
`b35 = gs:[0x27CF] * (gs:[0x0ACA] >> 1) + 6; gs:[0x67BB] = 1`". The disassembly:

    0x7378  mov ax, word ptr gs:[0x27cf]
    0x737C  mov dx, word ptr gs:[0xaca]
    0x7381  shr dx, 1
    0x7383  mul dx
    0x7385  add ax, 6
    0x7388  mov word ptr gs:[0xb35], ax
    0x738C  mov byte ptr gs:[0x67bb], 1

and the port is `units.wrapping_mul(step >> 1).wrapping_add(6)`. Settled ASM.

`location_var_offset` cited "SCRIPT2: 0x0F4E", and disassembling BLOODPRG.EXE
there produces `sub ax,0x6652 / xor ax,ax / stosw ...` — plausible-looking and
completely unrelated, because 0x0F4E IS NOT AN EXECUTABLE OFFSET. It is a record
offset in the SCRIPT2 DATA, a fifth address space beside the file offsets, `DS:`,
`XDB:` and `DRV:` that `re/CLAUDE.md` documents.

That is why `check_cited_instructions.py` reports 0 wrong on it: the doc line has
no mnemonic, so it lands in the 84 "non-mnemonic lines skipped". The guard is not
failing here, it is CORRECTLY DECLINING — but the effect is a whole class of
citation that no tool checks, and the row still counted as provisional evidence.

Verified the only way this space can be: by RUNNING the function on the shipped
SCRIPT2 and asserting it finds `0x0F4E`. It does. Settled TESTED, not ASM,
because the evidence is a data run rather than a disassembly.

`re/CLAUDE.md` now documents `SCRIPT<N>:0xNNNN` with the rule that a bare
`0x0F4E` reads as a file offset, which is exactly how this one went unchecked.

RUNNING TALLY of the individual review: four rows examined, two verified as
written, one had the wrong address behind a right answer (#298), one was in an
address space the tooling cannot see. Half the provisional rows I have looked at
needed a correction — none of them wrong in their CONCLUSION, all of them wrong
or uncheckable in their EVIDENCE. That is the distinction the ledger's `?` is
for, and it is why these get promoted one at a time.

Citations: 586 verified, 0 wrong. 604 lib tests, 0 failures.

## #300 — listing what the guard SKIPS, and finding a real citation inside the count

#299 ended on an uncomfortable fact: `check_cited_instructions.py` reports "84
non-mnemonic lines skipped" and that number had never been looked at. A skipped
line is not automatically harmless — `location_var_offset`'s SCRIPT-space
citation was in there, unchecked while its row still read as evidence.

Added `--skipped`, which lists them grouped by the word that was claimed (a word
recurring across many lines is ordinary prose around an address; a one-off is
worth a look). Most of the 84 are exactly what they should be: `0x83CC` followed
by "the", `0x86F1` followed by "nav", register names like `al`/`dl` in sentences
such as "sets al".

ONE WAS NOT. `src/ship3d.rs:2182: 0x9A34 claimed as 'lodsd'` — `lodsd` IS an x86
mnemonic. The set listed `lodsb`, `lodsw` and `stosd` but not `lodsd`, so a real
instruction citation in the projector had been going unchecked since it was
written. Added, and it VERIFIES: 587 checked, still 0 wrong.

THIS IS THE SECOND TIME. #249 added `movsx`/`movzx` after the same audit found
them missing. The failure mode is worth naming because it is silent by
construction: a missing mnemonic does not fail, it stops checking — and the
headline "0 wrong" gets slightly less true each time without anything moving.
The string family in particular needs all its widths or the omission is invisible,
so `movsd`, `scasb/w/d` and `cmpsb/w/d` went in alongside.

The guard's summary line was measuring its own coverage and reporting it as
confidence. It still says "0 wrong", but now the skipped set can be READ rather
than trusted, which is the only thing that makes the 0 meaningful.

Citations: 587 verified (from 586), 83 skipped (from 84), 0 wrong.

## #301 — a doc that cited its guard clause and not its formula

`dlg_line_asset_id_ds_offset` claims the offset is `0x1FB5 + line_id*4 + 2`, and
its only citation was for the REJECTION: "`or ax,ax; js` at `0x9D20`". That check
verifies (`or ax,ax` @`0x9D20`, `js` @`0x9D22`), and the guard was happy — but the
formula, which is the entire content of the function, had no address behind it.
The `?` was right and the reason was not the one I expected.

The arithmetic is at `0x9D65`..`0x9D6E` and matches exactly:

    0x9D65  mov bx, ax       the line id
    0x9D67  shl bx, 2        *4 -- four bytes per entry
    0x9D6A  add bx, 0x1fb5   the table base
    0x9D6E  mov si, [bx+2]   +2 -- the ASSET ID is the entry's second word

WORTH RECORDING HOW IT WAS FOUND, because the first attempt produced garbage.
`find_imm.py` located `add bx,0x1fb5` @`0x9D6A`, so I disassembled from `0x9D60`
— and got `add [bx],cx`, `test [bp+si-0x7500],ax`, `fadd st(1)`, `jcxz`. All
phantoms: an FPU instruction in the middle of a dialogue dispatcher is the tell.
Decoding from `0x9D4D`, an actual branch target, gives clean code and the four
instructions above. Same lesson `refs_in_routine.py` was built for (#234) —
x86 self-synchronises, so a decode has to START somewhere known, and "near the
address I want" is not that.

A CITATION THAT VERIFIES CAN STILL BE THE WRONG CITATION — not wrong in the #298
sense (pointing at another routine) but MISPLACED: attached to a guard clause
while the claim it is meant to support sits twenty bytes away. The guard cannot
detect this, since the instruction it checks really is there and really is what
the doc says. Only reading the doc against the function can.

Citations: 593 verified (from 587), 0 wrong. 604 lib tests, 0 failures.
Row tally so far: five reviewed, two correct as written, one wrong address, one
uncheckable address space, one citing the wrong part of its own routine.

## #302 — the shape was decoded, the POLICY was invented, and one citation covered both

`promote_queued_presentation` scans every record for a `{0xC3, related, 1}`
triple and starts the first one it finds. Its citation: "the pending-slot
protocol around 0x5C64".

The triple is real. The 0xC3 handler writes exactly it:

    0x6F4B  mov word ptr es:[bp], 0xc3
    0x6F51  mov word ptr es:[bp+2], bx     the related object
    0x6F55  mov word ptr es:[bp+4], 1      1, where C4..C8 write 0

The scan is not. `0x5C64` is `presentation_start_travel_arm` — straight-line
state setting (`[0x24F3]=9`, `[0x67F8]=0`, `[0x27D7]=0`, ...) that consumes a
pending C4 through `[0x675E]`. It scans nothing, and it is about C4, not C3. The
citation was a REGION POINTER — "around 0x5C64" — the same shape as #298's "see
REVERSE.md @0x94BA", and the hedging word is the tell in both.

WHAT MAKES THIS ONE DIFFERENT from the four before it: those had right answers
behind wrong or unverifiable addresses. Here the answer is PARTLY INVENTED. The
record shape is faithful; the promotion POLICY — that the scan is linear, that
first match wins, that record order decides which queued presentation starts —
has no binary behind it at all. A single citation covered a decoded fact and a
port construction sitting in the same function, and read as if it covered both.

Doc now says so explicitly, and the row STAYS PROVISIONAL. Settling it would
record "checked against the disassembly" for a policy no disassembly supports.
Finding the engine's own scan of C3 records is the task.

Citations: 596 verified (from 593), 0 wrong. 604 lib tests, 0 failures.
Row tally: six reviewed, two correct as written, four needing correction —
one wrong address, one uncheckable address space, one citing its guard clause
instead of its formula, one covering an invented policy.

## #303 — found the C3 promoter, and the invented policy was wrong on the merits

#302 left a task: find the engine's own scan of C3 records. It exists, and the
port's first-match policy was not merely unsourced — it was WRONG.

`find_imm.py` on `0xc3` gives ten confirmed instructions. Two are the handler's
own (`0x6F21`, `0x6F4B`), four are `mov [0x6768],0xc3` in the presentation area,
four are phantoms inside the dispatch table at `0x142F9`. The tenth is
`0x05D37: cmp ax, 0xc3` — a READER, and `labels.csv` already names it
`c3_promoter_branch`. It sits in the very region the old doc waved at, which is
why "around 0x5C64" felt plausible enough to write and was still not a citation.

    0x5D37  cmp ax, 0xc3           the record is typed C3
    0x5D3C  mov bx, ds:[bp+2]      its related word
    0x5D40  cmp bx, gs:[0x674e]    ...must be `blood`
    0x5D45  jne                    otherwise no takeover

A queued presentation is promoted ONLY when its related object is `blood`. The
port promoted the first typed-C3 record it found, whatever the related word.
That is a behavioural difference, not a documentation gap: any C3 record queued
against another object would have taken over a presentation it should never have
touched.

FIXED, with the built-in it needs. `VmMachine` resolved `arche`, `orxx` and `Ark`
by name but not `blood`, so there was nothing to compare against; `blood_offset`
now comes from the same DEB name scan the game runs at `0x5486`. The gate is
skipped when no DEB is loaded, because then there is no `blood` and rejecting
everything would be a worse guess than the old behaviour.

Pinned from three sides: related == blood promotes, related != blood does not,
and no-DEB keeps the previous shape.

STILL UNDECODED, narrowed: the ITERATION. Which records the engine walks, and in
what order, is not established, so first-match AMONG blood-related records
remains a port choice — but a much smaller one than #302 had to record.

Citations: 605 verified (from 596), 0 wrong. 706 tests across all binaries, 0
failures.

## #304 — decoded the presentation scan, implemented it, and reverted it

#303 narrowed the open question to the ITERATION. It is decodable, and the answer
is that the port's shape is wrong — but the fix does not follow from what is
decoded so far, and trying it proved that.

THE SCAN, found by walking back from `c3_promoter_branch` through
`record_c1_ship3d_action` (`0x5B38`, whose label says "from the presentation
scan") to its near callers:

    0x582F  mov si, es:[di+0x10]        the entry's OBJECT offset
    0x5833  test byte [si+2], 1         the object must be ACTIVE
    0x583B  mov bx, [si]                its kind
    0x583D  mov ax, 0x13 / call 0x6023  selector 0x13 for that kind
    0x5845  mov bp, ax                  bp = obj + field
    0x5A64  add di, 0x14                next directory entry...
    0x5A6B  cmp ax, 1 / je              ...while its +0x12 kind is 1

So the engine walks the `gs:0x672c` DEB DIRECTORY in directory order — the same
walk as `build_nav_source_list` and `active_object_list_build` — and examines
each active object's SELECTOR-0x13 slot. The port scans raw record addresses,
which visits slots no object owns and orders them by address.

IMPLEMENTED IT, AND IT BROKE SIX TESTS, including
`directed_drive_plays_the_story_to_fin_hnm` and three interception tests. The
queued interception that should promote at record 1788 stopped being found.
Reverted.

WHY, and this is the part I got wrong: I assumed the `bp` read at
`mov ax,[bp+4]` @`0x5A51` is still the `bp` computed at `0x5845`. Between them
sits roughly 500 bytes of ladder dispatching on the OBJECT kind (`cmp bx,2`
@`0x5847` and onwards), and whether `bp` survives it unchanged is NOT
established. Six failing tests are the evidence that it does not — or that some
other precondition on the path is missing.

THE REVERT IS THE POINT. A plausible-looking replacement that breaks the story
drive is worse than a known approximation, and the tests caught exactly the case
a static reading could not. The linear scan stays, documented as the wrong shape,
with the decoded scan recorded beside it and the specific unknown named: trace
`bp` from `0x5845` to `0x5A51` through the object-kind ladder.

What survives from this entry: the scan's structure is now cited in the doc
rather than guessed at, and the next attempt starts from a named question instead
of from scratch.

Citations: 612 verified (from 605), 0 wrong. 605 lib tests, 0 failures.

## #305 — #304's revert was right, its REASON was wrong, and the scan works

#304 implemented the decoded presentation scan, saw six tests fail, reverted, and
blamed `bp` not surviving the object-kind ladder between `0x5845` and `0x5A51`.

That diagnosis was wrong, and checking it took one query. The only writes to BP
in that entire 520-byte span are:

    0x5845  mov bp, ax     the selector-0x13 slot
    0x5984  push bp
    0x598E  xor bp, bp     <- bracketed
    0x5995  pop bp         restored

`bp` reaches the C3 arm intact. The real cause was mundane and invisible from the
disassembly: the failing tests load a COD (and VAR) but never call
`load_deb_objects`, so `self.directory` is EMPTY. A faithful directory walk had
nothing to walk, while the old linear scan over raw record addresses did not care.

CHECKED AGAINST REAL DATA before re-applying, rather than assuming twice:
`decoded_presentation_scan_over_the_real_directory` loads SCRIPT2 WITH its DEB
and finds 341 directory entries, 118 active selector-0x13 slots — and record
1788, the one the linear scan promoted, IS among them. So the decoded scan
reaches the same record by the engine's own route.

ADOPTED, with the directory path taken whenever a DEB is present and the old
linear scan kept only for DEB-less harnesses. That fallback is documented as
modelling a state the shipped game never reaches (the DEB loads at startup); it
exists so test fixtures that skip it do not silently promote nothing.

THE LESSON IS ABOUT MY OWN DIAGNOSIS, not the code. #304 had the right instinct —
do not ship a change that breaks the story drive — and then attached a confident
explanation to it that was never checked. A revert with a wrong reason is worse
than a revert with an open question, because the wrong reason gets recorded as
decode knowledge and the next attempt starts from a false constraint. The fix
cost one `capstone` pass over a byte range.

Citations: 605 verified, 0 wrong. 707 tests across all binaries, 0 failures.

## #306 — I settled a row I had not checked, in the same command as one I had

Having closed the C3 chain, `promote_queued_presentation` was genuinely ready to
settle. I passed `start_actor_presentation` to the same `audit_settle.py`
invocation because it was adjacent in the file and its origin column looked
similar.

It was not checked. Its citation is `0x5816`, which turns out to be
`presentation_scan` — the ENTRY of the very scan #305 decoded, so the address is
right and the guard is satisfied. But the routine's kind-1 PRESENTATION START
does considerably more than the port's five lines:

    sets   0x67AC = 1        active
    sets   0x67B7            start-lock
    ors    0x2793 |= 4       busy
    ors    record+3 |= 0x80
    clears 0x6782, 0x6784, 0x6776, 0x67F8, 0x67BA..0x67BC, 0x679A

The port writes `{0xC4, related}` and sets three of its own flags. Whether the
seven dialogue cells are cleared elsewhere in the port's lifecycle is not
established, and I had established nothing when I settled it.

PUT BACK to `ASM?` (via `audit_settle.py UNVERIFIED` plus an inventory pass,
which the heuristic then restores to provisional), and the doc now lists what is
missing so the next reader inherits the QUESTION rather than my assumption.

This is #298's finding turned on its author. Eighteen entries of arguing that
provisional rows must be promoted one at a time, and the failure mode arrived
anyway — not as a deliberate shortcut but as a second argument on a command line.
Bulk settlement does not require intent; it requires only that checking one row
makes the next one feel checked.

The count moved by one instead of two. That is the correct number.

Citations: 605 verified, 0 wrong. 606 lib tests, 0 failures.

## #307 — `gs:0x2793` holds TWO flags and the port set the wrong one at presentation start

#306 left the question of whether the port covers the presentation start's other
effects. Checking them turned up something sharper than a missing write.

`find_imm.py` on `0x2793` separates cleanly into two populations:

    bit 0   TESTED ONLY, seven sites:  0x594A 0x5CE5 0x5D4C 0x5F93 0x6A70
                                       0x6E9F 0x7652   (`test ...,1`)
    bit 2   SET/CLEARED, five sites:   0x1B7B 0x593A (`or ...,4`)
                                       0x1D5B 0x59BF 0x5E99 (`and ...,0xfb`)

Two independent flags sharing a byte. Bit 0 is never OR'd anywhere — the only
writer that raises it is `mov word ptr [0x2793], 9` @`0xB505`, the NAVIGATION
FINAL RESET, which the port already models as `SHIP_3D_FINAL_RESET_HUD_FLAGS = 9`
(bits 0 and 3). Bit 2 is the presentation flag: `or byte ptr gs:[0x2793], 4`
@`0x593A` sits inside the kind-1 PRESENTATION START of `presentation_scan`
(`0x5816`), and the teardown clears it at `0x59BF`/`0x5E99`.

THE PORT HAS ONE FLAG FOR BOTH. `presentation_busy` is documented as
"`gs:0x2793` bit0 — 0xCE branches when CLEAR", and `0x3388` reads
`state_u8(state, 0x2793) & 1`. That part is right: the 0xCE opcode does test bit
0. But `start_actor_presentation` SETS `presentation_busy`, i.e. sets bit 0 —
while the game's presentation start sets bit 2 and leaves bit 0 alone.

So at presentation start the port raises a flag the engine does not, and the
opcode that reads it (0xCE) sees a state the game would not produce. The two bits
were conflated because they live in one byte and one of them was decoded first.

NOT CHANGED YET, and deliberately. Splitting the field means `0xCE` stops seeing
presentation starts, which is a behavioural change reaching the story drive —
exactly the shape #304 shipped and had to revert. The evidence above is strong
enough to record and not yet strong enough to act on: what is missing is the
reader of bit 2, i.e. what the engine does differently while a presentation is
up. That reader is the next task, and it decides whether the port needs a second
field or a rename.

Citations: 605 verified, 0 wrong. 606 lib tests, 0 failures.

## #308 — correcting #307: I reasoned from `tail -12` of a 66-line list

#307 claimed `gs:0x2793` holds two flags, that bit 0 "is never OR'd anywhere —
the only writer that raises it is `mov word [0x2793], 9` @`0xB505`", and that bit
2 is set and cleared. I ran `find_imm.py 2793 | tail -12`, read twelve lines, and
wrote a structural claim about the byte. The tool's own first line said
"66 confirmed instruction(s)".

THE FULL AGGREGATION over all 66:

    test FLAG, 1      x7     bit 0 read
    or   FLAG, 4      x4     bit 2 set
    and  FLAG, 0xfb   x3+1   bit 2 cleared
    mov  FLAG, 1      x2     bit 0 RAISED, other bits cleared
    test FLAG, 0xe    x1     bits 1|2|3 read TOGETHER
    test FLAG, 8      x1     bit 3 read
    mov  FLAG, 0      x1     all cleared
                             (plus `mov word [0x2793], 9` @0xB505)

WHAT WAS WRONG. Bit 0 has two more writers I never saw: `mov word [0x2793], 1` at
`0x0FC8` and at `0x1A5E`, and `0x1A5E` is `dlg_clear_a`, the DIALOGUE CLEAR. And
bit 2 IS read after all — `test byte [0x2793], 0xe` @`0x1095` tests bits 1, 2 and
3 as a group, which is why a search for `test ...,4` found nothing and I concluded
there was no reader.

WHAT SURVIVES. The core of #307 stands: bit 0 and bit 2 are distinct, the
presentation start ORs bit 2 (`0x593A`), and the port's `start_actor_presentation`
sets bit 0 instead. That is still a defect.

WHAT GOT WORSE, usefully. `dlg_clear_a` setting the word to exactly 1 means bit 0
is RAISED while dialogue state is being CLEARED — which sits badly with the port's
name for it, "presentation-busy". A flag the dialogue teardown turns ON is more
plausibly an idle/ready bit than a busy one, and `0xCE` branching when it is clear
reads differently under that reading. The port's naming is now suspect, not just
its write site.

THE METHOD ERROR IS THE POINT, and it is the same one as #296's probe: I took an
instrument's output, looked at part of it, and reasoned as though I had seen the
whole. `tail -12` on a tool that prints a COUNT ON ITS FIRST LINE is a
self-inflicted blind spot — the number was right there and I piped it away.
#300 was about a guard that hid a citation inside a summary count; this is the
same failure committed by hand, one entry later.

Nothing in the port changed. #307's proposed fix is still deferred, now with a
better-posed question: decode what bits 1, 2 and 3 mean as the group `0x1095`
tests them, and what bit 0 means given `dlg_clear_a` raises it.

Citations: 605 verified, 0 wrong. 606 lib tests, 0 failures.

## #309 — the tool was truncating silently, which is what #307 and #308 were both reading

#308 blamed itself for piping a 66-hit result through `tail -12`. That was true
and it was not the whole cause. `find_imm.py` prints `real[:limit]` with `limit`
defaulting to 20 AND SAYS NOTHING about the other 46. So the honest reading of
that session is: the tool showed 20 of 66 without a word, and I then showed
myself 12 of those 20.

FIXED TWO WAYS.

  * It now says `... 46 more NOT SHOWN (--max 66 for all)` when it truncates.
    Silence about dropped rows is the same defect #300 found in the citation
    guard's skip count, in a second tool.
  * It now leads with a `--- by operation ---` aggregation over ALL hits, so the
    SHAPE of a result is visible in a few lines and a truncated read cannot hide
    a population. This is the part that would have prevented both entries.

THE FLAG BYTE, read properly for the first time:

    bit 0   test x8
    bit 2   or x19, and x9, test x1        <- `test FLAG, 4` EXISTS
    bit 3   test x6, or x2, xor x1
    bit 4   test 0x10 x2
    bit 5   test 0x20 x1
    bit 6   test 0x40 x2
            composite masks 0xe, 0xc, 0x50, 0x90
            whole-word: mov ax,FLAG x2, mov FLAG,ax x1

`gs:0x2793` is a MULTI-BIT UI/STATE WORD with at least six live bits. #307 called
it "TWO flags"; #308 corrected two of #307's details and kept the two-flag
framing; both were wrong about the structure, and both were wrong because of the
same invisible cap. #308 also stated that no `test ...,4` exists — it does, once.

The port's own constant already has the right name for this: `VM_UI_FLAGS`.

WHAT STILL SURVIVES, third time of asking: the presentation start ORs BIT 2
(`or byte gs:[0x2793], 4` @`0x593A`), and `start_actor_presentation` sets the
port's bit-0 model instead. That defect has outlived two wrong explanations of
its surroundings, which is a fair reason to trust it and no reason at all to
trust the rest of what I said about the byte.

THE PATTERN ACROSS #296, #300, #308 AND THIS: every one is an instrument
reporting a summary that conceals its own incompleteness — a probe reaching the
wrong branch and printing a clean trace, a guard counting skipped lines it never
listed, a search capping its output in silence. The finding is not "check your
work"; it is that a tool which summarises MUST disclose what it left out, or it
will be read as complete every time.

606 lib tests, 0 failures. 605 citations verified, 0 wrong.

## #310 — sweeping the toolset for the same silent truncation

#309 fixed `find_imm.py` for hiding 46 of 66 hits. The obvious next question is
whether it was the only one, so I swept every tool for the shape: a slice like
`[:limit]`, `[:40]`, `most_common(N)` on something the caller reads as a result
set.

FOURTEEN candidates, and most were already honest — which is worth saying,
because the interesting finding here is how narrow the defect was:

  * `search_bytes.py` prints `... N more (raise --limit)`. Fine.
  * `audit_suggest.py` prints `... and N more (--all to list)`. Fine.
  * the rest are SAMPLES inside a line (`addrs[:4]`, `labels[:6]`), where the
    surrounding text already carries the total. Not the same defect.

TWO WERE NOT.

`analyze_handler.py` printed `data offsets: ...` capped at 40 with NO COUNT
ANYWHERE. A handler touching ninety data cells rendered identically to one
touching forty. Now prints the total and says how many it withheld.

`whatis.py` printed `labels.csv: 12 row(s)` and then listed six. The count is
there, which feels like disclosure and is not: a reader who sees six lines under
a heading takes the six as the answer, and the number two lines up does not
correct that. Now says `... 6 more row(s) not shown`.

THE DISTINCTION THAT MATTERS, and it is the one I got wrong in #308: printing a
TOTAL is not the same as disclosing a TRUNCATION. `find_imm.py` printed "66
confirmed instruction(s)" on its first line and I still read twenty of them as
the population. The count answers a question nobody asked; the missing-rows
notice answers the one the reader is actually acting on.

Four entries (#296, #300, #308/#309, this) have now turned on an instrument
whose output was complete-looking and partial. The rule I would give a future
reader: a tool that shows a subset must say so ON THE SAME LINES it shows them,
not in a summary above or below.

606 lib tests, 0 failures.

## #311 — #307's "wrong bit" is really a CONFLATION, and the tests prove it cannot be split yet

#307 claimed the port sets bit 0 of `gs:0x2793` at presentation start where the
game ORs bit 2. #308 and #309 corrected the surroundings twice while that claim
survived. It is now decodable in full, and it is not a wrong-bit typo.

BOTH BITS DECODED, via the readers the fixed `find_imm.py` finally showed:

  bit 2 — `test byte [0x2793], 0xe / jne` @`0x1095`, the entry of
  `main_loop_busy_gate`: if ANY of bits 1|2|3 is set, SKIP the pending-profile
  dispatch. It then ORs ten subsystem-active flags. So bits 1-3 mean "something
  is running, defer loading a new scene", and the presentation start's
  `or ...,4` @`0x593A` is exactly that. Also read at
  `test word [0x2793], 4` @`0x975A`.

  bit 0 — tested by `0xCE` itself: `test byte gs:[0x2793], 1 / jne` @`0x6494`,
  branch when CLEAR. Its writers are `mov word [0x2793], 1` at `0x0FC8` (a
  SCENE/PROFILE LOAD — it also sets `[0x27D9]`, then far-calls and loads a
  filename) and at `0x1A5E` (`dlg_clear_a`), plus `mov word [0x2793], 9`
  @`0xB505`. Note `mov word ...,1` sets bit 0 while CLEARING bit 2: the two are
  near-exclusive, not two names for one state.

So bit 0 is closer to "a scene/profile is loaded" and bit 2 to "a presentation is
running". The port's single `presentation_busy` field carries BOTH: it is set at
presentation start (bit-2 semantics), read by the `0xCE` opcode (bit-0
semantics), used as the promoter's already-busy guard, and cleared by the
teardown.

TESTED THE SPLIT RATHER THAN ASSERTING IT. Removing just the
`presentation_busy = true` from `start_actor_presentation` fails SIX tests —
`directed_drive_plays_the_story_to_fin_hnm`,
`faithful_vm_reproduces_the_script1_tutorial_flow`, both interception runs, the
frame loop, and the Scruter chain. The port's story flow depends on the amalgam.
Restored.

WHAT THAT MEANS, stated carefully: the port is not simply setting the wrong bit,
it is representing two engine flags with one field, and the shipped behaviour
currently rides on that. Splitting them requires modelling bit 2's READER — the
main-loop profile gate at `0x1095`, which the port does not have — because
without it a correctly-split bit 2 would be written and never read, and bit 0
would stop being set at all.

NOT CHANGED. The next step is not "fix the flag", it is "port the main-loop busy
gate", and only then does the split have somewhere to land. Recorded in the doc
beside the field.

606 lib tests, 0 failures. 605 citations verified, 0 wrong.

## #312 — decoded the main-loop busy gate, and did NOT ship a one-flag version of it

#311 ended by naming the next step: port the main-loop profile gate at `0x1095`,
since the `0x2793` bit split has nowhere to land without it. It decodes cleanly —
bits 1|2|3 of `0x2793`, then ten subsystem flags OR'd together, and only if all
are clear does `[0x6780]`'s pending profile get selected and the slot cleared to
`0xFFFF`.

The port already has the request half: `0xD2` posts `pending_profile` and emits
`VmEvent::ProfileRequest`. What it lacks is the DEFERRAL — `main.rs` (three
sites), `bin/playthrough.rs` and `engine.rs` all act on the request immediately,
so a scene load requested mid-presentation can swap resources under a running
scene instead of waiting.

I DID NOT IMPLEMENT THE PREDICATE, and the reason is the interesting part.
`VmMachine` models exactly ONE of the ten flags (`0x67AC`, as
`presentation_active`). `0x27DA` does not appear anywhere in the tree. The rest
exist as constants in other modules, not as VM state.

A `may_dispatch_pending_profile()` written over one flag would defer on
presentation and nothing else. It would pass any test I wrote for it, because the
tests I could write would exercise presentations — the one input it models. That
is the #302 failure shape exactly: a decoded fact and an invented policy sharing
one function, with a citation over both. Better to leave the gap visible than to
close it with something that reads as decoded and behaves as a guess.

Recorded in docs/port-validation.md as an APPROX row with the routine that must
replace it and the specific missing state, per the project's rule that a
capture-or-approximation may stand only when the binary routine that replaces it
is named.

606 lib tests, 0 failures. 605 citations verified, 0 wrong.

## #313 — subtitle wrapping was REACTIVE where the game is PREDICTIVE (two copies)

Reviewing `assemble_words` (`src/engine.rs`), whose doc claims the 0xA6 text rule
from `0x66CD`..`0x6739`. Most of it verifies exactly: the attaching-punctuation
set is `0x2C , / 0x2E . / 0x3F ? / 0x21 ! / 0x3A :` at `0x6709`..`0x6720`, each
jumping back to the next word WITHOUT emitting a space; otherwise `mov ah,0x20`
stores one; long words are never split.

THE WRAP CONDITION DID NOT.

    0x66FF  mov di,[si] / call 0x67a7   al = strlen(the NEXT word)
    0x6728  inc dl                      dl = line length INCLUDING the space
    0x672A  add al, dl
    0x672C  cmp al, 0x23 / jb           under 35 -> keep going
    0x6730  xor dl,dl / al=0x0D / stosb else newline and reset

The game adds THE NEXT WORD'S LENGTH before comparing, so it breaks BEFORE a word
that would overflow. The port compared the line length alone and broke after one
already had. Different break points on any line where the two disagree — i.e.
visible, on screen, in the subtitles.

TWO COPIES, both wrong: `engine::assemble_words` and `script.rs`'s subtitle
assembly at line 376. #267's lesson again — a duplicated RULE has to be fixed in
every copy, and finding the second one is part of fixing the first.

NO TEST CAUGHT IT, and the reason is instructive. `subtitle_wraps_long_lines`
asserted `line.chars().count() <= 35 + 12` — a bound loose enough to pass under
EITHER rule. Its comment even stated the reactive version as the decoded one. So
the wrong behaviour had a test, a comment, and a citation, and all three agreed
with each other and not with the binary.

Added `subtitle_wrap_breaks_before_the_word_that_would_overflow`, with words
chosen so the rules disagree (line of 22 including the space, next word 13 long:
`22 + 13 = 35` wraps predictively, `22 < 35` does not react). Tightened the old
test's bound to the column itself, and corrected its comment.

Citations: 606 verified, 0 wrong. 607 lib tests, 0 failures.
Row tally: seven reviewed, two correct as written, five needing correction — one
wrong address, one uncheckable address space, one citing its guard clause, one
covering an invented policy, and now one whose doc, test and citation all
described the wrong rule.

## #314 — two choice-box rows that verify EXACTLY, and why that is worth recording too

Seven rows reviewed so far and five needed correction, which risks turning the
review into a search for defects. These two did not need any, and the check is
the same either way.

`choice_box_text_top` / `choice_box_top_y`. The doc claims a default seed of 0
"from `xor bp,bp` at `0x8436`" and that the world/entity box takes the
`[0xADD]&1` branch, seeding the height 10 higher:

    0x8436  xor bp, bp                default seed 0
    0x8438  mov dx, 0x64              default floor 100
    0x843B  test byte [0xadd], 1 / je
    0x8442  mov bp, 0xa               <- the seed, 10
    0x8445  mov dx, 0x37              <- the floor, 55

Both the seed and the kind-10 floor are exactly as documented. `0xADD` is a
one-bit flag (`test ...,1` at three sites, set at `0xB0DC` on the ship/world click
path, cleared at `0x89AC`), which supports the port keying it as
`console_box_kind == 10`.

`choice_box_geometry`. `w = widest.max(floor) + 0x14` then `x0 = anchor - w/2`:

    0x84A1  add dx, 0x14
    0x84AD  shr dx, 1
    0x84AF  sub dx, word ptr [0xac6]
    0x84B3  neg dx                    -> anchor - w/2

and the hit-test band the doc claims, checked rather than assumed after #306:

    0x84EE  cmp ax, bx / jl           reject below x0
    0x84F2  add bx, cx
    0x84F4  cmp ax, bx / jg           reject above x0 + w

Every cited instruction is at its cited address and does what the doc says.
Settled ASM.

WHY THIS ENTRY EXISTS. A review that only records what it breaks produces a
misleading base rate — the running tally is now nine reviewed, four correct as
written, five corrected. Five-in-nine is bad enough to justify one-at-a-time
promotion without me quietly dropping the clean ones from the count.

Citations: 606 verified, 0 wrong. 607 lib tests, 0 failures.

## #315 — "engine-level analogue" was underselling a decode, and hiding a missing case

`present_scene_buffer` described itself as "the engine-level analogue of the
game's `gs:[0x1fa7]` blit base" — the kind of phrase that reads as "we did our own
thing here", so the row sat provisional with a bare DS offset for evidence.

It is not an analogue. `gs:0x1FA7` is read by the blit as a row offset
(`add bx, word ptr gs:[0x1fa7]` @`0xA464` and @`0xAB6E`), and its writers give the
cases directly:

    mov word ptr [0x1fa7], 0x23   @0x18BE, @0xB3FA   the BAND top, 35
    mov word ptr [0x1fa7], 0      @0x1A37, @0x7C45   FULL-SCREEN, 1:1
    mov word ptr [0x1fa7], 0xa    @0x7B5F            a THIRD case, 10

The doc's `0x23` band top and its full-screen 1:1 case are both exactly right,
and now cited. So the function was better than its own description.

AND WORSE, in one specific way the description concealed: THERE IS A THIRD BASE.
`0x7B5F` sets the offset to 10, clears `[0x131C]`, and jumps to `0x7B80`. The port
has no ten-row placement, so any scene the game draws there lands at the band top
or full-screen instead. What selects that path is undecoded.

The word "analogue" was doing real damage: it framed a faithful decode as a port
invention, which both understated the evidence and made the missing case
invisible — nobody audits an analogue for completeness. The row STAYS
PROVISIONAL, now for the honest reason (one unmodelled case) rather than the
vague one.

`load_scene_hnm`, reviewed alongside it, carries NO address at all — its doc only
points at `render_dialogue_frame`. Its `ASM?` came from the ledger picking up a
neighbour's offset. Left alone; it needs a decode, not a settlement.

Citations: 607 verified (from 606), 0 wrong. 607 lib tests, 0 failures.

## #316 — a third citation class the guard cannot check: DATA CELLS

Three rows in this review have now cited something that is not an instruction
address, and each time disassembling there produced convincing nonsense:

  * #299 `location_var_offset` — "SCRIPT2: 0x0F4E", a SCRIPT-DATA record offset.
    The EXE at `0x0F4E` decodes as `sub ax,0x6652 / xor ax,ax / stosw ...`.
  * #315 `present_scene_buffer` — `gs:0x1FA7`, a DS cell.
  * this one, `load_bas_menus` — `gs:0x6772`, a DS cell. The EXE at `0x6772`
    decodes as `add [di-0x80],sp / push cs / stosb ...`, pure phantom.

`check_cited_instructions.py` is right to skip all three (no mnemonic beside the
address), and #300 made that skip visible. But the ledger's `ASM?` heuristic
reads ANY hex as evidence of a decode, so a data-cell reference and a routine
citation look identical to it. That is a THIRD class beside #298's wrong-routine
and #301's misplaced-within-routine.

WHAT THE CELLS ACTUALLY ARE, since checking them is cheap once you stop
disassembling:

  `gs:0x6772` — a POINTER, five sites, used stack-like: `mov [0x6772],ax`
  @`0x5461`/`0x574E`, `mov [0x6772],si` @`0x5805`, read back as `bx` @`0x57F7`
  and `si` @`0x5B07`. Consistent with the doc's "menu system", but the doc makes
  no CHECKABLE behavioural claim, so the row stays provisional: there is nothing
  to verify, which is different from having verified it.

ALSO LABELLED, from #315's loose end: `0x7B4C` as
`scene_blit_base_ten_row_setup` — `al=[0x6CDE]`, `cbw`, `shl ax,7`,
`add ax,0x1320`, store `[0x131A]`, then the missing `gs:[0x1FA7]=0xA`. The
128-byte-stride table at `0x1320` indexed by `[0x6CDE]` is the next step for
whoever closes that gap; recorded rather than chased, because decoding a new
table subsystem is not what this review is for.

Citations: 607 verified, 0 wrong. 607 lib tests, 0 failures.

## #317 — 124 of 200 "decode" rows cite a DATA CELL, not a routine

#316 named a third citation class after hitting it three times by hand. Rather
than keep finding them one row at a time, I taught the ledger to tell them apart.

THE DISCRIMINATOR is a MNEMONIC beside the address — precisely what makes a
citation checkable by `check_cited_instructions.py`. `mov word ptr [0x1fa7],0x23`
is a claim about code; `gs:0x6772` is a name for a cell. Both are hex in a doc
comment, and `audit_inventory.py` counted both as evidence of a decode.

Rows whose addresses never carry a mnemonic are now `CELL?` rather than `ASM?`.
The split:

    before   ASM? 200
    after    ASM?  76      cites at least one instruction
             CELL? 124     names cells only -- nothing for any tool to check

NEARLY TWO THIRDS. The `ASM?` queue was not 200 unchecked decode claims; it was
76, plus 124 rows whose "evidence" was a variable name in hex. That is the kind
of number that changes what the queue MEANS: the provisional-decode backlog is
much smaller than it looked, and a separate backlog of undocumented-but-plausible
port code was hiding inside it.

CHECKED THE REGENERATION RATHER THAN TRUSTING IT. The diff reported "82 settled
rows changed", which looked like the disaster #217 exists to prevent — until I
re-keyed on (item, file) instead of (item, file, LINE). Line numbers shift on
every edit. Re-checked: ZERO settled statuses lost, all 403 ASM / 236 ORACLE /
202 TESTED / 85 DATA / 96 INFRA intact. The confirmed count is unchanged at
1022 (46.0%), which is correct — this reclassifies OPEN work, it does not settle
any.

WHAT IT DOES NOT DO: `CELL?` is not a verdict on the code. `present_scene_buffer`
(#315) turned out to be a faithful decode whose doc merely named a cell instead
of an instruction. The label says "nothing here is checkable as written", which
is a statement about the DOC, and the fix for such a row is to find the
instructions — which is exactly what #315 did.

Citations: 607 verified, 0 wrong. 607 lib tests, 0 failures.

## #318 — the port dropped a term, and the doc had already recorded the symptom

Working the cleaned `ASM?` queue (76 real decode claims after #317).
`dlg_line_asset_id_from_source_byte` computes `(byte - 1) * 16` for non-negative
source bytes. The fill routine at `0x7684` computes something else:

    0x768B  js  0x7694        negative -> store sign-extended, untouched
    0x768D  dec ax
    0x768E  shl ax, 4         (byte - 1) * 16
    0x7691  add ax, 0xdd7     <- A BASE the port never had
    0x7694  stosw

So the port returned 0 where the game stores `0x0DD7`.

THE DOC HAD THE ANSWER AND CALLED IT A MYSTERY. Its caveat read: "In the hub
savestate the live `+2` fields hold `0x0DD7`, which is not 16-aligned and points
into an `fd\\xxxxxxxxxxxx` path template's name field. So either another path
populates the table in that state, or this value is later replaced." Neither.
`0x0DD7` is what a source byte of 1 stores: index zero ON THE BASE. The probe was
right, the reading of it was wrong, and the missing addend explains both the
value and the non-alignment that made it look anomalous.

THE TEST ENCODED THE BUG, twice over. It asserted
`dlg_line_asset_id_from_source_byte(1) == 0`, and then looped over every
non-negative byte asserting `result % 16 == 0` — a property that only holds
because the base was absent, and which the caveat three lines above had already
observed to be false of the real data. Doc and test contradicted each other in
the same file and both passed.

Corrected: the base is now `DLG_ASSET_NAME_TABLE_BASE = 0x0DD7`, cited at
`0x7691`; the test asserts `byte 1 -> 0x0DD7` and checks alignment RELATIVE TO
THE BASE.

Settled alongside, after verifying their instructions: `DLG_LINE_ASSET_ENTRY_STRIDE`
(4 — `shl bx,2` @`0x9D67`, and `stosw` + `add di,2` @`0x7694`/`0x7695`, which
together advance exactly four), `DLG_LINE_ASSET_ID_OFFSET` (2 — `mov si,[bx+2]`
@`0x9D6E`), `DLG_ASSET_NAME_STRIDE` (16 — `shl ax,4` @`0x768E`).

Citations: 613 verified (from 607), 0 wrong. 607 lib tests, 0 failures.

## #319 — a count from a raw byte search was two high

`dlg_line_id_for_selector`'s doc is unusually careful: it warns that this
function is only ONE path to the active line id, and quantifies it — "ONE of 29
writers of `gs:0x6788`. A byte search for every `mov [0x6788], …` encoding finds
29 sites — this one, four register writes, and 24 IMMEDIATE writes."

Its core claims all verify (and were already checked in #298): `lodsb` @`0x668D`,
the one-byte `cbw` @`0x668E` that capstone renders `cwde`, the store to
`DS:0x1FAB` @`0x668F`, and `add ax,9` @`0x11F5` forming `gs:0x6788`.

THE COUNT DOES NOT. Re-counted with `find_imm.py`, which rejects
mid-instruction matches (19 of them here): 39 confirmed references — 27 writes
and 12 reads/compares. The writes split 22 immediate, 5 register. The doc's
structure `1 + 4 + 24 = 29` gets the register side right (5 including this one)
and the immediate side wrong by two.

THE CAUSE IS IN THE DOC'S OWN SENTENCE: "a byte search". A raw byte search cannot
distinguish an instruction from the same bytes appearing inside another one, and
two of the 29 were exactly that. This is #234's phantom problem, committed by a
method that the tooling had already been fixed to avoid — and it produced a
number precise enough to look authoritative.

I ALSO DID NOT TRUST MY OWN TALLY. Having read the aggregation, I counted the
writes by hand and got 27, then re-derived it with a script rather than publish
the arithmetic — #308 was a wrong structural claim built on a hand-read of tool
output, and the fix for that is not to be more careful, it is to stop counting
by hand.

Corrected to 27 with the method named, and the row settled: everything it claims
about the instructions holds.

Citations: 613 verified, 0 wrong. 607 lib tests, 0 failures.

## #320 — five rows that verify, including one that can be READ instead of trusted

Continuing the cleaned `ASM?` queue. Five rows, all correct as written, checked
rather than assumed:

`VM_FIELD_OFFSET_SELECTOR_ENCOUNTER = 8`. The doc claims `FIELD_OFFSETS[8]` is
non-zero in exactly one column — `0x36` at column 1, i.e. kind 2. Read from the
matrix at `DS:0x6D60`: `00 36 00 00 ...`, non-zero at column 1 only. Exact.

`OBJECT_FLAG_PAIR_SEEN = 0x8000` — `or word ptr [si+2], 0x8000` at BOTH `0x5DD2`
and `0x5DFA`, as claimed.

`OBJECT_FLAG_ACTIVE = 1` — `test word ptr [si+2], 1` @`0x91D4` and
`test byte ptr [si+2], 1` @`0x83D9`. Note the two filters test the same bit at
different WIDTHS; the doc says so and both are there.

`STATUS_STRING_TABLE`. Three of its four citations name an instruction directly
(`mov si,0x12E` @`0x8369`, `mov si,0x14B` @`0x839F`); the other two say
"`0x836C`'s branch" and "`0x8376`'s branch", which is the loose form that has
been wrong before (#301). Followed both: `cmp word fs:[bp],0x10 / jne` @`0x836C`
guards `mov si,0x137` @`0x8373` (kind `0x10` = SHIP), and
`test word fs:[bp],0x100 / je` @`0x8376` guards `mov si,0x13E` @`0x837E`
(kind `0x100` = BLACK HOLE), with the planet header set first as the default.
Loose phrasing, correct content.

`LOCATION_PANEL_BOX = [0x64, 0x14, 0xA0, 0x46]` — and this one is better than a
citation. The doc says `DS:0x2780` is STATIC with no writer anywhere, which means
the four words can be READ OUT OF THE IMAGE rather than trusted:
`64 00 14 00 a0 00 46 00`. Exact match. Added
`location_panel_box_matches_the_image`, so a drift between the literal and the
shipped data fails a test instead of surviving as a plausible number — #227's
rule, applied to a table nobody had checked that way.

Running tally: fourteen rows reviewed, nine correct as written, five corrected.
The corrected five are still the more interesting half, but nine-in-fourteen is
the honest denominator.

Citations: 613 verified, 0 wrong. 608 lib tests, 0 failures.
(Wrote 617 first — the count did not move, because these five rows verified
EXISTING citations rather than adding new ones. Corrected before commit; the
same slip as #295's 585-for-583, and the reason to read the tool rather than
estimate from 'I checked several things'.)

## #321 — six more verified rows, and the settle tool refusing a name I was about to fudge

Continuing the `ASM?` queue into `engine.rs`. Six rows, all correct:

`CHOICE_BOX_PITCH = 11` — `add bp, 0xb` @`0x847A`. The doc also claims the
hit-test is `row = dy/11 + 1` from "`div bl,0x0B` @`0x8508`", which is not a real
operand form: `div bl` takes its divisor from the register. Followed it —
`mov bl, 0xb` @`0x8506` immediately precedes `div bl` @`0x8508`, and `inc al`
@`0x850A` supplies the `+1`. Shorthand, accurate in substance.

`CHOICE_BOX_ANCHOR_CONCEPT = 0xE1` — `mov word ptr [0xac6], 0xe1` @`0x89A6`,
already labelled `list_anchor_console_window`.

`TEXT_SELECTED = 0xEF` — `mov al, 0xef` @`0x858B`.
`TEXT_SELECTED_MOUSE = 0xFE` — `mov al, 0xfe` @`0x8595`, gated by
`test byte gs:[0xa3e], 1` @`0x858D`. All three exact.

THE TOOL CAUGHT ME. `audit_settle.py` REFUSED `TEXT_SELECTED`: "name not unique
in file". There are two, at 2231 and 2365 — local consts in two different drawing
functions. I had verified ONE instruction and was about to settle a bare name
that would have matched both rows.

This time the duplicate happened to be identical (same value `0xEF`, same
citation `0x858B`), so I checked the second and settled both explicitly by line.
But that is luck, not diligence: the failure mode is #306 exactly — one checked
row making an unchecked neighbour feel checked — and the guard that stopped it is
a tool refusing an ambiguous argument, not me noticing. Worth recording that the
safeguard which actually worked here was mechanical.

Ledger: 2222 items, 1034 confirmed (46.5%). The provisional split is now
`ASM?` 66, `CELL?` 123, `DATA?` 46, `ORACLE?` 41, `INFRA?` 8 — the genuine
decode-claim queue is down from 76 to 66.

608 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #322 — a self-declared "PROVENANCE DEFECT" whose provenance nobody had checked

`MENU_SUBMENU = ["EXPLANATIONS", "GAME"]` carries an unusually honest doc: it
calls itself a "PROVENANCE DEFECT — these are still transcribed literals", names
where the words really live, and defers the fix. Content-bearing literals in Rust
source ARE a defect by this project's rules, so the doc is right to say so.

But the claims in it had never been checked, and all of them hold:

    SCRIPT1.DIC  0x02FC = "explanations"
                 0x0309 = "game"
                 0x030E = "GAME"
    SCRIPT1.COD  0x4A9  = fc 02 09 03 00 00
                        = word list [0x02FC, 0x0309], zero-terminated

So the transcription is faithful AND the real path is exactly where the doc says:
an 0xA6 record's post-`0xFFFF` word list pointing at DIC offsets, which
`menu_submenu_labels` already reads when a script is loaded. The const is the
no-script fallback, not the authority.

PINNED BOTH ENDS. `menu_submenu_literals_match_the_dic_words` now asserts the DIC
words, the upper-casing the widget applies, AND the three words at COD `0x4A9`.
The literal, the dictionary and the script must agree or the test fails — which
is the smaller half of the fix, available now, while the builder routine that
would remove the literal entirely is still unfound.

SETTLED **DATA**, not ASM. The evidence here is shipped data read at named
offsets; there is no disassembly behind it. #299 made the same distinction for
`location_var_offset` (TESTED, because a data run was the evidence). Recording
the status that matches the evidence is the whole point of having more than one.

609 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #323 — the fallback literal was fine; the SELECTION beside it is the guess

#322 pinned `MENU_SUBMENU`'s transcribed literals to the DIC and the COD and
settled the row DATA. Reading the accessor that uses them turned up the real
problem, one line away.

`menu_by_offset` is faithful: each 0xA6 line record's offset maps to its menu
rows, and the dialogue path looks a menu up BY THE CURRENT LINE'S OFFSET —
`menu_by_offset.get(&line.offset)`. That is how the game reaches a menu.

`menu_submenu_labels` ignores it:

    self.menu_by_offset.iter().min_by_key(|(off, _)| **off)

It takes the globally LOWEST offset as a stand-in for "the MENU submenu". There
is no citation behind that and no rule it could cite — it is a heuristic that
works because SCRIPT1's `0x4A9` record happens to be early in the file. A script
whose first menu record is some other list returns the wrong rows and says
nothing.

THE ORDERING OF SUSPICION WAS BACKWARDS. The doc flagged the LITERAL as the
"PROVENANCE DEFECT" — and the literal turned out to be exactly right, verifiable
at both ends against shipped data. The undocumented `min_by_key` one line below
it was never flagged at all. A transcribed constant announces itself; a
plausible-looking selection does not, which is why it survived longer.

Recorded as an APPROX row in docs/port-validation.md naming `0x8428` (the console
list widget that consumes these word-offset lists) as the replacement path.
Not fixed here: reaching the menu the way the game does means decoding the MENU
click dispatch, which is a decode task rather than a review one.

609 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #324 — partial progress on the MENU dispatch, and independent support for #311

Took #323's decode task: find what supplies the MENU submenu's word-offset list,
so `menu_submenu_labels` can stop guessing with `min_by_key`.

The widget is `0x8428` (`list_widget_layout_unified`), and it takes the list in
SI — so the caller decides identity. Eight near callers. The in-window concept
path is `0x89C1`, and its setup block is now labelled:

    0x8998  or  byte [0x2793], 4      <- bit 2
    0x899D  inc byte [0x67BA]
    0x89A1  mov byte [0xADC], 0
    0x89A6  mov word [0xAC6], 0xE1    the concept anchor, centre-X 225
    0x89AC  mov byte [0xADD], 0       (so NOT the world/entity box branch)
    0x89B6  mov byte [0xADA], 4
    0x89BB  mov byte [0x27E6], 1
    0x89C1  call 0x8428

SI IS NOT SET HERE. It is inherited from this routine's caller, so the list's
identity is decided further up and the search continues there. Recorded rather
than guessed — naming the wrong caller would be exactly the #302 shape.

INDEPENDENT SUPPORT FOR #311, which is the part worth keeping. `or byte
[0x2793], 4` @`0x8998` is a SECOND setter of bit 2, in a completely different
subsystem from the presentation start (`0x593A`). #311 read bit 2 as "something
is running, defer the pending-profile load", from the main-loop gate's
`test byte [0x2793], 0xe` @`0x1095`. A concept list being shown is exactly the
kind of thing that should defer a scene load, and it raises the same bit. Two
unrelated paths, one meaning — that reading has now survived #308's and #309's
corrections of everything around it and gained a second witness.

Not fixed: the MENU dispatch. The APPROX row in docs/port-validation.md stands,
now with `0x8428`'s call sites enumerated and the concept path identified.

609 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #325 — three content literals that were in the image all along

`QUICKSAVE_SLOT_NAME = "LAST"` has a thorough doc: the game copies the literal
into the slot-name buffer at `DS:0x270D`, points `[0x2734]` at it, clears
`[0x2739]`, and jumps STRAIGHT to `vm_state_save` — a save with no rename prompt.
Every part verifies:

    0x1B58  mov si, 0x161          <- the SOURCE literal's address
    0x1B5B  mov di, 0x270d
    0x1B5E  mov word [0x2734], di
    0x1B62  mov cx, 2 / rep movsd  eight bytes
    0x1B68  mov byte [0x2739], 0
    0x1B6D  jmp 0x1c3f             vm_state_save

`mov si, 0x161` is the useful part: it names WHERE THE STRING IS. Reading there
gives `4c 41 53 54 00 50 41 55` — `"LAST\\0PAU..."` — a contiguous NUL-separated
UI string block:

    DS:0x159  "LOADING"    = LOADING_TEXT
    DS:0x161  "LAST"       = QUICKSAVE_SLOT_NAME
    DS:0x166  "PAUSE"      = PAUSE_TEXT
    DS:0x16C  "UNKNOWN"    the roster's empty caption

Three port literals, all exact. `ui_string_literals_match_the_image_block` now
reads them out and also asserts the block is CONTIGUOUS — each string ending
exactly where the next begins — which is what makes four offsets evidence rather
than four coincidences. Settled DATA (shipped data at named offsets, no
disassembly behind the strings themselves).

THE PATTERN, fourth time this session: #227 (`TEMP_SND_CALLBACK_OFFSETS`), #320
(`LOCATION_PANEL_BOX`), #322 (`MENU_SUBMENU` via the DIC), and now this. A
literal in this tree is USUALLY readable from the shipped data, and the address
is usually sitting in the routine that uses it — `mov si, 0x161` was one
instruction away the whole time. The project rule says content literals are a
defect; the practical corollary is that most of them are one disassembly from
being pinned instead.

ALSO NOTED: `or byte [0x2793], 4` @`0x1B7B`, in the save/rename path — a THIRD
independent bit-2 setter after #311's `0x593A` and #324's `0x8998`. Presentation,
concept list, save dialogue: three unrelated subsystems, one "defer the scene
load" bit.

610 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #326 — a sweep for SHORT display literals, and three self-corrections before it was usable

#325 was the fourth literal this session (after #227, #320, #322) that turned out
to be readable from shipped data. `check_content_literals.py` could not have
found any of them: it looks for PROSE, three or more words, and `"LAST"` is one.

`tools/check_ui_literals.py` closes that gap — every short display string in
`src/`, classified IN-IMAGE / IN-DATA / ABSENT. Getting it honest took three
passes, each removing a way it manufactured a finding:

  1. IT ATTRIBUTED COINCIDENCES. First run "found" `DEB`, `DIC`, `FORM` and
     `ILBM` in shipped files and reported them as game content. They are parser
     magic and file extensions; a 3-char string matches any binary by chance.
     Added `MIN_ATTRIBUTABLE = 5`, the same rule as `check_literal_tables.py`'s
     MIN_BYTES and #263's warning about matching by value.
  2. IT REPORTED FILENAMES. `TB.BIG`, `HONKF.SPR`, `BLOODPRG.EXE` dominated the
     ABSENT list and buried the real hits.
  3. IT READ COMMENTS. `/// The comms "Hate TV" screen` and three others were
     reported as suspect content. A doc comment QUOTING on-screen text is
     documentation. This is the same defect I have now found in three other
     tools this session, committed while writing a tool to find defects.

Honest output: 223 display literals — 41 in the image, 44 in shipped data, 138 in
neither, 54 too short to attribute.

THE ONE REAL FINDING in the engine modules: `PHONE_CONTACTS`. `DESCRIPT.DES`
holds `Bob_Morlock` at `0x09EB`; the port carries `"BOB MORLOCK"` — upper-cased,
underscore replaced by a space. So the names DO come from the game's data, and
the defect is narrower and more specific than "invented text": a literal standing
in for a lookup PLUS an unverified formatting rule. Recorded as an APPROX row
naming what must replace it.

The rest of the ABSENT list is port-side: window titles, module headings, CLI
arguments. That is the answer I wanted and could not have asserted without
checking.

610 lib tests, 0 failures. 613 citations verified, 0 wrong.

## #327 — the game has no underscore substitution, and its case-fold preserves one

#326 recorded `PHONE_CONTACTS` as "transcribed AND transformed": `DESCRIPT.DES`
holds `Bob_Morlock`, the port carries `"BOB MORLOCK"`. Two operations are implied
— upper-case, and underscore→space. I went looking for both.

UPPER-CASING IS REAL. `0x2760` folds a NUL-terminated string in place:

    0x2762  mov al, byte ptr es:[di]
    0x2765  cmp al, 0x61          'a'
    0x2767  jb  0x276e            BELOW 'a' -> leave untouched
    0x2769  and al, 0xdf          upper-case
    0x276B  mov byte ptr es:[di], al
    0x276F  or al,al / jne        until NUL

THE UNDERSCORE SUBSTITUTION IS NOT, and the argument is two-sided:

  * `find_imm.py 5f` finds ZERO confirmed instructions comparing against `0x5F`
    (28 candidates, all rejected as mid-instruction phantoms). No code path in
    the executable special-cases an underscore.
  * The one case-folding loop explicitly SKIPS characters below `'a'`, and `0x5F`
    is below `'a'`. So an underscore does not merely survive by omission — the
    loop's own guard is what preserves it.

The faithful caption for `Bob_Morlock` is therefore `BOB_MORLOCK`, and the space
in the port most likely came from reading a screenshot — the exact provenance the
prime rule forbids.

NOT CHANGED. `0x2760` has not been shown to be the routine that renders THESE
captions; it is the general fold. Rewriting nine display strings on an inference
is how capture-derived content gets IN, which would be a poor way to fix content
that got in the same way. The finding is recorded on the constant, labelled at
`0x2760`, and the APPROX row already names what must replace the table.

WHAT MAKES THIS ONE STRONGER THAN A GUESS: the negative search. "The game never
converts underscores" is a claim about ABSENCE, and absence is normally the
weakest kind of evidence — but `find_imm` enumerates every instruction with a
given immediate and rejects phantoms, so zero hits is a real zero, not a failure
to look. Same instrument that was silently truncating two entries ago (#309);
worth noting that fixing it is what makes this argument available.

616 citations verified (from 613), 0 wrong. 610 lib tests, 0 failures.

## #328 — #327 attributed a routine it had not placed

#327 argued the port's `BOB MORLOCK` should be `BOB_MORLOCK`, on two legs:
no `0x5F` comparison exists in the image, and "the game's only case-folding loop"
at `0x2760` preserves characters below `'a'`.

Went looking for that loop's callers, to show it renders the phone captions.
It has NONE. `find_near_callers.py` returns nothing; `xref.py 0:2160` returns
zero far calls. It is not a helper at all — it is a fall-through block, and the
instructions immediately above it are:

    0x274D  mov bx, word ptr gs:[0xa88]   a file handle
    0x2752  mov cx, 0xffff
    0x2755  mov ah, 0x3f / int 0x21       DOS READ FILE
    0x275C  pop di / pop es
    0x2760  ...the fold

So it upper-cases something JUST LOADED FROM A FILE — most plausibly a path or
name being normalised. Calling it "the caption renderer's fold" was an
attribution I had not earned, and calling it "the game's only case-folding loop"
was wrong twice over: `0x77B5` is a second one, which #327's own label mentions.

WHAT SURVIVES, and it is the leg that mattered: NO instruction anywhere in
BLOODPRG.EXE compares against `0x5F`. That claim never depended on `0x2760` —
it is an exhaustive enumeration over the whole image with phantoms rejected. So
nothing in the game can special-case an underscore, whatever renders the caption,
and `BOB MORLOCK` is still unsupported.

THE SHAPE OF THE MISTAKE: I found a routine that did the right thing and put it
in the story without checking whether it was on the path. That is #302's error
(a decoded fact and an invented policy under one citation) in its subtler form —
here BOTH facts were true, and only the CONNECTION between them was invented.
A true statement placed in the wrong argument still makes the argument wrong.

Label and doc corrected in place; the table is unchanged, as #327 already decided.

615 citations verified, 0 wrong (down one: a citation was removed with the
withdrawn claim). 610 lib tests, 0 failures.

## #329 — a row is settled on ITS evidence, not its neighbour's

Five rows in the save-header cluster, all correct, and one procedural point worth
keeping.

`SAVE_WRITE_SIZE_IMMEDIATES` records where each of `vm_state_save`'s three
`int 21h` AH=0x40 write sizes lives, and its doc flags a trap: the third pair
emits `mov dx` FIRST, so its immediate sits at `0x1C76` rather than where the
earlier spacing suggests. Verified:

    0x1C60  mov cx, 2        immediate at 0x1C61   PROFILE_SIZE = 2
    0x1C6A  mov cx, 0x200    immediate at 0x1C6B   FLAGS_SIZE = 0x200
    0x1C72  mov dx, 0x6cde   <- the flip
    0x1C75  mov cx, 0x60     immediate at 0x1C76   STATE_SIZE = 0x60

and `the_header_sizes_are_the_writers_own_immediates` already reads all three
back out of the image, along with the deliberate save/load asymmetry (the writer
streams the CURRENT profile from `DS:0x677E`, the reader posts it as PENDING into
`DS:0x6780`). This row was cited AND pinned the whole time.

`SHIP_3D_PARENT_LINK_SENTINEL` — `cmp si,-1` @`0x61CD`, `mov si,gs:[0x6752]`
@`0x61D2`. Confirmed again rather than trusted because I wrote it in #291, and a
citation being mine is not evidence.

THE PROCEDURAL POINT: `audit_settle.py` REFUSED `PROFILE_SIZE` and `FLAGS_SIZE`
— "ASM needs a cited address". Both were verified by the check above, but the
addresses lived on the TABLE, not on the constants. The tool is right and the
distinction is not pedantry: a reader who lands on `pub const FLAGS_SIZE` sees a
number with no provenance, and "the neighbouring item is cited" is exactly the
reasoning that let #306 settle an unchecked row. Added the citations to the
constants themselves, then settled.

Second time this session a settle refusal caught something (#321 was the other,
on an ambiguous name). The tool is a better auditor of my shortcuts than I am.

620 citations verified (from 615), 0 wrong. 610 lib tests, 0 failures.

(Wrote 617 first. THIRD citation-count slip this session — #295 said 585 for
583, #320 said 617 for 613, this said 617 for 620. Every one came from
estimating the number instead of reading the tool that prints it, and the
estimate was wrong in both directions, so it is not a bias I could correct for.
The rule that works is the same one #319 arrived at for counting instructions:
run the tool, paste the number, do not predict it.)

## #330 — the bridge view verifies; its arrival side-effects do not exist in the port

`update_view` claims the frame→yaw sync at `0x97E4` snaps the ring cursor to an
8-unit boundary before the screen rebase. Verified end to end:

    0x97E7  mov ax, [0x2795]        the panorama frame index
    0x97EA  mov [0x2f6d], ax        -> the ship-3D yaw index, 1:1
    0x97ED  shl ax, 3               frame * 8
    0x97F0  sub ax, 0xa0            ...minus half a screen
    0x97F3  mov [0x27a7], ax
    0x97F6  and word [0xa2a], 0xfff8  <- the snap

The port's `ring_mouse_x &= !(RING_PX_PER_FRAME - 1)` with `RING_PX_PER_FRAME = 8`
is `&= 0xFFF8` exactly, and its screen rebase
`ring_mouse_x - (frame * RING_PX_PER_FRAME - HALF_SCREEN)` is `[0xa2a] - [0x27a7]`
with `HALF_SCREEN = 0xA0`. Settled.

`update_view_steer` NOT settled, and checking its own citations is why. The seek
arithmetic verifies (`mov dx,[0x279b] / shr dx,1` @`0x9667` halves the target arc
into a frame index, which is what the port computes). But the ARRIVAL path does
two things the port does not:

    0x9671  xor word ptr [0x2793], 8
    0x9676  mov word ptr [0x279d], 0

Bit 3 of `0x2793` is one of the three tested together by the main-loop busy gate
(`test byte [0x2793], 0xe` @`0x1095`), so finishing a station seek participates in
the pending-profile decision. The port just clears its `seeking` flag.

THIS IS THE FOURTH SUBSYSTEM to land on the same missing piece — presentation
(#311), concept lists (#324), the save dialogue (#325), and now the bridge seek
all touch `0x2793`'s upper bits, and none of them can be finished until those
flags are modelled as state. That is a stronger argument for doing #312's work
than #312 itself made: it is not one unported gate, it is the thing four
separate paths are waiting on.

620 citations verified, 0 wrong. 610 lib tests, 0 failures.

## #331 — built the thing four subsystems were waiting on

#330 made the argument that #312 had not: `gs:0x2793` is not one unported gate,
it is what presentation (#311), concept lists (#324), the save dialogue (#325)
and the bridge station seek (#330) are all blocked on. So it is now STATE.

`VmMachine::ui_flags: u16` models the word as a word — which #309 established it
is, with at least six live bits read individually and as composite masks. Four
bit constants, each cited at the instruction that reads or writes it:

    UI_FLAG_CE_BRANCH   0x0001  test byte gs:[0x2793],1 / jne  @0x6494
    UI_FLAG_BUSY        0x0004  or byte [0x2793],4  @0x593A, @0x8998, @0x1B7B
    UI_FLAG_SEEK_ARRIVED 0x0008 xor word [0x2793],8 @0x9671   (toggle, not set)
    UI_FLAG_DEFER_MASK  0x000E  test byte [0x2793],0xe / jne  @0x1095

The presentation lifecycle now raises `UI_FLAG_BUSY` on start and clears it in
the 0xC9 teardown, matching `or ...,4` / `and ...,0xfb`.

ADDITIVE ON PURPOSE. `presentation_busy` still sets bit-0 semantics, which #311
proved wrong and #311's own experiment proved LOAD-BEARING — removing it fails
six tests including the story drive. So this adds the correct bit beside the
wrong one rather than swapping one guess for another. The divergence is now
narrowed from "the port sets the wrong flag" to "the port also sets bit 0", which
is a smaller and better-documented lie.

STILL NOT PORTED, and #312's reasoning stands: the GATE. It ORs ten separate
subsystem-active bytes at `0x109C`..`0x10BF` that the port does not model, and a
predicate over the bits I now have would defer on presentation and nothing else —
passing every test I could write for it. The blocker has moved from "the flag
word does not exist" to "the ten flags do not exist", which is progress of the
kind that can be checked.

I ALSO CAUGHT MYSELF: the new test was called
`presentation_lifecycle_raises_and_clears_the_defer_bit` and only exercised the
raise — the clear sits in `step()`'s 0xC9 arm and needs a full presentation to
reach. Renamed to `presentation_start_raises_the_defer_bit` with the scope stated.
That is #313's finding (a test whose name promised more than its assertions) with
my name on it, one entry after I wrote it up.

629 citations verified (from 620), 0 wrong. 611 lib tests, 0 failures.

## #332 — the ten flags were two-thirds already named, under a name that hid their reader

#312 declined to port the main-loop busy gate because `VmMachine` models "exactly
ONE of the ten" flags it ORs. #331 moved the blocker to "the ten flags do not
exist". Before building them I checked what the port already had — and eight of
them were there.

`VM_PRESENTATION_INPUT_GATE_A..H` are `0x24F3`, `0x2751`, `0x5E64`, `0x2565`,
`0x2736`, `0x2737`, `0x27DA`, `0x2792`. The gate at `0x109C`..`0x10BF` ORs
`0x67AC`, `0x24F3`, `0x2751`, `0x67B0`, `0x5E64`, `0x2565`, `0x2736`, `0x2737`,
`0x27DA`, `0x2792`. A..H is exactly that list minus `0x67AC` and `0x67B0`.

The constants were named from where they are WRITTEN — each doc says "Touched by
the game at `mov byte ptr [...], 1` @..., found by decoding forward from a
verified routine entry". Correct, careful, and blind to the fact that ten of them
are read TOGETHER by one instruction sequence. A group identified by its writers
does not announce its reader.

`VM_PRESENTATION_INPUT_GATE_I` (`0x2A19`) IS NOT ONE OF THEM. The gate never
reads it. Sharing the `INPUT_GATE` name with A..H makes it look like the ninth
member of a set it does not belong to — noted on the constant.

WHAT THIS CHANGES. #312's "the port does not model them" was true of the STATE
and false of the DECODE: two-thirds of the list was sitting in the file with
addresses and citations. The remaining work is smaller and much better defined —
model these as bytes, add `0x67AC`/`0x67B0`, and find what SETS each, which their
existing docs already half-answer (`mov byte [0x2751],1` @`0x8836`,
`mov byte [0x2736],1` @`0x892C`, `mov byte [0x2737],1` @`0x893C`).

I did not build the gate. Nothing about this entry changes #312's reason: an
under-fed gate defers on presentation and nothing else, and passes every test I
could write. But the argument for it being close is now evidence rather than
hope.

639 citations verified (from 629), 0 wrong. 611 lib tests, 0 failures.

## #333 — derive the gate's operand list from the instruction stream

#332 found eight of the main-loop gate's ten flags already named in the port.
This adds the list itself — `MAIN_LOOP_BUSY_BYTES` — and does NOT transcribe it.

The sequence at `0x109C`..`0x10BF` is perfectly regular:

    a0 ac 67          mov al, [0x67ac]
    0a 06 f3 24       or  al, [0x24f3]
    0a 06 51 27       or  al, [0x2751]
    ... seven more ...
    jne               @0x10C3

so `main_loop_busy_bytes_match_the_or_sequence` PARSES it — `0xA0` plus a word,
then `0x0A 0x06` plus a word until the pattern stops — and asserts the port's
array equals what it decoded, in order. It also asserts the walk ENDS at `0x10C3`,
the branch the decode named, which is what makes "ten" a result rather than an
assumption.

Two more assertions carry #332's finding into the test: every
`VM_PRESENTATION_INPUT_GATE_A..H` must appear among the operands, and
`INPUT_GATE_I` (`0x2A19`) must NOT. A naming mistake that no longer matches the
code will now fail rather than mislead.

THIS IS THE STRONGEST FORM AVAILABLE for a table like this — the same move as
#320 (`LOCATION_PANEL_BOX` read from `DS:0x2780`) and #325 (the UI string block),
but applied to CODE rather than data: the constant is derived by decoding the
instructions that use it, so it cannot drift from them. #227's rule generalises
further than I had been applying it.

The gate itself remains unported for #312's reason, unchanged: nine of the ten
bytes have no writer in the port, so a predicate over them would defer on
presentation alone.

641 citations verified (from 639), 0 wrong. 612 lib tests, 0 failures.

## #334 — `find_imm.py` has FALSE NEGATIVES, and one of my arguments rested on a zero from it

Chasing what SETS the ten gate flags. `0x2737` is the smallest, six confirmed
references, and NONE of them is a setter — only a clear at `0x1D67`. But the
port's `VM_PRESENTATION_INPUT_GATE_F` doc cites `mov byte ptr [0x2737], 1`
@`0x893C`, which was not in the list.

The doc is right. The raw bytes at `0x8937` are

    c6 06 38 27 01    mov byte [0x2738], 1
    c6 06 37 27 01    mov byte [0x2737], 1     <- 0x893C

an unmistakable pair. `find_imm.py` REJECTED the second as a phantom.

WHY: `confirmed()` decodes from seven back-anchors and requires a MAJORITY to
agree that the address starts an instruction. A genuine instruction sitting in a
neighbourhood that decodes badly loses that vote. The heuristic is sound for
*suppressing* noise and unsound as *proof of absence*.

WHICH BREAKS AN ARGUMENT I MADE. #327 concluded "no instruction anywhere compares
against `0x5F`, so nothing converts an underscore to a space", citing
`find_imm.py 5f` returning zero. That zero is no longer evidence.

RE-DERIVED, and the conclusion survives on a better basis: the plausible
ENCODINGS are absent as raw bytes — `3c 5f` (`cmp al,imm8`), `80 fc 5f`
(`cmp ah,imm8`), `2c 5f` (`sub al,imm8`) and `80 3e .. .. 5f`
(`cmp byte [imm16],imm8`) each occur ZERO times in the image. A byte search
cannot have false negatives for a fixed encoding, so this is the argument #327
should have made. Doc corrected in place.

FIXED THE TOOL the way #309 and #310 fixed the others: `--rejected` now lists the
discarded candidates under the header "MAY CONTAIN REAL INSTRUCTIONS", and the
docstring says plainly that a zero result is not proof of absence and names the
byte-search alternative.

THE THEME, now five instruments deep (#296 probe, #300 guard, #309/#310 search
truncation, and this): every one reported a clean summary that concealed its own
limits. This is the first where the limit produced a WRONG CONCLUSION in the
tree rather than a missed one — and the only reason I found it was chasing an
unrelated flag and noticing a doc that disagreed with a tool.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #335 — #319's "correction" was the error: the count goes back to 29

#334 established that `find_imm.py` rejects real instructions. The obvious next
question is what I concluded FROM it, and #319 is the answer: I took a doc saying
"ONE of 29 writers of `gs:0x6788`", noted that its method was "a byte search",
argued a byte search cannot reject phantoms, recounted with `find_imm`, and wrote
the doc down to 27.

The reasoning was right in general and wrong here. Byte searches have FALSE
POSITIVES; `find_imm` has FALSE NEGATIVES. I replaced a number carrying one error
mode with a number carrying the other, and did not check which was actually
biting.

RECOUNTED PROPERLY by enumerating every WRITE ENCODING as raw bytes:

    c7 06 88 67      mov word [0x6788], imm16
    a3 88 67         mov [0x6788], ax          <- the accumulator form,
                                                  which my first recount missed
    89 /r 88 67      mov [0x6788], reg16
    65 <each above>  the gs-prefixed variants

with overlaps collapsed, since `65 c7 06 88 67` also matches `c7 06 88 67` one
byte in and would otherwise be counted twice. Result: 29 DISTINCT SITES, agreeing
exactly with the original doc.

So the doc was right, my correction was wrong, and the doc now says so along with
the reason to trust 29: a byte search cannot MISS a fixed encoding — that is the
error mode it does not have.

TWO THINGS I GOT WRONG IN ONE ENTRY, worth separating. First, treating "this
method has a known weakness" as "this number is wrong" — a weakness is a reason
to CHECK, not to overwrite. Second, my own first recount here was also short (26)
because I forgot `a3`, the accumulator-direct store; I only caught it because 26
disagreed with both 27 and 29 and disagreement is the thing worth chasing.

#319 also settled the row on the strength of that recount. The row stays settled —
its INSTRUCTION claims were verified independently in #298 and are unaffected —
but the entry that settled it contained a false correction, which is now recorded
next to it.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #336 — give the search the OPPOSITE error mode, and make it name what it skips

#334 found `find_imm.py` misses real instructions; #335 found I had written a
wrong count into the tree by trusting it over a byte search. The fix is not to
pick a better tool — both are wrong in different directions — it is to show BOTH
in one place.

`--encodings` now enumerates the fixed WRITE/READ encodings for an address as raw
bytes: `c7 06`, `c6 06`, `a3`, `a1`, `f6 06`, `80 0e`, `80 26`, each also in its
`65`-prefixed form with the prefixed count reported as a SUBSET so it is not
double-added. For `0x6788` that gives 24 + 3 immediate/accumulator writes and 5
reads.

  * the CONFIRMED list has no false positives and misses instructions (#334);
  * the ENCODING list has no false negatives and can match bytes inside another
    instruction.

An absence claim now has a sound instrument (#327's underscore argument), and a
presence claim still has a filtered one. #335's error was having only the second.

AND IT SAYS WHAT IT DOES NOT SEARCH. `mov [imm16], reg16` is `89 /r` with a modrm
byte that varies by register, which a substring search cannot express — and that
form is exactly the two sites #335 needed to reach 29. So the output ends with a
line naming the unsearched encodings and pointing at the regex approach. A list
of encodings that quietly omits one is the same defect as a count that quietly
truncates (#309), and it would have been easy to ship without noticing, since the
numbers it does print are correct.

Sixth instrument this session to be corrected for concealing its own limits.
The pattern is now specific enough to state as a rule: A TOOL THAT FILTERS MUST
REPORT WHAT IT FILTERED, AND A TOOL THAT SEARCHES A SET MUST REPORT WHAT IS NOT
IN THE SET.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #337 — what actually sets two of the gate flags: a console-mode dismiss ladder

#332 left the work as "find what SETS each of the ten". With the search fixed
(#334/#336), `0x2736` and `0x2737` — `INPUT_GATE_E` and `F` — are answerable.

Their setters are consecutive arms of one `dec al / jns` ladder at `0x8923`:

    0x8923  dec al / jns   arm 0 -> mov [0x2738],1 ; mov [0x2736],1
    0x8933  dec al / jns   arm 1 -> mov [0x2738],1 ; mov [0x2737],1
    0x8943  dec al / jns   arm 2 -> mov [0xB13],2 ; [0xA3E]=0 ; [0xA40]=0
    0x8956  mov word [0x2a19], 0      <- clears INPUT_GATE_I
    0x895C  and byte [0x2793], 0xfb   <- clears the BUSY bit
    0x8962  ret

So the "ten subsystem-active flags" are per-CONSOLE-MODE markers, selected by AL,
each arm also raising the shared `[0x2738]`. That makes the main-loop gate's
question concrete: it is asking whether any console mode is still up.

THREE THINGS FALL OUT.

  * A THIRD clear site for `UI_FLAG_BUSY` — `0x895C`, beside `0x59BF` and
    `0x5E99`. The constant's doc now lists it. Every setter found so far
    (presentation `0x593A`, concept list `0x8998`, save dialogue `0x1B7B`) has a
    matching teardown, which is the shape a "something is running" bit should
    have.
  * `INPUT_GATE_I` (`0x2A19`) is cleared in the SAME tail — so #332 was right
    that the gate never reads it, and also right to keep it nearby: it belongs to
    this family, just not to that reader.
  * The port CAN eventually feed these. It already has console functions
    (HONK / TELEPHONE / CRYOBOX / MENU / OPTION), which is what AL selects here.
    Mapping AL values to those is the next step and is a decode, not a guess.

NOT WIRED YET: which AL value is which console mode is not established, and
assigning them by order would be exactly #302's invented-policy shape. The ladder
gives the structure; the mapping needs the caller that loads AL.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #338 — AL is the picked menu row: the console-mode flags come from the widget's return

#337 found the `dec al / jns` ladder that sets the gate flags and left the
question "what loads AL". Scanning back, the ladder has FIVE arms — `0x88C3`,
`0x88D4`, `0x8923`, `0x8933`, `0x8943` — and immediately before the first:

    0x88B4  mov byte ptr [0x2565], 0    clear INPUT_GATE_D
    0x88B9  push cs
    0x88BA  call 0x8428                 <- the unified LIST WIDGET
    0x88BD  or ax, ax / js              negative = nothing picked

The call target is `e8 6b fb`, rel `-0x495` from `0x88BD` = `0x8428`, which is
`list_widget_layout_unified`. So AL IS THE ROW THE PLAYER PICKED, and each ladder
arm raises the subsystem flag for that row's console mode.

THAT CLOSES THE LOOP the last several entries have been circling:

    the player picks a row in the console list widget (0x8428)
      -> AL = row index
      -> the ladder raises that mode's flag (e.g. [0x2736], [0x2737])
      -> the main-loop gate (test [0x2793],0xe + OR of ten bytes, 0x1095)
         sees a mode is up and DEFERS the pending script-profile load
      -> dismissing the mode clears the flag and the busy bit (0x895C)

Five arms is also a match for the port's five console functions (HONK,
TELEPHONE, CRYOBOX, MENU, OPTION), which is suggestive and NOT yet a mapping.

WHAT STILL BLOCKS THE MAPPING is the same thing #323 recorded for
`menu_submenu_labels`: the widget takes its rows as a word-offset list in SI, and
which list is passed is decided by the caller. Row ORDER determines which AL
value means which mode. So two open questions that looked unrelated — "which menu
does `menu_submenu_labels` mean?" and "which console mode is AL=2?" — are the
same question, and answering it once settles both.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #339 — `mov si` is not where SI came from when a string instruction sits between

#338 left one question: which word-offset list reaches the console list widget.
Reading back from the call at `0x88BA` to the nearest `mov si, imm16` gives
`0x2AAB` @`0x8892`. That is the wrong answer, and the reason is worth recording.

    0x8892  mov si, 0x2AAB
    0x8895  mov di, 0x25CF
    0x8898  movsd                <- SI += 4
    0x889A  movsd                <- SI += 4
    0x88A3  push si              (now 0x2AB3)
    0x88A4  mov si, 0x2AAB
    0x88AA  lcall 0x8B:0x0FAD
    0x88AF  pop si               (restores 0x2AB3)
    0x88BA  call 0x8428          <- SI = 0x2AB3

The two `movsd` ADVANCE SI by eight, and the `push`/`pop` pair around the lcall
preserves the ADVANCED value, not the loaded one. So the widget's row list starts
at `DS:0x2AB3`; `DS:0x2AAB` is an 8-byte parameter block ahead of it, built at
`0x9029` from the mouse coordinates `[0x0A2A]`/`[0x0A2C]` plus two `4`s, and
copied to `0x25CF` by those very `movsd`s.

I NEARLY RECORDED `0x2AAB` AS THE LIST. The check that caught it was mechanical:
`movsd` moves DS:SI to ES:DI and increments both, so any read-back that stops at
the last `mov si` is wrong whenever a string instruction intervenes. Same family
as #328 (a routine that did the right thing but was not on the path) — the
instructions were all read correctly and the conclusion still would not have been.

This is now the third distinct way a plausible pointer can be wrong in this
codebase: phantom decode (#234), off-path attribution (#328), and silent register
advance (this). All three produce an address that disassembles cleanly.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #340 — #339 solved the trap and then answered the question wrong anyway

#339 correctly identified that two `movsd` advance SI by eight, so reading back
to the nearest `mov si, imm16` gives `0x2AAB` and that is definitely wrong. Then
it concluded SI = `0x2AB3` and recorded that as the console menu's row list.

The second half does not survive one more check. `0x2AB3` has exactly THREE
references in the whole image — `0x844E`, `0x8493`, `0x854E` — and all three are
INSIDE `list_widget_layout_unified`, where it is the per-label WIDTH SCRATCH
(`mov di, 0x2ab3` then `lodsw` from SI, measure, store). Nothing anywhere writes
a row list to it.

So I have two checked facts that do not fit:

  * SI at `0x88BA` traces to `0x2AB3` — `mov si,0x2AAB` @`0x8892`, two `movsd`,
    and a `push si`/`pop si` pair that preserves the ADVANCED value, with no
    later load;
  * `0x2AB3` is the widget's own output buffer and no external code fills it.

Either the trace misses a write to SI that I have not found, or this call site
uses the widget in a mode where SI is already the width buffer. I do not know
which, and the label now says so instead of asserting the tidier answer.

WHAT I DID WRONG: I had a real finding — the `movsd` trap — and let it carry a
conclusion it did not support. Establishing that `0x2AAB` is wrong is not the
same as establishing that `0x2AB3` is right, and the second claim got the first
one's credibility. That is #328's shape a second time (true facts, invented
connection), and this time I was the one who had just written up #328.

THE CHECK THAT CAUGHT IT was asking who WRITES the address, not who reads it —
the same question #302 needed for the C3 promoter and #315 for the blit base.
"Where does this data come from" is apparently the question I skip when the
pointer arithmetic was hard enough to feel like the answer.

The `movsd` lesson stands and is worth keeping: a read-back that stops at the
last `mov si` is wrong whenever a string instruction intervenes.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #341 — the contradiction is real: control flow confirms the trace

#340 left two facts that would not fit. One of them could have been an
incomplete trace — SI might arrive from a predecessor I had not looked at. It
does not.

Scanning `0x8700`..`0x8900` for every short jump, near jump and near `jcc`
landing in `0x88B4`..`0x88BD` finds exactly ONE: `je 0x88B9` @`0x88A1`, the
branch already accounted for. So the call at `0x88BA` is reached only by
fall-through or by that `je`, and BOTH paths carry `SI = 0x2AB3`. The trace is
complete.

So the contradiction is not an artefact:

  * SI at the call IS `0x2AB3` (confirmed from the control-flow side);
  * `0x2AB3` is `list_widget_layout_unified`'s own width scratch, and nothing
    outside the widget writes it.

ONE HYPOTHESIS, recorded as such. The widget has TWO `mov di, 0x2ab3` sites
(`0x844E`, `0x8493`) and eight near callers (#324). If an earlier invocation
fills `0x2AB3` with per-label widths, then THIS call is not rendering a
word-offset list at all — it is re-entering the widget over its own prior output,
which would explain why no external code ever fills the buffer. That is coherent
and unproven; distinguishing it needs the caller ORDER, i.e. which of the eight
sites runs first in a console-menu open.

WHY I AM STOPPING HERE rather than guessing. The remaining step is to establish an
ordering between eight call sites, and every wrong answer available produces a
plausible mapping from AL to console mode — which is the mapping #338 wanted and
#337 warned against inventing. A hypothesis in the ledger costs nothing; a
fabricated mapping would be found later by someone who trusted it.

WHAT THIS THREAD PRODUCED, since it did not reach its goal: the gate flags are
per-console-mode (#337), AL is the picked row (#338), the `movsd` pointer trap
(#339), and the retraction (#340). The open question is now precisely stated
instead of vaguely open — "which of the eight `0x8428` call sites runs first, and
does it fill `0x2AB3`" — which is the difference between a dead end and a next
step.

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #342 — the status headers are in the same string block as the UI literals

Four `bloodprg.rs` rows — `STATUS_HEADER_PLANET_DS` and its three siblings — are
DS-offset constants whose citations are the `mov si, imm` operands verified in
#320 (`0x8369`, `0x836C`'s branch, `0x8376`'s branch, `0x839F`).

Reading the image at those offsets closes the loop:

    DS:0x12E  "PLANET: "
    DS:0x137  "SHIP: "
    DS:0x13E  "BLACK HOLE: "
    DS:0x14B  "LIFE SUPPORT:"

and these sit in the SAME contiguous block as #325's UI strings (`0x159`
`"LOADING"`, `0x161` `"LAST"`, `0x166` `"PAUSE"`, `0x16C` `"UNKNOWN"`). It is one
NUL-separated table spanning at least `0x12E`..`0x173`, not two coincidental
neighbourhoods.

`ui_string_literals_match_the_image_block` now reads all of it and asserts the
contiguity across the status headers too — each string ending exactly where the
next begins. That is what turns eight offsets into one verified table: any single
offset could be right by luck, but a chain of terminators cannot.

Settled ASM: the constants are addresses, the instructions that load them are
verified, and the strings at those addresses are what the names say.

Ledger: 2222 items, 1048 confirmed (47.2%).

641 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #343 — the alien PRNG verifies, including the carry semantics the port depends on

`alien_anim_prng_next` is in the CROOLIS OVERLAY, a different address space from
the executable (`XDB:croolis:` per `re/CLAUDE.md`). Disassembled there:

    0x16A4  mov si, [di+0x16]            the routine entry is a vtable dispatch
    0x16AA  test word [di+0x36], 0xffff
    0x16AF  je 0x16B4
    0x16B1  jmp word ptr [si+0xe]
    0x16B4  mov ax, word ptr fs:[0x105c] <- the PRNG proper
    0x16B8  ror ax, 7
    0x16BB  sbb ax, 0

The port is `rotate_right(seed, 7)` then subtract `rotated >> 15`, and the whole
thing turns on a carry rule worth stating: `ROR` by n leaves CF equal to the LAST
BIT ROTATED, which for a right rotate is the MSB of the RESULT. So `sbb ax,0`
subtracts the rotated value's top bit. The port's `rotated >> 15` is exactly that,
and the doc already explained why — one of the better-argued docs in the tree.

CITATION TIGHTENED. The doc named the routine (`0x16A4`) and then quoted the three
instructions, which is the loose form #301 caught being wrong elsewhere. Here the
content was right but the addresses were three instructions off, because the entry
is a dispatch. Now cited individually.

THE GUARD USED THE RIGHT IMAGE without being told: `check_cited_instructions.py`
picks an overlay by source filename (`src/croolis.rs` -> `croolis.xdb`), so the
four new citations were checked against the overlay, not the executable. Worth
noting because a cross-space citation is the failure #316 catalogued, and this is
the one tool that already handles it.

645 citations verified (from 641), 0 wrong. 612 lib tests, 0 failures.

## #344 — a correction that reached labels.csv and not the source

`proximity_visible`'s doc still said `DS:0x22EC` "is a WORD (`movsx eax,word ptr
[0x22ec]` @`0xBFA`)". audit-fixes #271 established the opposite — it is the HIGH
WORD of a 32-bit accumulator at `0x22EA` — and corrected `re/labels.csv`. The
source doc kept the old claim, so the tree carried BOTH readings for the same
cell, in two files, with the correction only in one.

The binary settles it in three lines:

    0x1FC5  add dword ptr [0x22ea], eax    X  -> high word 0x22EC
    0x1FD5  add dword ptr [0x22ee], eax    Y  -> high word 0x22F0
    0x1FE5  add dword ptr [0x22f2], eax    Z  -> high word 0x22F4

Every axis is a dword accumulator. The doc had already reasoned this correctly
for Y (`0x22F0` "is NOT" a word) while asserting the reverse for X — the two
sentences were adjacent and contradicted each other about identical structures.

Corrected, with #271's rule restated where it applies: a LOAD tells you what the
caller wanted, only the STORE tells you how wide the cell is. The consequence for
wiring also got bigger — `camera: [i16; 3]` would drop fractional motion on ALL
THREE axes, not just the one the doc noticed.

WHY IT SURVIVED: #271 fixed the label because the label was what the entry was
about. Nothing checks that a source doc agrees with `labels.csv` about a cell's
width, and `cell_widths.py` (written FOR #271) reports widths from the executable,
not the overlays. So an overlay claim can disagree with an overlay label
indefinitely.

647 citations verified (from 645), 0 wrong. 612 lib tests, 0 failures.

## #345 — extend the width scanner to the overlays, and reintroduce a bug it documents

#344 found a 16-bit-load-hiding-a-32-bit-accumulator claim still asserted in
`src/croolis.rs` three entries after the label was corrected. `cell_widths.py`
exists precisely to catch that class — and it read only `BLOODPRG.EXE`, so an
overlay cell could disagree with its own label indefinitely.

`--overlay croolis.xdb` now scans the overlays. Result:

    18 written DS cells; 6 written 32 bits wide
      0x22de -> high word 0x22e0      0x22ea -> high word 0x22ec
      0x22e2 -> high word 0x22e4      0x22ee -> high word 0x22f0
      0x22e6 -> high word 0x22e8      0x22f2 -> high word 0x22f4

SIX accumulators, not three. The camera triple (`0x22EA`/`0x22EE`/`0x22F2`) is
known; `0x22DE`/`0x22E2`/`0x22E6` are a SECOND triple immediately before it, and
their existence fits the labels' note that each camera axis is stepped by
`[0x22d2|0x22d6|0x22da] * ebx >> 3` — a velocity triple beside a position triple.

NO NEW DEFECT: the port references exactly one of the six, and only as a recomp
test leaf. Nothing treats them as words. That is the result I wanted and could
not have asserted without scanning.

AND I REINTRODUCED THE BUG THE TOOL DOCUMENTS. Its docstring records `--image`
leaking its value into the positional list, "where it was parsed as a hex
address (the same bug `find_imm.py` fixed for `--max`)". Adding `--overlay` I
wrote a second `if` and left the value to leak — identical bug, identical place,
with the fix described three lines above in the same file. Now a `VALUE_FLAGS`
SET, so the next flag cannot repeat it.

Third time this session a tool has been given a defect it already warns about
(#326 read comments while hunting content; #336 omitted an encoding while
listing encodings). Documenting a trap does not stop you walking into it; only
making it structurally impossible does.

647 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #346 — the code was right and the sentence describing it was wrong

Two lifted I/O routines from the recompiler.

`func_cc0` (`0x0CC0`) verifies instruction for instruction: `push ax`,
`xor ax,ax`, `mov al,gs:[0x5232]`, `int 0x10`, `pop ax`, `retf`. Settled.

`func_d4a` (`0x0D4A`) also verifies as CODE — and its prose does not. The doc
said it sets "the mouse cursor's horizontal (fn 7) then vertical (fn 8) range to
[AX,BX]". The instruction order says otherwise:

    0x0D4E  mov cx, ax / mov dx, bx    the range
    0x0D52  mov ax, 7 / int 0x33       fn 7 -- horizontal, gets [AX,BX]
    0x0D57  pop dx
    0x0D58  pop cx                     <- CALLER'S cx/dx restored HERE
    0x0D59  mov ax, 8 / int 0x33       fn 8 -- vertical, gets the CALLER'S
                                       cx/dx, not [AX,BX]

The port's body is FAITHFUL: it pops before the second call, exactly as the game
does. Only the description was wrong.

WHY THAT IS WORTH AN ENTRY. Every defect in this review so far has been a claim
unsupported by the binary. This is the inverse — correct code with a comment that
misdescribes it — and it is arguably more dangerous, because the comment provides
a REASON to change the code. A reader who believes both calls take `[AX,BX]` sees
`pop dx / pop cx` sitting between them as stray housekeeping and moves it after
the `int 33h` to tidy up. The tests would very likely still pass; the vertical
mouse range would silently start coming from a different place.

A wrong doc over wrong code gets found when the code is checked. A wrong doc over
RIGHT code only gets found by reading them against each other, which is the one
thing a green test suite never does.

652 citations verified (from 647), 0 wrong. 612 lib tests, 0 failures.

## #347 — two more lifted routines verify, including a De Morgan the lift got right

`func_d0e` (`poll_mouse`, `0x0D0E`) reproduces the routine exactly: `ax=3`,
`int 33h`, the three stores (`gs:[0xA2A]=cx`, `[0xA2C]=dx`, `[0xA2E]=bx`), the
latch update, and the four pops in order (`dx, cx, bx, ax`) before `retf`.

The part worth checking was the branch, because the game writes it as two jumps
with opposite senses:

    0x0D26  cmp cx, gs:[0xa38] / jne 0xd34    x differs -> UPDATE
    0x0D2D  cmp dx, gs:[0xa3a] / je  0xd45    both equal -> SKIP
    0x0D34  [0xa38]=cx ; [0xa3a]=dx ; [0xb3b]=0

i.e. update UNLESS both coordinates match the latch. The lift is
`if cx != last_x || dx != last_y { … }`, which is that same condition after De
Morgan, and its comment says so. A negated compound condition is where a
hand-lift usually goes wrong; this one is right.

`func_b32` (`detect_cdrom`, `0x0B32`) likewise: `mov ax,0x1500` / `xor bx,bx` /
`int 0x2f` / `or bx,bx` / `setne byte gs:[0xae6]` / near `ret`. Its doc even
notes the missing register preservation — a `near` helper that clobbers AX/BX —
which is the kind of detail that matters to a caller and is easy to omit.

Four io_lift rows checked across #346 and this: three correct in code AND prose,
one correct in code with wrong prose. The recompiler's lifts are in better shape
than the hand-written subsystems this review has been finding defects in, which
is what you would expect from code produced mechanically from the instructions
rather than reconstructed from behaviour.

2227 items, 1053 confirmed (47.3%). `ASM?` down to 57.
652 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #348 — two more lifts verify, including the word-`out` the project warns about

`func_2dd3` (`cmos_rtc_read`, `0x2DD3`): `push ax` / `xor ax,ax` /
`out 0x70,al` (select CMOS register 0) / `in al,0x71` / `mov ah,al` /
`mov word cs:[0xaee],ax` / `pop ax` / `retf`. The doc's two easily-omitted
details are both there — AL is DUPLICATED into AH before the store, and AX is
preserved across the call.

`func_17af` (`page_offset_helper`, `0x17AF`): for each of `[0x5219]` and
`[0x521D]`, `or ax,ax / js` clamps a negative to zero, otherwise `add ax,0x4000`
adds the VGA page size; then

    0x17D1  mov dx, word ptr [0xa9e]   the CRTC base
    0x17D5  mov al, 0xc                start-address-high index
    0x17D7  out dx, ax                 <- a WORD out
    0x17D8  ret                        near

The `out dx, ax` is the detail worth confirming: a 16-bit `out` to a CRTC index
port writes index in AL and DATA in AH in one instruction, and this project's
own notes list "word OUT" as a gotcha that bit the runtime earlier. The lift and
its doc both have it right, including that AH carries the high byte of the value
just stored to `[0x521D]`.

That is six `io_lift` rows examined (#346, #347, #348) with one prose defect and
no code defect. I am treating that as a reason to keep batching this module
rather than a reason to stop checking it — the sample is small, and #346's defect
was in the one place a mechanical lift cannot help: the sentence a human wrote
above it.

652 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #349 — the lift was right and its LABEL was wrong twice

`func_7ea` (`0x07EA`) verifies against the binary: `cli`, `mov al,0x36 /
out 0x43,al`, `mov al,0xff` then `out 0x40,al` TWICE, `mov byte gs:[0xb21],0`,
`sti`, then the saved INT 08h vector restored from `gs:[0xB1D]`/`[0xB1F]`.

`re/labels.csv` described the same routine as: "cli; out 0x43,0x36 (PIT ch0 mode
3 square wave) + out 0x42/al=0xff divisor. Reprograms the 8253 PIT tick rate for
the game's timing (faster than the default 18.2Hz)."

TWO ERRORS.

  * THE PORT IS 0x40, NOT 0x42. Raw bytes at `0x07F4`/`0x07F6` are `e6 40 e6 40`.
    `0x40` is PIT CHANNEL 0, the system timer; `0x42` is CHANNEL 2, the PC
    SPEAKER. A reader chasing sound would land here and find nothing, or worse
    would wire a speaker behaviour to a timer routine.
  * IT IS A RESTORE, NOT A SPEED-UP. Writing `0xFF` as both divisor halves gives
    `0xFFFF` — the LARGEST divisor, hence the SLOWEST rate, which is the stock
    ~18.2 Hz. The label says "faster than the default 18.2Hz"; the instruction
    does the opposite. It is the teardown counterpart of `0x079C`, which is what
    the port's own doc calls it.

THE PORT HAD BOTH RIGHT. `src/recomp/io_lift.rs` says "reprogram PIT channel 0
back to the default ~18.2 Hz" and "the teardown counterpart of func_79c". So the
Rust doc was more accurate than the RE label it was presumably derived from —
the reverse of the direction defects have travelled in this review so far, where
`labels.csv` was the authority and the source drifted (#344).

Label corrected with the raw bytes quoted, so the next reader can check it
without re-deriving. Row settled.

652 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #350 — teach the label checker about PORTS, after two self-inflicted false positives

#349 found `re/labels.csv` claiming `out 0x42` (the PC SPEAKER) over code doing
`out 0x40` (the system TIMER). `check_labels.py` could not have caught it twice
over: `out` and `in` were not in its mnemonic set at all, and even if they were,
it compares MNEMONICS — and `out 0x42,al` and `out 0x40,al` are both `out`.

Added `out`/`in` (plus `cli`, `sti`, `setne`, `movsx`, `stosd`, `lodsd`, …) and a
PORT-NUMBER check. Getting it right took two corrections, both mine:

  1. ADDING `out` BROKE A CORRECT LABEL. `ship_3d_plane_copy_mapmask_all`
     describes a PAIR — `mov ax,0xf02` @`0xB70E` then `out dx,ax` @`0xB711` — and
     opens its comment "out 0x3C4, ax=0x0F02", pointing at the VALUE LOAD, which
     is the instruction a reader needs. The opening-mnemonic check read that as a
     claim about the labelled address. `out`/`in` are now exempt from THAT check
     while still subject to the port check.
  2. PAIRING THE FIRST CLAIM WITH THE FIRST I/O INSTRUCTION broke two more.
     `program_pit` names both `0x43` (mode) and `0x40` (divisor); `cmos_rtc_read`
     names both `0x70` (select) and `0x71` (read). A label naming several ports
     is normal, so the check is now SET MEMBERSHIP: every port a comment names
     must appear among the ports the routine's I/O actually uses, scanning to the
     `ret`.

VERIFIED BY PERTURBATION, because a checker reporting zero problems may be
running zero checks — my own tool did exactly that in #290. Re-introducing #349's
error gives:

    PROBLEM 357: program_pit names port 0x42 but the routine's I/O uses {0x40, 0x43}

Restored; 501 code labels and 238 data labels check clean.

The two false positives are the point worth keeping: both came from assuming a
label describes ONE instruction at ONE address. Labels describe ROUTINES, and a
routine has several ports, several instructions, and a natural way of naming its
effect rather than its opcodes.

652 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #351 — checking INT numbers found two labels describing the wrong API entirely

#350 added port checking. Interrupt numbers are the same class of operand claim,
so they went in too. First run: four hits, two of them real and substantial.

`0x27E9` was `dos_char_out`: "if [0xae0]&1, ah=0x0e; int 10h (BIOS teletype).
Outputs a character to the screen". The code:

    0x27F7  mov ah, 0x0e
    0x27F9  mov dl, byte ptr [0x1b9]
    0x27FD  int 0x21                  <- INT 21h, not 10h
    0x27FF  mov dx, 0x1da
    0x2802  mov ah, 0x3b              <- DOS CHDIR

`AH=0x0E` under INT 10h is teletype output; under INT 21h it is SELECT DEFAULT
DRIVE, with DL the drive number. Then `AH=0x3B` changes directory to the path at
`DS:0x1DA`. The routine is LAUNCH-PATH SETUP — which fits the game's own
`WRIC:\\cblood\\` argument — and the label had its interrupt, its function, its
purpose and its name all wrong. Renamed `dos_set_drive_and_chdir`.

`0xBD26` was `dos_ioctl_device`: "ax=0x4400; int21h (IOCTL get/set device info).
Configures the sound/CD device driver". The code does `mov ax,0x4400` and then
`int 0x67` — the EMS entry point, where `AH=0x44` is MAP HANDLE PAGE. It is
followed by `mov cx,0x1000 / rep movsd`, copying 16384 bytes: exactly one EMS
page. Renamed `ems_map_page_and_copy`.

BOTH ERRORS HAVE THE SAME SHAPE: a function number that is valid under TWO
interrupts, and the label picked the wrong one. `AH=0x0E` and `AH=0x44` each mean
something plausible under both `int 21h` and the interrupt actually issued, so
the descriptions read as competent and were checkable only against the operand.

TWO FALSE POSITIVES FIRST, both suppressed. A label naming `int 08h` while
issuing only `int 21h` is usually CORRECT — it is installing or restoring that
VECTOR through DOS function 25h/35h. Detecting that needed both encodings:
`mov ah,0x25` and `mov ax,0x2508`, the latter packing function and vector into
one word, which my first regex missed and which flagged `program_pit`.

AND A LESSON ABOUT MY OWN CORRECTIONS: both replacement labels initially named
the interrupt they were explaining the routine does NOT use ("looks like DOS
IOCTL (INT 21h AH=44h)"), and the checker flagged them — correctly, by its own
rule. Reworded to state only what each routine does. A label is not the place to
argue against a previous reading; that belongs here.

501 code labels and 238 data labels clean, 17 inline claims verified.
612 lib tests, 0 failures.

## #352 — did the two wrong labels reach the code? No.

#351 renamed two routines whose labels named the wrong interrupt. The question
that matters more than the labels is whether either misreading PROPAGATED — a
wrong description is cheap; a port built on one is not.

`0x27E9` (`dos_char_out` -> `dos_set_drive_and_chdir`): the only `0x27e9` hits in
`src/` are in `recomp/auto.rs`, and they are `test byte ptr [0x27e9], 1` /
`mov byte ptr [0x27e9], 0` — a DS DATA CELL that happens to share the number with
the code offset. Different address space, no relation. Nothing in the port
implements a "char output" routine here.

`0xBD26` (`dos_ioctl_device` -> `ems_map_page_and_copy`): the one `ioctl` in the
runtime is `0x44 =>` inside the INT 21h dispatcher — genuine DOS IOCTL, correctly
placed. And the actual behaviour the routine needs IS implemented: `fn int67`
carries an EMS handle table, logical-to-physical page mapping and `ems_unmap`, so
`int 67h AH=44h` is served properly.

So the port had the mechanism right while the RE notes described a different API
entirely. That is the same direction as #349, where `io_lift.rs` was more accurate
than the label it was derived from, and the opposite of #344, where the label was
right and the source had drifted. Neither artefact is reliably the authority —
only the bytes are, which is why both now get checked mechanically.

WHAT THIS COST: nothing in the shipped port. What it would have cost is a future
reader implementing a "sound/CD device driver configuration" that does not exist,
or looking for console output in a routine that changes directory. Both plausible
enough to have happened, which is why #350/#351's checks are worth the two false
positives each took to tune.

501 code + 238 data labels clean. 612 lib tests, 0 failures.

## #353 — ten constants settled, every one checked at its own instruction

A batch from the `ASM?` queue, all in `src/vm.rs`. Six were constants I created
earlier in this session and re-checked rather than trusted (my own citation is
not evidence — #329's rule):

    DLG_ASSET_NAME_TABLE_BASE  add ax, 0xdd7                @0x7691   (#318)
    UI_FLAG_CE_BRANCH          test byte gs:[0x2793], 1     @0x6494   (#311)
    UI_FLAG_BUSY               or byte [0x2793], 4          @0x593A + 2 more
    UI_FLAG_SEEK_ARRIVED       xor word [0x2793], 8         @0x9671   (#330)
    UI_FLAG_DEFER_MASK         test byte [0x2793], 0xe      @0x1095   (#311)
    MAIN_LOOP_BUSY_BYTES       derived from the OR sequence @0x109C   (#333)

Four were pre-existing and verified now:

    ENTITY_CANDIDATE_KIND_MASK     test bx, 0x98             @0x727E
    ENTITY_CANDIDATE_READY_BIT     test byte es:[di+2], 2    @0x7284
    SHIP_CLICK_LOCATION_KIND_MASK  test word es:[eax], 0x140 @0xB0FB
    OBJECT_FLAG_IN_PLAY            test byte fs:[bx+2], 2    @0x6073

All ten exact, no corrections needed. `NAV_CHART_KIND_MASK`'s `test bx,0x118`
@`0x723D` checked out too while I was in the neighbourhood, though that row was
already settled.

WORTH NOTING ABOUT THE SHAPE: every one of these is a MASK OR BIT tested by a
single instruction, and that is the easiest kind of claim to verify — one
address, one operand, no interpretation. The defects this review has found were
never in constants like these; they were in PROSE (#346), in ADDRESSES pointing
at the wrong routine (#298), in POLICY invented around a real fact (#302), and in
COUNTS derived from the wrong instrument (#335). A settled ledger row is worth
exactly as much as the difficulty of the claim it settles, and rows like these
are the cheap end.

2227 items, 1066 confirmed (47.9%). `ASM?` down to 44.
612 lib tests, 0 failures.

## #354 — the mixer verifies, and a constant that should never be ASM

`mix_unsigned_pcm_sources` cites `0xBB6D`. The routine is five instructions:

    0xBB6D  lodsb
    0xBB6E  add al, byte ptr es:[di]
    0xBB71  rcr al, 1
    0xBB73  stosb
    0xBB74  loop 0xbb6d

`rcr al,1` AFTER the `add` is the detail that makes it correct: the add's CARRY
becomes the rotated-in high bit, so this is a nine-bit sum halved — a true
average that cannot overflow. Writing it as `shr` would lose the top bit on every
sample above the midpoint.

The doc's harder claim also holds: mixing N sources is this applied N times, so
an earlier source is halved again by every later one and THE LAST SOURCE
DOMINATES. That is iterated `(a+b)/2`, not an equal-weight average, and the doc
explicitly warns against "correcting" it. Settled ASM.

`SILENCE = 0x80` settled INFRA, NOT ASM, and the distinction is the point. Its
own doc says: "DEFINITIONAL, not decoded... This constant carried an origin of
`0x4049,0xBB6D` — neither of which contains `0x80`; `0x4049` is `int 21h` and
`0xBB6D` is `lodsb`. The addresses had been absorbed from a TEST comment eighty
lines away by a scanning bug (#252)."

So the row LOOKED like a decode claim because a tool had glued two unrelated
addresses to it, and the honest status is "no binary counterpart" — `0x80` is the
midpoint of an unsigned 8-bit sample, which is what silence IS in that
representation. Settling it ASM would have manufactured exactly the evidence #252
was written to remove.

That is the second row this session settled as something other than ASM on
purpose (#322's `MENU_SUBMENU` went DATA). Having five settled statuses is only
useful if the one recorded matches the evidence that exists — otherwise the
ledger says "checked against the disassembly" about a definition.

2227 items, 1068 confirmed (48.0%). `ASM?` down to 42.
612 lib tests, 0 failures.

## #355 — the proximity gate tests THREE axes; the port tested two

Verifying `proximity_visible`'s citations turned up a missing check.

The overlay gate at `0xA30` filters an object on all three camera-relative axes,
each against a fixed window:

    0xA62  add ax, [0x22f0]              screen Y
    0xA66  js  0xaa0                     reject negative
    0xA68  cmp ax, 0x80  / jg 0xaa0      reject above 128
    0xA6D  mov ax, [si+0x42]
    0xA70  add ax, [0x22ec]              world X
    0xA74  cmp ax, 0xff00 / jl 0xaa0     reject below -256
    0xA79  cmp ax, 0x100  / jg 0xaa0     reject above +256
    0xA7E  mov ax, [si+0x4a]
    0xA81  add ax, [0x22f4]              world Z   <- NOT PORTED
    0xA85  cmp ax, 0xff00 / jl 0xaa0
    0xA8A  cmp ax, 0x100  / jg 0xaa0

The Y bound (128) and X bound (±256) match the port's constants exactly. The Z
test was absent — the port returned on X and never looked at `pos[2]`.

Everything needed was already there: `AlienCamera` stores `z_fixed` (`0x22F2`,
read as `[0x22F4]`) with a working `axis(2)`, and `pos` is `[i32; 3]`. Only the
check and a `z()` accessor were missing, both now added with the instructions
cited.

WHY NO TEST CAUGHT IT, which is the reusable part: `proximity_gate_advances_and_
windows_on_screen` exercised the state flag, the X window and the Y window — and
every one of those cases leaves `pos[2]` at ZERO. A missing axis is invisible to
any test that never moves along it. The new assertions push Z past the window in
BOTH directions and then back inside, so the check is pinned as a WINDOW rather
than a blanket rejection that would also pass.

This is the same shape as #313's subtitle wrap: a test whose cases were all
chosen from the behaviour the port already had, so the gap sat in the one
dimension nobody varied.

2227 items, 1069 confirmed (48.0%). 612 lib tests, 0 failures.
652 citations verified, 0 wrong.

## #356 — `+= 0xFA` was a shared counter, not a per-object accumulator

`AlienObject::step`'s doc said the state change does "`+0x3C += 0xFA`". Both
numbers in it are right — `0x32` is the timer reload (50) and `0xFA` is 250, and
the port's constants match. The STRUCTURE is not:

    0x16C2  movsx ebx, word ptr cs:[0x16a2]   a counter in the CODE segment
    0x16D8  mov dword ptr [di+0x3c], ebx      the object takes its CURRENT value
    0x16DC  add bx, 0xfa                      the SHARED counter advances
    0x16E0  mov word ptr cs:[0x16a2], bx

The `0xFA` steps a counter shared by every alien, and each object receives the
value that counter held when IT last changed state — stored as a dword. The port
adds 250 to a per-object field.

FOR ONE OBJECT THE TWO ARE INDISTINGUISHABLE, which is exactly why the tests
pass: `step()` is exercised on a single `AlienObject`, and a private accumulator
starting at the same place produces the same sequence. The divergence needs a
COLONY — the thing `AlienColony` exists to model — where the game interleaves one
sequence across objects and the port gives each its own.

Same failure mode as #355 one entry earlier: the test cases never varied the
dimension where the port and the game differ. There it was a spatial axis left at
zero; here it is the object COUNT left at one.

ALSO UNPORTED from the same block, now recorded rather than discovered later:
`+0x3A = 0`, a SECOND PRNG step landing in `+0x42` — the field the proximity gate
reads as the object's X, so the two routines are coupled through it — and
`[si+0x50] = ax & 0xFFC` / `[si+0x52] = 0`.

NOT FIXED: correcting it needs the colony's shared counter as real state, and
guessing at that is how #302 happened. Recorded as an APPROX row naming the
routine and the exact instructions.

656 citations verified (from 652), 0 wrong. 612 lib tests, 0 failures.

## #357 — a row that must NOT be settled, and a flag reader found while not settling it

`render_star_map_navview` sits in the `ASM?` queue with three citations. It is
not settleable, and its own doc says why: it is "an approximation... a fabricated
surface sitting beside the real one", reached "only from each other and from
tests", kept solely because its tests still exercise the pyramid/orb composition,
with removal as the stated end state.

Settling it ASM would record "checked against the disassembly" for a surface the
game does not draw. Left provisional — the correct outcome for a row whose
subject is honest about being wrong.

BUT ONE CITATION PAID OFF. `0xB193` is `test word ptr [0x2793], 8` — a DIRECT
READER of UI-flag bit 3, the station-seek arrival bit from #330. Until now bit 3
was known only as something WRITTEN (`xor word [0x2793],8` @`0x9671`) and read as
part of the gate's composite `0xE` mask at `0x1095`. A dedicated `test ...,8`
means something branches on it alone, so it is a flag in its own right rather
than an accumulator bit that only matters in aggregate. Added to the constant.

THE PATTERN ACROSS THIS SESSION'S FLAG WORK: every time I have gone looking at
`0x2793` for one reason, another site has turned up — three setters of bit 2
(#311, #324, #325), a third clear site (#337), and now a solo reader of bit 3.
`find_imm`'s aggregate said "at least six live bits" back in #309; the individual
sites have been arriving one investigation at a time ever since, which is a fair
warning that the flag word is not finished being decoded.

656 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #358 — finish the flag word: 66 sites, and the encoding family I omitted first

#357 warned that `gs:0x2793`'s sites had been arriving one investigation at a
time and the word was not finished. So I enumerated all of them at once, by
searching every fixed ENCODING rather than by decoding (no false negatives —
#334's lesson).

FIRST PASS: 59 sites, and bit 3 showed SIX READS AND NO WRITER. That is not a
possible state for a flag something branches on, so the census was wrong before
its shape could mislead me.

THE OMISSION: `xor word ptr [0x2793], 8` @`0x9671` is `83 36`, the
SIGN-EXTENDED-IMM8 family (`83 /N` on a word operand), and I had only searched
`81 /N` and the byte forms. Adding `83 /0,/1,/4,/6,/7` brought in seven more
sites — including bit 3's two `or`s and its `xor`, and three more bit-2 clears.

That is the THIRD time this session an encoding family has been missed while
enumerating encodings (#336's `89 /r`, #335's `a3`, now `83 /N`), and the second
time by me AFTER writing the rule that a set-search must state what it omits.
The rule is right and evidently not sufficient on its own; what caught it here
was a RESULT THAT COULD NOT BE TRUE — reads with no writer — rather than any
discipline about listing forms.

THE COMPLETE CENSUS is now on the constant: 66 immediate-form sites plus
`mov [0x2793],ax` @`0x9544` and two `mov ax,[0x2793]` reads, with `89 /r` and
`8b /r` confirmed absent. Two facts fall out that change what the port must model:

  * BIT 1 IS NEVER REFERENCED ALONE — only inside the `0x0E` gate mask. Nothing
    sets or tests it individually, so it needs no separate representation.
  * BITS 4..7 ARE READ BUT NEVER OR'd. They can only be set by one of the six
    whole-word writes, so their meaning is fixed by those writers rather than by
    incremental flag-setting — a different modelling problem from bits 2 and 3.

`0x000C` is also new: one site ORs bits 2 AND 3 together, which no reading of
them as independent flags would have predicted.

658 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #359 — build the encoding table once, then find two bugs in it

Three misses of the same kind (#335 `a3`, #336 `89 /r`, #358 `83 /N`) mean the
fix is not another reminder. `re/tools/addr_forms.py` is the table: every fixed
x86-16 encoding that references a DIRECT address — `80/81/83 /N` for all eight
ALU ops, `C6/C7`, `F6/F7`, `88/89/8A/8B /r` for all eight registers, the `A0..A3`
accumulator short forms, and `FF /N` — each with a `65`-prefix option so a
GS-prefixed instruction reports as ONE site.

It found #358's census exactly: 69 sites for `0x2793`, and independently 29
writers for `0x6788`, the number #335 derived by hand after `find_imm` said 27.
Two instruments agreeing from different directions is the point of building it.

IT ALSO HAD TWO BUGS, both mine, both instructive.

FIRST, I NAMED IT `encodings.py`. That shadows a Python STDLIB PACKAGE which is
imported during interpreter bootstrap, so it can never be overridden by a path
insert — importing it from any other script gets the stdlib. This repo already
documents exactly this hazard for `re/tools/dis.py` versus stdlib `dis`, in
`re/CLAUDE.md`, in a note I have read several times this session. Renamed
`addr_forms.py`.

SECOND, AND WORSE: modrm for reg=5 is `0x2E`, and `0x2E` as a byte IS THE REGEX
WILDCARD `.`. The `sub` pattern therefore matched EVERY modrm, so `or`, `and` and
`xor` sites were being reclassified as `sub` — silently, with the totals still
right. The census printed 19 sites at value `0x0004` as writes rather than SETs,
which looked plausible enough to publish.

What caught it was cross-checking against #358's hand census and seeing the KINDS
disagree while the COUNTS matched. A bug that preserves totals and corrupts
classification is invisible to any check that only counts — and counting is what
every one of these tools does by default.

Fixed with `re.escape` on the modrm AND the address bytes, since an address
containing `0x2A`, `0x2B`, `0x3F`, `0x5B`, `0x5C`, `0x7C`, `0x5E` or `0x24` would
have the same effect for any caller.

658 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #360 — the encoding table earns itself: setters for all ten gate flags

#332 ended with "find what SETS each of the ten", and #334 showed why that had
been hard: `find_imm` rejects real instructions, and one of the ones it rejected
was `mov byte [0x2737],1` @`0x893C` — a setter for one of these very flags.

With `addr_forms.py` (#359) the census is immediate. Every flag's set-to-1 site:

    0x67AC  0x5904                 0x2565  0x86C1
    0x24F3  0x8160                 0x2736  0x892C
    0x2751  0x8836                 0x2737  0x893C   <- what #334 rejected
    0x67B0  0x122C, 0x677F         0x27DA  0x7FF5, 0x8A62
    0x5E64  0x673D, 0x761B         0x2792  NONE

`0x2751`'s `0x8836` and `0x2736`'s `0x892C` match the addresses already in the
port's own constant docs, which is the corroboration worth having: the tool
agrees with citations derived independently by decoding forward from verified
entries.

`0x2792` IS GENUINELY DIFFERENT, and only a complete census could say so. Five
sites: two `mov byte [m],0` clears, and three reads comparing it to 0, to 1, and
testing bit 1. NOTHING sets it non-zero, and its baked value in the image is
`0x00`. So its live state must come through the save/load block restore, or the
branches reading it are dead. That is a real, narrow question — and the kind that
an incomplete search would have answered "no setter found, look harder".

WHAT THIS UNBLOCKS: #312 declined to port the busy gate because nine of ten flags
had no writer in the port. The BINARY side of that is now answered for nine of
them; what remains is mapping each setter's routine to a port event, which is
ordinary decode work rather than a search problem.

658 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #361 — from "which addresses" to "which subsystems": the gate's wiring table

#360 located every setter. Placing each inside its enclosing labelled routine
turns a list of addresses into something a port can act on:

    0x67AC  presentation_scan                presentation start
    0x24F3  nav_actor_handler_2              nav actor
    0x2751  nav_choice_handler_2             nav choice
    0x67B0  dlg_line_activate  AND  vm_a6_accept_clears_active_bit
    0x5E64  vm_a6_accept_clears_active_bit  AND  index_lookup_dca
    0x2565  console_menu_hit_test            console hit test
    0x2736  console_mode_dismiss_ladder      console mode, arm 0
    0x2737  console_mode_dismiss_ladder      console mode, arm 1
    0x27DA  nav_actor_handler_0  AND  camera_fsm_state_gate
    0x2792  none (#360)

So the "ten subsystem-active flags" are exactly what the name promised —
presentation, nav, dialogue, text accept, console menu, console mode, camera —
and the port already models every one of those as distinct state.

THREE FLAGS HAVE TWO RAISERS EACH, in different subsystems. `0x67B0` is raised by
dialogue-line activation AND by the 0xA6 accept; `0x5E64` by that same accept AND
by the asset index lookup; `0x27DA` by a nav actor handler AND the camera FSM. So
these are not one-flag-per-event, and a port wiring that assumed otherwise would
clear a flag one subsystem still needs.

WHAT REMAINS is no longer a search: it is whether each port-side event fires at
the same MOMENT as the instruction that raises the flag. That is a per-row
question with a named routine on each side — the difference between #312's
"the port does not model these" and a checklist.

This is the fourth entry in a row where the answer came from `addr_forms.py`
rather than from decoding: #358 (census), #359 (the table itself), #360
(setters), #361 (routines). The instrument was the bottleneck, not the binary.

658 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #362 — first row of the wiring table checked, and it holds

#361 reduced the gate work to a per-row question: does each port event fire at
the same MOMENT as the instruction raising its flag? Starting with the row the
VM itself owns.

`0x5904` — `mov byte ptr gs:[0x67ac], 1` — is inside the presentation-START
block:

    0x58F8  mov byte ptr [0x5b55], 1
    0x58FD  mov word ptr gs:[0xa32], 1
    0x5904  mov byte ptr gs:[0x67ac], 1
    0x590A  xor ax,ax, then clears 0x6782 0x6784 0x6776 0x67F8
            0x2A19 0x67BA 0x27D7 0x67BC

That is the block #306 catalogued when it found `start_actor_presentation`
models only part of it. The port sets `presentation_active` in that same
function, so for this flag the modelled state and the game's instruction
coincide. Row checked.

A DETAIL THAT CORROBORATES TWO EARLIER ENTRIES: `0x2A19` is cleared in this
block. #332 concluded `INPUT_GATE_I` belongs to the flag family but is NOT read
by the main-loop gate; #337 found the console-dismiss tail clearing it. Here the
presentation start clears it as well — three independent sites treating it as
family state, none of them the gate. The naming was misleading and the grouping
was right, which is what #332 argued.

NINE ROWS REMAIN, plus `0x2792`'s missing writer (#360). Each is now a
concrete comparison between one named routine and one named port function, which
is the form this work needed and did not have four entries ago.

658 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #363 — bit 3 was named backwards, and finding its SETTER is what showed it

Checking the wiring table's `0x2565` row led to the console/nav-choice open at
`0x86B4`:

    0x86B6  or  byte ptr [0x2793], 0xc     bits 2 AND 3 together
    0x86BB  mov word ptr [0x279b], 0x5a    the SEEK TARGET ARC = 90
    0x86C1  mov byte ptr [0x2565], 1       the gate flag
    0x86C6  mov al,bl / dec al / mov cl,0x12 / mul cl / add ax,0x50
    0x86D1  mov word ptr [0x253f], ax      SHIP_3D_NAV_CHOICE_TARGET_Y
    0x86D9  mov word ptr [0xac6], 0x64     the hub console anchor

Two things fall out.

FIRST, `0x000C` IS IDENTIFIED. #358's census found one site raising bits 2 and 3
together and could not say what it was. It is the console/nav-choice open.

SECOND, AND IT CORRECTS ME: I named bit 3 `UI_FLAG_SEEK_ARRIVED` in #331, from
#330's observation that station-seek ARRIVAL does `xor word [0x2793], 8`. That is
backwards. The bit is RAISED here, in the same instruction pair that arms the
seek by writing its target arc to `0x279B` — the cell `bridge.rs` halves into a
frame index at `0x9667`. Arrival then TOGGLES it, which clears it BECAUSE IT WAS
SET.

A toggle is only meaningful against a known prior state, and I had been reading
one in isolation. Renamed `UI_FLAG_SEEK_ACTIVE`, with both instructions cited.
The new reading also explains #357's direct `test word [0x2793], 8` @`0xB193`:
something branching on "a seek is running" is ordinary; branching on "a seek
arrived once" is not.

THE GENERAL POINT: #330, #331 and #357 all examined bit 3 and each added detail
without noticing the direction was wrong, because every one of them looked at a
READ or the TOGGLE. The error survived three passes and fell out immediately once
the SETTER was in hand — which is the same lesson as #302 and #315, arrived at
from the opposite side: find who writes it.

662 citations verified (from 658), 0 wrong. 612 lib tests, 0 failures.

## #364 — bits 4..7 are not flags at all, and bit 1 is a lock: the census could not see either

#358's census left two conclusions about `gs:0x2793`: "bit 1 is never referenced
alone" and "bits 4..7 are read but never OR'd, so they can only be set by one of
the whole-word writes". Both were wrong, and wrong for the SAME reason.

`0x9512` does everything through a REGISTER, so an address-form census is blind
to it:

    0x9512  mov ax, [0x2793] / and ax, 0xff0f   clear bits 4..7
    0x9518  test ax, 2 / jne 0x9544             skip if BIT 1 is set
    0x951D  mov bx, 1
    0x9520  mov dx, [0x2795]                    the panorama frame index
            frame <= 0x16 or > 0x9D -> bx = 1
            frame <= 0x43           -> bx = 2
            frame <= 0x70           -> bx = 4
            else                    -> bx = 8
    0x953F  shl bx, 4 / or ax, bx / mov [0x2793], ax

BITS 4..7 ARE A ONE-HOT QUADRANT. The panorama is 180 frames at 2°, and the
boundaries 22/67/112/157 cut it into four 90° sectors. So the high nibble is a
small integer describing WHERE THE BRIDGE VIEW IS POINTING, which is why it is
read with masks like `0x50` (sectors 4|6) and `0x90` (4|7) — those are direction
RANGES, not flag combinations. Nothing OR-sets them because nothing raises them;
they are recomputed from the frame every time.

BIT 1 IS THE LOCK on that recompute, tested as `test ax,2` after the load. #358
said it "appears only inside the `0x0E` gate mask" — true of every memory-form
encoding, and false of the flag, which has a dedicated reader working on a
register copy.

WHAT THIS SAYS ABOUT THE METHOD. `addr_forms.py` (#359) is exhaustive over
INSTRUCTIONS THAT NAME AN ADDRESS, and I have been treating it as exhaustive over
USES. It is not, and cannot be: `mov ax,[m]` followed by arithmetic on `ax` is
invisible to it by construction. Four entries (#358, #360, #361, #362) leaned on
that census, and only the two conclusions ABOUT UNSEEN WRITES were wrong — the
setter locations were fine, because setters do name their address.

So the tool's limit is precise rather than general: it finds every site that
NAMES the cell, and says nothing about what happens to a value once it is in a
register. That belongs in its docstring, and the `0x2793` doc now carries the
corrected reading with the register routine cited.

667 citations verified (from 662), 0 wrong. 612 lib tests, 0 failures.

## #365 — the quadrant gates the nav actors: what the high nibble is FOR

#364 decoded `gs:0x2793`'s high nibble as a one-hot bridge quadrant and left the
obvious question unanswered — what reads it. All eight readers, placed:

    0x7F9C  nav_actor_handler_0          test 0x10
    0x7EC0  nav_actor_handler_1          test 0x10
    0x813B  nav_actor_handler_2          test 0x90
    0x817E  nav_actor_handler_3          test 0x40
    0x81FB  nav_actor_handler_4          test 0x20
    0x8082  nav_actor_handler_5          test 0x10
    0x78D4  presentation_mode_dispatch   test 0x50, then 0x40

SIX OF THE EIGHT ARE A HANDLER'S FIRST INSTRUCTION. Each bridge nav actor is
gated on WHICH WAY THE PLAYER IS LOOKING: handler 3 acts only in quadrant 3,
handler 4 only in quadrant 2, handler 2 in either of quadrants 1 and 4, and three
handlers share quadrant 1. That is a real behavioural mechanism, and it explains
the mask shapes the census found — `0x50` and `0x90` are two-sector ranges, which
is a sensible thing to gate on and a strange thing for independent flags to be.

THE PORT HAS NO QUADRANT. It models the panorama frame and gates the bridge menu
on a frame RANGE (40..60), which sits inside quadrant 2 but is narrower and
separately decoded — so no divergence today, because the nav actor handlers are
not wired. Recorded as an APPROX row precisely so they are not wired without it:
a handler ported without its direction gate runs everywhere, which would look
like an actor that never stops rather than an obvious bug.

THE CHAIN THIS CLOSES: #331 modelled the flag word as state; #358 censused it;
#364 found the high nibble is computed, not flags; this says what computes it FOR.
Four entries to turn "a multi-bit word the port conflates" into "a view-direction
gate on six named handlers" — and the last two only became possible after #359
built an instrument whose limits I could state.

667 citations verified, 0 wrong. 612 lib tests, 0 failures.

## #366 — implement the quadrant, and test the boundaries rather than the middles

#365 recorded the bridge view quadrant as unmodelled. It is a PURE FUNCTION of
`BridgeView::frame`, which the port already has, so implementing it needs no new
state and cannot go stale:

    frame <= 0x16  or  frame > 0x9D   -> 1     the WRAP sector, both ends
    frame <= 0x43                     -> 2
    frame <= 0x70                     -> 4
    otherwise                         -> 8

Returned UNSHIFTED, because 1/2/4/8 is what the readers compare against once you
account for the `shl bx,4` — `test 0x10` is quadrant 1, `test 0x90` is quadrants
1|4. The shift is storage, not meaning.

THE TEST CHECKS BOUNDARIES, NOT MIDDLES, and that is the whole point. The ladder
is four comparisons mixing `jle` and `jg`, which invites four different
off-by-one variants — every one of which agrees with a test that samples sector
centres. So it asserts `0x16 -> 1` and `0x17 -> 2`, `0x43 -> 2` and `0x44 -> 4`,
`0x70 -> 4` and `0x71 -> 8`, `0x9D -> 8` and `0x9E -> 1`, plus that every frame
in 0..179 maps to exactly ONE bit and all four sectors are reachable.

That last pair of assertions is cheap insurance of a kind this session has needed
repeatedly: #355's missing Z axis and #356's single-object model both survived
because the tests never varied the dimension that mattered. "One-hot for every
input in range" is a property that cannot be satisfied by an accidentally-correct
implementation.

DOCUMENTED AS PURE: the accessor does not model bit 1's LOCK (`test ax,2 / jne`
@`0x9518` skips the recompute), because that only matters once something writes
the stored nibble. A caller needing the locked value must read the flag word.

678 citations verified (from 667), 0 wrong. 613 lib tests, 0 failures.

## #367 — the claim was right; the address was the guard, not the action

`subtitle_reveal_progress` claims a driver plays the `tb.snd` chatter (clip 0)
when the reveal completes, "the decoded one-chatter-per-completed-line behaviour
(@0x94BA)".

`0x94BA` plays nothing. It is the block's GUARD — `test byte [0x24f3],4 / jne`,
then two more tests on `[0x67BB]` and `[0x67BC]`, each jumping past the whole
thing. My first read of it therefore looked like an unsupported claim.

The action is three tests later:

    0x94B4  inc word ptr [0x5e58]        the reveal pointer this function returns
    0x94CF  mov byte ptr [0xcfb], 0      <- SELECT clip 0
    0x94D4  mov ax,[0xaca] / shl ax,2 / mov [0xb35], ax    the hold timer
    0x94DD  mov byte ptr [0x67bb], 1     latch: hold armed

`0xCFB` is the voice-clip selector — `vm.rs` already documents "accepting a line
sets `gs:[0xCFB]` (`0x66AF`), the clip picker gates on that". So the game SELECTS
clip 0 at completion and a driver plays it, which is exactly what the doc said.
Claim verified, citation moved from the guard to the instruction.

THIS IS #301's SHAPE INVERTED. There a doc cited its guard clause while the
formula sat twenty bytes away, and the citation was wrong. Here the same
mispointing hid a claim that was RIGHT — I nearly recorded it as unsupported
because the cited address did nothing resembling the description.

A citation that points at a guard is unhelpful in both directions: it fails to
support a true claim and fails to expose a false one. Worth checking the whole
block before concluding either way, which is cheap and I did not do it first.

683 citations verified (from 678), 0 wrong. 613 lib tests, 0 failures.

## #368 — an enum proved EXHAUSTIVE rather than merely consistent

`LocationPanelState` maps `gs:0x2788` to four variants. Its citations verify
individually — `test byte [0x2788],1` @`0x9087` (ZoomingOpen),
`test byte [0x2788],2` @`0x9125` (ZoomingShut) — and the sibling
`LocationInfoPanel` claims the draw scales by `bh = (3*[0x2789])/2+1`, which is
exactly `mul bh` / `mov bh,al` / `shr bh,1` / `inc bh` at `0x924D`..`0x9253`.

But verifying the READS only shows the variants are POSSIBLE. Censusing the
WRITES shows they are ALL of them:

    0x9043  mov byte ptr [0x2788], 1   start zooming OPEN
    0x922F  mov byte ptr [0x2788], 2   start zooming SHUT
    0x9120  mov byte ptr [0x2788], 0   open complete -> drawn
    0x9217  mov byte ptr [0x2788], 0   close complete -> idle

Four writes, three distinct values, and the enum has exactly those plus the
`Idle` the port names for "zero with no selection". No fifth state exists to have
been missed. That is a stronger result than "each cited instruction is real", and
it is only available because the write census is complete (#359).

ALSO CORRECTED: the doc said "`0x921C` clears both together". `0x921C` clears
`[0x27BF]`; `[0x2788]` is cleared five bytes earlier at `0x9217`. Adjacent
instructions, one citation — the same one-of-a-pair mispointing as #367, this
time harmless because both instructions are in the same breath.

WHAT I WOULD DO DIFFERENTLY: I verified the reads first and nearly settled on
them. For an enum or any closed set of states, the READS tell you the variants
are reachable and only the WRITES tell you the set is closed. Reading first is
backwards for this shape of claim.

689 citations verified (from 683), 0 wrong. 613 lib tests, 0 failures.

## #369 — the closure check does NOT transfer to a runtime vtable

#368 settled `LocationPanelState` by censusing WRITES to `gs:0x2788` and showing
the enum's variants are all of them. `AlienMethod` looked like the same shape —
an enum over a dispatch table at `fs:0x103A` — so I tried the same argument.

It does not apply, for two independent reasons:

  * THE TABLE IS NOT BAKED. `croolis.xdb` at file offset `0x103A` is sixteen
    bytes of zeros. Whatever populates the vtable does so at runtime, so there is
    nothing to read.
  * NOTHING NAMES THE ADDRESS. `addr_forms.py` finds ZERO direct references to
    `0x103A` in the overlay. That is consistent with the dispatch actually
    observed at `0x16A4`: `mov si,[di+0x16] / add si,0x5e / jmp word ptr [si+0xe]`
    — a per-object structure reached through a register, which #364 established
    the census cannot see by construction.

So `AlienMethod`'s individual variants are cited and verified (`0x1D27` the null
`ret`, `0x16A4` the state machine, `0xA30` the proximity gate — #343, #355), but
the SET is open: nothing available statically proves there is no fourth method.
Row stays provisional, and the reason is now recorded rather than rediscovered.

THE GENERAL POINT, which #368 did not state carefully enough: the write census
proves closure only for a FIXED CELL whose writers all name it. It proves nothing
about a table filled at runtime, or a dispatch reached through a register. Those
need a trace, not a search — and mistaking one for the other would have let me
declare an enum exhaustive on evidence that does not exist.

613 lib tests, 0 failures. 689 citations verified, 0 wrong.

## #370 — a tautology replaced, and a perturbation that perturbed nothing

`OPTION_BOX` is the OPTION choice box's single row. Its doc says the label is
"the game's own string, not a transcription: `DS:0x0174` (file `0x0D594`)". Both
check out — `0xD420 + 0x174 = 0xD594`, and the image there holds `CANCEL`.

THE TEST DID NOT CHECK THAT. It asserted

    assert_eq!(EngineState::OPTION_BOX_LABEL, "CANCEL");

which compares the constant to a second copy of the same transcription. It cannot
fail unless someone edits one side, and it proves nothing about the game — the
self-referential shape this project's own notes name as NOT evidence. The
constant records `OPTION_BOX_LABEL_FILE_OFFSET` precisely so the check can be
real; nothing was using it.

Replaced with a read of the image at that offset, plus an assertion that the DS
offset and the file offset agree (the doc states both, and only their consistency
makes either one checkable).

AND THEN I PERTURBED IT WRONG. My first perturbation edited
`OPTION_BOX_LABEL_FILE_OFFSET,` — with a trailing comma, which does not occur in
the new code. The string replace matched nothing, the file was unchanged, and the
suite passed. I nearly read that as "the check does not fire".

The second attempt compared against `PAUSE_TEXT` instead and failed properly:
`left: "CANCEL", right: "PAUSE"` — the image read genuinely produces CANCEL,
independently of the constant.

THAT IS #225's FAILURE REPEATED — a perturbation that perturbs nothing, then
reads as evidence. The defence is not care, it is checking that the perturbed
build actually DIFFERS: a perturbation which leaves the suite green should be
suspected of being a no-op before it is believed as a result.

613 lib tests, 0 failures. 689 citations verified, 0 wrong.

## #371 — the self-reference guard knew one tautology shape; now it knows two

#370 found `assert_eq!(EngineState::OPTION_BOX_LABEL, "CANCEL")` — a constant
compared to a copy of its own definition. `tools/check_selfref_asserts.py` exists
precisely to stop that, and could not have caught it: it matches `len() == CONST`
only, the shape that once hid a font table truncated from 176 entries to 128.

Added the second shape. It parses every `const NAME: &'static str = "..."` in the
tree, then flags any `assert_eq!(NAME, "...")` whose literal EQUALS that
definition. Cross-file, since a constant is often defined and asserted in
different modules.

TWO CALIBRATION PASSES, both against a deliberately re-introduced tautology
(#370's own lesson: a perturbation that changes nothing reads as a pass):

  1. The length rule clears an assertion when anything in the FILE is grounded.
     Reintroducing the tautology, the checker SAW it and reported it grounded,
     because `engine.rs` reads the image elsewhere.
  2. Narrowing to in-TEST grounding still cleared it — that test reads the image
     for another purpose entirely.

So the rule for this shape is UNCONDITIONAL. Grounding elsewhere does not rescue
`CONST == "its own value"`: the assertion is vacuous in itself, and the fix is to
replace it with a read of whatever the constant claims to come from — which is
what #370 did — not to leave it standing beside better evidence. The two shapes
need different rules, and treating them alike would have kept this one invisible.

Verified by re-introduction: the guard reports it, with the file, test name,
constant and the advice. Restored; the tree is clean at 8 length assertions and 0
tautologies.

613 lib tests, 0 failures. 689 citations verified, 0 wrong.

## #372 — the save UI is the slot list, and the doc proves it in three instructions

`SaveSlot` carries an unusually strong claim: THERE IS NO SEPARATE SAVE SCREEN.
The save flow reuses the ordinary ten-row list widget, with one row swapped for
an edit buffer while it is being typed into. Three citations, all exact:

    0x1BAB  mov word ptr [0x2734], ax   the record being renamed
    0x1BBD  rep movsd (cx=4)            16 name bytes -> edit buffer DS:0x273B
    0x8573  cmp si, word ptr [0x2734]   the widget, mid-draw...
    0x8577  jne 0x857c
    0x8579  mov si, 0x273b              ...substitutes the buffer for THAT row

The substitution is the whole argument: a widget that swaps one row's source
pointer while drawing cannot be a different screen, it is the same list. That is
the kind of claim worth checking precisely because it is a NEGATIVE — "there is
no save screen" — and negatives are usually asserted from absence. This one is
asserted from a `cmp`/`jne`/`mov` triple that could not exist if the claim were
false.

Also settled: `croolis::z`, the accessor added in #355, cited to
`add ax, word ptr [0x22f4]` @`0xA81`.

Ledger: 2229 items, 1075 confirmed (48.2%). `ASM?` down to 38 from 200 at
#317's reclassification — though the honest comparison is 38 from the 76 that
were genuine decode claims once `CELL?` was split out.

613 lib tests, 0 failures. 689 citations verified, 0 wrong.

## #373 — stop typing the status line; generate it

Four wrong numbers in one session, all in the summary line: #295 (585 for 583),
#320 (617 for 613), #329 (617 for 620), #372 (1076 for 1075). Two were caught
before commit, two after.

#319 already drew the right conclusion for INSTRUCTION counts — "run the tool,
paste the number, do not predict it" — and I kept not applying it here, because
the summary reads like prose rather than a measurement. It is a measurement.

`tools/audit_status.py` prints it:

    2229 items, 1075 confirmed (48.2%), 1154 open
    (901 UNVERIFIED + 253 provisional). 689 citations verified, 0 wrong.

It also encodes the counting rule so the STRICT reading cannot drift: a status
ending in `?` counts as OPEN, per #286a. The lenient alternative — counting the
253 provisional rows as settled — reads about eleven points higher, and the whole
point of #286a was that the flattering number is one `Counter` away at any moment.
Now it is not reachable by accident.

THE PATTERN THIS CLOSES is the one #359 named for encodings: when the same
mistake recurs, the fix is not a firmer rule but removing the step where the
mistake happens. Four repetitions is more than enough evidence that "be careful
with the number" does not work on me.

Per-status breakdown, for the record: ASM 450, ORACLE 236, TESTED 202, INFRA 97,
DATA 90 settled; UNVERIFIED 901, CELL? 121, DATA? 45, ORACLE? 41, ASM? 38,
INFRA? 8 open.

613 lib tests, 0 failures.

## #374 — the list menu verifies across four entries' worth of citations

`draw_list_menu` and its `TEXT` constant claim the widget's whole layout. Every
piece now checked, several of them in earlier entries:

    add bp, 0xB              @0x847A   11px row pitch          (#321)
    mov al, 0xE8             @0x8565   idle text index         (here)
    mov al, 0xEF             @0x858B   selected row            (#321)
    mov al, 0xFE             @0x8595   selected + hovered      (#321)
    add dx, 0x14             @0x84A1   w = widest + 20         (#314)
    shr dx,1 / sub [0xac6] / neg dx    x0 = anchor - w/2       (#314)
    sub bx,[bp] / shr bx,1 / add bx,cx @0x857D  label centring (here)

The centring is the piece that had not been checked: `label_x = x0 + (widest -
width)/2`, which is `sub bx,[bp]` (widest minus this label's width), `shr bx,1`,
`add bx,cx`. Three instructions, exactly as documented.

WHAT MAKES THIS ONE UNREMARKABLE IS THE POINT. Seven claims, seven matching
instruction sequences, no corrections. The widget documentation was written from
the assembly and stayed true to it — which is what the majority of this review
has found, and worth stating plainly given how many entries here are about
defects. The running tally across the whole review is roughly two rows correct
for every one needing a fix.

2229 items, 1077 confirmed (48.3%), 1152 open (901 UNVERIFIED + 251 provisional).
689 citations verified, 0 wrong. 613 lib tests, 0 failures.

## #375 — settling my own work, checked rather than trusted

Four rows from the `ASM?` queue, all functions written earlier in this session.
The rule from #329 applies with force here: a citation being MINE is not
evidence, and three sessions' worth of entries in this file exist because a
plausible-looking citation was wrong.

`branch` cites only `vm_branch` @`0x6462`, and the routine is six instructions:

    0x6463  sub word ptr gs:[0x6884], 2      the stack pointer, POP
    0x6469  mov ax, word ptr gs:[0x6884]
    0x646D  mov bp, ax
    0x646F  mov si, word ptr [bp + 0x6820]   the resume position
    0x6473  mov byte ptr gs:[0x67ad], 0      clear query mode
    0x647A  ret

which is exactly "pop the resume position into PC; clear query mode". The port
models the `0x6884` pointer and `0x6820` array as a `Vec` — an abstraction, but a
faithful one: the observable behaviour is a LIFO of resume positions.

`refresh_nav_source_scratch` (#294) cites the terminator write, and
`mov word ptr [bp], 0xffff` @`0x6289` is there. `c1_position_records` (#292) and
`derive_ship_3d_position_runtime` (#291) carry the record-walk citations checked
when they were written — `mov ax,[si]` @`0x61AB`, `vm_field_offset` @`0x6023`,
`cmp si,-1` @`0x61CD` — and re-read now rather than taken on trust.

2229 items, 1081 confirmed (48.5%), 1148 open (901 UNVERIFIED + 247 provisional).
689 citations verified, 0 wrong. 613 lib tests, 0 failures.

## #376 — a "verified by enumerating every site" claim, refuted by enumerating every site

`c4_set_write_decision` reads object active bits from VAR-initial data, and
justified it with the strongest kind of statement in this tree: "verified 2026-07
by enumerating every `or/and byte [reg+2],imm` site in BLOODPRG.EXE — are NEVER
SET at runtime; the sole runtime writer is `0x5B8D`".

Its two instruction citations are exact (`test byte es:[di+2],1` @`0x6CC3`,
`mov word es:[bp],0xC4` @`0x6D01`). The ENUMERATION is not.

It searched `or/and byte [reg+2],imm` — the `80 /N` form. Repeating it across
`80`, `81` and `83` (word and sign-extended-imm8) gives NINE sites, three
touching bit 0:

    0x5B8D  and byte ptr [bx+2], 0xfe     clears bit 0   <- the one it found
    0x5233  or  word ptr [bx+2], 3        SETS bits 0|1
    0x52B5  and word ptr [bx+2], 0xfffc   clears bits 0|1

`0x5233` sits in object initialisation: `mov bx,[0xc02]` / write `+0` from
`gs:[0xA6A]` / `or word [bx+2],3` / `mov dword [bx+4],ebp`. Objects created there
are ACTIVATED AT RUNTIME, which is exactly what the claim denies.

THE OMISSION IS THE ONE I KEEP MAKING. #335 missed `a3`, #336 missed `89 /r`,
#358 missed `83 /N`, and #359 built a table so it would stop happening — for
DIRECT addresses. This claim is about `[reg+disp]` forms, which that table does
not cover, so the same family gap reappeared in someone else's enumeration and I
found it only because I had learned to check for it.

WHAT I DID NOT DO: conclude the port is wrong. Reading VAR-initial bits still
gives the right answer for objects this path never creates or re-activates —
that is now an ASSUMPTION where the doc claimed a proof, and the open question
(can the C4 flow observe such an object?) is recorded in
docs/port-validation.md rather than answered by guess.

2229 items, 1081 confirmed (48.5%), 1148 open. 695 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #377 — extend the encoding table to `[reg+disp]`, the shape #376 fell through

#359 built `addr_forms.py` so an enumeration could not silently omit a family
again. #376 then found exactly that omission in a pre-existing claim — and the
table could not have prevented it, because the claim was about `[reg+2]` and the
table only knows `mod=00, r/m=110`, the DIRECT form.

`reg_disp_forms(disp)` / `reg_disp_census(data, disp)` now cover the other shape:
`80`/`81`/`83` across every ALU op, all eight base registers, both displacement
widths, with the immediate captured so a caller can ask WHICH BITS a site
touches — the question that matters for a flag byte.

Run against `[reg+2]` it gives 12 sites and reproduces #376's three bit-0 writers
exactly (`0x5233` SET, `0x52B5` and `0x5B8D` CLR). The two extra hits are both
worth understanding, because they are what the tool CANNOT decide:

  * `0x847  adc word ptr gs:[si+2], 0` — a REAL instruction, but semantically a
    32-bit carry propagation (`inc ax / mov gs:[si],ax / adc gs:[si+2],0`), not a
    flag write at all. It touches bit 0 only as arithmetic.
  * `0x67E4 cmp [bp+2], 0x6e` — a PHANTOM. Decoding from `0x67E0` gives
    `add [bx+di+0x75],bp / adc al,[bx+si+0x27e] / outsb`, so those bytes are
    mid-instruction.

So the census answers "which bytes could encode a write here", and a human still
decides which are instructions and which are flag operations. That is the same
division of labour #364 recorded for the direct table, and stating it twice is
deliberate: the tool's value is that it CANNOT miss a form, not that it
understands what it finds.

2229 items, 1081 confirmed (48.5%), 1148 open. 695 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #378 — withdrawing #376: two of the three writers are a different structure

#376 refuted a "never set at runtime" claim by finding three `[reg+2]` sites that
touch bit 0 where the original enumeration found one. The extra sites are real
and the original enumeration WAS incomplete. The refutation is still wrong.

Placing each site inside its routine — the step #376 skipped:

    0x5233  or  word [bx+2], 3       resource_name_write_c00 (0x5190)
    0x52B5  and word [bx+2], 0xfffc  resource_free_inner     (0x529C)
    0x5B8D  and byte [bx+2], 0xfe    obj_active_bit_sole_runtime_clear

Two of the three operate on the RESOURCE DESCRIPTOR area at `FS:0xC00` — `bx`
comes from `[0xC02]`, and the routine's own label says it "writes into the
FS:0xc00 resource descriptor area". Their `+2` is a resource flag word that
happens to share an offset with an object record's. So `0x5B8D` IS the sole
runtime writer of an object's active bit, the original claim holds, and
`c4_set_write_decision`'s use of VAR-initial bits is justified.

THIS IS #328's ERROR, THIRD TIME. In #328 I found a routine that did the right
thing and put it in the story without checking it was on the path. In #340 I
solved a pointer trap and answered the wrong question with it. Here I enumerated
correctly and attributed every hit to one structure without asking what the base
register held. Every time: the individual facts were right and the CONNECTION was
assumed.

The check that would have caught all three is the same one, and it is cheap:
BEFORE concluding, place the address inside its routine and ask what the pointer
points at. I have a script for the first half (nearest preceding label) and used
it four entries ago in #361. I did not use it here because the encoding search
felt like the whole job.

Both records are kept — the refutation and its withdrawal — because #376's
finding about the ENUMERATION'S METHOD is real and worth keeping even though its
conclusion was not.

2229 items, 1081 confirmed (48.5%), 1148 open. 698 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #379 — make the step I keep skipping into the first line of a tool

Three wrong conclusions this session from the same omission: #328 attributed an
off-path routine, #340 answered the wrong question with a solved pointer trap,
#376 attributed resource-descriptor writes to object records. Every time the
instructions were read correctly and attached to the wrong SUBJECT, and every
time the missing step was: PLACE THE ADDRESS INSIDE ITS ROUTINE.

`whatis.py` was the obvious home for that and did not do it. Given `0x5233` it
searched the address as TEXT and returned three labels that CITE `0x5233` — as a
DS offset naming a ring-buffer end, an unrelated meaning that happens to share
the number. Useful, and not the question.

It now leads with the enclosing routine:

    ENCLOSING: resource_name_write_c00 at 0x05190 (+0xa3) -- resource name-table
    write: ds=es=fs; [0xc00]=ax; bx=ax. Writes into the FS:0xc00 resource
    descriptor area...

which is the sentence that would have stopped #376 before it was written. Checked
against the other two failures as well: `0x2760` reports `string_upcase_in_place`
(#328's off-path fold) and `0x88BA` reports `console_menu_pick_dispatch` (#338's
call site).

THE PATTERN, stated once more because it is now four tools deep: #359 built the
encoding table after three missed families; #373 generated the status line after
four wrong numbers; this adds the enclosing-routine lookup after three wrong
attributions. In each case the rule was already known and written down, and
writing it down did not work. Making the tool answer the question first does.

WHAT IT STILL CANNOT DO: say what a base register points at. `bx` came from
`[0xC02]` and only the routine's own comment revealed that was a resource
descriptor. The enclosing label puts that comment in front of the reader, which
is as close as a static tool gets.

2229 items, 1081 confirmed (48.5%), 1148 open. 698 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #380 — the enclosing check confirms an attribution, and a doc argues itself open

Two rows, using #379's new `ENCLOSING:` line as the first step rather than the
last.

`SHIP_3D_TARGET_NAME_TO_RECORD = 4` claims one constant used in both directions:
`add ax,4` when a menu is built, `sub ax,4` when a row is selected. Both
instructions are there — and the enclosing lookup is what makes the CLAIM
checkable rather than just the addresses:

    0x87D5  add ax, 4   inside nav_choice_handler_2 ("CONSOLE ROW 2 = THE
                        CONTACT MENU, built from live state")
    0xB33D  sub ax, 4   inside ship_3d_target_record_select

A menu builder and a target selector, exactly as the doc frames them. Settled.
This is the first time the #379 check has CONFIRMED an attribution rather than
refuted one, which is worth noting: its value is not that it finds errors, it is
that the question gets asked at all.

`text_selector_voice_clip_index` STAYS OPEN, and its own doc is why. The scope
notes verify — `mov byte gs:[0xcfb], 1` @`0x66AF` is the 0xA6 accept setting the
flag, pairing with the `mov byte [0xcfb], 0` @`0x94CF` that #367 found clearing it
— but the doc then says plainly:

    STILL UNVERIFIED, and the reason these rows stay open: the game's mapping is
    `line_id = b3 + 9`, whereas this computes `b3 - 1`. Both derive from `b3`,
    but they are not the same function... Do not treat this as decoded.

A row whose doc argues against its own settlement should not be settled, and the
verified surroundings are not a reason to override it — the same judgement as
#357's fabricated star-map surface. Two rows examined, one settled, one correctly
left alone.

2229 items, 1082 confirmed (48.5%), 1147 open. 698 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #381 — a limit of the enclosing check, found by using it

Continuing the queue with #379's `ENCLOSING:` lookup as the first step.

`assemble_dialogue_from_offsets` (`script.rs`) cites `0x6701` and `0x672A`. Both
verify — `call 0x67a7` (strlen_b) and `add al,dl` — and they are the citations
I added in #313 when fixing the predictive subtitle wrap, re-read now rather than
trusted (#329's rule). Settled.

BUT THE ENCLOSING LOOKUP MISATTRIBUTED THEM, in a way worth recording. It reports
both as inside `vm_a6_accept_clears_active_bit` at `0x6693`, because that is the
nearest PRECEDING label. The instructions actually belong to the text-assembly
loop at `0x66CD`..`0x6739` — a distinct block that simply has no label of its own.

So "nearest preceding label" is a lower bound on specificity, not an answer. It
is still the right first question — it would have caught #328, #340 and #376,
where the true enclosing routine was a DIFFERENT one, not merely a finer-grained
one. But a reader who takes its output as the definitive owner will attribute
instructions to whatever was labelled last, which in a sparsely-labelled region
can be some distance away.

The honest reading of the tool's line is "no labelled routine begins between this
address and X", and the fix when that matters is to label the intervening block.
Not done here: the text-assembly loop deserves a label, but naming a region I
have only partly decoded is how misleading labels get written (#349, #351).

2229 items, 1083 confirmed (48.6%), 1146 open. 698 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #382 — the palette applier verifies; its label named only its prologue

`parse_palette_block` (`src/hnm.rs`) cited `0xA0E6 mul bh`. It verifies, and so
does the rest of the block loop, read at `0xA0D3..0xA0EC`:

    lodsw            al=start, ah=count
    cmp ax,-1        WORD compare -> terminator is BOTH bytes 0xFF
    di=0x5251+start*3    (bl=3, mul bl)
    mov al,bl / mul bh   cx = count*3 BYTES
    rep movsb

So `count==0` sets `cx=0` and copies ZERO entries — the port's earlier fix (drop
the "0 means 256" special case) is confirmed by the instruction, not by argument.
Two divergences now written down rather than left implicit: the game stores RAW
6-bit DAC values and the port's 6->8 expansion is its own; and the game has NO
`idx < 256` clamp, so a malformed `start+count > 256` runs past the buffer where
the port silently drops entries.

THE LABEL WAS WRONG, in the #349/#351 way. `0x00A0C3` was `draw_cleanup_set_dirty`
— "draw epilogue... Common draw-routine cleanup" — which describes its first FIVE
instructions. The routine does not end there: it falls straight through into the
palette loop and rets at `0xA116`. Its two near callers (`find_near_callers.py`)
are `0xA062` in `resource_switch` and `0xA780` in `list_d8c_init` — a RESOURCE
path, with no draw routine among them. Renamed `resource_palette_blocks_apply`,
with the tail (`0xA0EE..0xA115`, the `gs:0xDAF` adjust) marked NOT read line by
line rather than guessed at.

AND A CORRECTION I ALMOST WROTE INSTEAD. Three labels call `DS:0x5251` a 576-byte
/ 192-triple buffer; `live_palette` and #72 call it 768 bytes / 256 entries. I had
this queued as a CONFLICT entry before checking `0x8166`, where the bytes say
`si=0x5251, di=0x5b58, cx=0x90, rep movsd`. Both readings are right: the BUFFER is
768 bytes, the per-screen COPY is 192 entries, deliberately leaving 192..255 (the
console/text bank) untouched — which the `0x8166` label already states in full. I
had misread two correct labels as disagreeing. The check that caught it was the
same one this session keeps relying on: read the instruction before writing the
entry about it.

2229 items, 1084 confirmed (48.6%), 1145 open. 698 citations verified, 0 wrong.
502 code + 238 data labels clean. 613 lib tests, 0 failures.

## #383 — the confirm dialog verifies from two independent places

`ARE_YOU_SURE?` (`engine.rs`) cited `mov al,0xE8` @`0x14F7`. Read the whole
routine at `0x14E6..0x1528`; every documented value is exact — `bx=0x5A cx=0x50
dx=0x8C bp=0x28`, `lcall 0x299:0xCDC`, `si=0x17B/0x189/0x18D`, `add bx,0xA/0x14/
0x3C`, `dx=0x58` then `+0x11`, `bp=0x2555/0x255D`.

What makes this row worth more than a checkmark: the two derivations agree. The
CODE walks `bx` 90 -> 100 -> 120 -> 180 and `dx` 88 -> 105; the DATA at `DS:0x2555`
(dumped statically, `ds_dump.py`) reads (120,105,30,10) and (180,105,20,10). Drawn
text and clickable rect are the same layout arrived at from two unrelated places
in the image, and `DS:0x18D` really is `N`,`O`,0 — so none of the three strings is
a transcription.

ONE THING IN THE COMMENT WAS WRONG. It read "`mov al,0xE8` ... feeding the string
draw at `0x299:0xBB5`". `0xBB5` is not the string draw: it is called at `0x14F9`
with `al` alone and sets the COLOUR. The string draw is a different entry point,
`0x299:0x176`, called three times (`0x1507`, `0x1515`, `0x1520`) with si/bx/dx.
The behaviour was right and the attribution was not — the same shape as #382 and
#351, one call site attributed to a neighbouring one. Both the doc block and the
inline comment now name the two entry points separately, which also put the three
`lcall 0x299:0x176` lines under the citation guard (698 -> 702 checked).

Not a defect, but recorded: the box rect reads (x,y,w,h) rather than two corners,
because 90+140 and 80+40 bound every drawn item and the corner reading would put
y2 above y1.

2229 items, 1085 confirmed (48.7%), 1144 open. 702 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #384 — the choice box's row loop, and the phantom that hid its first byte

`draw_choice_box`'s three colours were marked ORACLE? even though the comment
already said ASSEMBLY-SOURCED. Read the loop end to end at `0x8565..0x85A6`; all
three verify, and so does the selection mechanism the comment only implied:

    0x8565  mov al,0xE8              unselected -- and this is the LOOP TOP
    0x8584  dec byte gs:[0x27C7]     the selected-row countdown
    0x8589  jne 0x8597               only the row that hits 0 recolours
    0x858B  mov al,0xEF              selected
    0x858D  test byte gs:[0xA3E],1   mouse-on-row
    0x8595  mov al,0xFE              selected AND moused
    0x8597  lcall 0x299:0x176        the string draw
    0x85A0  add dx,0xB               row pitch
    0x85A6  jmp 0x8565

Two things fall out for free. `add dx,0xB` is `CHOICE_BOX_PITCH = 11`, which was
sitting in the port as a bare constant. And `0x299:0x176` is the SAME string-draw
entry point the confirm dialog calls — independent confirmation of #383's
correction one entry later, from a routine 0x7000 bytes away.

THE PHANTOM. Disassembling from `0x8560` renders `0x8564: sal byte ptr [bx+si+
0x26e8], 0x8b`, which SWALLOWS the `b0 e8` at `0x8565` — the cited instruction
appears not to exist. That is the documented self-sync trap, and it would have
read as "citation wrong" to anyone who checked from a round address. Decoding
from `0x8565`, the verified entry, shows the real instruction immediately. The
comment now carries that warning inline, because the next person to check this
citation will pick a round number too.

2229 items, 1086 confirmed (48.7%), 1143 open. 702 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #385 — a content literal that survived by being unused

`OPTION_BOX_LABEL_FILE_OFFSET` settles cleanly: `DS:0x0174` / file `0x0D594` is
`C,A,N,C,E,L,0`, and `0x0D59B` is the `A` of `ARE_YOU_SURE?` — the same string
table #383 verified, so the two rows corroborate each other. Settled DATA.

Next to it sat `Engine::CONSOLE_MENU`:

    pub const CONSOLE_MENU: [&str; 5] = ["HONK","TELEPHONE","CRYOBOX","MENU","OPTION"];

documented as "baked into the golden menu of the TB.BIG panorama frames (verified
against the live capture)". That is a sourcing claim about PIXELS, which the prime
rule forbids — and the array had NO reference anywhere in the crate. Deleted. The
port's real console handling is index-based and cited: `selected_menu_item`
(`DS:0x2A19`) and `menu_row_under_cursor` (`0x8613..0x868D`) in src/bridge.rs,
where those names appear only in doc comments. 613 tests still pass, which is the
evidence nothing depended on it.

WHY IT SURVIVED, AND THE NEW GUARD. A `pub` item is exempt from rustc's dead_code
lint — the compiler assumes an external consumer, and this crate has none. So the
one category of content literal that can never be caught by a wrong pixel or a
failing test is precisely the one nothing consumes: it asserts game content, cites
a capture, and is checked against nothing forever.

`tools/check_dead_pub_consts.py` now asks that question directly — string-bearing
`pub const`s with no reference outside their declaration (DEAD) or references only
in `#[cfg(test)]` (TEST-ONLY, the self-referential shape the faithfulness memo
names). It reports 0 and 0. Since a checker that finds nothing proves nothing
(#370), it was perturbed: a planted dead string const IS flagged, a planted
numeric one is correctly ignored, and removing the probe returns it to zero.

2228 items, 1087 confirmed (48.8%), 1141 open. 702 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #386 — three labels sat inside an instruction, and a checker that finds them

Chasing `menu_submenu_labels`' documented APPROX (the `min_by_key` proxy) meant
finding what the console MENU click dispatches to. Decoded it: `nav_choice_dispatch`
(`0x85E2`) reads `[0x2A19]`, and if a row is selected jumps to `0x86F1`, which does
`dec bx / add bx,bx / call word cs:[bx+0xF29]` — a five-entry near-pointer table
at file `0x8709` (CS segment `0x071E`, base file `0x77E0`). All five entries land
exactly on the existing `nav_choice_handler_0..4` labels, which PROVES the handler
numbering those labels asserted. `[0x2A19]=row+1` on click matches the `dec bx`.

That does NOT close the APPROX, and I am not going to pretend it does: the row ->
console-function mapping is still open. Handler 0 is solid (it links a C3 record to
a named Honk object — a data-side identification), but handlers 1 and 2 BOTH look
communication-related ("reloads radio.snd"; "THE CONTACT MENU, built from live
state"), and the old HONK/TELEPHONE/CRYOBOX/MENU/OPTION order came from the capture
-sourced const deleted in #385. Naming rows from the artwork is what the prime rule
forbids. The open question is now much smaller and sharper than "find the routine".

THE LABEL ERRORS. `console_menu_hit_test` was recorded at `0x8613` — the last byte
of the `jne 0x86F1` at `0x8610`. Decoding there yields `add byte [bx+di+0x2795], ah`,
a phantom swallowing the `a1` of `mov ax,[0x2795]`. It is also not a routine at all:
nothing calls either address; it is a FALLTHROUGH block of `nav_choice_dispatch`
entered when no row is selected. Corrected to `0x8614`, and the SEVEN port-side
citations of `0x8613` rewritten.

Two more of the same shape, found by the new checker and each confirmed by hand:
`angle_wrap_180` at `0x97D6` is the `0x1E` immediate of `add bx,0x1e` (renders
`push ds`) -> `0x97D7`; `bridge_frame_to_yaw_sync` at `0x97E4` sat INSIDE the
4-byte `mov [0x2795],bx` that is the sync itself -> `0x97E3`.

`re/tools/check_label_alignment.py` asks the question `check_labels.py` never did:
does a label's ADDRESS decode? It took three passes to be worth trusting, and the
failures are the point. First run: 34 hits, including data tables and `0x9D10`,
which I had disassembled cleanly hours earlier — a straddle cannot tell a real
off-by-one from a linear decode that desynced. Adding relative-branch targets as a
second signal left `0x9D10` still flagged, because it is reached only by a FAR call
(`9A`, image-relative segment). With far targets and the DATA_HINT filter
check_labels.py already uses: 10 candidates, and the two I checked by hand were
both real. The rescued and unreachable buckets are counted separately and NOT
reported as problems, because "the sweep cannot answer" is not "the label is wrong".
It also tripped the stdlib-shadowing trap again — `sys.path.insert` of `re/tools`
shadows `dis` for capstone's `inspect` import, exactly #359's `encodings.py`.

Eight MISALIGNED candidates remain unchecked; that is the queue, not a claim.

2228 items, 1087 confirmed (48.8%), 1141 open. 702 citations verified, 0 wrong.
502 code + 238 data labels clean. 613 lib tests, 0 failures.

## #387 — working the misalignment queue: eight more, and two claims that dissolved

#386's checker left ten candidates. Worked all of them; the queue is now one.

SIX WERE REAL OFF-BY-ONES, each confirmed by hand before touching it:

  cmd_handler_numeric_value     0x652  -> 0x651   the 0x66 operand-size PREFIX
  draw_using_linear_a46         0xE36  -> 0xE37   the `06` immediate of `add bx,6`
  croolis_method_motion         0x146C -> 0x1468  inside `mov eax,0x1510`
  presentation_start_travel_arm 0x5C64 -> 0x5C63  inside `mov gs:[0x24f3],9`
  presentation_box_and_subtitle 0x9450 -> 0x944F  inside `mov bp,0x5eaf`
  location_info_panel           0x9100 -> 0x90FF  inside `inc byte [0x2789]`

Several are self-confirming: `0x944F` IS the `mov bp,0x5eaf` that loads the rect
records the label describes at `[bp]`; `0x90FF` writes `0x2789`, the scale the
label names; `0x5C63` is the `[0x24F3]=9` the label quotes. The label text and the
corrected address agree — which is what a right answer looks like here.

TWO CLAIMS DISSOLVED UNDER THE CHECK, and this is the part worth keeping:

`manu3_perframe_caller` at `0x32BD` rendered the phantom `call 0xb4e` — a
FABRICATED CALL under a label named `_caller`, the most persuasive kind of wrong.
Checked the claim itself: `gfx_clipped_primitive_a` (`0x32AC..0x3320`) contains no
far call at all (no `0x9A`/`0xEA` byte anywhere in it), and the quoted return
address `0x022D:0x07F2` converts to file `0x30C2`, which has none before it
either. The observation came from CALLERWATCH — a RUNTIME watch — and its
addresses are not in the main image's SEG:OFF space. Downgraded to UNVERIFIED.

`pending_slot_c4_writer` at `0x77A0` sat inside `mov byte es:[di],0`. Here the
surrounding decode is authoritative rather than guessed: `0x779C` is a real branch
target (`js`/`jb` from `0x7793`/`0x7797`), and its chain is `dec si / mov byte
es:[di],0 / mov byte gs:[0x27e8],1 / pop es / ret` — nothing writes 0xC4. A
whole-image scan for every `mov byte [reg+0x30],0xC4` encoding, bare and with all
four segment prefixes, found ZERO sites. Downgraded to UNVERIFIED.

Both were parked at sentinel note addresses rather than deleted, so the evidence
against them survives. Two others (`input_jump_table_static_limit_CONFIRMED`) were
methodology notes whose addresses were incidental; parked likewise.

ONE LEFT, and it stays open honestly: `project_tail_9bba` at `0x9CF7`. Decoding
from `0x9CF0` makes it a valid boundary right after a `ret` at `0x9CF6`, so the
checker's straddle may be a desync — but nothing branches to it, and `push es`
followed by `jmp` with `pop si` after reads wrong in both alignments. Not enough
to act on.

The checker also created a false positive for itself: sentinel notes at 0x0..0x7
are below the 0x600 MZ header and are not code. Now skipped.

2228 items, 1087 confirmed (48.8%), 1141 open. 702 citations verified, 0 wrong.
502 code + 238 data labels clean, 1 alignment candidate left. 613 lib tests, 0 failures.

## #388 — the orb box: both halves of the claim, checked

`PanoramaFrameHeader` (`tbbig.rs`) asserted two things. Both hold.

FIELD ORDER, from the hit test at `0x8269`:

    cmp ax, [si]      ax = [0xA2A] mouse x   -> [si]   = x
    sub ax, [si+4]                           -> [si+4] = w
    cmp ax, [si+2]    ax = [0xA2C] mouse y   -> [si+2] = y
    sub ax, [si+6]                           -> [si+6] = h

which also settles the box's SENSE, not just its layout: the pair of compares is
`x <= mx <= x+w`, inclusive on both ends. And the whole test is gated by
`test byte [0xa3e],1` — the same mouse-present flag #384 found deciding the choice
box's selected-row colour, two unrelated widgets agreeing on one flag.

THE STATION-TABLE COPY at `0x981B` took a detour worth recording: `0x2A1B` has
ZERO direct-address sites, because it is never addressed as `[0x2A1B]` — it is
loaded as an IMMEDIATE (`mov di,0x2A1B`). The census tool that has been reliable
all session answers a different question than the one I asked, and a bare "0
sites" would have read as "no such table". Scanning `B8+r imm16` for the value
found four sites.

Both halves then verify: the RESET at `0x985F..0x9875` is `mov cx,4` / `add di,0xc`
/ two `stosd` of 0xFFFFFFFF / `add di,4` — 0xC + 8 + 4 is exactly the 0x18 stride,
so all four stations are blanked; the COPY at `0x9877` is `mov ax,[si+8]` (the
ninth/tenth byte, the station word) / `mov dx,0x18` / `mul dx` / `add ax,0xc`.

That also explains the all-0xFFFF boxes on frames 21, 64, 71 as the RESET VALUE
LEFT IN PLACE rather than a stored sentinel — a distinction the doc had backwards
in spirit even though its conclusion ("no orb here") was right.

2228 items, 1088 confirmed (48.8%), 1140 open. 705 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #389 — c1_set_plan verifies, and a comment edit that deleted a `return`

Read `0x6BCE..0x6C0A` end to end against `c1_set_plan`. Every cited line is exact:

    0x6BCE  test byte es:[di+2],1 / je 0x6C73   owner active?
    0x6BE0  cmp ax,2 / je 0x6BEA                operand selects the redirect...
    0x6BE5  cmp ax,1 / jne 0x6C04               ...1 or 2, else skip
    0x6BEA  call 0x60DD                         distance(operand, owner)
    0x6BED  or ax,ax / je 0x6C04                zero distance -> no redirect
    0x6BF3  mov ax,0x11 / call 0x6023           selector 0x11 = parent link
    0x6BFF  cmp ax,0x10 / jne 0x6C73            redirected target must be kind 0x10

Two details the row gains by being read rather than sampled. The two kind-0x10
tests have DIFFERENT failure targets — the redirected one exits at `0x6C73` (the
branch), the direct one at `0x6C04`/`0x6C0A` jumps to `0x6C55` (the destination-
empty check) — and the port distinguishes them correctly. And after a successful
redirect the game RE-TESTS kind 0x10 at `0x6C04`, redundantly, because the
redirect path falls through; the port's `if rec_read(owner) == 0x10` reproduces
that redundancy rather than optimising it away, which is the right call.

Fixed one loose citation: `0x6BCE` is the `test`; its `je` is at `0x6BD3`. The
same file already cited it correctly 80 lines later, so this was an inconsistency
within one function.

AND I BROKE THE BUILD DOING IT. The line read `return Some(None); // owner
inactive (0x6BCE je)`. Replacing "the comment" replaced the whole statement,
because the comment was a TRAILING comment on a statement line — the return went
with it. One test failed immediately, which is the only reason it did not ship as
a silent behaviour change: a handler that stopped returning early would have kept
executing on an inactive owner. Restored, 613 pass.

The lesson is narrow and worth having: a trailing comment is not a separate line,
and an edit that "only touches a comment" on such a line touches code. Match on
the comment text alone, or include the statement in the replacement deliberately.

2228 items, 1089 confirmed (48.9%), 1139 open. 705 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #390 — c4_set_write_decision: the withdrawal holds up

Checked every address in this row's doc, including the reasoning #378 used to
WITHDRAW #376. All five instructions are exact:

    0x6CC3  test byte ptr es:[di + 2], 1     the guard
    0x6D01  mov word ptr es:[bp], 0xc4       the write
    0x5233  or  word ptr [bx + 2], 3         83 /1  (word, sign-extended imm8)
    0x52B5  and word ptr [bx + 2], 0xfffc    83 /4
    0x5B8D  and byte ptr [bx + 2], 0xfe      80 /4  (byte)

The encoding families the whole argument turns on are confirmed by the bytes:
`0x5233`/`0x52B5` really are the `83 /N` WORD form that the original enumeration
missed by scanning only `80 /N`, and `0x5B8D` really is the byte form.

The withdrawal's own claim also holds, which is the part I most wanted to check
since #378 reversed a conclusion I had already published. `0x5229` is
`mov bx, word ptr [0xc02]` — so `bx` at `0x5233` is a RESOURCE DESCRIPTOR, not an
object record — and the surrounding sequence matches the doc line for line
(`mov ax, gs:[0xa6a]` / `mov [bx], ax` / `or word [bx+2],3` / `mov [bx+4], ebp`).
Both sites sit in resource routines: `resource_name_write_c00` (0x5190) and
`resource_free_inner` (0x529C).

So `0x5B8D` is the sole runtime writer of an OBJECT's active bit, reading
VAR-initial bits stays justified, and the SUPERSEDED section is correctly marked.
No fix needed — the row is settled as written, which after four self-corrections
this session is worth stating plainly rather than hunting for something to change.

2228 items, 1090 confirmed (48.9%), 1138 open. 705 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #391 — checking 28 opcode citations against the table instead of by eye

`VmState::step` documents one handler address per opcode (`// 0xA0 PUSH
(0x6559)`, `// 0xA2 (0x6588)`, …), written by hand across many sessions. Spot-
checking two of them is what this row would normally get. The image has the
ground truth — the dispatch table the interpreter indexes with the opcode byte,
already decoded by `re/tools/dump_handler_table.py` — so all of them can be
checked at once.

`tools/check_vm_opcode_citations.py` compares the two. Result: **28 citations,
28 matches, 0 mismatches.** Settled ASM on that basis rather than on a sample.

It reports two other things without calling them errors. 27 dispatched opcodes
carry NO citation — mostly the shared-handler families (`0xAD/0xAF/0xB2/0xB3/
0xBA/0xBB/0xBC` are one handler, `0xB1/0xB4/0xB5/0xB6/0xBE/0xBF/0xC0` another),
which the port groups without per-opcode comments. A missing comment is a
documentation gap, not a false claim, so it is a count and not a finding. And an
`UNDISPATCHED` bucket exists for opcodes cited but absent from the table, because
the TOKEN bound and the DISPATCH bound differ at OP_MAX and a citation there
should be deliberate rather than assumed wrong.

PERTURBED, per #370: changing one cited address by a single digit produces
`MISMATCH src/vm.rs:6731: 0xa2 cites 0x06589, table says 0x06588`, and reverting
returns it to 28/0. A checker reporting zero has to be shown capable of reporting
one.

Also verified the two handlers by hand while here, and the A0 listing had a hole:
it jumped `0x655F -> 0x6565`, omitting `0x6563 mov bp,ax`. That instruction is
precisely WHY the doc's "POST-increment" is correct — bp keeps the old pointer,
`add ax,2` bumps the stored one, and `mov [bp+0x6820],ax` writes the slot bp
still names. The conclusion was right with its reason missing; now listed.

2228 items, 1091 confirmed (49.0%), 1137 open. 707 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #392 — the opcode GROUPS match the binary's shared handlers, 9 of 9

#391 verified each opcode's cited ADDRESS. The port also makes a second, stronger
claim the table can settle: it writes shared handlers as grouped match arms
(`0xAD | 0xAF | 0xB2 | 0xB3 | 0xBA | 0xBB | 0xBC => { .. }`), asserting those
opcodes ARE one handler in the game. A wrong group means merging behaviours the
game keeps apart, or splitting ones it shares — worse than a wrong citation,
because it changes what the port DOES.

Extended the checker to compare each grouped arm against the handler's full
opcode set. **9 grouped arms, 9 exact matches, 0 with wrong members.** Both large
families are confirmed against the dispatch table, not against a comment:
`0xAD/0xAF/0xB2/0xB3/0xBA/0xBB/0xBC` -> `0x6946`, and `0xB1/0xB4/0xB5/0xB6/0xBE/
0xBF/0xC0` -> `0x6863`.

THE SPLIT BUCKET FOUND FIVE, AND ALL FIVE WERE FINE. Investigated each rather
than reporting them:

  * `0xCE | 0xD0 | 0xD1 => pc += 1` (twice) is not a DISPATCH arm at all — it is
    inside script SCANNERS, grouping by operand LENGTH, and all three really are
    one-byte opcodes. The dispatcher has separate `0xCE`/`0xD0`/`0xD1` arms, which
    matters because the three handlers `0x6494`/`0x64A0`/`0x64AC` are byte-for-byte
    identical APART FROM the flag they test (`gs:[0x2793]`, `gs:[0x252A]`,
    `gs:[0x274F]`) — merging them would have tested one flag for three opcodes.
  * `0xAA | 0xAC` (twice): distinct handler addresses, identical bodies — both set
    the yield flag `gs:[0x67b4]`.
  * `0xC5 | 0xC6 | 0xC7 | 0xC8`: one arm that discriminates INSIDE via `match op`,
    which is where the genuinely different per-opcode write guards live.

So the bucket has a 100% false-positive rate on current code and is now ADVISORY,
off unless `--splits` is passed, with those three benign shapes written into the
tool. Printing five non-findings as findings is the failure mode this session has
corrected in three other tools; better to catch it in the same commit that
introduces it.

2228 items, 1091 confirmed (49.0%), 1137 open. 707 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #393 — every dispatched opcode now cites a handler the table confirms

#391 reported "27 dispatched opcodes carry no citation". That number was WRONG,
and it was wrong in the direction that matters: it described the PORT as
under-documented when in fact every one of those arms carried its handler
address and my regex could not see the form. The port writes citations three
ways —

    // 0xA0 PUSH (0x6559)                  one opcode, one address
    // 0xAA/0xAC (0x6855/0x685C)           n opcodes, n addresses, paired
    // 0xAE/0xB0 (0x6902)                  n opcodes sharing one address
    // The 0x6946 family (AD/AF/B2/...)    address first, bare opcode bytes

— and the tool knew only the first. A coverage number that undercounts is worse
than none, because it invents work.

Teaching it all three raised the check from 28 citations to 66 and immediately
produced a MISMATCH at `vm.rs:7004`: `0xBC cites 0x6989, table says 0x6946`. That
was MY bug too — loosening the pattern to `//.*?` let it match prose, and the line
is `// SET (0x6985): 0xBC stores the RAW value to gs:0x6782 (0x6989)`, where
`0x6989` is an address INSIDE the 0x6946 handler. A citation is the opcode LEADING
its comment; an opcode mid-sentence is prose. Re-anchored.

Then `0xD3` showed as dispatched-but-uncited. Its table slot has near-offset
`0x0000` while every real entry is non-zero — an EMPTY slot the dumper still
resolves to an address (`0x53A0`). The VM dispatches `0xA0..0xD2`; `0xD3` is past
the bound, exactly the TOKEN-vs-DISPATCH distinction already recorded at OP_MAX.
Zero slots are now dropped.

That left four genuinely uncited: `0xC5..0xC8`, whose arm is grouped but whose
handlers are FOUR distinct routines (`0x6D18`/`0x6D80`/`0x6DCF`/`0x6F62`). Added
the citation, noting that this is why the arm re-tests `op` internally rather
than sharing one path.

FINAL: **54 opcode citations, 54 confirmed against the dispatch table, 0
mismatches, 0 dispatched opcodes uncited.** Three tool errors of my own were
found and fixed getting there, all three in the direction of over-reporting work
or inventing a defect — which is the failure mode to prefer catching early, but
three in one entry is worth noticing.

2228 items, 1091 confirmed (49.0%), 1137 open. 707 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #394 — the presentation-start block is longer than its doc said

`start_actor_presentation` is documented as modelling a SUBSET of `0x5816`'s
kind-1 start, with the missing work listed. Re-read the block; THE LIST WAS
INCOMPLETE. It named seven cleared cells. There are ten, and two effects it did
not mention at all:

    0x5904  mov byte gs:[0x67ac],1     active
    0x590A  xor ax,ax                  everything below cleared to 0
    0x590C  0x6782   0x5910  0x6784    0x5914  0x6776   0x5918  0x67f8
    0x591C  0x2A19  <-- NOT LISTED
    0x5920  0x67ba   0x5924  0x27d7  <-- NOT LISTED
    0x5928  0x67bc   0x592C  0x67bb    0x5930  0x679a
    0x5934  mov byte gs:[0x67b7],1     start-lock
    0x593A  or byte gs:[0x2793],4      busy (the bit the port does set)
    0x5940  or byte [bx+3],0x80        record+3 |= 0x80
    0x5944  and byte gs:[0x2751],0x7f  <-- NOT LISTED

`gs:0x2A19` is the CONSOLE MENU SELECTION — the cell `console_menu_hit_test`
writes as `row+1` and `nav_choice_dispatch` reads at `0x860A`, both decoded in
#386. So starting a presentation clears the console selection: a link between the
VM and the bridge that the doc had no idea it was describing.

That link matters because in the port `selected_menu_item != 0` BLOCKS the entire
eye-orb/station click scan (`bridge.rs:358`). A selection never cleared leaves the
bridge permanently unclickable. The port does have `release_menu()` and it has SIX
live callers in main.rs, all on screen-CLOSE paths — which plausibly corresponds
to the game's `console_mode_dismiss_ladder` (`0x8956`, one of the four sites that
zero `0x2A19`). It does NOT obviously cover `0x591C`, which fires when a
presentation STARTS, a different moment.

ROW NOT SETTLED. The doc is now correct and complete where it was incomplete, and
the mechanism is located on both sides, but "does the port clear the selection at
the moment the game does" is unanswered — and that is what `ASM?` means. Settling
it here would be recording the investigation as the conclusion.

2228 items, 1091 confirmed (49.0%), 1137 open. 713 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #395 — a correspondence I nearly claimed, and why it is wrong

Following #394's open question: does the port clear the console selection where
the game does? The game zeroes `gs:0x2A19` at exactly four sites (#386's census):
`0x591C` (presentation START), `0x87B0` (`nav_choice_handler_1`), `0x883B`
(`nav_choice_handler_2`), `0x8956` (`console_mode_dismiss_ladder`).

The port calls `release_menu()` at six sites. Two of them sit in a `match kind`
whose arms are `1` and `2` — and rows 3/4/5 deliberately do NOT release. That is
a striking shape: the game clears on handlers 1 and 2 and not on 0/3/4. I had the
site-by-site correspondence written out before checking what `kind` is.

IT IS `engine.console_box_kind`, NOT THE DISPATCH ROW. The whole handler is
guarded by `!engine.console_box.is_empty()` — it runs only when a console BOX is
already open, and classifies which box, then which row inside it. The game's
`nav_choice_handler_N` are the ROW dispatch, one level up. Matching `1`/`2`
against handler 1/handler 2 is a coincidence of small integers.

So the honest answer to #394 is unchanged and now better characterised: the port
releases the selection at BOX-level events (a box row that leaves the bridge, a
submenu close, a click-off), while the game clears it at ROW-dispatch and at
presentation start. Those are different levels of the same interaction, and
whether they coincide in effect is exactly what remains unverified.

Worth recording as its own entry because the wrong version was persuasive: two
independent-looking systems agreeing on the numbers 1 and 2, where one of them
turned out to be counting something else entirely. `0x591C` remains the one clear
site with no identified port counterpart.

2228 items, 1091 confirmed (49.0%), 1137 open. 713 citations verified, 0 wrong.
613 lib tests, 0 failures.

## #396 — the console selection is never cleared on dispatch: a real port bug

#395 said the port's `release_menu()` sites are box-level while the game's clears
are row-level, and left it there. Followed it to the actual counterpart:
`console_menu_click` in main.rs dispatches rows 0..4 — HONK, phone, cryobox,
submenu, option box — and it calls `release_menu()` on NONE of them.

The game does. `nav_choice_handler_1` (`0x87B0`) and `nav_choice_handler_2`
(`0x883B`) both END with the identical epilogue:

    mov word ptr [0x2a19], 0        the selected console row
    and byte ptr [0x2793], 0xfb     clear the BUSY bit
    pop es / ret

and handlers 0, 3, 4 do not — matching #386's census, which found `0x2A19`
written by exactly these two plus the presentation start (`0x591C`) and the
dismiss ladder (`0x8956`).

THIS HAS A VISIBLE CONSEQUENCE, which is what makes it a bug rather than a
cosmetic gap. In `BridgeView::click`, a non-zero `selected_menu_item`
short-circuits the whole eye-orb/station scan. So in the port, opening the phone
or the cryobox from the console left the selection set, and the bridge orbs stayed
unclickable on return.

Fixed with `clear_menu_selection()`, deliberately NARROWER than the existing
`release_menu()`: the handlers write `0x2A19` and nothing about the clamp, so
dropping `menu_engaged` too would be an invention. Called from rows 1 and 2 only.
A regression test pins exactly that distinction — selection cleared, clamp
retained — because the two methods are one word apart and the wrong one is easy
to reach for.

The busy half is left to the presentation lifecycle, where the port already
clears `UI_FLAG_BUSY` on teardown (`0x59BF`/`0x5E99`); rows 1/2 do not start a
presentation in the port, so re-clearing it there would be a no-op dressed up as
fidelity.

The test I first wrote asserted through a method that does not exist
(`orb_click`); the scan is inside `click`. Rather than build a synthetic orb to
keep the assertion, the consequence is stated and the DECODED fact is what gets
pinned. An assertion that needs scaffolding to pass tests the scaffolding.

2228 items, 1091 confirmed (49.0%), 1137 open. 713 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #397 — closing 0x591C, the last unmatched clear

#396 fixed rows 1 and 2. Row 0 (HONK) was still wrong, and for a different
reason: `nav_choice_handler_0` (`0x8713`) does NOT clear `0x2A19`. The clear for
that path comes from the PRESENTATION START itself — `mov word gs:[0x2a19],0`
@`0x591C`, inside `presentation_scan`, which #394 found while correcting that
block's cleared-cell list from seven entries to ten.

Row 0 is the case where it matters most: HONK plays ON the bridge, so unlike rows
1 and 2 no screen opens and closes afterwards to release anything. The selection
would have stayed set for the rest of the session, with the eye-orb scan
short-circuited the whole time.

WHERE THE FIX LANDS IS AN ARCHITECTURE CONSEQUENCE, not a choice I like.
`start_actor_presentation` is on `VmMachine`, which has no `BridgeView` — the port
splits VM state from bridge state where the game has one data segment. So the
clear is applied at the call site instead, flagged inside the VM borrow and run
after it drops. Only ONE live call site needed it: the other main.rs sites are
non-bridge contexts, and the kind-2 console-box path already calls
`release_menu()` before starting its presentation.

All four of the game's `0x2A19` clears now have identified port counterparts:
`0x87B0` and `0x883B` (#396), `0x8956` (the dismiss ladder, the pre-existing
box-level `release_menu` sites), and `0x591C` here. That question has been open
across #394, #395 and #396; it is closed.

The scattered-call-site shape is a latent hazard worth naming: a future
`start_actor_presentation` call from a bridge context will not clear the
selection, and nothing will catch it. The real fix is for the engine to own the
invariant rather than each caller — recorded, not done, because inventing an
engine-level lifecycle hook is a bigger change than the decode justifies.

2228 items, 1091 confirmed (49.0%), 1137 open. 713 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #398 — the seek steering, including the magic 4

`update_view_steer` cited a range (`0x96D0..0x96DD`) for the cursor drag and left
the step arithmetic uncited. Read `0x96A7..0x96F3`; every constant in the port
comes out of the instructions:

    0x96A7  mov dx,[0x279d] / or dx,dx / jne     memoise the initial distance
    0x96AF  mov word ptr [0x279d], ax             ...ONCE, while still zero
    0x96B2  mov dx,ax / shr dx,1                  step = distance/2
    0x96B6  jne / inc dx                          ...floored at 1
    0x96B9  shl ax,2                              drag = distance*4
    0x96CC  neg dx / neg ax                       direction on the other branch
    0x96D0  mov cx,[0x279d] / cmp cx,0x28 / jl    long-seek gate
    0x96DB  add word ptr [0xa38], ax              drag the ring anchor
    0x96DF  add ax,dx / +0xB4 or -0xB4            wrap over 180 frames

So `(distance / 2).max(1)` is `shr` + `inc`, the `* 4` is a `shl ax,2`, and
`PANORAMA_FRAME_COUNT` is the `0xB4` the wrap adds and subtracts. The port had all
three right and cited none of them — the `* 4` in particular read as a tuning
constant, which is exactly the shape a capture-measured number would have.

One ordering detail worth pinning: `mov [0x279d], ax` @`0x96AF` happens BEFORE
`shl ax,2` @`0x96B9`, so the memo holds the RAW distance, not the quadrupled one.
The port stores `distance`. Had it stored the drag value the long-seek gate would
compare `0x28` against a number four times too large and every seek over ten
frames would drag the cursor.

2228 items, 1092 confirmed (49.0%), 1136 open. 713 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #399 — three named constants that turned out to be immediates

`commit_world_destination` / `ship_click_initial_target` carried three constants
with descriptive names — `ARCHE_LOCATION_FIELD`, `SHIP_CLICK_LOCATION_KIND_MASK`,
`SHIP_3D_TARGET_NAME_TO_RECORD`. A good name hides the question of where a number
came from, so each was checked against `0xB0DC..0xB116`:

    0xB0EA  mov di, word ptr [0x6752]        di = the ARCHE global
    0xB0F3  mov ax, word ptr es:[di + 0x16]  -> ARCHE_LOCATION_FIELD = 0x16
    0xB0FB  test word ptr es:[eax], 0x140    -> ..._KIND_MASK = 0x140
    0xB10D  mov word ptr [0x251b], di
    0xB111  sub word ptr [0x251b], 4         -> ..._NAME_TO_RECORD = 4

All three are immediates in the instruction stream, and `0x6752` being the arche
global (already recorded in four places) is what makes `es:[di+0x16]` an
arche-relative field rather than an unknown record's.

Also pinned an ordering fact the port relies on silently: at `0xB10D`, `di` is
still the `[0x250B]` head read at `0xB0F7`, because the `jne` @`0xB101` jumped
over the re-root at `0xB103` that would have overwritten it. The port's
`candidates.first()` is correct only under that branch structure, and nothing said
so.

Not modelled, and now stated rather than left silent: `0xB0DC` also sets
`[0xADD]=1` and `[0xADA]=0xA` before any of this. Those are UI-state cells the
port does not carry here.

2228 items, 1093 confirmed (49.1%), 1135 open. 713 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #400 — the alien PRNG is one shared stream, not a seed per object

Verifying `croolis::step`'s citations (all exact, in `XDB:croolis`) turned up the
thing the citations were next to:

    0x16B4  mov ax, word ptr fs:[0x105c]     read the seed
    0x16B8  ror ax, 7 / sbb ax, 0            step it
    0x16BE  mov word ptr fs:[0x105c], ax     WRITE IT BACK

`fs:0x105C` is a GLOBAL. Every object in the colony draws from ONE stream, in the
order their `+0x38` timers expire. The port gave each object its own `prng` field
and advanced it privately — and the struct doc even labelled that field
"`fs:[0x105C]`", so the correct address was sitting on the wrong shape.

WORSE, A TEST DEFENDED IT. `assert_ne!(colony.objects[0].prng, colony.objects[1].prng)`
with the comment "Objects are seeded distinctly so they don't all change state in
lockstep", against a constructor seeding `base_seed + i * 0x9E3B`. That is a
plausible-sounding invention with a test pinning it in place — the exact failure
the faithfulness memo names, and it survived because the invention was reasonable
engineering. The overlay achieves de-sync differently: objects share the stream
and separate because their TIMERS expire on different frames.

Fixed. `AlienColony::prng` is now the stream (`fs:[0x105C]`); `AlienObject::step`
and `dispatch` take `&mut u16` and draw from it, keeping what they drew;
`AlienColony::new`'s `base_seed` seeds the STREAM, not the objects; the engine's
standalone object draws from an `alien_prng` field the same way.

The replacement test encodes the REAL behaviour and had to be corrected once
mid-write: my first version asserted the three objects had drawn after three gate
updates, but a 50-frame timer only ticks to 47 in 21 frames, so nothing had drawn
at all. It now asserts both halves — nothing drawn early, then after the timers
expire the three objects hold CONSECUTIVE values of one stream and the stream ends
where the last object left it. That is a claim the old per-object model cannot
satisfy.

Still not modelled, and still cited: the second `ror/sbb` @`0x16E5` (NOT written
back to the global) landing in `+0x42`, which is the field the proximity gate
reads as the object's X.

2228 items, 1093 confirmed (49.1%), 1135 open. 713 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #401 — the anim counter is shared too, and +0x42 is a position

#400 fixed the PRNG. The SAME BLOCK has a second shared cell the port also owned
per-object, which #356 flagged and nothing had acted on:

    0x16C2  movsx ebx, word ptr cs:[0x16a2]   the counter, SIGN-EXTENDED
    0x16D8  mov dword ptr [di+0x3c], ebx      object takes its CURRENT value
    0x16DC  add bx, 0xfa                      ...and only THEN it advances
    0x16E0  mov word ptr cs:[0x16a2], bx      stored in the CODE segment

The port did `self.anim = self.anim.wrapping_add(250)` on a per-object `u16`.
Three things wrong at once: the counter is shared, the object takes the value
BEFORE the advance rather than after, and the store is a DWORD of a sign-extended
WORD, so `anim` is `i32` and the sign extension is observable.

AND `+0x42` IS THE OBJECT'S X. The block ends with a SECOND `ror ax,7 / sbb ax,0`
@`0x16E5` whose result goes to `mov word ptr [di+0x42], ax` @`0x16EB` — and
`+0x42` is `pos[0]`, the field the proximity gate reads as X. That second step is
NOT written back to `fs:[0x105C]`, so it is derived from the stream rather than
advancing it. The port never modelled it, which means alien X positions never
moved from their initial values.

Both shared cells now live in one `AlienStreams` (`fs:[0x105C]` + `cs:[0x16A2]`),
which is a better shape than #400's bare `&mut u16` and says plainly that these
are the overlay's globals rather than object fields.

Tests pin the ordering that is easy to get backwards: `+0x3C` records the counter
BEFORE the advance (first draw records 0, the stream moves to 0xFA), and the
derived `+0x42` leaves `prng` untouched. Both would pass trivially under the old
model's arithmetic and fail under its structure — which is the point.

2228 items, 1093 confirmed (49.1%), 1135 open. 719 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #402 — the tail writes are on a NEIGHBOUR, not this object

Before settling `croolis::step` I checked what its remaining unmodelled writes
actually target, because they use `si` where everything else uses `di`. They are
not the same record:

    0x16A4  mov si, word ptr [di + 0x16]   a pointer out of THIS object
    0x16A7  add si, 0x5e                   ...advanced by ONE OBJECT STRIDE
    0x16AA  test word ptr [di + 0x36], 0xffff
    0x16AF  je 0x16B4                      state_flag == 0 -> the state machine
    0x16B1  jmp word ptr [si + 0xe]        else -> THAT record's sub-method

So `si` is a related record at `[di+0x16] + 0x5E`. The tail writes
(`[si+0x50] = ax & 0xFFC`, `[si+0x52] = 0`, and `mov word ptr [si+0xe], 0x1727`
@`0x16FE` — which INSTALLS a sub-method on the neighbour) all land there.

This matters twice over. First, the port maps `anim_counter` to `+0x50` on the
object itself; if a future pass had "completed" `step` by writing that field from
`0x16F1`, it would have written the wrong record and looked correct. Second, the
same `si` is what the vtable jump at `0x16B1` dispatches through, so an object
whose `state_flag` is set runs its NEIGHBOUR's method — object linkage, not a
per-object state machine.

ROW STAYS PROVISIONAL, and now for a stated reason: modelling this needs the
object-list linkage `+0x16` names, and the port's colony is a `Vec<AlienObject>`
with no cross-references. That is the blocker; it is written into the code so the
next pass starts from it instead of re-deriving it. Building that linkage is the
task, not a reason to stop — but inventing a neighbour relation to satisfy a
checkbox is exactly the kind of plausible fiction #400 just removed.

2228 items, 1093 confirmed (49.1%), 1135 open. 725 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #403 — the alien objects are a TREE, and that is the blocker's answer

#402 stopped at "modelling the tail writes needs the `+0x16` linkage, which the
port does not carry". That is a description of the next task, so I did it.

`+0x16` is a CHILD-ARRAY POINTER. The colony dispatcher spells the structure out:

    0x12DE  mov si, word ptr [di + 0x16]   the child array base
    0x12E1  mov cx, word ptr [di + 0x1a]   the child COUNT
    0x12E4  add si, 0x5e                   iteration starts at element 1
    0x1301  call word ptr [si + 0xe]       each child's method

and the state machine at `0x16A4` opens with the IDENTICAL two instructions,
which is why its `[si+…]` writes land on child 1 rather than on itself.

A census of every `[reg+0x16]` access in the overlay — sixteen of them, at
`0x36A`, `0x966`, `0x999`, `0xA01`, `0xA37`, `0xB50`, `0xB60`, `0x12DE`, `0x16A4`,
`0x1A86`, `0x1B85`, `0x1BCD`, `0x1C1C`, `0x207C`, `0x2291`, `0x23D0` — finds NOT
ONE WRITE. The field is set up outside this overlay and only ever followed, which
is consistent with it being part of the shipped object data rather than runtime
state, and means the overlay cannot tell us how the tree is built.

That census also corrected my first attempt at it: scanning modrm bytes
`0x44..0x47` found only two accesses and would have supported "the field is
barely used". Those are the `reg=ax` encodings; `mov si,[di+0x16]` is modrm
`0x75`. Filtering by `mod`/`rm` instead of by literal byte found all sixteen.
Same family of encoding blind spot as #335 and #359 — the third time this
session that enumerating one encoding of an instruction under-reported a census.

CONSEQUENCE FOR THE PORT, stated concretely: `AlienColony { objects: Vec<..> }`
is the wrong shape. An object OWNS a child array (`+0x16`, count `+0x1A`, stride
`0x5E`), both the dispatcher and the state machine skip element 0, and `step`
reaches into child 1. That specification is now in the code where the work will
happen, so the next pass builds against it rather than re-deriving it.

2228 items, 1093 confirmed (49.1%), 1135 open. 729 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #404 — eighteen decoded types that nothing in the game ever builds

About to restructure `AlienColony` into the tree #403 decoded, I checked one
thing first: does anything BUILD an `AlienColony`? No. Its only construction is
in its own test. The colony dispatcher, its `cs:0xB72` frame gate, the shared
streams #400/#401 just fixed — all decoded, all ported, and the running game
never instantiates any of it. Restructuring it would have been elaborating dead
code, and I would have found that out after the refactor rather than before.

`check_unfed_runtime.py` (#290) could not see this. It looks for `with_`/`set_`
BUILDERS whose call sites are all in tests, so a whole subsystem can be unfed
while every builder inside it looks fine. Extended it with an UNFED-TYPE bucket:
a `pub struct` whose every construction site is test code.

**18 types.** `AlienColony` and `AlienCamera` (croolis), `MenuAnimDescriptor` and
`MenuTweenList` (manu3), and eleven `Ship3d*State` types — `Ship3dNavChoiceState`
alone is built 49 times, all in tests. Two of those, `MenuAnimDescriptor` and
`Ship3d*`, are ALREADY open ledger rows, which is a useful cross-check: the audit
had flagged them for other reasons and this says plainly why they resist
settling.

The check took three tries and each failure is the same shape — my construction
pattern `Name\s*(::\w+\(|\{)` matched the DECLARATION. First run: 0 dead types,
because `pub struct Name {` counted as a production construction of itself.
Excluding `struct` still hid `AlienColony`, because `impl AlienColony {` counted
too. Only after excluding `struct`/`enum`/`trait`/`impl` lines did the real list
appear. A checker reporting zero is the least trustworthy result there is, and
twice here it was zero for a reason that had nothing to do with the codebase.

This reframes the alien work: #400 and #401 fixed real decode defects in code
that runs only under test, and #403's tree spec describes a structure the port
has no live instance of. Wiring the colony into the game is the task that makes
any of it observable — recorded here as the finding, not started, because it is a
different piece of work from the decode that surfaced it.

2228 items, 1093 confirmed (49.1%), 1135 open. 729 citations verified, 0 wrong.
614 lib tests, 0 failures.

## #405 — following #401 into its only live consumer

#401 changed `AlienObject::anim` from a per-object `u16` accumulator to the
SHARED `cs:[0x16A2]` counter sign-extended to `i32`. #404 then showed the colony
is dead code — but `anim` has ONE live reader, in `engine.rs`'s alien scene:

    self.scene_frame.wrapping_add(self.alien_object.anim as usize % 3)

`anim as usize` on a NEGATIVE `i32` wraps to an enormous value. The counter is a
16-bit cell that passes `0x7FFF` after ~262 draws, so the sign extension #401
made faithful would, at exactly that point, have turned this nudge into garbage.
The port change was right and its consumer had to follow; reading the value back
as the `u16` the cell actually is fixes it.

Also labelled what that line IS: the nudge is PORT-SIDE — the game's consumer of
`+0x3C` has not been traced — while the VALUE is decoded. Those are different
confidence levels sharing one statement, and the comment now says which is which.

A regression test pins the boundary, and it failed on the first run for a reason
worth keeping: I seeded one step below `0x7FFF`, so the second draw landed exactly
ON `0x7FFF` — still the largest POSITIVE `i16`. The boundary was off by one STEP,
not one unit. Seeded at `0x7FFF`, the first draw is positive and the second is
negative, which is the transition the engine cast has to survive.

The general point: a type change made for fidelity is not finished at the type.
#401 passed 614 tests and shipped a latent defect two files away, because nothing
connects "this field is now signed" to "someone casts it to usize".

2228 items, 1093 confirmed (49.1%), 1135 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #406 — the vtable is real, but not where a static dump can see it

`AlienMethod` claims the behaviour method is "selected via the vtable at
`fs:0x103A` (near-ptr entries indexed by `bx = [di+0x34]`)". Dumping `0x103A` out
of `croolis.xdb` gives TWELVE ZERO WORDS — which, taken alone, reads as "there is
no such table".

It is the same shape as `fs:0x105C` in #400: the overlay's `fs` segment is
zero-initialised in the file and filled at runtime. A static dump cannot verify a
runtime table, and reporting the zeros as the answer would have been the error.

The CODE settles it instead, at both dispatch sites:

    0x1CFC  mov bx, word ptr fs:[di + 0x34]     the index, out of the object
    0x1D00  call word ptr fs:[bx + 0x103a]      the vtable call

So the claim holds exactly, including that `bx` is used unscaled — `[di+0x34]`
holds a byte offset, not an entry number. And `0x1D27`, named as the null method,
is a bare `ret`. Settled ASM.

The general rule this instance sharpens: for an OVERLAY, "the bytes at that
address are zero" is evidence about the FILE, not about the table. Ask what reads
it. Two of this file's key structures — the PRNG stream and the vtable — are
invisible to a static dump and fully determined by their access sites.

2228 items, 1094 confirmed (49.1%), 1134 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #407 — a flaky test, caught by running the suite a different way

Checking `nav_world_label_sample`'s data source, I ran the `levels::` tests alone
and got `primary_worlds_are_the_named_planets ... FAILED` — a test the full suite
reports as passing every time. It passed in isolation. It passed the next three
runs. It failed again once in a later batch, then passed twelve runs straight.

So: real, intermittent, roughly one run in ten under that filter, invisible to
`cargo test --lib`. The suite's "0 failures" — quoted in every entry this session
— was true and not the whole truth.

THE RACE, and the test's own comment is half of the diagnosis. `RUNTIME_DIRECTORY`
is a process-global `OnceLock` that any test may install via
`init_level_directory`, and cargo runs tests in PARALLEL threads. The test read
the global TWICE:

    let names: Vec<_> = primary_worlds().map(|e| e.stem).collect();   // sample 1
    ...
    let expected = if directory().len() > LEVEL_DIRECTORY.len() { 32 } else { 16 };
    assert_eq!(names.len(), expected, ...);                            // sample 2

A test installing the real 95-slot table between those two lines makes `names` a
pre-install list and `expected` a post-install number: 16 vs 32. The comment
above it already records fixing an ORDER dependence by keying off
`directory().len()` — which removed the dependence on WHICH tests ran and left a
dependence on WHEN, because the two samples are not atomic.

Fixed by snapshotting once. 12 consecutive `levels::` runs clean, full suite
clean.

Worth stating plainly: I found this only because I ran the tests through a filter
I had not used before. A suite that is green under one invocation and red under
another is not green — and the fix is one line, but noticing cost nothing except
running it differently.

2228 items, 1094 confirmed (49.1%), 1134 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #408 — bounding #407's blast radius, and the seek flag

Two things, one commit.

FIRST, how far #407's race reached. Swept every test that touches the racy
global: `RUNTIME_DIRECTORY` has exactly ONE installer
(`derived_directory_reproduces_the_literal`, which calls `init_level_directory`
itself before asserting), and three other readers. Two of them —
`world_resource_ids_match_the_fs0c04_table` and
`directory_indices_are_dense_and_ordered` — only ever index BELOW 53, where the
transcribed prefix and the image's 95-slot table agree by construction and by
`level_directory_literal_matches_the_image`. So they cannot observe the
difference, and `primary_worlds_are_the_named_planets` was the sole exposure.
Only `MIXER` (audio) is a second process-global, and no test installs it.

That is a NEGATIVE result and it is the reason to record it: the interesting
question after finding one flaky test is how many more there are, and the answer
here is bounded rather than assumed.

SECOND, `UI_FLAG_SEEK_ACTIVE` settles. Both citations are exact:

    0xB193  test word ptr [0x2793], 8      the flag read on its own
    0x1095  test byte ptr [0x2793], 0xe    the main-loop DEFER gate, any of 1|2|3

which also confirms the pair's relationship — the same cell is tested as a WORD
at one site and a BYTE at the other, and `0x8` really is inside the `0xE` mask
the gate uses, so the doc's "read directly as well as through the gate's mask" is
two instructions rather than an inference.

2228 items, 1095 confirmed (49.1%), 1133 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #409 — the approach camera's origin, confirmed from three places

`Ship3dCameraApproach::default` carries #275's correction: `origin_y` is 12000,
not 0, because the phase-3 reset never writes `[0x2F67]`. That is an argument
from ABSENCE — "the instruction isn't there" — which is the shape most worth
re-checking, since a missed instruction and a missing one look identical.

It holds, and three independent sources agree:

    0x8AF2  mov word ptr [0x2f69], 0x4e20    Z
    0x8AF8  mov word ptr [0x2f71], 0         yaw
    0x8AFE  mov word ptr [0x2f65], 0x2710    X   <- and then 0x8B04 moves on
                                                    to `mov si,0x1f22`; there is
                                                    genuinely no [0x2F67] write

    0x8CB4  mov word ptr [0x2f65], 0x2710    the FULL origin reset...
    0x8CBA  mov word ptr [0x2f67], 0x2ee0    ...which does write Y: 12000
    0x8CC0  mov word ptr [0x2f69], 0

    DS:0x2F65 in the shipped image: 0x2710, 0x2EE0, 0x0000

So the value Y "keeps" is written by the full reset AND is what the image ships
at that address. Code and data give the same three words, and the partial reset's
omission is visible as an instruction boundary rather than inferred from a gap.

Settled. Worth noting what made this row cheap to confirm: #275 had written down
WHICH instruction was absent and WHERE the value came from instead, so checking
it was three lookups rather than a re-derivation.

2228 items, 1096 confirmed (49.2%), 1132 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #410 — the subtitle block advances in Y only

`SUBTITLE_X`/`SUBTITLE_Y` cited "called from 0x94EE with BX=[0x5E5C] and
DX=[0x5E5E]". Both the data and the loop check out, and reading the whole loop
rather than the two cited words adds a fact the constants depended on silently:

    0x94E6  mov bx, word ptr [0x5e5c]     X, loaded ONCE, before the loop
    0x94EA  mov dx, word ptr [0x5e5e]     Y
    0x94EE  lcall 0x299:0x6a0             draw this line
    0x94F8  mov al, 0xd / repne scasb     scan to the next CR
    0x9503  add dx, 8                     Y += 8
    0x9508  jmp 0x94ee                    loop

`ds_dump.py DS:0x5E5C` gives 10 and 8, so the two constants are shipped data, not
measurements. And BX is NOT reloaded inside the loop — every line is left-aligned
at the same X, and the block advances in Y only, by the same 8 as
`GAME_FONT_LINE_HEIGHT`. The doc said "each CR-delimited line advances DX by 8",
which is right; what it did not say is that X is loop-invariant, and that is the
part a reimplementation could get wrong while matching the sentence.

Also confirms the CR delimiter is literally `0xD` (`mov al, 0xd` feeding
`repne scasb`), rather than a convention inferred from the text data.

2228 items, 1097 confirmed (49.2%), 1131 open. 730 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #411 — a hand-lifted ISR install, checked against every instruction

`func_79c` is a HAND-WRITTEN lift (io_lift.rs), not machine-generated, so nothing
in the recomp pipeline proves it — the oracle-verified lifts are elsewhere. Read
`0x079C..0x07E5` and compared it line by line. It matches completely: the five
entry pushes, `mov ax,0x3508` + `int 0x21`, the vector saved to `gs:[0xB1D]` /
`gs:[0xB1F]`, `mov ah,0x25` with `bx=cs` / `ds=bx` / `dx=0x213`, the PIT
programming (`out 0x43,al` with 0x36, then `0x1746` low byte and `mov al,ah` for
the high), and the four state writes.

TWO ORDERING DETAILS the lift preserves and a rewrite would likely not. The game
writes `gs:[0xB27]` @`0x07D5` BEFORE `gs:[0xB25]` @`0x07DC` — out of address
order, for no reason visible here — and the lift keeps that. And the pops run
ds/es/dx/bx/ax, the exact mirror of the pushes. Neither is load-bearing for a
leaf routine on paper, but "faithful except where I thought it didn't matter" is
how a port stops being checkable.

The `cli`/`sti` pair is deliberately not modelled, and the code says so rather
than silently dropping them.

Adding the per-instruction citations took the guard from 730 to 740 checked, so
the next reader of this row gets the same check for free.

2228 items, 1098 confirmed (49.3%), 1130 open. 740 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #412 — four more hand lifts, and a label that named the wrong family

#411 checked one hand-written lift; four more were sitting at `CELL?`. All four
match their routines instruction for instruction:

  * `func_cef` / `0xCEF` — `xor ax,ax` + int33, `mov ax,2` + int33 (hide),
    `cx=0xC / dx=0xC / ax=0xF` + int33 (mickey ratio), pops mirroring the pushes.
  * `func_2f90` / `0x2F90` — `mov dx,0x3c8 / xor al,al / out / inc dl /
    mov cx,0x300 / rep outsb`. The lift expands `rep outsb` correctly: reads
    `DS:SI`, writes `0x3C9`, increments si, decrements cx.
  * `func_2fa6` / `0x2FA6` — the same head, then `out dx,al` / `loop` writing 768
    ZERO bytes.
  * `func_bff` / `0xBFF` — `ds=cs`, `ax=0x2523 / dx=0x619` + int21, then
    `al=0x24 / dx=0x61A` + int21: INT 23h and INT 24h installed back to back.

THE LABEL AT `0x2FA6` WAS WRONG, and wrong in the way that spreads. It read
`gfx_display_draw_family` — "display-buffer draw primitives (0x2fa6/0x2fbb/
0x3000/0x3066/0x3d7b): lds/les to gs:[0x5221] (display page) and draw a span".
The routine at `0x2FA6` contains no `lds`, no `les`, and never mentions
`gs:[0x5221]`; it is the DAC blank above.

The family is real — `0x2FBB` and `0x3000` both open `lds si, ptr gs:[0x5221]`
and already carry their own labels — but `0x2FA6` is not a member. A FAMILY label
listing five addresses and attached to the first of them is only as good as its
first entry, and this one had swept in the routine that merely sits next to them.

Note which side was right: the port called it `vga_dac_clear` all along. The
labels file was the wrong one, so "check the port against the labels" would have
produced a false correction here. The instruction is the authority, not either
description of it.

2228 items, 1102 confirmed (49.5%), 1126 open. 740 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #413 — the generated lifts already had the strongest proof available

`func_22e0` and `func_a4ed` (`ptr_leaves_gen.rs`, AUTO-GENERATED by
`re/tools/emit_one.py`) sat at `ASM?`. They carry a better warrant than any of the
hand lifts #411/#412 settled: a test that runs the ORIGINAL BYTES through the
interpreter with realistic seeded data and compares the lift's output bit-exactly.

Checked three things rather than trusting the header:

  * The generated per-instruction comments match the image. `0x22E0` really is
    `push bp / push ax / push bx / push cx / push dx / push es`, and `0xA4ED` is
    `push ax / push bx / mov bp, di` — the comments are the disassembly, not a
    paraphrase of it.
  * The oracle tests EXIST and RUN: `func_22e0_matches_interpreter_oracle` and
    `func_a4ed_matches_interpreter_oracle` both pass, neither is `#[ignore]`d.
    That last part matters — 7 tests in this crate are ignored, and a proof that
    does not execute is a comment.
  * The comparison is against the interpreter running the original bytes, not
    against another copy of the lift. `ptr_leaves.rs` also carries
    `native_blend_table_matches_the_lifted_builder`, which is the self-referential
    shape; the oracle tests are the ones that bear on faithfulness.

Settled both. Worth stating the contrast: #411 and #412 settled five lifts by my
reading five routines, which is exactly as reliable as my reading. These two are
settled by execution, and they were the ones already marked provisional.

2228 items, 1104 confirmed (49.6%), 1124 open. 740 citations verified, 0 wrong.
615 lib tests, 0 failures.

## #414 — the prescribed test command runs 615 of 718 tests

Chasing #413's remark about ignored tests, the ignore list turned out clean —
all twelve are diagnostics, dumps or demos (`dump_cutscene`,
`render_all_uncovered_scenes`, `demo_render_full_dialogue_scene`), not disabled
correctness checks. But the COUNT did not add up: grep finds 12 `#[ignore]`s and
`cargo test --lib` reports 7.

The missing five are all in `src/extract/`, which is `mod extract;` in
**src/main.rs** — a module of the BINARY, not the library. So `cargo test --lib`,
the command `CLAUDE.md` prescribes and that every entry this session has quoted,
never compiles them.

    cargo test --release --lib    615 passed,  7 ignored
    cargo test --release          718 passed, 16 ignored

103 tests in `src/extract` plus the `tests/oracle_suite.rs` integration tests were
outside the number I have been reporting. They all PASS — I checked before
writing this, and the workspace has zero failures — so nothing was broken. But
"615 tests, 0 failures" was a smaller claim than it sounded, and #410 settled
`SUBTITLE_X` in `src/extract/render.rs` while citing a test run that could not
have covered that file.

This is the same lesson as #407 two entries apart: a suite is only as good as the
invocation you run it with, and I found both by running it a different way. #407
was a filter that ran FEWER tests and exposed a race; this is the default command
running fewer tests than the codebase has.

Reporting the whole-workspace number from here. Not changing `CLAUDE.md`'s
prescribed command — moving `extract` into the lib is a real refactor and the
right call is the user's — but the status line should not quietly describe half
the suite.

2228 items, 1104 confirmed (49.6%), 1124 open. 740 citations verified, 0 wrong.
718 workspace tests (615 lib + 103 bin), 0 failures.

## #415 — the consumer a shared-invariant test named but could not reach

`decode_vm_words` documents a real fix: an `0xA6` word list may carry a choice
menu after an `0xFFFF` separator, and requiring EVERY offset to resolve made the
function return None for menu-bearing lines, so both call sites skipped them.

The rule is already pinned — `resolving_a_word_list_never_yields_menu_words` in
vm.rs exists for exactly this, and its doc NAMES
`extract::script::decode_vm_words` as one of three consumers that got it wrong.
But that test cannot call this function: `src/extract` is a module of the BINARY
(#414), and vm.rs is in the library. The named consumer had no direct test.

So a test that lists the call sites it protects was protecting two of them and
naming the third. That is a subtle way for coverage to look complete: the
invariant is stated, the offenders are enumerated, and one of them is out of
reach of the assertion.

Added `decode_vm_words_stops_at_the_menu_separator`, which pins three things —
the spoken prefix decodes, menu words never appear, and an unresolvable offset in
the SPOKEN section is still None. That last case matters because the fix narrowed
the resolve requirement rather than removing it, and a test that only checked
"menu lines now decode" would pass on a version that resolved nothing at all.

Not verified, and left in the comment as its own claim: "211 of 3650 A6 lines"
is a measurement of the port's extraction with no test behind it. The RULE is now
guarded; the COUNT is still a remembered number.

2228 items, 1105 confirmed (49.6%), 1123 open. 740 citations verified, 0 wrong.
719 workspace tests, 0 failures.

## #416 — the remembered number was wrong in both digits

#415 left one loose end: `decode_vm_words`'s comment claimed the old bug
"silently dropped 211 of the 3650 A6 lines across the five scripts" — a
measurement with nothing behind it. Measured it.

    A6 lines: 3687, menu-bearing: 214, recovered: 214

Both figures in the comment were wrong. Not catastrophically — the FIX is right
and the shape of the claim held — but a number written from memory and never
re-run is exactly the kind of thing that gets quoted later as if it were counted.

The measurement also forced a distinction the comment blurred. "Dropped" and
"menu-bearing" are not the same set: the old rule returned None when ANY offset
failed to resolve, and `0xFFFF` never resolves, so every menu-bearing line was
dropped — but a line with an unresolvable SPOKEN word was dropped too and is NOT
recovered by this fix. So the test counts three numbers, and the third (214
recovered of 214 menu-bearing) is the one that actually describes what the fix
bought. It happens to be all of them; that is a result, not an assumption.

My first version of the test asserted the remembered numbers and failed — which
is the correct outcome for a test written against a claim rather than against the
data, and how the discrepancy surfaced at all.

2228 items, 1105 confirmed (49.6%), 1123 open. 740 citations verified, 0 wrong.
720 workspace tests, 0 failures.

## #417 — the wrong number had spread to two more files

#416 measured `decode_vm_words`'s "211 of the 3650" and got 214 of 3687. Swept
for the same shape — `N of M` in a comment — and found the WRONG figure repeated
verbatim in two more places: `engine.rs:3592` (the subtitle builder's bug
description) and `engine.rs:7258` (the choice-menu doc). Both corrected to the
counted values, with the old figures named so the correction is legible.

That is the real cost of an unverified number: it gets copied. Three files
asserted the same measurement and none of them had run it.

The sweep found five other `N of M` claims. Checked the cheapest one to check:
`vm.rs:10306`, "the shipped corpus uses 32 of the 51 implemented opcodes". The
enclosing test ALREADY computes that set from the five shipped `.COD` files and
already asserts `0xD3` is absent from it — so the number was one line from being
checked and nobody had written the line. Added `assert_eq!(seen.len(), 32)`. It
passes, and it is not vacuous: the assertion sits after the test's
`if checked == 0 { return }` guard, so it only runs when the scripts are present,
and they are in this checkout.

The remaining four (`palette.rs` 68/256, `levels.rs` 13/50, `bloodprg.rs` 30/51,
`vm.rs` 630/640) are left as-is. Two of them cite the tool that produced them,
which is weaker than a test but is not nothing; recorded here as the queue rather
than fixed silently.

2228 items, 1105 confirmed (49.6%), 1123 open. 740 citations verified, 0 wrong.
720 workspace tests, 0 failures.

## #418 — a distribution that never summed to its own total

Working #417's queue of unverified `N of M` claims.

`630 of the 640 kind-1 objects` (vm.rs) is CORRECT. Ran the tool it cites and it
reproduces exactly: "TOTAL kind-1 objects 640: 630 carry their name inline at +4,
10 do not", and the ten misses are `blood` and `orxx` in each of the five scripts
— which is precisely the explanation the comment gives. A cited tool that still
runs and still agrees is the strongest of the four.

`EXT_WORLD_MAGIC`'s distribution (levels.rs) was NOT correct, in a way that is
almost embarrassing to state: byte 7 was described as `0x81` in 37 files, `0x80`
in 10, `0x84` in one and `0x8B` in another. **37 + 10 + 1 + 1 = 49**, and there
are 50 shipped worlds. Nobody had added it up. Counting the files finds a fifth
value — one world with `0x00` — that had simply never been noticed.

Everything else in that doc holds: 50 files, all three leading bytes `02 00 00`,
byte 3 is `0x01` in 49 and `0x02` in one, and the rejection count (50 − 37 = 13)
was right regardless, because it depends only on the 37.

`ext_world_header_byte_distribution` now measures all of it, and asserts
`b7.values().sum() == total` explicitly — the check that would have caught this
the day it was written. That assertion is the point of the entry: a distribution
is the one kind of claim that can be validated against ITSELF, with no oracle and
no binary, just by adding it up.

Two of #417's four remain: `palette.rs` 68/256 and `bloodprg.rs` 30/51.

2228 items, 1105 confirmed (49.6%), 1123 open. 740 citations verified, 0 wrong.
721 workspace tests, 0 failures.

## #419 — the last two numbers: one right, one off by one in the denominator

Closing #417's queue.

`palette.rs`'s "a squared-RGB nearest search reproduces only 68 of the 256
entries" is CORRECT — 68 exactly. But it was guarded by `assert!(agree < 128)`,
a BOUND, not a measurement: the test would have passed at 67, at 12, at 127. The
figure in the doc had never been checked by the test that exists to protect it.
Now `assert_eq!(agree, 68)`.

`bloodprg.rs`'s "only 30 of the 51 adjacent pairs ascend" has the right numerator
and the wrong denominator. The table has 52 slots, the last (`0xD3`) NULL, so 51
LIVE entries — which form FIFTY adjacent pairs, not 51. The error is visible
without touching the data: n entries have n−1 gaps. 30 of 50 is now asserted.

That is the third arithmetic slip of this shape in four entries (#416's 3650,
#418's distribution summing to 49, this denominator), and none of them needed the
binary to catch — only addition. The pattern is that a number written while
LOOKING at something is trusted forever after, and the cheapest checks (does the
distribution sum? does n−1 match?) are the ones nobody runs because the claim
already feels observed.

All four of #417's claims are now closed: two were right (630/640, 68/256) and
two were wrong (the byte-7 spread, this denominator). Every one is asserted rather
than stated.

2228 items, 1105 confirmed (49.6%), 1123 open. 740 citations verified, 0 wrong.
722 workspace tests, 0 failures.

## #420 — a split provenance, asserted in both directions

`GAME_SCREEN_PALETTE_DAC_LOWER_IS_BINARY` (= 128) claims the palette's provenance
splits: colours 0..127 are the baked DAC at file `0x12F78`, colours 128..191 are
a savestate capture and an acknowledged APPROX.

`palette_lower_half_matches_the_baked_dac_in_the_image` proves BOTH halves, and
the second one is what makes this row settleable:

    assert_eq!(&DAC[..n], &exe[BAKED..BAKED + n])          // the lower half IS baked
    assert_ne!(&DAC[n..n + 192], &exe[BAKED + n..+192])    // the upper half is NOT

An `assert_ne!` is an unusual thing to want, and here it is exactly right: it
pins the DEFECT in place so nobody can later "tidy" the doc by claiming the whole
table came from the image. Its failure message says what to do if it ever passes
— retire the APPROX, because that would mean the capture had been replaced by
real data. A test that tells you what its own failure MEANS is rarer than it
should be.

Settled DATA on the strength of the equality half. The palette constant itself
stays UNVERIFIED, correctly: its upper bank is still scene state frozen into a
global, and #419's neighbours in this same file (the 68/256 count, the console
bank remap) are the parts that were checkable and are now checked.

Worth noting the doc's own strongest argument is not the byte comparison but the
independent corroboration: `engine.rs` had narrowed its hand-palette installs to
`202..=251` after a rendering defect, landing on the same boundary from the
opposite direction. Two unrelated lines of evidence agreeing is the shape this
project keeps finding most trustworthy — #388's orb box, #409's camera origin,
and this.

2228 items, 1106 confirmed (49.6%), 1122 open. 740 citations verified, 0 wrong.
722 workspace tests, 0 failures.

## #421 — what the remaining 901 rows actually are

Every entry this session has worked the PROVISIONAL queue (`ASM?`/`CELL?`/`DATA?`
/`ORACLE?`, 221 left). The other 901 open rows are `UNVERIFIED`, and I had not
looked at what they contain. Measured it:

    0 of 901 cite a binary address in their evidence
    662 of 901 have NO doc comment at all
    239 have prose but no address
    kinds: 473 fn, 305 const, 110 struct, 9 enum, 4 static

So the two halves of the open ledger are NOT the same kind of work, and the
percentage has been hiding that. A provisional row is a CLAIM someone wrote and
nobody rechecked — the work is verification, which is why this session has been
closing them at a few per entry, and why several turned out wrong (#382, #386,
#400, #412, #418). An `UNVERIFIED` row is mostly an item with nothing to check:
no citation, usually not even a sentence.

Those 901 are a mix, and a sample makes the mix concrete: `frame_header`
(tbbig — parses the very header #388 verified), `video_scenes` (parses shipped
DESCRIPT data), `Ship3dDepthState` (one of #404's 18 types nothing constructs),
`render` in audio.rs and `find` in progress.rs (port scaffolding with no binary
counterpart at all), and bare consts with no doc.

That matters for what "100%" means. Some of these need a decode. Some need a
sentence and a citation for a decode that already happened elsewhere — the
`tbbig::frame_header` case, where #388 verified the layout and the FUNCTION row
still reads UNVERIFIED. And some are port infrastructure that will never have an
address, for which "verified against the binary" is not a coherent goal; the
honest end state for those is a stated classification, not a citation.

Recording this as a finding rather than acting on it, because the right move —
splitting `UNVERIFIED` into "needs a decode" and "port-internal, no counterpart"
— changes what the ledger's denominator MEANS, and that is a call about how this
project reports itself rather than a defect to fix quietly.

2228 items, 1106 confirmed (49.6%), 1122 open (901 UNVERIFIED + 221 provisional).
740 citations verified, 0 wrong. 722 workspace tests, 0 failures.

## #422 — the ledger was stale, and refreshing it moved the denominator

#421 named `tbbig::frame_header` as the clearest case of a row reading UNVERIFIED
for a decode that had already happened: #388 verified that exact 10-byte layout
from the game's hit test. Wrote the citation into the function's doc — and
`audit_settle` REFUSED it, "ASM needs a cited address".

The refusal was right. The ledger's `origin` column is a SNAPSHOT taken when
`audit_inventory.py` last ran; it does not re-read the source. So a doc comment
added after that run is invisible to the settle tool, and every row I have
enriched this session was being checked against its stale evidence.

Re-ran the inventory. Three consequences, all worth stating rather than absorbing:

  * `frame_header` picked up its citations and settled — 900 UNVERIFIED now.
  * THE ITEM TOTAL ROSE 2228 -> 2231, because the refresh sees code that did not
    exist when the ledger was built. Two of the three new rows are mine:
    `AlienStreams` and its `new` (#401).
  * Three settled statuses were DROPPED as ambiguous, and the tool said so
    loudly. Only `croolis.rs` really lost one: adding `AlienStreams::new` made
    `new` a THREE-way name collision in that file, so the settled `TESTED` on
    `AlienObject::new` could no longer be matched to a line. Re-settled by line.

So the percentage moved for a reason that has nothing to do with verification:
1106/2228 (49.6%) became 1107/2231 (49.6%). Adding code adds rows. A ledger
percentage rises with work done and falls with work discovered, and this session
has been quoting it every entry as though only the first happens.

Also noted, not fixed: the origin extractor treats any `0x...` in a doc as an
address. `AlienColony::new` now shows `origin=0x105C,0x9E3B`, but `0x9E3B` is the
old per-object seed multiplier that #400 DELETED — it appears only because my
comment explains what used to be there. A citation extractor cannot tell a
historical mention from a claim.

2231 items, 1107 confirmed (49.6%), 1124 open. 742 citations verified, 0 wrong.
722 workspace tests, 0 failures.

## #423 — my own change tripped a guard, and the guard was right

The ledger refresh in #422 broke `no_decoded_rule_is_implemented_twice_under_one_name`.
I committed before reading the test output, which is how I found out — the wrong
way round, and worth recording as such.

TWO PROBLEMS, one mine and one the tool's.

The tool's: refreshing `origin` from a session's worth of new doc comments swept
in ordinary numbers as addresses — `0x0`, `0x100`, `0x181`. Real code addresses in
this image are >= `0x600` (the MZ header bound #387 already established for
labels.csv), so `audit_inventory.py` now drops anything below it. That removed 5
of the 65 reported clusters.

Mine: the actual FAILURE was `0x0105c  new  in src/croolis.rs, src/engine.rs` —
one NAME implemented twice for one address. #400/#401 had me cite `fs:[0x105C]`
in BOTH `AlienColony::new` and the engine's constructor, because I gave both the
shared-streams field. The guard exists for exactly that shape: it was written
after `subtitle_draw_glyph` turned up in two files with the second copy carrying
three already-fixed defects.

Here the engine is a CALLER, not a second implementation — but the fix is not to
teach the guard about my special case. It is that the engine should never have
cited the address: the decode belongs to `AlienStreams`, and a holder pointing at
the type says so more accurately than a holder repeating the citation. Reworded;
the guard passes on its own terms rather than being relaxed.

The remaining 60 address clusters are reported and do NOT fail, which the test's
own doc says is intended — "a routine and its helper citing one address is
normal". Better citation coverage necessarily produces more of them, and the
guard was designed knowing that.

2231 items, 1107 confirmed (49.6%), 1124 open. 742 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #424 — four rows settled on work already done, and one descriptor decoded

After #422's refresh, three rows this session had already verified by hand became
settleable because their citations were finally visible to the tool:
`clear_menu_selection` (#396's `0x87B0`/`0x883B` epilogue), `AlienStreams` (#401's
`fs:[0x105C]` / `cs:[0x16A2]`), and `AlienColony` (#403's `0x12DE` dispatcher).
`croolis::step` stays open — #402 left it provisional for a stated reason and a
refresh does not change that.

`MenuAnimDescriptor` (manu3) was new work. Its doc described a packed word and two
fields; `XDB:manu3:0x01DF..0x01FE` confirms every part:

    0x01DF  mov si, word ptr [0x102e]     the descriptor pointer
    0x01E3  movzx ecx, word ptr [si]      the packed word...
    0x01E7  or cl, cl / je                ...low byte = frame COUNT, 0 ends the list
    0x01EB  cmp ch, byte ptr [0x102c]     ...high byte = PHASE, gated on 0x102C
    0x01F8  mov bp, word ptr [si + 4]     the TARGET field address
    0x01FE  mov ax, word ptr [si + 6]     the END value

The packing is the part worth having in the file: `phase<<8 | count` is one
`movzx` and then `cl`/`ch` used separately, which is invisible from the struct's
two `u8` fields. A reimplementation reading two bytes in the wrong order would
produce a descriptor that parses and animates nothing.

2231 items, 1111 confirmed (49.8%), 1120 open. 749 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #425 — a dword stored, a word read, and one axis that rounds differently

`manu3_hand::compose` cites `0x2274`, which is a DATA SEGMENT offset — the live
manu3 dump — so the row's real evidence is the code that defines the layout, at
`XDB:manu3:0x03DE`. Read it. The record layout the module header documents is
exact:

    0x03E2  movsx ebx, word ptr [di + 0x54]   the speed, a WORD
    0x03EE  sar eax, 0x10                     ...>> 16 per axis
    0x03F2  add dword ptr [di + 0x42], eax    L.x
    0x0405  add dword ptr [di + 0x46], eax    L.y
    0x0414  add dword ptr [di + 0x4a], eax    L.z

TWO DETAILS THE STRUCT CANNOT SHOW.

`L` is accumulated as a DWORD and read back as a WORD for the transform —
`movsx ebx, word ptr [di + 0x42]` @`0x041A`. The port already gets this right and
knew why: `compose` uses `st16` on those fields with a comment noting the authored
values have nonzero high words. Good, but it was an unattributed decision; the
address is now on it.

AND ONLY THE Y AXIS ROUNDS. `adc eax, 0` @`0x0401` follows Y's shift; X (`0x03EE`)
and Z (`0x0410`) have no such instruction. That is exactly the sort of asymmetry a
reimplementation tidies into consistency — three axes, three identical lines —
and it is now written down as undecided-but-real rather than smoothed away. I am
not claiming to know whether it is deliberate rounding or a compiler artefact;
the claim is only that the game does it on one axis and not the others.

2231 items, 1112 confirmed (49.8%), 1119 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #426 — deleting a fabricated surface its own doc had condemned

`render_star_map_navview`'s doc said it plainly: "a VISUAL APPROXIMATION",
"SUPERSEDED AND NOT LIVE (audit-fixes #277)", reproducing the game's composition
"without the exact recovered geometry/projection", reached "only from each other
and from tests" — and "the end state is removal once those tests point at the
projected renderer".

That precondition was ALREADY MET and nobody had checked. Its two tests assert
that pyramids and an orb are drawn and that the grid pans with heading;
`projected_navview_draws_perspective_grid_and_orb` asserts exactly those three
things plus the pan, against `render_star_map_navview_projected` — the renderer
using the projection decoded from `0x9BBA` and verified instruction by
instruction in #273, and the one `engine.rs` actually calls.

So the approximation's tests were not protecting coverage; they were testing that
an invention still draws its invention. Removed both functions and both tests,
116 lines. 614 lib tests pass, the workspace is clean, and the ledger drops from
2231 items to 2229.

This is #385's shape a second time — a decoded surface and a fabricated one
sitting side by side, the fabricated one kept because deleting things feels like
losing work. It is the opposite: under the prime rule a plausible surface next to
a real one is the defect, and its tests make it look maintained. The doc had
already reasoned all of this out; what was missing was somebody doing it.

2229 items, 1112 confirmed (49.9%), 1117 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #427 — the fabricated-surface sweep, and one row that must NOT be deleted

After #426 I swept for the same shape — docs saying "VISUAL APPROXIMATION",
"FABRICATED", "NOT LIVE", "stopgap". Six hits, four of them already-resolved
prose ("no fabricated stand-in", "replacing the old fabricated 7x4 grid").

One was live and instructive: `NAV_DEST_X` and its three siblings, whose doc opens
"APPROX — FABRICATED LAYOUT, and the decoded replacement already exists." Read on
its own that is a delete, exactly like #426. It would have been WRONG.

`#240` had already traced what fills that list: `nav_destinations` is built in
main.rs from the SCRIPT3..5 BUNDLES, so it is a PORT-SIDE AFFORDANCE for reaching
scenes — not a second model of the game's destination list, which comes from the
DEB candidate records (`0x7259`) and routes through `console_box`. The doc says so
explicitly, and says why the earlier reading (#239, "a second layout for a game
surface") was too broad.

So the sweep's value here was NEGATIVE, and that is worth as much as a deletion:
two surfaces documented in nearly identical language, one of which had to go and
one of which had to stay, distinguished only by someone having traced the data
that feeds it.

Reclassified it `INFRA` rather than leaving it provisional. `ASM?` was simply the
wrong question: these four numbers have no binary counterpart and never will, so
no amount of decoding closes the row. `INFRA` is an existing settled category with
97 members meaning exactly that. This is #421's point applied to one row — holding
an unverifiable-by-construction item in the open queue misrepresents what is left.
The doc keeps its APPROX label, because the numbers are still invented and a
reader must not mistake them for decoded ones.

2229 items, 1113 confirmed (49.9%), 1116 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #428 — the citation that belonged to the function above it

`nav_world_label_sample` sat at `ASM?` citing `0x9BBA`, `0x100000` and `0x4F09`.
It cites nothing: its body is `self.nav_world_labels.iter().take(7)`. The ledger
had picked up the PRECEDING doc block — the star-map pyramid renderer's — because
the two comments run together with no blank line between them, so the inventory's
"the doc above an item" rule swept in a paragraph belonging to its neighbour.

That is the same failure #381 found in `whatis.py` (nearest-preceding-label
attribution) and #412 found in a labels.csv family entry, now in a third tool. In
all three the mechanism is identical: an address near a thing is not an address
about that thing.

Gave it its own doc — separated by the blank line the extractor needs — saying
plainly that there is no binary counterpart, that the citations belong to the
renderer above, and that what IS data-backed is the CONTENT: `nav_world_labels`
comes from `levels::primary_worlds()`, which
`level_directory_literal_matches_the_image` holds to the bytes at file `0xCDF4`.
The `7` is the helper's own.

Classified INFRA. The ledger crosses 50.0% on this row, which is worth naming for
what it is: this was a MISFILED row, not a decoded one. The number moved because
the classification got more accurate, not because more of the game is understood
— and #421 is the reason that distinction is now visible at all.

2229 items, 1114 confirmed (50.0%), 1115 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #429 — the hand's laws are decoded; its data is a savestate

`manu3_hand.rs` opens "Everything here is decoded from manu3.xdb's own code". The
LAWS are — #425 verified the composition and the integrator instruction by
instruction. The DATA is not, and nothing said so.

Measured it. Neither blob the module `include_bytes!`s appears in the shipped
overlay:

    node-tree state ds[0x2274..0x2974]   searched verbatim in manu3.xdb -> not found
    seg2 vertex/face pool                not found; not even its first 32 bytes
    dump vs shipped overlay              16192 of 62544 bytes agree (26%)

So `accuracy/manu3/manu3_ds.bin` and `manu3_seg2_1b76.bin` are RUNTIME STATE
lifted from a savestate. The port does not parse shipped data for the hand's mesh
or skeleton; it ships a capture of the game's memory.

That is the prime rule's defect, and it was hiding behind precise language about
everything ELSE. `STATE_BASE`'s own doc is scrupulous — it says outright that no
instruction names `0x2274` and that it was searched for as an immediate AND a
displacement — but the module header above it still claimed the whole file was
decoded, and `docs/port-validation.md` graded the row **ASM+DATA (live, exact)**.
"DATA" in that matrix means shipped-file provenance. These blobs have none.

Corrected both: the header now states the split and names the open task, and the
matrix row reads **ASM (laws) + APPROX (data)** with the measurement in it.

The open task is real and stated: something fills that data segment, and 26%
agreement with the shipped file says part of it IS the file's initial content. Find
the initialiser and the port can build the state instead of shipping a photograph
of it.

2229 items, 1114 confirmed (50.0%), 1115 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #430 — the savestate is the shipped file, shifted by 0x1370

#429 ended with an open task: something fills the manu3 data segment, and 26% of
it matches the shipped overlay. Worked it, and the 26% was an artefact of my own
measurement — I had compared the two files AT THE SAME OFFSET.

Scanning 64-byte chunks of the dump for their position in `manu3.xdb` gives a
dominant delta of **4976 (0x1370)**, 422 of 425 hits. Re-measuring with the shift:

    ds[i] == xdb[i + 0x1370]   52698 / 57568 bytes = 91%

    seg2 vertex/face pool  0x1B76..0x2274   1769/1790  (98.8%)
    node tree              0x2274..0x2974   1301/1792  (72.6%)
    fs:0x2300 pool         0x2300..0x2400    226/256   (88%)

So the data segment IS the shipped overlay, loaded with its first 4976 bytes (the
code) skipped. The MESH is shipped data, not a capture — the port could read it
from `manu3.xdb` directly. Only the node tree diverges materially, and it is the
one block the pose tweens write, so its 27% divergence is the live animation
state and nothing more.

That converts #429's standing defect from "we ship a photograph of memory and do
not know why" into a mechanical change: read the mesh from the file at `+0x1370`;
keep only the node tree as initial state, and even that is 73% file content.

THE LESSON IS ABOUT THE MEASUREMENT, not the overlay. #429 reported "26% byte
agreement" and drew a correct conclusion from it (the blobs are not verbatim in
the file) but an incorrect implication (therefore they are runtime state). A
same-offset comparison of a loaded segment against its file cannot show a load
shift — the number was real and the inference was wrong, and one more scan
settled it.

2229 items, 1114 confirmed (50.0%), 1115 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #431 — correcting #430: it was the vertices, not the mesh

#430 concluded "the MESH is shipped data the port could read straight from
manu3.xdb". That is half right, and I published it one entry ago, so it gets a
correction rather than a quiet edit.

What #430 measured was the range the module calls the "seg2 vertex/face pool",
`0x1B76..0x2274`. Splitting it finds the boundary exactly:

    VERTEX POOL   0x1B76..0x223A   1732/1732   the file, byte for byte
    tail          0x223A..0x2274     37/58     the ROW SCRATCH -- 0x2258, 0x2264
                                               and 0x2270 are the three dwords
                                               #425's integrator reads each frame

So the vertices are shipped data with ZERO differences, and the 21 "differences"
#430 counted were an adjacent live scratch area, not mesh at all. That half is
stronger than #430 claimed.

THE FACE LIST IS THE PART THAT WAS WRONG. It lives at `0x0B18`, not in the range
#430 measured, and it is 1527/1728 (88%). I tested two explanations and both
failed: the differences are NOT confined to the `link` field (they spread evenly
over all eight byte positions, 34/14/33/19/34/16/35/16), and the list is NOT a
permutation of the file's (as multisets the two differ; 170/216 records match in
place, 189 appear anywhere at all, 171 vertex triples match in place). About 27
records differ in CONTENT for a reason not yet decoded.

So the correct statement is narrower than #430's: the VERTEX POOL has a
shipped-file provenance and can be read from `manu3.xdb`; the FACE LIST does not
yet. The module header and the validation row now say that.

Two hypotheses tested and rejected is a better outcome than one asserted — but
the reason this needed correcting is that #430 generalised from the range it had
measured to a word ("mesh") covering a range it had not.

2229 items, 1114 confirmed (50.0%), 1115 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #432 — two entries in a row measured the wrong blob

This module includes TWO captures. `DS` is the data segment; `SEG2` is a second
segment and is where the MESH lives — the vertex pool and face list the renderer
reads. #430 and #431 both measured `DS` while writing conclusions about the mesh.
They are not the same bytes: `seg2[i] == ds[0x1B76 + i]` agrees only 30%, despite
the capture's filename (`manu3_seg2_1b76.bin`) inviting exactly that assumption.

Measured properly. Each blob is the shipped overlay at its OWN fixed shift:

    DS    ds[i]   == xdb[i + 0x1370]    52698/57568  (91%)
    SEG2  seg2[i] == xdb[i + 0x50A0]    38141/41904  (91%)   266/266 chunks agree
                                                              on the delta

and within `SEG2`, where the mesh actually is:

    vertex pool  0x0000..0x0B18  (110 x 20B)   2228/2840  (78%)
    face list    0x0B18..0x11D8  (216 x 8B)    1362/1728  (78%)

So the corrected picture is simpler than either of the last two entries: BOTH
captures are overwhelmingly the shipped file loaded at a fixed offset, and NEITHER
mesh region is verbatim. #430's "the mesh is shipped data" was wrong. #431's
correction of it was ALSO wrong — its face-list analysis (link-position spread,
the permutation test) ran on `DS:0x0B18`, which is not the face list at all.

What survives: the SHIFTS are real and cleanly measured, `DS`'s vertex-pool region
really is byte-identical to the file, and `DS:0x2258/0x2264/0x2270` really are the
integrator's three dwords. What does not: any claim that the port's mesh has a
shipped-file provenance. It is 78% the file and the rest is unexplained.

THE MISTAKE WAS THE SAME BOTH TIMES and it is worth naming precisely: I measured a
blob I had, and wrote about a blob I meant. The filename encouraged it, but the
check that would have caught it — "is this the array the renderer actually reads?"
— costs one grep and I did not do it until the third pass.

2229 items, 1114 confirmed (50.0%), 1115 open. 752 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #433 — a cursor cell cited one digit off

`move_mouse_rel`'s doc explains why the runtime feeds RELATIVE motion: the bridge
warps the hardware cursor every frame, so an absolute injection fights the warp.
All three cited routines check out —

    0x9722  mov ax, 4 / int 0x33          INT 33h fn 4, set-position: the re-centre
    0x97FD  mov bx, word ptr [0xa2a]      the cursor cell...
    0x9801  sub bx, word ptr [0x27a7]     ...rebased against 0x27A7
    0x0D0E  poll_mouse                    the per-frame poll

— except that the doc named the cell `gs:0x2A2A`. The instruction reads `0x0A2A`:
the encoding is `8b 1e 2a 0a`, and `0x0A2A` is the mouse-X cell the eye-orb hit
test reads (`mov ax, word ptr [0xa2a]` @`0x8271`, verified in #388). `0x2A2A` is a
different address that happens to be one hex digit away.

That is the sort of error a citation guard cannot catch: `check_cited_instructions`
verifies an address only when a mnemonic is quoted beside it, and this one was
prose. It survived because `0x2A2A` LOOKS like a plausible DS cell and sits in the
same neighbourhood as the bridge's `0x2A19`/`0x2A1B` — the console selection and
the station table, both real addresses this session has worked with. A wrong digit
that lands in a familiar range reads as correct.

Fixed, with the encoding quoted so the next reader can check it without
disassembling, and cross-referenced to #388 where the same cell was decoded from
the other side. Now under the guard: 752 -> 756 checked.

2229 items, 1115 confirmed (50.0%), 1114 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #434 — checking the citations a guard cannot see, and two holes in the tool built to prevent this

#433 found a DS cell cited one digit wrong and noted why nothing caught it:
`check_cited_instructions.py` verifies an address only when a MNEMONIC is quoted
beside it. Prose citations — "rebases `gs:0x2A2A` against `gs:0x27A7`" — are
unchecked. `tools/check_cited_cells.py` now asks the automatable half of that
question: is the cited cell touched by ANY instruction in the image?

Its first run reported **78 of 223 cells untouched**, which is not a finding, it
is a broken tool. Two rounds of chasing that number found the cause, and the cause
is the interesting part:

  * `reg_disp_census` enumerates OPCODES and omits `8A` (byte loads). The
    `vm_field_offset` matrix at `gs:[bx+0x6D60]` — `65 8a 87 60 6d` @`0x6023`,
    a routine this session has cited repeatedly — came back untouched.
  * `address_forms` has no `C4`/`C5` (`les`/`lds`). `les di, gs:[0x6724]` is the
    VM's record-table pointer, read at `0x6B4D` and a dozen other sites, and
    census reported ZERO for it.

`re/tools/addr_forms.py` EXISTS BECAUSE OF THIS FAILURE MODE. #359 built it after
#335 found a one-encoding scan under-reporting, and #403 hit the same thing a
third time. The tool written to stop enumerating opcode families still enumerates
opcode families, one level down. Both gaps are now recorded in it.

The fix in the checker is to match the MODRM instead: `mod=00, rm=110` is a direct
address and `mod=10` is a reg+disp16, whatever opcode carries it. 78 untouched
became 9.

ALL NINE ARE EXPLAINED, so this sweep found no new defect: `0x22EC`, `0x2300` and
`0x6400` are OVERLAY cells (croolis/manu3) that are correctly absent from
BLOODPRG.EXE, and `DS:0x2573` is a STRING that only a pointer list at `DS:0x2567`
names — no instruction ever mentions it, and none should. A negative result, but
the tool that produced it is now sound enough for the next citation to be checked
by machine instead of by eye.

2229 items, 1115 confirmed (50.0%), 1114 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #435 — I nearly reported a correct doc as wrong, from my own truncated output

`Runtime::run`'s subtitle-persistence comment claims the game's reveal draw is
gated on `gs:[0x27e2]&2`, and that "the one-shot present (0xbe29) sets 27e2=2 then
clears it". Checking it: `0xBE11` is `mov byte ptr gs:[0xba0], 1` exactly as
documented, and `0x93FA` is `test byte ptr [0x27e2], 2` — the gate.

Then `0xBE29` disassembled as `mov word ptr [0x5e58], ax`, not a `0x27E2` write.
I ran a census of `0x27E2`, printed it, saw writes of 1 and 0 but none of 2, and
had "the doc claims a write the game never makes" half-composed.

I HAD PRINTED `[:6]` OF AN 8-ELEMENT LIST. The last two entries are
`mov byte ptr [0x27e2], 2` @`0xBE32` and `mov byte ptr [0x27e2], 0` @`0xBE5C` —
precisely the set-then-clear the doc describes, inside the routine it names.

This is #308 exactly: reasoning from a truncated view of a tool's output. That
entry drew the conclusion "run the tool, paste the number, do not predict it" for
counts; the same discipline applies to LISTS, and slicing one for readability
while treating it as complete is the same error wearing different clothes. #319
and #335 are the same family. The only reason it did not ship this time is that
"a flag tested but never set" was implausible enough to check once more.

The doc was right. Enriched it with the two write addresses and the gate test so
the next reader does not re-derive them, and settled the row.

2229 items, 1116 confirmed (50.1%), 1113 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #436 — the structural fix for the truncation error, four entries late

#308, #319, #335 and #435 are one error: reasoning from a SLICED view of a tool's
output as though it were the whole. #435 nearly published "the game never writes
this flag" from `[:6]` of an eight-element census whose last two entries were the
writes in question.

#373 solved the same failure for COUNTS structurally — the status line is
generated by `audit_status.py`, so it cannot be typed from memory. `show_census`
is that fix for LISTS: it prints the TOTAL before any rows and states the omitted
tail explicitly, so a slice cannot look complete.

    8 site(s) for 0x27e2
      0x08338  or byte [m],i8   SET  imm=1
      0x0833f  mov byte [m],i8  W    imm=0
      0x08347  test byte [m],i8 R    imm=3
      ... 5 MORE NOT SHOWN

Whether I actually use it is the open question — the discipline "print the count"
was available all four times. What makes this worth doing anyway is #373's
precedent: after the generator existed, the count slips stopped. A helper that is
easier to call than a hand-rolled loop gets called.

2229 items, 1116 confirmed (50.1%), 1113 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #437 — the underscore, settled from the data instead of the renderer

`PHONE_CONTACTS` has carried `"BOB MORLOCK"` through three entries. #327 tried to
justify it from a case-folding loop; #328 withdrew that (the loop has zero callers
and folds a DOS file read), keeping only the strong negative — NO instruction in
the image compares against `0x5F`, so nothing can special-case an underscore. It
then stopped, "because the caption renderer is still unfound".

THE RENDERER IS NOT THE ONLY EVIDENCE. Searching all 261 shipped files:

    'Bob Morlock' / 'BOB MORLOCK' / 'bob morlock'   0 files
    'Bob_Morlock'                                   31 files

including `DESCRIPT.DES` (`EBob_Morlock`, a tagged record) and `SCRIPT2.DIC`
@`0x462F`. The spaced spelling exists nowhere in the game. Combined with the
`0x5F` negative — nothing in the image can fold an underscore away — there is no
route by which the game's own `Bob_Morlock` reaches a screen as `BOB MORLOCK`.

Corrected. The prime rule's test is not "did we find the renderer" but "does this
literal come from the game", and that question had a cheap answer for three
entries. #328 was right to withdraw a bad argument and right that the renderer
matters — but "we cannot confirm it the way I was looking" is not the same as
"we cannot confirm it".

The other eight entries are single words with no separator, so this was the only
one at risk. Their provenance is still a transcription and the row stays open.

2229 items, 1116 confirmed (50.1%), 1113 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #438 — two more shortened names, and the reason I did not change them

Applying #437's method to the other eight contacts: all nine names exist in the
shipped files, so none is invented. But `DESCRIPT.DES` carries a tagged
character-name table —

    0x084D  2Hom            0x08B9  8Jerry_Khan      0x0912  ;Tina_Burner
    0x0991  AMaxxon         0x09B5  CIzwalito        0x09EB  EBob_Morlock

— alongside `Super_Tromp`, `Anna_Haf`, `Kran_Dobu`, `Otto_Von_Smile` and two
dozen more. So the port's `JERRY` and `TINA` are SHORT FORMS of the game's
`Jerry_Khan` and `Tina_Burner`.

I DID NOT CHANGE THEM, and the reason is the interesting half. #437 was safe
because `Bob Morlock` with a space appears in ZERO of 261 shipped files — the
spelling was invented, full stop. `JERRY` cannot be ruled out that way: it is a
real substring of a real name, and a phone UI displaying a first name where the
character table holds a full one is an ordinary design choice. Changing a
SEPARATOR the game cannot produce is a correction; shortening or lengthening a
display name on the strength of a neighbouring table is a guess.

What this does establish is the fix's shape. The port already parses that table —
`DescriptDb::character_names()` has existed all along — so `PHONE_CONTACTS` can
be sourced from data rather than transcribed, once the phone's own record list is
decoded. That is now written where the table is, so the next pass starts from the
parser instead of the literal.

2229 items, 1116 confirmed (50.1%), 1113 open. 756 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #439 — the caption path #328 called unfound

#328 left the phone captions open with "the caption renderer is still unfound".
#438 declined to shorten or lengthen names without it. It was one handler away.

`nav_choice_handler_2` (`0x87BD`) builds the contact menu:

    0x87C5  mov si, 0x6d3e        the contact SOURCE list
    0x87C8  mov di, 0x2b13        the menu it builds
    0x87CB  lodsw / or ax,ax / je   a ZERO slot is SKIPPED -- an empty contact
    0x87D0  cmp ax,-1 / je          0xFFFF ends the list
    0x87D5  add ax, 4               <-- the entry is an OBJECT OFFSET...
    0x87D8  stosw                       ...and +4 is its INLINE NAME

That `add ax, 4` is the whole answer. #418 verified that 630 of 640 kind-1 objects
hold their DEB name at `+4`; this handler turns each contact's object offset into
exactly that pointer. A contact's caption IS its object's inline name.

Two confirmations followed. `DS:0x6D3E` is all zeros in the shipped image, so the
list is runtime state filled as crew become callable — which is what that
handler's label had claimed without the mechanism. And the `.VAR` records carry
the names in full: `Bob_Morlock` at SCRIPT1.VAR +78, which is object `0x4A` + 4,
the same object the inline-name tool names. `Jerry_Khan` (+726), `Tina_Burner`
(+1806), `Maxxon`, `Izwalito` and `Hom` are all present.

So `JERRY` and `TINA` were short forms of names the game stores in full, and are
now `JERRY_KHAN` and `TINA_BURNER` — corrected on the same footing as #437's
underscore, but with the mechanism rather than an absence argument.

#438 was right to refuse the change on the evidence it had, and wrong that the
evidence was unavailable: I stopped at "the caption path is unfound" without
asking which routine BUILDS the menu, when #386's own census had already put
handler_2 in front of me twice.

STILL OPEN: `Migrax` and `Hanz` appear in no `.VAR`, so two entries have no object
backing them. The table stays a literal until the runtime slot list is modelled.

2229 items, 1116 confirmed (50.1%), 1113 open. 762 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #440 — the APPROX row the prime rule requires

`CLAUDE.md` is specific about literals like `PHONE_CONTACTS`: a stand-in "may
stand in temporarily ONLY if the row in docs/port-validation.md explicitly labels
it APPROX with the binary routine that must replace it". That row did not exist.
The table has been a transcription through #326, #327, #328, #437, #438 and #439
— six entries — and the matrix graded its screen `DATA+ORACLE` with no mention of
the literal at all.

Written now, with what #439 decoded as the replacement: `nav_choice_handler_2`
(`0x87BD`) walks the runtime slot list at `DS:0x6D3E` — zero = empty slot,
`0xFFFF` terminates — and `add ax, 4` @`0x87D5` turns each entry into its object's
INLINE NAME, the `+4` field verified for 630/640 objects in #418. That is not a
vague "should come from data"; it is the routine, the list, and the field.

The row also records what is still unaccounted for: `Migrax` and `Hanz` appear in
no `.VAR`, so two of the nine entries have no object behind them.

Settled the ledger row ASM — its CITATIONS are now verified end to end — while the
CONTENT defect is tracked where the prime rule says it belongs. Those are two
different axes, and conflating them is how a transcription survives six entries of
scrutiny: every pass checked whether the addresses were right, and none asked
whether the table should exist.

2229 items, 1117 confirmed (50.1%), 1112 open. 762 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #441 — enforcing the rule that let a literal survive six entries

#440 found `PHONE_CONTACTS` had gone six audit entries as a transcription with no
APPROX row, because every pass checked whether its ADDRESSES were right and none
asked whether the table should exist. `CLAUDE.md` states the rule exactly — a
stand-in may stand "only if the row in docs/port-validation.md explicitly labels
it APPROX with the binary routine that must replace it" — and nothing enforced it.

`tools/check_approx_rows.py` pairs the two sides: an item whose own doc admits it
is APPROX / fabricated / a stand-in / transcribed, against whether the matrix
names that identifier. Matching is by IDENTIFIER deliberately; a row gesturing at
"the phone screen" without naming the literal is what #440 found, and a looser
match would have hidden it.

Result: **21 items admit stand-in status; 8 are named in the matrix, 13 are not.**

The first run said 16, and three were NEGATIONS — `exit_query`'s doc says the
game's behaviour is "the same ... rather than an approximation of it". The word is
not the claim, which is the trap `check_labels.py` already documents for mnemonics
in prose. Polarity is now excluded.

TWO SPOT-CHECKS, and they land differently, which is the useful part:

  * `SHIP_3D_HUD_PYRAMID_VERTICES` is genuinely open — the star-map doc says of it
    "the game's own projection is still being decoded".
  * `VM_FIELD_OFFSET_TABLE` says "Transcribed from BLOODPRG.EXE 0x14180..0x142CF",
    which sounds worse than it is: `native_field_offset_matches_the_lifted_resolver`
    loads that matrix STRAIGHT FROM THE IMAGE and checks the resolver against it,
    so the transcription cannot drift. It is a mirror, and only the matrix row is
    missing.

So UNPAIRED means "unrecorded", not "unprotected", and the entry says so rather
than reporting 13 defects. The 13 are the queue the prime rule asks for; they are
now visible instead of being rediscovered one at a time six entries apart.

2229 items, 1117 confirmed (50.1%), 1112 open. 762 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #442 — three of the ten get real rows; the other seven do not get invented ones

Worked #441's queue. First the tool got a third false-positive class fixed:
`stand-?in` without a trailing boundary matches "STANDING", so `FIELD_OFFSETS`
("the port's standing ...") and `DIALOGUE_FONT_ASCII_MAP_LEN` ("left standing
here") were flagged for admitting nothing at all. 13 UNPAIRED became 10. That is
the third refinement to one tool in one entry-pair — prose keeps finding new ways
to contain a keyword without making its claim.

Then wrote APPROX rows for the three items this session actually characterised,
each naming the replacement as `CLAUDE.md` requires:

  * `VM_FIELD_OFFSET_TABLE` — 0x150 transcribed bytes, but ALREADY PINNED:
    `native_field_offset_matches_the_lifted_resolver` loads the matrix straight
    from the image and checks the resolver against it. The row says so, because
    "transcribed" and "can drift" are not the same condition and the difference
    decides how urgent it is.
  * `GAME_SCREEN_PALETTE_DAC`'s upper bank — the savestate capture from #420,
    replaced by the per-scene HNM palette that `parse_palette_block` (#382)
    already decodes.
  * `SHIP_3D_HUD_PYRAMID_VERTICES` — replaced by the `0x9BBA` projection verified
    in #273, which the live renderer already uses since #426 deleted its
    fabricated neighbour.

THE OTHER SEVEN GET NOTHING, deliberately. I could write a row for each by
paraphrasing its own doc comment, and the tool would report 0 UNPAIRED. That would
record the claim rather than check it — the exact move #440 criticised, since
`PHONE_CONTACTS` survived six entries by having its addresses confirmed while
nobody asked whether the table should exist. A row that says "APPROX; see the
comment" is worth less than the comment.

So: 10 named in the matrix, 8 unpaired, and the number goes down when someone does
the work rather than when someone writes a row.

2229 items, 1117 confirmed (50.1%), 1112 open. 762 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #443 — I committed the error I had just warned against

#442 wrote three APPROX rows and made a point of refusing to write seven more,
because "a row that says APPROX; see the comment records the claim rather than
checking it". One of the three I DID write was that exact mistake.

For `SHIP_3D_HUD_PYRAMID_VERTICES` I named the replacement as "the `0x9BBA`
projection verified in #273". I had not read the item's doc. Reading it:

  * `0x9BBA` is the STAR-MAP projection. This is the HUD pyramid surface, and its
    doc says outright the projection for it "is still unlocated — it runs before
    `0x299:0x1467` fills the 0x6212 records with already-projected coords".
  * The doc goes further: the `0x6212` builder `@0x40D0` writes
    `((flags & 4) | 0x83)`, which is the SPRITE bank dispatch, so the pyramids are
    probably sprites drawn at projected positions rather than a wireframe — "why
    single-routine estimates kept being wrong".
  * And the VERTICES are not the stand-in at all. Their bytes are data-backed;
    they alias palette bank 192..255, the conflict resolved in commit `bd930b8`.
    What is missing is the consumer, not the constant.

So I filled the "binary routine that must replace it" column from a nearby memory
instead of from the item, which is precisely how `0x9BBA` — a projection I had
verified myself in a DIFFERENT context two days ago — came to stand in for an
unlocated one. #432 was the same shape (measuring a blob I had, writing about the
blob I meant), two entries earlier.

Row corrected to say the replacement is NOT YET KNOWN, with the open task the doc
names: find the routine projecting the `0x5491` verts into the `0x6212` records,
plus the compass→matrix-angle map.

The count is unchanged at 10 named / 8 unpaired, because a wrong row and a right
row look identical to the tool. That is worth stating: `check_approx_rows.py`
verifies that a row EXISTS, never that it is true, and nothing can check the
replacement column but reading the item.

2229 items, 1117 confirmed (50.1%), 1112 open. 762 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #444 — narrowing the HUD pyramid projection, and stopping short of claiming it

#443 restored the honest state of `SHIP_3D_HUD_PYRAMID_VERTICES`: the vertex bytes
are data-backed, the PROJECTION that consumes them is unlocated, and the doc names
the task — find what turns the `0x5491` verts into `0x6212` display-list records.
Worked it as far as it goes cleanly.

Both addresses are base-register loaded, so a direct-address census reports ZERO
for each — the #388 shape again, and the reason to scan `B8+r imm16` instead:

  * `0x5491` has exactly TWO immediate loads: `mov di, 0x5491` @`0xB09D` and
    `mov si, 0x5491` @`0xB166`. BOTH are `rep movsd` of `0x10` dwords. Neither
    reads vertices; at those sites `0x5491` is a 64-byte copy buffer. So the
    projection is not at either, which removes the two most obvious candidates.
  * `0x5491` = `live_palette` (`DS:0x5251`) + `0x240` = palette entry 192. That is
    the palette/vertex alias resolved earlier from a byte comparison, now confirmed
    from the ARITHMETIC — the verts and DAC colours 192..255 are the same storage,
    not merely equal.
  * `0x6212` has 19 immediate loads, 17 of them in `0x40D0..0x44A2` (the
    entity-flag accessor family) and two elsewhere: `0x90D9` and `0x9241`, in the
    entity-draw region. Those two are where a projection writing display-list
    records would have to live.

STOPPING THERE. The item's own doc says this is "genuinely multi-session: needs
the projection→position math AND the pyramid sprite source", and I have narrowed
the search rather than found the routine. Writing "the projection is at 0x9241"
because it is the remaining candidate is exactly what #443 caught me doing one
entry ago with `0x9BBA`.

What the next pass inherits: two ruled-out sites, a confirmed alias, and two
addresses to read.

2229 items, 1117 confirmed (50.1%), 1112 open. 764 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #445 — the search was wrong, not merely unfinished

#444 left two candidates for the HUD pyramid projection: `0x90D9` and `0x9241`,
the only two `0x6212` immediate loads outside the entity-flag accessor family.
Read both. Neither is it, and WHY they are not reframes the problem:

    0x90D9  mov si, 0x6212 / les di, ptr [si + 4] / mov ax, word ptr es:[di]
    0x9241  mov si, 0x6212 / les di, ptr [si + 4] / mov ax, word ptr es:[di]

They open identically and both READ. `[0x6212 + 4]` IS A FAR POINTER; the record
data lives behind it, and these two only follow it and scale what they find
(`mul 0xE` then `shr 5` at `0x90E4`; `3 * [0x2789]` at `0x924B` — the same scale
cell the location-info panel uses at `0x90FF`, which #387 corrected).

So a projection filling those records writes THROUGH the far pointer and need
never name `0x6212`. Enumerating immediate loads of `0x6212` could not have found
it at any point. All 19 are now accounted for — 17 flag accessors, 2 readers — so
that avenue is EXHAUSTED rather than unfinished, which is a different and more
useful thing to record than "still looking".

The next approach follows from it: find who writes `[0x6212+4]`, then what fills
the block it points at.

Three entries on this item (#443 corrected a wrong replacement claim, #444 ruled
out the `0x5491` sites, this one rules out the `0x6212` sites) and it is still
open. That is what the doc meant by "genuinely multi-session", and the value each
time has been eliminating a search rather than announcing a routine.

2229 items, 1117 confirmed (50.1%), 1112 open. 766 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #446 — the record points at resource data, so the premise may be wrong

#445 left one step: find who writes `[0x6212+4]`. It is `entity_object_populate`
@`0x40D0`, and what it writes changes the question.

    0x40D7  mov di, 0x6212 / shl ax,5 / add di,ax   record = 0x6212 + i*32
    0x40EF  mov ax, [si] / and ax,4 / or al,0x83    the SPRITE bank dispatch
    0x40F6  mov word ptr gs:[di], ax                 ...into record+0
    0x40F9  add si, 4                                skip the directory header
    0x40FF  mov ebp, dword ptr ds:[bp + si]          a PACKED DWORD entry
    0x4105  and ax, 0xf / add si, ax                 low nibble advances si
    0x410A  shr ebp, 4                               the rest is the payload
    0x4114  mov word ptr gs:[di + 6], ax             record+6 = segment (ds)
    0x4118  mov word ptr gs:[di + 4], si             record+4 = offset

`si` is walking a RESOURCE SUBOBJECT DIRECTORY — that is the label already on
`0x40F9` — so the record's far pointer aims into LOADED RESOURCE DATA.

THAT UNDERCUTS THE TODO'S PREMISE. It was written as "find the routine that
projects the `0x5491` verts into the `0x6212` display-list records", assuming a
projection exists and is merely unlocated. But if the block behind `record+4` is
authored resource data, there may be no such routine: the coordinates would be
SHIPPED, and the pyramids drawn as sprites at them — which is exactly what the
`or al, 0x83` dispatch two instructions earlier says, and what that same doc had
already guessed ("very likely SPRITES drawn at projected positions") without
following the pointer.

So the next step is not a better search for a projection. It is to decode the
packed dword at `0x40FF` and read what `record+4` points at, and only then ask
whether anything projects anything.

Four entries on this item, and this is the first that changed the QUESTION rather
than narrowing the answer. Ruling out searches (#444, #445) was worth doing, but
the reason they kept failing is that the thing being searched for may not exist.

2229 items, 1117 confirmed (50.1%), 1112 open. 775 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #447 — the packed dword is a 20-bit linear offset

#446's next step was to decode the packed dword at `0x40FF`. The four instructions
after it say exactly what it is:

    0x410A  shr ebp, 4        the high 28 bits are a PARAGRAPH count...
    0x410E  mov ax, ds
    0x4110  add ax, bp        ...added to ds to form the SEGMENT
    0x4112  mov ds, ax
    0x4114  mov word ptr gs:[di + 6], ax    record+6 = that segment
    0x4118  mov word ptr gs:[di + 4], si    record+4 = si + (packed & 0xF)
    0x411C  lodsw / mov gs:[di + 0xc], ax   record+0xC = its first word

It is a byte offset into the loaded resource, split the DOS way: `>> 4` is the
paragraph added to `ds`, and the low nibble — added to `si` back at `0x4108` — is
the byte remainder. `add ax, bp` after a `shr 4` is segment arithmetic, and that
is the whole trick; read on its own the `shr` looks like a field extraction.

So the entity records point at SHIPPED RESOURCE DATA reached through a SHIPPED
offset table. Nothing in this path computes a coordinate.

That closes the step #446 asked for and leaves the item in a different state than
it has been in for four entries: not "the projection is unlocated" but "a
projection of this kind probably is not what fills these records". What remains is
to read one subobject's block and see what it actually holds — a data question,
answerable from the shipped files, rather than another search of the image.

Five entries on this thread. Worth noting what made the last two productive after
three that only eliminated candidates: I stopped looking for the routine I
expected and followed the pointer the code actually writes.

2229 items, 1117 confirmed (50.1%), 1112 open. 782 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #448 — the resource is a sprite bank, and the five-entry question was misposed

#447 left a data question: read what `record+4` points at. The decoded directory
shape — `{flags, count}` then `count` packed 20-bit offsets — is exactly the
shipped `.SPR` layout:

    CARTE.SPR     1463 bytes  flags=0x0004  count=7
                  -> 0x01C 0x069 0x10B 0x15F 0x233 0x358 0x40D
                     ascending, every one inside the file
    CROOLIS1.SPR  4873 bytes  flags=0x0004  count=1  -> 0x004

`flags = 4` is why the dispatch computes `(4 & 4) | 0x83` = `0x87`. So
`record+4:+6` is a pointer to a SPRITE FRAME and `record+0xC` is that frame's
first word.

THE QUESTION IS ANSWERED, and it was the wrong question. There is no routine
projecting the `0x5491` verts into the `0x6212` records, because those records do
not hold projected coordinates — they hold sprite-frame pointers into a shipped
bank. `SHIP_3D_HUD_PYRAMID_VERTICES`'s TODO asked for a projection on the strength
of a sentence written before the `0x6212` builder was read; the SAME doc later
guessed "the HUD pyramids are very likely SPRITES drawn at projected positions"
and that guess was right, but the earlier sentence was never withdrawn, so five
audit entries (#443-#447) searched for a routine that does not exist.

What remains is genuinely open and much smaller: where the POSITIONS come from.
That is a different question from the one the TODO posed.

The lesson is about stale premises rather than about this subsystem. #444 and #445
each eliminated a search and reported that as progress; they were eliminating
searches for something that was never there. The check that would have caught it
earlier is the one #446 finally applied — follow the pointer the code writes,
instead of hunting for the routine the doc predicts.

2229 items, 1117 confirmed (50.1%), 1112 open. 782 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #449 — removing the sentence, not annotating it

#448 established there is no projection filling the `0x6212` records. The doc that
sent five entries looking for one still said, in its own words, "the actual
vertex→screen PROJECTION for the pyramids is still unlocated ... TODO: find the
routine that projects the 0x5491 verts into the 0x6212 display-list records (that
IS the missing projection)".

Removed it. The temptation was to leave it with a note attached — the paragraph
records real work (it correctly withdrew an earlier claim that `0x22E0` was the
perspective transform, and correctly identified it as a nearest-point search). But
a doc that states both "find the projection" and "there is no projection" is worse
than either, and the next reader would weigh the two sentences rather than the
evidence. The withdrawal says what it used to say and why it is wrong, which
preserves the history without preserving the instruction.

Swept for the same shape: the codebase now has ONE `TODO:` and two "still
unlocated" mentions, both of which are this entry's own withdrawal text and the
question that genuinely remains (where the POSITIONS come from). No other stale
premise of this kind is sitting in a doc waiting to misdirect a future pass.

That is the cheapest possible sweep — one grep — and it was worth running only
because #448 showed what a stale premise costs: five entries, three of which
reported "eliminated a search" as progress.

2229 items, 1117 confirmed (50.1%), 1112 open. 782 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #450 — the doc enumerated one encoding family too

`present_scene_buffer`'s citations all verify: `add bx, word ptr gs:[0x1fa7]`
@`0xA464` and @`0xAB6E` read the blit base, `mov word ptr [0x1fa7], 0x23` @`0x18BE`
sets the band top, `mov word ptr [0x1fa7], 0xa` @`0x7B5F` is the third case the
port does not model. The doc's acknowledged gap is real and correctly stated.

BUT ITS CASE LIST IS NOT THE CENSUS. Running the full enumeration finds TEN sites,
of which six are immediates:

    0x018BE =35   0x01A37 =0    0x07B5F =10   0x07C45 =0   0x0B3FA =35
    0x09DC7 READ
    0x01F1E  0x05C94  0x0B12E  0x0B526   mov [m], ax  -- COMPUTED

Four sites write a value held in `ax`, so the blit base is NOT limited to
{0, 10, 35}. "The writers give the cases" was true of the writers that pass
enumerated — the immediate forms — and the sentence reads as exhaustive.

That is the fifth appearance of one blind spot: #335 (an immediate scan
under-reporting), #359 (which built `addr_forms.py` to fix it), #403 (a modrm
family missed), #434 (two more encoding gaps in `addr_forms` itself), and now a
DOC making the same mistake in prose. The tools have been patched three times; the
habit of writing "the writers are X" after looking at one form has not.

Used #436's `show_census` for this, which prints the total before the rows — the
helper written two entries ago for exactly this failure. It is the reason the four
register writes were visible at all.

2229 items, 1117 confirmed (50.1%), 1112 open. 782 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #451 — auditing the exhaustiveness claims, including one I had relied on

#450 was the fifth instance of a one-encoding enumeration reading as exhaustive.
Swept for others.

POSITIVE claims: three docs assert a complete writer set. Two are the `0x5B8D`
"sole runtime writer" claim, which #390 had already re-verified across the `80`,
`81` and `83` families after #376/#378 got it wrong once; the third is about
record types, not an address census. Nothing new.

NEGATIVE claims are the riskier kind — a missed encoding turns "nothing does X"
from a finding into an artefact — and #437 CHANGED THE PORT on one: "NO
instruction anywhere in the image compares against `0x5F`", which is what let
`BOB MORLOCK` become `BOB_MORLOCK` without finding the caption renderer.

Re-ran it across every compare-with-`0x5F` form: `cmp al,i8` (`3C`), `cmp ax,i16`
(`3D`), `cmp r/m8,i8` (`80 /7`), `cmp r/m16,i8` sign-extended (`83 /7`),
`cmp r/m16,i16` (`81 /7`). **Zero sites.** The claim holds, and the change it
justified stands.

That is the outcome I wanted and not the one I expected: after five entries of
finding enumerations too narrow, the one that mattered most was wide enough. Worth
checking anyway — the cost was one scan, and #437 is a shipped behaviour change
resting entirely on it.

2229 items, 1117 confirmed (50.1%), 1112 open. 782 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #452 — the divisor is written as two halves, which is why it needed a citation

Opened the `CELL?` queue (103 rows, untouched until now). `pit_divisor_to_hz`
claimed the beep handler at `0x06C0` writes divisor `0x2E9C` for ~100 Hz. It does,
and the way it does is the reason the claim was worth checking:

    0x06C4  mov al, 0xb6 / out 0x43, al   control word: channel 2, mode 3
                                           (square wave), lo-then-hi byte
    0x06C8  mov al, 0x9c / out 0x42, al   divisor LOW  byte
    0x06CC  mov al, 0x2e / out 0x42, al   divisor HIGH byte  -> 0x2E9C

`0x2E9C` appears NOWHERE in the instruction stream — it is assembled from two
byte writes to the same port, in the order the `0xB6` control word demands. A
search for the immediate would find nothing, and a reader checking "does the game
write 0x2E9C" by grepping would conclude the doc was wrong.

Two things fall out. `1193182 / 11932` is 99.998 Hz, so "~100 Hz" is the
hardware's answer rather than a rounded intention. And `0xB6` independently
confirms CHANNEL 2 and SQUARE WAVE — the function's doc asserted both from its
name, and now from the control word.

103 CELL? rows remain. They cite DS cells rather than instructions, which is a
different verification: the question is whether the cell is real and the claim
about it holds, not whether an opcode matches.

2229 items, 1118 confirmed (50.2%), 1111 open. 785 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #453 — dividend, not divisor

`TEXT_SPEED_STEP_DS` described `DS:0x0ACA` as "the divisor in the dialogue
updater's reveal rate". It is the DIVIDEND:

    0x94AB  mov ax, word ptr [0xaca]
    0x94AE  shr ax, 2
    0x94B1  mov word ptr [0xb31], ax     -> reveal_frames_per_char

The cell holds the text-speed step; the reveal rate is that value SHIFTED RIGHT BY
2, and the quotient lands in `0xB31`. With the shipped value of 2 the result is 0.

A one-word error, and the kind that survives because both words describe a
division. The distinction decides which end of the relationship the port models:
"the divisor" implies the reveal rate is `something / [0x0ACA]`, which would make
a LARGER text-speed setting mean a FASTER reveal. The instruction says the
opposite.

The shipped value checks out independently: `DS:0x0ACA` is 2 at file `0x0DEEA`,
which is what `TEXT_SPEED_STEP_INITIAL` claims and what
`text_speed_labels_and_steps_match_the_binary` already reads back out of the image.

The census also gave the cell's full traffic — one write (`0x1B3D`), four reads
(`0x735E`, `0x737C`, `0x94AB`, `0x94D4`) — which is now in the doc, so the next
reader does not have to decide whether the one site quoted was the only one. That
habit is what #450 was about.

2229 items, 1119 confirmed (50.2%), 1110 open. 788 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #454 — the wrong word did not become a wrong implementation

#453 corrected `TEXT_SPEED_STEP_DS`'s description from "divisor" to "dividend".
That raised a question worth answering rather than assuming: did the loose word
produce a loose implementation?

No. `vm::reveal_frames_per_char` is `(step >> 2).max(1)`, which is `shr ax, 2`
@`0x94AE` exactly, and its doc carries the three instructions plus a justification
for the `.max(1)` from the loop's own structure (`mov ax,[0xb31] / or ax,ax / jne`
@`0x94A4` skips the reveal while the countdown is nonzero, so a stored zero still
costs the frame that runs the check). #298 had already corrected that citation
once.

The third reader models correctly too. `record_end_hold_ticks` claims
`b35 = [0x27CF] * ([0x0ACA] >> 1) + 6`, and `0x737C` is:

    0x737C  mov dx, word ptr gs:[0xaca]
    0x7381  shr dx, 1
    0x7383  mul dx
    0x7385  add ax, 6
    0x7388  mov word ptr gs:[0xb35], ax

So the same cell is shifted RIGHT BY 2 for the per-character rate and RIGHT BY 1
for the end-of-record hold — two different derivations from one setting, both
implemented, both cited.

The description that was wrong lived in `bloodprg.rs`, where the CONSTANT is
declared; the implementations live in `vm.rs` with the instructions beside them.
That is the useful shape of the finding: a constant's home is where a vague
description is least likely to be caught, because nothing there computes anything.

2229 items, 1119 confirmed (50.2%), 1110 open. 788 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #455 — "stack" was the wrong word for a pair

`BasMenuStack` says it mirrors "the game's `gs:0x6772`/`gs:0x6774` menu stack".
Those two cells are a CURRENT and ONE SAVED PREVIOUS. The push is three
instructions:

    0x57F7  mov bx, word ptr gs:[0x6772]   read the CURRENT menu
    0x57FC  mov word ptr gs:[0x6774], bx   save it as the PREVIOUS
    0x5805  mov word ptr gs:[0x6772], si   the new current

A census gives `0x6772` five sites (three writes, two reads) and `0x6774` exactly
TWO — both writes, the init at `0x5464` (which sets it from the same `ax` as
`0x6772`, so they start equal) and this save. `0x6774` has NO direct-address read
at all: one level of history is kept, and nothing in that form reads it back.

So the port's `Vec<usize>` models UNBOUNDED nesting where the game keeps one step.
For a menu nested deeper than one level the two diverge — the port can still walk
back, the game cannot.

RECORDED, NOT CHANGED. Whether any shipped script nests concept menus more than
one deep is not established, and narrowing the port to a pair on the strength of
two cells would be the same unfounded move as calling them a stack. What is fixed
is the DESCRIPTION, which asserted a structure the game does not have.

Third entry running where the defect was one word — #453's "divisor" for a
dividend, #455's "stack" for a pair — and in both the word implied a shape rather
than a value, which is why neither showed up as a failing test.

2229 items, 1119 confirmed (50.2%), 1110 open. 791 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #456 — a parameter read by its position, not its name

Following #455's open question — does any shipped script nest concept menus more
than one deep? — I reached for `decode_menus(bas, dic, 3)` as evidence that the
port assumes nesting up to three, and said so before checking.

The signature is `decode_menus(bas: &[u8], dic: &[u8], min_labels: usize)`. The
`3` is a filter on menu SIZE — only menus with at least three labels are decoded —
and says nothing whatever about depth. The function returns a FLAT list of every
`0xA3` menu in the script.

So #455's question stands unanswered rather than answered: nesting depth is a
runtime property of how the conversation is navigated (push on entering a submenu,
pop on `bye_bye`/`talk`), not a static property of the `.BAS` I can count.

Small, and worth an entry only for the mechanism: I read a bare integer argument
and supplied a meaning from what I was looking for, which is the same move as
#443's replacement column filled from a nearby memory and #432's blob measured
because I had it. Three instances in this session of the same habit — reaching for
the nearest plausible source rather than the actual one — and the check is always
the same and always cheap: open the thing.

2229 items, 1119 confirmed (50.2%), 1110 open. 791 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #457 — a row that was already right, including the awkward part

`HEADER_SIZE` (= `PROFILE_SIZE + FLAGS_SIZE + STATE_SIZE`) claims each component
is the `mov cx,imm` of one of `vm_state_save`'s three `int 21h` AH=0x40 writes.
All five citations are exact:

    0x1C60  mov cx, 2        the profile
    0x1C6A  mov cx, 0x200    the flag block...
    0x1C6D  mov dx, 0x6ade   ...and its source
    0x1C72  mov dx, 0x6cde   the state source comes FIRST here
    0x1C75  mov cx, 0x60     then its size

Including the awkward part. The doc warns that "the operand order FLIPS here —
`mov dx, 0x6cde` @`0x1C72` comes FIRST — which is why the immediate is not where
the earlier spacing would put it", and that is exactly what the bytes show. A
reader scanning for `mov cx` at a fixed stride past `0x1C6A` would land on the
wrong instruction, and the doc says so before they do.

`the_header_sizes_are_the_writers_own_immediates` also reads all three back out of
the image, so the constants are pinned rather than transcribed — #329 had already
noticed they carried no address of their own and fixed that.

Settled. Three entries in a row now (#453, #455, this) where the CELL? queue's
docs were substantially right and only the wording or the enumeration needed work;
this one needed neither. That is worth recording alongside the corrections, since
an audit that only reports what it changed gives a false impression of the
codebase it is auditing.

2229 items, 1120 confirmed (50.2%), 1109 open. 791 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #458 — a census of zero that means nothing

`flag_bit` says its 512-byte block "mirrors the game's `[0x6ADE]` region". A
direct-address census of `0x6ADE` returns ZERO sites — which, taken at face value,
reads as "no such region".

It is an IMMEDIATE, not a direct address. `mov reg, 0x6ade` at four sites:

    0x008A4  mov si, 0x6ade   as a source block
    0x01C6D  mov dx, 0x6ade   the SAVE write (int 21h AH=0x40, cx=0x200)
    0x01D0F  mov dx, 0x6ade   its LOAD counterpart
    0x053F6  mov di, 0x6ade   as a destination block

So it is a REGION passed to DOS by address, never a cell read or written in place,
and the save/load pair at `0x1C6D`/`0x1D0F` is exactly where a 512-byte block
would show its size. The doc's claim holds.

THE POINT IS THE ZERO. This is the third time this session a census of 0 has meant
"wrong question" rather than "nothing there": `0x2A1B` in #388 (loaded as an
immediate), `0x6D60` and `0x6724` in #434 (a byte-load and an `les` the tool did
not cover), and now this. A census answers "is this address used as a direct
operand", and three different addressing modes make that a much narrower question
than it looks.

`check_cited_cells.py` (#434) already compensates by falling back to modrm
matching, but a bare `census(...)` call in an ad-hoc probe does not — and ad-hoc
probes are what I actually use. The habit that works is the one applied here:
when a census returns 0 for an address the code plainly uses, scan `B8+r imm16`
before concluding anything.

2229 items, 1121 confirmed (50.3%), 1108 open. 795 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #459 — making the zero explain itself

#458 was the third time a census of 0 was read as "nothing there" when the address
was simply reached another way (#388 `0x2A1B`, #434 `0x6D60`/`0x6724`, #458
`0x6ADE`). Each time the fix was a habit — remember to scan `B8+r imm16` next.
Three repeats is enough evidence that the habit does not hold.

`show_census` now answers for itself. On an empty result it prints

    0 direct-address site(s) for 0x6ade -- THIS IS NOT 'unused'
      but 4 `mov reg16, 0x6ade` IMMEDIATE load(s):
        0x008a4  mov si, 0x6ade
        0x01c6d  mov dx, 0x6ade
        ...

and when there is no immediate either, it names the remaining possibilities
(modrm `mod=10` reg+disp16, `mod=00/rm=110` direct) rather than leaving a bare 0.

Checked against both addresses that caused the problem: `0x6ADE` reports its four
immediates, `0x5491` its two. Neither can now be mistaken for an unused cell.

This is the same move as #373 (the status line became generated after four typed
counts were wrong) and #436 (`show_census` printing totals after four truncated
lists). The pattern in all three: a discipline I kept failing to apply became a
line of code that applies it. What I cannot claim is that this one will be reached
for — ad-hoc probes call `census` directly, and only `show_census` was taught to
explain. That gap is real and stated rather than papered over.

2229 items, 1121 confirmed (50.3%), 1108 open. 795 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #460 — closing the gap #459 admitted, and refusing the number it produces

#459 taught `show_census` to explain a zero but left the gap it named: ad-hoc
probes call `census` directly and get a bare `{}`. `census_all` closes it — one
call merging direct forms, `[reg+disp]` forms, `mov reg16, imm16` loads, and any
modrm carrying the value.

The difference on addresses this session got wrong:

    0x6ADE   census 0   census_all 6      (#458)
    0x2A1B   census 0   census_all 4      (#388)
    0x6D60   census 0   census_all 3      (#434)
    0x6724   census 0   census_all 32     (#434)
    0x2793   census 69  census_all 135

THE LAST ROW IS THE ONE TO BE CAREFUL WITH. `0x2793` is the VM UI-flags word this
session has reasoned about repeatedly (#364, #365, #408), and it is tempting to
report "half its traffic was invisible". That would be wrong. The last two sources
are HEURISTIC: any two bytes matching the address preceded by a plausible modrm
byte are reported, and over data or mid-instruction that produces false positives.
The 135 is a SUPERSET, not a count.

So the docstring says what the function is for — asking "have I missed a way in",
then disassembling each candidate — and what it is not for: quoting. `census`
remains the one whose output can go in a doc.

That distinction is the whole point. Five entries this session found an
enumeration too narrow; the fix for that is not an enumeration that is too wide
and reported with the same confidence.

2229 items, 1121 confirmed (50.3%), 1108 open. 795 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #461 — using the wide tool the way its own docstring says to

#460 added `census_all` and warned its count is a superset, not a finding. Then
used it as intended on `0x2793`: 135 candidates, 69 from the direct census, 66
extra — and checked the extras instead of reporting them.

ALL 66 ARE ONE FALSE POSITIVE. `0x2793`'s little-endian bytes are `93 27`, which
is `xchg bx, ax` followed by `daa` — a common pair — and the byte before them is
usually `06`, `push es`, which satisfies `modrm & 0xC7 == 0x06` by coincidence.
Three sampled candidates all disassemble as `push es / xchg bx, ax`.

So `census`'s 69 IS the real traffic for `0x2793`, and every conclusion this
session drew from it (#364, #365, #408) rests on the right number. That was the
question worth answering, and the answer is reassuring rather than interesting —
which is the usual shape when a caution turns out to be warranted.

The measurement is now in the docstring, so the next reader gets the failure mode
with a worked example rather than a warning: expect the heuristic to be useless
whenever an address's bytes spell common opcodes.

Worth noting what this cost: #460 built the wide tool, #461 immediately showed its
wide half is noise on the first address tried. That is not an argument against
building it — `0x6ADE`, `0x2A1B`, `0x6D60` and `0x6724` all had REAL extra sites
it would have found — but it does mean the tool's value is entirely in the
immediate/reg+disp halves, and the modrm half earns its keep only where the byte
pattern is rare.

2229 items, 1121 confirmed (50.3%), 1108 open. 795 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #462 — the sentinel is the game's

`BloodSave::profile` documents `[0x6780]` with "`0xFFFF` = none". A sentinel is
the kind of claim that is usually a convention someone adopted, so it is worth
knowing which. This one is the game's: of six sites, TWO write the sentinel —
`mov word ptr [0x6780], 0xffff` @`0x10D3` and @`0x1CFA` — and
`cmp word ptr [0x6780], -1` @`0x108E` is the test that reads it back. The
remaining three are two reads and one computed write.

So the port's `0xFFFF` is not a chosen "no value" marker that happens to fit a
`u16`; it is the constant the game stores and compares.

Small, and the reason to record it is the shape rather than the fact. Three
entries this session turned on a description implying the wrong thing — #453's
"divisor" for a dividend, #455's "stack" for a pair, #456's positional read of a
`min_labels` argument — and all three were cheap to check the moment someone
asked "says who?". A sentinel with two writes and a compare behind it can be
quoted; one without is a habit wearing a hex value.

2229 items, 1122 confirmed (50.3%), 1107 open. 797 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #463 — the toggle's two faces are adjacent strings

`ds_pointer_list_strings` walks a `0xFFFF`-terminated list of DS string pointers.
Read `DS:0x2567` out of the shipped image:

    0x2573 0x2581 0x258B 0x2590 0x2595 0xFFFF
    TEXT   MUSIC_OFF SAVE  LOAD  QUIT

Five entries and the terminator, exactly as documented, and every string resolves.

THE INTERESTING PART is the one that is NOT in the list. `DS:0x2578` is
`MUSIC_ON`, eight characters plus a NUL, so it ends at `0x2580` — immediately
before `MUSIC_OFF` at `0x2581`. The two faces of the music toggle are adjacent
strings, and the pointer list names only the OFF one.

That explains a shape the port already had without a reason: `music_on_label`
carries its own hard address instead of indexing this list, and the doc called it
"the alternate face ... which the pointer list does not name". Now it is clear WHY
the list cannot name it — a menu row has one pointer, and a toggle needs two
labels, so the second lives beside the first and is reached by address.

Settled DATA. Both the list and the label are read out of the image rather than
transcribed, which is the shape the prime rule asks for and the reason this row
was cheap: there was nothing to disassemble, only bytes to look at.

2229 items, 1123 confirmed (50.4%), 1106 open. 797 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #464 — the game toggles a label by patching its own pointer list

#463 found `MUSIC_ON` (`DS:0x2578`) sitting immediately before `MUSIC_OFF`
(`DS:0x2581`), with only the OFF one in the menu's pointer list. Followed it, and
the mechanism is complete:

    0x88EB  mov byte ptr [0xba3], 0    music-enabled flag CLEAR
    0x88F0  mov ax, 0x2578             MUSIC_ON
    0x88F3  mov word ptr [0x2569], ax  <- SLOT 1 of the list at DS:0x2567

    0x8902  mov byte ptr [0xba3], 1    flag SET
    0x8907  mov ax, 0x2581             MUSIC_OFF
    0x890A  mov word ptr [0x2569], ax  <- the same slot

`DS:0x2569` is `0x2567 + 2` — the second entry of the very pointer list #463 read.
The game does not have a MUSIC_OFF row; it has ONE row whose label is swapped by
PATCHING THE LIST IN PLACE. That is why only one of the two adjacent strings is
ever named statically, and it answers #463's "why does `music_on_label` need its
own address" completely: nothing indexes to it, the pointer is rewritten.

THE PORT IS ONE-WAY. `1 => music.stop()` stops music and no path starts it, the
`[0xBA3]` flag is not held, and `bloodprg::music_on_label` has NO CALLER — the
dead-accessor shape of #385 and #404. So the row is a switch where the game has a
toggle, and a player who turns music off cannot turn it back on.

Recorded at the fix site rather than fixed: the port's OPTION box and its console
box are two different paths (the OPTION box renders only `CANCEL`), so wiring the
flag correctly is a restructure, not a line. What the next pass needs is now
written where it will be read — the flag cell, the two label addresses, the slot
that gets patched.

ALSO SPOTTED: `engine.rs` compares against an inline
`["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"]`, which is the same content
literal #385 deleted as `CONSOLE_MENU`. It survived because it is a COMPARISON
rather than a declaration, and `check_dead_pub_consts.py` only looks at `pub
const`. Noted, not removed — it drives `is_baked_menu`, so deleting it needs the
replacement decided first.

2229 items, 1123 confirmed (50.4%), 1106 open. 800 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #465 — a content literal hiding in a comparison, and it was dead

#464 spotted `engine.rs` comparing `console_box` against the inline literal
`["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"]` — the same five names #385
deleted as `CONSOLE_MENU` — and said `check_dead_pub_consts.py` misses it because
that tool only looks at declarations.

TRUE BUT INCOMPLETE, and the correction matters: `check_ui_literals.py` HAS been
flagging it all along, as "IN-DATA ... PIN IT". The literal was not unguarded, it
was reported and unaddressed. Two checkers, one blind to it by design and one
naming it every run, and I described only the blind one.

Then the literal turned out to be DEAD. Nothing in the library ever assigns those
five names to `console_box` — grepping `"HONK"` finds this comparison and four
sites in `runtime_boot.rs`, a diagnostic probe binary. So `is_baked_menu` was
ALWAYS FALSE, the `if !is_baked_menu` guard always taken, and the comparison
existed only to be false. #385 deleting `CONSOLE_MENU` without breaking the build
was the evidence, a hundred entries earlier: if anything had assigned that list,
the const would have had a caller.

Removed both the guard and the binding; the body now runs unconditionally, which
is what it already did. 614 lib tests pass, the workspace is clean, and the
library no longer contains those five names.

Worth noting how it survived #385. That entry deleted a `pub const` after checking
for references TO IT, which is the right check for a dead declaration and blind to
a second copy written out longhand somewhere else. The same content, in the same
file, in a different syntactic position.

2229 items, 1123 confirmed (50.4%), 1106 open. 797 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #466 — a checker whose advice was mostly already taken

#465 noted `check_ui_literals.py` had been flagging a literal "every run" without
anyone acting. Looked at what else it flags, and the answer reframes the tool: 84
literals marked "PIN IT", and the advice was ALREADY TAKEN for nearly all of them.
`WORLD_ART_DIRECTORY`'s 42 world names are held to the image byte-for-byte by
`world_art_directory_matches_the_ds2bc7_table`; `CANCEL` by
`option_box_label_is_the_games_own_string`; `LOADING`/`PAUSE` by
`ui_string_literals_match_the_image_block`. All three tests exist and pass.

The tool appended "PIN IT" unconditionally, so a reader saw 84 demands of which
~80 were satisfied. That is worse than silence: it trains you to skip the output,
which is exactly what had happened.

Taught it to detect a pin. The first attempt asked whether the literal STRING
appears in the file's tests, and caught 5 of 41 — because a pinning test is
precisely the kind that does NOT repeat the value it pins. `world_art_...` reads
the image and compares programmatically; "Kortex" never appears in it. The second
attempt asks whether the tests name the literal's ENCLOSING ITEM, which is how the
relationship is actually expressed.

    before   41 in the image (all "PIN IT"),  43 in shipped data
    after    41 in the image (1 unpinned),    43 in shipped data (42 unpinned)

The one remaining image literal is `EMMXXXX0`, the EMS driver signature — a DOS
constant, not game text. The 42 data ones are almost all in `src/bin/*`,
diagnostic probes rather than the port's runtime.

So the library's display strings are, in fact, pinned. That is worth knowing
precisely because the tool had been saying the opposite for its whole existence.

2229 items, 1123 confirmed (50.4%), 1106 open. 797 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #467 — three bridge rows, settled on work already done

Back to the ledger after five tooling entries. Three `CELL?` rows verify against
decodes this session already established, which is the cheap half of an audit and
worth taking:

  * `croolis::from_vtable_offset` cites `0x103A` — #406 read both dispatch sites
    (`mov bx, fs:[di+0x34]` / `call word ptr fs:[bx+0x103a]`) and confirmed the
    table is runtime-filled, which is why a static dump of it is all zeros.
  * `bridge::click` cites `0x86A4` (`test byte [0xa3e], 1`, the mouse-present flag
    #384 and #388 both turned on), `0x86F1` (`test byte [0x2793], 8`, the
    SEEK_ACTIVE gate from #386) and `0x7DC8` (`call 0x8269`, the orb hit test
    #388 verified field by field).
  * `bridge::apply_menu_palette` cites `0x862B..0x86A3`, which I had not read.

The palette one is exact where it counts:

    0x862B  mov dx, 0x3c8    the DAC INDEX port
    0x862E  mov al, 0x7b     first menu-row entry
    0x8630  out dx, al
    0x8631  inc dl           -> 0x3c9, the DATA port
    0x8633  mov cx, 5        FIVE rows
    0x8636  mov al, 0x10     R = 16
    0x8639  mov al, 0x0c     G = 12

so "the five menu-row DAC entries (0x7B..0x7F)" and the dark-gold idle colour
`(16, 12, 0)` are both read off the instructions rather than off a screenshot —
which for a PALETTE claim is the distinction that matters, since a capture would
give the same numbers and no provenance.

2229 items, 1126 confirmed (50.5%), 1103 open. 797 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #468 — five croolis rows, and a duplicated write in the game

Five `CELL?` rows settle together, all verified in the overlay:

  * `update_position` / `0x999` — opens `mov si, [di+0x16]` / `mov cx, [di+0x1a]`,
    the child array and count from #403, then `mov di, 0x4000` and
    `mov bp, 0x7fff`. So `ALIEN_POSITION_WRAP` (16384) and `POSITION_WRAP_MASK`
    (0x7FFF) are immediates, not chosen bounds.
  * `x` / `0x22EC` — `mov ax, [0x22ec]` / `mov bx, [0x22f0]` / `mov dx, [0x22f4]`
    at `0x09A8`, the three camera HIGH WORDS exactly as `AlienCamera` documents.
  * `reset` / `0x36A` — `mov dword ptr [si + 0x12], 0x8000` @`0x0385`, `[si+0x22]`
    @`0x038D`, `[si+0x32]` @`0x0395`.
  * `AlienObject` / `0x105C` and `step` / `0xB72` — both read in #400 and #403.

A DUPLICATED WRITE in the initializer, recorded because a rewrite would tidy it
away: `mov dword ptr [si + 0x3a], 0` appears TWICE, at `0x0375` and `0x037D`,
byte-for-byte identical (`66 c7 44 3a 00 00 00 00`). The natural reading is that
the second was meant to be `+0x3e`, so that field is never initialised. Neither
offset is modelled in the port, so nothing is wrong today — but if `+0x3a`/`+0x3e`
are ever added, the game zeroes the first twice and the second never, and a
faithful port has to do the same.

That is the second such artefact in this overlay, after #425's `adc eax, 0` on
only the Y axis. Both are the kind of asymmetry that looks like a mistake and must
be reproduced anyway: the question a port answers is what the game DOES, not what
it meant.

2229 items, 1131 confirmed (50.7%), 1098 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #469 — eleven rows, mostly on decodes already banked

Eleven `CELL?` rows settle in one pass, which is what the earlier entries bought:
most cite addresses this session already read, and the rest were four
disassemblies away.

Already established: `StationRecord`/`0x2A1B` (#388's `mov di,0x2a1b` +
`mul 0x18` station table), `mouse_screen_x`/`0x97FC` (#433, where the cell turned
out to be `0x0A2A` not `0x2A2A`), `menu_row_under_cursor`/`0x8614` (#386's
off-by-one correction), `set_frame_orb_box`/`0x9860` (#388's reset loop),
`bas_vm::new`+`current`/`0x6772`+`0x6774` (#455's pair-not-a-stack),
`croolis::new`/`0x105C` (#400's shared stream).

Newly read, and they close the menu palette cleanly:

    0x8633  mov cx, 5        five rows      (#467)
    0x8636  mov al, 0x10     idle R = 16
    0x863F  loop 0x8636      ...the loop those five run
    0x869D  mov al, 0x3f     hover R = 63 -- the DAC maximum
    0x868D  cmp al, 5        the five-row bound again, in the hit test

So `MENU_ROW_IDLE_DAC` `(16, 12, 0)` and `MENU_ROW_HOVER_DAC` `(63, 0, 0)` are
both immediates, and the "5" appears three times independently — as the loop
count, the hit-test bound, and the row count the port uses.

The provisional queue is now 176, from 246 at the session's start. What made this
entry cheap is that eight of the eleven were paid for earlier: verifying an
address once makes every later row citing it nearly free, which is an argument for
reading the neighbourhood rather than the single instruction a row names.

2229 items, 1142 confirmed (51.2%), 1087 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #470 — twelve more, and an address that is two addresses

Twelve `CELL?` rows in `engine.rs`. Six ride on earlier work — `OPTION_BOX_LABEL`
`0x0D594` (#385's `C,A,N,C,E,L,0`), `console_menu_click` `0x8614` (#386),
`load_scene_hnm` `0x1FA7` (#450), `load_bas_menus` `0x6772` (#455),
`render_alien_view` `cs:0x16A2` (#401) and `0x7FFF` (#468), `load_dialogue`
`0x108E` (#462's profile compare).

The rest verify at labelled routines: `0x954A` is `mov byte ptr [0x5b55], 1`, the
screen-dirty flag #382 met in `resource_palette_blocks_apply`; `0x0FFB` is
`mov word ptr [0xb2d], 8` in the main loop; `0x2AD3` opens `resource_load...`;
`0x604E` and `0x721A` open `active_object_list_build` and `nav_chart_list_build`,
which is what `NavChartObject` claims.

ONE CITATION IS TWO ADDRESSES. `current_voice` cites `0xCFB`, and `0xCFB` is:

    a CODE address   0x0CFB  int 0x33            (inside the mouse routine)
    a DS CELL        gs:[0xcfb]  written 1 @0x66AF, 0 @0x94CF

Both are real and they have nothing to do with each other. Disassembling `0xCFB`
to "check the citation" lands on an `int 33h` that has no connection to voice
playback, and would read as the citation being wrong. The row means the CELL, and
the two writes settle it.

This project already separates five address spaces in `re/CLAUDE.md` (file, `DS:`,
`XDB:`, `DRV:`, `SCRIPT<N>:`) precisely because a bare number is ambiguous. This is
the same hazard INSIDE one binary: a DS offset and a code offset can collide, and
`0xCFB` is small enough to be plausible as either.

Provisional queue: 164, from 246 at the session's start.

2229 items, 1154 confirmed (51.8%), 1075 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #471 — twelve more, settled partly from the data

Twelve rows. Four ride on earlier entries (`0x604E`/`0x721A` #470, `0x0FFB` #470,
`0x1FA7` #450, `0xB31`/`0xB35` #453/#454). Four verify at labelled routine
entries: `0x79E5` `screen_mode_update`, `0x9240` `entity_draw_full`, `0x94BA`
`dlg_...`, `0x4BAA`. And `0x7362` is `mov word ptr gs:[0xb35], ax` — the
record-end hold #454 traced from the other side.

TWO ARE DATA, not code, and settle from the bytes:

`DS:0x2B97` is the box-open zoom table, and the shape is unmistakable once dumped:

    (155, 67, 10, 15)   x,y shrinking...
    (143, 57, 34, 35)
    (120, 51, 80, 47)   ...w,h growing

which is exactly "a 6-phase zoom, {x,y,w,h} growing from a point to the full
320x130 frame". Disassembling `0x2B97` gives `xchg dx, ax` — meaningless, and the
third instance this session of a citation that reads as wrong when checked in the
wrong address space (#470's `0xCFB`, #452's two-halves divisor, this).

`BoldConsoleFont` cites file `0x1451A = DS:0x70FA` and `0x145CA = DS:0x71AA`, and
the arithmetic confirms both: `0x1451A - 0xD420 = 0x70FA`, `0x145CA - 0xD420 =
0x71AA`. A file offset given WITH its DS equivalent is checkable without
disassembling anything, which is why those two took seconds and `0x2B97` took a
dump to figure out.

NOT settled: `response_text` and `menu_submenu_labels` cite only `0xFFFF`, a
terminator value rather than an address. There is nothing there to verify, and
`audit_settle` would take them on a technicality.

Provisional queue: 152, from 246.

2229 items, 1166 confirmed (52.3%), 1063 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #472 — the cursor law, verified end to end

Twelve more rows. The one worth reading is `menu_camera_pan`, whose doc described
"centre-delta steering" without an instruction behind the centre. All of it is at
`XDB:manu3:0x0034..0x0058`:

    0x0034  push word ptr [0x23e2]   save pitch
    0x0038  push word ptr [0x23e4]   save yaw
    0x003C  mov ax, word ptr [0x1a]  cursor X
    0x0043  sub ax, 0xa0             X - 160
    0x0046  add ax, ax               ...doubled
    0x0048  add word ptr [0x23e4], ax
    0x004C  sub bx, 0x64             Y - 100
    0x004F  add bx, bx               ...doubled
    0x0051  add word ptr [0x23e2], bx
    0x0055  call 0x270               compose
    0x0058  pop word ptr [0x23e4]    restore

So `MENU_CAMERA_CENTRE` (160, 100) is `0xA0`/`0x64` as immediates; the doubling is
`add reg,reg`, not a shift; and — the part the port's two-line function cannot
express — the angles are ADDED to the stored values, composed, then POPPED BACK.
The hand aims by DISPLACEMENT and does not accumulate. `manu3_hand`'s module
header already said this ("receives the cursor law non-destructively"); now the
push/pop pair is on the function that computes the delta.

Getting there needed the address-space discipline of the last two entries:
`0x2306`, `0xFFC` and `0x23E2` all disassemble as nonsense (`enter -0x769a, 0x4c`
for one) because they are DATA offsets. The way in was to search the entry region
for references TO `0x23E2`, which found `0x0034` immediately.

Provisional queue: 140, from 246 at the session's start.

2229 items, 1178 confirmed (52.8%), 1051 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #473 — the +9 that has been open since the session began

Eleven more rows. Most ride on earlier entries (`0x2F65`..`0x2F6B` #409, `0x981B`
#388, `0x2274`/`0x2974` #425/#429, `0x6212` #445, `0x0A2A` #433). The newly-read
ones verify cleanly: `0x65F2` is `test byte ptr [0x67ad], 1` (the VM query flag),
`0x0B02` is `lcall gs:[0xa4a]` (the PRNG call #392 found cited by both `next` and
`rand`), `0x2D50` is `les di, ptr gs:[0x5229]` (the panorama unpack), `0x1A5E` is
`mov word ptr [0x2793], 1`, and `0x97E7` is `mov ax, word ptr [0x2795]` — the
frame index, one instruction past #387's corrected `0x97E3`.

`DLG_LINE_ID_BIAS` cites `0x11F5`, and the instruction is **`add ax, 9`**.

That is the `+9` from this session's FIRST open question. The summary I resumed
from recorded `text_selector_voice_clip_index` as unsettled because "the game's
mapping is `line_id = b3 + 9`, whereas this computes `b3 - 1`", and I spent the
opening moves tracing `dlg_line_id_scene_dispatch` looking for it. The bias was a
settled row's citation the whole time, one `add` instruction, in a different file.

It does not close that question — knowing where `+9` lives does not connect a line
id to a talk-HNM, which is what `text_selector_voice_clip_index` needs. But it
does mean the two halves of the discrepancy are now both located, and the row's
"the game's mapping is b3 + 9" is no longer an unsourced assertion.

One settle refused for a good reason: the row is
`run_ship_3d_navigation_final_reset`, not `run_ship_3d_navigation_final` — the
tool declined a name that does not exist rather than matching a prefix.

Provisional queue: 129, from 246 at the session's start.

2229 items, 1189 confirmed (53.3%), 1040 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #474 — the helper paid for itself

Seven `vm.rs` rows. The routine entries verify as code — `0x6034`
`vm_record_lookup_by_...`, `0x6946` the 7-opcode shared handler from #392, `0x6EEE`
`vm_op_c3_state_record`, `0x69C7` `vm_op_cd_state_gated` — and `0x903E` is
`mov byte ptr [0xada], 8`, so `LOCATION_PANEL_ZOOM_STEPS = 8` is an immediate.

Four citations disassembled as nonsense (`ror byte ptr [bp+si+0x75d0]` for one),
which by now is a recognised signal rather than a puzzle: they are DS cells.
`0x0ADA` has 9 direct sites, `0x2AAB` 5, `0x677E` and `0x674E` and `0x6886` one
each. Two showed zero — and `show_census` answered on its own:

    0 direct-address site(s) for 0x2120 -- THIS IS NOT 'unused'
      but 2 `mov reg16, 0x2120` IMMEDIATE load(s):
        0x067c8  mov bp, 0x2120
        0x067d5  mov bp, 0x2120

`0x67C8` is `vm_op_a8_load_string`, whose dispatch-table entry reads "copy
NUL-terminated operand into buffer 0x2120 (bp)". The tool found the buffer's two
loads and one of them IS the handler that the row is about.

THAT IS #459 PAYING OFF. It was written three entries after the fourth time I read
a zero census as "nothing there", with the explicit caveat that I could not
promise to reach for it. Here it ran without being asked, on two addresses, and
turned what used to be a wrong conclusion into a two-line answer. `0x6724`'s zero
is the remaining known gap — `les`/`lds`, documented in `addr_forms` since #434.

Provisional queue: 123, from 246.

2229 items, 1195 confirmed (53.6%), 1034 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #475 — the game has no clock

Opened the `DATA?` queue (47 rows, all with EMPTY origins — they are data-parsing
functions, verified against shipped files rather than instructions).
`descript::parse` and `decode_text` settle immediately: four tests parse the real
`DESCRIPT.DES` and assert 145 records with non-empty names, and every one of those
names passes through `decode_text`.

Then `load_tv_programs` picks "the ad channel's seasonal variant, by today's (UTC)
civil date" — `SystemTime::now()`, month/day, `(12,25) => christmas`,
`(1,1) => year`.

THE GAME CANNOT DO THIS. Scanning BLOODPRG.EXE:

    mov ah,0x2A (DOS get DATE)   0 sites
    mov ah,0x2C (get TIME)       0 sites
    mov ah,0x2B (set date)       0 sites

It never asks the system for a clock, by any of the three routes, so it cannot
branch on the calendar at all.

THE CONTENT IS REAL THOUGH, which is what makes this worth care rather than
deletion: `christmas` and `year` are genuine DESCRIPT.DES records, at `0x26` and
`0x38` in the name table. So this is not an invented SURFACE like #385's console
menu or #426's star-map renderer — it is real shipped material reached by an
invented RULE.

Left in place, labelled, and given an APPROX row in `port-validation.md` naming
what is undecoded: whatever the game uses to reach those records. That is the
distinction #427 drew for `NAV_DEST_X` — delete a fabricated surface, keep and
mark a real one behind a port-side affordance.

Provisional queue: 121.

2229 items, 1197 confirmed (53.7%), 1032 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #476 — the port paces on a measurement; the game has a programmed cadence

#475 established the game never reads a clock, which made every wall-clock use in
the port worth checking. Most are extraction nonces. One is the frame loop:
`Duration::from_millis(46)`, sourced in its own comment as "MEASURED game rate:
21.6 fps at the hub (FRAMERATE probe: VGA page flips per PIT-timed second)".

An oracle measurement standing in for a decoded value — and the decoded value is
right there:

    0x00FFB  mov word ptr [0xb2d], 8      the frame budget, in PIT ticks
    0x012C9  cmp word ptr [0xb2d], 0      the main loop SPINS...
    0x012CE  jne 0x12c9                   ...until it reaches zero
    0x012D1  call 0x17af / 0x178b         then flips the page

#411 verified the PIT divisor `0x1746` = 5958, so the tick is 1193182/5958 =
200.26 Hz and EIGHT of them is 39.95 ms = **25.0 fps**. That is the programmed
cadence; 21.6 fps is what the game ACHIEVES when frames overrun it. The port's 46
ms bakes the average overrun into every frame, which is why it reads as authentic
in a capture and is not what the game asks for.

Given an APPROX row naming the replacement: pace on the 8-tick budget with a
~40 ms floor, letting slow frames take longer, exactly as the spin-wait does.

AND ONE GENUINE PUZZLE. Nothing in the image decrements `[0xB2D]`. Its
little-endian bytes occur exactly TWICE in `BLOODPRG.EXE` — the write and the
compare — so the loop's exit condition comes from the INT 08h handler `func_79c`
installs at `cs:0x213`, reaching the cell by a route that is neither a direct
address nor an immediate. A spin-wait whose variable nothing visibly changes is a
good reminder that "no sites found" bounds the SEARCH, not the program.

Provisional queue: 121.

2229 items, 1197 confirmed (53.7%), 1032 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #477 — pacing on the decode, and a 15% timing error it exposed

#476 found the frame loop pacing on a FRAMERATE probe (46 ms) while the game has a
programmed cadence. Implemented the replacement:

    const PIT_HZ: f32 = 1_193_182.0 / 5958.0;   // 0x1746 divisor, #411
    const FRAME_TICKS: f32 = 8.0;               // [0xB2D] budget, 0x0FFB
    const GAME_TICK_SECS: f32 = FRAME_TICKS / PIT_HZ;   // 39.95 ms

Both numbers are decoded — the divisor verified in #411, the budget read at
`0x0FFB` in #476 — so the tick is no longer a measurement at all.

THE INTERESTING PART IS WHAT IT EXPOSED. The state-countdown beat was
`countdown_accum += 8.011 * 0.046`: the true beat rate (200.27/25 = 8.011 Hz,
`[0xB27]` reload verified in #411) multiplied by the MEASURED frame time. Rederived
from the decode it is `(PIT_HZ / 25) * (8 / PIT_HZ)` = **8/25 = 0.32 exactly** —
the PIT frequency CANCELS, because frames and beats are both counted in the same
PIT ticks: 8 per frame, 25 per beat.

So the correct value is a ratio of two integers the game states, and the old one
was 0.36851 — running scripted countdowns **15% fast**. Those countdowns release
GUARD `state[i]==0` blocks (the comment names SCRIPT2's Scruter interception), so
timed script events were firing early.

That is the argument for the prime rule in one line. Mixing a decoded rate with a
measured one produced a float that looked plausible and was wrong by 15%; using
only decoded quantities produced an exact rational and fixed it. The error was
invisible to every test because nothing measured against the game — and a capture
would have AGREED with the wrong value, since 46 ms is what the game achieves.

2229 items, 1197 confirmed (53.7%), 1032 open. 798 citations verified, 0 wrong.
723 workspace tests, 0 failures.

## #478 — pinning the ratio, and finding it was the only one

#477 replaced a decoded-times-measured constant with an exact rational. Two
follow-ups.

PINNED. `the_countdown_beat_is_exactly_eight_twentyfifths_of_a_frame` derives the
relationship from the three decoded integers — PIT divisor `5958` (`0x1746`,
#411), frame budget `8` (`[0xB2D]`, #476), beat reload `25` (`[0xB27]`, #411) —
and asserts that beats-per-frame is 8/25 with the PIT frequency CANCELLED. It also
asserts the superseded `8.011 * 0.046` came to ~0.3685, so the size of the old
error stays visible rather than being quietly forgotten.

That test cannot be satisfied by measurement: it is a statement about the
relationship between two decoded counts, and it holds for any PIT frequency. A
capture-derived value can only pass it by accident.

SWEPT for the same shape — a decoded rate multiplied by a measured duration —
across `main.rs`, `engine.rs` and `vm.rs`. The only hits are this entry's own
comments and the new test. So #477's was a single instance, not a pattern, which
is worth knowing after finding it: the reasonable fear on discovering one mixed
constant is that the codebase is full of them.

2229 items, 1197 confirmed (53.7%), 1032 open. 798 citations verified, 0 wrong.
724 workspace tests, 0 failures.

## #479 — four rows, and the DATA/INFRA line

The `DATA?` queue is asset loaders with empty origins, so settling them means
asking what they read rather than checking an address. Four resolve, and they
split across the line #421 drew:

DATA — genuinely reading shipped content:

  * `load_intro` takes the credit subtitles from the DESCRIPT `present` record's
    cues, which its own doc states is "sourced from the game data rather than
    hard-coded" — the divergence fixed earlier in this project's history.
  * `intro_clip_music` returns that same record's music stem.
  * `start_descript_cutscene` is explicitly "a general, data-driven cutscene
    player; each cutscene's music/subtitles come from the [record], not
    hardcoded".

INFRA — no binary counterpart at all:

  * `collect_hnm_paths` recursively walks a directory collecting `*.hnm` files
    into a name map. The game does not scan directories; it loads by RESOURCE ID
    through the `FS:0x0C04` name table (#463, and the loader `0x3FC7`). This is
    the port finding files on a modern filesystem, and no amount of decoding will
    give it an address.

Classifying it DATA would have been the easy move — it does touch game files — and
would have been wrong in the way that matters: DATA asserts the port reads what
the game reads, and this reads a directory the game never looks at.

Provisional queue: 117, from 246 at the session's start.

2229 items, 1201 confirmed (53.9%), 1028 open. 798 citations verified, 0 wrong.
724 workspace tests, 0 failures.

## #480 — real names, invented enumeration

Five more asset loaders, and the split needed a distinction #479 did not yet have
to make: a function can use REAL game asset names and still discover them a way
the game never would.

Checked the names against the image: `cryorad` IS in `BLOODPRG.EXE`, `hyper_` IS
in `BLOODPRG.EXE`, `tvgren` is not (it is a filename on disk only). So the
cryobox and cyberspace stems are the game's own.

DATA — `TvProgram` (a DESCRIPT Sequence record that self-identifies as a channel),
`render_tv` (plays that programming with its tick-timed cues), `load_cryobox`
(loads the named `cryorad.hnm` and takes its palette from the HNM's own header).

INFRA — `load_tv_channels` and `load_cyberspace`. Both GLOB: "globs `sq/` for
`tv*`", "globs `sq/hyper_*.hnm`, sorted so segments advance in order". The game
loads by resource ID through the `FS:0x0C04` table; it has no directory listing
and no sort. `load_tv_channels`'s own doc calls itself a "legacy fallback for when
the DESCRIPT programming is unavailable", which is the port covering a case the
game does not have.

The `hyper_` case is the interesting one, because the prefix is a REAL string in
the image and it would be easy to read that as provenance for the whole function.
It is provenance for the NAME and not for the ENUMERATION — the game knows what
`hyper_` files exist because its resource table lists them, and the port knows
because it asked the filesystem. Same set, different authority.

Provisional queue: 112, from 246.

2229 items, 1206 confirmed (54.1%), 1023 open. 798 citations verified, 0 wrong.
724 workspace tests, 0 failures.

## #481 — the game cannot enumerate a directory

#480 classified two globbing loaders INFRA on the grounds that "the game loads by
resource ID; it has no directory listing". That was an inference from the resource
table's existence. Checked it directly, and it is stronger than inferred:

    mov ah,0x4E (DOS FindFirst)   1 site
    mov ah,0x4F (DOS FindNext)    0 sites
    wildcard filename strings     none — no "*.hnm", no "*.*", nothing

FindNext is the call that ITERATES a directory. With zero of them, and no wildcard
string anywhere to pass to FindFirst, the game has no enumeration available by any
route. The single FindFirst is a STAT: `mov ebp, dword ptr es:[bx + 0x1a]` @`0x3FF4`
lifts the file SIZE out of the DTA, and `mov ax, 0x3d00` @`0x3FF9` opens that same
known file.

So the game learns what exists from the `FS:0x0C04` table and asks DOS only how
big a named file is.

THE PORT SCANS DIRECTORIES IN SEVEN RUNTIME PLACES (`engine.rs` ×4, `script.rs`,
`snd.rs`, `hnm.rs`), plus tests and the DOS runtime — where `read_dir` is
legitimate, since `recomp/runtime.rs` is EMULATING the file access the game makes.
Two are already classified INFRA (#480). The rest reach the same files the game
would, by a route it has no instruction for.

Recorded centrally, next to the resource table, rather than at each site: the
finding is about the game, and a reader asking "may the port scan for assets?"
should meet the answer where the alternative lives.

I have not changed the scanners. Replacing seven of them with table lookups is a
real piece of work and each needs its own check that the table actually covers the
files it wants — `collect_hnm_paths`, for instance, resolves DESCRIPT talk-HNM
names, and whether every such name has a resource ID is exactly the sort of thing
#438 found untrue for two phone contacts.

2229 items, 1206 confirmed (54.1%), 1023 open. 800 citations verified, 0 wrong.
724 workspace tests, 0 failures.

## #482 — the game does NOT search for its media: a path-TEMPLATE table names the directory

#481 left a concrete question: can `collect_hnm_paths`'s directory scan be replaced
by a `FS:0x0C04` resource-table lookup? Measured, the answer is NO, and for a blunt
reason — the resource table's 95 names contain **zero** `.hnm` entries, while 701
HNM files ship under `ob`, `pe`, `pl`, `sq`. That table cannot resolve a talk-HNM.

But the game plainly does not scan, so the directory had to be somewhere. It is.
The image holds RELATIVE PATHS as literals, and most of them are TEMPLATES:

    0x0E11C  'sn\tb.snd'          0x0E14D  'mu\xxxxxxxx.voc'
    0x0F48B  'sq\mind.HNM'        0x0F557  'pe\xxxxxxxxxxxx'

The twelve `x`s are a PLACEHOLDER patched with the filename at load time. So the
DIRECTORY IS A PROPERTY OF THE SLOT, fixed in the binary; only the name varies.

`re/tools/dump_asset_table.py` decodes the table at **0x0F48B..0x0F915 — 45
records**. Prefix census: `pe\` 33, `sq\` 10, `pl\` 1, `ob\` 1. Those are EXACTLY
the four directories the port had discovered by scanning, which is the confirmation
that this table is the thing the scan was standing in for.

TWO STRUCTURE ERRORS ON THE WAY, both mine, both the same species:

- I read the first slot as a fixed 26-byte record (`pe\` + 12 `x`s + NUL + 10). It
  is VARIABLE length — NUL-terminated path THEN 10 metadata bytes. `sq\the_star.HNM`
  is 26, `sq\cryogel.hnm` is 25. A uniform 26 desynchronised at the first short name
  and corrupted every record after it, printing `q\cryorad.hnm` and `\pollup.hnm`
  — visibly truncated prefixes I could have caught by reading the output.
- Walking backward with a `contains a backslash` test settled one byte INSIDE a
  record, because the truncated tail `\xxxxxxxxxxxx` passes that test. The table
  start moved from a real 0x0F48B to a phantom 0x0F53F. Fixed by requiring a full
  two-character prefix (`^[a-z0-9]{2}\\`).

The metadata is nine zero bytes then `0x10` — uniform across all 45 records EXCEPT
the last (`sq\pollup.hnm`), whose tenth byte is `0x00`. A terminator, and an
independent check that the record parse is aligned: the flag lands in the same
column 45 times running.

WHO PATCHES A SLOT IS NOT ANSWERED, and four searches failed to find out:
DS-displacement census of the slot addresses (0 sites), the raw immediate `0x206B`
(0 occurrences ANYWHERE in the file), a pointer array over consecutive slots (none,
at ANY paragraph-aligned base — scanned all 4096), and a far pointer resolving to a
slot (one hit at 0x918B, which disassembles as the operand of `int 0x27` running
into `add bx,6` — mid-instruction coincidence, the standard x86 phantom).

The negative result is itself informative: variable-length records CANNOT be
indexed, only walked, so a pointer array SHOULDN'T exist and its absence is
consistent rather than surprising. The table is reached by a pointer computed at
runtime — an overlay's code, most likely, which is where the search goes next.

So the scanners stay for now, and this is why: the mechanism is decoded but the
slot-to-callsite mapping is not, and wiring a template table to the wrong caller
would be worse than a scan that reaches the right file. Recorded as the task.

## #483 — three dead trackers left behind by two REFUTED audio models

`cargo` reported "value assigned to `voice_line` is never read" three times. Traced,
`voice_line` had four writes and NO reads. So did `voice` — and `voice` was never
assigned `Some` ANYWHERE, so the stream it names was always empty. `chatter_done_line`
was the same: declared, cleared twice, never set and never read.

They are the residue of two models this project already refuted IN COMMENTS SITTING
DIRECTLY ABOVE THEM:

- A per-line voice clip, refuted at `main.rs` @0x66AF/0xB898/0xB8AB/0x94CF: "There is
  no per-line or per-speaker clip selection anywhere in the executable." The old code
  selected `b3 - 1` bounded by `talk_hnms.len()` — an AUDIO index bounded by a count
  of talk VIDEOS — and was removed. `voice`/`voice_line` outlived it.
- A single end-of-line blip, refuted by the @0xB898 decode of the CONTINUOUS burble:
  "This is the continuous honk-burble under the text, not a single end-of-line blip."
  `chatter_done_line` was that blip's edge detector.

What kept them compiling is worth naming, because it is a pattern to watch for:
`let _ = &voice;` and `let _ = &chatter_done_line;` — statements that read as
keep-alives but only silence the unused warning. The first even carried the comment
"keep the stream alive while the line plays", describing a stream that is always
`None`. A pin like that converts a compiler diagnostic into a false reassurance.

`let _ = &chatter;` is NOT the same and stays: `chatter` IS assigned a live
`MusicPlayer` @0xB898, and dropping it would cut the burble. Same syntax, real
purpose — which is exactly why the two needed telling apart rather than a blanket sweep.

Also removed: the declaration comment claiming the game "plays sn/tb.snd clip 0 once
per fully-revealed subtitle line (@0x94BA)". That is the refuted model stated as fact
at the top of the file while the loop 2000 lines below implements the correct one.
A stale comment outranks a dead variable as a hazard — the variable does nothing,
but the comment actively misinforms the next reader about a decoded behaviour.

Deleted: 3 declarations, 6 assignments, 2 pins, 1 wrong comment. The remaining
warnings in this sweep are cosmetic and stay: `mx`/`my` @`bin/blood.rs:99` are the
VERIFYSCRIPT harness's scripted mouse (INFRA, not game state).

616 tests, 0 failures.

## #485 — the ledger's MZ bound silently un-cited every OVERLAY function

Settling `run_ship_3d_temp_snd_setup` (#484) needed a ledger refresh first, and the
refresh exposed something bigger: `audit_settle.py` REFUSES `ASM` on a row whose
`origin` is empty, and nine manu3 rows had empty origins while their doc comments
carried addresses all along. `MenuTween::step` cites `0x19B` in its first sentence
and the ledger called it UNVERIFIED.

The cause is a filter I added in #423 and was right to add: drop addresses below
`0x600`, because prose numbers (`0x100`, `0x181`) were being harvested as citations
and 65 rows read as evidenced on that basis. What it missed is that **the MZ bound
is a fact about ONE address space**. manu3.xdb's method entries are `0x000`,
`0x181`, `0x19B`, `0x1DF`; its matrix build is `0x270..0x3DE`. Every real overlay
citation sits exactly where the junk does.

So the rule "below 0x600 is not an address" was true of the image and false of the
overlays, and applying it globally deleted the evidence of the modules that are
ENTIRELY overlay ports — manu3.rs and manu3_hand.rs.

The fix uses the qualified form `re/CLAUDE.md` already defines rather than a new
heuristic: `XDB:manu3:0x19B`. `audit_inventory.py` matches it separately, exempts
it from the MZ bound, and KEEPS THE PREFIX in `origin`, so the two spaces can never
compare equal. 26 citations across the two files were rewritten to it (several were
already in that form, which is what confirmed the convention rather than inventing one).

Both downstream consumers of `origin` had to learn the same distinction, and this is
the part that would have bitten later: `check_duplicate_rules.py` keys its
"one rule implemented twice" cluster BY ADDRESS, and `check_liftable_twins.py`
matches origins against lifted IMAGE offsets. An overlay `0x1234` and an image
`0x1234` are unrelated numbers. The first now keys by `(space, address)`; the second
skips XDB citations outright, since a lift is always an image offset. Both still
report cleanly, "No same-name duplicates".

The nine rows are now citable. Six of them are settled ASM below on real evidence,
not on the citation merely existing — and verifying one of them found #486.

## #486 — the menu tween is ONE FRAME LONG and one step behind

Verifying `MenuTween` against `XDB:manu3:0x19B` (the whole method disassembled, not
sampled) matched the port everywhere: `[di+4]` is the target, `[di+8]` is the high
word of the dword accumulator at `[di+6]` — which is exactly `accumulator >> 16` —
`dec [di] / js` is the expiry, `add [di+6],eax` the advance, and `sub bx,2 /
xchg [bx],di` the swap-remove the port already mirrored.

The CONSTRUCTOR did not match. At `XDB:manu3:0x1FE`:

```text
0x0207  shl eax, 0x10      ; (end - current) << 16
0x020B  shl ebp, 0x10      ; current << 16
0x020F  cdq / idiv ecx     ; delta = ((end-current)<<16) / count
0x0214  dec cx             ; <-- counter = count - 1
0x0215  mov [di+0xa], eax  ; delta
0x0219  add ebp, eax       ; <-- accumulator = (current<<16) + delta
0x021C  mov [di], cx
0x021E  mov [di+6], ebp
```

The port stored `count` and a bare `current << 16`. The binary PRE-ADVANCES by one
frame in both fields, and the two go together: because the step loop writes the
accumulator BEFORE advancing it, the first value written is `current + delta`, and
the tween lands on `end` after exactly `count` writes. The port instead wrote the
unmoved `current` first and took `count + 1` frames — every menu animation one frame
long and one step behind, on every item, for as long as this has been in.

The existing test asserted `output() == 10, "starts at current"` — it encoded the
PORT's behaviour, which is how this survived. Note what it got right: its
`reaches end after count frames` assertion passes under BOTH readings, because the
extra step just returns remove-me without advancing. Only the first-value assertion
could ever have caught this, and it was written to the wrong answer. Now 15, with
the expiry edge (seven advancing steps, then `false`) pinned separately.

`count == 0` cannot reach the constructor in the binary — `or cl,cl / je` @0x1E7
skips the descriptor — so the port's saturating floor is its own guard and is
labelled as such rather than presented as decoded.

616 tests, 0 failures.

### #486a — the same routine was ported TWICE, and the copies disagreed

Verifying `PosePlayer::step` (`manu3_hand.rs`) against the same `XDB:manu3:0x1DF` /
`0x19B` pair found it already correct:

```rust
self.active.push((count - 1, target, (cur << 16) + step, step));
```

`count - 1` and the pre-advanced accumulator, both of them, written independently
of `MenuTween::to_target` — which had neither. Two models of ONE overlay method
lived in the port for as long as both files have existed, and they disagreed by a
frame. #486's fix is now confirmed from a second direction rather than only by my
reading of the disassembly: the corrected constructor agrees with the copy that was
already right.

`check_duplicate_rules.py` DOES cluster these two — that is what it is for, and the
cluster was sitting in its output. What it cannot do is compare the bodies, so
"two functions cite 0x19B" was reported and read as legitimate (a routine and its
helper often do share an address, which the tool says in its own footer). The
lesson is narrow and worth keeping: a duplicate-citation cluster is a prompt to
DIFF THE TWO IMPLEMENTATIONS, not merely to check that both are cited.

Its phase half matches too: `cl` is the count and `ch` the phase (`movzx ecx,[si]`
@0x1E3), `count == 0` ends the sequence (`or cl,cl / je 0x23E`), and a phase
mismatch breaks after `inc word [0x102c]` @0x239 — advancing exactly one phase per
frame, which is what the port's `break` does.

## #487 — teaching the duplicate check to find what #486a found, and failing twice first

#486a's lesson was that a shared citation is a prompt to DIFF THE BODIES, and the
tool only ever printed the cluster. So: flag the clusters where no member CALLS
another, since a delegating pair (`field_offset` -> `vm_field_offset`, under a
comment reading "ONE resolver, not two") is benign and an independent pair is not.

The first version did not catch the case it was built from, and the reason is
almost funny. It asked `other in body` — a substring test. After #486 the comment
inside `MenuTween::to_target` reads "one frame long and one step behind", the other
member of its cluster is named `step`, and so my own explanatory prose registered as
a call. The check silently cleared the exact pair it existed to find. Fixed by
stripping comments and requiring a call form (`name(` or `::name`).

That still did not catch it. The cluster for `XDB:manu3:0x1DF` has THREE members —
`tween`, `to_target`, `step` — and `tween` genuinely does call `to_target`. Asking
"does any member call any other" then declared the whole cluster linked, hiding the
third member, which is the independent one. Linkage is not a property of a cluster;
it is a property of a PAIR. Now every pair is tested and the unlinked ones are named:

```text
  XDB:manu3:0x1df
      step        src/manu3_hand.rs   ASM
      to_target   src/manu3.rs        ASM
      tween       src/manu3.rs        ASM
        no call edge: step <-> to_target
        no call edge: step <-> tween
```

Correctly silent on `to_target <-> tween`. Both failures were the same mistake in
different clothes — a cheap approximation of "are these related", accepted without
checking it against the one case whose answer I already knew. A new detector should
be run against its own motivating example before it is believed, and that is the
step I skipped twice in a row.

HONEST LIMIT, because the number looks worse than it is: 45 pairs are flagged and
they are NOT 45 defects. Co-citation is common and legitimate — I checked
`actor_record_is_active` / `record_owner_is_active` @0x6073 (different resolution
paths, different unknown-handling, both documented), `current_line_hold` /
`reveal_frames_per_char` @0xB31 (caller and callee, the call is behind a `use`), and
`field_offset` / `vm_field_offset` @0x6023 (already deliberately merged). All three
are fine. The output is a WORKLIST, not a defect count, and it is recorded as such
so a later reader does not mistake 45 for a regression.

616 tests, 0 failures.

## #488 — three uncited magic numbers in the alien visibility gate, all sourced

`VISIBLE_SCREEN_Y_MAX = 128`, `VISIBLE_WORLD_X_HALF = 256` and
`VISIBLE_ANIM_Y_BIAS = 60` sat in `croolis.rs` with prose doc comments and NO
address — the shape `CLAUDE.md` calls a defect, and the ledger had filed the first
as `INFRA?`, which would have been a false claim that it has no binary counterpart.
It has one. All three are immediates in the `0xA30` gate:

```text
  0x0A47  mov ax, fs:[bp+0x36]   bp = timer & 0xFFC -- a timer-indexed entry
  0x0A4C  sar ax, 8              ...its HIGH BYTE
  0x0A5C  sub ax, 0x3c           VISIBLE_ANIM_Y_BIAS = 60
  0x0A5F  add ax, [si+0x46]      the object's y
  0x0A62  add ax, [0x22f0]       the camera's y
  0x0A66  js  0xAA0              negative y rejected -- the floor is a SIGN TEST
  0x0A68  cmp ax, 0x80 / jg      VISIBLE_SCREEN_Y_MAX = 128
  0x0A74  cmp ax, 0xff00 / jl    the world-x floor
  0x0A79  cmp ax, 0x100  / jg    VISIBLE_WORLD_X_HALF = 256
```

Two things the values alone would not have told you, and both are now in the docs:

- **The y window is asymmetric and the x window is not.** Y's lower bound is `js`,
  a sign test, so it is `[0, 128]` — there is no `-128`. Writing the pair as two
  symmetric half-extents would have been a natural guess and wrong.
- **`0xFF00` is -256, not 65280.** The jumps are `jl`/`jg`, the SIGNED forms, so
  the x window really is `[-256, +256]`, matching the port's `-VISIBLE_WORLD_X_HALF`.
  With `jb`/`ja` the same bytes would mean something entirely different.

The Z axis @`0xA85`/`0xA8A` reuses the same two immediates, which the new test
asserts by comparing the byte ranges directly rather than restating the numbers —
`visibility_bounds_are_croolis_xdb_immediates` pins all three to croolis.xdb, so a
constant edited away from the image fails rather than passing quietly.

Settled ASM: the three constants, plus `gpu.rs`'s `new`/`present` as INFRA (X11 and
wgpu presentation, genuinely no binary counterpart).

617 tests, 0 failures.

## #489 — the list widget's six layout seeds, and a name that reads as arithmetic

304 ledger rows are UNVERIFIED CONSTANTS — 126 in `ship3d.rs` alone — and they sit
interleaved with cited ones, so the file looks evidenced at a glance. Six of them
belong to one routine, `list_widget_layout_unified` @`0x8428`, and they came out
together:

```text
  0x8436  xor bp, bp      /  0x8438  mov dx, 0x64    seeds 0 and 100
  0x843B  test byte [0xadd], 1 / je 0x8448
  0x8442  mov bp, 0xa     /  0x8445  mov dx, 0x37    seeds 10 and 55
  0x847A  add bp, 0xb                                11 per row
  0x84A1  add dx, 0x14    /  0x84A7  add bp, 8       padding before centring
```

THE NAMES INVITE A WRONG READING. `DEFAULT_MAX_WIDTH = 100` beside
`EXTRA_WIDTH = 55` reads as "100, plus 55 when there is an extra entry" — 155. It
is nothing of the sort: `test byte [0xadd],1` selects ONE PAIR OR THE OTHER, and
the flag that adds a row picks the SMALLER width seed, not a larger one. The port's
code was already right (`if has_extra_entry { EXTRA_WIDTH } else { DEFAULT_MAX_WIDTH }`),
so this is a documentation defect rather than a behavioural one — but an uncited
constant whose name implies the opposite of its use is exactly how a later edit
"fixes" the addition that was never there.

Why the smaller seed goes with the extra row is now recorded too, because it makes
the design legible: `dx` is a running MAXIMUM (`cmp ax,dx / jb / mov dx,ax`
@`0x8472`), so these are FLOORS that the widest measured label can only raise. A
list with the extra row starts from a lower floor and lets its labels set the width.

Settled ASM: all six. The remaining 298 uncited constants stay on the queue, and
this is the shape the work takes — find the routine, read the seeds out of it in
one pass, and cite them together rather than one number at a time.

617 tests, 0 failures.

## #490 — 41 render-driver offsets, sourced mechanically instead of one at a time

#489 ended by saying the work should go routine by routine rather than number by
number. `bloodprg.rs`'s 41 `RENDER_*_OFFSET` constants allow something better still,
because they are not values at all — they are ENTRY POINTS in the render driver, and
the game reaches them with `lcall 0x299, <offset>`. So every one of them has a
mechanical citation: a real call site, or an explanation for why there isn't one.

`re/tools/render_driver_calls.py` scans the image for `9A off16 seg16` far calls into
the segment and cross-checks both directions. 32 constants matched, 143 call sites
in all — and, importantly, the reverse check found NO called-but-unnamed offset, so
the port's list of driver entries is complete rather than merely correct so far.

NINE HAD NO CALL SITE, and that was the interesting part rather than a gap. They are
the sprite blitter entries, and they are reached BY INDEX. Segment `0x299` maps to
file `0x2F90`, so the table at driver offset `0x1592` is file `0x4522`, and its first
eight words are:

```text
  0x15a6 0x172c 0x1c18 0x1d46 0x1fd2 0x210a 0x210b 0x210c
```

— exactly the eight `RENDER_SPRITE_BLIT_*` constants, in the port's own order, with
nothing left over. A dispatch table is why "no far call exists" was the right
measurement of the wrong question yet again (#485's lesson, third instance).

The three `NOOP` entries sit one byte apart, which looks like a mis-decode until you
read the bytes: `c3 c3 c3`, three consecutive single-byte `ret`s. Three distinct
no-op slots, each its own instruction. And the next named routine,
`RENDER_DIRTY_RECTS_COPY_OFFSET = 0x210d`, begins exactly where they end — an
independent check that the table's tail is read right, which the new test asserts as
`NOOP_7 + 1`.

Settled ASM: all 41. Ledger 1252/2232 (56.1%), UNVERIFIED down to 870.

618 tests, 0 failures.

## #491 — 287 is not a measurement, it is two adds (and I nearly filed it as fabricated)

The `SHIP_3D_NAV_CHOICE_*` block — 14 constants, an undocumented enclosing function
(`hit_test_ship_3d_nav_choice`), and screen-shaped values like 287, 110, 72 — is
exactly the profile of geometry lifted off a capture, which `CLAUDE.md` forbids
outright. I searched the image for 287 (`0x011F`) in every immediate form a 16-bit
instruction can carry: `mov`/`cmp`/`sub`/`add`/`test` against ax/bx/cx/dx/si/di/bp.
**Zero occurrences.** The conclusion sitting right there was "fabricated, delete it".

It is in the binary. The routine COMPUTES it:

```text
  0x8642  sub ax, 0x2d       axis - 45          AXIS_BIAS
  0x864B  shl ax, 3          * 8
  0x864E  neg ax
  0x8650  add ax, 0xe8       + 232 ...
  0x8653  add ax, 0x37       ... + 55  =  287   RIGHT_BASE
  0x8656  cmp bx, ax / jg    the right bound
  0x865C  sub ax, 0x6e       - 110              X_WIDTH
  0x8663  cmp bx, ax / jl    the left bound
```

232 + 55, in two instructions, so no scan for the constant could ever find it. This
is the THIRD time this session that a zero-result meant the wrong question, after
the overlay address space (#485) and the blitter dispatch table (#490) — and #484
had already recorded the identical shape, where the viewport descriptor's `0`, `1`
and `4` are built by `xor`/`inc`/`add ax,3`. I had written that lesson down and
still spent the search before remembering it.

Worth being precise about what saved it: not the disassembly, which I only ran
afterwards, but `docs/audit-fixes.md` containing `0x86D1 ... SHIP_3D_NAV_CHOICE_TARGET_Y`
from an earlier session. The routine had been found before and the constants were
never annotated with it. A decode that lives only in the fix log and not in the
code it explains is one refactor away from being deleted as unsourced.

Thirteen settled ASM from the two routines around it:

```text
  0x862E  mov al, 0x7b        PALETTE_FIRST, out to DAC index port 0x3C8
  0x8633  mov cx, 5           COUNT -- also `cmp al,5 / jge` @0x868D
  0x8679  mov cl, 0x12        ROW_HEIGHT_BASE, reduced by `shr al,1 / sub cl,al`
  0x86AB  mov [0xa32], 5      PRESENTATION_MODE
  0x86B6  or  [0x2793], 0xc   HUD_SELECT_FLAGS
  0x86BB  mov [0x279b], 0x5a  HOLD_TICKS (90)
  0x86C1  mov [0x2565], 1     HANDLER_PHASE -- the widget phase cell read @0x8874
  0x86CA  mov cl,0x12/mul cl  TARGET_Y_STEP, applied to bl-1 (`dec al` @0x86C8)
  0x86CE  add ax, 0x50        TARGET_Y_BASE
  0x86F1  test [0x2793], 8    DISPATCH_BLOCK_FLAG -- bit 3 BLOCKS the call @0x8700
```

The last one is the kind of thing a name alone hides: `0x0C` is RAISED by this same
routine @`0x86B6` and `0x08` is tested as a BLOCK a few instructions later, so the
two flag constants overlap in bits but not in meaning.

STILL OPEN in this block: `MIN_GATE` (40), `MAX_GATE` (60) and `Y_BASE` (72) — not
found in the decoded stretch, and after 287 I am not willing to call them absent on
a failed scan. They stay UNVERIFIED until the routine's head is read.

618 tests, 0 failures.

## #492 — the target-row draw loop, and what its constant NAMES were hiding

Nine more `ship3d.rs` constants, all from the widget's draw pass `0x84E1..0x85B6`,
and three of them mean something different from what their names suggest.

**The three text colours are a LADDER, not three states.** `mov al,0xe8` @`0x8565`
is what every row starts as. `dec byte gs:[0x27c7]` @`0x8584` counts down to the
hovered row, and only when it hits zero does `mov al,0xef` @`0x858B` run; only then
is `[0xa3e]` bit 0 tested @`0x858D` to reach `mov al,0xfe` @`0x8595`. So ACTIVE is
not "the active row" — it is "the hovered row, while the button is down", and a row
can never be active without first being hovered. Three flat `pub const`s give no
hint of that nesting.

**The exit sentinel has a sibling.** `0xFFFF` ends the list (`cmp ax,-1 / je`
@`0x8456`, `cmp si,-1 / je` @`0x856E`), but so does a ZERO entry (`or ax,ax / je`
@`0x8452`) — two terminators, only one of which the port names. Both compares are
SIGNED against -1, which is the detail that makes `0xFFFF` the right spelling.

**The alias is a substitution, not an offset.** `ALIAS_LABEL_OFFSET` is loaded only
when the entry equals `[0x2734]` (`cmp si,[0x2734] / jne` @`0x8573`), swapping one
particular label for another — and the layout pass does the same swap @`0x8467`, so
the box is measured with the substituted text rather than the original.

Two cross-confirmations fell out. `lcall 0x299,0x176` @`0x8597` is the draw call,
and `0x176` is `RENDER_UI_TEXT_OFFSET` — one of the six call sites #490's census
attributed to it, reached here independently. And `mov si,0x174` @`0x85B3` sits
under `test byte [0xadd],1`, the SAME extra-entry flag whose two layout seeds #489
sourced; the flag that narrows the box is the flag that adds this label.

`ship3d.rs` is down from 115 uncited constants to 93.

618 tests, 0 failures.

## #493 — the nav-choice gate is the PANORAMA FRAME, and the list is in perspective

#491 left three constants open rather than settle them on a failed scan. All three
are in the routine's head, found by scanning for the RANGE TEST rather than for
either value alone — `cmp` with 40 and `cmp` with 60 within 24 bytes of each other
occurs exactly ONCE in the image:

```text
  0x8614  mov ax, [0x2795]      the BRIDGE PANORAMA FRAME
  0x8617  cmp ax, 0x3c / jg     MAX_GATE
  0x861E  cmp ax, 0x28 / jl     MIN_GATE
```

`[0x2795]` is not an abstract "gate value" — it is the panorama frame that
`bridge_view_sector_update` (@`0x9512`, labels.csv) steps. So the nav choice exists
ONLY while the bridge view faces frames 40..60. A constant named `MIN_GATE` says
nothing about that; the cell it is compared against says all of it, which is the
argument for citing the COMPARISON rather than the number.

`Y_BASE = 72` is `mov bx,0x48` @`0x8674`, and it is only a base:

```text
  0x8674  mov bx, 0x48      y origin = 72 ...
  0x8677  add bx, ax        ... + |axis - 45| ...
  0x8679  mov cl, 0x12      row pitch = 18 ...
  0x867B  shr ax, 2         quarter = |axis-45| >> 2
  0x867E  add bx, ax        ... + quarter
  0x8680  shr al, 1         half the quarter, on the LOW BYTE
  0x8682  sub cl, al        ... - that
```

The list SLIDES DOWN and its rows COMPRESS as the player looks away from centre —
a perspective effect, not a fixed layout. I checked the port against it rather than
assuming, after #486: `hit_test_ship_3d_nav_choice` implements all of it, including
that `shr al,1` acts on AL so the truncation to 8 bits happens BEFORE the shift
(`(quarter_axis as u8) >> 1`, not `(quarter_axis >> 1) as u8`). Those differ once
the quarter exceeds 255. No defect here — but the check is the point, since three
constants sourced is worth less than knowing the arithmetic around them agrees.

`ship3d.rs`: 93 uncited constants -> 90.

618 tests, 0 failures.

## #494 — the nav-choice DISPATCH TABLE, found by solving for a segment

`call word ptr cs:[bx+0xf29]` @`0x8700` is where the five nav choices become five
routines, and it could not be read directly: `cs:0x0F29` needs `cs`, the routine is
entered by a NEAR call, and no far pointer anywhere in the image names its segment
(the far-pointer scan over `0x85F0..0x8720` returns nothing).

So solve for it. `docs/audit-fixes.md` records handler 4 at file `0x886C`. For a
paragraph-aligned base `B`, the table would sit at `B + 0x0F29` and must CONTAIN
`0x886C - B`. Scanning all 4096 candidates gives two, and only one is coherent:

```text
  cs = 0x071E  (cs:0 = file 0x077E0),  table @ file 0x8709
  entries: 0x0f33 0x0f4c 0x0fdd 0x1068 0x108c
        -> 0x8713 0x872C 0x87BD 0x8848 0x886C
```

Five ascending, contiguous routines beginning immediately after the table, the
table itself immediately after the dispatcher's `ret` @`0x8708`, handler 4 landing
exactly on the known address, and the count matching the `cmp al,5 / jge` @`0x868D`
that #491 had already sourced as `NAV_CHOICE_COUNT`. The rejected candidate
(`cs=0x04FF`) scatters its entries across `0x609A..0xCBE1` with no such structure.

THE TEST CAUGHT MY OWN OVER-CONSTRAINT. I asserted each handler opens with
`test byte [0x2565],1` at byte 0, allowing one leading `push es`. Handler `0x872C`
does `push es / mov es,[0x6726] / mov si,0x2b13` first, so the assertion failed on a
CORRECT decode. Relaxed to "tests the phase cell within its opening 24 bytes",
which is the claim actually being made. Worth recording because the failure looked
at first like the segment solution was wrong, and the tempting move was to doubt
the decode rather than the assertion.

Five constants settled alongside it, two of which link subsystems already decoded:
`mov byte [0xada],0xa` @`0x86E4` sets the very cell `ship_3d_interpolation_gate`
divides by @`0x1E63` (#490's four-word gate), so the nav box animates over 10 ticks;
and `mov word [0xac6],0x64` @`0x86D9` writes the centring cell the layout pass reads
@`0x84AF` (#489). The nav choice configures the widget and then opens it.

`ship3d.rs`: 90 uncited constants -> 85.

619 tests, 0 failures.

## #495 — the navigation box, and a correction to #494's "no far pointer names it"

`mov byte [0xada],6` occurs three times in the image; the one at `0xB3CD` sits ten
bytes from `mov si,0x253b` @`0xB3D7`, which is the navigation routine. Six constants
came out of the stretch `0xB3C3..0xB41D`, and the clip pair is the interesting one:

```text
  0xB407  mov word [0x5239], 0x23   band top 35
  0xB40D  mov word [0x523b], 0xa5   clip bottom 165 ...
  0xB415  lcall 0x299, 0xe2f        ... for THIS call only
  0xB41D  mov word [0x523b], 0xc8   restored to 200
```

`RENDER_CLIP_BOTTOM` and `RENDER_CLIP_RESTORED_BOTTOM` are not two settings — they
are the same cell before and after one draw, which is why both must exist and why
neither is "the" clip bottom.

A CORRECTION TO #494, and it matters more than the constants. I wrote that the
dispatch table's segment had to be solved because "the routine is reached by a NEAR
call and no far pointer anywhere in the image names its segment". The second half is
false. `lcall 0x71e, 0xc48` @`0xB3DA` resolves to file `0x8428` —
`list_widget_layout_unified` — and `0x71E` is exactly the segment I solved for.

The solved answer was right; the claim about why it had to be solved was not. What
I actually did was scan for far pointers TARGETING `0x85F0..0x8720`, the dispatching
routine's own range, and conclude from that emptiness that the segment was unnamed.
But a segment is named by EVERY far call into it, at any offset — and this one is
called from a routine 11KB away. The narrow scan answered a narrower question than
the one I reported.

So the constraint-solve was unnecessary work that happened to produce the right
answer, and the honest version is that both routes agree. `re/labels.csv` is
corrected; the entry now records the direct confirmation and the scoping error,
because "I searched and found nothing" is only as strong as what was searched.

`ship3d.rs`: 85 uncited constants -> 79.

619 tests, 0 failures.

## #496 — the panorama auto-turn, and a function documented WITHOUT settling it

Five `SHIP_3D_PROCEDURAL_*` constants, located by scanning for 180/360/1440 in
word-immediate forms and taking the only window holding more than one — `0x9748`.
The routine is the panorama auto-turn, `0x9733..0x97FC`:

```text
  0x9733  test word [0x2793], 8       HUD_ACTIVE_FLAG
  0x9748  cmp ax, 0xb4 / jl           HALF_TURN ...
  0x974D  sub ax, 0x168 / neg ax      ... folding to the SHORTEST distance
  0x975A  test word [0x2793], 4       TARGET_LIST_FLAG picks the step size
  0x9794  shl bp, 2                   angle * 4 ...
  0x979D  add cx, 0x5a0               ... + 1440 = MOUSE_RING
  0x97A8  int 0x33  (ax = 4)          SET CURSOR POSITION
```

`MOUSE_RING` is the one whose name only makes sense with the call beside it: the
game DRIVES THE HARDWARE CURSOR round a ring of four units per degree, offset by a
whole 360*4 so the coordinate never goes negative. A constant named "1440" beside
`int 0x33` is a mechanism; alone it is a number.

`HALF_TURN` and `FULL_TURN` are likewise not two constants but one fold —
`cmp 180 / sub 360 / neg` is the standard shortest-angular-distance idiom, and
reading either number without the other makes the routine look like it clamps.

THE FUNCTION IS DOCUMENTED BUT NOT SETTLED, deliberately.
`run_ship_3d_procedural_update` had no citation at all and now names its routine.
Its constants are verified instruction by instruction; its BODY is not. The binary
works in degrees and wraps at `0x168`, while the port models frames and doubles
(`angle * 2`) — consistent with the 180-frame panorama and the `[0x2795]` cell
(#493), but not checked against every wrap site, and the two step sizes the branch
selects (`0x28` @`0x977C`, `0x1E` @`0x97C4`) are still unnamed. Settling it ASM now
would claim a whole-function transcription I have not done — which is exactly the
move #491's near-miss and #494's over-constrained test both warn against. The doc
says what is checked and what is not; the row stays provisional.

`ship3d.rs`: 79 uncited constants -> 74.

619 tests, 0 failures.

## #497 — the auto-turn's tail closes both of #496's gaps, and 1440 has two jobs

Seven more constants from the tail of `0x9733..0x980B`, and the tail answers the two
questions #496 deliberately left open rather than guess at.

**A frame IS half a degree-count.** The routine accumulates in DEGREES, wrapping at
`0x168`, and then:

```text
  0x97E1  shr bx, 1
  0x97E3  mov word ptr [0x2795], bx     the panorama FRAME cell
```

So `run_ship_3d_procedural_update` modelling frames and doubling (`angle * 2`) is
not a plausible-looking mapping, it is the exact inverse of an instruction. #496
recorded that as unestablished; it is established now, and the doc comment says so
instead of still hedging.

**The two step sizes have names.** `TARGET_LIST_STEP` is the `0x28` @`0x977C` and
`AUTO_ROTATE_STEP` the `0x1E` @`0x97C4` — the target-list branch turns faster than
the plain one, which is the whole reason the `[0x2793]` bit-2 test @`0x975A`
selects between two otherwise identical wrap paths.

**1440 is one value doing two jobs**, which is why the port has two constants for
it and neither is redundant: `add cx,0x5a0` @`0x979D` is the ring ORIGIN added to
`angle * 4`, while `add bx,0x5a0` @`0x9807` and `cmp bx,0x5a0` @`0x980B` are the
ring MODULUS wrapping a delta. The new test asserts they are equal AND cites both
sites, so the coincidence is documented rather than looking like duplication.

THE TEST CAUGHT ME ON THE `83 /N ib` FORM AGAIN. I asserted the align mask by
reading a WORD at `0x97F9`; `and word ptr [0xa2a],0xfff8` encodes as
`83 26 2a 0a f8`, where the immediate is a single SIGN-EXTENDED byte at `0x97FA`.
The word read returns `0xF80A` — the mask's byte plus the following opcode. This is
the same encoding family my own notes list as the recurring blind spot, and it has
now cost a test failure in three separate sessions. The assertion is rewritten to
check the `83 /4` opcode pair explicitly and then sign-extend the byte, so the next
reader sees the form rather than a bare offset.

`ship3d.rs`: 74 uncited constants -> 67.

620 tests, 0 failures.

## #498 — the music row TOGGLES; the port only ever stopped it

#464 decoded the mechanism and stated the fix in a comment, then left it: "This port
stops music and never starts it, and `bloodprg::music_on_label` has no caller." That
comment has been sitting above `1 => music.stop()` ever since. Wiring it now.

The game has ONE music row whose LABEL is patched in place — the pointer list at
`DS:0x2567` is self-modifying:

```text
  0x88DF  test byte [0xba3], 1 / je 0x88F8   which way are we going?
  ; latch SET -> switch OFF:
  0x88EB  mov byte [0xba3], 0
  0x88F0  mov ax, 0x2578 / mov [0x2569], ax   slot 1 := MUSIC_ON
  ; latch CLEAR -> switch ON:
  0x8902  mov byte [0xba3], 1
  0x8907  mov ax, 0x2581 / mov [0x2569], ax   slot 1 := MUSIC_OFF
  0x8914  mov si, 0xd3d                       mu\tablo2.voc, this branch ONLY
```

The port now holds the `[0xBA3]` latch, swaps slot 1 between the two faces, and
starts `tablo2.voc` on the ON branch instead of only stopping.

THE INITIAL STATE IS NOT A GUESS. `[0xBA3]` starts SET, because the SHIPPED list
carries `MUSIC_OFF` in slot 1 (#463) — the row that offers to turn music off is the
row you see when it is already on. The data settles it, so the port does not have to
pick a default.

I ALSO CHECKED HOW FAR THE LATCH REACHES before gating anything on it, rather than
assuming a menu flag only drives a menu. It is tested at four sites: the toggle
itself and three in the audio subsystem, where `test byte [0xba3],1 / je 0xbc49`
@`0xBBC4` skips the entire music path. So it is a GLOBAL music enable, and gating
the port's nav-view `tablo2` loop on it is faithful rather than an extrapolation —
without that, switching music off in the OPTION menu would have been undone by the
next return to the nav view.

Three constants settled with it: the two label faces and the `mu\tablo2.voc` path
offset, all named in `ship3d.rs` and previously uncited.

`ship3d.rs`: 67 uncited constants -> 64.

620 tests, 0 failures.

## #499 — using refs_in_routine properly means FINDING THE ENTRY first

The matrix/projection constants are DS cells, and `re/tools/refs_in_routine.py`
exists precisely for them: it decodes forward from a VERIFIED entry and reports
every displacement, so each hit is at a real instruction boundary. Its value
depends entirely on the entry being right, which took two tries.

`0x981B` is a genuine entry — the `retf` @`0x981A` that ends the auto-turn (#497)
puts a prologue immediately after it — but it is the WRONG routine: `int 0x21` with
`ah=0x3f` and `ax=0x4200`, a file read. Its cells are file-handle scratch, and had I
been matching cells to names by proximity I would have attributed them to the
matrix. The matrix routine is the NEXT one, `0x98B9`, after the `ret` @`0x98B8`.

From that entry the tool gives the four cleanly:

```text
  0x98CB  mov bp, 0x4f45     ANGLE_TABLE   (indexed by the three angle cells)
  0x98CE  mov si, 0x2f7d     TEMP scratch  (read back as [si]..[si+0x14])
  0x992D  mov di, 0x2f95     PROJECTION_MATRIX, nine stosd'd dwords
  0x9941  sar ebx, 0xf       FIXED_SHIFT
```

`FIXED_SHIFT = 15` is worth stating as a format rather than a number: every product
in the compose is an `imul` followed by `sar e_x,0xf` (`0x9941`, `0x996D`, `0x9982`,
`0x9999`, `0x99A5`, ...), so the cells are 1.15 fixed point — which is exactly why
`ALIEN_TRANSFORM_NEUTRAL` is `0x8000` over in `croolis.rs`. The same format links
two subsystems that otherwise share no code.

The lesson is small and repeatable: a tool that decodes from an entry is only as
good as the entry, and "the routine before the cells I want" is not the same as "the
routine that uses them". Both entries here were verified by the `retf`/`ret`
criterion; only one was the right routine.

`ship3d.rs`: 64 uncited constants -> 60.

620 tests, 0 failures.

## #500 — the point projector, and a clip rectangle confirmed from BOTH ends

Six constants from the projector at `0x9A10`, found by walking one more routine
boundary along from #499 (`retf` @`0x9A0F`, prologue at `0x9A10`):

```text
  0x9A1D  mov word [0x2f77], 0x3e8   POINT_CLOUD_COUNT = 1000, the LOOP COUNTER
  0x9A23  mov si, 0x2fc1             POINT_BUFFER, the source
  0x9A26  mov di, 0x4f01             WORK_VECTOR, the per-point scratch
  0x9A31  mov bp, 0x2f95             the matrix #499 sourced
  0x9A3F  mov ax, [0x2f65]           CAMERA_X ...
  0x9A44  mov ax, [0x2f67]           ... Y ...
  0x9A4A  mov ax, [0x2f69]           ... Z
```

`POINT_CLOUD_COUNT` is the one that changes meaning once you see the instruction:
1000 is written INTO A CELL AS THE LOOP COUNTER at routine entry, so the star field
is a fixed-size cloud, not a list the projector walks until a terminator. A port
that treated it as a capacity could silently project fewer points and look right.

The better result is a CONFIRMATION FROM THE OTHER END. The projector calls `0x9B04`
@`0x9AEB`, which is the clip test, and it reads `[0x5239]` @`0x9B19` and `[0x523b]`
@`0x9B1F` as its y bounds — the exact cells the navigation routine WRITES as 35 and
165 (#495). Until now those two constants were sourced only from the side that sets
them, where "band top" and "clip bottom" were names I gave the writes. Seeing the
plot test consume them as a bound pair proves they are a clip rectangle rather than
two unrelated cells that happen to be adjacent. Both constants now cite the reader
as well as the writer.

NOT FOUND, and left alone: `PROJECTED_X/Y/DEPTH_OFFSET` (`0x2fb9`/`0x2fbb`/`0x2fbd`)
appear in neither routine's displacement list, so they are reached through a base
register. After #491, a failed scan is not evidence of absence — they stay
UNVERIFIED rather than being attributed to this routine because they sit eight bytes
below the point buffer and the arithmetic would be tidy.

`ship3d.rs`: 60 uncited constants -> 54.

620 tests, 0 failures.

## #501 — the tidy inference I declined in #500 was WRONG, and that is the point

#500 left `PROJECTED_X/Y/DEPTH` (`0x2fb9`/`0x2fbb`/`0x2fbd`) unverified with a note
that they sit "eight bytes below the point buffer" and that the arithmetic being
tidy is not evidence. Followed them properly, and the tidy reading was wrong.

They are not relative to the point buffer at all:

```text
  0x9A31  mov bp, 0x2f95            the PROJECTION MATRIX base
  ...
  0x9AAD  add ax, 0xa0              screen-centre x (+160)
  0x9AB0  mov word [bp+0x24], ax    0x2f95 + 0x24 = 0x2FB9   PROJECTED_X
  0x9AE2  add ax, 0x64              screen-centre y (+100)
  0x9AE5  mov word [bp+0x26], ax    PROJECTED_Y
  0x9AE8  mov word [bp+0x28], cx    PROJECTED_DEPTH
```

`0x24` is 36 — exactly nine dwords — so the projected triple begins IMMEDIATELY
after the 3x3 matrix. These are fields of ONE structure whose base is the matrix,
which is why no immediate or direct displacement for them exists anywhere in the
image: they are only ever reached as `[bp+disp]`.

Both readings put the cells in the same place. Only one says what they ARE. Had I
taken the tidy inference, the port would carry three constants documented as
neighbours of a buffer they have no relationship to, and the next person to move
the point buffer would have "fixed" the offsets by the wrong rule.

Two details fell out that a bare address would not carry: the stored X and Y are
ALREADY SCREEN-CENTRED (`+160`, `+100` after the perspective divide), and the stored
depth is `cx`, the very divisor the two `idiv ecx` @`0x9AAA`/`0x9ADF` divided BY —
so it is the perspective denominator, not a transformed z. Also confirmed in
passing: `dec word [0x2f77] / jne 0x9A34` @`0x9AEE` closes the loop on the cell
#500 sourced as `POINT_CLOUD_COUNT`, so 1000 really is an iteration count.

`ship3d.rs`: 54 uncited constants -> 51.

620 tests, 0 failures.

## #502 — the star plot: 320 is two shifts, and the shade is a depth cue

Six constants from the plot routine `0x9B04` and the projector's tail, and two of
them are only meaningful as arithmetic.

**320 IS NEVER AN IMMEDIATE.** The row stride is built from two shifts:

```text
  0x9B25  mov di, bx      di = y
  0x9B27  xchg bh, bl     bx = y * 256   (bh was zero)
  0x9B29  shl di, 6       di = y * 64
  0x9B2C  add di, bx      di = y * 320
```

`64 + 256`. Searching the image for `0x140` to source `SCREEN_WIDTH` would return
nothing at all — the same shape as #491's 287 (`232 + 55`) and #484's computed
descriptor. Three instances now, and the pattern is worth stating plainly: THIS
COMPILER PREFERS SHIFTS AND ADDS TO IMMEDIATES, so a constant's absence from an
immediate scan is close to no evidence at all in this binary.

**The shade is a depth cue, not a palette pick.** `mov ax,[bp+0x28]` @`0x9B37`
reads back the depth #501 sourced, then `shr ax,0xc` @`0x9B3A`, `neg al` @`0x9B3D`,
`add al,0xef` @`0x9B3F`. The plotted colour is `0xEF - (depth >> 12)`: points darken
with distance from 239 downward. `SHADE_BASE = 239` documented as a bare palette
index would invite someone to "correct" it against a palette dump; documented as the
top of a ramp it cannot be misread.

The other four are direct: `add ax,0xa0` @`0x9AAD` and `add ax,0x64` @`0x9AE2` are
the screen centres applied AFTER the divide, and `sar eax,7` @`0x9AA4`/`0x9AD9` is
the pre-divide scale on both axes.

`ship3d.rs`: 51 uncited constants -> 45.

620 tests, 0 failures.

## #503 — the object pass: a copy wider than its own record

Seven constants from the per-object pass at `0x9B9A`. The descriptor address is one
expression spread over four instructions, and no part of it is a stored constant:

```text
  0x9BD1  mov ax, [0x2f77]     the loop index
  0x9BD4  add ax, 0x15         + 21     INDEX_BIAS
  0x9BD7  shl ax, 5            * 32     STRIDE, as a SHIFT
  0x9BDA  add ax, 0x6212       + base   DESCRIPTOR_BASE
  0x9BE1  test ax, 0x80        VISIBLE_FLAG gates the body
```

`(index + 21) * 32 + 0x6212`. The bias is the part a name cannot carry: anchor 0
addresses descriptor TWENTY-ONE, so the anchor table indexes into the middle of a
larger descriptor array. And `STRIDE = 32` is a `shl`, so — as in #502 — an
immediate scan for 32 finds nothing.

**THE COPY IS WIDER THAN THE RECORD.** Anchors are 6 bytes (`add bx,6` @`0x9CF5`),
but the loop copies them with two dword moves:

```text
  0x9BC2  mov eax, [bx]     / mov [di], eax
  0x9BC8  mov eax, [bx+4]   / mov [di+4], eax     EIGHT bytes
```

Each record's copy runs two bytes into the next one. It is harmless in the game
because only three words are then used (`sub` on `[di]`, `[di+2]`, `[di+4]`), so the
trailing word is never read — this is the compiler taking two aligned dword moves
over three word moves. But it is exactly the kind of detail that misleads: a reader
inferring the record size FROM THE COPY would get 8, and a port with an 8-byte
stride would silently visit every other anchor and drop the rest. The stride is the
`add bx,6`, not the copy width.

One more shared-cell note: `ANCHOR_COUNT` is written to `[0x2f77]` @`0x9BB4`, the
same loop-counter cell `POINT_CLOUD_COUNT` loads with 1000 (#500). One counter
reused per pass, so 11 and 1000 are never live at the same time.

`ship3d.rs`: 45 uncited constants -> 38.

620 tests, 0 failures.

## #504 — the object scale: a guarded divide and a double-precision shift

Four constants from the object pass's projection body, `0x9C23..0x9CC8`.

**The divide is guarded, and the guard is two separate branches.** `SHIP_3D_OBJECT_
DEPTH_WRAP_BIAS` looks like an unconditional bias until the jumps are read:

```text
  0x9C23  je 0x9CF4         depth ZERO -> skip the object entirely
  0x9C27  jns 0x9C30        depth POSITIVE -> no bias
  0x9C29  add ecx, 0x10000  depth NEGATIVE -> wrap it positive
  0x9C3D  div ecx           unsigned divide, now safe
```

Zero exits, negative wraps, positive passes through. A port applying the bias
unconditionally would corrupt every positive depth, and one omitting the zero exit
would divide by zero on the first object at the camera plane.

**`SCALE_NUMERATOR` is computed** — `mov eax,0x8000000` @`0x9C30` then `shr eax,7`
@`0x9C36` = `0x100000`. Fourth instance of this compiler preferring a shift to an
immediate (#502's 320, #503's 32, #491's 287). Searching for `0x00100000` finds
nothing.

**`SCALE_SHIFT` is a `shrd`, not a `shr`.** `shrd ax,dx,0xa` @`0x9CBB`/`0x9CC8` is
the 386 DOUBLE-PRECISION shift: the value spans `dx:ax` and the ten bits shifted out
of `ax` are refilled from `dx`. Modelled as a plain `>> 10` on a 16-bit value it
silently drops the high half, which for a scale factor means large objects collapse
rather than clip.

`PROJECTED_SCALE_OFFSET` lands at `[bp+0x2a]` = `0x2FBF`, immediately after the
projected x/y/depth triple #501 sourced — so the object pass writes its scale into
the same matrix-based structure the point projector uses. That structure is now
four fields deep and every one of them was found as a `[bp+disp]`, never as an
address.

`ship3d.rs`: 38 uncited constants -> 34.

620 tests, 0 failures.

## #505 — object descriptors ARE sprite slots, and the citation guard earned its keep

Four flag constants, and the routines that own them settle a structural question the
port had left implicit.

`0x299:0x1241` (file `0x41D1`) and `0x299:0x133D` (file `0x42CD`) — the slot-state
and extent entries #490 sourced by call site — BOTH open with:

```text
  shl ax, 5
  mov bx, 0x6212
  add bx, ax
  mov ax, gs:[bx]
```

That is exactly `SHIP_3D_OBJECT_DESCRIPTOR_STRIDE` and `..._BASE_OFFSET` from #503.
So "object descriptor" and "sprite slot" are two names for ONE 32-byte record, and
the flags live in its first word alongside the visible bit. The port names them in
two families; the binary has one table.

The flags are a handoff rather than independent bits:

```text
  0x41DE  or al, al / jns      bit 7 VISIBLE ...
  0x41E2  test al, 1 / je      ... and bit 0 ACTIVE
  0x41E6  and al, 0xfe         clear ACTIVE ...
  0x41E8  or al, 2             ... set DIRTY in the same breath
  0x42DD  test al, 0x81        the extent entry tests the PAIR at once
  0x42ED  btr ax, 4            EXTENT_CHANGED, cleared when extents match
```

`EXTENT_CHANGED_FLAG` is worth its own note: the code contains the BIT INDEX `4`,
never the mask `0x10`, because `btr` is a 386 bit-test-and-reset. An immediate scan
for `0x10` finds nothing relevant — the same lesson as #503's `shl 5` for 32, now
in a form where the value is not even a power-of-two multiplier but an index.

I SETTLED THE ROWS BEFORE READING THE TEST OUTPUT and the suite failed on the next
run — the #423 mistake repeated. What caught it was
`quoted_instructions_match_the_disassembly`: I had written `test al,1 / je` at
`0x41E4`, which is the `je`'s address; `test` is at `0x41E2`, two bytes earlier. The
guard reported exactly that, including the correct address. 944 cited instructions
verified, one wrong, and the wrong one was mine and two bytes off — a citation that
would have read as perfectly plausible to any human reviewer.

`ship3d.rs`: 34 uncited constants -> 30.

620 tests, 0 failures.

## #506 — a sentinel the reader never compares, and an alias kept as an alias

Six more `ship3d.rs` constants. Two are worth more than their addresses.

**`DIRTY_RECT_SENTINEL` is not tested for.** The port names `0xFFFF`, and `0xFFFF`
is indeed what the writer stores, but the walker at file `0x50B7` terminates with:

```text
  0x50B4  mov ax, es:[di]
  0x50B7  or ax, ax
  0x50B9  js 0x517B          <- ANY negative ends the list
```

A sign test, not a comparison. So the game's rule is "negative terminates" and
`0xFFFF` is merely the negative it happens to write. A port terminating on equality
with `0xFFFF` is STRICTER than the game and would run past any other negative entry.
Same species as #492's zero-or-`0xFFFF` pair: the constant records what is written,
the instruction records what is accepted, and only the second is the rule.

**`FINAL_RESET_SCROLL_MODE` is deliberately left an alias** of
`SHIP_3D_SCROLL_MODE_HOLD` rather than given its own `= 10`. The final reset really
does restore the hold mode, so duplicating the literal would create a second thing
to keep in step with `cmp word [0x524d],0xa` @`0xB6F0`. One value, one citation.

The rest are direct: `DIRTY_RECT_LIST_OFFSET` is `mov di,0x6612` @`0x787C` sitting
immediately before the `lcall 0x299,0x210d` @`0x787F` that #490 sourced by census —
the list and the routine that consumes it, confirmed from both sides again. The
temp-SND path pair (`0x0D23` `sn\3D.snd` @`0xB5D7`, `0x0CFC` `sn\tb.snd` @`0xB610`)
comes from #484's routine, cited now rather than left implicit in the fix log.

Noted in passing: the dirty-rect walker computes its row address with
`xchg bh,bl / shl cx,6 / add` @`0x50C4` — the SAME 320-stride idiom #502 found in
the star plot. Two independent routines, one arithmetic trick, and neither contains
`0x140`.

`ship3d.rs`: 30 uncited constants -> 24.

620 tests, 0 failures.

## #507 — the intro flyby ACCELERATES, and the constant that says so is one indirection away

Five constants from the intro camera approach, `0x8A76..0x8AFE`, a four-phase state
machine on the counter `[0x27DF]` (`inc byte [0x27df]` @`0x8AAC`/`0x8AD9`).

`SHIP_INTRO_Z_ACCEL_STEP` is the one that would be easy to get wrong, because the
100 is not applied to the thing it appears to move:

```text
  0x8ABF  add ax, [0x2f6b]          Z gains the VELOCITY ...
  0x8AC3  add word [0x2f6b], 0x64   ... and the velocity gains 100
  0x8AC8  mov [0x2f69], ax
```

The step feeds a velocity cell, so the approach ACCELERATES. A port adding 100 to Z
each frame would produce a constant glide — same endpoints, wrong motion, and
nothing in the constant's name or value distinguishes the two.

`SHIP_INTRO_YAW_WRAP` similarly hides a direction: `dec ax / jns` @`0x8A8B` with
`mov ax,0xb4` @`0x8A8E` means the yaw counts DOWN and reloads at 180. The routine
also contains an increment-and-wrap-to-zero path @`0x8A9E` against the same 180, so
"wrap" alone does not say which way the ship spins.

Four cross-confirmations arrived free, all from constants sourced in earlier
entries: the routine reads `[0x2f65]` and `[0x2f69]` (camera X/Z, #500) and
`[0x2f71]` (matrix angle, #499), and calls `lcall 0x299,0x12b0` @`0x8AD4` and
`lcall 0x299,0x1241` @`0x8AED` — the sprite range-dirty and slot-state entries #490
found by census and #505 disassembled. The intro, the projector, the matrix and the
sprite driver are now demonstrably the same subsystem rather than four separately
named groups of constants.

`ship3d.rs`: 26 uncited constants -> 21.

620 tests, 0 failures.

## #508 — three kinds, one branch: the position resolver

Six constants from the object-position resolver, `0x60E0..0x61D9`, which turns an
object's KIND word into a field offset through `vm_field_offset` (`0x6023`, the
`bsf` matrix whose two port copies #486a merged).

The structure the constants alone do not show:

```text
  0x61B2  cmp ax, 8     / je 0x61DF   ┐
  0x61B7  cmp ax, 0x10  / je 0x61DF   ├─ SAME target: one behaviour
  0x61BC  cmp ax, 0x200 / je 0x61DF   ┘
  0x61AD  cmp ax, 0x100 / je 0x61EB   its own path
  0x6114  cmp ax, 0x40  / jne         selector-11 path @0x611B
```

Three of the five kinds jump to the SAME address, so `DIRECT_8`, `DIRECT_10` and
`DIRECT_200` are one case wearing three names. The port's naming already implied
that; the shared branch target proves it, and it means a change to one must be a
change to all three or the port's cases diverge from the game's single one.

`KIND100` earns its different name: it alone runs a two-word comparison (selectors
12 and 14, then 9 or 10 depending on the result) — the `mov ax,9 / cmp / inc ax`
sequence whose neighbours were already cited at `0x6101`/`0x6108`.

And the fall-through case is a LINK WALK, worth recording even though no constant
names it: selector `0x11` reads a link, `cmp si,-1` @`0x61CD` tests for the end, and
on -1 it reloads from `gs:[0x6752]` @`0x61D2` and loops back to `0x61AD`. So an
object with none of the five kinds resolves its position by following owner links
until a sentinel — which is the sort of thing that looks like recursion in a port
and is a loop in the game.

`ship3d.rs`: 21 uncited constants -> 15.

620 tests, 0 failures.

## #509 — two record types in a VM chain, and an asymmetry I did not smooth over

`SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE` (`0xC3`) and
`SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE` (`0xC4`) are named as ship-3D navigation
constants and live in `ship3d.rs`, but they are VM RECORD TYPES, tested in the
post-update dispatch chain:

```text
  0x5D37  cmp ax, 0xc3 / jne 0x5D8F      -> falls through to ...
  0x5D8F  cmp ax, 0xc4 / jne 0x5E22      -> ... and then to the 0xC6 branch
  0x5E13  mov word ptr [di], 0xc4        the 0xC4 handler STAMPS 0xC4 into a record
```

The `0x5E13` write is the better citation for `DEFERRED_RECORD_TYPE`, because the
port's use is `effect.deferred_record_type = Some(...)` — a WRITE. The handler marks
a record deferred by stamping its own opcode into the field selector `0x13` resolves.

`0xC3` HAS NO SUCH WRITE that I could find, and I am recording that as an asymmetry
rather than resolving it. The port sets it as a `deferred_record_type` too, but only
its read side is evidenced. What I explicitly did NOT do is conclude the port is
wrong: #502 established that this binary builds values with shifts and adds often
enough (320, 287, 32, `0x100000`) that "no immediate-form write exists" is close to
no evidence. The honest position is that the read is verified, the write is not
found, and the doc comment says exactly that.

This is the same discipline as #500's declined inference, and #501 showed why it
matters — there the tidy story was wrong. Here I do not yet know whether the port's
`0xC3` write is right, and pretending the citation covers it would hide a real open
question behind a real address.

`ship3d.rs`: 15 uncited constants -> 13.

620 tests, 0 failures.

## #510 — one row boundary, two names; and "a reason" is not "evidence"

Three constants, and two of them were justified by REASONING rather than cited.

`SHIP_3D_PROJECTION_SCREEN_HEIGHT` carried the note "the DOS pixel helper computes
`y * 320 + x`; 200 native rows cover it". That is a correct inference and not a
citation — nothing in it could fail if the value were wrong. The binary states it
directly where it restores the clip: `mov word [0x523b],0xc8` @`0xB41D` (#495).

`SHIP_3D_HUD_BAND_TOP = 165` is the SAME `0xA5` the navigation routine writes as the
clip bottom @`0xB40D`. The scene band ends exactly where the HUD band begins, so
this is ONE row boundary written into ONE cell and read under two names — not two
independently chosen values that happen to match. Anyone changing one must change
the other, and until now nothing in either doc said so.

`RADIO_SND_PATH_OFFSET` is `mov si,0xd16` @`0x8860`, `sn\radio.snd`, and the detail
worth keeping is the argument: the loader `lcall 0xb1b:0x855` @`0x8866` is called
with **AX=1** here, where the `sn\3D.snd` and `sn\tb.snd` loads of #484/#506 pass
**AX=0**. Three calls to one loader, two different modes — so AX is a parameter the
port must carry, not a constant zero. The routine is four instructions and its `ret`
@`0x886B` lands immediately before nav-choice handler 4, which is how #494's
dispatch table and this load corroborate each other's addresses.

WITH THESE, `ship3d.rs` HAS 10 UNCITED CONSTANTS LEFT, from 115 when the tranche
began. The remainder are the target-record and navigation-record flags, which need
their own routines found rather than another pass over ones already read.

620 tests, 0 failures.

## #511 — into vm.rs: the TEXT flags, and bit 7 that is never `0x80`

First tranche of `vm.rs`'s 37 uncited constants, from the `0xA6` TEXT handler
(`vm_op_a6_text` @`0x660C`) and two hold computations.

`TEXT_ACTIVE_DISPLAY_FLAG` is the interesting one: the port names `0x80` and the
handler never contains it. Bit 7 is tested with a SIGN BRANCH (`jns 0x67A0`
@`0x6649`) and cleared with a COMPLEMENT MASK (`and byte ptr [si+1],0x7f`
@`0x6698`). This is now the third distinct disguise for bit 7 in this binary —
croolis's y-floor `js` (#488), the dirty-rect sign terminator (#506), and now this
— which is worth stating as a rule: IN THIS BINARY, A BIT-7 FLAG IS USUALLY A SIGN
TEST, so its mask will not be in the code and its absence means nothing.

The other three are direct: `test cl,8` @`0x661E` arms the skip counter
(`DS:0x67AB`), `test cl,0x10` @`0x6630` sets the loop target (`DS:0x6778`) — both
matching labels.csv's existing `b4&0x08` / `b4&0x10` notes, which had never been
carried into the port's constants — and `test word ptr es:[di+2],0x8000` @`0x665A`,
a WORD test where the others are byte tests.

`ACTIVE_LINE_ID_BIAS = 9` sits between two instructions that give it meaning:
`mov ax,[0x1fab]` @`0x11F2` reads the b3 selector the TEXT handler stored, `add ax,9`
@`0x11F5`, `mov [0x6788],ax` @`0x11F8`. The active line ID is the SELECTOR PLUS NINE
— so the two are different numbering spaces, and code comparing a raw selector
against an active line ID is off by nine.

`CHATTER_HOLD_EXTRA_TICKS = 6` is the constant term of
`gs:[0x27cf] * (gs:[0xaca] >> 1) + 6` @`0x7385` — a FLOOR, so even a zero text speed
still holds six ticks.

620 tests, 0 failures.

## #512 — a flag whose name is a NEGATIVE, proved by the branch it skips

Two more `vm.rs` constants, and both are cases where the instruction says something
the value cannot.

**`TEXT_PRESERVE_ACTIVE_FLAG` does not set anything.** `test cl,1 / jne 0x669C`
@`0x6693` jumps OVER `and byte ptr [si+1],0x7f` @`0x6698` — the very instruction
#511 identified as the clear of `TEXT_ACTIVE_DISPLAY_FLAG`. So b4 bit 0 preserves
bit 7 by SKIPPING ITS CLEAR. The name was right and unevidenced; now the mechanism
is on record, and the two flags are documented as the pair they are rather than two
unrelated bits that happen to live in the same byte.

**`TEXT_SELECTOR_NONE` is a byte constant living in a word cell.** The port declares
`u8 = 0xFF`; the cell `DS:0x1FAB` is a word holding `0xFFFF`. Both are right, and
the bridge is a sign extension:

```text
  0x668D  lodsb                      b3, one byte
  0x668E  98                         CBW -- 0xFF becomes 0xFFFF
  0x668F  mov word gs:[0x1fab], ax
```

with the same `0xFFFF` written directly as the reset value at `0x1A64`, `0xB460`
and `0xB529`. Worth noting that `0x98` prints as `cwde` in the disassembler's
16-bit mode — the trap #497 hit and `check_opsize_mnemonics.py` guards. Read as
CWDE the store would preserve AH and a b3 of `0xFF` would NOT become the reset
value, breaking the correspondence that makes these two constants one thing.

`TEXT_SELECTOR_SILENT` (`0x00`) and `TEXT_EXTRA_CONTROL_WORD_FLAG` (`0x04`) are NOT
settled: neither appears in this handler's immediates, and after #509 I am not
inferring them from the neighbours that do.

620 tests, 0 failures.

## #513 — one kind is an EQUALITY test, the next is a BITMASK, and that is not cosmetic

`LOCATION_KIND_BLACK_HOLE = 0x100` is `test word ptr fs:[bp],0x100 / je` @`0x8376`,
in the status-header selector at `0x8369`. The port's `kind & ... != 0` matches.

What makes it worth an entry is the instruction three lines earlier:

```text
  0x8369  mov si, 0x12e                        default: PLANET
  0x836C  cmp  word ptr fs:[bp], 0x10 / jne    SHIP: EQUALITY
  0x8376  test word ptr fs:[bp], 0x100 / je    BLACK HOLE: BITMASK
```

Two adjacent kind tests, two different operators. So a location's kind word is not
an enum: it carries a VALUE that may equal `0x10` and independent FLAG BITS of which
`0x100` is one. Writing `kind == LOCATION_KIND_BLACK_HOLE` would pass every test
with a location that has no other bits set, and fail silently on one that does —
which is exactly the sort of bug that only appears on the one map object that
combines flags.

NOT SETTLED, and worth saying why rather than leaving them silently open: the five
`LOCATION_PANEL_*` geometry constants (`Y`, `ROW_PITCH`, `NAME_GAP`, and the two
colours) are not in this routine. `0xEE` does not occur as `mov al,0xEE` anywhere in
`0x8000..0x9000`, and the two `mov al,0xFE` sites that DO occur there — `0x8595`,
`0x85CC` — are the target-row colours #492 already attributed to the ship-3D list
widget. Adopting either as the panel's row colour because the byte matches would be
the #501 mistake exactly: same value, wrong routine, plausible story. They stay
UNVERIFIED until the panel's own draw is found.

620 tests, 0 failures.

## #514 — the panel draw found, and #513's refusal vindicated

#513 left the five `LOCATION_PANEL_*` constants UNVERIFIED rather than adopt the
`mov al,0xFE` sites sitting a few hundred bytes from the header selector. Found the
real routine, and the refusal was right.

`mov al,0xEE` occurs EXACTLY ONCE in the whole image, at `0x9181`, immediately
before the header's `lcall 0x299,0x202` (RENDER_STRING, #490). That anchors the
panel at `0x9140..0x91D9`:

```text
  0x915B  mov bx, 0x6e        x cursor = 110
  0x915E  mov dx, 0x19        y cursor            PANEL_Y
  0x9181  mov al, 0xee        HEADER_COLOR
  0x9183  lcall 0x299, 0x202  draw the header
  0x9188  add bx, [0x27cd]    advance by the MEASURED header width ...
  0x918C  add bx, 6           ... then a fixed gap    NAME_GAP
  0x91A5  add dx, 0xa         next row               ROW_PITCH
  0x91C0  mov ax, 0xfe        ROW_COLOR
```

**`ROW_COLOR` is `0x91C0`, not `0x8595`.** The `0xFE` at `0x8595` is the ship-3D
target row's active colour (#492) — a different routine, a different subsystem, the
same byte. Had #513 taken the nearer site because the value matched, this constant
would now carry a citation into the list widget, and anyone changing the widget's
hover ladder would have been told they were changing the location panel.

`NAME_GAP` also reads differently once its neighbour is visible: `add bx,[0x27cd]`
@`0x9188` advances the x cursor by the MEASURED width of the header just drawn, and
only then does `add bx,6` apply. So 6 is a GAP BETWEEN TWO STRINGS, not a column —
a port treating it as an x offset would misplace every name whose header differs in
width from the one it was tuned against.

One structural note: the header selector (`0x137`/`0x13E` with `test ...,0x100`)
appears TWICE — at `0x8369`, which assembles the strings into a buffer with `0x0D`
separators, and again at `0x9170` inside this draw. Two copies of the same choice in
the game, which is worth knowing before "deduplicating" the port's.

620 tests, 0 failures.

## #515 — the gs: prefix moves the address, and #505 taught me to check

Three C2 constants from the post-update region, and the process detail is the part
worth recording.

A byte-pattern search for `mov word [0x6788],0x27` reported `0x5D01`. The
INSTRUCTION is at `0x5D00`: the store carries a `65` GS SEGMENT PREFIX, and the
pattern I searched for begins one byte inside it. Citing `0x5D01` would have been
wrong by exactly the margin #505's citation guard caught last time — and it would
have passed unnoticed by any reader, because `0x5D01` looks like a perfectly
ordinary address. I disassembled from the candidate before writing the citation,
which is now the habit: a byte-search offset is a LEAD, not an address.

With the true addresses, the branch structure explains the names:

```text
  0x5CFA  mov byte gs:[0x1fb2], 0        clear the presentation gate
  0x5D00  mov word gs:[0x6788], 0x27     KIND2 active line
  0x5D09  cmp bx, 0x400 / jne            the other kind ...
  0x5D19  call 0x7409                    ... calls vm_c2_descript_lookup ...
  0x5D26  mov word gs:[0x6788], 0x2b     ... then KIND400 active line ...
  0x5D2D  or byte gs:[0x67aa], 2         ... then sets the BUSY flag
```

`C2_PRESENTATION_BUSY_FLAG` is set ONLY on the kind-0x400 path and only AFTER the
descript.des helper returns, which makes it a COMPLETION signal, not an entry
marker. A port raising it on entry would report the presentation busy during a
lookup that may not succeed.

Both active-line values are written at several sites (`0x27` also at `0x195E` and
`0x6EBA`; `0x2B` at `0x1922`, `0x5FC0`, `0x6A9D`, `0x6EE0`), so these are shared
line IDs rather than constants private to the C2 handler — the doc lists them so a
future change is known to touch five places, not one.

620 tests, 0 failures.

## #516 — a cell name proved by a dereference, and a census that gives counts not addresses

Two presentation cells, and one tooling note.

`VM_PRESENTATION_PRIMARY_C4_RECORD` is named for a record type; the code shows the
name is literally true, in three instructions:

```text
  0x586C  mov bx, word ptr gs:[0x675e]   load the cell
  0x5871  mov ax, [bx]                   DEREFERENCE it
  0x5873  cmp ax, 0xc4                   the pointed-to record's TYPE
```

So it holds a POINTER to a record, not a record id — and the `0xC4` it is checked
against is the same record type #509 sourced. A port storing an id here would
compile and be wrong at the first dereference.

`VM_PENDING_RESOURCE_PROFILE`'s setter is one instruction and a `ret` (`0x64BB`,
`0x64BF`), and the cell is reset to `0xFFFF` at `0x10D3` and `0x1CFA` — so `0xFFFF`
is its EMPTY value, not a profile number. Worth stating because "pending profile" is
the kind of field a reader assumes is zero-initialised.

TOOLING: `re/tools/addr_forms.py` reports "6 distinct site(s) referencing 0x6780"
and a breakdown by immediate, but NOT the addresses. That is fine for the question
it was built for (#436: does anything write this cell?) and useless for citing,
which needs a site. I scanned the ten common `[disp16]` encodings directly instead —
including the `65` GS-PREFIXED variants, whose instruction address is one byte
BEFORE the pattern match (#515's lesson, applied rather than re-learned). Neither
tool change is worth making yet: the direct scan is six lines and this is the second
time I have needed it.

`0x67AF` (`VM_PRESENTATION_RELATED_FLAG20`) matched NONE of those forms, so it is
reached by base register. It stays UNVERIFIED — after #491 that is not evidence of
absence, and after #501 it is not an invitation to attribute it to a neighbour.

620 tests, 0 failures.

## #517 — the opcode groups are the GAME'S grouping, read from the dispatch table

`ASSIGN_7`, `BITMASK_5` and `ASSIGN_5` list opcodes that the port handles together.
Three uncited arrays of hex bytes are exactly the shape of a port-side convenience
grouping — a guess that becomes fact by being written down. They are not: every
group is a set of opcodes THE DISPATCH TABLE POINTS AT ONE HANDLER.

Reading the table required resolving its entries, which are near offsets, not
addresses. `DS:0x6EB0` is file `0x142D0`; entry `i` is opcode `0xA0 + i`; and the
segment base is file `0x53A0` (seg `0x4DA`). I did not assume that base — I checked
it against four handlers whose addresses were already known from their own decodes:

```text
  0xA0 -> 0x6559  vm_op_a0_push
  0xA6 -> 0x660C  vm_op_a6_text
  0xB7 -> 0x6AA7  vm_op_b7_record_op
  0xB8 -> 0x6B06  vm_op_b8_record_readwrite
```

All four land exactly. Then:

```text
  ASSIGN_7   0xB1 0xB4 0xB5 0xB6 0xBE 0xBF 0xC0  -> 0x6863   (all seven)
  BITMASK_5  0xAE 0xB0                           -> 0x6902   (both)
  ASSIGN_5   0xAD 0xAF 0xB2 0xB3 0xBA 0xBB 0xBC  -> 0x6946   (all seven)
```

One handler per group, three distinct handlers, no opcode in two groups. The port's
grouping is the binary's.

`dispatch_table_groups_share_one_handler` now PARSES the table out of the image and
asserts all of it, including the base-validating spot checks — so if the groups were
ever edited to match a port refactor rather than the game, the test fails. That is
the difference between a comment saying these share a handler and evidence that they
do.

One naming note left as-is: `BITMASK_5` contains TWO opcodes, so its "5" is not a
count. Whatever it refers to (a mode, a field width) is not established, and I have
not renamed it on a guess — the doc records that the number is not the size.

621 tests, 0 failures.

## #518 — a tool for the dispatch table, and what the 0xD3 entry does NOT prove

#517 resolved the opcode dispatch table by hand. `re/tools/vm_dispatch.py` makes it
repeatable: it reads `DS:0x6EB0` (file `0x142D0`), resolves each near offset against
the segment base (file `0x53A0`, seg `0x4DA`), and prints the map grouped by handler.

The base check is FATAL, not advisory. If `0xA0`/`0xA6`/`0xB7`/`0xB8` stop landing
on `0x6559`/`0x660C`/`0x6AA7`/`0x6B06`, the tool exits rather than printing a map,
because every number it would print is derived from that base. A tool that degrades
quietly when its assumption breaks is worse than no tool — it produces citable-looking
addresses that disassemble into plausible nonsense, which is the exact failure #499
hit by entering the right-looking routine.

The full map: **52 opcodes, 37 distinct handlers, four shared groups.** Three are
`ASSIGN_7`/`BITMASK_5`/`ASSIGN_5` from #517. The fourth is `0xB8 0xB9 0xBD -> 0x6B06`
— and I went looking for a gap there, because `vm.rs`'s handler doc names only
`0xB8`. There is none: the port matches `0xB8 | 0xB9 | 0xBD` in one arm and already
documents it as "a true family". Worth the check; not worth a change.

WHAT THE `0xD3` ENTRY DOES NOT PROVE. Its offset is `0x0000`, and I nearly wrote
that this confirms the existing OP_MAX decode (dispatch ends at `0xD2`, tokens run
to `0xFF`). Disassembling `0x53A0` shows a REAL PROLOGUE — `push bx / push cx /
push dx / push es / push di` — so offset zero lands on the segment's first routine
by layout, not on a null. The `0xD3` slot therefore looks like a live handler if you
read only the offset. The OP_MAX conclusion still stands on its own evidence (the
table is 104 bytes and `vm_token_advance` indexes a different, larger table), but
this entry is NOT additional support for it, and recording it as such would have
been a fabricated corroboration of a correct conclusion.

621 tests, 0 failures.

## #519 — two field selectors, and a kind that is read rather than assumed

`VM_FIELD_OFFSET_SELECTOR_C9_RELATED` is `mov ax,0x13` @`0x6FD7`, inside the `0xC9`
handler — an address I did not have to hunt for, because `re/tools/vm_dispatch.py`
(#518) resolves `0xC9 -> 0x6FB9` directly. First time this session that a tool built
one entry earlier paid for itself immediately. Worth noting the selector is NOT
private to `0xC9`: the `0x5816` handoff loads the same `0x13` @`0x583D`.

`VM_FIELD_OFFSET_SELECTOR_PRESENTATION_HANDOFF` was harder — `0x02` is too common to
scan for. Found by looking for the SHAPE instead: `mov ax,2` followed within a few
bytes by a near `call` resolving to `vm_field_offset` (`0x6023`). Exactly two sites
in the image, `0x5895` and `0x73D6`, and the first is inside the `0x5816` routine
this constant belongs to.

THE INSTRUCTION BEFORE IT IS THE ONE THAT MATTERS: `mov bx,word ptr [si]` @`0x5893`
loads the KIND from the record. The selector is a literal, the kind is not. I checked
the port against that specifically, because its test call sites pass a literal `2`
for both arguments and that would have been a plausible thing to copy into the real
path. It does not — `post_update_presentation_handoff` reads `owner_kind` from the
record and passes it. Correct, and now evidenced rather than coincidental.

The general point, since this is the second time in three entries (#513's
equality-vs-bitmask, now this): WHAT A HELPER'S ARGUMENTS COME FROM is as much part
of the decode as the helper's address. A citation that names only the constant's
instruction can be right while the surrounding call is wrong.

621 tests, 0 failures.

## #520 — `0xFFFF` produced by `not ax`, and why the guard is part of the constant

Two `vm.rs` constants, and the second is the clearest example yet of a value that
exists only as arithmetic.

`SPECIAL_OBJECT_SLOT_COUNT = 16` is `mov cx,0x10` @`0x5FFB`, the bound for the slot
array at `mov bp,0x6d3e` @`0x5FF8` — and it drives BOTH loops in the routine
(`loop 0x5FFE` @`0x6006`, `loop 0x600E` @`0x6017`). So 16 is the array's real size,
not a capacity the port picked.

`C4_POST_UPDATE_SENTINEL = 0xFFFF` is never written as an immediate anywhere:

```text
  0x5D96  mov ax, ds:[bp+4]     the field ...
  0x5D9A  or ax, ax / jne       ... must be ZERO to reach here
  0x5DAA  not ax                so ax is 0xFFFF
  0x5DAC  mov ds:[bp+4], ax     stamped into the record
```

THE GUARD IS PART OF THE CONSTANT. `not ax` only equals "store `0xFFFF`" because the
branch four instructions earlier proved `ax` was zero. Lift the store without the
guard and it writes the complement of whatever the field held. The port already has
the guard — `state_u16(record + 4) != 0 -> return None` — so this is evidence for
code that was already right, but the doc now says WHY the two lines belong together,
which is what stops a later simplification from separating them.

This is the fifth constant this session that is computed rather than stored (#484's
descriptor, #491's 287, #502's 320, #504's `0x100000`, now this). The pattern has
earned a standing conclusion: IN THIS BINARY, "the value is not in the image" is the
normal case, not a red flag — and the interesting question is always which
instructions produce it and what they assume.

621 tests, 0 failures.

## #521 — seven presentation cells, and a "gate" that is really a counter

Seven `DS` cells sourced by scanning the twelve common `[disp16]` encodings directly
(the technique #516 fell back to when `addr_forms.py` gave counts without addresses),
including the GS-prefixed forms whose instruction address is one byte before the
match (#515).

`VM_PRESENTATION_INPUT_GATE_H` is misnamed in a way worth keeping rather than
renaming blind. It IS tested as a gate — `cmp byte ptr gs:[0x2792],0 / jne` @`0x5E29`
— but it is armed with `inc byte ptr gs:[0x2792]` @`0x5E3B`, not a store of 1, and
the same test/inc pair repeats at `0x5E5B`/`0x5E63`. So the cell counts. A port
modelling it as a bool gets the TEST right and the STATE wrong, and the divergence
only appears after 256 arms wrap it back to zero — at which point the gate silently
reopens. I have not renamed it: the port's `!= 0` test matches the game, and the
doc now records that the underlying storage is a counter.

`VM_PRESENTATION_PAIR_WRITE_DISABLED` has exactly two sites and they explain each
other: written by the FIRST instruction of the `0x5816` handler (`0x5817`) and read
by the C4 handler (`test ... ,1` @`0x5DA0`, whose `jne` skips the entire C4 body). It
is a handoff between two handlers, not a general flag — which is why it has no
initialiser anywhere.

`VM_PRESENTATION_HANDOFF_GATE` crosses a subsystem boundary: tested at `0x585C`
inside the VM's handoff and again at `0x8970` in the console dismiss ladder. Worth
recording, because a reader working in either file would see only half its users.

`VM_PRESENTATION_DEFERRED_RECORD_AUX` is a read-modify-write pair inside ONE routine
(`0x5A3B` / `0x5A4D`), so it carries a value across that routine rather than between
subsystems — the opposite shape to the two above, and the reason its name should not
be read as "shared state".

621 tests, 0 failures.

## #522 — the idle-gate array IS an instruction sequence, order included

`MAIN_PENDING_PROFILE_IDLE_GATES` lists ten cells the main loop consults before
letting a pending script profile load. An array of ten already-defined constants is
the shape of a port-side collection — a set someone assembled by noticing which
gates mattered. It is not. It is one instruction sequence:

```text
  0x109C  mov al, byte ptr [0x67ac]     ACTIVE
  0x109F  or  al, byte ptr [0x24f3]     INPUT_GATE_A
  ...                                    (stride 4)
  0x10BF  or  al, byte ptr [0x2792]     INPUT_GATE_H
```

A load and nine ORs at a fixed four-byte stride, and THE ARRAY'S ORDER IS THE
INSTRUCTION ORDER, element for element. That is a stronger result than "these ten
cells are the gates": it means the port's first element is the game's LOAD and the
rest are its ORs, so the array is a transcription rather than a set.

It also makes the set demonstrably CLOSED. An eleventh gate would require an
eleventh instruction, so `idle_gates_are_the_main_loop_or_chain` asserts the bytes
after the last `or` are NOT another `or` — the array cannot be quietly extended to
fix a symptom without the test noticing the game disagrees.

Found via `refs_in_routine.py` on the main-loop routine at `0x1095`, which reports
195 instructions touching 40-odd cells; the ten gates were the first ten it listed,
consecutively. The tool built for #499's DS-cell problem answered a question about
an ARRAY's provenance, which is not what I built it for.

`vm.rs` is now at 3 uncited constants, from 37.

622 tests, 0 failures.

## #524 — five UI strings the port was still transcribing

`STATUS_STRING_TABLE` carries a note this project wrote earlier: the strings "used
to be [here], pinned to these bytes by a test — a verified transcription, which is
far better than a loose literal and still A COPY: it breaks against a differing
build instead of following it. The port now READS them." That conversion was done
for the location headers and never for five others:

```rust
pub const LOADING_TEXT: &'static str = "LOADING";
pub const PAUSE_TEXT:   &'static str = "PAUSE";
pub const CONFIRM_TITLE: &'static str = "ARE_YOU_SURE?";
pub const CONFIRM_YES:   &'static str = "YES";
pub const CONFIRM_NO:    &'static str = "NO";
```

All five were pinned by tests, so nothing was WRONG — and all five were still
content-bearing literals, which `CLAUDE.md` calls a defect outright. Replaced with
`EngineState::load_ds_strings`, which reads them from the image at the DS offsets
the instructions name, keyed into a map; `main.rs` calls it beside the OPTION-menu
labels it already reads that way.

The DS offsets keep their own citations, and they are LOADS, not data addresses:
`mov si,0x159` @`0x16BC`, `mov si,0x166` @`0x1ABB`, `mov si,0x17b` @`0x14FE`,
`mov si,0x189` @`0x150C`, `mov si,0x18d` @`0x151A`, each immediately before its draw
call.

TWO THINGS THE DECODE ADDED beyond removing literals. The confirm dialog places YES
and NO by RELATIVE steps from the title — `add bx,0x14 / add dx,0x11` @`0x150F`
then `add bx,0x3c` @`0x151D` — so the three positions are one anchor plus offsets,
not three independent coordinates. And the two status overlays, which look symmetric
in this file, use DIFFERENT render entries and register conventions: LOADING passes
`ax`/`bx`/`dl` to `0x299:0xD6`, PAUSE passes `bx`/`dx`/`al` to `0x299:0x498`. Only
the shared `y = 0x60` is genuinely common.

The tests were rewritten to prove the CHAIN rather than the value: the DS offset is
read from the instruction operand, the string is read from the image at that offset,
and the engine's loaded string must equal it. No literal appears anywhere in the
assertion.

622 tests, 0 failures.

## #525 — the confirm dialog is ONE anchor and three `add`s

Six `engine.rs` constants, and the confirm geometry is the interesting half.

The port stores three independent-looking coordinate tuples. The game stores one
anchor and steps it:

```text
  0x1501  add bx, 0xa      x = the box's 90, + 10       TITLE
  0x1504  mov dx, 0x58     y = 88, the only ABSOLUTE y
  0x150F  add bx, 0x14     +20 -> 120                   YES
  0x1512  add dx, 0x11     +17 -> 105
  0x151D  add bx, 0x3c     +60 -> 180                   NO
```

NO has no `add dx` of its own — it inherits YES's row, so a single `add bx` is the
entire difference between the two buttons. Three tuples in the port; three
instructions in the game. Anyone nudging the title's y in the port must move two
other constants to match, and nothing said so until now.

`ENGINE_SCREEN_WIDTH` and `ENGINE_SCREEN_HEIGHT` were uncited in the file that
defines them for the whole port. Both are now sourced to instructions found earlier
in this session rather than re-derived: 320 is the `xchg bh,bl` + `shl ...,6` row
stride (#502, and the same idiom again in the dirty-rect walker, #506), and 200 is
`mov word ptr [0x523b],0xc8` @`0xB41D`, the clip bottom the navigation routine
restores (#495). Neither number appears as an immediate anywhere — which is why
they had gone uncited while a dozen constants derived FROM them were settled.

That is the pattern worth naming: the most fundamental constants are the last to get
citations, because nothing about them looks uncertain. 320x200 is obviously right,
so nobody checks where the game says it — and "obviously right" is exactly the
status that survives a wrong value.

622 tests, 0 failures.

## #526 — a sixth literal, and a string the port both read and copied

`OPTION_BOX_LABEL = "CANCEL"` is the same defect #524 removed five of, and its own
doc already said as much: the value "was previously justified by an ORACLE CAPTURE
… which is exactly backwards under the prime rule". It had since been pinned to
`DS:0x0174` by a test — better, still a transcription.

What makes it worth its own entry is that THE PORT ALREADY READ THIS STRING.
`bloodprg::list_widget_cancel_label()` reads `DS:0x0174` and `main.rs` appends it to
the OPTION-menu labels. So the port held a literal copy of a string it was
simultaneously reading correctly a few hundred lines away. Now both paths read it,
via #524's `ds_text`, which cost one line in `UI_STRING_OFFSETS`.

Also deleted: `pub const OPTION_BOX: [&str; 1] = [OPTION_BOX_LABEL]` — a
one-element array wrapping the literal — and with it the assertion
`OPTION_BOX[0] == OPTION_BOX_LABEL`. The surrounding comment already identified that
as a tautology ("both sides are the same transcription", #370) and it survived
anyway, because deleting an assertion feels like weakening a test. It was comparing
a copy to itself.

The DS offset earns a citation of its own, and it is a load I had already decoded:
`mov si,0x174` @`0x85B3` — the list widget's shared trailing row (#492), where it is
named `SHIP_3D_TARGET_EXTRA_LABEL_OFFSET`. ONE string, ONE load site, TWO port names
in two files. Both are now cross-referenced, so a reader in either file learns the
other exists rather than discovering a second copy later.

`check_ui_literals.py` now reports 39 display literals present in the image with
ONE unpinned, down from the six this pass started with.

622 tests, 0 failures.

## #527 — the EMS signature is the game's own bytes, and a checker that skipped its own line

`check_ui_literals.py` reported one unpinned display literal left in the image:
`b"EMMXXXX0"` in `recomp/runtime.rs`. It is not a display string — it is the EMS
DRIVER SIGNATURE an `int 67h` handler must expose at `seg:0x0A`, written by the
emulated BIOS so the game's EMS detection succeeds. That is `recomp/runtime.rs`
doing its job (#480), so the literal is legitimate.

Legitimate is not the same as unchecked. The GAME CARRIES ITS OWN COPY at file
`0x997` and compares against it, so the right pin is the game's bytes:
`ems_signature_is_the_one_the_game_checks_for` asserts the emulator's signature
equals what the executable holds. Previously the emulator exposed a signature
remembered from the DOS convention, and a typo would have shown up as "the game
does not detect EMS" — a symptom several layers from its cause.

THE CHECKER THEN STILL REPORTED IT, and the reason is a bug in the checker. Its
owner search walks BACKWARDS from `line - 1` looking for the declaration a literal
belongs to. A literal declared INLINE with its constant —

    const EMS_DRIVER_SIGNATURE: &[u8; 8] = b"EMMXXXX0";

— has its owner on its OWN line, so starting one line earlier walked straight past
it and found whatever declaration came before. The literal read as unowned, and the
pinning test that names `EMS_DRIVER_SIGNATURE` could never be matched to it. Fixed
by starting the walk at `line`.

That bug had been silently weakening the tool for every inline-declared literal, not
just this one — a checker reporting a genuinely-pinned value teaches its reader to
skip its output, which is the exact failure #466 introduced the pin logic to avoid.

`check_ui_literals.py`: 39 display literals in the image, **0 unpinned** (was 1).

623 tests, 0 failures.

## #528 — a `.DIC` is a word list, so half the literal warnings were coincidence

With the in-image literals at zero (#527), the checker's remaining output was 42
unpinned matches in shipped data. Sampling them showed two unrelated kinds reported
identically:

```text
  src/engine.rs:4360: 'FRONT' in SCRIPT1.DIC        <- a port-side view label
  src/engine.rs:3281: 'IZWALITO' in SCRIPT2.DIC     <- a character name
```

`'FRONT'` is a match arm producing a caption for the port's own debug view; it
"appears in SCRIPT1.DIC" because **a `.DIC` IS THE GAME'S WORD DICTIONARY** — the
table subtitles are assembled from — and therefore contains most ordinary English
words. `CLICK`, `QUICK`, `RIGHT`, `COURSE`, `REACH`, `WAITING` are all flagged for
the same non-reason. That is the identical failure mode as a four-letter string
matching any binary, which the tool ALREADY guards against with its
`MIN_ATTRIBUTABLE` rule; the dictionary case had simply not been noticed.

Split them: matches in RECORD files (`.DES`, `.DEB`, `READ.ME`) name specific
things and are real evidence; `.DIC` matches are advisory and now listed only under
`--dict`. **20 real candidates, down from 42.**

This is the third time in three entries that a checker's own noise was the problem
(#526's tautological assertion, #527's owner walk, now this). A tool that reports
mostly-noise is not neutral — it trains its reader to skip the real finding sitting
in the middle of it, which is what #466 introduced pinning to prevent.

WHAT THE 20 ACTUALLY ARE, recorded rather than fixed here: character and location
names in `DESCRIPT.DES` (`Izwalito`, `Beauregard`, `Sinox`, `PTERRA`, `USINE`),
mostly in `src/extract/`. Those are content-bearing literals under the prime rule
and the next task in this line. `PHONE_CONTACTS` is among the flagged and is NOT
new work — it is a documented APPROX row (#440) with its own matrix entry, waiting
on the runtime slot list.

623 tests, 0 failures.

## #529 — a 35-name allow-list that was redundant twice over

`CHAR_CONTEXTS` listed 35 character record names with a background HNM each, and 19
of those entries carried `background_hnm: None` — no information at all, except
that the renderer SKIPPED any name it could not find:

```rust
let Some(context) = lookup_character_context(&scene.record_name) else {
    return Ok(false);   // skip the character entirely
};
```

So the table doubled as an allow-list, and I nearly deleted the `None` rows as
inert before reading that line. They were not inert; deleting them would have
silently dropped 19 characters from the export, which is exactly the class of
change that looks like tidying and is a regression.

It was redundant TWICE, and both proofs were needed before touching it:

1. `DescriptDb::character_scenes_for_snd` ALREADY filters
   `kind == RecordKind::Character`, so every scene reaching the renderer is a
   character record. The allow-list can never reject anything.
2. The table's 35 names are EXACTLY the 35 `RecordKind::Character` records in
   DESCRIPT.DES — measured, not assumed: 35 = 35, with every name present.

With both established, the allow-list is derivable and the names are a copy of
enumerable data — the defect `CLAUDE.md` names outright. Replaced by
`CHARACTER_BACKGROUNDS`, the 16 pairs that carry actual information, and the
renderer now treats "no background" as a standalone talking head rather than a
reason to skip.

`check_ui_literals.py`: unpinned RECORD-file literals **20 -> 5**.

SEPARATE FINDING, recorded not fixed: `src/extract/` is a module of the BINARY, not
the library, so its **100 tests never run** under `cargo test --release --lib` — the
command `CLAUDE.md` names as the one that must stay green. They pass under
`--bins`, and I ran both here, but a routine that omits them lets a hundred tests
rot unnoticed.

623 lib tests + 100 bin tests, 0 failures.

## #530 — a hundred tests that were never being run

#529 noticed in passing that `src/extract/` belongs to the BINARY crate, not the
library. `CLAUDE.md` names `cargo test --release --lib` as the command that must
stay green, and `--lib` does not compile the binary — so the export/QA pipeline's
**100 tests had been invisible to the project's own definition of green.**

They pass. That is the good outcome and also the reason it went unnoticed: nothing
ever failed, because nothing ever ran. A hundred assertions covering sprite blits,
scene-band fills, character scenes and the render recompilation were sitting in the
tree accruing the appearance of coverage.

The command is now `cargo test --release --lib --bins`, with the reason recorded
beside it in `CLAUDE.md` so it does not get "simplified" back. 623 lib + 100 bin =
**723 tests**, all green.

Worth being precise about what this was NOT: not a bug, not a regression, nothing
broken. It is a MEASUREMENT error — the number I have been reporting after every
entry ("623 tests, 0 failures") was describing a smaller suite than the repository
contains. Every such report in this session was accurate about what it ran and
understated what exists.

`re/tools/reverify_lifts.sh` keeps its narrower `--lib recomp`; that one is a
deliberate targeted run, not a definition of green.

723 tests, 0 failures.

## #531 — an ORACLE-CAPTURED menu, admitted in its own doc comment

`BOB_TOPICS` carried its provenance in its doc:

```rust
/// Bob Morlock's concept-menu topics — ORACLE-CAPTURED (cryobox_enter vs_007).
pub const BOB_TOPICS: [&'static str; 8] = ["BYE_BYE", "BLACK_HOLE", ...];
```

`CLAUDE.md` forbids exactly this — "Never derive geometry, colors, labels, MENUS,
flows … from an oracle capture and wire it into the port" — and names conversation
menus as the worked example: they must come from the `0xA6` line records'
`0xFFFF`-separated word lists, executed by the VM.

THAT PATH ALREADY EXISTED. `main.rs` fills `engine.bob_topics` from
`start_actor_presentation`'s collected menu — the bytecode's own words. The capture
was only a fallback for when the VM had not supplied them.

Which is the worse half of the defect. A fallback that produces PLAUSIBLE TEXT when
the real path fails does not degrade visibly — it renders a correct-looking menu
built from a screenshot, and the failure it is covering never surfaces. Ten lines
below, the same function already states the opposite rule for dialogue: "NO
FALLBACK LINE. Every word Bob speaks…". The lines followed the rule; the menu
beside them did not.

Deleted. `bob_topics` is empty until the VM supplies it, and an empty menu draws
nothing — which is the honest rendering of "the VM did not run", and the state a
failing VM path should produce.

ALL 723 TESTS STILL PASS, and that is itself the evidence: nothing asserted the
fallback's contents, so it was never load-bearing for anything but concealment.

Remaining flagged literals, judged not fixed: `'Commander Blood'` @`blood.rs:718`
is the X11 `WM_NAME` — the port's own window title, matching READ.ME by nature
rather than by copying. `PTERRA`/`USINE` @`extract/mod.rs:290` are DESCRIPT
LOCATION names hardcoded in the export pipeline's nav-view demo, and are the next
one of these to convert.

723 tests, 0 failures.

## #532 — the QA demo's destination names, and where a port-side choice legitimately stays

The export pipeline's nav-view render used

```rust
let dest_names = ["PTERRA", "USINE", "MAGNUS", "EKATOMB"];
```

— four DESCRIPT LOCATION names spelled into port source. The comment above them
even said "data-driven layout from the location names", which was true of the
LAYOUT (widths measured with `game_font_advance`) and false of the NAMES.

Now read from `DESCRIPT.DES`, filtering `kind == 1` (Location). Note the two
`DescriptDb` types: `commander_blood_tools::descript` exposes a `RecordKind` enum,
while `extract::descript` keeps the raw byte — the same value, two representations,
and the compiler caught the mix-up rather than silently comparing wrong.

WHAT STAYS A PORT CHOICE, deliberately: `.take(4)`. WHICH locations exist is the
game's data and is now read; HOW MANY to draw in a QA image is a rendering decision
for a diagnostic that is not a game surface. Reading the names while keeping the
count is the honest split, and the comment says so — the alternative, rendering
every location, would make the demo worse without making it more faithful.

If DESCRIPT.DES cannot be found the demo SKIPS rather than substituting anything —
the #531 rule applied one entry later: no plausible stand-in for missing data.

**Content literals in shipped-data files: 42 -> 2.** Both survivors are judged, not
overlooked: `MIGRAX` belongs to `PHONE_CONTACTS`, a documented APPROX row (#440)
waiting on the runtime slot list, and `'Commander Blood'` is the X11 `WM_NAME` —
the port's own window title, which matches READ.ME by nature rather than by copying.

723 tests, 0 failures.

## #533 — seven offsets already decoded, sitting uncited in another file

`bloodprg.rs` holds the port's DS-offset constants, and seven of them name cells
this session had already decoded — in `ship3d.rs`, in `vm.rs`, in the fix log —
without the citation ever reaching the file that declares them:

```text
  0x279B  nav-choice hold timer     `mov word [0x279b],0x5a` @0x86BB   (#491)
  0x253F  nav-choice target y       `mov word [0x253f],ax`   @0x86D1   (#491)
  0x0ADA  interpolation duration    two writers, one reader            (#494/#495)
  0x0174  list widget's extra row   `mov si,0x174`           @0x85B3   (#492)
  0x0ADC  preserve-widths flag      `mov byte [0xadc],1`     @0x86D4   (#491)
  0x0B3B  presentation hold timer   `mov word [0xb3b],0`     @0xB644   (#484)
  0x04DA  the VM code segment       validated against 4 handlers       (#517)
```

`0x0ADA` is the one worth reading twice: TWO WRITERS, ONE READER. The nav choice
sets it to 10 (`@0x86E4`) and the navigation box to 6 (`@0xB3CD`), and
`ship_3d_interpolation_gate` divides by whatever is there (`@0x1E63`). A port
treating it as a per-widget constant instead of a shared cell would animate both
boxes at whichever speed it picked.

`VM_CODE_SEGMENT` now records that it was VALIDATED rather than assumed — the
dispatch table's near offsets resolve against it onto four independently decoded
handlers, and `vm_dispatch.py` re-checks that on every run (#518).

A NOTE ON SPELLING, left alone: five of these were written in DECIMAL (`10139`,
`2778`, `2780`, `10931`, `2875`) among hex neighbours, which is why they read as
values rather than addresses and stayed uncited longest. I have not renumbered
them — a cosmetic edit to a constant is a chance to fat-finger one — but each doc
now leads with the hex, so the next reader sees `0x279B` before the decimal.

That is the shape of this entry generally: nothing here was undiscovered, and all
of it was unrecorded WHERE IT WOULD BE READ. A decode that lives only in the fix
log is one refactor from being deleted as unsourced (#491 made that point; this is
seven more instances of it).

723 tests, 0 failures.

## #534 — the constants I "discovered" in #494 were already declared in bloodprg.rs

#494 solved for the nav-choice dispatch table's segment by scanning 4096
paragraph-aligned bases, and reported it as a decode. `bloodprg.rs` already held:

```rust
pub const NAV_CODE_SEGMENT: u16 = 0x071e;
pub const NAV_CHOICE_SUBDISPATCH_TABLE_FILE_OFFSET: usize = 0x008709;
pub const NAV_CHOICE_SUBDISPATCH_ENTRY_COUNT: usize = 5;
```

The same segment, the same table address, the same count — UNCITED, which is why
`audit_settle.py` had them in the UNVERIFIED queue and why I never saw them while
working in `ship3d.rs`. The relationship is the right one in the end: the file
asserted the numbers, #494 proved them. But the search was avoidable, and the cost
of a decode living in the fix log rather than beside the constant it explains is
now measured in duplicated work rather than hypothesised (#491, #533).

The pair also turns out to be a PAIR. `bloodprg.rs` declares a second table —
`NAV_ACTOR_SUBDISPATCH` at `0x7EB4` with 6 entries — and it is the same mechanism:

```text
  0x7E09  call word ptr cs:[bx+0x6d4]      actor:  6 entries at cs:0x06D4
  0x8700  call word ptr cs:[bx+0xf29]      choice: 5 entries at cs:0x0F29
```

Both tables sit immediately before their own handlers. The actor table's second
entry resolves to `0x7EC0` = `0x7EB4 + 12`, the byte right after its twelve bytes —
so each table's length is bounded by where its code begins, which is what makes
"6 entries" and "5 entries" checkable rather than asserted.

`NAV_CODE_SEGMENT` is now confirmed three independent ways: the constraint solve
(#494), the direct `lcall 0x71e,0xc48` (#495), and both tables resolving onto real
routines within it.

723 tests, 0 failures.

## #535 — a tool for the #533/#534 pattern: decoded here, uncited there

#533 and #534 were the same failure twice: a constant sitting UNVERIFIED in one
file while the decode explaining it lived in the fix log or in another file's doc.
#534 cost a 4096-base constraint solve to re-derive a segment `bloodprg.rs` had
already declared. That is worth automating rather than noticing a third time.

`tools/check_decoded_but_uncited.py` takes every UNVERIFIED constant, reads its
declared value out of the source line, and looks for that value in
`docs/audit-fixes.md` NEXT TO AN ADDRESS. A hit means the work is done and only the
citation is missing — the cheapest rows in the ledger.

**12 leads out of 108 uncited constants**, and the tool is explicit that they are
LEADS. Its own output proves why: `ALIEN_TRANSFORM_NEUTRAL = 0x8000` matched a fix
entry about `test word es:[di+2],0x8000` — the TEXT line-record flag, a completely
different cell that shares a value. Taking that would have been #501 exactly. So
would `VIEWPORT_W = 320` matching a `0x140` mask and `TEX_W = 256` matching a line
about `DS:0x2567`. Roughly half the leads are noise, which is the honest yield for
a value-match heuristic and why the tool prints the warning rather than a count.

Four genuine ones closed here:

```text
  0x6ADE  save flags source   `mov dx,0x6ade` @0x1C6D, then int 21h
  0x6CDE  save state source   `mov dx,0x6cde` @0x1C72 + `mov cx,0x60` -> 96 bytes
  0x6780  load profile dest   the cell #516 cited; 0xFFFF is EMPTY, not a profile
  0x2AB3  widget width scratch  the cell #339 spent an entry proving is NOT a row list
```

`0x6CDE` is the nicest of them: the cell AND its length come from the same three
instructions, so `STATE_SOURCE_DS` and the 96-byte size cannot drift apart without
one of them disagreeing with `0x1C72`.

723 tests, 0 failures.

## #536 — the bold console font, and a guard that reads doc arithmetic literally

Three `font.rs` constants, from one nine-byte stretch that decodes the whole
lookup:

```text
  0x3684  mov bx, 0x70fa      the glyph MAP
  0x3687  xlatb               AL = [bx + AL]  -- a 256-byte TRANSLATE table
  0x3689  or al, al / js      a NEGATIVE entry means "no glyph"
  0x368D  mov bp, 0x71aa      the glyph BITMAPS
  0x3691  shl ax, 3           index * 8
  0x3694  add bp, ax
```

`xlatb` is what makes the map's shape unambiguous: it is indexed by the character
code directly, so the table is 256 bytes and dense, not a list to search. And
`ADVANCE = 8` is not a measured glyph width — it is `shl ax,3`, the scale factor
between glyph index and bitmap offset, which is the same fact the main font's row
table already implies (one byte per row, eight rows).

THE OFFSET GUARD CAUGHT MY WORDING. I wrote the file offsets as arithmetic:

    /// `DS:0x70FA` (= file `0xD420 + 0x70FA`), loaded ...

and `documented_ds_and_file_offsets_agree` reported
`doc says DS:0x70fa and file 0x0d420`. It reads the first file offset in the doc
and checks it against the DS offset — so an EXPRESSION reads as a claim that the
file offset IS `0xD420`. The guard was right and my doc was wrong in exactly the
way it exists to catch: a reader skimming would also have taken `0xD420` as the
answer. Replaced with the resolved values (`0x1451A`, `0x145CA`).

Worth noting I chained the settle onto the same command as the test run, so the
rows were settled while the suite was failing — the #423/#505 mistake a third time.
The docs were fixed and the suite is green before this entry was written, but the
ordering was luck rather than discipline.

723 tests, 0 failures.

## #537 — settling now REFUSES on a red tree, because discipline failed three times

`audit_settle.py` writes a claim into the ledger: this row was checked against the
binary. Three times I made that claim while the suite was FAILING — #423, #505, and
#536 — twice by chaining the settle onto the same shell command as the test run and
reading the output afterwards. Every time the fix was small and the ordering was
luck. Three occurrences is not a lapse, it is a process that does not work.

So the tool enforces it. `audit_settle.py` now runs
`cargo test --release --lib --bins --quiet` before writing anything and refuses if
it fails, printing the failing test:

```text
  REFUSING TO SETTLE: the test suite is not green.
    font::tests::game_font_row_table_is_one_byte_per_row --- FAILED
  Fix the tree first. `--no-verify` is for reverting to UNVERIFIED only.
```

`--no-verify` exists for exactly one case: putting a MIS-SETTLED row back to
`UNVERIFIED`, which has to work when something is broken — that is the undo path
the tool's own docstring calls out, and gating it behind a green suite would make
the tool refuse to fix its own mistakes.

I VERIFIED THE GUARD FIRES, rather than trusting that it would. A guard that never
triggers is indistinguishable from no guard — #527's checker had been silently
failing to flag inline-declared literals for exactly that reason. Injected a failing
assertion into `font.rs`, confirmed the refusal above, restored the file, confirmed
723 tests green again.

The cost is ~5 seconds per settle. The thing it prevents is a ledger row that says
"verified" about a tree where nothing was.

723 tests, 0 failures.

## #538 — the same routine is ported TWICE, in two files, and they agree

`bridge.rs`'s golden console menu and `ship3d.rs`'s `hit_test_ship_3d_nav_choice`
are two independent ports of ONE routine, `0x8614..0x868D`. Eight constants are
duplicated between them:

```text
  bridge.rs                        ship3d.rs
  MENU_FRAME_MIN       0x28        NAV_CHOICE_MIN_GATE   40
  MENU_FRAME_MAX       0x3C        MAX_GATE              60
  MENU_REST_FRAME      45          AXIS_BIAS             45
  MENU_RIGHT_AT_REST   0xE8+0x37   RIGHT_BASE            287
  MENU_WIDTH           0x6E        X_WIDTH               110
  MENU_TOP_AT_REST     0x48        Y_BASE                72
  MENU_ROW_PITCH_AT_REST 0x12      ROW_HEIGHT_BASE       18
  MENU_ROW_COUNT       5           COUNT                 5
```

Note `MENU_RIGHT_AT_REST` is written `0xE8 + 0x37` — this file kept the value in
its COMPUTED form, which is the thing #491 spent a search rediscovering after
concluding 287 might be fabricated because no immediate exists. The answer was in
`bridge.rs` the whole time, spelled the way the instructions spell it.

THEY AGREE, and I checked rather than assumed (#486a's lesson): this file's
`0x48 + |d| * 1.25` is `ship3d`'s `72 + |d| + (|d| >> 2)`, and its
`0x12 - |d| / 8` is `18 - ((|d| >> 2) >> 1)`. Two people-hours apart, two spellings,
same arithmetic. That is a genuinely good outcome — and exactly the configuration
#486a found broken in `manu3`, where two models of one method disagreed by a frame
for as long as both existed.

`check_duplicate_rules.py` could not have caught this pair: it keys on the ledger's
`origin` column, and `bridge.rs`'s constants were UNCITED, so they had no address to
cluster on. Now that they cite `0x8614`..`0x868D`, the checker can see them — the
uncited state was hiding a duplicate from the tool built to find duplicates.

The cross-reference is recorded on both sides so a change to either is known to
require the other.

723 tests, 0 failures.

## #539 — the third cross-file duplicate: one flag word, two vocabularies

`entity.rs`'s `flag` module and `ship3d.rs`'s sprite-slot constants describe THE
SAME WORD — the `+0x00` of the 32-byte record at `DS:0x6212`. Every routine in the
family opens identically (`shl ax,5 / mov bx,0x6212 / add bx,ax`, at `0x41D1`,
`0x420D`, `0x428C`), which is what #503 and #505 decoded from the ship-3D side.

```text
  entity::flag::ACTIVE 0x80  =  ship3d::SHIP_3D_OBJECT_VISIBLE_FLAG
  entity::flag::STATE0 0x01  =  ship3d::SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
  entity::flag::STATE1 0x02  =  ship3d::SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
```

THE NAMES DISAGREE ABOUT MEANING while agreeing on every value: bit 7 is "active"
here and "visible" there; bit 0 is "state 0" here and "active" there. Neither is
wrong for its own subsystem — one file models the object lifecycle, the other the
sprite pipeline — but a reader of either would reasonably conclude the other bit
set belongs to a different record. Both sides now say otherwise.

Three constants gained more than an address:

* `ACTIVE` has NO immediate anywhere. It is read by sign (`or al,al / jns`
  @`0x41DE`) or as the pair `test al,0x81` @`0x421D`/`0x42DD`. #511 recorded
  bit-7-by-sign-test as this binary's habit; here it is again, and the empty scan
  is confirmation rather than absence.
* `TOGGLE5`/`TOGGLE6` are `xor al,0x20` @`0x427E` and `xor al,0x40` @`0x429D` —
  genuine XORs. The state bits beside them are set and cleared outright, so the
  name "toggle" is a real distinction in the instructions, not a loose word.
* `STATE1` is set by the same `and al,0xfe / or al,2` handoff #505 found: clearing
  `STATE0` and setting `STATE1` is ONE transition, and a port that does either
  alone has invented a state the game has no name for.

`SOURCE` (`0x04`) stays UNVERIFIED: no site in the `0x41C0..0x4320` family, and the
doc says it is carried "during populate", which is elsewhere. Not inferred from its
neighbours (#509).

723 tests, 0 failures.

## #540 — the duplicate finder could not see constants, which is all three duplicates

Three cross-file duplicates in three entries — `bridge.rs`/`ship3d.rs` porting
`0x8614..0x868D` twice (#538), `entity.rs`/`ship3d.rs` naming the `0x6212` flag word
twice (#539), `engine.rs`/`ship3d.rs` naming `DS:0x0174` twice (#526) — and
`check_duplicate_rules.py` found none of them. Its row filter is:

```python
if r["kind"] == "fn" and not r["file"].startswith(...)
```

CONSTANTS WERE EXCLUDED, and every one of the three was a set of constants. The
tool built to find duplicated decodes was structurally blind to the form the
duplicates actually took. I found all three by hand, one per entry, which is the
failure mode the tool exists to remove.

Now clustered too. Constants sharing an address is NORMAL — a routine's whole seed
set cites the same entry — so the signal is a shared address ACROSS FILES: two
subsystems that have independently named one thing. **50 such clusters**, and they
are a REFERENCE rather than a defect list; the tool's other two sections flag risk,
this one maps shared vocabulary.

The map immediately pays for itself on cells this session already found the hard
way:

```text
  0x0ADA  bloodprg SHIP_3D_INTERPOLATION_DURATION_DS_OFFSET
          ship3d   NAVIGATION_INTERPOLATION_DURATION, NAV_CHOICE_INTERPOLATION_DURATION
```

— the two-writers-one-reader cell #533 wrote a paragraph about, now visible as one
line. Likewise `0x0AC6`'s layout centre named in three files and `0x0A32`'s
presentation mode in three.

The general lesson is about tool scope, not this tool: a checker's FILTER is a
claim about where the problem can occur, and that claim silently expires when the
codebase's shape changes. This one was written when duplicated decodes meant
duplicated FUNCTIONS. The port has since moved most of its decoded knowledge into
constants, and the filter stayed.

723 tests, 0 failures.

## #541 — SUBTITLE_Y is a CELL with two values, and the port models one

The subtitle reveal takes its position from cells, not immediates:

```text
  0x94E6  mov bx, word ptr [0x5e5c]      x
  0x94EA  mov dx, word ptr [0x5e5e]      y
  0x94EE  lcall 0x299, 0x6a0             the reveal draw (#490)
```

`[0x5e5e]` is written `8` @`0x7C60` — which is the port's `SUBTITLE_Y` — and also
written **`1`** @`0x7A08`, on a different path. So the port has hardcoded one of two
values the game uses for the same cell. A subtitle drawn after the other path sits
seven rows higher, and nothing in `render.rs` says the value can change.

Recorded rather than fixed: which path leads to which write is not yet decoded, and
guessing would be worse than the current state — the 8 is a real value from a real
instruction, just not the only one. The doc now says so at the constant.

`[0x5e5c]` (SUBTITLE_X) has NO immediate write at all; it is loaded from a register
elsewhere, so its `10` stays UNVERIFIED rather than being attributed to a neighbour
(#509's discipline).

The three font-metric 8s are the same 8 seen three ways, and now say so: the row
table is `glyphs * height` bytes with one byte per row (which is what makes the
glyph eight pixels WIDE), and the bold console font scales its index by `shl ax,3`
@`0x3691` for the identical reason (#536). Three constants, one fact.

A DECODE NOTE, because it cost a step: disassembling from `0x7C5A` — two bytes
before the address the byte-scan reported — produced `add dh,al / push es / jmp`,
plausible garbage. Decoding from `0x7C60` itself gives the real
`mov word ptr [0x5e5e],8`. x86 self-synchronises, so an arbitrary start is a
phantom generator; #515 made this point about the GS prefix and it applies to any
address not already known to be an instruction boundary.

723 tests, 0 failures.

## #542 — two reserved palette slots, four port writers, no name saying so

`RETICLE = 0xFE` and `BAR = 0xFD` in the cyberspace HUD are the SAME two indices the
subtitle reveal draws through (`SUBTITLE_COLOR_REVEALED`/`_REVEAL_EDGE`, #541). That
much is decoded: `0xC0..0xFF` are RESERVED entries the game fills at RUNTIME, which
is why a scene's LBM/HNM palette leaves them `[0,0,0]` and why the port has a helper
to install subtitle colours before drawing.

What is NOT decoded is this screen's use of them for a reticle and a progress bar,
or the RGB it writes. Those are port choices, and they are now labelled as such
rather than sitting in the UNVERIFIED queue looking like undone decode work.

THE COUPLING IS THE POINT. Four port sites write these two indices with different
colours — the cyber HUD here, the nav object-marker path, and the subtitle helper.
Whichever runs last wins, and NO CONSTANT'S NAME SUGGESTS IT SHARES A SLOT:
`RETICLE`, `BAR`, `SUBTITLE_COLOR_REVEALED` and a bare `scene_palette[0xFD] = ...`
read as four independent decisions. If a screen ever draws a HUD and a subtitle
together, one silently recolours the other, and the symptom (a wrong-coloured
subtitle) is three files from its cause.

Classified INFRA, not ASM: `RETICLE`/`BAR` have no binary counterpart as a
reticle-and-bar pairing. Same for `NAV_DEST_Y/PITCH/W`, whose own doc already says
they are "AN INVENTED LAYOUT WEARING A GAME PROVENANCE" kept as a port affordance —
leaving those UNVERIFIED reads as "not yet decoded" when the truth is "deliberately
not from the game", and the ledger should not blur the two.

723 tests, 0 failures.

## #543 — a guard for #542's coupling, which first failed to see #542

`tools/check_palette_slot_writers.py` lists every reserved slot (`0xC0..0xFF`) the
port writes and flags those written with MORE THAN ONE colour — the coupling #542
found by hand, where four sites share two slots under names that give no hint of it.

THE FIRST VERSION MISSED THE CASE THAT MOTIVATED IT. Its regex matched literal
indices only, and #542's sites are `scene_palette[RETICLE as usize]` — named
constants. So it reported one conflict and silently passed the two it was written
for. That is #527's owner-walk and #540's `kind == "fn"` filter a third time: a
guard whose matcher is narrower than the problem reports clean and teaches its
reader nothing. Fixed by resolving named indices against the file's own `const`
definitions.

Test sections are skipped, for the #528 reason: `render.rs`'s subtitle test installs
`[1,2,3]`/`[4,5,6]` into the reserved pair precisely to prove the renderer reads
them, and reporting that beside real writers is noise that trains the reader past
the real ones.

Two genuine conflicts remain, and both are RECORDED not fixed, because "fine if the
screens never coexist" is a claim I cannot make from the writes alone:

```text
  0xE0   engine.rs:1828  [255,255,255]   the Bob concept menu's engaged row
         engine.rs:5402  [0,0,0]         the presentation box-open animation
  0xFD   engine.rs:3002  [120,220,245]   the cyberspace progress bar
         engine.rs:4385  [255,80,80]     the nav object marker
```

`0xFE` turns out NOT to conflict — five writers, all `[245,245,160]` — which is the
tool earning its keep in the other direction: same-colour repeats are the subtitle
helper doing its job, and only distinct colours are reported.

723 tests, 0 failures.

## #544 — the save-slot geometry: one constant from code, two from the file itself

Three constants, and they need DIFFERENT KINDS of evidence — which is why they had
sat together uncited under one comment.

`SLOT_NAME_LEN = 16` is in the CODE: the rename copies the name with
`mov cx,4 / rep movsd` @`0x1BB7`-`0x1BBD` into `DS:0x273B`. Four dwords, sixteen
bytes, settled ASM.

`SLOT_RECORD_LEN = 32` and `SLOT_COUNT = 10` are NOT in the code — I searched the
save-UI region `0x1900..0x1E00` for a `mov cx,10`, an `add si,32` and a `320`, and
found none of them. They are in the DATA: the shipped `blood.sav` is **exactly 320
bytes**, and 320 = 10 * 32. Settled DATA, not ASM, because that is what the evidence
is.

The distinction matters more than the two rows. A constant with no instruction
behind it is not automatically unverified — the game's own shipped file can be the
authority, and here it is the ONLY authority. Filing these ASM would have implied an
address that does not exist; leaving them UNVERIFIED would have implied nothing had
been checked. `audit_settle.py`'s `DATA` status exists for exactly this, and the
`REFUSED ... (ASM needs a cited address)` on the first attempt is the tool holding
the line correctly.

`shipped_slot_directory_is_ten_thirty_two_byte_records` now pins it, and asserts
the parser accepts the shipped image — so if the format ever disagrees with the
constants, the file itself fails the test rather than the port silently mis-parsing.

724 tests, 0 failures.

## #545 — four constants feeding a lifted routine: three are the game's, one is the harness's

`build_console_bank_remap_table` runs `recomp::auto::func_242d` — a bit-exact lift —
and sets up four constants for it. They looked like one group and are two:

```text
  PAL_DS    0x5251  `mov si,0x5251` @0x243E, inside the routine
  TABLE_DS  0x6011  `mov bx,0x6011` @0x9625, at the CALL SITE
  BANK_BASE 0x00E0  `mov ax,0xe0`   @0x9622, the same setup
  GS        0x2600  NOTHING -- a scratch segment the port picked
```

The call site is `lcall 0x1ce:0x14d` @`0x9628`, which resolves to `0x242D`; the two
instructions before it are the routine's arguments. So `TABLE_DS` and `BANK_BASE`
are the game's values, just not from the routine that consumes them — they are its
CALLER's, which is where a search inside `0x242D` would never have found them.
`refs_in_routine.py` on `0x242D` reports `0x5251` and nothing else, and I nearly
concluded the other two were port inventions on that basis.

`GS = 0x2600` IS a port invention, and correctly so. The routine does
`mov ax,gs / mov ds,ax` @`0x2436`, so it works against whatever segment it is given;
the port hands it an arbitrary base to lay the emulator's buffers out in. The game's
own data segment is `0x0CE2` (`bloodprg::DATA_SEGMENT`), which is NOT what this is —
settled INFRA, with the difference spelled out so nobody "corrects" it to the real
segment and moves the buffers out from under the lift.

The general shape, worth naming because it will recur with every lifted routine: a
harness that runs decoded code needs BOTH the game's arguments and its own
scaffolding, and the two sit adjacent in the source looking identical. Only the
citation distinguishes them.

724 tests, 0 failures.

## #546 — the alien engine's four values, and 0x8000 turning out to be 1.0

Four `croolis.rs` constants, all from `croolis.xdb`:

```text
  0x0385/0x038D/0x0395  mov dword [si+0x12/0x22/0x32], 0x8000   TRANSFORM_NEUTRAL
  0x099F/0x09A2         mov di,0x4000 / mov bp,0x7fff           WRAP + MASK
  0x16C9/0x16CE/0x16D3  state=1, timer=0x32, accumulator=0      TIMER_RELOAD
  0x16DC                add bx,0xfa                             ANIM_STEP
```

`ALIEN_TRANSFORM_NEUTRAL = 0x8000` IS 1.0. `ship3d::SHIP_3D_MATRIX_FIXED_SHIFT` is
15 (`sar e_x,0xf`, #499), so `0x8000 >> 15 == 1` — the alien initializer's "neutral
transform" and the projection's fixed-point format are the same fact reached from
two subsystems that share no code. #499 predicted this ("which is exactly why
`ALIEN_TRANSFORM_NEUTRAL` is `0x8000`"); the citation now closes the loop from the
other end.

Two are worth reading as SEQUENCES rather than values. The wrap loads its mask and
its half-extent two instructions apart (`0x99F`/`0x9A2`), so `0x4000` and `0x7FFF`
are one setup, not two constants that happen to be related. And the timer reload sits
between the state flag and the accumulator clear (`0x16C9`..`0x16D3`): choosing a
state, arming the timer and zeroing the accumulator is ONE sequence, so a port doing
any of the three alone produces a state the game never holds.

`ALIEN_ANIM_STEP`'s citation restates what #400 established and the constant never
carried: the `add bx,0xfa` @`0x16DC` advances the SHARED counter `cs:[0x16A2]`, not
a per-object field. That was the bug #401 fixed; the constant now says so where
someone editing it will read it.

724 tests, 0 failures.

## #547 — I labelled an address that was already labelled, and the checker caught it

`check_labels.py` reported `DUPLICATE ADDRESS 0x08709 has 2 rows:
nav_choice_subdispatch_table (line 213), nav_choice_handler_table (line 789)`. The
second is mine, added in #494 when I "discovered" the nav-choice dispatch table.

`nav_choice_subdispatch_table` was already there, and its comment already read "THE
PER-ROW HANDLER TABLE for the bridge console / nav choice. Five u16 near offsets
0f33, …" — the same five entries I re-derived. This is #534 for the second time:
#534 found `bloodprg.rs` already DECLARED the constants I solved for; this finds
`labels.csv` already NAMED the table I labelled. Both times the existing knowledge
was one grep away and I did not run it.

The rows are merged, my segment-solve comment folded into the existing label, and
the duplicate is gone (505 code labels, 0 duplicates).

WHAT ACTUALLY GOES WRONG when this is not caught: two names for one address means
a later reader greps for one, finds it, and never learns the other exists — which
is exactly the state that produced #538's bridge/ship3d duplicate and #539's
entity/ship3d duplicate. The duplicate-label check is cheap and I should be running
`check_labels.py` BEFORE adding a label, not after a session's worth of them.

Worth noting the checkers as a set are now catching my mistakes faster than I make
them: #536's offset guard caught a doc expression, #543's palette guard needed
fixing before it could see its own case, #537's settle guard now blocks a red-tree
claim, and this one caught a duplicate label. That is the intended direction — the
tools are the memory, and I keep proving why they need to be.

724 tests, 0 failures.

## #548 — a fourth cross-file duplicate, and three constants that are honestly the port's

`extract/mod.rs`'s nine uncited constants split three ways, and the split is the
useful part — they had been sitting in one block looking alike.

**Two are the game's, duplicated from `vm.rs`.** `SCRIPT_OBJECT_TALK_FIELD = 0x3A`
and `SCRIPT_OBJECT_LOCATION_FIELD = 24` are the TALK and LOCATION columns of the
field-offset matrix at `DS:0x6D60` — the same two values `vm::LOCATION_FIELD` holds
and `vm::field_offset`'s doc spells out ("character location = obj+0x18 / talk =
obj+0x3A"). That is the FOURTH cross-file duplicate this session, after
bridge/ship3d (#538), entity/ship3d (#539) and engine/ship3d (#526). The pattern is
now unmistakable: the port's subsystems were decoded separately and each named what
it needed, so shared game facts have two or three homes and no cross-links until
someone cites them.

**Two are the game's, already decoded elsewhere.** `VIEWPORT_W`/`VIEWPORT_H` are the
320 built from two shifts (#502) and the 200 restored as a clip bottom (#495) —
`engine::ENGINE_SCREEN_*` all over again (#525), in a third file.

**Four are honestly the port's**, settled INFRA rather than left in a queue that
implies undone decoding: `ISO_URL` (where to download the disc image),
`OUTPUT_SCALE`/`OUTPUT_W`/`OUTPUT_H` (the export's 3x upscale, a rendering choice
for MP4s the game never produced).

`HNM_FPS = 15` is NOT settled. It could be the port's encoder choice or the HNM
format's own rate, and I have not read the format's header to find out. Guessing
either way would put a wrong label on a row that currently says "unknown", which is
at least true.

724 tests, 0 failures.

## #549 — chasing one uncited constant found a 1.67x timing bug in the exports

#548 left `HNM_FPS = 15` UNVERIFIED rather than guess whether it was the game's rate
or the port's. Reading the format settles it: `HnmFile::open` parses header size, a
palette block and frame offsets — THE HNM HEADER CARRIES NO FRAME RATE. So 15 is the
export's encoding choice, and INFRA is the honest label.

Then its use sites showed it doing a second job it has no business doing:

```rust
vm::reveal_complete_hold_ticks(step) as f64 / HNM_FPS as f64
```

That converts a GAME TICK COUNT to seconds by dividing by the VIDEO frame rate. The
game's tick is `8 / (1193182 / 5958)` = 39.948 ms — **~25 Hz**, decoded in #477 —
so every completed-line subtitle hold in the exported videos was stretched by
25.03/15 = **1.67x**.

TWO TESTS AGREED WITH THE BUG, which is why it survived. One asserted the formula
including `/ HNM_FPS`; the other hardcoded `0.633333` with a comment stating the
reasoning outright: "8 ticks at 15fps = 0.5333s". Both were computed the same wrong
way as the code, so they could only ever confirm it. The corrected value, 0.419576,
is checkable by hand: 3 chars / 30 cps = 0.1s reveal, plus 8 x 39.948 ms = 0.319576s
hold.

`GAME_TICK_SECS` now lives in `lib.rs` with its derivation, and its doc says the
thing that was missing: any duration the game expresses in TICKS converts through
it, never through a frame rate. `HNM_FPS`'s doc says the converse. A test pins the
two apart and names the 1.67 factor, so re-conflating them fails.

THE SETTLE GUARD (#537) CAUGHT THIS MID-FIX. I ran the settle while the bin tests
were red from my own change; it refused and printed the failing test. That is the
third time this session a guard has caught me and the first time it prevented a
false claim rather than merely reporting one.

725 tests, 0 failures.

## #550 — the same shape as #549, found by looking for it rather than tripping over it

#549 was a tick count divided by the wrong rate, hidden because the tests were
computed the same way as the code. Having found one, I looked for others: every
tick-to-seconds conversion in the port.

`extract/` converts `SubtitleCue.tick` as `tick / 10.0` at FIVE sites and writes it
back as `duration * 10.0` at one. The unit is nowhere recorded — `SubtitleCue` is
`{ tick, text }` — the divisor appears in no fix entry and no matrix row, and no
binary routine consuming the field has been identified.

I have NOT shown it is wrong, and want to be exact about that: DESCRIPT is authored
data and tenths of a second is an entirely plausible authoring unit. What I have
shown is that nothing establishes it, and that the read/write symmetry makes it
UNFALSIFIABLE BY THE PORT'S OWN TESTS — the same structural blindness that let
#549 stand.

Recorded as an UNVERIFIED row in `docs/port-validation.md` with what would settle
it: find the routine that consumes a cue tick and compare it against the timer it
drives. `GAME_TICK_SECS` is ~25 Hz, so if cues share that clock the divisor is out
by 2.5x; if they are a separate authoring clock, 10 is right and the row closes.

The general point: a self-consistent conversion is a claim no test can refute, so
it needs an EXTERNAL anchor or an explicit "unverified" label. Two of these existed
in one file and one of them was a real bug.

725 tests, 0 failures.

## #551 — a negative result worth writing down

Tried to settle #550 by finding the game's consumer of DESCRIPT command `0x0D`
(the subtitle cue: command byte, `u16` tick, NUL string).

`cmp al,0x0d` occurs EXACTLY ONCE in the entire image, at `0x1DED`, and it is not
the cue handler — the two instructions after it are `cmp al,0x30 / jb` and
`cmp al,0x39`, an ASCII digit range, so that `0x0D` is a CARRIAGE RETURN in the
save-name editor. No `sub al,0x0d` exists either.

So the DESCRIPT interpreter does not compare command bytes directly; it dispatches
them some other way — a jump table, or code not yet located. Given #502's standing
conclusion (this compiler prefers shifts and tables to immediates) that is the
expected shape, and the `0x0D` scan was always the cheap thing to try first.

RECORDED IN THE MATRIX ROW rather than left as a dead end in my head. The row now
says what was searched and what the single hit turned out to be, so the next
attempt starts from "find the command dispatch" instead of re-running a scan that
returns one misleading match. A negative result costs the same to write down as a
positive one and saves exactly as much time.

Status unchanged: #550 remains UNVERIFIED, honestly.

725 tests, 0 failures.
