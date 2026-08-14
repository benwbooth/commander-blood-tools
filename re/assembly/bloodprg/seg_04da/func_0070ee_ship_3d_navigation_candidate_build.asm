; Commander Blood recovered routine assembly
; module: bloodprg
; artifact: re/bin/BLOODPRG.EXE
; artifact_sha256: 7e756c597190d20e71a0210da3898b9746c39e04db922455b07f74ec26166823
; file_offset: 0x0070ee
; seg_off: 04da:1d4e
; group: seg_04da
; provenance: recursive_graph, relocation_proven_far_transfer_target
; label: ship_3d_navigation_candidate_build
; label_comment: calls ship_3d_nav_source_list_build_full with inherited ES:DI and SS:BP=0x6886, then filters the 0xffff-terminated source through DS=GS into a zero-terminated SS/DS:0x2b53 list. It excludes DS:0x6754 Honk before lookup and keeps only exact kind 2 records whose +2 flag byte has bit 0 set. The addr32 ES:[EAX+EDI] lookup clears EAX but requires incoming upper EDI to be zero.
; incoming: call@0x00b37b->04da:1d4e
; byte_count: 79
; boundary: cfg_blocks_8_terminals_2
; terminal: jmp 0x710b:1, retf:1
; direct_callees: 0x00624b
; indirect_calls: 0
; routine_bytes_sha256: 900f4f7fa776f880c818e637a2d67d768aafbc885b5f9f9f021956cec82f6bf7

0070EE:  55                           push     bp
0070EF:  56                           push     si
0070F0:  66 50                        push     eax
0070F2:  1E                           push     ds
0070F3:  66 33 C0                     xor      eax, eax
0070F6:  BD 86 68                     mov      bp, 0x6886
0070F9:  0E                           push     cs
0070FA:  E8 4E F1                     call     0x624b
0070FD:  8C E8                        mov      ax, gs
0070FF:  8E D8                        mov      ds, ax
007101:  BE 86 68                     mov      si, 0x6886
007104:  BD 53 2B                     mov      bp, 0x2b53
007107:  C4 3E 24 67                  les      di, ptr [0x6724]
00710B:  AD                           lodsw    ax, word ptr [si]
00710C:  83 F8 FF                     cmp      ax, -1
00710F:  74 21                        je       0x7132
007111:  3B 06 54 67                  cmp      ax, word ptr [0x6754]
007115:  74 19                        je       0x7130
007117:  67 26 8B 1C 38               mov      bx, word ptr es:[eax + edi]
00711C:  83 FB 02                     cmp      bx, 2
00711F:  75 0F                        jne      0x7130
007121:  67 26 F6 44 38 02 01         test     byte ptr es:[eax + edi + 2], 1
007128:  74 06                        je       0x7130
00712A:  89 46 00                     mov      word ptr [bp], ax
00712D:  83 C5 02                     add      bp, 2
007130:  EB D9                        jmp      0x710b
007132:  C7 46 00 00 00               mov      word ptr [bp], 0
007137:  1F                           pop      ds
007138:  66 58                        pop      eax
00713A:  5E                           pop      si
00713B:  5D                           pop      bp
00713C:  CB                           retf    
