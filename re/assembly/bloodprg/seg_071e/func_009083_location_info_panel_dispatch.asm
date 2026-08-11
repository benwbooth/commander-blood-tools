; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x009083
; seg_off: 071e:18a3
; group: seg_071e
; provenance: recursive_graph
; label: location_info_panel_dispatch
; label_comment: the info panel's per-frame DISPATCH: es=gs; `test byte [0x2788],1 / je 0x9125` selects zoom-open vs the rest; within zoom-open `cmp [0x2789],0 / jne 0x90FE` runs the ONE-TIME setup only on the first frame (resolve the object's artwork through DS:0x2BC7, load it, position entity 0 at the cursor, and stash the scaled width at [0x277E]). PORTED: engine.rs LocationPanelState + step_location_info_panel
; byte_count: 445
; boundary: cfg_blocks_26_terminals_6
; terminal: jmp 0x90a7:1, jmp 0x91c3:1, jmp 0x91f1:1, jmp 0x923f:2, ret:1
; direct_callees: 0x009240
; indirect_calls: 16
; cxx_source: re/borland/bloodprg/seg_071e/func_009083_location_info_panel_dispatch.cpp
; routine_bytes_sha256: 290e3acc332846a85d22dd7219164a91efa14d5b024d1e6552a311ec778f12d6

009083:  8C E8                        mov      ax, gs
009085:  8E C0                        mov      es, ax
009087:  F6 06 88 27 01               test     byte ptr [0x2788], 1
00908C:  0F 84 95 00                  je       0x9125
009090:  06                           push     es
009091:  80 3E 89 27 00               cmp      byte ptr [0x2789], 0
009096:  75 66                        jne      0x90fe
009098:  66 8E 06 26 67               mov      es, word ptr [0x6726]
00909D:  BE C7 2B                     mov      si, 0x2bc7
0090A0:  8B 3E BF 27                  mov      di, word ptr [0x27bf]
0090A4:  83 C7 04                     add      di, 4
0090A7:  80 3C 00                     cmp      byte ptr [si], 0
0090AA:  74 52                        je       0x90fe
0090AC:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0090B1:  72 05                        jb       0x90b8
0090B3:  83 C6 16                     add      si, 0x16
0090B6:  EB EF                        jmp      0x90a7
0090B8:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0090BB:  0D 00 80                     or       ax, 0x8000
0090BE:  06                           push     es
0090BF:  C4 3E 7C 0A                  les      di, ptr [0xa7c]
0090C3:  9A 37 10 99 02               lcall    0x299, 0x1037
0090C8:  33 ED                        xor      bp, bp
0090CA:  8B C5                        mov      ax, bp
0090CC:  8B 1E 2A 0A                  mov      bx, word ptr [0xa2a]
0090D0:  8B 0E 2C 0A                  mov      cx, word ptr [0xa2c]
0090D4:  9A BE 11 99 02               lcall    0x299, 0x11be
0090D9:  BE 12 62                     mov      si, 0x6212
0090DC:  C4 7C 04                     les      di, ptr [si + 4]
0090DF:  26 8B 05                     mov      ax, word ptr es:[di]
0090E2:  B3 0E                        mov      bl, 0xe
0090E4:  F6 E3                        mul      bl
0090E6:  C1 E8 05                     shr      ax, 5
0090E9:  A3 7E 27                     mov      word ptr [0x277e], ax
0090EC:  07                           pop      es
0090ED:  B8 CE FF                     mov      ax, 0xffce
0090F0:  33 DB                        xor      bx, bx
0090F2:  8B CB                        mov      cx, bx
0090F4:  8B D3                        mov      dx, bx
0090F6:  BF 11 5F                     mov      di, 0x5f11
0090F9:  9A 00 00 CE 01               lcall    0x1ce, 0
0090FE:  07                           pop      es
0090FF:  FE 06 89 27                  inc      byte ptr [0x2789]
009103:  E8 3A 01                     call     0x9240
009106:  B8 00 00                     mov      ax, 0
009109:  BB 01 00                     mov      bx, 1
00910C:  9A E1 14 99 02               lcall    0x299, 0x14e1
009111:  BF AB 2A                     mov      di, 0x2aab
009114:  BE 80 27                     mov      si, 0x2780
009117:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00911C:  0F 83 1F 01                  jae      0x923f
009120:  C6 06 88 27 00               mov      byte ptr [0x2788], 0
009125:  F6 06 88 27 02               test     byte ptr [0x2788], 2
00912A:  0F 85 C3 00                  jne      0x91f1
00912E:  F6 06 3E 0A 01               test     byte ptr [0xa3e], 1
009133:  0F 85 F3 00                  jne      0x922a
009137:  B8 00 00                     mov      ax, 0
00913A:  BB 00 00                     mov      bx, 0
00913D:  9A E1 14 99 02               lcall    0x299, 0x14e1
009142:  8B 1E 80 27                  mov      bx, word ptr [0x2780]
009146:  8B 0E 82 27                  mov      cx, word ptr [0x2782]
00914A:  8B 16 84 27                  mov      dx, word ptr [0x2784]
00914E:  8B 2E 86 27                  mov      bp, word ptr [0x2786]
009152:  8B 36 C8 0A                  mov      si, word ptr [0xac8]
009156:  9A 0E 04 99 02               lcall    0x299, 0x40e
00915B:  BB 6E 00                     mov      bx, 0x6e
00915E:  BA 19 00                     mov      dx, 0x19
009161:  BE 2E 01                     mov      si, 0x12e
009164:  66 8E 06 26 67               mov      es, word ptr [0x6726]
009169:  8B 3E BF 27                  mov      di, word ptr [0x27bf]
00916D:  26 F7 05 10 00               test     word ptr es:[di], 0x10
009172:  74 03                        je       0x9177
009174:  BE 37 01                     mov      si, 0x137
009177:  26 F7 05 00 01               test     word ptr es:[di], 0x100
00917C:  74 03                        je       0x9181
00917E:  BE 3E 01                     mov      si, 0x13e
009181:  B0 EE                        mov      al, 0xee
009183:  9A 02 02 99 02               lcall    0x299, 0x202
009188:  03 1E CD 27                  add      bx, word ptr [0x27cd]
00918C:  83 C3 06                     add      bx, 6
00918F:  1E                           push     ds
009190:  8B F7                        mov      si, di
009192:  83 C6 04                     add      si, 4
009195:  8C C1                        mov      cx, es
009197:  8E D9                        mov      ds, cx
009199:  9A 02 02 99 02               lcall    0x299, 0x202
00919E:  1F                           pop      ds
00919F:  BE 4B 01                     mov      si, 0x14b
0091A2:  BB 6E 00                     mov      bx, 0x6e
0091A5:  83 C2 0A                     add      dx, 0xa
0091A8:  9A 02 02 99 02               lcall    0x299, 0x202
0091AD:  83 C2 0A                     add      dx, 0xa
0091B0:  BD 86 68                     mov      bp, 0x6886
0091B3:  9A AB 0E DA 04               lcall    0x4da, 0xeab
0091B8:  BD 86 68                     mov      bp, 0x6886
0091BB:  1E                           push     ds
0091BC:  8C C0                        mov      ax, es
0091BE:  8E D8                        mov      ds, ax
0091C0:  B8 FE 00                     mov      ax, 0xfe
0091C3:  8B 76 00                     mov      si, word ptr [bp]
0091C6:  83 C5 02                     add      bp, 2
0091C9:  83 FE FF                     cmp      si, -1
0091CC:  74 20                        je       0x91ee
0091CE:  F7 04 02 00                  test     word ptr [si], 2
0091D2:  74 EF                        je       0x91c3
0091D4:  F7 44 02 01 00               test     word ptr [si + 2], 1
0091D9:  74 E8                        je       0x91c3
0091DB:  83 7C 36 00                  cmp      word ptr [si + 0x36], 0
0091DF:  74 E2                        je       0x91c3
0091E1:  83 C6 04                     add      si, 4
0091E4:  9A 02 02 99 02               lcall    0x299, 0x202
0091E9:  83 C2 0A                     add      dx, 0xa
0091EC:  EB D5                        jmp      0x91c3
0091EE:  1F                           pop      ds
0091EF:  EB 4E                        jmp      0x923f
0091F1:  FE 0E 89 27                  dec      byte ptr [0x2789]
0091F5:  E8 48 00                     call     0x9240
0091F8:  B8 00 00                     mov      ax, 0
0091FB:  BB 01 00                     mov      bx, 1
0091FE:  9A E1 14 99 02               lcall    0x299, 0x14e1
009203:  BF 80 27                     mov      di, 0x2780
009206:  BE AB 2A                     mov      si, 0x2aab
009209:  9A AD 0F 8B 00               lcall    0x8b, 0xfad
00920E:  73 2F                        jae      0x923f
009210:  33 C0                        xor      ax, ax
009212:  9A 41 12 99 02               lcall    0x299, 0x1241
009217:  C6 06 88 27 00               mov      byte ptr [0x2788], 0
00921C:  C7 06 BF 27 00 00            mov      word ptr [0x27bf], 0
009222:  C7 06 6A 67 00 00            mov      word ptr [0x676a], 0
009228:  EB 15                        jmp      0x923f
00922A:  C6 06 8C 27 00               mov      byte ptr [0x278c], 0
00922F:  C6 06 88 27 02               mov      byte ptr [0x2788], 2
009234:  C6 06 DB 0A 00               mov      byte ptr [0xadb], 0
009239:  FE 06 89 27                  inc      byte ptr [0x2789]
00923D:  EB B2                        jmp      0x91f1
00923F:  C3                           ret     
