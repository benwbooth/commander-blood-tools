# Commander Blood — faithful RE port (BLOODPRG.EXE → Rust)

## THE PRIME RULE: assembly is the source of truth; the oracle is VERIFICATION ONLY

**All information about how the game works MUST come from analyzing the assembly
code (and the game's own data files it defines the layout of). The oracle — the
interpreter running the real EXE, DOSBox captures, screenshots, scenario runs —
is ONLY for verifying that a decoded implementation matches. Never the reverse.**

Concretely:

- **Never** derive geometry, colors, labels, menus, flows, timings, or any other
  behavior from an oracle capture and wire it into the port. If a capture shows a
  surface the port lacks, the next step is to FIND THE CODE that produces it
  (disassemble, xref, trace the handler) and port THAT — then use the capture to
  verify the ported code's output.
- **Never** hardcode content that lives in the game's data (script bytecode,
  DIC words, DESCRIPT records, sprite/level tables). The port executes/parses
  the real data; content-bearing literals in Rust source are a defect.
  (Example of the right shape: conversation menus come from the 0xA6 line
  records' 0xFFFF-separated word lists, executed by the VM — not from lists
  transcribed off the screen.)
- A capture-measured constant is at best a TODO marker: it may stand in
  temporarily only if the row in docs/port-validation.md explicitly labels it
  APPROX with the binary routine that must replace it. Finding that routine is
  the actual task.
- Every mechanism claim in code comments must cite its binary address
  (routine/table offset, labels.csv name). "Oracle-verified" is a statement
  about TESTING, never about where the behavior came from.
- The dual-run scenario harness (VERIFYSCRIPT / verify_port /
  tools/verify_compare.py) exists to SCORE the port against the real game.
  Scenarios discovering an unknown surface produce RE TASKS (decode the code
  behind it), not port patches.

## Project conventions

- RE workspace: `re/` (REVERSE.md, labels.csv, dead_ends.md, tools/). DOS MZ
  80386 real-mode; file offsets `0xNNNNN` (image = file − 0x600), DS-relative
  `DS:0xNNNN` (DS base = file 0xD420), far calls store image-relative segments
  (runtime seg = stored + 0x1A2).
- Disassembly: `re/tools/dis.py <fileoff> [n]`; overlays:
  `re/tools/dis_xdb.py <xdb> <off> [n]`; xref/search/table tools per re/CLAUDE.md.
- The interpreter oracle: `runtime_boot` probes (savestate resume at
  accuracy/script2.state; VERIFYSCRIPT scenarios in accuracy/scenarios/).
  Python via `nix develop --command env PYTHONSAFEPATH=1 python3`.
- Validation ledger: docs/port-validation.md — every module row cites its
  binary evidence; APPROX/UNVERIFIED rows are the work queue.
- Tests: `nix develop --command cargo test --release --lib --bins` must stay green;
  every decoded behavior gets a regression test encoding the REAL behavior.
  `--bins` is not optional: `src/extract/` is a module of the BINARY, not the
  library, so ~100 tests (the whole export/QA pipeline) are invisible to `--lib`
  alone and were going unrun for as long as that was the documented command
  (audit-fixes #529/#530).
- Never declare the port finished; report status against the validation matrix.
- **KEEP GOING.** Reporting "N open, not finished" is a status line, NOT a stopping
  point. While any row of the matrix (or `docs/audit-fixes.md`) is open, keep
  working it autonomously — pick the next item, decode it, fix it, test it, commit.
  Do not hold for direction, do not ask which item to take, do not restate the
  tally instead of working. "This needs a new subsystem built" is a description of
  the next task, not a blocker: build it. The only legitimate pause is a genuinely
  destructive or irreversible action needing consent.
  - Corollary: if an item's blocker is real, that blocker becomes the task
    (build the runtime, wire the setter, stand up the pipeline) — record the
    decode in `re/labels.csv`/`re/dead_ends.md` and keep moving.
