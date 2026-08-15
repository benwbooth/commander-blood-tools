# BAS control-flow recovery

These files are generated from each shipped `SCRIPTn.BAS` image together with
its `.VAR`, `.DIC`, and `.DEB` companions. They join the byte-exact BAS grammar
to the native object's conversation entrypoint and selector-list dispatcher.

Generate them with:

```sh
cargo run --bin cbvm -- analyze-bas-control-flow \
  accuracy/cblood_install/cblood re/vm/bas-control-flow
```

The entrypoint derivation is static. `vm_cod_scan` at executable file offset
`0x739B` reads the object's kind, resolves field selector 2 through
`vm_field_offset`, and adds that field's value to the BAS base. The shipped
conversation objects have kind `0x0002`, for which selector 2 resolves to
object offset `+0x1A`. `vm_control_flow` at `0x56FE` begins matching one byte
after that saved BAS offset. `value_scan_match` at `0x577A` reads a node as
`{selector:u16, next:u16}`, returns `node + 4` on a match, and follows `next`
directly on a mismatch.

Across the five profiles, 37 nonzero selector-2 object fields resolve one for
one to all 37 physical `AA AC <selector-node>` list roots. The analyzer rejects
missing, duplicate, interior, and non-token entrypoints. Following the links
owns all 321 selector nodes exactly once and accounts for all 284 nonzero
`next` fields. Every matched body begins with `MENU` and ends at the precise
`AC`, `AA`, or `FF` boundary used by the native executor.

The duplicate selector `0x0001` in SCRIPT3's Izwalito list is retained. It is
not an analyzer artifact: both nodes belong to the same object-owned chain and
are represented as distinct match tests at offsets `0x0397` and `0x03C9`.

## Edge kinds

| Kind | Meaning |
| --- | --- |
| `selector_match` | The selected dictionary value matches and execution enters the node body. |
| `selector_mismatch` | The value does not match and the scanner follows the node's nonzero `next` offset. |
| `selector_miss_exit` | The terminal node does not match and the scanner returns zero. |
| `body_yield` | A matched body reaches its `AC` or `AA` yield boundary. |
| `body_end` | A matched body reaches the final `FF` image terminator. |

Offsets in the JSON are zero-based BAS byte offsets. `prefix_yield_b` is the
object field's exact value; `root_node` is the selector node one byte later.
