# DOS Integration Gates

These programs link recovered natural-C routines into real-mode DOS
executables and run them under DOSBox-X. They are the bridge between the
per-routine Unicorn oracles and replacing routines in the original XDB
execution environment.

Run the current MANU3 gate from the repository root:

```sh
NIXPKGS_ALLOW_UNFREE=1 nix shell --impure \
  nixpkgs#open-watcom-bin nixpkgs#dosbox-x -c \
  python3 re/tools/manu3_dos_integration.py
```

`manu3_renderer_empty.c` calls the recovered MANU3 `0x0700` renderer with one
fully clipped face. This is a complete original control-flow path: the sorter
rejects the face, initializes the 200-record raster free list, scans all 320
empty buckets, and returns. The test compares every byte of the raster arena
and verifies that the geometry arena was not modified.

Both `0x0700` and its real `0x0D7D` callee are linked from the recovered source.
The clipped input intentionally does not call `0x0D7D`; executing active faces
requires binding the candidate's named C globals to the original DS/ES/FS
arena layout and is the next integration boundary.
