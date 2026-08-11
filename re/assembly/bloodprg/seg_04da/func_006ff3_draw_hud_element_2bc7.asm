; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x006ff3
; seg_off: 04da:1c53
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: draw_hud_element_2bc7
; label_comment: draw a HUD element: ax=0x1f; lcall 0x299:0x1241 (draw); bp=0x2bc7 (element data table). Renders a HUD/status element from the 0x2bc7 layout data
; incoming: call@0x008c9d->04da:1c53
; byte_count: 251
; boundary: cfg_blocks_19_terminals_4
; terminal: jmp 0x7009:1, jmp 0x7084:1, jmp 0x70ad:1, retf:1
; direct_callees: 0x006023
; indirect_calls: 4
; cxx_source: re/borland/bloodprg/seg_04da/func_006ff3_draw_hud_element_2bc7.cpp
; routine_bytes_sha256: d3a65c8af19b8d9e6e126b8eaea038f1a7acdf239a0594e6681e1e028d612698

006FF3:  06                           push     es
006FF4:  57                           push     di
006FF5:  1E                           push     ds
006FF6:  56                           push     si
006FF7:  66 50                        push     eax
006FF9:  53                           push     bx
006FFA:  51                           push     cx
006FFB:  66 52                        push     edx
006FFD:  55                           push     bp
006FFE:  B8 1F 00                     mov      ax, 0x1f
007001:  9A 41 12 99 02               lcall    0x299, 0x1241
007006:  BD C7 2B                     mov      bp, 0x2bc7
007009:  8B 46 00                     mov      ax, word ptr [bp]
00700C:  0B C0                        or       ax, ax
00700E:  74 09                        je       0x7019
007010:  C6 46 14 00                  mov      byte ptr [bp + 0x14], 0
007014:  83 C5 16                     add      bp, 0x16
007017:  EB F0                        jmp      0x7009
007019:  65 C4 3E 2C 67               les      di, ptr gs:[0x672c]
00701E:  65 8E 1E 26 67               mov      ds, word ptr gs:[0x6726]
007023:  65 8B 36 52 67               mov      si, word ptr gs:[0x6752]
007028:  BD 86 68                     mov      bp, 0x6886
00702B:  C7 46 00 00 00               mov      word ptr [bp], 0
007030:  66 33 C0                     xor      eax, eax
007033:  8B C8                        mov      cx, ax
007035:  8B 1C                        mov      bx, word ptr [si]
007037:  B8 0B 00                     mov      ax, 0xb
00703A:  E8 E6 EF                     call     0x6023
00703D:  67 66 8B 14 30               mov      edx, dword ptr [eax + esi]
007042:  26 8B 75 10                  mov      si, word ptr es:[di + 0x10]
007046:  65 3B 36 52 67               cmp      si, word ptr gs:[0x6752]
00704B:  74 37                        je       0x7084
00704D:  8B 1C                        mov      bx, word ptr [si]
00704F:  81 FB 00 01                  cmp      bx, 0x100
007053:  74 1A                        je       0x706f
007055:  B8 0B 00                     mov      ax, 0xb
007058:  E8 C8 EF                     call     0x6023
00705B:  0B C0                        or       ax, ax
00705D:  74 25                        je       0x7084
00705F:  67 66 3B 14 30               cmp      edx, dword ptr [eax + esi]
007064:  75 1E                        jne      0x7084
007066:  89 76 00                     mov      word ptr [bp], si
007069:  83 C5 02                     add      bp, 2
00706C:  41                           inc      cx
00706D:  EB 15                        jmp      0x7084
00706F:  B8 09 00                     mov      ax, 9
007072:  E8 AE EF                     call     0x6023
007075:  67 66 3B 14 30               cmp      edx, dword ptr [eax + esi]
00707A:  74 EA                        je       0x7066
00707C:  67 66 3B 54 30 04            cmp      edx, dword ptr [eax + esi + 4]
007082:  74 E2                        je       0x7066
007084:  83 C7 14                     add      di, 0x14
007087:  26 8B 45 12                  mov      ax, word ptr es:[di + 0x12]
00708B:  83 F8 01                     cmp      ax, 1
00708E:  74 B2                        je       0x7042
007090:  8C E8                        mov      ax, gs
007092:  8E D8                        mov      ds, ax
007094:  C7 46 00 00 00               mov      word ptr [bp], 0
007099:  BD 86 68                     mov      bp, 0x6886
00709C:  66 8E 06 26 67               mov      es, word ptr [0x6726]
0070A1:  E3 3F                        jcxz     0x70e2
0070A3:  49                           dec      cx
0070A4:  BE C7 2B                     mov      si, 0x2bc7
0070A7:  8B 7E 00                     mov      di, word ptr [bp]
0070AA:  83 C7 04                     add      di, 4
0070AD:  8B 04                        mov      ax, word ptr [si]
0070AF:  0B C0                        or       ax, ax
0070B1:  74 2F                        je       0x70e2
0070B3:  9A C4 02 CE 01               lcall    0x1ce, 0x2c4
0070B8:  72 05                        jb       0x70bf
0070BA:  83 C6 16                     add      si, 0x16
0070BD:  EB EE                        jmp      0x70ad
0070BF:  C6 44 14 01                  mov      byte ptr [si + 0x14], 1
0070C3:  8B 44 10                     mov      ax, word ptr [si + 0x10]
0070C6:  C4 3E 7C 0A                  les      di, ptr [0xa7c]
0070CA:  0D 00 80                     or       ax, 0x8000
0070CD:  9A 37 10 99 02               lcall    0x299, 0x1037
0070D2:  8B 44 12                     mov      ax, word ptr [si + 0x12]
0070D5:  B9 18 FC                     mov      cx, 0xfc18
0070D8:  8B D9                        mov      bx, cx
0070DA:  BD 00 00                     mov      bp, 0
0070DD:  9A BE 11 99 02               lcall    0x299, 0x11be
0070E2:  5D                           pop      bp
0070E3:  66 5A                        pop      edx
0070E5:  59                           pop      cx
0070E6:  5B                           pop      bx
0070E7:  66 58                        pop      eax
0070E9:  5E                           pop      si
0070EA:  1F                           pop      ds
0070EB:  5F                           pop      di
0070EC:  07                           pop      es
0070ED:  CB                           retf    
