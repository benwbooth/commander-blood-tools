# Big Bug Bang unified BloodScript profiles

These 17 `bloodscript 8` files are the canonical editable source for Big Bug
Bang's 68 active VM resources. Each profile derives its `SCRIPTn.COD`, `.DEB`,
`.DIC`, and `.VAR` images. The explicit `dialect big_bug_bang` declaration
selects the sequel instruction framing, eight-byte dictionary prefix, 74-byte
actor records, and 26-byte location records.

Big Bug Bang does not own a per-profile BAS resource. The disc's lone
`SCRIPT2.BAS` is preserved separately, but original-executable analysis found
no legitimate state path that can make its selector roots reachable. It is not
silently replaced with an empty program or attached to SCRIPT2's dictionary.

Regenerate these sources from extracted resources and verify recompilation:

```sh
cargo run -p commander-blood-script-compiler \
  --example recover_bbb_profiles -- \
  output/big-bug-bang/imported-assets/resources \
  re/vm/big-bug-bang-profiles
```

Compile one profile to an output directory:

```sh
cargo run --bin cbvm -- compile-profile \
  re/vm/big-bug-bang-profiles/script1.blood /tmp/bbb-script1
```

The authentic-corpus test compares all 637,922 emitted bytes:

```sh
cargo test -p commander-blood-script-compiler \
  --test sequel_source all_sequel_unified_profiles_rebuild_every_active_companion \
  -- --ignored
```

State declarations own all 184 fixed records in directory order. `opponent`
is the sequel actor word at byte 72, and `settler` is the location word at byte
24. The compiler derives `tblood`, globals, procedure addresses, directory
records, dictionary offsets, and reserved zero fields. Numeric presentation
selectors remain numeric because the sequel's presentation catalog is distinct
from Commander Blood's named HNM table.
