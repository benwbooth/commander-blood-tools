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

- **XDB:<name>:0xNNNN** — an offset inside an OVERLAY (`croolis.xdb`, `manu3.xdb`,
  `amer.xdb`, `scrut.xdb`), whose runtime `cs` maps 1:1 to file offsets. A third
  address space alongside the executable and the drivers; the overlay's name is
  part of the address because the same offset means different things in each.
- **SCRIPT<N>:0xNNNN** — an offset inside SHIPPED SCRIPT DATA (`SCRIPT1..8.COD`
  and their record/VAR space), NOT inside the executable. A fifth address space,
  added after `location_var_offset` was found citing "SCRIPT2: 0x0F4E" (audit-fixes
  #299): disassembling BLOODPRG.EXE at `0x0F4E` decodes unrelated bytes, and
  `check_cited_instructions.py` cannot verify such a claim because there is no
  instruction to check. Claims in this space are verified by RUNNING the port
  against the shipped script instead — the `location_var_offset` test does exactly
  that. Always write the script name; a bare `0x0F4E` reads as a file offset.
- **DRV:0xNNNN** — an offset inside a SHIPPED SOUND DRIVER (`dnsdb.drv`,
  `nosound.drv`), a second binary the game loads and calls through a far-pointer
  vector table (`re/tools/drv_vectors.py`). It is a DIFFERENT ADDRESS SPACE: a
  bare `0x0305` in the addr column would be read as a BLOODPRG.EXE offset and
  decode unrelated bytes. The prefix exists because a driver label was first
  added without one (2026-07-25) and would have done exactly that.

## Deviations from the generic `re` skill

- **Disassembler = capstone** (CS_MODE_16), wrapped in `dis.py`, instead of a
  hand-written `instruction_set.py`. Rationale: a full, correct 386 decoder
  (incl. all 0x66/0x67 forms) is impractical to hand-roll; capstone is a
  deterministic library (not an interactive RE framework like radare2/Ghidra,
  which the skill rightly forbids). dis.py still auto-loads labels.csv so the
  knowledge base accumulates the same way.
- **End goal = Rust reimplementation of the DOS engine**, not a standalone web
  port. The event-driven renderer remains the first vertical slice because it
  validates the VM, renderer, and audio semantics. Ph4/Ph7's `web/catalog.html`
  may still be used as a visual asset validator if useful, but the deliverable
  is a Rust runtime that can run the original data files.
- **Oracle deferred**: user chose RE-first; reference captures (dosbox-x) come
  after the renderer can emit output. Target scenes: Bob_Morlock, Izwalito,
  a multi-character scene, a subtitle-only screen, a full HNM cutscene.
