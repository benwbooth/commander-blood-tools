# Code Reverse-Engineering Inventory

This inventory answers the practical scope question for the bit-exact
decompilation track: which shipped artifacts contain executable logic that must
be reverse engineered, and which artifacts are data/assets that still need
accurate parsers but are not themselves native code.

## Runtime Native Code

These artifacts can execute during a normal game session. They are in scope for
native reverse engineering if the goal is full game-accurate behavior.

| Artifact | Size | SHA-256 | Current role | RE requirement |
|---|---:|---|---|---|
| `re/bin/BLOOD.EXE` / `output/_tmp_iso/BLOOD.EXE` | 696 | `cecd2d07b576cedd460aeb7cfb6ea3e93fbf2cfd1f1890d5ce37c80c4d36c335` | Tiny MZ launcher for the main program | Needed for exact boot/distribution behavior; not expected to contain core game semantics. |
| `re/bin/BLOODPRG.EXE` / `output/_tmp_iso/BLOODPRG.EXE` | 86680 | `7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823` | Main DOS MZ executable: startup, resource manager, VM, presentation, rendering, sound host, input, game loop | Mandatory. This is the first native decompilation target. |
| `output/_tmp_dat/amer.xdb` | 266800 | `6fddeb5cc7c62fe5e638900746decc299f1637ea11612c0c51aced367dd12b31` | Native overlay module | Mandatory for full behavior. Treat as raw 16-bit/386 code+data overlay until proven otherwise. |
| `output/_tmp_dat/croolis.xdb` | 258832 | `13eba7c4e4f38662c44c849bd07be293c7583d2b2dc9ae273d7fe94746048c31` | Native overlay module; already partially decoded as interactive alien/3D object screen | Mandatory for full behavior. Treat as raw 16-bit/386 code+data overlay until proven otherwise. |
| `output/_tmp_dat/scrut.xdb` | 258080 | `8522e77aad6639cf9bb06f148048fed3c6bf1cfbe6459c9a0be7b987c3c3ac77` | Native overlay module | Mandatory for full behavior. Treat as raw 16-bit/386 code+data overlay until proven otherwise. |
| `output/_tmp_dat/manu3.xdb` | 62544 | `d0f64e99a646197906e273edfa0124172307a5cd766c88591c12ebd9ea556d31` | Native overlay module; different family from the alien/scrutinizer overlays; contains 3D/menu/manual math | Mandatory for full behavior. Treat as raw 16-bit/386 code+data overlay until proven otherwise. |
| `output/_tmp_dat/dnsdb.drv` | 2734 | `c3105741eefb0956654d42d41ce9766696c579e50bd1cc25ba47eeb903757e13` | Swappable native sound driver loaded COM-style at offset `0x100`; driver vectors are `E9 rel16` entries | Mandatory to model the original sound-driver ABI and timing; full native decompilation is needed if reproducing the driver binary or exact hardware behavior. |
| `output/_tmp_dat/nosound.drv` | 285 | `0c226e54cc265bbbe337cc69161645428503886c552d4d199920216601fcad85` | Minimal swappable native sound driver loaded through the same ABI | Mandatory to model the no-sound driver ABI; useful as the smaller reference driver. |

Short answer for native gameplay code: **`BLOODPRG.EXE` + four `*.xdb` overlays
+ two `*.drv` sound drivers**, with `BLOOD.EXE` added if launcher/boot-path
exactness matters.

## Runtime VM Code And Script Resources

The `SCRIPT1` through `SCRIPT5` bundles are not native x86 code, but they are
executable game logic for the custom VM. They must be decoded and replayed
faithfully for game-accurate cutscenes, dialogue routing, scene selection,
state transitions, and per-frame gameplay behavior.

Canonical ISO-root copies:

| Script | `BAS` | `COD` | `DEB` | `DIC` | `VAR` | RE requirement |
|---|---:|---:|---:|---:|---:|---|
| `SCRIPT1` | 32 | 3084 | 2740 | 2663 | 4666 | Decode as VM/source metadata bundle, not native code. |
| `SCRIPT2` | 22565 | 39042 | 6840 | 24772 | 4882 | Decode as VM/source metadata bundle, not native code. |
| `SCRIPT3` | 15242 | 34874 | 7060 | 21068 | 5198 | Decode as VM/source metadata bundle, not native code. |
| `SCRIPT4` | 10585 | 20824 | 4880 | 15558 | 5428 | Decode as VM/source metadata bundle, not native code. |
| `SCRIPT5` | 16312 | 20963 | 4880 | 18669 | 5008 | Decode as VM/source metadata bundle, not native code. |

The same script files also appear under `output/_tmp_iso/cblood/`; the duplicate
copies compare byte-identical to the ISO-root copies above. Treat the ISO-root
copies as the canonical inventory paths.

## Executables Outside The Game Runtime

These are MZ executables shipped with the disc, but they are setup/help tooling,
not known game-runtime modules. They are out of scope for game-accurate video
and gameplay behavior unless the goal expands to reproducing the whole disc
environment bit-for-bit.

| Artifact | Size | SHA-256 | Current role | RE requirement |
|---|---:|---|---|---|
| `output/_tmp_iso/INSTALL.EXE` | 64200 | `7f7f3e30a0a2cadd1b557461e2d71f2f3bcb27253679e0bfac8a1d45076a073c` | Installer; Detect It Easy reports Borland TLINK(5.0) | Not required for runtime game behavior. Useful only as a toolchain clue and if recreating installation behavior. |
| `output/_tmp_iso/HELP_4_U/CDTEST.EXE` | 17093 | `b01f00ad773af081cf158fc48d36d1c93f4d2add7244db6d5a66897236d8c637` | CD/helper utility | Not required for runtime game behavior. |
| `output/_tmp_iso/HELP_4_U/SNAP.EXE` | 26958 | `7d667d8d8aa0a6b5f881910a1dad08480174b61eb763639bf73acab563b2bb99` | Helper utility | Not required for runtime game behavior. |

## Data And Asset Formats

The remaining shipped resource families still matter for accuracy, but the
working assumption is that they are data formats rather than native code:

- `*.HNM`: video/animation streams.
- `*.VOC`, `*.SND`: audio payloads/banks.
- `*.LBM`, `*.SPR`, `*.EXT`, `*.FD`: images, sprites, level/world data, fonts,
  and related resource payloads.
- `DESCRIPT.DES`, `TB.BIG`, `BLOOD.DAT`, `README.TXT`, saves, and miscellaneous
  tables: resource databases or metadata.

These formats need exact parsing and renderer/player semantics, but they should
not be counted as native code decompilation targets unless a future binary
signature, loader path, or emulator trace proves that a specific payload is
executed.

## Current Scope Statement

For full game-accurate behavior, the RE list is:

1. Native code: `BLOODPRG.EXE`, `amer.xdb`, `croolis.xdb`, `scrut.xdb`,
   `manu3.xdb`, `dnsdb.drv`, `nosound.drv`, and optionally `BLOOD.EXE` for the
   launcher.
2. VM/script logic: `SCRIPT1` through `SCRIPT5` `BAS/COD/DEB/DIC/VAR` bundles.
3. Data formats: all referenced asset/resource formats, parsed as data rather
   than decompiled as native code.

So `BLOODPRG.EXE` plus `*.xdb` is close, but not complete. The `.drv` files are
native executable code, and the `SCRIPT*` bundles are the other major source of
runtime logic even though they are VM code rather than x86.
