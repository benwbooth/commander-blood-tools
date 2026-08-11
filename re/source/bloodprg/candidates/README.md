# BLOODPRG Natural C Candidates

This directory contains human-written natural C candidates for recovered
`BLOODPRG.EXE` routines. These files are not an emulation layer and are not
accepted replacement source yet.

Candidate rules:

- One recovered assembly routine maps to one C function.
- Use named globals, data declarations, and calls.
- Do not model CPU registers, flags, segments, or byte-addressed machine memory
  as C objects.
- Keep register/carry-return routines pending until their ABI can be expressed
  naturally or isolated behind a small assembly boundary.
- Promote a candidate only after a candidate DOS compiler emits a close assembly
  shape for the routine.

Run the current candidate sanity check with:

```sh
python3 re/tools/source_candidates.py --check
```
