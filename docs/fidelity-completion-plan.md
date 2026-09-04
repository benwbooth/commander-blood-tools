# Completing the modern Rust fidelity audit

Status: initial verification review, 2026-09-04. This is a completion plan, not
a claim that the entire C-to-Rust implementation has been reviewed or verified.
Use `fidelity-inventory-2026-09-04.md` for the current source/test inventory.

## Evidence policy

The recovered C describes intended behavior; original `BLOODPRG.EXE` and XDB
execution adjudicate discrepancies. The C is itself a reconstruction, not an
infallible oracle. Reuse the existing binary-vector and differential runners.
Cross-check their timing, graphics, and sound adapters against independent
DOSBox captures before treating host-dependent behavior as authoritative.
The reference emulator remains test tooling, never the modern game's runtime.

Keep these claims separate: accounted for, translated, independently reviewed,
executed in production, branch tested, and compared with reference behavior.
Neither a function inventory nor a successful Rust-only campaign proves parity.

Every behavior fix needs:

1. A specific C/assembly or authored-script contract and its relevant inputs.
2. A failing comparison at the first divergent boundary, with artifacts retained.
3. A narrowly scoped fix to the actual production caller and state owner.
4. The original regression plus adjacent transition/cancellation tests passing.
5. A recorded scope and remaining gaps. Do not rewrite expected output from Rust
   merely to make a test pass; correct an expectation only with reference evidence.

## Findings from the initial review

- `compare_port_runtime_traces.py` previously accepted mismatched game-frame
  deltas even with `--require-game-frame-clock`, silently skipping temporal state.
  Empty selections and duplicate action indices could also pass. The strengthened
  comparator rejects these; strict temporal mode requires its formerly optional
  PRNG/name-effect fields. Diagnostic non-strict comparison remains available.
- The startup temporal gate now requires 47 comparisons, zero bridge-frame
  tolerance, and retains both traces, process logs, source revision/dirty diff,
  and executable/input fingerprints. A passing result concerns that short startup
  interval only. Screen-hash diversity is not reference image equality.
- `runtime/platform.rs` uses fixed timer ticks and bypasses pacing/interpolation
  for scenarios; interactive play uses monotonic elapsed time and render-only
  refreshes. Existing scripted passes do not validate those live paths.
- `startup_phone_runtime.rs::run_production_scenario_internal` still waits on an
  unbounded child process and deletes temporary traces on return. Failure
  retention and per-scenario watchdogs must extend beyond the startup oracle.
- The inherited Pterra/alien changes remain uncommitted and unverified. They
  must be judged against source-backed phase/ownership contracts, not accepted
  because a new assertion describes the implementation's own state.

## Execution order

| Stage | Work | Exit evidence |
| --- | --- | --- |
| 1. Reproducible baseline | Build the current dirty tree; preserve fingerprints; run fail-closed prerequisites, routine vectors, then production campaigns with timeouts and failure retention | Exact failing tests and first-divergence artifacts, no silent skips |
| 2. Runtime contracts | Audit native-to-runtime adapters in lifecycle, services, script backend, presentation, navigation, audio, and input | Every shared C writer/read-after-callback mapped to its Rust owner; ordered side effects compared across callbacks |
| 3. Presentation ownership | Intro, phone on/off, cancel, Bob, Pterra arrival/departure, scene replacement | Matching state transitions, selected resources, frame/time, RGBA composition, cursor visibility and sound events through teardown |
| 4. Timing/input | Drive the production scheduler with an injectable deterministic clock and input source, not scenario-specific timing rules | Same tick/event code tested for elapsed time, slow frames, interpolation, focus/capture, held/released buttons and cancellation |
| 5. Whole-game differential routes | Extend existing dual-run scenarios across all profiles, contacts, choices, planets, saves and alien loops | Matched checkpoints plus authored branch/transition coverage; continuous start-to-ending routes |
| 6. Acceptance | Repeat on release build and live SDL/wgpu path, with original and enhanced presentation modes distinguished | No unexplained divergence in the declared matrix; all remaining exclusions named and justified |

For the adapter audit, examine import/copy-back on early returns and errors,
shared aliases split across structs, signed/wrapping arithmetic, callback order,
resource lifetime, and queue-drain vs playback-complete vs retained-frame states.
Review the callees' effects, not just their names or the caller's local variables.

Use authentic save checkpoints and replay from a known state to shorten cases.
State pokes are targeted component tests, not proof that normal play reaches the
same state. Reuse existing scenarios and ledgers rather than building another port.

## Differential observables

Compare logical VM records/variables/history, active profile/line, PRNG, input
latches, timers, presentation owner and queue, resource identity/frame, subtitle
bytes/reveal positions, and ordered audio start/stop/loop/sample events. Preserve
full snapshots around the first mismatch; hashes alone are insufficient to debug.
Report explicitly which fields were compared, missing, skipped, or unaligned.

Capture final composed RGBA and GPU readback at matched presentation boundaries.
Software-buffer hashes, draw-call counts, and Rust flags do not prove what was
visible. Capture mixed audio output as well as the requested sound events.
Add controlled test perturbations to prove that wrong timing, colors, missing
hand geometry, stale frames, and missing sound events cause their gates to fail.

## Modernization boundary

Flat memory, SDL3, wgpu, higher resolution, and render-only interpolation remain
requirements. Old indexed assets may use palette data during decoding; runtime
artwork/video layers should own resolved RGBA and explicit fades, not a mutable
global VGA palette that recolors unrelated content. Preserve the visible effects
and timing of original palette operations without preserving DOS hardware state.

Exact logical decisions and legacy-resolution decoded frames can be compared
byte-for-byte. Enhanced GPU rasterization/interpolation cannot be certified by
those hashes: separately validate pose, geometry, material, occlusion, timing,
and matched-resolution output with documented tolerances. Higher visual refresh
must not increase simulation speed or advance scripts/audio more frequently.

There is no justified single accuracy percentage today. Completion means the
declared behavioral matrix is independently verified, not that every mapped
routine has been called once or that no player has reported another bug yet.

## Initial verification record

On 2026-09-04, the current dirty tree built successfully and the strengthened
startup oracle passed all 47 selected action checkpoints, with 47 matching
game-frame deltas, no missing temporal fields, and zero bridge-frame tolerance.
It performed **zero indexed RGB comparisons**: this result does not certify the
rendered hand, bridge, video colors, or audio. Artifacts are retained locally at
`output/fidelity/startup-temporal.0LWEyk/`, including `report.json` and hashes.
Injecting one extra Rust frame into the retained trace made the comparator fail
at action 10, rather than silently skipping that timing discrepancy.

The comparator tests (23), routing-audit tests (8), production-coverage-audit
tests (6), startup-trace-verifier tests (3), and Rust port-ledger tests (4) passed.
The full asset-backed campaign gate was not rerun for this initial review;
inherited Pterra and alien scenario changes still need investigation. No game
behavior fix is claimed by these verification-tool changes.
