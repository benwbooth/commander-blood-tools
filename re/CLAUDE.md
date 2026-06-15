# Commander Blood RE — Conventions

Reverse-engineering workspace for `BLOODPRG.EXE`, driving the accuracy work in
the parent Rust crate. Read [REVERSE.md](REVERSE.md) for findings and the task
list; read [dead_ends.md](dead_ends.md) before re-trying a stuck approach.

## Platform

- DOS MZ executable, 16-bit real-mode segmented, **80386** instruction set
  (0x66/0x67 prefixes; eax/esi/edi/ebp, fs/gs in use).
- Large-model far linkage expected → functions are reached by FAR call/jmp
  (opcodes 9A / EA) whose segment word is in the relocation table.
- Large memory via EMS (int 67h) + XMS (int 2Fh AX=43xx), NOT a flat extender.

## Tool prefix & invocation

All tools are run through the nix dev shell from the **repo root**:

    nix develop --command python3 re/tools/<tool>.py ...

Tools: `mzfile.py` (shared loader), `dis.py`, `search_bytes.py`, `xref.py`,
`seg_offset.py`, `strings_dump.py`. They auto-load `re/labels.csv`.

## Addressing model

- **file offset** `0xNNNNN` — byte offset in BLOODPRG.EXE (also the disasm address).
- **image offset** — file offset minus the 0x600 header (`--img` in dis.py).
- **SEG:OFF** — relative segment (paragraph index into the load image, base 0)
  and offset; `file = 0x600 + SEG*16 + OFF`.
- **DS:0xNNNN** — offset within the startup data segment (DS=0x0CE2, file 0xD420).

Convert with `seg_offset.py`. labels.csv accepts `0xNNNNN`, `SEG:OFF`,
`DS:0xNNNN`, `IMG:0xNNNN` in the addr column.

## Deviations from the generic `re` skill

- **Disassembler = capstone** (CS_MODE_16), wrapped in `dis.py`, instead of a
  hand-written `instruction_set.py`. Rationale: a full, correct 386 decoder
  (incl. all 0x66/0x67 forms) is impractical to hand-roll; capstone is a
  deterministic library (not an interactive RE framework like radare2/Ghidra,
  which the skill rightly forbids). dis.py still auto-loads labels.csv so the
  knowledge base accumulates the same way.
- **End goal = event-driven renderer in the Rust crate**, not a standalone web
  port. Ph4/Ph7's `web/catalog.html` may still be used as a visual asset
  validator if useful, but the deliverable is accurate video output.
- **Oracle deferred**: user chose RE-first; reference captures (dosbox-x) come
  after the renderer can emit output. Target scenes: Bob_Morlock, Izwalito,
  a multi-character scene, a subtitle-only screen, a full HNM cutscene.
