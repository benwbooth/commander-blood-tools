# Modern Rust port

This workspace ports recovered game behavior into typed Rust while retaining the
original data files as authoritative resources. `ported.tsv` is deliberately
conservative: a routine is listed only after its Rust behavior has direct binary
oracle coverage and documentation.

## Memory model

The runtime uses a flat address space. Skeleton parents, vertices, faces,
animation targets, and projection aliases are Rust indices, ranges, slices, and
owned collections. It has no near, far, or huge pointers, segment registers,
selectors, paragraph arithmetic, or emulated DOS memory.

XDB relocation deltas are interpreted only by the format decoder to locate file
sections. The decoder immediately produces typed owned data; native addresses do
not survive into runtime state.

## MANU3 status

Ten of the twelve MANU3 routines have direct Rust coverage. The current path
decodes all authored skeleton, animation, geometry, texture, trigonometry, and
raster-reciprocal data; runs recovered fixed-point animation and projection;
preserves face selection and activation decisions; and submits indexed textured
triangles to a depth-tested wgpu pipeline.

The DOS software scanline pool, linked raster records, Mode X plane writes, and
framebuffer segments are implementation details replaced by wgpu. The reserved
hand palette bank at indices 202 through 251 is decoded from `BLOODPRG.EXE` and
merged without overwriting scene-owned lower palette entries.

The two remaining routines are native-only adapters. Offsets `0x0121` and
`0x06f6` install relocated segments and must be classified as eliminated
flat-memory setup rather than recreated. `eliminated.tsv` records those mappings
separately from translated routines, and the coverage gate verifies that all
twelve recovered MANU3 entries are accounted for exactly once.

Run the current interactive path with original assets:

```sh
nix develop -c cargo run -p commander-blood-game -- \
  --asset output/_tmp_dat/fd/pterra1f.lbm \
  --manu3 output/_tmp_dat/manu3.xdb
```

The renderer never warps or moves the host pointer. SDL mouse events are mapped
into the letterboxed 320-by-200 game coordinate system.
