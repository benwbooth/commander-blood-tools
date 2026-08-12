# Toolchain Fingerprints

This directory records derived fingerprints for the shipped DOS executables.
The raw game binaries are intentionally not stored here; `re/bin` and extracted
ISO trees are ignored because they contain copyrighted game files.

Generated with:

```sh
python3 re/tools/toolchain_fingerprint.py --sample-limit 16 \
  re/bin/BLOODPRG.EXE \
  re/bin/BLOOD.EXE \
  output/_tmp_iso/INSTALL.EXE \
  > re/source/toolchain_fingerprints/mz_profiles.json
```

Current input hashes:

| file | bytes | sha256 |
| --- | ---: | --- |
| `re/bin/BLOODPRG.EXE` | 86680 | `7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823` |
| `re/bin/BLOOD.EXE` | 696 | `cecd2d07b576cedd460aeb7cfb6ea3e93fbf2cfd1f1890d5ce37c80c4d36c335` |
| `output/_tmp_iso/INSTALL.EXE` | 64200 | `7f7f3e30a0a2cadd1b557461e2d71f2f3bcb27253679e0bfac8a1d45076a073c` |

`output/_tmp_iso/BLOODPRG.EXE` and
`commander-blood-audio/_tmp_iso/BLOODPRG.EXE` matched the `re/bin` hash at the
time this profile was generated. The two extracted `INSTALL.EXE` copies under
`output/_tmp_iso` and `commander-blood-audio/_tmp_iso` also matched each other.

## Observations

`BLOODPRG.EXE` is the main game program: an 86680-byte MZ executable with a
1536-byte header, 85144-byte load image, entry `0x0000:0x0000`, stack
`0x0ce2:0x7e78`, 367 relocations, 12 inferred segment bases, 365 relocated far
transfers, and 107 distinct far-transfer targets. Its relocation sites are
monotonic. The profile found no Borland, Microsoft, Watcom, QuickBASIC, DOS
extender, or other marker strings in this executable.

`BLOOD.EXE` is only a tiny 696-byte launcher. It has a 512-byte header,
184-byte load image, 4 relocations, and no far-transfer targets. It is not the
game logic payload.

`INSTALL.EXE` is a 64200-byte installer/configuration program. It has a
512-byte header, 63688-byte load image, 24 relocations, and one string marker
hit for `Microsoft`. Its relocation order has two backtracks, unlike
`BLOODPRG.EXE`. The visible strings include `REM *** BLOOD launcher V5.12 ***`,
so this file is useful for installer provenance but should not be treated as
proof of the main game's compiler.

## Current Local Toolchain Check

On 2026-08-11 in this shell, `command -v` found Wine and `objdump`, but did not
find `dosbox`, `dosbox-x`, `bcc`, `tcc`, `wcc`, `wasm`, or `nasm` on PATH. A
bounded `/home/ben` search for common DOS compiler executables such as
`TC.EXE`, `TCC.EXE`, `BCC.EXE`, `BCC32.EXE`, `WCL.EXE`, `CL.EXE`, `LINK.EXE`,
and `TLINK.EXE` completed with no matches.

## Interpretation

The main executable still has no definitive compiler ID. The strongest current
facts are structural:

- it is segmented real-mode MZ code, not a 32-bit flat protected-mode program;
- it uses many 386 prefixes (`66`, `67`, `64`, `65`) inside 16-bit code;
- it contains many far calls/returns and relocation-derived segment references;
- it does not expose an obvious runtime/compiler signature string.

The next evidence step is to compile the `re/compiler_corpus` samples with real
candidate DOS compilers and fingerprint/disassemble those outputs against
`mz_profiles.json` and the recovered routine assembly.
