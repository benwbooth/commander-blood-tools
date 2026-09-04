# C -> Rust fidelity inventory and test evidence

Snapshot: 2026-09-04. This is a source-grounded inventory/evidence report,
not a semantic audit and not a fidelity certification.

## Inventory

Counts below were recomputed with a tabular parser (`csv.DictReader`, tab
delimiter), not by running the gate.

| Source or ledger | Rows | Component breakdown / meaning |
|---|---:|---|
| `re/source/bloodprg/candidates/manifest.tsv` | 338 | recovered BLOODPRG routines |
| `re/source/xdb/candidates/manifest.tsv` | 183 | recovered XDB routines: MANU3 12, AMER 55, CROOLIS 54, SCRUT 62 |
| `re/rust-port/ported.tsv` | 474 | BLOODPRG 302; MANU3 10; AMER 52; CROOLIS 51; SCRUT 59 |
| `re/rust-port/eliminated.tsv` | 47 | BLOODPRG 36; MANU3 2; AMER 3; CROOLIS 3; SCRUT 3 |
| `re/rust-port/partial.tsv` | 0 | no rows currently marked partial |
| `re/rust-port/production-campaign-covered.tsv` | 300 | expected production-campaign witness rows: BLOODPRG 224; MANU3 8; AMER 24; CROOLIS 22; SCRUT 22 |
| `re/rust-port/production-routing-dispositions.tsv` | 35 | reviewed non-routing rows: unreachable 12; semantically inlined 11; modernized replacement 10; external entry unused 1; ABI adapter only 1 |

The recovered partition is 521 rows (338 + 183), matching 474 ported + 47
eliminated + 0 partial. This establishes ledger accounting only. The 300
production rows are a subset of the 474 ported rows; 174 ported rows are not
listed in that campaign witness set. Neither fact is a correctness percentage.

The `evidence` suffixes parse as positive numeric row annotations: 5,934
across ported rows and 483 across eliminated rows. These are summed annotations
and are not a deduplicated count of executed tests or differential cases.

## Test evidence classes

The workspace has three members (`Cargo.toml:5-11`), with the modern game in
`crates/commander-blood-game`; `commander-blood-script-compiler` explicitly
sets library tests and doctests off (`crates/commander-blood-script-compiler/Cargo.toml`).
The game package's source and integration-test trees contain 889 `#[test]`
attributes across 201 files in this snapshot. This is a textual inventory, not
the number of tests executed by any particular command. The examples below
describe evidence classes rather than listing every test.

* **Ledger and binary-vector checks.** `tests/port_coverage.rs:38-195`
  parses the recovered manifests and the three routine ledgers, checks unique
  keys, Rust symbol/doc presence, evidence-file existence, and fixed counts.
  Its additional checks cover 71 shared aliases, 25 shared UI writers, and 7
  PRNG callers (`:214-344`). It does not execute the C program or compare
  C/Rust behavior. `src/native/bloodprg/game_lifecycle_tests.rs:340-385`
  loads 14 cases from `re/tools/oracle_vectors/func_0eb0_natural.json` and
  compares a Rust lifecycle test host with recorded call/state expectations.
  That is binary-derived fixture evidence, not a live differential run.

* **Rust-only integration and data-contract checks.** `tests/campaign_oracle.rs`
  exercises the Rust runtime against imported original assets, decoded script
  manifests, saves, contact choices, and authored presentation/audio contracts
  (`:481-689`). `tests/startup_phone_runtime.rs` launches only the modern
  `commander-blood` binary (`:1845-1912`) for authored scenarios, including
  startup, phone, bridge, save/load, Pterra, and alien paths. These tests can
  prove Rust-side behavior against source/data expectations; they do not by
  themselves run the original executable.

* **Actual original/Rust differential.**
  `re/tools/run_startup_phone_temporal_oracle.sh:65-108` builds `runtime_boot`
  and `commander-blood`, runs `runtime_boot` with the original
  `BLOODPRG.EXE`, runs the Rust binary on the same scenario, then calls
  `compare_port_runtime_traces.py`. The comparator requires action-aligned
  game-frame clocks and checks exact semantic paths (`:326-447`); the script
  requires at least 47 compared records and zero bridge-frame tolerance.
  This is a live original-binary-to-Rust differential lane currently
  represented in source. No such run was performed for this report.

* **Production campaign instrumentation and routing.** The gate runs the
  game package's startup campaign with LLVM instrumentation, merges profiles,
  and correlates them with the 300-row expected set
  (`re/tools/run_rust_fidelity_gate.sh:81-100`). The production audit is a
  ledger-to-LLVM function correlation, not semantic equivalence. The routing
  audit builds the debug binary, examines demangled retained symbols, checks
  non-test source callers, and validates the 35 disposition rows
  (`re/tools/audit_rust_port_routing.py:307-384`).

## Mapping boundary and limitations

`src/native/mod.rs:1-7` labels `native` as direct translations of recovered
native routines. The routine ledger maps native implementation files plus a
small amount of host glue; it does not map high-level adapter composition.
All 38 files under `crates/commander-blood-game/src/runtime/` have zero
ported/eliminated/partial routine-ledger rows. This includes lifecycle,
startup, script backend, services, bridge, ship, presentation, video, input,
save/load, and state adapters (`src/runtime.rs:3-40`). Top-level entry/render
glue is likewise outside the ported-routine set: `src/main.rs`, `src/app.rs`,
`src/render.rs`, `src/bridge_render.rs`, and `src/alien_render.rs` have no
ported rows; some have eliminated adapter rows.

The 474-row ledger proves documented source/routine/evidence associations as
checked by the source tests. It does not prove every Rust call path, complete
campaign reachability, pixel parity, audio parity, timing parity, or semantic
equivalence of the adapter layer. The production witness set and LLVM results
must not be read as proof of complete C-to-Rust coverage. `docs/port-validation.md`
contains retired-root-engine material and is intentionally not used as current
proof here.

Verification for this report was limited to source inspection and structured
TSV count checks. No tests, fidelity gate, expensive campaign, commit, or push
was run.
