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

## Known Contents By Artifact

This is the current high-confidence content map. It deliberately separates
confirmed structure from likely purpose where an overlay has not yet been decoded
as deeply as `croolis.xdb` or `manu3.xdb`.

| Artifact | What it contains |
|---|---|
| `BLOOD.EXE` | A tiny launcher MZ executable. It is part of the shipped boot path, but no recovered evidence says it contains gameplay systems. |
| `BLOODPRG.EXE` | The main game host: custom DOS startup, segmented 386 real-mode setup, EMS/XMS/CD-ROM/mouse/video probing, resource directory, script-profile loader, custom VM dispatch, presentation/cutscene handlers, dialogue font/tables, renderer/blitters, ship/bridge/3D systems, sound-bank host code, and far-pointer calls into the external sound driver. It has at least 308 internal code entries before trace-only input/presentation callbacks. |
| `amer.xdb` | One member of the alien-examination overlay cycle selected with `croolis.xdb` and `scrut.xdb`. It shares the alien overlay engine shape: self-relocating 386 overlay entry, 0x5E-byte object records, the shared alien animation PRNG, and the same loaded-overlay calling convention. Its individual data/behavior tables still need a deeper pass. |
| `croolis.xdb` | The best-decoded alien overlay so far. It contains a self-relocating entry stub, a full 256-color VGA DAC palette upload, mouse range/position setup, VGA plane clear, a null-terminated alien object list, vtable-dispatched object methods, shared PRNG/timer animation state, object position wrapping, proximity/visibility gates, camera accumulators, 3D projection/blit setup, and script-variable list tables. |
| `scrut.xdb` | Another alien overlay using the same shared engine/template. Known contents include the shared object/PRNG machinery, Scruter/examination-specific tables, a 111-entry exam record table with script-record result sinks, and variable-list tables matching the `croolis` family. |
| `manu3.xdb` | A different overlay family for the 3D pyramid/menu/manual/hand interface. Known contents include its far-call API, cursor-to-pose law, menu-item dispatch, tween/animation descriptor processing, 3D camera pan, trig tables, matrix/projection math, and data that feeds the pyramid/hand rendering path. Some live mesh/state regions are still partly runtime-derived rather than fully explained from shipped bytes. |
| `dnsdb.drv` | The real sound driver. It is a COM-style native code module loaded at offset `0x100`, beginning with nine `E9 rel16` vector jumps. Known vectors include service calls, playback/queue setup, and vector 8 reading the 8237 DMA current-count register so the host can compute playback position. |
| `nosound.drv` | A minimal no-sound driver using the same loaded-driver ABI. It has eight `E9 rel16` vector jumps, all currently resolving to the same target, making it a compact reference for the driver interface. |
| `SCRIPT1.{COD,BAS,VAR,DIC,DEB}` | VM profile 1. The executable's profile table loads resource IDs `2..6`: `script1.cod`, `.bas`, `.var`, `.dic`, `.deb`. Current decoded symbol shape: 122 object records, 13 functions, and 1 kind-5 sequence/cutscene-like symbol. Known gameplay role: opening/tutorial/console/HONK/CRYOBOX/MENU flow. |
| `SCRIPT2.{COD,BAS,VAR,DIC,DEB}` | VM profile 2. Loaded from resource IDs `37..41`. Current decoded symbol shape: 122 objects, 127 functions, 85 kind-5 symbols, and 7 kind-4 symbols. Known gameplay role: bridge/consultation hub, psychotherapy/concept-menu flow, current-location state, and profile handoffs toward later worlds. |
| `SCRIPT3.{COD,BAS,VAR,DIC,DEB}` | VM profile 3. Loaded from resource IDs `76..80`. Current decoded symbol shape: 130 objects, 166 functions, 50 kind-5 symbols, and 6 kind-4 symbols. Known gameplay role: one of the later destination/world script sets and part of the navigation/profile chain. |
| `SCRIPT4.{COD,BAS,VAR,DIC,DEB}` | VM profile 4. Loaded from resource IDs `81..85`. Current decoded symbol shape: 136 objects, 85 functions, 21 kind-5 symbols, and 1 kind-4 symbol. Known gameplay role: one of the later destination/world script sets and part of the navigation/profile chain. |
| `SCRIPT5.{COD,BAS,VAR,DIC,DEB}` | VM profile 5. Loaded from resource IDs `86..90`. Current decoded symbol shape: 130 objects, 89 functions, and 24 kind-5 symbols. Known gameplay role: later destination/world script set including the Bigbang/concert ending state chain. |
| `INSTALL.EXE` | Setup utility. It is Borland TLINK-marked and useful as a local toolchain clue, but it is not evidence that `BLOODPRG.EXE` used the same compiler/linker. |
| `HELP_4_U/CDTEST.EXE` | Helper utility for CD/test support. Not known to be loaded by the game runtime. |
| `HELP_4_U/SNAP.EXE` | Helper utility shipped with the help tools. Not known to be loaded by the game runtime. |

The script bundle extensions have stable roles across all five profiles:

| Extension | Meaning |
|---|---|
| `.COD` | Bytecode consumed by the custom VM. Opcodes are biased at `0xA0`; current dispatch maps 52 opcode slots to 37 distinct handlers. |
| `.BAS` | Binary token/listing-style script companion data. It is not plain text BASIC source and has no QuickBASIC/BRUN signature. |
| `.VAR` | Initial object/runtime state image for the profile. Switching away from a profile frees/reloads this state from disk. |
| `.DIC` | NUL-separated CP437 dictionary words keyed by byte offset; text tokens refer into this dictionary. |
| `.DEB` | Fixed 20-byte symbol records: 16-byte name, 2-byte offset, 2-byte kind. Used to recover objects, functions, cutscene/sequence references, and dialogue context. |

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
