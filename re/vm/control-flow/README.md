# COD control-flow recovery

These files are generated from the five shipped `SCRIPTn.COD` images and their
`SCRIPTn.DEB` symbol tables. They are static evidence for lifting BloodScript
above the byte-exact token layer; they are not a replacement VM runtime.

Generate them with:

```sh
cargo run --bin cbvm -- analyze-control-flow \
  accuracy/cblood_install/cblood re/vm/control-flow
```

The analyzer starts at byte zero and every recovered kind-2 DEB procedure. It
tracks the native query bit and exact guard-target stack, verifies every target
is a decoded instruction boundary, and accounts for every self-modifying byte
write. Every one of the 480 kind-2 DEB procedures begins with an `A9` activation
header: 420 are initially enabled and 60 are initially disabled. All 413 shipped
`AB` writes store zero or one at exactly `named procedure start + 1`, the `A9`
flag byte; 149 enable a procedure and 264 disable one. They patch 270 distinct
procedures, of which 269 can take both enter and skip states. BloodScript
therefore renders the headers as `activation enabled|disabled until target` and
the writes as `procedure.enabled = true|false`.

Across 7,010 basic blocks, 7,005 are reachable and no branch-capable
instruction is missing its native guard target. The five unreachable blocks
are retained in the JSON. Each follows an `A9` whose flag is permanently zero,
so the native direct-skip path bypasses its body. Four are in `SCRIPT3` at
`0x193A`, `0x47A5`, `0x6926`, and `0x7B84`; one is in `SCRIPT5` at `0x0174`.
No POKE enables them, and the only POKE touching this set writes zero to the
already-zero flag at `SCRIPT3:0x7B81`.

## Edge kinds

| Kind | Meaning |
| --- | --- |
| `fallthrough` | The instruction completes at the next token. |
| `block_enter` | An enabled `A9` enters its body and installs the root guard. |
| `block_skip` | A disabled `A9` jumps directly to its encoded target. |
| `guard_enter` | `A0` pushes a nested failure target. |
| `guard_exit` | `A1` leaves query mode and pops a non-root target. |
| `guard_pass` | A conditional VM operation succeeds. |
| `guard_failure` | A conditional operation consumes the current guard and branches. |
| `jump` | `A4` transfers directly to its encoded target. |
| `text_continue` | Text processing continues at the following token. |
| `text_skip` | Text flags skip a recovered number of following tokens. |
| `frame_resume` | Text schedules a future-frame resume; this is not an immediate transfer. |

`from_block` and `to_block` are byte offsets of block leaders.
`from_instruction` and `to_instruction` preserve the exact transfer endpoints;
structured-region proofs use these exact addresses rather than treating a block
leader as the destination. Procedure names include their starting offset so
duplicate DEB names stay unambiguous.
