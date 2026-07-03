# Commander Blood (DOS) — Reverse Engineering Notes

End goal (reframed from the generic `re` skill's "web port"): **recover the DOS
engine semantics needed to build a Rust reimplementation that runs the original
Commander Blood CD data files**. The current cutscene/video exporter remains the
first vertical slice: recover the script VM + presentation semantics, then drive
an event-based renderer in Rust. Data files remain the source of truth for
*assets*; the binary provides the *semantics* (opcode meanings, channel routing,
timing, layout, input, navigation, and game-state behavior).

See [`../docs/decompilation-roadmap.md`](../docs/decompilation-roadmap.md) for
the phase plan and definition of done.

See [CLAUDE.md](CLAUDE.md) for tool prefix, addressing model, and conventions.

## Binary Identification

| Field | Value |
|-------|-------|
| File | `re/bin/BLOODPRG.EXE` (from `output/CMDR_BLOOD.iso`) |
| Format | DOS MZ executable (no PE/NE/LE/LX overlay; image == whole file) |
| Platform | MS-DOS, real-mode segmented |
| CPU | 80386 required (`386 minimum !` string; uses 0x66/0x67 prefixes, eax/esi/fs/gs) |
| Memory model | NOT a flat 32-bit DOS-extender. EMS (int 67h) + XMS (int 2Fh AX=43xx) for large memory |
| File size | 86680 (0x152D8) bytes |
| Header size | 1536 (0x600) bytes = 96 paragraphs (`e_cparhdr`) |
| Relocations | 367 (`e_crlc`), table at file 0x1E (`e_lfarlc`) |
| Entry (CS:IP) | 0000:0000 → **file offset 0x600** |
| Initial SS:SP | 0CE2:7E78 |
| Startup DS | **0x0CE2** (`mov ax,0x0ce2; mov ds,ax` at entry) → data seg base = file 0xD420 |
| Startup FS | 0x0BBF (`mov ax,0xbbf; mov fs,ax`) |
| Launcher | `BLOOD.EXE` (696 bytes) — tiny loader, separate MZ |

Evidence trail (verified with `re/tools/mzfile.py`): MZ size accounts for the
whole file (`trailing_bytes=0`), so there is no hidden protected-mode payload;
no DOS/4GW / PMODE / Phar Lap / CauseWay / DJGPP / DPMI(int31) signatures; entry
code is 16-bit real-mode (`int 21h`, far calls, segment setup) with 386 register
extensions. Disassembled cleanly from 0x600.

## Memory Map (load image, base segment 0)

| Region | File range | Notes |
|--------|-----------|-------|
| MZ header | 0x000000–0x000600 | header + 367-entry relocation table |
| Code (hypothesis) | 0x000600–0x00D420 | relative segments 0x000–0xCE1; far-linked (large model) |
| Data segment (DS=0x0CE2) | 0x00D420–0x0152D8 | string table starts here (`386 minimum !` at 0xD420 = DS:0x0000) |
| └ dialogue font | 0x14C22–0x14D28 | ASCII map @0x14C22 (DS:0x7802), advances @0x14CD2, 8-byte glyphs @0x14D28 |

The code/data split is a working hypothesis from the startup DS value; large
model can interleave additional code segments — verify as functions are found.

### Segment map (recovered via `re/tools/dump_segments.py`)

10 code segments + 2 data segments (relative bases). Far calls (9A/EA) target
these; use `dump_segments.py --contains <imgoff>` to map an offset to its segment.

| Seg base | File range | Notes / known contents |
|----------|-----------|------------------------|
| 0x0000 | 0x00600–0x00EB0 | entry/startup |
| 0x008b | 0x00EB0–0x022E0 | |
| 0x01ce | 0x022E0–0x02F90 | |
| 0x0299 | 0x02F90–0x05190 | **render_string @:0202 (0x3192)**, text renderer |
| 0x04b9 | 0x05190–0x053A0 | |
| 0x04da | 0x053A0–0x077E0 | **VM: token_advance @:0F16(0x62B6), token_walker @:1FCF(0x73AF)** |
| 0x071e | 0x077E0–0x09D10 | **sub-dispatch tables cs:0x06D4, cs:0x0F29** |
| 0x0971 | 0x09D10–0x0AFA0 | |
| 0x0a9a | 0x0AFA0–0x0B7B0 | |
| 0x0b1b | 0x0B7B0–0x0C1F0 | |
| 0x0bbf | 0x0C1F0–0x0D420 | FS data segment |
| 0x0ce2 | 0x0D420–0x152D8 | DS data segment (strings, font, tables) |

## Data-Range Map

| Start | End | Size | Classification | Notes |
|-------|-----|------|----------------|-------|
| 0x000000 | 0x000600 | 0x600 | header+relocs | MZ |
| 0x000600 | 0x00D420 | 0xCE20 | code (unverified extent) | entry at 0x600 |
| 0x00D420 | 0x0152D8 | 0x7EB8 | data | strings, font, tables |
| 0x014C22 | 0x014D28 | 0x106 | asset: dialogue font | confirmed (README + screenshots) |

## Key Findings

### Architecture

- Entry @0x600: sets DS/SS=0x0CE2, SP=0x7E78, zeroes 32-bit regs (edi/esi/ebp/ebx),
  `call 0xCCB` (early init, returns status in AX), then sets FS=0x0BBF, GS=DS.
- Far-linked program (large model expected) → use FAR call/jmp (9A/EA) xrefs
  (`re/tools/xref.py SEG:OFF`) as the primary call-graph tool.

### Script VM — bytecode model (PARTIALLY MAPPED)

The scripts are **compiled BASIC**: every `scriptN.cod` has a matching
`scriptN.bas` source (string table at file 0xCE14+). The VM executes the
`.cod` bytecode. Filenames also confirm `son.snd` (voices/SFX) at DS:0x00A6 and
`mus.snd` (music) at DS:0x00AE as the two audio banks.

**Opcode encoding** (decoder at file `0x62B6`, `token_advance`):
- Opcode bytes are **biased by 0xA0**: valid opcodes are `0xA0`–`0xD3`.
  (`sub al,0xa0` appears exactly once in the whole binary, at 0x62BF — so the
  one shared decoder front-ends all token walking.)
- Per-opcode descriptor table at **`DS:0x6F18` (file `0x14338`)**, 2 bytes per
  opcode, indexed by `(op-0xA0)`:
  - `byte0` = token length (bytes incl. opcode) in **mode 0**.
  - `byte1` = token length in **mode 1**, *unless* its high bit is set, in which
    case it is a **mode-control sentinel**:
    `0xFF`→enter mode 1; `0xFE`→enter mode 0; `0xFD`/`0xFB`→if next byte==`0xA1`
    skip it (conditional). Current mode is held in `gs:[0x67AD]`;
    `gs:[0x67B2]&1` gates the 0xFB case.
- `token_advance` computes the length then does `dec al; add si, ax` (one byte
  already consumed by `lodsb`). Length 0 ⇒ special:
  - `0xA6` (TEXT call): skip 5 bytes, then a **0x0000-terminated word list**
    (matches README's `0xa6 … 00 00`).
  - other length-0 opcodes (`0xA8 0xAC 0xCC 0xD3`): call `0x6293`.

**Recovered opcode length table** (mode0 / mode1; `*`=mode-control sentinel,
`var`=length-0 special). Dump with `re/tools/dump_opcode_table.py`:

| op | m0 | m1 | | op | m0 | m1 | | op | m0 | m1 |
|----|----|----|-|----|----|----|-|----|----|----|
| A0 | 3 | * | | AE | 5 | * | | BC | 5 | * |
| A1 | 1 | * | | AF | 5 | * | | BD–C0 | 7 | 7 |
| A2 | 3 | 3 | | B0 | 5 | * | | C1–C8 | 5 | * |
| A3 | 3 | * | | B1 | 7 | 7 | | C9 | 3 | * |
| A4 | 3 | 3 | | B2/B3 | 5 | * | | CA | 5 | 5 |
| A5 | 4 | 2 | | B4–B6 | 7 | 7 | | CB | 6 | 6 |
| A6 | var(TEXT) | | B7 | 4 | * | | CC | var |  |
| A7 | 3 | 3 | | B8/B9 | 7 | 7 | | CD | 7 | * |
| A8 | var | | | BA/BB | 5 | * | | CE–D1 | 1 | 1 |
| A9 | 4 | * | | | | | | D2 | 2 | 2 |
| AA | 1 | 1 | | | | | | D3 | var |  |
| AB | 4 | 4 | | AC | var | | | | | |
| AD | 5 | 5 | | | | | | | | |

`0xC4` is a confirmed 5-byte token: `C4 <record:u16> <related:u16>`. The first
word is the record offset the Rust extractor uses as `object_offset + 0x3A` for
speaker tracking; the second word is a related record offset consumed by the DOS
handler. The core VM token exposes both operands.

`token_walker` at file `0x73AF` uses this decoder to scan a script for `0xA6`
text tokens matching an index `bx`, via far pointers `lds si,[gs:0x6720]` /
`les di,[gs:0x6724]` (a per-script COD/offset table).

### 0xA6 TEXT token — parameter block (DECODED from data; semantics pending)

Layout (confirmed against SCRIPT1/2.COD with `re/tools/dump_text_tokens.py`):

    A6  b1 b2 b3 b4 b5   w0 w1 ... wN  0x0000

- `b1,b2`: constant within a script, differ between scripts (`0A 07` in script2,
  `72 08` in script1). Hypothesis: subtitle position (x,y) or script default
  actor. (TBD via handler.)
- `b3`: **voice / dialogue-line selector**. `0xFF` = no voice line; otherwise a
  small index that increments per spoken line (`01,02,03,04,05…` observed in
  order). Strong candidate for the `son.snd` voice-clip index. *(This is one of
  the fields the current heuristic ignores → wrong subtitle/voice pairing.)*
- `b4`: **display/animation flag bits** — observed `00,08,10,20,28(=20|08),
  30(=20|10),a0…` i.e. clean bitfield, not a counter.
- `b5`: flags; **bit 0x80 = engine "active" flag** (set in-place by
  `token_walker` via `or [si+4],0x80`); also seen `0x90(=80|10)`, `0xA0(=80|20)`.
- `w*`: u16 **dictionary-word offsets** into `SCRIPT*.DIC`, `0x0000`-terminated.
  A `0xFFFF` word appears occasionally — likely an inline marker, not a real
  dict offset (verify in handler).

Next: find the **0xA6 handler** to pin b1..b5 semantics (map b3→son.snd clip,
b4/b5 bits→animation/subtitle-sound/reveal behavior). That handler is the direct
fix for the "wrong subtitle sound/animation/font per line" complaint.

### Dialogue scene LAYOUT (from real-game oracle capture + 0x3D7B)

Real frames (`accuracy/captures/`, `BLOODPRG.EXE` direct boot) show the dialogue/
location view is **letterboxed**, NOT full-screen:
- Scene region = framebuffer rows **`gs:0x5239`..`gs:0x523B`** (the scene clear at
  file 0x3D7B fills exactly this band of `gs:0x5221`). Letterbox values seen:
  `0x23..0xA5` (rows 35..165, a 130px band) — which matches the 320x130 talk-HNM
  update frames.
- Top rows (0..~35) are black; bottom rows (~165..200) are the **HUD panel**
  (the pyramids + central "eye" navigation orb).
- The **landscape** fills the scene band; the **character** (talk HNM) is
  composited *in the band*, positioned within the scene — NOT jammed at (0,0).
The current exporter renders character+background full-screen at (0,0) with no
letterbox/HUD → the placement/clipping the user reports. Fix = render the scene
in rows `gs:0x5239..0x523B` (character HNM at y≈35 over the landscape), matching
the captured layout. (A green "1" overlay = a scene/debug index in direct-boot.)

**TWO render modes (sess 002, confirmed by disasm + `accuracy/captures/`):**
- **Dialogue/location view** (talking head over landscape): letterboxed AND the
  bottom shows the **HUD panel** (perspective pyramid grid + animated central
  "eye" navigation orb). Native-320×200 layout from captures: black top rows
  0..~40; scene band (landscape+character) ~41..145; 1px divider @145; **HUD panel
  rows ~146..193**; black 194..199. So the lower "band" is NOT a black bar — it's
  the HUD. Gated by `gs:0x2793 & 8` (tested @file 0x1018/0x965D/0x9733/0xB193).
  The band top `gs:0x1fa7=0x23` (@0xB3FA), band height 0x82=130 (`gs:0x523B = top
  + 0x82` @0x9DC6). The `gs:0x5239=0x23/0x523B=0xA5` window set when line id==5
  (@0x9EC0) is a TRANSIENT clip window around blits, then restored to 0/0xC8 —
  not a persistent letterbox.
- **Full-HNM cutscene** (e.g. Mindscape logo, astronaut `frame_04`): single
  centered full-frame HNM, **NO HUD** (capture bottom-region mean ≈6.5 = black).
  This is the exporter's `letterbox==false` path and is already correct.
- **HUD asset**: engine-rendered, NOT a single static LBM. Orb = `pe/eye01..10.hnm`
  / `pe/eyeer.hnm` (+ `pe/borb_*`/`bor_*` orb-rotation frames) in BLOOD.DAT's `pe/`
  bank (same bank as the talk-head HNMs); the pyramid grid is drawn procedurally
  by the UI/compass routine @file 0x9656 (angle math 0xB4=180°/0x5A=90°). There is
  no single "pyramids panel" bitmap to blit.
- **Character compositing**: fixed band-relative origin (band top y=35); talk-HNM
  update frames are authored 320×130 to fill the band. NO per-HNM x/y offset
  found in the blit path (uses band clip + buffers `gs:0x5221/0x5229/0x5219`). The
  current fixed-at-band-top compositing is correct.
- **Renderer gap**: in dialogue mode the exporter fills rows ~146..193 with BLACK
  instead of the HUD. Reproducing it faithfully needs either (a) RE'ing the
  procedural pyramid grid + orb HNM animation, or (b) compositing the HUD region
  from a real capture as a static overlay (real pixels, but frozen orb; arguably a
  heuristic). full-HNM path is already correct.

**ON-SHIP vs PLANET classification — RESOLVED (sess 002): NOT statically derivable.**
The HUD is the *player's-ship* pyramid-nav interface; it appears for on-ship
dialogue, not planet/surface dialogue. Gating flag = `gs:0x2793 & 8` (bit 3 =
"ship pyramid-nav UI / HUD active"). All 10 writers of bit 3 (setters @0xB0B7,
0xB505, 0x86B6, 0x7DE1; clearers @0xAFC6, 0x79B4, 0x0FC8, 0x1A5E, 0x187C; toggle
@0x9671) live in the engine's interaction/scene FSM (driven by `gs:0x24F3`,
`gs:0x2535`, the 6-entry nav table `gs:0x2A1B`) — **NONE in the COD VM handler
region**, so no script/DESCRIPT byte maps to it. on-ship/planet is therefore pure
runtime navigation state, not a stored per-scene field. DESCRIPT shape does NOT
encode it either: enemy *spaceships* (Kukaracha/Kraner/Shark) use the SAME 4-LBM
"surface" Location shape as planets, and the player's own ship interior is NOT a
Location record at all (on-ship close-up lines have empty `background_record`).
**Best static proxy = the one the exporter already uses**: a line resolving to a
`kind==1` Location LBM → planet/surface (letterbox scene-band, NO HUD, the
majority — 75/83 dialogue videos); no location → on-ship/narrator close-up
(full-screen — 8/83). That proxy is CONFIRMED correct for scene-band vs
full-screen. But it does NOT cleanly isolate "show HUD": "no-location" conflates
on-ship-crew (HUD) with narrator/intro close-ups (no HUD), so the HUD cannot be
reliably placed from static data. CONCLUSION: the procedural-3D HUD is a minority
case (≤8/83 videos) that can't be statically gated; not built. The validated
letterbox logic (`letterbox = landscape_lbm.is_some()`) stands as correct.

### Subtitle REVEAL TIMING (DECODED) — dialogue updater file 0x93F8–0x94B8

The subtitle reveals one character at a time from the buffer at `gs:0x0E18`,
tracked by reveal pointer `gs:0x5E58` (starts at the buffer start). The advance is
rate-limited by timer `gs:0xB31`: when it hits 0, `inc gs:0x5E58` (reveal one more
char) and reset `gs:0xB31 = gs:0xACA >> 2` (i.e. `gs:0xACA/4` frames per char).
After the reveal pointer reaches the terminating NUL, the dialogue state enters
a line-complete hold: `0x94BA..0x94DD` sets `gs:0xB35 = gs:0xACA*4` and
`gs:0x67BB=1`, while `0x115D..0x1188` keeps that flag alive until the timer
expires and then clears it. A second line-layout path at `0x7350..0x738C` also
sets `gs:0x67BB=1` with duration `gs:0x27CF * (gs:0xACA/2) + 6`. So the old
exporter behavior of mixing one `tb.snd` clip per visible character was wrong.
The Rust SFX track schedules one sidecar chatter event per fully revealed
subtitle line and uses `tb.snd` clip 0, matching `verified-video-scenes.tsv`.
Static RE has not found a direct `0x67BB` → `0x0B1B:0x011D` SND call; the direct
`AX=0` caller at `0x8534` is a separate presentation interaction path.
`gs:0xACA = (textspeed/2)+1` (init @0x1B3A; `textspeed` from a config getter,
special-cased so index 4 → 7). So reveal rate = `4 * frame_rate / gs:0xACA`
chars/sec; at ~15 fps and a mid text speed (`gs:0xACA≈5`) ≈ **12 chars/sec**
(the old `SUBTITLE_CHARS_PER_SEC = 36` was ~3× too fast).

### Subtitle TEXT ASSEMBLY (DECODED) — 0xA6 handler file 0x66CD–0x6739

How the game builds the on-screen subtitle string from the 0xA6 word list (DIC
word offsets), into a buffer at `gs:0x0E18` (DIC segment = `gs:0x672A`):
- For each word: copy its DIC bytes into the buffer (count chars in `dl`).
- Peek the FIRST char of the NEXT word; if it is `,` `.` `?` `!` `:`
  (`0x2C 0x2E 0x3F 0x21 0x3A`) → insert **no space**; otherwise insert `0x20`.
- After inserting a space, if the current line length `dl >= 0x23` (**35**),
  insert a line break `0x0D` and reset `dl=0`. (No wrap check on the no-space
  path.) Long single words are not split.
- At end of list (word offset 0 or 0xFFFF): append `0x0D` then `0x00`.
So subtitles are **multi-line, wrapped at 35 chars**, `0x0D`-separated, with
punctuation-aware spacing. (The current Rust `words.join(" ")` is wrong on both
counts.) Implemented as `assemble_dialogue` in `src/extract/script.rs`.

### Subtitle glyph blitter (DECODED) — file ~0x31C8 `render_string`

Renders a NUL-terminated ASCII string with the dialogue bitmap font. Inputs:
`SI`=string, `DI`=screen offset, `DL`=pixel color, `ES`=screen/back buffer,
`GS`=data seg (=DS). Algorithm per char:
- `0x00` ⇒ end; `0x20` (space) ⇒ `di += 6`.
- `al = gs:[0x7802 + al]` (`xlatb`): ASCII→glyph index via font map; if result has
  bit7 set, char is skipped (not in font).
- `dh = gs:[0x78B2 + glyph]`: per-glyph advance width.
- glyph bitmap = `gs:[0x7908 + glyph*8]` (8 rows × 8 bits).
- plot: for each of 8 rows, shift the row byte; set `es:[di]=dl` per 1-bit;
  `di += 0x140` (**screen stride 320** ⇒ 320×200 mode) between rows.
- after the glyph, `di += dh` (advance).
Confirms README font offsets and gives the exact layout + 320-wide framebuffer.
Font-table references also at file 0x30E6/0x3253/0x35C2 (width-measure / variants
— other text routines in seg 0x0299, not yet mapped; one is likely the animated
cutscene-subtitle reveal).
The string SI is *already resolved ASCII* — so 0xA6 dict-word lookup happens
upstream (find that to reach the 0xA6 handler).

**render_string ABI** (far `0x0299:0x0202`): `BX`=x, `DX`=y, `SI`=string offset,
`AL`=color, `DS`(or ES via ds=es)=string segment, font in GS=DS. Returns width in
`gs:[0x27cd]`-related. Its 5 far callers (file 0x8FEB, 0x9183/99/A8/E4 in seg
0x071e) are all **UI/HUD** text (object-name tooltip at cursor `gs:[0xA2A/0xA2C]`;
status panel; roster loop over object list at `[0x6886]`). Not the cutscene path.

**Object/actor struct fields** (seen in the roster loop, es:di base):
`+0x00` u16 flags (bit1 tested), `+0x02` u16 flags (bit0 tested), `+0x04` name
string (ASCIIZ, passed to render_string), `+0x36` u16 (nonzero gate). This is not
the same table as the six 0x18-byte navigation slots iterated at file `0x7E09`.

### Script VM — execution dispatch (FOUND)

**Main interpreter loop** at file `0x55F5`–`0x569E` (segment 0x04DA):
- `di = 0x6EB0` (handler table base, DS-relative); `lds si, gs:[0x671C]` (COD
  far-ptr). Loop head `0x5613`: `lodsb` opcode; `0xFF` ⇒ end (→0x568A).
- **Dispatch** `0x5627`: `bl=op; sub bl,0xA0; bx=(op-0xA0)*2;
  call word ptr gs:[bx + 0x6EB0]` — a **52-entry near-handler jump table at
  `DS:0x6EB0` (file `0x142D0`)** for opcodes 0xA0..0xD3, offsets into seg 0x04DA
  (file base 0x53A0). Table is immediately followed by the length table at
  0x6F18 (0x68 bytes = 52×2 apart). Dump: `re/tools/dump_handler_table.py`.
- Post-handler: handler may set `gs:0x67B4` (control signal); `gs:0x67AB` =
  skip-N-tokens counter (calls `token_advance` N times → IF/branch skip);
  `gs:0x67B1` bit0=loop active, bit1=loop range; `gs:0x6778/0x677A` loop
  start/resume addrs; `gs:0x6772` = PC.

**Opcode → handler file offsets** (seg 0x04DA). Families share handlers:

| op | handler | | op | handler | | op | handler |
|----|---------|-|----|---------|-|----|---------|
| A0 | 0x6559 | | AC | 0x685C | | C2 | 0x6E34 |
| A1 | 0x6572 | | AD/AF/B2/B3/BA-BC | 0x6946 | | **C3** | **0x6EEE** (record link) |
| A2 | 0x6588 | | AE/B0 | 0x6902 | | **C4** | **0x6C7E** (actor/record op) |
| A3 | 0x6596 (collect words) | | B1/B4-B6/BE-C0 | 0x6863 | | **C5** | **0x6D18** (record entry) |
| A4 | 0x65DB | | B7 | 0x6AA7 | | **C6** | **0x6D80** (record entry) |
| A5 | 0x65EB | | B8/B9/BD | 0x6B06 | | **C7** | **0x6DCF** (record entry) |
| **A6** | **0x660C** (TEXT) | | C1 | 0x6B4C | | **C8** | **0x6F62** (record entry) |
| A7 | 0x67BA | | CA | 0x64E5 | | **C9** | **0x6FB9** (record clear) |
| A8 | 0x67C8 | | CB | 0x6510 | | CD | 0x69C7 |
| A9 | 0x6830 | | CC | 0x64CE | | CE–D2 | 0x6494–0x64B8 (1–2 byte ops) |
| AA | 0x6855 | | | | | D3 | 0x53A0 (seg base = no-op/default) |
| AB | 0x684C | | | | | | |

Secondary jump tables (sub-dispatch within handlers):

- `cs:0x06D4` table at file `0x7EB4`, called indirectly at file `0x7E09`
  from loop routine `0x7D7B`. The loop walks six 24-byte records at
  `DS:0x2A1B`, `0x2A33`, `0x2A4B`, `0x2A63`, `0x2A7B`, `0x2A93`; because `CX`
  counts down while `BP` increments, slot 0 uses table index 5 and slot 5 uses
  table index 0.

      idx  cs:off  file
      0    0x07BC  0x7F9C
      1    0x06E0  0x7EC0
      2    0x095A  0x813A
      3    0x099E  0x817E
      4    0x0A1B  0x81FB
      5    0x08A2  0x8082

- `cs:0x0F29` table at file `0x8709`, called indirectly at file `0x8700` from
  routine `0x85E2`. The caller rejects `AL >= 5`; the preceding
  `test [0x2565],1` does not branch at the call site and is state consumed by
  the handlers, not a dispatch gate.

      idx  cs:off  file
      0    0x0F33  0x8713
      1    0x0F4C  0x872C
      2    0x0FDD  0x87BD
      3    0x1068  0x8848
      4    0x108C  0x886C

Recovered candidate layout for the 24-byte records iterated by `0x7D7B`:

    +00 flags/state byte/word; bit0 active, bit2 initialized/rendered, bit3 hit
    +02 action/object id used by helper 0x7E1C; handlers write 0x12..0x15
    +06 max frame/count loaded by helper
    +08 current frame/counter
    +0A selection/angle compare against 2*gs:0x2795
    +0C hit rect x
    +0E hit rect y
    +10 hit rect w
    +12 hit rect h
    +14 render/blit x passed as BX
    +16 render/blit y passed as CX

### 0xA6 TEXT handler @ file 0x660C — field semantics (DECODED)

On entry `si` points at the token's `b1`. The handler:
- `les di, gs:[0x6724]`; `ax = [b1,b2] (u16)`; `di += ax` ⇒ **`b1:b2` is a u16
  index into the per-line offset/record table** (`gs:0x6724`). `es:[di]` is that
  line's record; `es:[di+2]` holds a flag word (bit15 = already shown/skip).
  The handler sets this `0x8000` bit after accepting a line. Rust exposes this
  as `TEXT_LINE_ALREADY_SHOWN_FLAG` and an opt-in
  `ExecutionContext::with_text_line_display_gating()`. It is intentionally not
  enabled by default yet: raw `SCRIPT*.VAR` line flag words are not the same as
  the initialized runtime line-record table, and applying them directly drops
  valid text from real-script traces.
- saves `si@b3` to `gs:0x677C`; reads **`cx = [b4,b5] (u16)` = the control word**:
  - `b4 & 0x08` ⇒ set skip-count `gs:0x67AB = ((b5>>4)&7)+1` (conditional IF skip).
  - `b4 & 0x10` ⇒ loop: `gs:0x67B1|=1`, next word → `gs:0x6778` (loop target).
  - `b4 & 0x01` ⇒ after accepting a line, clear bit7 of `b5` in the COD stream.
  - `b4 & 0x04` ⇒ skip one extra u16 control word before the dictionary-word loop.
  - **`b5 & 0x80` (bit7) = ACTIVE/DISPLAY flag**: `or cx,cx; jns →skip` — if bit7
    clear the line is not shown (explains why real data always has 0x80).
  - global mutes `gs:0x5E64`, `gs:0x67B0` also gate display.
- later: `si=gs:0x677C; al=[b3]; gs:0x1FAB = (s8)b3` ⇒ **`b3` is the per-line
  selector stored to global `gs:0x1FAB`**. `0xFF` and `0x00` are no-voice
  channels; `1..=N` selects the actor's one-based `son.snd` talk clip.
- dict-word resolution + on-screen display continue past 0x675E (uses `render_*`
  text routines in seg 0x0299).

**b3 selector flow (traced):** `b3` → signed word `gs:0x1FAB` → (reader @0x11F2)
`gs:0x6788 = sign_extend(b3) + 9`, tracked as the **active dialogue-line id**
(compared vs `bx` at 0x120F; reset to `0xFFFF` on clear). Voice clip selection is
resolved in Rust as `b3 == 0xFF || b3 == 0x00` → no voice, `b3 in 1..=N` → actor
`son.snd` clip `b3 - 1`. `src/vm.rs` now owns this as
`text_selector_active_line_id` and `text_selector_voice_clip_index`.

**Clear / scene-reset routines** (the renderer's *clear* event): file `0x1A64`
and `0xB529` both reset `gs:0x1FAB`,`gs:0x6788` (→0xFFFF) plus the display gates
`gs:0x5E64`,`gs:0x67B0`,`gs:0x67BC`,`gs:0x67BA` and call the common stop routine
`0x071E:0x14B6`. Useful as the authoritative subtitle/scene-clear semantics.

**Remaining for full accuracy:** (1) verify whether audible `tb.snd` chatter is
triggered by a dynamic callback rather than the now-decoded `gs:0x67BB` hold
flag; (2) decode any remaining line-record display flags that affect
subtitle/talk-HNM routing; (3) map the remaining C1/C2/CA/CB/CD line-state and
global-condition handlers; (4) `gs:0x6724` line-record layout.

### 0xB7 bit-flag handler @ file 0x6AA7 — state flag set/test (DECODED)

`0xB7` is a 4-byte state/line-record bit flag operation, with an optional `0xA1`
prefix after the opcode. Shape:

    B7 [A1?] <base:u16> <bit:u8>

The handler computes `byte = base + (bit >> 3)` and uses **high-bit-first**
numbering inside each byte: bit 0 = mask `0x80`, bit 1 = `0x40`, ..., bit 7 =
`0x01`.

- Mode 0 (`gs:0x67AD == 0`): no prefix sets the bit (`or es:[byte],mask`); `A1`
  clears it (`and es:[byte],~mask`).
- Mode 1: no prefix tests that the bit is set; `A1` tests that it is clear. A
  failed test calls branch helper `0x6462`.

Shipped scripts use true `B7` tokens in SCRIPT2 and SCRIPT3. Rust now exposes
them as `VmToken::BitFlag` and `execute_trace` applies/evaluates the same
high-bit-first bit semantics.

### 0xB8/0xB9/0xBD pair-record handler @ file 0x6B06 — pair state (DECODED)

`0xB8`, `0xB9`, and `0xBD` share a 7-byte pair-record handler:

    <B8|B9|BD> <record:u16> <first:u16> <second:u16>

The handler loads `les di, gs:[0x6724]`, adds the record offset to `di`, then:

- Mode 0: writes `es:[record]=first` and `es:[record+2]=second`.
- Mode 1: compares both words and calls branch helper `0x6462` if either word
  differs.

After a mode-0 write it also calls helper `0x6034` and, if the result matches
`es:[gs:0x6752 + 0x16]`, clears that `+0x16` field. Rust does not model that
secondary bookkeeping field yet, but it now ports the direct pair write and
branch comparison in `interpret_line_states` / `execute_trace` and exposes the
raw token as `VmToken::PairRecord`.

### 0xC1/0xC2 line-record state handlers — token shape (PARTIALLY DECODED)

`0xC1` and `0xC2` are both fixed 5-byte line-record state operations with the
same raw token shape and an optional mode-1 `A1` inverted-compare prefix:

    <C1|C2> [A1?] <record:u16> <operand:u16>

They load the line-record/state table through `les di, gs:[0x6724]`, then use the
raw record and operand words to resolve additional table slots before either
mutating state (mode 0) or calling branch helper `0x6462` on a failed test (mode
1). `0xC1` has a confirmed success write of `{0x00C1, operand, 0x0002}` after it
finds an empty resolved destination slot. `0xC2` has presentation side effects in
mode 0 for special record kinds: it can clear `gs:0x1FB2` and set active dialogue
line ids `gs:0x6788 = 0x27` or `0x2B`.

Current shipped-script VM walks contain repeated true `C1` tokens and no true
`C2` tokens. Rust now exposes both as `VmToken::RecordState { ..., inverted }`
and the script disassembly emits `record_state` rows. `execute_trace` evaluates
direct mode-1 compares when host state already contains a concrete
`{opcode, operand, ...}` record entry, and Rust now applies the direct `C1`
mode-0 success write when `ExecutionContext` proves the owner object is active
and the destination record is empty. `C2` compare evaluation also requires the
DEB-derived `ExecutionContext` because the binary checks the owner object active
via helper `0x6034`. Rust also ports the direct C2 mode-0 operand-record write:
if the owner is active, `operand+2` has bit `0x20`, and the runtime sentinel
list accepts the operand, helper table `gs:0x6D60` selects a kind-specific field
and Rust writes `0xFFFF` there. Kind `2` records also clear `gs:0x1FB2` and set
active dialogue line `gs:0x6788 = 0x27`. Kind `0x0400` records call helper
`0x7409` with `DI = operand + 4`; that helper opens `descript.des`, scans its
18-byte directory entries for a NUL-terminated name matching `es:[DI]`, and
returns nonzero after dispatching the matched descriptor script. Rust models that
nonzero helper result through `ExecutionContext::with_descript_entry_name`, then
clears `gs:0x1FB2`, ORs bit `0x02` into `gs:0x67AA`, and sets
`gs:0x6788 = 0x2B`. Extractor VM traces now seed that context from parsed
`DESCRIPT.DES` record names. The deeper resolved-table C1 paths remain pending.

### 0xCA/0xCB global condition handlers — token shape (DECODED; runtime source pending)

`0xCA` and `0xCB` are condition handlers, not media commands. They call branch
helper `0x6462` when their comparison fails.

`0xCA` shape:

    CA <op:u8> <tag:u8> <value:u16>

The handler stores the first byte in `dl`, ignores the high byte of the first
`lodsw` (kept by Rust as `tag`), then compares `value` against `gs:0x0AA6`.
`op=0xF1` passes when signed `value > gs:0x0AA6`; `op=0xF2` passes when
signed `value < gs:0x0AA6`; other operators use equality.

`0xCB` shape:

    CB <op:u8> <packed:u16> <reserved:u16>

The handler compares `packed` high/low bytes against `gs:0x0AAA` and
`gs:0x0AA8` as a signed two-byte lexicographic pair. `op=0xF1` is greater-than,
`op=0xF2` is less-than, otherwise equality. The final word is consumed but not
used by the observed compare path, so Rust keeps it as `reserved`.

Rust now exposes these as `VmToken::GlobalWordCompare` and
`VmToken::GlobalPairCompare`, and `script-disassembly.tsv` emits
`global_word_compare` / `global_pair_compare` rows through the same mode-aware
VM walker, so mode-1 `CA`/`CB` tokens are no longer buried inside raw spans.
`execute_trace` branches on them when `ExecutionContext` supplies
`gs:0x0AA6/0x0AA8/0x0AAA`; host-side replay must choose the BIOS RTC
hour/month/day values for deterministic output.

**Runtime source recovered:** the VM wrapper at file `0x55B6..0x55BB` calls two
far routines immediately before `vm_exec_loop`: file `0x093B` reads BIOS RTC time
(`int 1Ah AH=02h`), BCD-decodes `CH`, and stores the current hour in
`gs:0x0AA6`; file `0x0950` reads BIOS RTC date (`int 1Ah AH=04h`),
BCD-decodes `DL`/`DH`, and stores day/month in `gs:0x0AA8`/`gs:0x0AAA`.
`CL` is also converted into a year at `gs:0x0AAC`, adjusted by `CH` century.
Current true script tokens use these as:

- `SCRIPT2`/`SCRIPT3` `CB == 12/25` and `CB == 1/1` date checks.
- Repeated `CA` hour-window checks (`>8`, `<2`, `>12`, `<8`, etc.) for time-of-day
  branch variants.

Rust now exposes `ExecutionContext::with_bios_rtc(hour_24, month, day)` for
deterministic host-side replay of those branches; default traces still leave the
globals absent rather than silently using the developer machine's current date.
The extractor's branch-scenario pass now derives representative RTC replay
contexts from true `CA`/`CB` tokens, including ordinary Jan 2 baselines plus
observed seasonal dates (Christmas/New Year) and hour-boundary candidates.

### 0xCD record-triple handler @ file 0x69C7 — token shape (PARTIALLY DECODED)

`0xCD` is a 7-byte line-record operation with an optional `0xA1` prefix in
mode 1:

    CD [A1?] <record:u16> <first:u16> <second:u16>

Mode 1 compares the direct record entry against `{0x00CD, first, second}` and
calls branch helper `0x6462` when the comparison fails; `A1` inverts the test.
Mode 0 resolves additional table state through helpers `0x6034`, `0x5FD8`, and
`0x5FF6`, writes a word into a computed destination, and can trigger the same
special active-line side effect as `0xC2` (`gs:0x6788 = 0x2B`).

Rust exposes the consumed token as `VmToken::RecordTriple`, emits `record_triple`
disassembly rows, and `execute_trace` now evaluates the direct mode-1 compare
including `A1` inversion. Mode-0 side effects still depend on the resolved
line-record table model and remain unexecuted.

### 0xC3 record-link handler @ file 0x6EEE — relation state (DECODED)

`0xC3` consumes two u16 operands with an optional mode-1 `A1` inverted-compare
prefix: `C3 [A1?] <record:u16> <related:u16>` (5/6 bytes). In mode 0 the handler
checks that both involved records are active and that the destination record is
not already a `0xC4` actor entry. On success it writes:

    es:[record + 0] = 0x00C3
    es:[record + 2] = related
    es:[record + 4] = 0x0001

This is relation/presentation line-record state, not a speaker marker. Several
real scripts emit narrator/system text after `C9` then `C3`; treating `C3` as a
speaker would reintroduce wrong actor/background attribution. Rust exposes it as
`VmToken::RecordLink { ..., inverted }`, applies the guarded mode-0 write when a
DEB-derived `ExecutionContext` can resolve the owner object, and evaluates direct
mode-1 compares with the same context. The parsers deliberately do not update
current speaker state from it. `script-disassembly.tsv` now emits it as
`record_link` instead of leaving those bytes in raw rows.

### 0xC5..0xC8 record-entry handlers — relation state (DECODED)

`0xC5`, `0xC6`, `0xC7`, and `0xC8` are 5-byte line-record operations with the
same token shape: `<opcode> <record:u16> <operand:u16>`. Their mode-0 success
paths write a 6-byte record entry:

| op | handler | stored entry | guard summary |
|----|---------|--------------|---------------|
| C5 | 0x6D18 | `{0x00C5, operand, 0}` | operand record active and type `0x0200`; destination empty |
| C6 | 0x6D80 | `{0x00C6, operand, 0}` | unconditional destination write in mode 0 |
| C7 | 0x6DCF | `{0x00C7, operand, 0}` | operand record active; destination empty or currently `0xC4` |
| C8 | 0x6F62 | `{0x00C8, 0, 0}` | destination empty; second token word is consumed but not stored |

Current shipped-script VM walks find two real `C6` tokens (SCRIPT3/SCRIPT4) and
no true `C5`/`C7`/`C8` opcode positions; raw byte scans see many false positives
inside operands and text data. Rust exposes this family as
`VmToken::RecordEntry { entry_opcode, record_offset, operand,
stored_related_offset, aux_word, inverted }`, and `script-disassembly.tsv` emits
`record_entry` rows for future line-record modeling. Rust now executes the
successful mode-0 writes for the whole family, including C5's active/type-0x0200
operand guard, C7's active-operand plus empty-or-C4 destination guard, and C8's
empty-destination write of `{0x00C8, 0, 0}` despite consuming a second token
word. Direct mode-1 record-entry compares are evaluated when host state has a
concrete record entry. Guarded mode-0 failure branches still need the fuller
line-record table model.

### 0xC4 actor/record handler @ file 0x6C7E — operands (DECODED)

`0xC4` is not a 3-byte actor marker. The binary consumes two u16 operands:
`C4 <record:u16> <related:u16>` (5 bytes total, matching the opcode length
table). The handler loads the per-line/record table through `les di, gs:[0x6724]`,
reads the first word into `bp`, reads the second word into `ax`, and on the
mode-0 success path writes:

    es:[bp + 0] = 0x00C4
    es:[bp + 2] = related
    es:[bp + 4] = 0x0000

Mode 1 first accepts an optional `A1` inverted-compare prefix, then tests that the
owning object is active, `es:[record] == 0x00C4`, and `es:[record+2] == related`;
the failed test calls branch helper `0x6462` unless inverted. The owning object
comes from helper `0x6034`, which maps a record offset back to the previous DEB
object offset (`record == object + 0x3A` for talk records).

The Rust VM now exposes this as `VmToken::Actor { record_offset,
related_record_offset, inverted, len }`. Mode 0 writes the direct
`{0x00C4, related, 0}` record entry and updates speaker context. Mode 1 does not
write that state; `execute_trace` only evaluates the branch when the host-side
state already has a concrete record entry. If the static `SCRIPT*.VAR` slot is
still zero, the line-record table truth is unresolved, so Rust preserves the
guarded actor context without taking the branch until the full runtime table
model is ported.

### 0xC9 record-clear handler @ file 0x6FB9 — speaker lifetime (DECODED)

`0xC9` consumes one u16 record operand (`C9 <record:u16>`, 3 bytes total). The
handler loads the line/record table via `les di, gs:[0x6724]`, reads the existing
record type at `es:[record]`, then zeros the three-word record:

    es:[record + 0] = 0
    es:[record + 2] = 0
    es:[record + 4] = 0

If the previous record type was `0xC4`, it treats the old `es:[record+2]` value
as a related actor subrecord, computes the type-dependent stride through helper
`0x6023` with selector `0x13`, zeros that related 6-byte subrecord too, and
resets presentation gate bytes `gs:[0x252A]=0`, `gs:[0x2531]=6`.

This matters for video accuracy: real scripts frequently emit text after a
matching `C9` and before the next `C4`, so carrying the previous actor through
that clear bleeds the wrong speaker/background into narrator or system lines.
Rust now exposes `VmToken::RecordClear`, clears the current speaker context when
`record_offset == actor_offset + 0x3A`, clears the recovered related C4 subrecord,
and resets the two presentation gate bytes. Unlike the comparison handlers, this
handler has no `gs:0x67AD` mode check: Rust now applies the direct clear in mode
0 and mode 1.

### Dialogue display state machine (seg 0x0971, file ~0x9E81)

Per-frame dialogue updater. `gs:0x6788` = active line id (set by signed 0xA6 b3+9);
`gs:0x678A` = currently-displayed line id. On `0x6788 != 0x678A` it latches the
new id and redraws (lcall render seg 0x299). Special line ids switch the
**viewport clip region**: id `5` and `0x27` set `gs:0x5239=0x23,gs:0x523B=0xA5`
(letterbox window ~rows 0x23..0xC8) then restore — i.e. cutscene vs normal-screen
subtitle framing. `gs:0x5239/0x523B` are the render_string y-clip bounds.
The son.snd/talk-animation trigger is one more hop, via `0xA1B4`/`0xA40B` called
here (seg 0x0971) — these reach the audio playback subsystem. 41 sites touch
`gs:0x6788` across segs 0x008B (display), 0x0971 (this updater), 0x0B1B
(audio/clear), so remaining work here is callback/FSM mapping rather than
another direct `b3` selector formula.

### Audio subsystem (segment 0x0B1B) — located

- `son.snd` (voices/SFX) and `mus.snd` (music) are **per-scene temp files**
  extracted from `BLOOD.DAT`, with DOS handles at `[0x0C47]` (son) / `[0x0C49]`
  (mus). The scene-change cleanup (file 0x12E8) closes (int21 AH=3E) and **deletes**
  (AH=41, dx=0xA6 son.snd / 0xAE mus.snd / 0xCB) them before re-extracting.
- Voice playback + file reads (int21 AH=3F) are all in **segment 0x0B1B**
  (file 0xBA00–0xC0FF). No `lseek` (AH=42) → the SND bank is read **into memory**
  and clips are indexed via the in-bank offset table (same layout `src/snd.rs`
  decodes: u16 num_clips, (num_clips+1) u32 offsets, clip hdr `01 .. sr_code ..`,
  PCM from +6). Temp-file extraction near son.snd name ref at file 0xC19D.
- **SND clip player** (file ~0xB9DE): entered with **`AX` = clip index**.
  - In-memory path: `bp = 0x0BBF + clip*4` → clip table at `DS:0x0BBF` (4 bytes/
    clip: u16 offset + u16 len). `lds si,[0x0BB3]` (bank base) `+ [bp]` (offset)
    `+ 6` (skip clip header) → PCM; `cx = [bp+2]` = length. Matches `audio.rs`.
  - Streamed path: lseek `AX=0x4200` to `[bp]:[bp+2]` (u32), read `AH=0x3F`
    length `[bp+4]-[bp]`, via son.snd handle `gs:[0x0C47]` into buffer `gs:[0xBB7]`.
- **Direct SND entry callers:** `output/bloodprg-snd-call-sites.tsv` is now
  generated from `BLOODPRG.EXE` and lists all direct far calls to `0x0B1B:0x011D`
  plus the constant `AX` clip index recovered immediately upstream. The current
  set is ten calls: clips `0,1,1,2,3,4,5,5,5,6`; file `0x8534` is the
  text/presentation render-path call with `AX=0`. One call (`0x7BF8`) carries
  `AX=1` across a setup far call and is flagged in the TSV.
- **Direct SND bank-loader callers:** `src/bloodprg.rs` now separately scans
  direct far calls to `0x0B1B:0x0855` (file `0xC005`). This is a bank
  loader/extractor, not a clip player: `AX=0` builds the in-memory clip table at
  `DS:0x0BBF` and reads the full bank into `DS:0x0BB3`; `AX!=0` preserves the
  existing table and may materialize temp `son.snd` through the int21
  create/write path at `0xC191..0xC1C7`. The seven direct callers recover these
  `SI` path arguments from static DS strings: `0x0CFC` = `sn\tb.snd`, `0x0D06`
  = `sn\xxxxxxxxxxxx`, `0x0D16` = `sn\radio.snd`, and `0x0D23` =
  `sn\3D.snd`. This keeps bank selection separate from `0x011D` clip playback in
  the Rust decompilation path.
- **Line-complete hold flag:** `gs:0x67BB` is now decoded as a post-reveal hold
  state, not a direct SND call. `0x94BA..0x94DD` sets `b35=aca*4`; `0x7378..0x738C`
  sets `b35=0x27cf*(aca/2)+6`; `0x115D..0x1188` consumes and clears the flag.
  Rust ports the arithmetic in `vm::reveal_complete_hold_ticks` and
  `vm::record_end_hold_ticks`. The shipped scene manifest supports `tb.snd#0`
  for subtitle sidecars, so Rust no longer cycles through `tb.snd` clips.
- Player internals: function entry ~0xB95x; `gs:0x0A5A` = current clip slot
  (`-1` = none → skips play). Buffer/stream state at `0x0BAB`/`0x0BAD`/`0x0BAF`.
  Mixer loop `0xBB6D..0xBB74` performs `lodsb; add al,es:[di]; rcr al,1; stosb`,
  i.e. unsigned PCM floor-average mixing. `src/snd.rs` ports this as
  `snd_mix_average` / `mix_unsigned_pcm_average`.
  Character voice selection is resolved in `script.rs`: `b3==0xFF` or `0x00`
  means no voice, and `b3 in 1..=N` maps to clip `b3-1`. Do not revive the old
  `b3==0xFF -> b4` branch; `b4` is the TEXT control-flag byte, not a clip index.

### BASIC VM nature (important for the renderer)

`analyze_handler.py --table` shows ~all opcode handlers (0xA0–0xD3) make **no far
calls** — they are BASIC language primitives (assign/arith/compare/branch), not
"play sound" commands. Presentation is data-driven: the 0xA6 line records
(`gs:0x6724`) + the per-frame dialogue/audio updaters consume the VM's state.
So the renderer should **walk the COD in execution order** (using the length
table + control flow) and read each 0xA6's (b1:b2 line index, b3 selector, b4/b5
flags), rather than expecting dedicated bg/music/voice opcodes.

### VM resource pointers and runtime state boundary

The resource profile selector at file `0x53A0` takes the desired profile index in
`AX`. If it differs from `DS:0x677E`, the routine releases the old five resource
offsets currently stored at `DS:0x6712` via far call `0x04B9:0x00F8`, stores the
new profile index in `DS:0x677E`, then copies five u16 offsets from
`FS:0x11F4 + AX*10` into `DS:0x6712`. Each copied offset is validated/loaded
through `0x01CE:0x059B`; failure jumps to the existing abort path at `0x5550`.
The same routine then clears the VM globals/lists and scans the selected DEB
object table to cache built-in object offsets.

The VM run wrapper at file `0x55A4` gates execution on `gs:0x67A8`, refreshes
BIOS RTC globals, then resolves the selected **five runtime resource offsets**
from `DS:0x6712` into far pointers at `DS:0x671C..0x672F` via far call
`0x04B9:0x0190`:

| pointer | observed use |
|---|---|
| `DS:0x671C` | COD pointer used by the main exec loop (`lds si, gs:[0x671C]`) |
| `DS:0x6720` | auxiliary COD pointer used by token walkers/helpers |
| `DS:0x6724` | runtime object/line-record state block used by record handlers |
| `DS:0x6728` | DIC pointer used by 0xA6 subtitle assembly (`gs:0x672A` segment) |
| `DS:0x672C` | DEB/object table pointer scanned as 20-byte records |

The `DS:0x6712` words are zero in the EXE image because they are populated from
the static `FS:0x11F4` profile table at runtime. The profile table entries are
resource IDs; each ID indexes the 16-byte resource-name table at `FS:0x0C04`.
Active profiles are:

| profile index | `D2` operand | resources loaded into `DS:0x6712` |
|---:|---:|---|
| 0 | 1 | `script1.cod`, `script1.bas`, `script1.var`, `script1.dic`, `script1.deb` |
| 1 | 2 | `script2.cod`, `script2.bas`, `script2.var`, `script2.dic`, `script2.deb` |
| 2 | 3 | `script3.cod`, `script3.bas`, `script3.var`, `script3.dic`, `script3.deb` |
| 3 | 4 | `script4.cod`, `script4.bas`, `script4.var`, `script4.dic`, `script4.deb` |
| 4 | 5 | `script5.cod`, `script5.bas`, `script5.var`, `script5.dic`, `script5.deb` |

Opcode `0xD2` at file `0x64B8` requests a profile switch by storing
`sign_extend(operand)-1` in `DS:0x6780`; operand 0 therefore stores `0xFFFF`,
the no-pending-script sentinel. The main loop checks `DS:0x6780` at file
`0x108E`, waits until presentation state is idle, then at `0x10C5` calls
`0x04DA:0000`, clears `DS:0x6780` back to `0xFFFF`, sets `DS:0x67A8=1`, and
runs the VM wrapper at `0x55A4`.

Save/load code at `0x1C3F`/`0x1CBD` serializes the current selected profile
index (`DS:0x677E`) and the `DS:0x6724` state block separately using a runtime
size derived from `DS:0x6716`. This is the boundary that blocks using raw
`SCRIPT*.VAR` line flag words as initialized display state: `SCRIPT*.VAR` is an
input image, while the game builds/serializes a live state block through this
pointer setup.

## Location backgrounds: planet (HNM) vs landscape (LBM) — root cause of wrong bg

Each DESCRIPT **Location** record carries TWO kinds of background:
- `FullHnm` (e.g. `ondoya.hnm`, `petrol10.hnm`) — the **planet** view (orbital
  HNM animation; the record's label literally says "planet Ondoya"). The current
  exporter uses this (`full_hnms.first()`) for dialogue → WRONG.
- **4× `Background` commands** = static **LBM** surface images in slots 1–4
  (e.g. Qx20: `petrol1f/1d/1g/1b.lbm`; Ondoya: `glacia1G.lbm` ×4) — the
  **landscape** the dialogue actually plays over.
Fix: dialogue background should load the Location's Background **LBM** (landscape),
not the FullHnm (planet). Slot 1–4 likely selects sub-view/zone (TBD which slot a
dialogue uses — check the scene/actor). LBM = IFF PBM (same format as the `.FD`
full-screen images per README; BLOOD.DAT `FD\*.LBM`).

## Intermediate Output Files

| File | Contents |
|------|----------|
| `re/labels.csv` | accumulated address labels (file-offset / SEG:OFF / DS: forms) |
| `re/bin/BLOODPRG.EXE` | unpacked target (MZ image == whole file, no decompression needed) |
| `script-text-flags.tsv` | extraction artifact listing every `0xA6` TEXT token's b3/b4/b5 control fields and decoded flag summary |
| `script-branch-trace.tsv` | extraction artifact listing `execute_trace` branch/control events per script |
| `script-branch-decisions.tsv` | extraction artifact listing default observed conditional path and alternate target/path |
| `script-branch-coverage.tsv` | extraction artifact summarizing all text calls vs default executed trace coverage per script |
| `script-branch-scenarios.tsv` | extraction artifact forcing each branch decision's opposite condition once and measuring newly exposed text calls |
| `script-branch-scenario-dialogue.tsv` | extraction artifact joining each forced branch scenario trace to decoded text/actor/background rows |
| `script-branch-scenario-dialogue-runs.tsv` | extraction artifact grouping branch scenario dialogue rows into renderer-ready run slices; full export also emits matching `branch-scenario-dialogue-run - ...mp4` files |
| `script-executed-dialogue.tsv` | extraction artifact joining `execute_trace` line order to decoded text/actor/background |
| `script-executed-dialogue-runs.tsv` | extraction artifact grouping executed dialogue by script/background run; MP4 names correspond to run-level composites |
| `script-dialogue-runs.tsv` | extraction artifact grouping VM-order dialogue lines by script/background run |

## Verification Checklist

- [x] Ph1: binary identified (MZ / 386 / EMS+XMS, not flat 32-bit) — tools confirm header
- [ ] Ph2: decompression — N/A (image == whole file, no packer)
- [ ] Ph3: 3+ functions traced (dispatch loop + 2 handlers) and cross-checked
- [ ] Ph4: presentation constants (font/layout/timing/palette) extracted & validated
- [ ] Ph5: script-VM opcode table + scene/actor structs decoded
- [ ] Ph6: generated cutscene compared against real-game capture with a
      frame-aligned pass threshold. `accuracy/compare_oracle.py` now normalizes
      host-window captures and generated MP4 frames to 320x200 and emits metrics,
      but no matched scene has passed yet.

## Reference Resources

- Codex thread (06-14) established the plan and the binary identification.
- `output/` already contains data-side extraction (DESCRIPT, scripts, HNM, SND).

## Next Tasks

### RE Investigation

- [x] Locate the VM token layer: decoder `token_advance` @0x62B6, walker
      @0x73AF, opcode length table @0x14338 (opcodes 0xA0–0xD3) — see Key Findings.
- [x] Decode 0xA6 TEXT token parameter block (b1..b5 + dict words) — data side.
- [x] Find the top-level opcode executor + dispatch + full handler table
      (vm_exec_loop @0x55F5, handler table DS:0x6EB0, 52 handlers resolved).
- [x] Find the 0xA6 TEXT handler @0x660C; decode b1..b5 fields (b1b2=line index,
      b3→gs:0x1FAB selector, b4/b5=control/active flags). See Key Findings.
- [x] Port `gs:0x1FAB` / `gs:0x6788` TEXT selector semantics:
      `src/vm.rs` models signed `b3 + 9` active-line ids and the one-based
      `son.snd` talk-clip selector (`0x00`/`0xFF` = no voice, `1..=N` =
      clip `N-1`). This centralizes the rule that previously lived as duplicated
      parser logic.
- [x] Port the TEXT active/already-shown display gate as an opt-in VM context:
      `b5 & 0x80` is the active/display bit, and `es:[line_index+2] & 0x8000`
      is the already-shown skip bit set by the 0xA6 handler. Default real-script
      traces remain ungated until the runtime initialization of `gs:0x6724` is
      recovered; raw `SCRIPT*.VAR` has incompatible pre-set flag words.
- [x] Port TEXT `b4 & 0x04` control-word parsing:
      both the VM walker and extractor skip the extra u16 before reading DIC word
      offsets, matching the `add si, 2` path in the handler.
- [x] Map the VM resource pointer setup boundary:
      `0x53A0/0x53C8` selects a five-offset resource profile from
      `FS:0x11F4 + AX*10` into `DS:0x6712`; `0x55A4/0x55D9` resolves those
      runtime offsets into COD, state, DIC, and DEB/object far pointers at
      `DS:0x671C..0x672F`, while save/load code serializes the `DS:0x6724`
      runtime state block separately.
- [x] Decode the `FS:0x11F4` resource-profile table:
      static resource IDs map through `FS:0x0C04` to five profiles for
      `script1`..`script5`; opcode `D2` stores operand-1 in `DS:0x6780` for the
      main-loop profile handoff.
- [x] Model `D2` cross-script profile scheduling in Rust execution traces:
      `ExecutionTrace` records D2 profile requests and
      `execute_script_profile_sequence` follows the last non-sentinel pending
      profile through the decoded script profiles.
- [x] Export cross-script profile sequences from the extractor:
      `script-profile-runs.tsv` and `script-profile-executed-dialogue.tsv`
      preserve the DOS main-loop SCRIPT1->SCRIPT2->... handoff order using the
      binary-derived resource profile table.
- [x] Consume profile-sequence dialogue rows in the event renderer/video grouping:
      `profile-dialogue-run` MP4s group by global execution order instead of
      per-script order, while the old per-script videos remain for comparison.
- [ ] Decode the `gs:0x6724` per-line record layout (es:[di], es:[di+2] flags).
- [ ] Verify audible `tb.snd` chatter trigger path, if any. `gs:0x67BB` itself is
      now decoded as post-reveal hold state rather than a direct SND caller.
- [ ] Map the presentation opcodes among the handler table: which set background,
      music (mus.snd), HNM actor, voice (son.snd), wait, clear. Start with the
      remaining C1/C2/CA/CB/CD handlers and presentation callbacks rather than
      expecting direct media-play opcodes.
- [x] Port 0xB7 bit-flag semantics. `src/vm.rs` exposes
      `VmToken::BitFlag`, applies high-bit-first set/clear writes in mode 0, and
      `execute_trace` evaluates mode-1 bit tests with optional `A1` inversion.
- [x] Port 0xB8/0xB9/0xBD pair-record semantics. `src/vm.rs` exposes
      `VmToken::PairRecord`, applies mode-0 two-word writes, and evaluates
      mode-1 pair compares through the branch stack. The handler's secondary
      `gs:0x6752+0x16` bookkeeping clear remains outside the current model.
- [x] Expose 0xC1/0xC2 line-record state tokens. `src/vm.rs` keeps their raw
      record/operand words and optional mode-1 inversion as
      `VmToken::RecordState`, and `script-disassembly.tsv` can now show true
      `record_state` rows instead of raw byte spans. Direct mode-1 compares are
      now executed when concrete host-state records are available. The direct
      C1 mode-0 success write `{0x00C1, operand, 0x0002}` is also applied when
      the DEB-derived context proves the owner active. C2 mode-0 now applies
      the `gs:0x6D60` kind-field write, the kind-2 active-line side effect
      (`gs:0x6788 = 0x27`), and the kind-0x0400/helper-0x7409 active-line side
      effect (`gs:0x67AA|=2`, `gs:0x6788 = 0x2B`) when `ExecutionContext`
      supplies the matching `descript.des` directory name. Extractor trace paths
      seed those names from parsed `DESCRIPT.DES`; resolved-table C1 paths
      remain pending.
- [x] Expose 0xCA/0xCB global condition tokens. `src/vm.rs` preserves the
      consumed compare operands as `VmToken::GlobalWordCompare` and
      `VmToken::GlobalPairCompare`; `execute_trace` evaluates their branches
      when `ExecutionContext` supplies `gs:0x0AA6/0x0AA8/0x0AAA`. The binary RTC
      writers are recovered; host replay chooses values via `with_bios_rtc`, and
      the extractor now emits RTC branch-scenario replays from real `CA`/`CB`
      operands. `script-disassembly.tsv` now uses the mode-aware VM walker for
      these tokens instead of ad hoc raw-byte spans.
- [x] Expose 0xCD record-triple tokens. `src/vm.rs` preserves the consumed
      record/first/second words and optional `A1` inverted-compare prefix as
      `VmToken::RecordTriple`, and `execute_trace` evaluates the direct mode-1
      record-triple compare. Resolved-table mode-0 side-effect execution remains
      pending.
- [x] Decode the cs:0x0F29 and cs:0x06D4 sub-dispatch tables. Table starts,
      indirect call sites, raw handler-offset arrays, target file offsets, and
      the 24-byte actor/object struct iterated at 0x7E09 are documented; handler
      semantics still need permanent names beyond temporary table-entry labels.
- [x] Reconcile 0xC4 length and operands. The handler consumes two u16 operands,
      writes a 6-byte record entry on success, and `src/vm.rs` now exposes both
      words plus optional mode-1 `A1` inversion instead of reducing the token to a
      single actor id.
- [x] Port 0xC3 record-link semantics. `src/vm.rs` exposes
      `VmToken::RecordLink` with optional mode-1 inversion, the context-aware VM
      applies guarded mode-0 writes and direct mode-1 compares using DEB object
      offsets, and parser tests lock in that `C3` does not restore speaker
      context after a `C9` clear.
- [x] Port 0xC5..0xC8 record-entry token semantics. `src/vm.rs` exposes the
      family as `VmToken::RecordEntry` including raw operand and recovered
      stored-related slot; disassembly now emits `record_entry` rows.
      Successful mode-0 writes for C5/C6/C7/C8 and direct mode-1 compares are
      now executed; guarded mode-0 failure branches remain pending.
- [x] Port 0xC9 record-clear speaker lifetime semantics. `src/vm.rs` exposes
      `VmToken::RecordClear`, the bounded interpreter clears the active actor
      when its talk-field record is cleared in either VM mode, and the script
      parsers stop carrying actor/background context past matching `C9` tokens.
      The port also applies the selector-0x13 related C4 subrecord clear and the
      `gs:0x252A/0x2531` presentation gate reset.
- [ ] Map presentation constants: subtitle position, reveal rate, colors, timing,
      HNM actor reset/loop policy, audio mix levels.

### Renderer Integration (replaces skill's "Web Port")

- [x] Embed the recovered opcode-length table + 0xA6 decoder as a verified Rust
      module `src/extract/vm.rs` (token type + `walk()`). Unit test
      `table_matches_binary` confirms the table is byte-exact vs BLOODPRG.EXE;
      `walks_synthetic_cod` confirms 0xA6 decode incl. the `b4&0x10` loop word.
- [x] Found + fixed two production decoder bugs in `decode_text_call_at`
      (`src/extract/script.rs`), grounded in the recovered fixed layout:
      (1) it only accepted `b5 == 0x80` exactly, dropping lines whose b5 carried
      extra flag bits (0x90/0xA0); (2) it didn't skip the `b4 & 0x10` loop-target
      word, misreading it as a dict offset and dropping the line.
      **Impact: ~18% of all dialogue (666 / 3682 lines: 493 extra-flag + 173
      loop) were being silently dropped; now recovered.** Covered by unit tests
      `decode_text_tests::*`. (`vm.rs` handles the same cases.)
- [x] **Whole-script linear walk WORKS** (control-flow interpreter NOT needed).
      The earlier "desync" was the variable-length opcodes `0xA8/0xAC/0xCC/0xD3`
      (helper 0x6293 scans for a `0x0000` word terminator — see `scan_zero_word`).
      With that fixed, `vm::walk` decodes all 5 scripts cleanly to the `0xFF`
      end marker, 0 invalid (SCRIPT2 = 3271 tokens / 1157 text). This is the
      foundation for execution-order scene-state tracking.
- [x] Object-ref opcodes decode correctly now: `0xC4` = 5-byte actor/record
      operation (`record_offset = object_offset + 0x3A` talk field for speaker
      tracking, plus a second related-record word; 71/95 first operands resolve
      to Characters), `0xC3` = non-speaker record link, and `0xC9` = record
      clear. Location is NOT set by referencing a location object.
### Runtime object-state model (CONFIRMED) — path to accurate location

- The VM keeps a **runtime object-state area** addressed `es:[bx+di]` with
  `les di, gs:[0x6724]`; `bx` = a variable/field address. `obj+24` in this space
  is a character's **current location** (LOCATION_FIELD). The 58%-covered lines
  read the *static initial* copy from `SCRIPT*.VAR`; runtime changes live here.
- **`0xAF` (and family @0x6946) is a CONDITIONAL**: `IF state[op1] == op2 { skip
  tokens }` (calls 0x6462 to skip). In SCRIPT2, 12 of these test
  `state[char+24] == <Location>` (e.g. usine/Ark/Hita) — i.e. the script branches
  on a character's current location. Confirms `obj+24` = location and that the
  state area holds it.
- **Implication:** because location-assignments are gated by these conditionals,
  computing the *actual* per-line location requires **evaluating** the script —
  a bounded interpreter over the `gs:0x6724` state area: object-field assignments
  + the comparison/branch opcodes (`0xAF` family etc.). The COD is linearly
  walkable (no jump-table chasing), so this is a state-machine interpreter, not a
  full CPU emulator. This is the genuine "replay the game's logic" the goal asks
  for, and the route to ~100% location/speaker coverage.
- [x] Assignment opcodes decoded and ported. 0x6863 family (b1/b4-b6/be-c0), 7-byte:
      `op [op1:u16] [operator:u8] [op2mode:u8] [op2:u16]`; operators `0xF5`=set,
      `0xF6`=add, `0xF7`=sub (+ comparisons `0xF0`=ne `0xF3`=le… for conditionals);
      writes `state[op1]` in mode 0 (`mov es:[bx+di],cx` @0x68FD). op2mode
      `0xC0/0xC2` = indirect (`op2 = state[op2]`). The Rust interpreter evaluates
      the recovered mode-1 branch comparisons without applying their mode-0
      state writes.
- [x] Ported the direct 0xC4 record write and evidence-based mode-1 compare.
      Rust now writes `{0x00C4, related, 0}` in mode 0, decodes optional `A1`
      inversion in mode 1, and branches only when a concrete record entry is
      available in host VM state. Zeroed static `SCRIPT*.VAR` line-record slots
      remain unresolved guarded actor contexts until the full `gs:0x6724` runtime
      table model is wired in.
- [x] Ported the other mode-0 mutation handlers to Rust: 0x6902 family (AE/B0)
      bitmask set/clear (`or es:[bx+di],ax` / `and es:[bx+di],~ax`) and 0x6946
      family (AD/AF/B2/B3/BA/BB/BC) direct assignment (`mov es:[bx+di],ax`
      @0x69C2). Rust now also mirrors the 0x6946 write-side sentinel bookkeeping
      for fields assigned to `blood`/`0xFFFF`: helper `0x5FD8` removes an owner
      object from the 16-word list at `DS:0x6D3E`, and helper `0x5FF6` inserts it
      before storing `0xFFFF`.
- [x] Ported the 0x6946 mode-1 special-object compare. Script metadata init at
      file `0x549a..0x54a1` matches the DEB object name `blood` (built-in string
      `DS:0x67BE`) and stores its object offset in `gs:0x674E`. The 0x6946 mode-1
      handler then remaps a RHS operand equal to `gs:0x674E` to `0xFFFF` before
      equality/inversion testing (`0x6963..0x696e`). `ExecutionContext` now
      carries that DEB-derived sentinel, and branch traces/scenario speech use it
      for game-accurate `AD/AF/B2/B3/BA/BB/BC` compares.
- [x] **Interpreter prototype VALIDATED** (Python): init state from `SCRIPT*.VAR`,
      walk + execute 0x6863-family assigns, track `state[actor+24]` per 0xA6 line.
      Location coverage **58% → 63%** (SCRIPT2 61%, SCRIPT3 68%, SCRIPT4 65%,
      SCRIPT5 67%, SCRIPT1 0% = trivial title script).
- Decision-relevant: the gain is **modest** because the dominant gap is the **22%
      of lines with no tracked speaker** (no `0xC4` even with cross-function
      persistence) — many are likely **narrator/system lines that legitimately
      have no character/location** (so the "gap" is partly correct-as-is).
      Conditional/branch evaluation would lift it further, but the
      speaker-attribution cap bounds the realistic ceiling.
- [x] Ported the interpreter to Rust (`vm::interpret_line_states`, tested) and
      INTEGRATED it: `parse_script_speech` runs `resolve_runtime_backgrounds` per
      script and each `0xA6` line prefers the executed **runtime** location
      (`state[actor+24]` → DESCRIPT) over the static initial one, with no
      hardcoded fallback. **Shipped result: background coverage 56% → 61%** in the
      exported `script-speech.tsv` / videos.
- [x] Ported the first branch-aware execution trace to Rust (`vm::execute_trace`),
      grounded in the A0/A1/0x6462 control stack:
      - A0 @0x6559: `gs:0x67AD=1`, push the u16 target operand into the stack at
        `gs:0x6820 + gs:0x6884`, then `gs:0x6884 += 2`.
      - A1 @0x6572: `gs:0x67AD=0`, pop one stack slot only when
        `gs:0x6884 != 2` (the first slot remains as the current block target).
      - branch-fail helper @0x6462: `gs:0x6884 -= 2`; `si = [0x6820+gs:0x6884]`;
        `gs:0x67AD=0`.
      - A4 @0x65DB and A9 @0x6830 direct jump/reset behavior is modeled for
        inspected script paths.
      `execute_trace` now evaluates mode-1 conditionals for the 0x6863 signed
      compare family, 0x6902 bitmask family, and 0x6946 equality family, while
      retaining the linear `interpret_line_states` path for all-possible-line
      manifests. Real-script smoke via `inspect-vm <COD> <VAR>` reaches
      `EndMarker` for all scripts: SCRIPT1 102 executed lines / 38 branch events;
      SCRIPT2 169 / 327; SCRIPT3 327 / 553; SCRIPT4 145 / 229; SCRIPT5 258 / 387.
- [x] Wire branch-aware initial-state execution into the current per-character
      dialogue video generator: `create_character_videos` now consumes
      `ScriptExecutedSpeechLine`, groups each character by script/location, and
      orders lines by `execute_trace` sequence index instead of raw COD offset.
      `script-dialogue-videos.tsv` is generated from the same executed rows.
- [x] Add branch-aware run-level dialogue composites: the full exporter now
      renders `script-executed-dialogue-runs.tsv` groups as
      `executed-dialogue-run - ...` MP4s, tracking `ShowSpeaker` events so a
      single scene can switch actor SND banks/talk HNMs without splitting by
      character.
- [ ] Further gains: make comprehensive dialogue generation cover alternate
      branches. The current exporter is no longer the linear all-lines path and
      no longer has to split one executed run by actor, but it still represents
      only the default initial-state execution. Full coverage needs branch
      enumeration or scenario selection. Bounded by the ~22% no-speaker lines
      (many are legitimately narrator/locationless).
- [x] Add branch planning artifacts: `script-branch-decisions.tsv` records each
      concrete conditional's observed path and alternate path/target, while
      `script-branch-coverage.tsv` summarizes static `0xA6` text calls vs the
      default executed trace per script. These are the manifest inputs for
      scenario-selected or branch-enumerated rendering.
- [x] Add branch override execution: `vm::execute_trace_with_overrides` can force
      a specific condition result, and `script-branch-scenarios.tsv` applies the
      opposite path to every concrete branch decision once, measuring text-call
      deltas. This turns the branch coverage gap into executable scenario data.
- [x] Expose TEXT control flags: `script-text-flags.tsv` lists every `0xA6`
      token's `b3`, `b4`, `b5`, active bit, conditional skip count, loop target,
      known parse-control bits, and still-unknown `b4` payload bits. This gives
      the subtitle sound/animation audit a concrete Rust artifact instead of
      burying those fields in raw token params.
- [x] Correct subtitle chatter timing: `src/extract/subtitle_sfx.rs` now follows
      the recovered `0x94BA..0x94DD`/`0x115D..0x1188` state machine by scheduling
      one `tb.snd` chatter event after a subtitle finishes revealing, instead of
      the previous one-SFX-per-character approximation.
- [x] Emit binary-derived SND entry call sites:
      `bloodprg-snd-call-sites.tsv` scans `BLOODPRG.EXE` for direct far calls to
      `0x0B1B:0x011D`, recovers the upstream constant `AX` clip index, and flags
      the one call where `AX` is carried across a setup far call. This gives the
      chatter/voice SFX audit a test-backed caller map instead of handwritten
      disassembly notes.
- [x] Emit binary-derived SND bank-loader call sites:
      `src/bloodprg.rs` scans direct far calls to `0x0B1B:0x0855`, recovers the
      upstream `AX` bank mode plus `SI` static SND path, and test-locks the seven
      direct loader calls. This prevents the Rust decompilation work from
      treating bank selection/extraction as clip playback.
- [x] Use `tb.snd` clip 0 for subtitle chatter:
      `src/extract/subtitle_sfx.rs` now reuses the first decoded `tb.snd` clip
      for every line-complete chatter event instead of cycling through a filtered
      `7..16` subrange. This matches `verified-video-scenes.tsv` (`sn/tb.snd#0`)
      and the direct text/presentation SND call at `0x8534` (`AX=0`).
- [x] Port recovered SND bank semantics into Rust:
      `src/snd.rs` now owns the `BLOODPRG.EXE` clip-player bank layout: original
      AX clip index, offset table, 6-byte clip header skip, sample-rate byte, and
      unsigned 8-bit PCM payload. Audio export, subtitle chatter, and character
      dialogue rendering now share this recovered model instead of duplicating
      local parsers.
- [x] Port the SND average mixer primitive:
      `src/snd.rs` implements the `0xBB6D..0xBB74` `add`+`rcr` unsigned PCM
      mixer as `snd_mix_average` and verifies it exhaustively for all u8 sample
      pairs against the 8086 carry/rotate behavior.
- [x] Centralize TEXT selector voice mapping:
      `src/vm.rs` exposes `text_selector_active_line_id` and
      `text_selector_voice_clip_index`; both public and extractor script parsers
      call that recovered VM/presentation rule instead of hand-rolling the old
      `b3` tests.
- [x] Port TEXT active/already-shown line gating as an opt-in trace mode:
      `src/vm.rs` exposes `TEXT_ACTIVE_DISPLAY_FLAG`,
      `TEXT_LINE_ALREADY_SHOWN_FLAG`, and
      `ExecutionContext::with_text_line_display_gating()`. The gated mode skips
      inactive `b5` lines, skips `line_index+2` words with bit `0x8000` set, and
      marks accepted lines as shown; the default path stays ungated because raw
      `SCRIPT*.VAR` is not the initialized runtime line-record table.
- [x] Map VM resource pointer setup:
      `re/labels.csv` now names the `DS:0x6712` source-offset table, the five
      far pointers at `DS:0x671C..0x672F`, the wrapper/resolve loop at
      `0x55A4/0x55D9`, the post-exec record updater at `0x5816`, and the
      save/load state serialization sites at `0x1C3F/0x1CBD`.
- [x] Map VM resource profile selection:
      `0x53A0` now names the selector taking profile index `AX`, the
      `0x53C8/0x53DA` copy/validate loop from `FS:0x11F4 + AX*10` into
      `DS:0x6712`, and `DS:0x677E` as the cached current profile index.
- [x] Decode script profile resources and `D2` handoff:
      `FS:0x0C04` is the 16-byte resource-name table, `FS:0x11F4` maps five
      script profiles to COD/BAS/VAR/DIC/DEB resource IDs, and opcode `D2`
      writes the pending profile index to `DS:0x6780` for the main loop at
      `0x108E/0x10C5`.
- [x] Port D2 profile-request scheduling:
      `src/vm.rs` now records `ScriptProfileRequestEvent`s in execution traces
      and exposes `execute_script_profile_sequence` to follow the DOS-style
      pending-profile handoff across decoded script profiles.
- [x] Wire binary profile sequences into exporter manifests:
      `src/extract/script.rs` loads COD/VAR/DIC/DEB resources from the
      BLOODPRG.EXE profile table and emits run-level plus global-order dialogue
      TSVs for the default profile chain.
- [x] Route profile-sequence rows into run-level videos:
      `src/extract/character.rs` now renders `ScriptProfileDialogueRun`s through
      the same VM event emitter as the existing dialogue videos, preserving
      cross-profile global sequence order.
- [x] Port the `gs:0x67BB` line-complete hold timers:
      `src/vm.rs` models `0x94D4..0x94DD` (`b35=aca*4`) and `0x7378..0x738C`
      (`b35=0x27cf*(aca/2)+6`) as checked helper functions. Labels and known
      symbols now name the set/consume sites.
- [x] Emit branch-scenario dialogue rows/runs:
      `script-branch-scenario-dialogue.tsv` reuses the same executed-dialogue
      resolver against each forced branch trace, and
      `script-branch-scenario-dialogue-runs.tsv` keeps scenario-tagged run slices
      separate from the default execution. Default full export still renders only
      the initial-state run videos; bulk alternate rendering needs an explicit
      selection policy to avoid exploding output volume.
- [x] Define the VM-event schema (`SceneEvent`: SetBackground, PlayMusic,
      ShowSpeaker, PlayVoice, PlayTalkHnm, DrawSubtitle, PlayChatter, Clear) +
      `emit_scene_events()` emitter in `src/vm.rs`, emitting state-change
      events on transition only. Unit-tested (`emits_state_changes_on_transition_only`).
- [x] Wire `emit_scene_events` into `character.rs`: the dialogue renderer
      (`create_character_dialogue_video`) now builds the `SceneEvent` IR and
      renders by consuming it (SetBackground/PlayMusic/PlayVoice/DrawSubtitle),
      instead of scanning grouped lines directly. The render path is now
      VM-event-driven.
- [x] Removed all heuristic fallbacks (per user "no fallbacks just compute it
      accurately"): dropped the static `CHAR_CONTEXTS` background fallback, the
      `lookup_character_context` gate, and the redundant `hnm_music` re-lookup.
      Background/music now come purely from the DESCRIPT-derived per-line data
      (actor location → location HNM → HNM music). Coverage from real data:
      ~68% location, ~58% background HNM, ~56% voice clip; the rest have no
      derivable value yet (not faked).
- [x] **Accurate voice clip selection** RESOLVED (sess 002): formula is
      `b3==0xFF|0x00 → no voice`, `b3∈1..=N → clip=b3-1`. The old `b3==0xFF →
      clip=b4` guess read the b4 flag word as an index and spuriously voiced
      513/1942 (26%) of lines; removed in `script.rs`. See dead_ends.md
      "RESOLVED — voice-clip selection". (Final AX arithmetic sits behind a SND
      callback `lcall [0xcdf]`, but the formula is confirmed by the +9 reader +
      player + export-data distribution.)
- [ ] Remaining for *full* faithfulness: replace the `(script,function)` grouping
      itself with branch-aware execution-order dialogue runs. The event IR and
      `execute_trace` are in place; the next renderer step is branch enumeration
      or scenario-selected execution so comprehensive videos do not collapse to
      only the default initial-state path.

### Oracle

The DOSBox-X oracle harness works (boots the real game on isolated Xvfb;
`BLOODPRG.EXE` runs directly into the intro cutscene — see `accuracy/`). But
without scripted input to drive it to specific scenes it is still not sufficient
for per-scene pass/fail comparison.

Current workflow: improve VM accuracy → export videos (`./target/release/
commander-blood-tools <dir>`) → compare frame candidates with
`accuracy/compare_oracle.py` → manually inspect mismatches → iterate. Next
oracle step is scripted input or a debug scene selector so one generated
dialogue run can be compared against a matched real-game capture with a
threshold.
