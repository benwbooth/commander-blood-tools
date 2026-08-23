# Automated Whole-Program Parity Plan

## Objective

Replace manual crash-by-crash debugging with a fail-closed verification pipeline
for all 337 BLOODPRG routines, 183 XDB routines, 25 byte-exact VM resources, and
their cross-module state transitions.

Static analysis is necessary but cannot prove the full game by itself. The
pipeline combines original-assembly contracts, emitted 16-bit code checks,
differential routine execution, runtime data-structure invariants, and
headless original-versus-rebuilt scenarios. A source change is accepted only
when the original binary supplies the behavioral oracle.

## Current Baseline

- Every recovered C routine has a corresponding assembly routine.
- All 337 BLOODPRG and 183 XDB routines compile and link.
- Package gates prove 496 global placements, 3,252 BLOODPRG symbolic accesses,
  1,476 XDB symbolic accesses, and 2,898 explicit XDB far-segment uses.
- The XDB ABI gate covers 218 contracts and 58 callback targets.
- The 25 VM resources compile byte-for-byte.
- Existing routine vectors cover all BLOODPRG candidates and 104 XDB
  candidates; 79 XDB routines still rely on static control-flow review. The
  XDB oracle now records conditional-edge coverage: 84 direct routines are
  complete and 20 have exact reviewed edge debt that cannot change silently.

These checks prove storage ownership and selected ABI boundaries. They do not
prove all caller-to-callee pointer provenance, loop invariants, signed overflow,
side-effect order, or complete game-state transitions.

## Phase 1: Interprocedural Pointer And ABI Manifest

Generate one machine-readable row for every routine and every call edge:

- near, far, interrupt, or tail-transfer control flow;
- parameter order, 16-bit width, signedness, and pointer segment provenance;
- return registers and width;
- stack cleanup and preserved/clobbered registers;
- original assembly call-site evidence and emitted-listing evidence;
- indirect table slot and allowed target set.

Propagate `(owner segment, offset)` through arguments. Reject a near-pointer
callee when the original call installs a different DS/ES owner or the rebuilt
call discards a far-pointer segment. Reject unproved indirect targets.

The first audit found exactly this class at `006B4C -> 0060DD`: the prior C
discarded the VM record segment and read offsets through GAME_DATA. The fix now
passes two full far pointers, and the emitted ABI gate verifies both segment
words at the caller and callee.

## Phase 2: Typed Global And Alias Ownership

Extend the data-layout manifest with:

- byte extent and element count;
- signedness and scalar/struct/pointer type;
- near, far, GAME_DATA, FS_DATA, CODE_DATA, or stack ownership;
- one defining translation unit;
- explicit alias groups and overlapping byte ranges.

Reject silent first-declaration wins, undeclared overlap, incompatible aliases,
and declaration/definition disagreement after Watcom preprocessing.

## Phase 3: Algorithm-Parity Static Audit

Compare each original routine with its emitted 16-bit listing using normalized
control-flow and side-effect contracts:

- conditional branch polarity and signed/unsigned condition class;
- loop count source, zero-count behavior, back edges, and termination state;
- divide guards and quotient width;
- modulo-16/modulo-32 arithmetic and sign-extension points;
- ordered global and dynamic memory reads/writes around calls;
- call, callback, and tail-transfer order;
- byte/word/dword access footprints for hardware-visible memory.

Natural compiler transformations are recorded as fingerprint-bound reviewed
equivalences. Any changed fingerprint invalidates its review. No new waiver is
accepted without assembly and differential evidence.

## Phase 4: Differential Routine Sanitizer

For every routine, capture or synthesize a complete machine-state input and run
the original and rebuilt implementations from identical snapshots. Compare:

- registers, flags, stack delta, and segment registers;
- all written memory bytes and I/O operations;
- callback sequence and arguments;
- termination, instruction budget, and fault behavior.

Use boundary-value and randomized inputs, then minimize the first divergence.
Prioritize the 79 XDB routines without direct vectors and all routines reachable
from reported failures.

## Phase 5: XDB Data-Structure Invariants

Instrument snapshots without changing release behavior:

- MANU3 raster offsets must be aligned, in-pool, acyclic, and terminate within
  the 200-record pool;
- active boundaries must be monotonic with heights in `1..256`;
- texture and framebuffer addresses must remain in their declared ranges;
- starfield plane cursors must remain inside their 384-record partitions;
- collection counts that feed original `LOOP` instructions must have proved
  nonzero caller invariants.

On the first violation, save the complete guest and overlay state, identify the
earliest corrupting write, and replay that state through both original and
rebuilt XDB code. Do not add release guards until original behavior for the
invalid state is known.

## Phase 6: Autonomous State Exploration

Run isolated original and rebuilt DOS guests through:

- all five profiles;
- all 65 contact entry procedures through completion, not only first lines;
- authentic Pterra and every other world transition;
- every word-choice branch and post-conversation state;
- ship navigation, MANU3 rendering, save/load, audio bank changes, and exits.

Compare ordered semantic checkpoints, resource IDs, loaded handles, script
state, audio selections, presentation state, and fault bundles. Shared failures
remain inconclusive; original-pass/rebuilt-fail is a release blocker.

## Acceptance Gate

A substantial correction is complete only when:

1. Original assembly or runtime behavior proves the mismatch.
2. A minimized oracle fails before the correction and passes afterward.
3. The emitted ABI, segment, algorithm, and invariant audits pass.
4. The affected original-versus-rebuilt scenario matches.
5. The complete package and scenario matrix pass with no unresolved high-risk
   findings.

Agents are useful for parallel evidence collection and review. They are not the
oracle, and their suggested fixes are never accepted without these gates.
