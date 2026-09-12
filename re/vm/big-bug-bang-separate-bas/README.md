# Big Bug Bang standalone BAS artifact

The sequel disc contains one `SCRIPT2.BAS` image, but the 17 active BBB script
profiles do not own BAS companions. Native ownership analysis establishes that
the image is unreachable from legitimate shipped state. Its encoded dictionary
offsets also do not resolve against `SCRIPT2.DIC` or any other shipped BBB
dictionary; the native reader would use them as direct offsets without a
relocation transform.

`script2.bas.blood` therefore ports only the independently recoverable BAS byte
framing and control structure. All word operands remain numeric. It does not
attach the artifact to profile 2, substitute a dictionary, or make the stream
runtime-reachable.

Regenerate and byte-verify the source:

```sh
cargo run -p commander-blood-script-compiler \
  --example recover_bbb_separate_bas -- \
  output/big-bug-bang/imported-assets/resources/SCRIPT2.BAS \
  re/vm/big-bug-bang-separate-bas/script2.bas.blood
```

The shipped image is 19,933 bytes with SHA-256
`3e2b4a6d7c26aca6be2f88b3b539972b655ab1423907bbf75fb0931717bb5314`.
The recovered source contains 1,102 typed statements: 593 text records, 122
menus, 122 linked selector nodes, and 265 other BAS instructions.
