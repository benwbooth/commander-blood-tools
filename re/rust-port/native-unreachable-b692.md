# BLOODPRG 0x00B692 Reachability

`0x00B692` is a complete 75-byte near routine ending at `0x00B6DC`. It starts
immediately after the far return from `alien_overlay_cycle`, so it is neither a
fallthrough block nor part of that coordinator.

The shipped `BLOODPRG.EXE` contains no native route to segment-relative entry
`0A9A:06F2`:

- `xref.py 0x0A9A:0x06F2` reports zero relocation-proven far calls or jumps.
- `xref.py --callers 0x0A9A:0x06F2` reports zero near calls.
- `xref.py --branches 0x0A9A:0x06F2 0x00B7B0` reports zero relative calls,
  jumps, conditions, loops, or short branches in the reviewed linked segment.
- `xref.py --imm16 0x06F2` reports zero raw offset occurrences anywhere in the
  executable, excluding a static near pointer or far-pointer offset.
- `indirect_dispatch_atlas.py` classifies all 48 indirect records into known
  internal tables, presentation callbacks, XMS, or sound-driver vectors and
  reports zero unknown records. None of its static tables contains this entry.

The routine is therefore recovered and oracle-tested in C and Rust, but it is
not inserted into the production runtime. Adding a call would create behavior
that the shipped executable cannot perform.
