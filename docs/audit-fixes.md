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

## Fixed + committed (19) — assembly-cited, regression-tested, oracle-verified where visual

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

## Verified FALSE POSITIVE for the PORT — finding correct for the assembly, wrong for the port's model (3)

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
  wrong. What's missing is the RUNTIME update of those bits on navigation/transfer, plus
  `owner_object_offset` is a nearest-below approximation, not the `0x6034` threshold
  lookup. Reverted to the safe working model `active_actor==Some(off)` because a stale
  active-bit gate could misfire mid-game; re-applying the assembly gate needs the runtime
  active-bit lifecycle modeled and verified in live play first. NOTE: this same VAR-initial
  active-bit availability makes the C4-WRITE and `0xC1` guards partially feasible (the
  guards read the same bits) — but with the same staleness caveat.

## Remaining (23) — each to be oracle-re-verified or carefully implemented

- **Geometry, needs oracle re-check** (likely more false positives like the tall-mode):
  choice-box x-band / min-width floor, in-window (kind-3) label centering, subtitle
  multi-line pitch (capture shows pitch-10 for the credit beat — conflicts the finding),
  palette 128–191 bank (runtime working buffer, not a static swap), nav-projection matrix
  term negation order.
- **Higher-risk VM guards** (need exact-operand disassembly; wrong ⇒ breaks dialogue):
  C4 mode-0 write guards, `0xC1` line-record state (unhandled live), C4 mode-0 unconditional.
- **Subtle / low visible impact:** B8 arche-reference cleanup, `0xA8` fin-flag +
  presentation-request side effects, `0x6946` query nuance, bridge ring-cursor 8px snap,
  hand seek-distance memo, thin-font unmapped-char skip, chatter re-roll determinism,
  A6 per-line C4 gate.
- **Rewrites:** nav destinations = flag-gated entity set (fabricated pyramid grid),
  world-destination from clicked row (not stale steering angle), A6 reveal-busy
  serialization handshake.
- **Infrastructure-blocked:** ending fires on all-visited (the `rec_103A` runtime write,
  documented separately in port-validation.md).
