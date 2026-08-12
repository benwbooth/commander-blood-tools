# BLOODPRG Natural C Candidates

This directory contains human-written natural C candidates for recovered
`BLOODPRG.EXE` routines. These files are not an emulation layer and are not
accepted replacement source yet.

Candidate rules:

- One recovered assembly routine maps to one C function.
- Use named globals, data declarations, and calls.
- Do not model CPU registers, flags, segments, or byte-addressed machine memory
  as C objects.
- DOS compiler intrinsics such as `inportb`, `outportb`, and `int86` are allowed
  when the recovered routine directly performs that hardware or BIOS operation.
  They express the target platform rather than simulate it.
- Keep register/carry-return routines pending until their ABI can be expressed
  naturally or isolated behind a small assembly boundary.
- Promote a candidate only after a candidate DOS compiler emits a close assembly
  shape for the routine.

Run the current candidate sanity check with:

```sh
python3 re/tools/source_candidates.py --check
```

Re-run the crafted direct-binary vectors for behaviorally verified candidates
with:

```sh
nix develop -c python3 re/tools/natural_candidate_oracle.py --check
```
